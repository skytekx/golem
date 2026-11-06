#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# The scanner of the plugin survey. It reads every jar of corpus.json and writes one JSON record
# per jar: the CraftBukkit, Minecraft, Mojang and Bukkit API references with the version tag of
# the class that holds them, the reflection sites with their resolved targets, the Paper and
# Adventure use, the version adapters, the shaded libraries and a summary with the level. This
# is the record format that the reconciled data of the survey keeps.
#
#     python3 -I scan.py --dir <survey dir> --spigot-api <spigot-api jar> [--jobs N]
#                        [--only SUBSTR ...] [--out PATH]
#
# The survey dir holds corpus.json and jars/. The API model of the spigot-api jar and the JDK
# members it inherits, which javap of a JDK on the PATH lists, are cached as api_model.json and
# jdk_members.json in the survey dir. The jars are untrusted data. This script reads them only
# with zipfile, in memory, nested jars included, parses the class files itself and never writes
# a file named from inside an archive. Nothing from a jar is executed, loaded or put on a
# classpath, and javap only ever sees classes of the JDK.
import sys, os, io, re, json, struct, zipfile, hashlib, time, traceback, subprocess
from collections import Counter, defaultdict

CORPUS = JARS_DIR = SPIGOT_JAR = JDK_CACHE = API_CACHE = OUT = None


def set_paths(survey, spigot_jar, out):
    global CORPUS, JARS_DIR, SPIGOT_JAR, JDK_CACHE, API_CACHE, OUT
    survey = os.path.abspath(survey)
    CORPUS = os.path.join(survey, 'corpus.json')
    JARS_DIR = os.path.realpath(os.path.join(survey, 'jars'))
    SPIGOT_JAR = os.path.abspath(spigot_jar)
    JDK_CACHE = os.path.join(survey, 'jdk_members.json')
    API_CACHE = os.path.join(survey, 'api_model.json')
    OUT = os.path.abspath(out) if out else os.path.join(survey, 'scan.jsonl')

MAX_JAR = 60 * 1024 * 1024
MAX_ENTRY = 60 * 1024 * 1024
MAX_NEST_DEPTH = 3
SIM_BUDGET_S = 240.0          # per jar, for the stack simulation only
SIM_MAX_CODE = 30000          # bytes of code per simulated method
ROUNDS = 6                    # interprocedural rounds (wrapper depth), stops early when nothing changes

# --------------------------------------------------------------------------------------------
# Class file parsing
# --------------------------------------------------------------------------------------------

class CFError(Exception):
    pass


def mutf8(raw):
    """Decode the modified UTF-8 of the JVM (NUL as C0 80, supplementary chars as surrogate pairs)."""
    try:
        return raw.decode('utf-8')
    except UnicodeDecodeError:
        pass
    b = raw.replace(b'\xc0\x80', b'\x00')
    try:
        s = b.decode('utf-8', 'surrogatepass')
    except UnicodeDecodeError:
        return b.decode('utf-8', 'replace')
    try:
        return s.encode('utf-16', 'surrogatepass').decode('utf-16')
    except UnicodeDecodeError:
        return s


class Method:
    __slots__ = ('access', 'name', 'desc', 'code', 'max_locals', 'handlers', 'annots')


class ClassFile:
    __slots__ = ('tags', 'vals', 'major', 'access', 'name', 'super', 'ifaces', 'fields', 'methods',
                 'bsm', 'annots', 'const_fields')

    def cname(self, i):
        return self.vals[self.vals[i]]

    def nat(self, i):
        a, b = self.vals[i]
        return self.vals[a], self.vals[b]

    def ref(self, i):
        c, nt = self.vals[i]
        a, b = self.vals[nt]
        return self.vals[self.vals[c]], self.vals[a], self.vals[b]


def _skip_ev(ub, pos):
    tag = ub[pos]
    pos += 1
    if tag in b'BCDFIJSZsc':
        return pos + 2
    if tag == 0x65:  # e
        return pos + 4
    if tag == 0x40:  # @
        pos += 2
        n = (ub[pos] << 8) | ub[pos + 1]
        pos += 2
        for _ in range(n):
            pos = _skip_ev(ub, pos + 2)
        return pos
    if tag == 0x5b:  # [
        n = (ub[pos] << 8) | ub[pos + 1]
        pos += 2
        for _ in range(n):
            pos = _skip_ev(ub, pos)
        return pos
    raise CFError('bad element_value tag %r' % tag)


def _annot_types(ub, pos, vals):
    n = (ub[pos] << 8) | ub[pos + 1]
    pos += 2
    out = []
    for _ in range(n):
        t = (ub[pos] << 8) | ub[pos + 1]
        np_ = (ub[pos + 2] << 8) | ub[pos + 3]
        pos += 4
        out.append(vals[t])
        for _ in range(np_):
            pos = _skip_ev(ub, pos + 2)
    return out


def parse_class(ub, want_code=True):
    if len(ub) < 10 or ub[:4] != b'\xca\xfe\xba\xbe':
        raise CFError('bad magic')
    minor, major, n = struct.unpack_from('>HHH', ub, 4)
    tags = bytearray(n)
    vals = [None] * n
    pos = 10
    i = 1
    while i < n:
        t = ub[pos]
        tags[i] = t
        pos += 1
        if t == 1:
            ln = (ub[pos] << 8) | ub[pos + 1]
            pos += 2
            raw = ub[pos:pos + ln]
            pos += ln
            try:
                vals[i] = raw.decode('utf-8')
            except UnicodeDecodeError:
                vals[i] = mutf8(raw)
        elif t == 7 or t == 8 or t == 16 or t == 19 or t == 20:
            vals[i] = (ub[pos] << 8) | ub[pos + 1]
            pos += 2
        elif 9 <= t <= 12 or t == 17 or t == 18:
            vals[i] = ((ub[pos] << 8) | ub[pos + 1], (ub[pos + 2] << 8) | ub[pos + 3])
            pos += 4
        elif t == 3:
            vals[i] = struct.unpack_from('>i', ub, pos)[0]
            pos += 4
        elif t == 4:
            pos += 4
        elif t == 5 or t == 6:
            pos += 8
            i += 1
        elif t == 15:
            vals[i] = (ub[pos], (ub[pos + 1] << 8) | ub[pos + 2])
            pos += 3
        else:
            raise CFError('bad constant tag %d' % t)
        i += 1
    if pos + 8 > len(ub):
        raise CFError('truncated')
    cf = ClassFile()
    cf.tags = tags
    cf.vals = vals
    cf.major = major
    access, this, sup, ic = struct.unpack_from('>HHHH', ub, pos)
    pos += 8
    cf.access = access
    cf.name = vals[vals[this]]
    cf.super = vals[vals[sup]] if sup else None
    cf.ifaces = [vals[vals[x]] for x in struct.unpack_from('>%dH' % ic, ub, pos)]
    pos += 2 * ic
    # fields
    fc = (ub[pos] << 8) | ub[pos + 1]
    pos += 2
    fields = []
    const_fields = []
    for _ in range(fc):
        fa, fn, fd, ac = struct.unpack_from('>HHHH', ub, pos)
        pos += 8
        fname, fdesc = vals[fn], vals[fd]
        fields.append((fa, fname, fdesc))
        for _ in range(ac):
            an, al = struct.unpack_from('>HI', ub, pos)
            pos += 6
            if vals[an] == 'ConstantValue' and al == 2:
                ci = (ub[pos] << 8) | ub[pos + 1]
                if tags[ci] == 8:
                    const_fields.append((fname, fdesc, vals[vals[ci]]))
            pos += al
    cf.fields = fields
    cf.const_fields = const_fields
    # methods
    mc = (ub[pos] << 8) | ub[pos + 1]
    pos += 2
    methods = []
    for _ in range(mc):
        ma, mn, md, ac = struct.unpack_from('>HHHH', ub, pos)
        pos += 8
        m = Method()
        m.access = ma
        m.name = vals[mn]
        m.desc = vals[md]
        m.code = None
        m.max_locals = 0
        m.handlers = ()
        m.annots = ()
        for _ in range(ac):
            an, al = struct.unpack_from('>HI', ub, pos)
            pos += 6
            aname = vals[an]
            if aname == 'Code' and want_code:
                ms, ml, cl = struct.unpack_from('>HHI', ub, pos)
                m.max_locals = ml
                m.code = ub[pos + 8:pos + 8 + cl]
                p2 = pos + 8 + cl
                el = (ub[p2] << 8) | ub[p2 + 1]
                hs = []
                for k in range(el):
                    s, e, h, ct = struct.unpack_from('>HHHH', ub, p2 + 2 + 8 * k)
                    hs.append((s, e, h, ct))
                m.handlers = hs
            elif aname == 'RuntimeVisibleAnnotations' or aname == 'RuntimeInvisibleAnnotations':
                try:
                    m.annots = tuple(m.annots) + tuple(_annot_types(ub, pos, vals))
                except (IndexError, CFError, TypeError):
                    pass
            pos += al
        methods.append(m)
    cf.methods = methods
    # class attributes
    cf.bsm = []
    cf.annots = ()
    if pos + 2 <= len(ub):
        ac = (ub[pos] << 8) | ub[pos + 1]
        pos += 2
        for _ in range(ac):
            an, al = struct.unpack_from('>HI', ub, pos)
            pos += 6
            aname = vals[an]
            if aname == 'BootstrapMethods':
                nb = (ub[pos] << 8) | ub[pos + 1]
                p2 = pos + 2
                for _ in range(nb):
                    mh, na = struct.unpack_from('>HH', ub, p2)
                    p2 += 4
                    args = struct.unpack_from('>%dH' % na, ub, p2)
                    p2 += 2 * na
                    cf.bsm.append((mh, args))
            elif aname == 'RuntimeVisibleAnnotations':
                try:
                    cf.annots = tuple(_annot_types(ub, pos, vals))
                except (IndexError, CFError, TypeError):
                    pass
            pos += al
    return cf


# Instruction lengths, where 0 means variable (tableswitch, lookupswitch, wide).
OPLEN = [1] * 256
for _op in (16, 18, 21, 22, 23, 24, 25, 54, 55, 56, 57, 58, 169, 188):
    OPLEN[_op] = 2
for _op in [17, 19, 20, 132, 178, 179, 180, 181, 182, 183, 184, 187, 189, 192, 193, 198, 199] + list(range(153, 169)):
    OPLEN[_op] = 3
for _op in (185, 186, 200, 201):
    OPLEN[_op] = 5
OPLEN[197] = 4
OPLEN[170] = OPLEN[171] = OPLEN[196] = 0
# Kind of pool operand: 1 = u2 ref, 3 = ldc u1, 4 = ldc_w u2
REFKIND = [0] * 256
for _op in (178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 189, 192, 193, 197):
    REFKIND[_op] = 1
REFKIND[18] = 3
REFKIND[19] = 4


def insn_len(code, pc):
    op = code[pc]
    ln = OPLEN[op]
    if ln:
        return ln
    if op == 196:
        return 6 if code[pc + 1] == 132 else 4
    pad = (4 - ((pc + 1) & 3)) & 3
    base = pc + 1 + pad
    if op == 170:
        low, high = struct.unpack_from('>ii', code, base + 4)
        return 1 + pad + 12 + 4 * (high - low + 1)
    npairs = struct.unpack_from('>i', code, base + 4)[0]
    return 1 + pad + 8 + 8 * npairs


_PARAMS_CACHE = {}


def method_params(desc):
    r = _PARAMS_CACHE.get(desc)
    if r is not None:
        return r
    out = []
    i = 1
    try:
        while desc[i] != ')':
            j = i
            while desc[j] == '[':
                j += 1
            if desc[j] == 'L':
                j = desc.index(';', j)
            out.append(desc[i:j + 1])
            i = j + 1
        r = (tuple(out), desc[i + 1:])
    except (IndexError, ValueError):
        r = ((), 'V')
    if len(_PARAMS_CACHE) < 200000:
        _PARAMS_CACHE[desc] = r
    return r


_DESC_RE = re.compile(r'L([^;<>]+);')


def desc_classes(desc):
    return _DESC_RE.findall(desc)


def elem_class(name):
    """Class name of a CONSTANT_Class entry; array descriptors give their element class or None."""
    if name and name[0] == '[':
        m = _DESC_RE.search(name)
        return m.group(1) if m else None
    return name


# --------------------------------------------------------------------------------------------
# Version tags
# --------------------------------------------------------------------------------------------

_TAG_RE = re.compile(r'(?<![0-9])(v|V)?1_([0-9]{1,2})(?:_([0-9]{1,2}))?(?:_[Rr]([0-9]{1,2}))?(?![0-9])')
# Mojang version -> Spigot revision
_MOJ2R = {
    (8, 0): 1, (8, 1): 1, (8, 2): 1, (8, 3): 2, (8, 4): 3, (8, 5): 3, (8, 6): 3, (8, 7): 3, (8, 8): 3, (8, 9): 3,
    (9, 0): 1, (9, 1): 1, (9, 2): 1, (9, 3): 2, (9, 4): 2, (10, 0): 1, (10, 1): 1, (10, 2): 1,
    (11, 0): 1, (11, 1): 1, (11, 2): 1, (12, 0): 1, (12, 1): 1, (12, 2): 1,
    (13, 0): 1, (13, 1): 2, (13, 2): 2, (14, 0): 1, (14, 1): 1, (14, 2): 1, (14, 3): 1, (14, 4): 1,
    (15, 0): 1, (15, 1): 1, (15, 2): 1, (16, 0): 1, (16, 1): 1, (16, 2): 2, (16, 3): 2, (16, 4): 3, (16, 5): 3,
    (17, 0): 1, (17, 1): 1, (18, 0): 1, (18, 1): 1, (18, 2): 2, (19, 0): 1, (19, 1): 1, (19, 2): 1, (19, 3): 2,
    (19, 4): 3, (20, 0): 1, (20, 1): 1, (20, 2): 2, (20, 3): 3, (20, 4): 3, (20, 5): 4, (20, 6): 4,
    (21, 0): 1, (21, 1): 1, (21, 2): 2, (21, 3): 2, (21, 4): 3, (21, 5): 4, (21, 6): 5, (21, 7): 5, (21, 8): 5,
    (21, 9): 6, (21, 10): 6, (21, 11): 7,
}


def tags_in(name):
    """Version tags in a class or package name: list of (as_written, spigot_equivalent_or_None)."""
    out = []
    for m in _TAG_RE.finditer(name):
        v, minor, patch, rev = m.group(1), int(m.group(2)), m.group(3), m.group(4)
        if minor < 7 or minor > 30:
            continue
        written = m.group(0)
        if rev is not None:
            spig = 'v1_%d_R%s' % (minor, rev)
        else:
            p = int(patch) if patch is not None else 0
            if patch is not None and int(patch) > 12:
                continue
            r = _MOJ2R.get((minor, p))
            spig = ('v1_%d_R%d' % (minor, r)) if r else None
            if patch is None and v is None:
                # bare "1_20" with no v and no patch is too weak a signal in a name
                if not re.search(r'(?:^|[/_$.A-Za-z])1_%d(?:$|[/$._])' % minor, name):
                    continue
        out.append((written, spig))
    return out


_PRE_RE = re.compile(r'(?:pre|legacy|old|before|below|until|upto|lt|max)_?v?$', re.I)


def class_version(name):
    """Version label of a class from its name, and whether the class would run on Spigot 1.20.1.

    Exact adapters carry a Spigot revision (v1_20_R1) or a version that forms a whole package segment
    (nms/v1_20_1/). Other version names in a class name are thresholds: Title_1_17 is for 1.17 and later,
    JoinListenerPre1_21 for versions before 1.21."""
    for m in _TAG_RE.finditer(name):
        v, minor, patch, rev = m.group(1), int(m.group(2)), m.group(3), m.group(4)
        if minor < 7 or minor > 30 or (patch is not None and int(patch) > 12):
            continue
        written = m.group(0)
        start, end = m.start(), m.end()
        seg_start = name.rfind('/', 0, start) + 1
        seg_end = name.find('/', end)
        is_pkg_segment = seg_end != -1
        seg = name[seg_start:seg_end if seg_end != -1 else len(name)]
        p = int(patch) if patch is not None else (1 if minor == 20 else 0)
        if rev is not None:
            spig = 'v1_%d_R%s' % (minor, rev)
            return spig, True, spig == 'v1_20_R1'
        r = _MOJ2R.get((minor, int(patch) if patch is not None else 0))
        spig = ('v1_%d_R%d' % (minor, r)) if r else None
        whole = seg == written or seg.endswith('_' + written) or seg.startswith(written + '_') or seg == 'v' + written
        if is_pkg_segment and whole:
            label = spig or written
            return label, True, spig == 'v1_20_R1'
        if patch is None and v is None and not re.search(r'(?:^|[/_$.A-Za-z])1_%d(?:$|[/$._])' % minor, name):
            continue
        before = name[seg_start:start]
        ver = (minor, p)
        if _PRE_RE.search(before):
            return '<1.%d%s' % (minor, ('.' + patch) if patch else ''), False, (20, 1) < ver
        return '>=1.%d%s' % (minor, ('.' + patch) if patch else ''), False, ver <= (20, 1)
    # year-style Minecraft versions (26.1 and later) written as v26_1 in a package or class name
    m = _YEAR_TAG_RE.search(name)
    if m:
        return 'v%s_%s%s' % (m.group(1), m.group(2), ('_' + m.group(3)) if m.group(3) else ''), True, False
    return '', False, True


_YEAR_TAG_RE = re.compile(r'(?<![0-9A-Za-z])[vV](2[5-9]|3[0-9])_([0-9]{1,2})(?:_([0-9]{1,2}))?(?![0-9])')


# --------------------------------------------------------------------------------------------
# Reference tables
# --------------------------------------------------------------------------------------------

SINK_LOOKUP = {
    'java/lang/Class': {'forName', 'getDeclaredField', 'getField', 'getDeclaredMethod', 'getMethod',
                        'getDeclaredConstructor', 'getConstructor', 'getDeclaredFields', 'getFields',
                        'getDeclaredMethods', 'getMethods', 'getDeclaredConstructors', 'getConstructors',
                        'getDeclaredClasses', 'getClasses', 'getRecordComponents'},
    'java/lang/ClassLoader': {'loadClass', 'findClass', 'defineClass', 'findLoadedClass'},
    'java/net/URLClassLoader': {'loadClass', 'findClass', 'addURL'},
    'java/lang/invoke/MethodHandles$Lookup': {'findVirtual', 'findStatic', 'findSpecial', 'findConstructor',
                                              'findGetter', 'findSetter', 'findStaticGetter', 'findStaticSetter',
                                              'findVarHandle', 'findStaticVarHandle', 'findClass', 'unreflect',
                                              'unreflectGetter', 'unreflectSetter', 'unreflectSpecial',
                                              'unreflectConstructor', 'unreflectVarHandle', 'defineClass',
                                              'defineHiddenClass', 'in'},
    'java/lang/invoke/MethodHandles': {'privateLookupIn'},
}
CORE_SINKS = {('java/lang/Class', x) for x in ('forName', 'getDeclaredField', 'getField', 'getDeclaredMethod',
                                                 'getMethod', 'getDeclaredConstructor')}
CORE_SINKS |= {('java/lang/invoke/MethodHandles$Lookup', x) for x in SINK_LOOKUP['java/lang/invoke/MethodHandles$Lookup']
               if x.startswith('find')}
SINK_USE = {
    'java/lang/reflect/Field': {'get', 'set', 'getInt', 'setInt', 'getLong', 'setLong', 'getBoolean', 'setBoolean',
                                'getDouble', 'setDouble', 'getFloat', 'setFloat', 'getShort', 'setShort', 'getByte',
                                'setByte', 'getChar', 'setChar'},
    'java/lang/reflect/Method': {'invoke'},
    'java/lang/reflect/Constructor': {'newInstance'},
    'java/lang/invoke/MethodHandle': {'invoke', 'invokeExact', 'invokeWithArguments'},
}
LIB_SINK_OWNERS = {'org/apache/commons/lang3/reflect/FieldUtils', 'org/apache/commons/lang3/reflect/MethodUtils',
                   'org/apache/commons/lang3/reflect/ConstructorUtils', 'org/apache/commons/lang/reflect/FieldUtils',
                   'org/apache/commons/lang/reflect/MethodUtils', 'org/joor/Reflect'}
EVENT_SINKS = {('org/bukkit/plugin/PluginManager', 'registerEvent'),
               ('org/bukkit/plugin/SimplePluginManager', 'registerEvent')}
VERSRC = {('java/lang/Class', 'getPackage'), ('java/lang/Package', 'getName'), ('java/lang/Class', 'getName'),
          ('java/lang/Class', 'getSimpleName'), ('java/lang/Class', 'getCanonicalName'),
          ('java/lang/Class', 'getPackageName'),
          ('org/bukkit/Bukkit', 'getBukkitVersion'), ('org/bukkit/Bukkit', 'getVersion'),
          ('org/bukkit/Server', 'getBukkitVersion'), ('org/bukkit/Server', 'getVersion')}
INTERESTING_DESCS = {'Ljava/lang/String;', 'Ljava/lang/Class;', 'Ljava/lang/reflect/Field;',
                     'Ljava/lang/reflect/Method;', 'Ljava/lang/reflect/Constructor;',
                     'Ljava/lang/invoke/MethodHandle;', 'Ljava/lang/invoke/VarHandle;', '[Ljava/lang/String;',
                     '[Ljava/lang/Class;'}
REFLECT_DESCS = {'Ljava/lang/Class;', 'Ljava/lang/reflect/Field;', 'Ljava/lang/reflect/Method;',
                 'Ljava/lang/reflect/Constructor;', 'Ljava/lang/invoke/MethodHandle;', 'Ljava/lang/invoke/VarHandle;',
                 '[Ljava/lang/Class;'}

PAPER_PREFIXES = [('io/papermc/', 'io/papermc/'), ('com/destroystokyo/paper/', 'com/destroystokyo/paper/'),
                  ('net/kyori/adventure/', 'net/kyori/adventure/'), ('org/spigotmc/', 'org/spigotmc/'),
                  ('net/md_5/bungee/api/chat/', 'net/md_5/bungee/api/chat/')]
PATCH_PREFIXES = [('java/lang/instrument/', 'java/lang/instrument/'), ('sun/misc/Unsafe', 'sun/misc/Unsafe'),
                  ('jdk/internal/', 'jdk/internal/'), ('org/objectweb/asm/', 'org/objectweb/asm/'),
                  ('net/bytebuddy/', 'net/bytebuddy/'), ('javassist/', 'javassist/'),
                  ('org/spongepowered/asm/mixin/', 'org/spongepowered/asm/mixin/'),
                  ('com/sun/tools/attach/', 'com/sun/tools/attach/'),
                  ('java/lang/invoke/MethodHandles$Lookup.defineClass', None)]
LIB_DEP_PREFIXES = ['de/tr7zw/', 'com/cryptomorin/xseries/', 'com/github/retrooper/packetevents/',
                    'io/github/retrooper/', 'com/comphenix/protocol/', 'net/kyori/adventure/platform/bukkit/',
                    'org/bstats/']

# Shaded libraries. Markers are class paths relative to the library root. '!' marks a marker that is
# distinctive enough to fix a root alone, and '?' marks a generic name that only counts toward the rule that
# two markers under one root fix that root. A plain marker found alone covers only that class and its
# inner classes (a copied XMaterial, a copied FastBoard).
LIBS = [
    ('nbt-api', ['de/tr7zw/changeme/nbtapi/', 'de/tr7zw/nbtapi/', 'de/tr7zw/itemnbtapi/'],
     ['!NBTReflectionUtil', 'NBTItem', 'NBTCompound', 'utils/MinecraftVersion', '!utils/nmsmappings/ClassWrapper',
      'NBTContainer', '?NBT']),
    ('xseries', ['com/cryptomorin/xseries/'],
     ['XMaterial', 'XSound', 'XEnchantment', 'XPotion', 'XBlock', 'XItemStack', '!reflection/XReflection',
      'XEntityType', 'XAttribute', 'XBiome', 'XTag', 'particles/XParticle', 'profiles/builder/XSkull',
      '?messages/ActionBar', '?messages/Titles', '?SkullUtils', '?ReflectionUtils', '?XEntity']),
    ('packetevents-api', ['com/github/retrooper/packetevents/'],
     ['!PacketEventsAPI', 'PacketEvents', '?event/PacketListener', 'protocol/packettype/PacketType',
      'wrapper/PacketWrapper', 'manager/server/ServerVersion']),
    ('packetevents-spigot', ['io/github/retrooper/packetevents/'],
     ['!factory/spigot/SpigotPacketEventsBuilder', 'bukkit/InternalBukkitListener',
      '!injector/SpigotChannelInjector', '!util/SpigotReflectionUtil', 'util/SpigotConversionUtil']),
    ('protocollib', ['com/comphenix/protocol/'],
     ['!ProtocolLibrary', '?ProtocolManager', 'events/PacketAdapter', '!utility/MinecraftReflection',
      'events/PacketContainer', '?PacketType']),
    ('adventure-platform-bukkit', ['net/kyori/adventure/platform/bukkit/'],
     ['!BukkitAudiences', '!MinecraftComponentSerializer', 'BukkitComponentSerializer', '?MinecraftReflection',
      'CraftBukkitFacet', 'BukkitFacet']),
    ('adventure-platform-facet', ['net/kyori/adventure/platform/facet/'],
     ['?Facet', '!FacetAudience', '?Knob', 'FacetPointers']),
    ('adventure-text-minimessage', ['net/kyori/adventure/text/minimessage/'], ['MiniMessage', '?tag/Tag', 'MiniMessageImpl']),
    ('adventure-text-serializer-gson', ['net/kyori/adventure/text/serializer/gson/'],
     ['GsonComponentSerializer', 'GsonComponentSerializerImpl']),
    ('adventure-text-serializer-legacy', ['net/kyori/adventure/text/serializer/legacy/'],
     ['LegacyComponentSerializer', 'LegacyComponentSerializerImpl']),
    ('adventure-text-serializer-bungeecord', ['net/kyori/adventure/text/serializer/bungeecord/'],
     ['BungeeComponentSerializer', '?GsonInjections']),
    ('adventure-api', ['net/kyori/adventure/'],
     ['text/Component', 'audience/Audience', '?key/Key', 'text/format/NamedTextColor', '!text/TextComponentImpl',
      'text/TextComponent', '?sound/Sound', '?bossbar/BossBar']),
    ('bstats', ['org/bstats/'], ['bukkit/Metrics', 'bukkit/MetricsLite', 'charts/CustomChart', '!MetricsBase',
                                 'charts/SimplePie', 'json/JsonObjectBuilder']),
    ('anvilgui', ['net/wesjd/anvilgui/'], ['AnvilGUI', 'version/VersionWrapper', 'version/VersionMatcher']),
    ('fastboard', ['fr/mrmicky/fastboard/'], ['FastBoard', 'FastReflection', 'FastBoardBase']),
    ('inventoryframework', ['com/github/stefvanschie/inventoryframework/'],
     ['gui/type/ChestGui', 'gui/GuiItem', 'pane/StaticPane', 'pane/PaginatedPane']),
    ('commandapi', ['dev/jorel/commandapi/'], ['CommandAPI', '!CommandAPIBukkit', 'CommandAPICommand']),
    ('acf', ['co/aikar/commands/'], ['BukkitCommandManager', '!ACFBukkitUtil', '?BaseCommand', 'PaperCommandManager']),
    ('aikar-timings', ['co/aikar/timings/lib/'], ['TimingManager', 'MCTiming']),
    ('commodore', ['me/lucko/commodore/'], ['Commodore', '!CommodoreProvider']),
    ('cloud', ['cloud/commandframework/', 'org/incendo/cloud/'],
     ['!bukkit/CloudBukkitCapabilities', '!execution/ExecutionCoordinator', 'arguments/CommandArgument',
      'bukkit/BukkitCommandManager', 'paper/PaperCommandManager', '?CommandManager']),
    ('lamp', ['revxrsal/commands/'], ['bukkit/BukkitCommandHandler', '?Lamp', '?CommandHandler', '!bukkit/BukkitLamp']),
    ('triumph-cmd', ['dev/triumphteam/cmd/'], ['bukkit/BukkitCommandManager', 'core/BaseCommand']),
    ('matt-framework', ['me/mattstudios/mf/'], ['base/CommandManager', 'base/CommandBase']),
    ('paperlib', ['io/papermc/lib/'], ['PaperLib', 'environments/Environment', 'environments/PaperEnvironment']),
    ('folialib', ['com/tcoded/folialib/'], ['!FoliaLib']),
    ('universalscheduler', ['com/github/Anon8281/universalScheduler/'], ['!UniversalScheduler']),
    ('morepaperlib', ['space/arim/morepaperlib/'], ['!MorePaperLib']),
    ('multilib', ['com/github/puregero/multilib/'], ['!MultiLib']),
    ('invui', ['xyz/xenondevs/invui/'], ['!InvUI', '?gui/Gui', '?window/Window']),
    ('inventoryaccess', ['xyz/xenondevs/inventoryaccess/'], ['!InventoryAccess', 'version/InventoryAccessRevision']),
    ('triumph-gui', ['dev/triumphteam/gui/'], ['guis/Gui', 'guis/BaseGui', '!builder/gui/BaseGuiBuilder']),
    ('foundation', ['org/mineacademy/fo/'], ['!remain/Remain', '!plugin/SimplePlugin', 'settings/SimpleSettings',
                                             '?ReflectionUtil', '?MinecraftVersion']),
    ('particlenativeapi', ['com/github/fierioziy/particlenativeapi/'], ['api/ParticleNativeAPI', 'core/ParticleNativeCore']),
    ('reflectionhelper', ['org/inventivetalent/reflection/'], ['?minecraft/Minecraft', 'resolver/ClassResolver']),
    ('jefflib', ['com/jeff_media/jefflib/'], ['!JeffLib', '?ReflUtils']),
    ('nbteditor', ['io/github/bananapuncher714/nbteditor/'], ['NBTEditor']),
    ('scoreboard-library', ['net/megavex/scoreboardlibrary/'], ['api/ScoreboardLibrary', 'implementation/ScoreboardLibraryImpl']),
    ('entitylib', ['me/tofaa/entitylib/'], ['!EntityLib']),
    ('tinyprotocol', ['com/comphenix/tinyprotocol/'], ['TinyProtocol']),
    ('packetwrapper', ['com/comphenix/packetwrapper/'], ['?AbstractPacket', 'WrapperPlayServerSpawnEntity',
                                                        'WrapperPlayServerEntityMetadata', 'WrapperPlayServerChat']),
    ('worldguardwrapper', ['org/codemc/worldguardwrapper/'], ['!WorldGuardWrapper']),
    ('libby', ['net/byteflux/libby/', 'com/alessiodp/libby/'],
     ['?LibraryManager', '?Library', '!classloader/URLClassLoaderHelper', 'BukkitLibraryManager']),
    ('jar-relocator', ['me/lucko/jarrelocator/'], ['!JarRelocator']),
    ('asm', ['org/objectweb/asm/'], ['?ClassReader', '?ClassWriter', '?Opcodes', '?ClassVisitor', '?MethodVisitor']),
    ('bytebuddy', ['net/bytebuddy/'], ['!ByteBuddy', 'agent/ByteBuddyAgent']),
    ('javassist', ['javassist/'], ['?ClassPool', '?CtClass']),
    ('mixin', ['org/spongepowered/asm/mixin/'], ['?Mixins', '?transformer/MixinTransformer']),
    ('authlib', ['com/mojang/authlib/'], ['?GameProfile', '?properties/Property']),
    ('brigadier', ['com/mojang/brigadier/'], ['?CommandDispatcher', '?arguments/StringArgumentType']),
    ('placeholderapi', ['me/clip/placeholderapi/'], ['?PlaceholderAPI', '?expansion/PlaceholderExpansion']),
    ('vault-api', ['net/milkbowl/vault/'], ['?economy/Economy', '?permission/Permission', '?chat/Chat']),
    ('kotlin-stdlib', ['kotlin/'], ['KotlinVersion', '!jvm/internal/Intrinsics', '?Unit']),
]

JDK_PREFIXES = ('java/', 'javax/', 'jdk/', 'sun/', 'com/sun/', 'org/w3c/', 'org/xml/', 'org/ietf/')
NON_JDK_JAVAX = ('javax/annotation/Nonnull', 'javax/annotation/Nullable', 'javax/annotation/CheckForNull',
                 'javax/annotation/concurrent/', 'javax/annotation/meta/', 'javax/inject/', 'javax/persistence/',
                 'javax/mail/', 'javax/activation/', 'javax/servlet/', 'javax/ws/', 'javax/xml/bind/', 'javax/json/')


def is_jdk(name):
    if name.startswith(JDK_PREFIXES):
        if name.startswith('javax/') and name.startswith(NON_JDK_JAVAX):
            return False
        return True
    return False


_DOMAIN_TLDS = {'com', 'org', 'net', 'io', 'me', 'dev', 'de', 'fr', 'it', 'xyz', 'co', 'eu', 'gg', 'uk', 'nl', 'pl',
                'ru', 'cn', 'br', 'es', 'at', 'ch', 'be', 'se', 'dk', 'no', 'fi', 'cz', 'sk', 'hu', 'ro', 'lt', 'lv',
                'us', 'ca', 'au', 'jp', 'kr', 'tw', 'top', 'cc', 'tk', 'pw', 'info', 'one', 'moe', 'club', 'fun',
                'site', 'space', 'games', 'network', 'studio', 'cloud', 'tech', 'app', 'world', 'team', 'ir', 'in',
                'vn', 'tr', 'ua', 'gr', 'pt', 'id', 'ar', 'mx', 'cl', 'ovh', 'gay', 'lol', 'wtf', 'sh', 'is', 'ac',
                'su', 'by', 'kz', 'nz', 'za', 'sg', 'hk', 'my', 'ph', 'th', 'pro', 'host', 'ink', 'dk', 're', 'mc'}


def pkg_group(name):
    segs = name.split('/')[:-1]
    if not segs:
        return '(default)'
    if len(segs) >= 4 and segs[0] in ('org', 'it') and segs[1] in ('apache', 'unimi') and segs[2] in ('commons', 'dsi'):
        return '/'.join(segs[:4]) + '/'
    n = 3 if segs[0] in _DOMAIN_TLDS else 2
    return '/'.join(segs[:n]) + '/'


# --------------------------------------------------------------------------------------------
# API model (spigot-api 1.20.1) and the JDK ancestors of its types
# --------------------------------------------------------------------------------------------

def build_api_model():
    if os.path.exists(API_CACHE) and os.path.getmtime(API_CACHE) > os.path.getmtime(__file__):
        with open(API_CACHE) as f:
            return json.load(f)
    zf = zipfile.ZipFile(SPIGOT_JAR)
    model = {}
    for info in zf.infolist():
        n = info.filename
        if not n.endswith('.class') or n.endswith('module-info.class') or n.endswith('package-info.class'):
            continue
        cf = parse_class(zf.read(info), want_code=False)
        model[cf.name] = {
            'a': cf.access, 's': cf.super, 'i': cf.ifaces,
            'f': {fn + ':' + fd: fa for fa, fn, fd in cf.fields},
            'm': {m.name + m.desc: m.access for m in cf.methods},
        }
    with open(API_CACHE, 'w') as f:
        json.dump(model, f)
    return model


def build_jdk_members(api):
    if os.path.exists(JDK_CACHE) and os.path.getmtime(JDK_CACHE) > os.path.getmtime(__file__):
        with open(JDK_CACHE) as f:
            return json.load(f)
    want = set()
    for c, info in api.items():
        for s in [info['s']] + info['i']:
            if s and s not in api:
                want.add(s)
    want |= {'java/lang/Object', 'java/lang/Enum', 'java/lang/Record', 'java/lang/Throwable'}
    jdk = {}
    todo = sorted(want)
    seen = set()
    while todo:
        batch = [c for c in todo if c not in seen and is_jdk(c)][:80]
        for c in todo:
            seen.add(c)
        todo = []
        if not batch:
            break
        out = subprocess.run(['javap', '-protected', '-s'] + [c.replace('/', '.').replace('$', '$') for c in batch],
                             capture_output=True, text=True).stdout
        cur = None
        pending = None
        for line in out.splitlines():
            if not line.startswith(' ') and ('class ' in line or 'interface ' in line) and line.rstrip().endswith('{'):
                hdr = re.sub(r'<[^<>]*>', '', line)
                hdr = re.sub(r'<[^<>]*>', '', hdr)
                hdr = re.sub(r'<[^<>]*>', '', hdr)
                m = re.search(r'(?:class|interface) ([\w.$]+)', hdr)
                cur = m.group(1).replace('.', '/')
                sup = re.search(r' extends ([\w.$, ]+?)(?: implements|\{)', hdr)
                imp = re.search(r' implements ([\w.$, ]+?)\{', hdr)
                supers = []
                if sup:
                    supers += [x.strip().replace('.', '/') for x in sup.group(1).split(',') if x.strip()]
                if imp:
                    supers += [x.strip().replace('.', '/') for x in imp.group(1).split(',') if x.strip()]
                jdk[cur] = {'super': supers, 'm': [], 'f': []}
                for s in supers:
                    if s not in seen:
                        todo.append(s)
                continue
            if cur is None:
                continue
            st = line.strip()
            if st.startswith('descriptor:') and pending:
                d = st.split(':', 1)[1].strip()
                if d.startswith('('):
                    jdk[cur]['m'].append(pending + d)
                else:
                    jdk[cur]['f'].append(pending + ':' + d)
                pending = None
            elif st.endswith(';') and st != '}':
                decl = st[:-1]
                if '(' in decl:
                    nm = decl[:decl.index('(')].split()[-1]
                    if nm == cur.split('/')[-1].replace('$', '.') or nm.replace('.', '/') == cur:
                        nm = '<init>'
                    pending = nm.split('.')[-1] if nm != '<init>' else nm
                else:
                    pending = decl.split()[-1]
    with open(JDK_CACHE, 'w') as f:
        json.dump(jdk, f)
    return jdk


class API:
    def __init__(self):
        self.model = build_api_model()
        self.jdk = build_jdk_members(self.model)
        self.simple = set(c.split('/')[-1].split('$')[0] for c in self.model)
        self.cache = {}
        # API events: subclasses of org/bukkit/event/Event
        ev = set()
        for c in self.model:
            if self.is_sub(c, 'org/bukkit/event/Event'):
                ev.add(c)
        self.events = ev

    def is_sub(self, c, target):
        seen = set()
        stack = [c]
        while stack:
            x = stack.pop()
            if x == target:
                return True
            if x in seen or x not in self.model:
                continue
            seen.add(x)
            info = self.model[x]
            if info['s']:
                stack.append(info['s'])
            stack.extend(info['i'])
        return False

    def resolve(self, owner, member, is_field):
        """Returns (status, declaring_class, access). status: api, jdk, missing, unknown-owner."""
        key = (owner, member, is_field)
        r = self.cache.get(key)
        if r is not None:
            return r
        if owner not in self.model:
            r = ('unknown-owner', None, 0)
            self.cache[key] = r
            return r
        seen = set()
        queue = [owner]
        jdk_seen = []
        r = None
        while queue:
            c = queue.pop(0)
            if c in seen:
                continue
            seen.add(c)
            info = self.model.get(c)
            if info is None:
                jdk_seen.append(c)
                continue
            tab = info['f'] if is_field else info['m']
            if member in tab:
                r = ('api', c, tab[member])
                break
            if info['s']:
                queue.append(info['s'])
            queue.extend(info['i'])
        if r is None:
            # JDK ancestors, then Object for interfaces
            q2 = jdk_seen + ['java/lang/Object']
            seen2 = set()
            while q2 and r is None:
                c = q2.pop(0)
                if c in seen2:
                    continue
                seen2.add(c)
                info = self.jdk.get(c)
                if info is None:
                    continue
                if member in (info['f'] if is_field else info['m']):
                    r = ('jdk', c, 0)
                    break
                q2.extend(info['super'])
        if r is None:
            r = ('missing', None, 0)
        self.cache[key] = r
        return r


# --------------------------------------------------------------------------------------------
# Values of the stack simulation
# --------------------------------------------------------------------------------------------

class Arr:
    __slots__ = ('t', 'items')

    def __init__(self, t):
        self.t = t
        self.items = {}


class SB:
    __slots__ = ('parts',)

    def __init__(self):
        self.parts = []


class New:
    __slots__ = ('t',)

    def __init__(self, t):
        self.t = t


U = ('u', '?')
NULL = ('n',)
STR_T = 'Ljava/lang/String;'
CLS_T = 'Ljava/lang/Class;'
NOT_HANDLED = object()
SB_CLASSES = {'java/lang/StringBuilder', 'java/lang/StringBuffer'}
_STR_KINDS = {'s', 'cat', 'cn', 'pn', 'der', 'ver', 'fmt'}
_CLS_KINDS = {'c', 'fn', 'rt', 'sup'}


def vtype(v):
    if isinstance(v, tuple):
        k = v[0]
        if k in _STR_KINDS:
            return STR_T
        if k == 'i':
            return 'I'
        if k == 'u':
            return v[1]
        if k == 'r':
            return method_params(v[3])[1]
        if k == 'f':
            return v[3]
        if k == 'a':
            return v[2]
        if k == 'this':
            return 'L' + v[1] + ';'
        if k in _CLS_KINDS:
            return CLS_T
        if k == 'fld':
            return 'Ljava/lang/reflect/Field;'
        if k == 'mth':
            return 'Ljava/lang/reflect/Method;'
        if k == 'ctor':
            return 'Ljava/lang/reflect/Constructor;'
        if k == 'mh':
            return 'Ljava/lang/invoke/MethodHandle;'
        if k == 'pkg':
            return 'Ljava/lang/Package;'
        return 'Ljava/lang/Object;'
    if isinstance(v, SB):
        return 'Ljava/lang/StringBuilder;'
    if isinstance(v, Arr):
        return '[' + v.t
    if isinstance(v, New):
        return 'L' + v.t + ';'
    return '?'


def is_wide(v):
    t = vtype(v)
    return t == 'J' or t == 'D'


def walk(v, depth=0):
    """Yield every sub-value."""
    if depth > 12:
        return
    if isinstance(v, tuple) and not v:
        return
    yield v
    if isinstance(v, tuple):
        for x in v[1:]:
            if isinstance(x, (tuple, Arr, SB)):
                yield from walk(x, depth + 1)
            elif isinstance(x, list):
                for y in x:
                    yield from walk(y, depth + 1)
    elif isinstance(v, Arr):
        for x in list(v.items.values())[:16]:
            yield from walk(x, depth + 1)
    elif isinstance(v, SB):
        for x in v.parts[:32]:
            yield from walk(x, depth + 1)


def has_arg(v):
    for x in walk(v):
        if isinstance(x, tuple) and (x[0] == 'a' or x[0] == 'this'):
            return True
    return False


def is_info(v):
    for x in walk(v):
        if isinstance(x, tuple):
            k = x[0]
            if k == 's' and x[1]:
                return True
            if k in ('c', 'ver', 'fld', 'mth', 'ctor', 'mh', 'pn', 'pkg'):
                return True
            if k == 'rt' and x[1].startswith('Lorg/bukkit/'):
                return True
    return False


def subst(v, env, depth=0):
    if depth > 14:
        return U
    if isinstance(v, tuple):
        if not v:
            return v
        k = v[0]
        if k == 'a':
            r = env.get(v[1])
            return r if r is not None else U
        if k == 'this':
            r = env.get('this')
            return r if r is not None else v
        if len(v) == 1:
            return v
        out = [k]
        for x in v[1:]:
            if isinstance(x, (tuple, Arr, SB)):
                out.append(subst(x, env, depth + 1))
            else:
                out.append(x)
        return tuple(out)
    if isinstance(v, Arr):
        a = Arr(v.t)
        for i, x in v.items.items():
            a.items[i] = subst(x, env, depth + 1)
        return a
    if isinstance(v, SB):
        s = SB()
        s.parts = [subst(x, env, depth + 1) for x in v.parts]
        return s
    return v


def freeze(v, depth=0):
    """Turn mutable SB into cat so a stored value no longer changes."""
    if depth > 14:
        return U
    if isinstance(v, SB):
        return ('cat', tuple(freeze(x, depth + 1) for x in v.parts))
    if isinstance(v, tuple) and len(v) > 1:
        return tuple([v[0]] + [freeze(x, depth + 1) if isinstance(x, (tuple, SB, Arr)) and (not isinstance(x, tuple) or x) else x
                               for x in v[1:]])
    if isinstance(v, Arr):
        a = Arr(v.t)
        a.items = {i: freeze(x, depth + 1) for i, x in v.items.items()}
        return a
    return v


def _short(owner):
    return owner.rsplit('/', 1)[-1]


def _dot(name):
    return name.replace('/', '.')


def _typename(t):
    dims = 0
    while t.startswith('['):
        dims += 1
        t = t[1:]
    prim = {'I': 'int', 'J': 'long', 'Z': 'boolean', 'B': 'byte', 'C': 'char', 'S': 'short', 'F': 'float',
            'D': 'double', 'V': 'void'}
    if t in prim:
        base = prim[t]
    elif t.startswith('L') and t.endswith(';'):
        base = _dot(t[1:-1])
    else:
        base = _dot(t)
    return base + '[]' * dims


def server_derived(v):
    """True when the value comes from the runtime class or package of an org.bukkit object, or the Bukkit version."""
    for x in walk(v):
        if isinstance(x, tuple):
            if x[0] == 'ver':
                return True
            if x[0] == 'rt' and x[1].startswith('Lorg/bukkit/'):
                return True
    return False


def render(v, depth=0):
    if depth > 10:
        return '{..}'
    if isinstance(v, SB):
        return ''.join(render(x, depth + 1) for x in v.parts)
    if isinstance(v, Arr):
        items = [v.items[i] for i in sorted(v.items) if isinstance(i, int)]
        return '[' + ', '.join(render(x, depth + 1) for x in items[:12]) + ']'
    if isinstance(v, New):
        return '{new %s}' % _short(v.t)
    if not isinstance(v, tuple):
        return '{?}'
    k = v[0]
    if k == 's':
        return v[1]
    if k == 'i':
        return str(v[1])
    if k == 'n':
        return 'null'
    if k == 'cat':
        return ''.join(render(x, depth + 1) for x in v[1])
    if k == 'c':
        return _typename(v[1]) if (v[1].startswith('[') or len(v[1]) == 1) else _dot(v[1])
    if k == 'fn':
        return render(v[1], depth + 1)
    if k == 'rt':
        t = v[1]
        return '<class of %s>' % (_typename(t) if t != '?' else '?')
    if k == 'sup':
        return '<superclass of %s>' % render(v[1], depth + 1)
    if k == 'a':
        return '{arg%d}' % v[1]
    if k == 'this':
        return '{this}'
    if k == 'f':
        return '{%s.%s}' % (_short(v[1]), v[2])
    if k == 'r':
        return '{%s.%s()}' % (_short(v[1]), v[2])
    if k == 'u':
        return '{?}'
    if k == 'ver':
        return '{%s.%s()}' % (_short(v[1]), v[2])
    if k == 'pkg':
        return '<package of %s>' % render(v[1], depth + 1)
    if k == 'pn':
        if isinstance(v[1], tuple) and v[1][0] == 'rt' and v[1][1] == 'Lorg/bukkit/Server;':
            return '{pkg}'
        return '{package of %s}' % render(v[1], depth + 1)
    if k == 'cn':
        if isinstance(v[1], tuple) and v[1][0] == 'rt' and v[1][1] == 'Lorg/bukkit/Server;' and v[2] == 'getName':
            return '{pkg}.CraftServer'
        if isinstance(v[1], tuple) and v[1][0] == 'c':
            n = v[1][1]
            if v[2] == 'getSimpleName':
                return _short(n).split('$')[-1]
            return _dot(n)
        return '{%s of %s}' % (v[2], render(v[1], depth + 1))
    if k == 'der':
        if server_derived(v[1]):
            return '{ver}'
        return '{%s(%s)}' % (v[2], render(v[1], depth + 1)[:80])
    if k == 'fmt':
        f = v[1]
        if isinstance(f, tuple) and f[0] == 's':
            args = v[2]
            items = []
            if isinstance(args, Arr):
                items = [args.items.get(i, U) for i in range(len(args.items))]
            parts = re.split(r'(%[-#+ 0,(]*\d*(?:\.\d+)?[sdxXfboc])', f[1])
            out = []
            ai = 0
            for p in parts:
                if p.startswith('%') and len(p) > 1:
                    out.append(render(items[ai], depth + 1) if ai < len(items) else '{?}')
                    ai += 1
                else:
                    out.append(p)
            return ''.join(out)
        return '{format}'
    if k == 'alt':
        rs = []
        for x in v[1]:
            r = render(x, depth + 1)
            if r not in rs:
                rs.append(r)
        return rs[0] if len(rs) == 1 else '(' + '|'.join(rs) + ')'
    if k == 'elem':
        a = v[1]
        if isinstance(a, Arr) and a.items:
            return render(('alt', tuple(a.items[i] for i in sorted(a.items, key=str))), depth + 1)
        return '{element of %s}' % render(a, depth + 1)
    if k == 'fld':
        return 'Field %s.%s' % (render(v[1], depth + 1), render(v[2], depth + 1))
    if k == 'mth':
        return 'Method %s.%s%s' % (render(v[1], depth + 1), render(v[2], depth + 1), render_params(v[3]))
    if k == 'ctor':
        return 'Constructor %s%s' % (render(v[1], depth + 1), render_params(v[2]))
    if k == 'mh':
        return 'MethodHandle %s %s.%s' % (v[1], render(v[2], depth + 1), render(v[3], depth + 1) if v[3] else '')
    if k == 'mt':
        return render_mt(v)
    if k == 'lk':
        return '{lookup}'
    return '{?}'


def render_params(p):
    if p is None:
        return '()'
    if isinstance(p, Arr):
        items = [p.items.get(i, U) for i in range(len(p.items))]
        return '(' + ', '.join(render(x) for x in items[:12]) + ')'
    if isinstance(p, tuple) and p[0] == 'mt':
        return render_mt(p)
    return '(' + render(p) + ')'


def render_mt(v):
    args = v[1]
    if not args:
        return '()'
    ret = render(args[0])
    rest = []
    for a in args[1:]:
        if isinstance(a, Arr):
            rest.extend(render(a.items.get(i, U)) for i in range(len(a.items)))
        else:
            rest.append(render(a))
    return '(' + ', '.join(rest) + ')' + ret


def classify_target(s):
    if not s:
        return 'unknown'
    if s.startswith('<class of org.bukkit.') and 'craftbukkit' not in s:
        t = s[len('<class of '):].split('>', 1)[0].replace('.', '/')
        info = _API.model.get(t) if _API is not None else None
        if info is None:
            # nested types print with '.' too
            parts = t.split('/')
            for k in range(len(parts) - 1, 0, -1):
                cand = '/'.join(parts[:k]) + '/' + '$'.join(parts[k:])
                if _API is not None and cand in _API.model:
                    info = _API.model[cand]
                    break
        if info is not None and not (info['a'] & 0x0200) and (info['a'] & 0x0010):
            return 'bukkit-api'      # a final API class: its runtime class is the API class itself
        return 'bukkit-impl'
    if 'craftbukkit' in s or s.startswith('{pkg}'):
        return 'craftbukkit'
    if 'net.minecraft' in s or 'net/minecraft' in s:
        return 'nms'
    if 'com.mojang' in s:
        return 'mojang'
    if s.startswith('org.bukkit.') or s.startswith('org.spigotmc.'):
        return 'bukkit-api'
    if s.startswith(('io.papermc.', 'com.destroystokyo.paper')):
        return 'paper'
    if s.startswith(('java.', 'javax.', 'sun.', 'jdk.', 'com.sun.')):
        return 'jdk'
    if s.startswith('net.kyori.adventure'):
        return 'adventure'
    if s.startswith('{') or s.startswith('<'):
        return 'unknown'
    return 'other'


# --------------------------------------------------------------------------------------------
# Stack simulation
# --------------------------------------------------------------------------------------------

def _known_call(o, n, d, recv, args, ret):
    if o in SB_CLASSES:
        if n == 'append':
            if isinstance(recv, SB):
                recv.parts.append(args[0] if args else U)
                return recv
            return ('u', ret)
        if n == 'toString':
            if isinstance(recv, SB):
                return ('cat', tuple(recv.parts))
            return ('u', STR_T)
        if n == '<init>':
            if isinstance(recv, SB) and args and d in ('(Ljava/lang/String;)V', '(Ljava/lang/CharSequence;)V'):
                recv.parts.append(args[0])
            return None
        if isinstance(recv, SB) and ret.startswith('Ljava/lang/StringBu'):
            if n in ('insert', 'reverse', 'replace', 'delete', 'deleteCharAt'):
                recv.parts.append(('u', STR_T))
            return recv
        return NOT_HANDLED
    if o == 'java/lang/String':
        if n == 'concat':
            return ('cat', (recv, args[0]))
        if n == 'valueOf':
            if d in ('(Ljava/lang/Object;)Ljava/lang/String;', '([C)Ljava/lang/String;'):
                a = args[0]
                return a if vtype(a) == STR_T else ('der', a, 'valueOf', ())
            a = args[0] if args else U
            if isinstance(a, tuple) and a[0] == 'i':
                return ('s', str(a[1]))
            return ('u', STR_T)
        if n == 'format':
            if d.startswith('(Ljava/lang/String;'):
                return ('fmt', args[0], args[1] if len(args) > 1 else None)
            return ('fmt', args[1] if len(args) > 1 else U, args[2] if len(args) > 2 else None)
        if n == 'formatted':
            return ('fmt', recv, args[0] if args else None)
        if n in ('replace', 'replaceAll', 'replaceFirst', 'substring', 'toLowerCase', 'toUpperCase', 'trim', 'strip',
                 'intern', 'split', 'repeat', 'stripLeading', 'stripTrailing', 'join', 'subSequence'):
            if isinstance(recv, tuple) and recv[0] == 's' and all(isinstance(a, tuple) and a[0] in ('s', 'i') for a in args):
                s = recv[1]
                av = [a[1] for a in args]
                try:
                    if n == 'replace' and len(av) == 2 and isinstance(av[0], str):
                        return ('s', s.replace(av[0], av[1]))
                    if n == 'replace' and len(av) == 2:
                        return ('s', s.replace(chr(av[0]), chr(av[1])))
                    if n in ('toLowerCase', 'toUpperCase', 'trim', 'strip', 'intern'):
                        return ('s', {'toLowerCase': s.lower, 'toUpperCase': s.upper, 'trim': s.strip,
                                      'strip': s.strip, 'intern': lambda: s}[n]())
                    if n == 'substring':
                        return ('s', s[av[0]:av[1]] if len(av) == 2 else s[av[0]:])
                    if n == 'split' and len(av) >= 1 and isinstance(av[0], str):
                        a = Arr(STR_T)
                        for i, p in enumerate(re.split(av[0], s)):
                            a.items[i] = ('s', p)
                        return a
                except Exception:
                    pass
            if n == 'split':
                a = Arr(STR_T)
                return ('der', recv, 'split', tuple(args)) if not server_derived(recv) else ('der', recv, 'split', ())
            if n == 'join':
                return ('der', args[0] if args else U, 'join', ())
            return ('der', recv, n, ())
        return NOT_HANDLED
    if n == 'getClass' and d == '()Ljava/lang/Class;':
        if isinstance(recv, tuple) and recv[0] == 'this':
            return ('rt', 'L' + recv[1] + ';')
        return ('rt', vtype(recv))
    if o == 'java/lang/Class':
        if n == 'forName':
            return ('fn', args[0] if args else U)
        if n in ('getName', 'getSimpleName', 'getCanonicalName', 'getTypeName'):
            return ('cn', recv, n)
        if n == 'getPackage':
            return ('pkg', recv)
        if n == 'getPackageName':
            return ('pn', recv)
        if n == 'getSuperclass':
            return ('sup', recv)
        if n in ('getDeclaredField', 'getField'):
            return ('fld', recv, args[0] if args else U, n)
        if n in ('getDeclaredMethod', 'getMethod'):
            return ('mth', recv, args[0] if args else U, args[1] if len(args) > 1 else None, n)
        if n in ('getDeclaredConstructor', 'getConstructor'):
            return ('ctor', recv, args[0] if args else None, n)
        if n == 'cast':
            return args[0] if args else U
        return NOT_HANDLED
    if o == 'java/lang/Package' and n == 'getName':
        if isinstance(recv, tuple) and recv[0] == 'pkg':
            return ('pn', recv[1])
        return ('u', STR_T)
    if n == 'loadClass' and d.startswith('(Ljava/lang/String;'):
        return ('fn', args[0] if args else U)
    if o == 'java/lang/invoke/MethodHandles':
        if n in ('lookup', 'publicLookup'):
            return ('lk',)
        if n == 'privateLookupIn':
            return ('lk', args[0] if args else U)
        return NOT_HANDLED
    if o == 'java/lang/invoke/MethodHandles$Lookup':
        if n == 'findClass':
            return ('fn', args[0] if args else U)
        if n in ('findVirtual', 'findStatic', 'findSpecial', 'findGetter', 'findSetter', 'findStaticGetter',
                 'findStaticSetter', 'findVarHandle', 'findStaticVarHandle'):
            return ('mh', n, args[0] if args else U, args[1] if len(args) > 1 else None,
                    args[2] if len(args) > 2 else None)
        if n == 'findConstructor':
            return ('mh', n, args[0] if args else U, None, args[1] if len(args) > 1 else None)
        if n.startswith('unreflect'):
            return ('mh', n, args[0] if args else U, None, None)
        return NOT_HANDLED
    if o == 'java/lang/invoke/MethodType' and n == 'methodType':
        return ('mt', tuple(args))
    if o == 'java/lang/reflect/Field' and n == 'getName':
        if isinstance(recv, tuple) and recv[0] == 'fld':
            return recv[2]
        return NOT_HANDLED
    if (o == 'org/bukkit/Bukkit' or o == 'org/bukkit/Server') and n in ('getBukkitVersion', 'getVersion'):
        return ('ver', o, n)
    return NOT_HANDLED


def _sink_kind(o, n):
    s = SINK_LOOKUP.get(o)
    if s is not None and n in s:
        return 'lookup'
    s = SINK_USE.get(o)
    if s is not None and n in s:
        return 'use'
    if o in LIB_SINK_OWNERS:
        return 'lib'
    if (o, n) in EVENT_SINKS:
        return 'event'
    return None


class JarCtx:
    """Jar-wide state of the interprocedural rounds."""

    def __init__(self, supers):
        self.supers = supers           # class -> super
        self.summ_prev = {}            # (cls, name, desc) -> {'ret': value, 'tpl': [site]}
        self.fields_prev = {}          # (owner, name, desc) -> [values]
        self.summ_new = {}
        self.fields_new = defaultdict(list)
        self.sites = []                # final-round sites

    def summary(self, owner, name, desc):
        c = owner
        for _ in range(12):
            s = self.summ_prev.get((c, name, desc))
            if s is not None:
                return s
            c = self.supers.get(c)
            if c is None:
                return None
        return None

    def field(self, owner, name, desc):
        vs = self.fields_prev.get((owner, name, desc))
        if vs is None:
            c = self.supers.get(owner)
            for _ in range(8):
                if c is None:
                    break
                vs = self.fields_prev.get((c, name, desc))
                if vs is not None:
                    break
                c = self.supers.get(c)
        if not vs:
            return None
        if len(vs) == 1:
            return vs[0]
        return ('alt', tuple(vs[:4]))


def decode(code):
    insns = []
    n = len(code)
    pc = 0
    while pc < n:
        op = code[pc]
        ln = insn_len(code, pc)
        insns.append((pc, op, ln))
        pc += ln
    return insns


_PRIM_BOX = {'java/lang/Void': 'V', 'java/lang/Boolean': 'Z', 'java/lang/Integer': 'I', 'java/lang/Long': 'J',
             'java/lang/Double': 'D', 'java/lang/Float': 'F', 'java/lang/Short': 'S', 'java/lang/Byte': 'B',
             'java/lang/Character': 'C'}


def join_val(a, b):
    if a is b:
        return a
    if a is None or b is None:
        return a if b is None else b
    if isinstance(a, tuple) and isinstance(b, tuple):
        if a == b:
            return a
        ia, ib = is_info(a), is_info(b)
        if not ia and not ib:
            return a
        if not ib:
            return a
        if not ia:
            return b
        items = list(a[1]) if a[0] == 'alt' else [a]
        for x in (b[1] if b[0] == 'alt' else (b,)):
            if x not in items and len(items) < 4:
                items.append(x)
        return items[0] if len(items) == 1 else ('alt', tuple(items))
    # mutable objects (StringBuilder, arrays): keep the first unless only the second carries information
    if isinstance(a, (SB, Arr, New)):
        return a
    return b if isinstance(b, (SB, Arr)) else a


def join_seq(a, b):
    if len(a) != len(b):
        return tuple(b) if len(b) > len(a) else tuple(a)
    return tuple(join_val(x, y) for x, y in zip(a, b))


def simulate(cf, m, cls_name, ctx, out_sites, budget):
    code = m.code
    insns = decode(code)
    params, ret_t = method_params(m.desc)
    static = bool(m.access & 0x0008)
    nloc = max(m.max_locals, 1)
    loc = [None] * (nloc + 2)
    li = 0
    if not static:
        loc[0] = ('this', cls_name)
        li = 1
    for i, p in enumerate(params):
        if li < len(loc):
            loc[li] = ('a', i, p)
        li += 2 if p in ('J', 'D') else 1
    handlers = {}
    for s, e, h, ct in m.handlers:
        handlers[h] = ct
    states = {}
    stack = []
    dead = False
    rets = []
    tags, vals = cf.tags, cf.vals
    msites = []
    t0 = time.time()

    def pop():
        return stack.pop() if stack else U

    def popn(k):
        if k == 0:
            return []
        if len(stack) >= k:
            r = stack[-k:]
            del stack[-k:]
            return r
        r = [U] * (k - len(stack)) + stack[:]
        del stack[:]
        return r

    def branch(target):
        st = states.get(target)
        if st is None:
            states[target] = (tuple(stack), tuple(loc))
        else:
            states[target] = (join_seq(st[0], stack), join_seq(st[1], loc))

    def const(idx):
        t = tags[idx]
        if t == 8:
            return ('s', vals[vals[idx]])
        if t == 7:
            return ('c', vals[vals[idx]])
        if t == 3:
            return ('i', vals[idx])
        if t == 4:
            return ('u', 'F')
        if t == 16:
            return ('u', 'Ljava/lang/invoke/MethodType;')
        if t == 15:
            return ('u', 'Ljava/lang/invoke/MethodHandle;')
        return U

    count = 0
    for pc, op, ln in insns:
        count += 1
        if (count & 1023) == 0 and time.time() - t0 > budget:
            break
        if dead:
            st = states.get(pc)
            if st is not None:
                stack = list(st[0])
                loc = list(st[1])
            elif pc in handlers:
                stack = [('u', 'Ljava/lang/Throwable;')]
            else:
                stack = []
            dead = False
        elif pc in states:
            st = states[pc]
            stack = list(join_seq(st[0], stack))
            loc = list(join_seq(st[1], loc))
            if pc in handlers:
                stack = [('u', 'Ljava/lang/Throwable;')]
        elif pc in handlers:
            stack = [('u', 'Ljava/lang/Throwable;')]
        if op <= 15:
            if op == 1:
                stack.append(NULL)
            elif 2 <= op <= 8:
                stack.append(('i', op - 3))
            elif op in (9, 10):
                stack.append(('u', 'J'))
            elif op in (11, 12, 13):
                stack.append(('u', 'F'))
            elif op in (14, 15):
                stack.append(('u', 'D'))
        elif op == 16:
            b = code[pc + 1]
            stack.append(('i', b - 256 if b > 127 else b))
        elif op == 17:
            stack.append(('i', struct.unpack_from('>h', code, pc + 1)[0]))
        elif op == 18:
            stack.append(const(code[pc + 1]))
        elif op == 19:
            stack.append(const((code[pc + 1] << 8) | code[pc + 2]))
        elif op == 20:
            idx = (code[pc + 1] << 8) | code[pc + 2]
            stack.append(('u', 'J' if tags[idx] == 5 else 'D'))
        elif 21 <= op <= 45:
            if op <= 25:
                idx = code[pc + 1]
                t = 'IJFDA'[op - 21]
            else:
                idx = (op - 26) & 3
                t = 'IJFDA'[(op - 26) >> 2]
            v = loc[idx] if idx < len(loc) else None
            if v is None:
                v = ('u', {'I': 'I', 'J': 'J', 'F': 'F', 'D': 'D', 'A': 'Ljava/lang/Object;'}[t])
            stack.append(v)
        elif 46 <= op <= 53:
            idxv = pop()
            arr = pop()
            if op == 50:
                if isinstance(arr, Arr):
                    if isinstance(idxv, tuple) and idxv[0] == 'i' and idxv[1] in arr.items:
                        stack.append(arr.items[idxv[1]])
                    else:
                        stack.append(('elem', arr))
                elif isinstance(arr, tuple) and arr[0] == 'a':
                    stack.append(('elem', arr))
                elif isinstance(arr, tuple) and arr[0] == 'der':
                    stack.append(('der', arr, 'element', ()))
                else:
                    t = vtype(arr)
                    stack.append(('u', t[1:] if t.startswith('[') else 'Ljava/lang/Object;'))
            else:
                stack.append(('u', 'IJFDAIII'[op - 46] if op != 50 else 'Ljava/lang/Object;'))
        elif 54 <= op <= 78:
            if op <= 58:
                idx = code[pc + 1]
            else:
                idx = (op - 59) & 3
            v = pop()
            if idx < len(loc):
                loc[idx] = v
            if is_wide(v) and idx + 1 < len(loc):
                loc[idx + 1] = None
        elif 79 <= op <= 86:
            v = pop()
            idxv = pop()
            arr = pop()
            if isinstance(arr, Arr) and isinstance(idxv, tuple) and idxv[0] == 'i':
                if len(arr.items) < 64:
                    arr.items[idxv[1]] = v
        elif op == 87:
            pop()
        elif op == 88:
            if stack and is_wide(stack[-1]):
                pop()
            else:
                pop(); pop()
        elif op == 89:
            v = pop()
            stack.append(v); stack.append(v)
        elif op == 90:
            v1 = pop(); v2 = pop()
            stack.extend((v1, v2, v1))
        elif op == 91:
            v1 = pop()
            if stack and is_wide(stack[-1]):
                v2 = pop()
                stack.extend((v1, v2, v1))
            else:
                v2 = pop(); v3 = pop()
                stack.extend((v1, v3, v2, v1))
        elif op == 92:
            if stack and is_wide(stack[-1]):
                v = pop()
                stack.extend((v, v))
            else:
                v1 = pop(); v2 = pop()
                stack.extend((v2, v1, v2, v1))
        elif op == 93:
            if stack and is_wide(stack[-1]):
                v1 = pop(); v2 = pop()
                stack.extend((v1, v2, v1))
            else:
                v1 = pop(); v2 = pop(); v3 = pop()
                stack.extend((v2, v1, v3, v2, v1))
        elif op == 94:
            v1 = pop()
            if is_wide(v1):
                v2 = pop()
                if is_wide(v2):
                    stack.extend((v1, v2, v1))
                else:
                    v3 = pop()
                    stack.extend((v1, v3, v2, v1))
            else:
                v2 = pop()
                v3 = pop()
                if is_wide(v3):
                    stack.extend((v2, v1, v3, v2, v1))
                else:
                    v4 = pop()
                    stack.extend((v2, v1, v4, v3, v2, v1))
        elif op == 95:
            v1 = pop(); v2 = pop()
            stack.extend((v1, v2))
        elif 96 <= op <= 131:
            if 116 <= op <= 119:
                v = pop()
                stack.append(('u', 'IJFD'[op - 116]))
            else:
                pop(); pop()
                if op <= 115:
                    t = 'IJFD'[(op - 96) & 3]
                elif op <= 125:
                    t = 'IJIJIJ'[op - 120]
                else:
                    t = 'IJ'[(op - 126) & 1]
                stack.append(('u', t))
        elif op == 132:
            idx = code[pc + 1]
            if idx < len(loc):
                loc[idx] = ('u', 'I')
        elif 133 <= op <= 147:
            pop()
            stack.append(('u', 'JFDIFDIJDIJFIII'[op - 133]))
        elif 148 <= op <= 152:
            pop(); pop()
            stack.append(('u', 'I'))
        elif 153 <= op <= 166 or op in (198, 199):
            if 159 <= op <= 166:
                pop(); pop()
            else:
                pop()
            off = struct.unpack_from('>h', code, pc + 1)[0]
            branch(pc + off)
        elif op == 167 or op == 200:
            off = struct.unpack_from('>h', code, pc + 1)[0] if op == 167 else struct.unpack_from('>i', code, pc + 1)[0]
            branch(pc + off)
            dead = True
        elif op == 168 or op == 201:
            off = struct.unpack_from('>h', code, pc + 1)[0] if op == 168 else struct.unpack_from('>i', code, pc + 1)[0]
            stack.append(('u', 'R'))
            branch(pc + off)
            stack.pop()
        elif op == 169:
            dead = True
        elif op == 170 or op == 171:
            pop()
            pad = (4 - ((pc + 1) & 3)) & 3
            base = pc + 1 + pad
            dflt = struct.unpack_from('>i', code, base)[0]
            branch(pc + dflt)
            if op == 170:
                low, high = struct.unpack_from('>ii', code, base + 4)
                for k in range(min(high - low + 1, 4096)):
                    branch(pc + struct.unpack_from('>i', code, base + 12 + 4 * k)[0])
            else:
                npairs = struct.unpack_from('>i', code, base + 4)[0]
                for k in range(min(npairs, 4096)):
                    branch(pc + struct.unpack_from('>i', code, base + 8 + 8 * k + 4)[0])
            dead = True
        elif 172 <= op <= 177:
            if op != 177:
                v = pop()
                if op == 176 and len(rets) < 4:
                    rets.append(v)
            dead = True
        elif 178 <= op <= 181:
            idx = (code[pc + 1] << 8) | code[pc + 2]
            o, nm, d = cf.ref(idx)
            if op == 178:
                if nm == 'TYPE' and o in _PRIM_BOX:
                    stack.append(('c', _PRIM_BOX[o]))
                    continue
                fv = ctx.field(o, nm, d)
                stack.append(fv if fv is not None else ('f', o, nm, d))
            elif op == 179:
                v = pop()
                if d in INTERESTING_DESCS:
                    v = freeze(v)
                    if is_info(v) and not has_arg(v):
                        lst = ctx.fields_new[(o, nm, d)]
                        if len(lst) < 4 and not any(render(x) == render(v) for x in lst):
                            lst.append(v)
            elif op == 180:
                pop()
                fv = ctx.field(o, nm, d)
                stack.append(fv if fv is not None else ('f', o, nm, d))
            else:
                v = pop()
                pop()
                if d in INTERESTING_DESCS:
                    v = freeze(v)
                    if is_info(v) and not has_arg(v):
                        lst = ctx.fields_new[(o, nm, d)]
                        if len(lst) < 4 and not any(render(x) == render(v) for x in lst):
                            lst.append(v)
        elif 182 <= op <= 185:
            idx = (code[pc + 1] << 8) | code[pc + 2]
            o, nm, d = cf.ref(idx)
            ps, rt = method_params(d)
            args = popn(len(ps))
            recv = None if op == 184 else pop()
            res = _known_call(o, nm, d, recv, args, rt)
            kind = _sink_kind(o, nm)
            if kind is not None:
                msites.append((kind, o, nm, d, recv, list(args), None))
            s = None
            if res is NOT_HANDLED or kind is not None:
                s = ctx.summary(o, nm, d)
                if s is not None:
                    env = {i: a for i, a in enumerate(args)}
                    if recv is not None:
                        env['this'] = recv
                    for t in s['tpl']:
                        if len(msites) < 400:
                            k2, o2, n2, d2, r2, a2, via = t
                            msites.append((k2, o2, n2, d2, subst(r2, env) if r2 is not None else None,
                                           [subst(x, env) for x in a2], ((o, nm, d),) + (via or ())))
                    if res is NOT_HANDLED and s['ret'] is not None:
                        res = subst(s['ret'], env)
            if res is NOT_HANDLED:
                res = ('r', o, nm, d) if rt != 'V' else None
            if rt != 'V':
                stack.append(res if res is not None else ('u', rt))
        elif op == 186:
            idx = (code[pc + 1] << 8) | code[pc + 2]
            bi, nt = vals[idx]
            nm, d = vals[vals[nt][0]], vals[vals[nt][1]]
            ps, rt = method_params(d)
            args = popn(len(ps))
            res = None
            if nm in ('makeConcatWithConstants', 'makeConcat') and bi < len(cf.bsm):
                mh, bargs = cf.bsm[bi]
                if nm == 'makeConcat':
                    res = ('cat', tuple(args))
                elif bargs and tags[bargs[0]] == 8:
                    recipe = vals[vals[bargs[0]]]
                    parts = []
                    ai = 0
                    ci = 1
                    buf = []
                    for ch in recipe:
                        if ch == '\x01':
                            if buf:
                                parts.append(('s', ''.join(buf))); buf = []
                            parts.append(args[ai] if ai < len(args) else U)
                            ai += 1
                        elif ch == '\x02':
                            if buf:
                                parts.append(('s', ''.join(buf))); buf = []
                            parts.append(const(bargs[ci]) if ci < len(bargs) else U)
                            ci += 1
                        else:
                            buf.append(ch)
                    if buf:
                        parts.append(('s', ''.join(buf)))
                    res = ('cat', tuple(parts))
            if rt != 'V':
                stack.append(res if res is not None else ('u', rt))
        elif op == 187:
            idx = (code[pc + 1] << 8) | code[pc + 2]
            t = vals[vals[idx]]
            stack.append(SB() if t in SB_CLASSES else New(t))
        elif op == 188:
            pop()
            stack.append(Arr('IJFDBCSZ'[0]))
        elif op == 189:
            pop()
            idx = (code[pc + 1] << 8) | code[pc + 2]
            t = vals[vals[idx]]
            stack.append(Arr(t if t.startswith('[') else 'L' + t + ';'))
        elif op == 190:
            pop()
            stack.append(('u', 'I'))
        elif op == 191:
            pop()
            dead = True
        elif op == 192:
            v = pop()
            idx = (code[pc + 1] << 8) | code[pc + 2]
            t = vals[vals[idx]]
            if isinstance(v, tuple) and v[0] in ('u', 'r', 'f') and not is_info(v):
                v = ('u', t if t.startswith('[') else 'L' + t + ';') if v[0] == 'u' else v
            stack.append(v)
        elif op == 193:
            pop()
            stack.append(('u', 'I'))
        elif op in (194, 195):
            pop()
        elif op == 196:
            op2 = code[pc + 1]
            idx = (code[pc + 2] << 8) | code[pc + 3]
            if 21 <= op2 <= 25:
                v = loc[idx] if idx < len(loc) else None
                stack.append(v if v is not None else ('u', 'IJFDL'[op2 - 21] if op2 != 25 else 'Ljava/lang/Object;'))
            elif 54 <= op2 <= 58:
                v = pop()
                if idx < len(loc):
                    loc[idx] = v
            elif op2 == 132:
                if idx < len(loc):
                    loc[idx] = ('u', 'I')
            elif op2 == 169:
                dead = True
        elif op == 197:
            dims = code[pc + 3]
            popn(dims)
            idx = (code[pc + 1] << 8) | code[pc + 2]
            stack.append(Arr(vals[vals[idx]][1:]))
        if len(stack) > 512:
            del stack[:-64]
    # summary
    tpl = []
    for s in msites:
        if site_has_arg(s[0], s[4], s[5]):
            if len(tpl) < 24:
                tpl.append(tuple([s[0], s[1], s[2], s[3], freeze(s[4]) if s[4] is not None else None,
                                  [freeze(a) for a in s[5]], s[6]]))
    ret = None
    if rets and (ret_t.startswith('L') or ret_t.startswith('[')):
        rs = [freeze(x) for x in rets]
        good = [x for x in rs if is_info(x) or has_arg(x)]
        if not good:
            good = [x for x in rs if isinstance(x, tuple) and x and x[0] in ('fld', 'mth', 'ctor', 'mh', 'fn', 'c')]
        uniq = []
        for x in good:
            if not any(render(x) == render(y) for y in uniq):
                uniq.append(x)
        if uniq:
            ret = uniq[0] if len(uniq) == 1 else ('alt', tuple(uniq[:4]))
    if tpl or ret is not None:
        ctx.summ_new[(cls_name, m.name, m.desc)] = {'ret': ret, 'tpl': tpl}
    out_sites.extend((cls_name, m.name, m.desc, s) for s in msites)
    return count


# --------------------------------------------------------------------------------------------
# Jar reading
# --------------------------------------------------------------------------------------------

class Unit:
    __slots__ = ('origin', 'entry', 'data', 'mr')


def read_units(zf, origin, depth, units, nested, problems, manifests):
    try:
        infos = zf.infolist()
    except Exception as e:
        problems.append('%s: cannot list: %s' % (origin or '(jar)', e))
        return
    for info in infos:
        name = info.filename
        if name.endswith('/'):
            continue
        if info.file_size > MAX_ENTRY:
            problems.append('%s%s: entry over 60 MB skipped' % (origin, name))
            continue
        is_class = name.endswith('.class')
        if is_class:
            base = name.rsplit('/', 1)[-1]
            if base in ('module-info.class', 'package-info.class'):
                continue
            try:
                data = zf.read(info)
            except Exception as e:
                problems.append('%s%s: %s' % (origin, name, str(e)[:80]))
                continue
            u = Unit()
            u.origin = origin
            u.entry = name
            u.data = data
            mr = re.match(r'META-INF/versions/(\d+)/', name)
            u.mr = int(mr.group(1)) if mr else 0
            units.append(u)
            continue
        if name.upper() == 'META-INF/MANIFEST.MF' and info.file_size < 1 << 20:
            try:
                manifests[origin] = zf.read(info).decode('utf-8', 'replace')
            except Exception:
                pass
            continue
        if info.file_size < 8:
            continue
        try:
            with zf.open(info) as f:
                head = f.read(4)
        except Exception:
            continue
        if head == b'PK\x03\x04' and depth < MAX_NEST_DEPTH:
            try:
                data = zf.read(info)
                sub = zipfile.ZipFile(io.BytesIO(data))
            except Exception as e:
                problems.append('%s%s: nested archive unreadable: %s' % (origin, name, str(e)[:80]))
                continue
            rec = {'path': origin + name, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest(),
                   'depth': depth + 1}
            try:
                names = sub.namelist()
                rec['plugin_yml'] = 'plugin.yml' in names
                rec['paper_plugin_yml'] = 'paper-plugin.yml' in names
            except Exception:
                pass
            before = len(units)
            read_units(sub, origin + name + '!/', depth + 1, units, nested, problems, manifests)
            rec['classes'] = len(units) - before
            nested.append(rec)
        elif head == b'\xca\xfe\xba\xbe':
            try:
                data = zf.read(info)
            except Exception:
                continue
            u = Unit()
            u.origin = origin
            u.entry = name
            u.data = data
            u.mr = -1   # hidden class file under another extension
            units.append(u)


# --------------------------------------------------------------------------------------------
# Per-jar scan
# --------------------------------------------------------------------------------------------

class CInfo:
    __slots__ = ('name', 'super', 'ifaces', 'origin', 'entry', 'tag', 'fields', 'mdecl', 'mrefs', 'crefs',
                 'dtypes', 'events', 'major', 'sim', 'unit', 'access', 'strs', 'is_listener_annot', 'exact', 'runs')


_STR_PATTERNS = [
    ('craftbukkit', re.compile(r'craftbukkit', re.I)),
    ('net.minecraft', re.compile(r'net[./]minecraft')),
    ('com.mojang', re.compile(r'com[./]mojang')),
    ('craft-class', re.compile(r'(?<![A-Za-z0-9])Craft[A-Z][A-Za-z0-9_]*')),
    ('version-tag', re.compile(r'(?<![0-9])v?1_[0-9]{1,2}_R[0-9]{1,2}(?![0-9])|(?<![0-9A-Za-z])v1_[0-9]{1,2}(?:_[0-9]{1,2})?(?![0-9])')),
]


def match_string(s, api_simple):
    hits = []
    for name, rx in _STR_PATTERNS:
        if name == 'craft-class':
            for m in rx.finditer(s):
                tok = m.group(0)
                if tok in api_simple or tok.startswith('Crafting'):
                    continue
                hits.append('craft-class:' + tok)
        elif rx.search(s):
            if name == 'version-tag':
                hits.extend('version-tag:' + m.group(0) for m in rx.finditer(s))
            else:
                hits.append(name)
    return hits


def scan_jar(p, api):
    t_start = time.time()
    out = {
        'jar': os.path.basename(p['path']), 'source': p.get('source'), 'id': p.get('id'), 'name': p.get('name'),
        'version': p.get('version'), 'plugin_name': p.get('plugin_name'), 'plugin_version': p.get('plugin_version'),
        'main': p.get('main'), 'descriptor': p.get('descriptor'), 'api_version': p.get('api_version'),
        'sha256': p.get('sha256'), 'bytes': p.get('bytes'), 'downloads': p.get('downloads'),
        'folia_supported': p.get('folia_supported'), 'plugin_yml_libraries': p.get('libraries') or [],
        'depend': p.get('depend') or [], 'softdepend': p.get('softdepend') or [],
    }
    path = os.path.realpath(p['path'])
    if not path.startswith(JARS_DIR + os.sep):
        out['error'] = 'path outside the jars directory'
        return out
    if os.path.getsize(path) > MAX_JAR:
        out['error'] = 'jar over 60 MB skipped'
        return out
    with open(path, 'rb') as f:
        raw = f.read()
    if hashlib.sha256(raw).hexdigest() != p.get('sha256'):
        out['sha256_mismatch'] = True
    units, nested, problems, manifests = [], [], [], {}
    try:
        zf = zipfile.ZipFile(io.BytesIO(raw))
    except Exception as e:
        out['error'] = 'not a zip: %s' % e
        return out
    import warnings
    with warnings.catch_warnings(record=True) as wlist:
        warnings.simplefilter('always')
        read_units(zf, '', 0, units, nested, problems, manifests)
    wc = Counter(str(w.message).split(':', 1)[0] for w in wlist)
    for k, n in wc.items():
        problems.append('zipfile warning x%d: %s' % (n, k))
    del raw

    # ---------------- pass A: parse every class
    infos = {}
    parse_errors = Counter()
    error_examples = []
    hidden = 0
    sink_methods = []        # (cls, method, desc, sinks, strings, classes)
    jar_strings = {}         # string -> [hits, set(classes)]
    sim_units = []           # (CInfo, ClassFile-or-None)
    const_field_vals = {}
    mr_dupes = 0
    not_class = []
    for u in units:
        try:
            cf = parse_class(u.data)
        except Exception as e:
            if u.mr == -1:
                # 0xCAFEBABE under another extension that is no class file: a Mach-O universal binary
                if len(not_class) < 10:
                    not_class.append(u.origin + u.entry)
                continue
            parse_errors[type(e).__name__] += 1
            if len(error_examples) < 5:
                error_examples.append('%s%s: %s' % (u.origin, u.entry, str(e)[:80]))
            continue
        if u.mr == -1:
            hidden += 1
        if cf.name in infos:
            # multi-release copy or duplicate in a nested jar: keep the first, count the copy
            mr_dupes += 1
            existing = infos[cf.name]
            if u.mr > 0 and existing.origin == u.origin:
                continue
            if u.origin == existing.origin:
                continue
            # a different nested jar defines the same class: scan it under a distinct key
            key = u.origin + cf.name
        else:
            key = cf.name
        ci = analyze_class(cf, u, api, sink_methods, jar_strings, const_field_vals)
        infos[key] = ci
    out['scan'] = {
        'entries_class': len(units) - len(not_class), 'classes': len(infos), 'hidden_class_files': hidden,
        'cafebabe_not_class': not_class,
        'parse_errors': dict(parse_errors), 'parse_error_examples': error_examples,
        'duplicate_class_copies': mr_dupes, 'nested_jars': nested, 'problems': problems[:20],
    }
    # manifest
    man = {}
    for origin, text in manifests.items():
        for line in re.split(r'\r?\n(?! )', text):
            if ':' in line:
                k, v = line.split(':', 1)
                k = k.strip()
                if k in ('Main-Class', 'Agent-Class', 'Premain-Class', 'Launcher-Agent-Class', 'Can-Redefine-Classes',
                         'Can-Retransform-Classes', 'Multi-Release', 'paperweight-mappings-namespace', 'Add-Opens',
                         'Add-Exports', 'Enable-Native-Access', 'Created-By'):
                    man.setdefault(k, {})[origin or '(jar)'] = v.replace('\r\n ', '').replace('\n ', '').strip()
    out['manifest'] = man

    defined = set()
    for key, ci in infos.items():
        defined.add(ci.name)
    supers = {ci.name: ci.super for ci in infos.values()}

    # ---------------- library detection
    lib_inst = detect_libs(infos, jar_strings)
    lib_roots = sorted(lib_inst.items(), key=lambda kv: -len(kv[0]))
    lib_cache = {}

    def lib_of(cname):
        r = lib_cache.get(cname)
        if r is not None:
            return r if r else None
        res = ''
        for root, inst in lib_roots:
            if inst['single']:
                if cname == root or cname.startswith(root + '$'):
                    res = root
                    break
            elif cname.startswith(root):
                res = root
                break
        lib_cache[cname] = res
        return res or None

    main = (p.get('main') or '').replace('.', '/')
    main_pkg = main.rsplit('/', 1)[0] + '/' if '/' in main else ''
    _segs = [x for x in main_pkg.split('/') if x]
    if _segs and _segs[0] in _DOMAIN_TLDS and len(_segs) >= 2:
        own_root = '/'.join(_segs[:2]) + '/'
    elif _segs:
        own_root = _segs[0] + '/'
    else:
        own_root = ''
    out['own_root'] = own_root

    def group_of(cname):
        root = lib_of(cname)
        if root:
            return root
        return cname.rsplit('/', 1)[0] + '/' if '/' in cname else '(default)/'

    # ---------------- pass B: aggregate references
    ext = {}   # key -> [kind, sites, classes, Counter(from tags), examples]
    ext_classes = {}  # class name -> [sites, classes, Counter, examples]
    via_sub = Counter()
    by_group = defaultdict(lambda: Counter())
    lib_users = defaultdict(set)
    events = Counter()
    events_where = defaultdict(set)
    extends = Counter()
    implemented = Counter()
    custom_events = []
    resolve_cache = {}
    ref_groups = defaultdict(set)
    _patch_pfx = tuple(p_ for _, p_ in PATCH_PREFIXES if p_) + ('java/lang/invoke/MethodHandles$Lookup.defineClass',)

    def from_tag(ci):
        return ci.tag

    for key, ci in infos.items():
        tag = ci.tag
        g = group_of(ci.name)
        my_lib = lib_of(ci.name)
        # supertypes from the API or outside the jar
        for s in [ci.super] + list(ci.ifaces):
            if s and s not in defined:
                if s.startswith(('org/bukkit/', 'org/spigotmc/', 'io/papermc/', 'com/destroystokyo/', 'net/md_5/')):
                    (extends if s == ci.super else implemented)[s] += 1
        if ci.super and ci.super not in defined:
            pass
        # class references
        for cn, sites in ci.crefs.items():
            if cn in defined:
                lr = lib_of(cn)
                if lr and lr != my_lib:
                    lib_users[lr].add(ci.name)
                continue
            rec = ext_classes.get(cn)
            if rec is None:
                rec = ext_classes[cn] = [0, 0, Counter(), [], 0]
            rec[0] += sites
            rec[1] += 1
            rec[2][tag] += 1
            rec[4] += ci.runs
            if len(rec[3]) < 3:
                rec[3].append(ci.name)
            if cn.startswith(_patch_pfx) and len(ref_groups[cn]) < 12:
                ref_groups[cn].add(g)
            cat = ref_cat(cn)
            if cat:
                by_group[g][cat] += 1
        for kind, o, n, d, sites in ci.mrefs:
            oc = elem_class(o)
            if oc is None or (oc != o):
                continue
            if o in defined:
                lr = lib_of(o)
                if lr and lr != my_lib:
                    lib_users[lr].add(ci.name)
                # inherited member: walk the jar hierarchy to an outside class
                rk = (o, n, d, kind)
                r = resolve_cache.get(rk)
                if r is None:
                    r = resolve_inherited(o, n, d, kind == 'F', infos_by_name(infos), api)
                    resolve_cache[rk] = r
                if r:
                    mk = r
                    rec = ext.get(mk)
                    if rec is None:
                        rec = ext[mk] = [kind, 0, 0, Counter(), [], 0]
                    rec[1] += sites
                    rec[2] += 1
                    rec[3][tag] += 1
                    rec[5] += ci.runs
                    if len(rec[4]) < 3:
                        rec[4].append(ci.name)
                    via_sub[mk] += max(sites, 1)
                continue
            mk = o + '.' + n + (':' + d if kind == 'F' else d)
            rec = ext.get(mk)
            if rec is None:
                rec = ext[mk] = [kind, 0, 0, Counter(), [], 0]
            rec[1] += sites
            rec[2] += 1
            rec[3][tag] += 1
            rec[5] += ci.runs
            if mk.startswith(_patch_pfx) and len(ref_groups[o]) < 12:
                ref_groups[o].add(g)
            if len(rec[4]) < 3:
                rec[4].append(ci.name)
            cat = ref_cat(o)
            if cat:
                by_group[g][cat + '_members'] += 1
        for (mn, md) in ci.events:
            ps, _ = method_params(md)
            if len(ps) >= 1 and ps[0].startswith('L'):
                ev = ps[0][1:-1]
                events[ev] += 1
                if len(events_where[ev]) < 3:
                    events_where[ev].add(ci.name)
        if ci.super and (ci.super == 'org/bukkit/event/Event' or (ci.super in api.events)):
            custom_events.append(ci.name)

    # ---------------- pass C: stack simulation for reflection
    t_sim = time.time()
    ctx = JarCtx(supers)
    for f, vs in const_field_vals.items():
        ctx.fields_prev[f] = vs
    sim_index = [(key, ci) for key, ci in infos.items() if ci.sim]
    sim_count = 0
    sim_errors = Counter()
    sim_error_examples = []
    sim_truncated = False
    cf_cache = {}
    final_sites = []
    rounds_done = 0
    prev_sig = None
    for rnd in range(ROUNDS):
        round_sites = []
        ctx.summ_new = {}
        ctx.fields_new = defaultdict(list)
        summ_names = {(k[1], k[2]) for k in ctx.summ_prev}
        for key, ci in infos.items():
            if not ci.sim and rnd == 0:
                continue
            if rnd > 0 and not ci.sim:
                # does this class call a summarized method or read a known field?
                hit = False
                for kind, o, n, d, sites in ci.mrefs:
                    if kind == 'F':
                        if (o, n, d) in ctx.fields_prev:
                            hit = True
                            break
                    elif (n, d) in summ_names and ctx.summary(o, n, d) is not None:
                        hit = True
                        break
                if not hit:
                    continue
            budget = SIM_BUDGET_S - (time.time() - t_sim)
            if budget <= 0:
                sim_truncated = True
                break
            cf = cf_cache.get(key)
            if cf is None:
                try:
                    cf = parse_class(ci.unit.data)
                except Exception:
                    continue
                if len(cf_cache) < 3000:
                    cf_cache[key] = cf
            for m in cf.methods:
                if m.code is None or len(m.code) > SIM_MAX_CODE:
                    continue
                if not method_wanted(cf, m, rnd, ctx, defined):
                    continue
                try:
                    simulate(cf, m, cf.name, ctx, round_sites, max(1.0, min(20.0, budget)))
                    sim_count += 1
                except Exception as e:
                    sim_errors[type(e).__name__] += 1
                    if len(sim_error_examples) < 3:
                        sim_error_examples.append('%s.%s%s: %s: %s' % (cf.name, m.name, m.desc, type(e).__name__, str(e)[:80]))
                    continue
        # carry forward the static constant values
        newf = dict(ctx.fields_new)
        for f, vs in const_field_vals.items():
            newf.setdefault(f, vs)
        sig_s = summary_signature(ctx.summ_new, newf)
        changed = sig_s != prev_sig
        prev_sig = sig_s
        ctx.summ_prev = ctx.summ_new
        ctx.fields_prev = newf
        final_sites = round_sites
        rounds_done = rnd + 1
        if sim_truncated or (rnd > 0 and not changed):
            break
    sim_time = time.time() - t_sim

    # ---------------- render the reflection sites
    site_agg = {}
    dropped_sites = 0
    for cls_name, mn, md, s in final_sites:
        kind, o, n, d, recv, args, via = s
        rec = render_site(kind, o, n, d, recv, args)
        if rec is None:
            continue
        rec['in'] = cls_name + '.' + mn + md
        if via:
            rec['via'] = [vo + '.' + vn + vd for vo, vn, vd in via]
        if site_has_arg(kind, recv, args):
            rec['template'] = True
        cinfo = infos.get(cls_name)
        if cinfo is not None and cinfo.tag:
            rec['v'] = cinfo.tag
        if not rec.get('template') and not site_informative(kind, rec):
            dropped_sites += 1
            continue
        k = json.dumps(rec, sort_keys=True)
        if k in site_agg:
            site_agg[k]['n'] += 1
        else:
            rec['n'] = 1
            site_agg[k] = rec
    sites = list(site_agg.values())
    for r in sites:
        if r.get('kind') == 'bukkit-impl' and r.get('name') and r.get('target', '').startswith('<class of org.bukkit.'):
            t = r['target'][len('<class of '):].split('>', 1)[0]
            pub = api_public_name(api, t, r['name'], 'Field' in r['call'] or 'Getter' in r['call'] or 'Setter' in r['call'])
            if pub is not None:
                r['api_public'] = pub
    sites.sort(key=lambda r: (r.get('kind') not in ('craftbukkit', 'bukkit-impl', 'nms'), r['call'], r.get('target', '')))

    # ---------------- assemble output
    def ref_list(prefix, exclude=None):
        cls, fld, mth = [], [], []
        for cn, (sites_, ncls, tags, ex, runs) in ext_classes.items():
            if cn.startswith(prefix) and not (exclude and cn.startswith(exclude)):
                cls.append(mk_ref(cn, sites_, ncls, tags, ex, runs))
        for mk, (kind, sites_, ncls, tags, ex, runs) in ext.items():
            if mk.startswith(prefix) and not (exclude and mk.startswith(exclude)):
                r = mk_ref(mk, sites_, ncls, tags, ex, runs)
                (fld if kind == 'F' else mth).append(r)
        for l in (cls, fld, mth):
            l.sort(key=lambda r: (-r['classes'], r['ref']))
        return cls, fld, mth

    # 1. CraftBukkit
    cb_c, cb_f, cb_m = ref_list('org/bukkit/craftbukkit/')
    cb_versions = Counter()
    for r in cb_c + cb_f + cb_m:
        cb_versions[r.get('pkg_version') or '(unversioned)'] += 1
    out['craftbukkit'] = {'classes': cb_c, 'fields': cb_f, 'methods': cb_m, 'package_versions': dict(cb_versions)}
    # 2. Minecraft and Mojang
    nms_c, nms_f, nms_m = ref_list('net/minecraft/')
    nv = Counter()
    for r in nms_c + nms_f + nms_m:
        nv[r.get('pkg_version') or '(unversioned)'] += 1
    out['minecraft'] = {'classes': nms_c, 'fields': nms_f, 'methods': nms_m, 'package_versions': dict(nv),
                        'packages': dict(Counter(r['ref'].rsplit('/', 1)[0] for r in nms_c)),
                        'naming': nms_naming(nms_c, nms_f + nms_m)}
    mj_c, mj_f, mj_m = ref_list('com/mojang/')
    out['mojang'] = {'classes': mj_c, 'fields': mj_f, 'methods': mj_m,
                     'libraries': dict(Counter('/'.join(r['ref'].split('/')[:3]) for r in mj_c)),
                     'shaded': sorted({'/'.join(ci.name.split('/')[:3]) for ci in infos.values()
                                       if ci.name.startswith('com/mojang/')})}
    # 3. Reflection
    refl_strings = []
    for s, (hits, classes) in jar_strings.items():
        if not hits:
            continue
        refl_strings.append({'s': s[:400], 'match': sorted(set(hits)), 'classes': len(classes),
                             'ex': sorted(classes)[:3],
                             'v': sorted({infos[c].tag for c in classes if c in infos and infos[c].tag})})
    refl_strings.sort(key=lambda r: (-r['classes'], r['s']))
    sink_methods.sort(key=lambda r: r['in'])
    out['reflection'] = {
        'methods': sink_methods[:1500], 'methods_total': len(sink_methods),
        'sites': sites[:4000], 'sites_total': len(sites), 'sites_unresolved_dropped': dropped_sites,
        'strings': refl_strings[:3000], 'strings_total': len(refl_strings),
        'forname_targets': sorted({r['target'] for r in sites if r['call'] in ('Class.forName', 'ClassLoader.loadClass',
                                                                              'Lookup.findClass')})[:500],
        'simulation': {'methods_simulated': sim_count, 'seconds': round(sim_time, 2), 'truncated': sim_truncated,
                       'summaries': len(ctx.summ_prev), 'rounds': rounds_done, 'errors': dict(sim_errors),
                       'error_examples': sim_error_examples},
    }
    # 4. Paper and kin
    paper = {}
    for label, prefix in PAPER_PREFIXES:
        c_, f_, m_ = ref_list(prefix)
        shaded = sorted({ci.name for ci in infos.values() if ci.name.startswith(prefix)})
        entry = {'refs_classes': len(c_), 'refs_members': len(f_) + len(m_),
                 'shaded_classes': len(shaded), 'classes': c_[:300], 'members': (f_ + m_)[:600]}
        if prefix != 'org/spigotmc/':
            entry['shaded_packages'] = sorted({'/'.join(x.split('/')[:4]) for x in shaded})[:20]
        if prefix == 'org/spigotmc/':
            miss = []
            for r in f_ + m_:
                kind = 'F' if ':' in r['ref'] else 'M'
                o_, mem = split_key(r['ref'], kind)
                if api.resolve(o_, mem, kind == 'F')[0] in ('missing', 'unknown-owner'):
                    miss.append(r['ref'])
            for r in c_:
                if r['ref'] not in api.model:
                    miss.append(r['ref'])
            entry['missing_in_spigot_api'] = miss[:100]
        paper[label] = entry
    adv = paper['net/kyori/adventure/']
    relocated = [{'root': root, 'lib': inst['lib'], 'classes': inst['classes'],
                  'users_outside': len(lib_users.get(root, ()))}
                 for root, inst in lib_inst.items() if inst['lib'].startswith('adventure') and inst['relocated']]
    shaded_unrel = [{'root': root, 'lib': inst['lib'], 'classes': inst['classes']}
                    for root, inst in lib_inst.items() if inst['lib'].startswith('adventure') and not inst['relocated']]
    server_refs = [r for r in adv['classes'] if r['ref'] not in defined]
    adv['mode'] = sorted(set((['server'] if adv['refs_classes'] else []) + (['shaded'] if shaded_unrel else []) +
                             (['relocated'] if relocated else []))) or ['none']
    adv['relocated_copies'] = relocated
    adv['shaded_copies'] = shaded_unrel
    out['paper'] = paper
    # 5. Runtime patching
    patch = {}
    for label, prefix in PATCH_PREFIXES:
        if prefix is None:
            hits = [mk_ref(mk, *rec[1:]) for mk, rec in ext.items() if mk.startswith(label)]
            if hits:
                patch[label] = {'members': hits}
            continue
        c_, f_, m_ = ref_list(prefix)
        shaded = sum(1 for ci in infos.values() if ci.name.startswith(prefix))
        if c_ or f_ or m_ or shaded:
            pk = set()
            for cn, gs in ref_groups.items():
                if cn.startswith(prefix):
                    pk |= gs
            patch[label] = {'classes': c_[:100], 'members': (f_ + m_)[:300], 'shaded_classes': shaded,
                            'from_packages': sorted(pk)[:24],
                            'from_libraries': sorted({lib_inst[g_]['lib'] for g_ in pk if g_ in lib_inst})}
    for root, inst in lib_inst.items():
        if inst['lib'] in ('asm', 'bytebuddy', 'javassist', 'mixin') and inst['relocated']:
            patch.setdefault('relocated_copies', []).append({'lib': inst['lib'], 'root': root, 'classes': inst['classes'],
                                                             'users_outside': len(lib_users.get(root, ()))})
    patch_strings = [r['s'] for r in refl_strings if False]
    unsafe_str = sorted({s for s in jar_strings if s in ('sun.misc.Unsafe', 'theUnsafe', 'jdk.internal.misc.Unsafe')
                         or s.startswith('jdk.internal.') or s.startswith('java.lang.instrument')})
    if unsafe_str:
        patch['strings'] = unsafe_str[:50]
    agent = {k: man[k] for k in ('Agent-Class', 'Premain-Class', 'Launcher-Agent-Class', 'Can-Redefine-Classes',
                                 'Can-Retransform-Classes') if k in man}
    if agent:
        patch['manifest'] = agent
    out['patching'] = patch
    # 6. Bukkit API
    members = {}
    missing = []
    types = Counter()
    sites_total = 0
    for cn, (sites_, ncls, tags, ex, runs) in ext_classes.items():
        if (cn.startswith('org/bukkit/') and not cn.startswith('org/bukkit/craftbukkit/')) or cn.startswith('org/spigotmc/'):
            types[cn] = ncls
    for mk, (kind, sites_, ncls, tags, ex, runs) in ext.items():
        if (mk.startswith('org/bukkit/') and not mk.startswith('org/bukkit/craftbukkit/')) or mk.startswith('org/spigotmc/'):
            owner, member = split_key(mk, kind)
            st, decl, acc = api.resolve(owner, member, kind == 'F')
            members[mk] = [sites_, ncls]
            sites_total += sites_
            if st == 'missing' or st == 'unknown-owner':
                missing.append(mk)
            if via_sub.get(mk):
                members[mk].append('via-subclass')
            if acc & 0x0002:
                members[mk].append('private')
            elif acc & 0x0004:
                members[mk].append('protected')
            elif st == 'api' and not (acc & 0x0001) and decl and not (api.model[decl]['a'] & 0x0200):
                members[mk].append('package-private')
    missing_types = sorted(t for t in types if t not in api.model)
    ev_out = {}
    for ev, n in events.most_common():
        if ev in api.events:
            st = 'spigot-api'
        elif ev in defined:
            st = 'own'
        elif ev.startswith('org/bukkit/') or ev.startswith('org/spigotmc/'):
            st = 'missing-in-spigot-api'
        elif ev.startswith(('io/papermc/', 'com/destroystokyo/paper/')):
            st = 'paper'
        else:
            st = 'other-plugin-or-library'
        ev_out[ev] = [n, st]
    reg_events = sorted({r.get('event') for r in sites if r['call'] == 'PluginManager.registerEvent' and r.get('event')})
    out['bukkit_api'] = {
        'types_count': len(types), 'members_count': len(members), 'sites': sites_total,
        'types': dict(sorted(types.items())), 'members': dict(sorted(members.items())),
        'missing_in_spigot_api': sorted(missing), 'missing_types': missing_types,
        'extends': dict(extends.most_common()), 'implements': dict(implemented.most_common()),
        'events': ev_out, 'events_spigot': sum(1 for v in ev_out.values() if v[1] == 'spigot-api'),
        'registered_events': reg_events, 'custom_events': sorted(custom_events)[:200],
        'defines_org_bukkit_classes': sorted(c for c in defined if c.startswith('org/bukkit/'))[:50],
    }
    # 7. Version adapters
    tagc = defaultdict(lambda: {'classes': 0, 'packages': set(), 'exact': False, 'runs_on_1_20_1': False,
                                'written': set()})
    for ci in infos.values():
        if not ci.tag:
            continue
        t = tagc[ci.tag]
        t['classes'] += 1
        t['exact'] = ci.exact
        t['runs_on_1_20_1'] = ci.runs
        for w, sp in tags_in(ci.name)[:1] or tags_in(ci.origin)[:1] or \
                [(m_.group(0), None) for m_ in _YEAR_TAG_RE.finditer(ci.name)][:1]:
            if len(t['written']) < 4:
                t['written'].add(w)
        if len(t['packages']) < 4:
            t['packages'].add(ci.name.rsplit('/', 1)[0])
    va = {}
    for w, t in sorted(tagc.items()):
        va[w] = {'classes': t['classes'], 'exact': t['exact'], 'runs_on_1_20_1': t['runs_on_1_20_1'],
                 'written': sorted(t['written']), 'packages': sorted(t['packages'])}
    spig_set = sorted({w for w, t in va.items() if t['exact'] and w.startswith('v1_')}, key=_tag_sort)
    year_set = sorted(w for w, t in va.items() if t['exact'] and re.match(r'v[23][0-9]_', w))
    out['version_adapters'] = {
        'tags': va, 'exact_revisions': spig_set,
        'thresholds': sorted(w for w, t in va.items() if not t['exact']),
        'year_versions': year_set,
        'has_v1_20_R1': 'v1_20_R1' in spig_set,
        'range': [spig_set[0], spig_set[-1]] if spig_set else None,
    }
    # 8. Libraries
    libs_out = []
    for root, inst in sorted(lib_inst.items()):
        members_in = Counter()
        for key, ci in infos.items():
            if lib_of(ci.name) == root:
                for cn in ci.crefs:
                    c = ref_cat(cn)
                    if c and cn not in defined:
                        members_in[c] += 1
        refl_n = sum(1 for r in sites if lib_of(r['in'].split('.', 1)[0]) == root and r.get('kind') in
                     ('craftbukkit', 'nms', 'bukkit-impl', 'mojang'))
        libs_out.append({'lib': inst['lib'], 'root': root, 'original': inst['original'], 'relocated': inst['relocated'],
                         'single_class': inst['single'], 'classes': inst['classes'],
                         'markers': inst['markers'][:6], 'internal_class_refs': dict(members_in),
                         'reflection_sites_internal': refl_n,
                         'users_outside': len(lib_users.get(root, ()))})
    deps = {}
    for pref in LIB_DEP_PREFIXES:
        cl = [cn for cn in ext_classes if cn.startswith(pref)]
        if cl:
            deps[pref] = {'classes': len(cl), 'examples': sorted(cl)[:5]}
    groups = []
    for g, cnt in by_group.items():
        if not any(k.startswith(('craftbukkit', 'nms')) for k in cnt):
            continue
        groups.append({'package': g, 'lib': lib_inst[g]['lib'] if g in lib_inst else None,
                       'own': bool(own_root and g.startswith(own_root)), 'refs': dict(cnt)})
    # reflection into internals, attributed by package
    refl_groups = Counter()
    for r in sites:
        if r.get('kind') in ('craftbukkit', 'nms', 'bukkit-impl'):
            refl_groups[group_of(r['in'].split('.', 1)[0])] += r['n']
    for g, n in refl_groups.items():
        for e in groups:
            if e['package'] == g:
                e['reflection_sites'] = n
                break
        else:
            groups.append({'package': g, 'lib': lib_inst[g]['lib'] if g in lib_inst else None,
                           'own': bool(own_root and g.startswith(own_root)), 'refs': {}, 'reflection_sites': n})
    groups.sort(key=lambda e: e['package'])
    out['libraries'] = {'shaded': libs_out, 'dependencies': deps, 'internals_by_package': groups}
    # external dependencies the runtime has to provide
    extgrp = defaultdict(lambda: [0, 0])
    for cn, rec in ext_classes.items():
        if is_jdk(cn) or cn.startswith(('org/bukkit/', 'net/minecraft/', 'com/mojang/', 'org/spigotmc/')):
            continue
        g = pkg_group(cn)
        extgrp[g][0] += 1
        extgrp[g][1] += rec[1]
    out['external_packages'] = {g: {'classes': v[0], 'refs': v[1]} for g, v in sorted(extgrp.items())}
    # summary
    cb_reach = [r for r in cb_c + cb_m + cb_f if r['on_1_20_1']]
    nms_reach = [r for r in nms_c + nms_m + nms_f if r['on_1_20_1']]
    refl_cb = [r for r in sites if r.get('kind') in ('craftbukkit', 'bukkit-impl')]
    refl_nms = [r for r in sites if r.get('kind') == 'nms']
    reach = set()
    if cb_c or cb_f or cb_m:
        reach.add('craftbukkit-direct')
    if nms_c or nms_f or nms_m:
        reach.add('nms-direct')
    if mj_c:
        reach.add('mojang-direct')
    api_private = []
    for r in sites:
        if r.get('template'):
            continue
        k = r.get('kind')
        lookup = r['call'].startswith(('Class.', 'Lookup.', 'ClassLoader.', 'MethodHandles.'))
        if k == 'craftbukkit':
            reach.add('craftbukkit-reflection')
        elif k == 'nms':
            reach.add('nms-reflection')
        elif k == 'mojang':
            reach.add('mojang-reflection')
        elif k == 'bukkit-impl' and lookup and r['call'] not in ('Class.forName',) and not r.get('api_public'):
            reach.add('impl-reflection')
        elif k == 'bukkit-api' and lookup and r.get('name'):
            acc = api_member_access(api, r.get('target', ''), r['name'], r['call'])
            if acc == 'non-public':
                reach.add('api-private-reflection')
                if len(api_private) < 50:
                    api_private.append(r['target'] + '#' + r['name'])
    strings_internal = any(any(m in ('craftbukkit', 'net.minecraft') or m.startswith('craft-class:') for m in r['match'])
                           for r in refl_strings)
    out['summary'] = {
        'level': 3 if reach & {'nms-direct', 'nms-reflection'} else
                 (2 if reach & {'craftbukkit-direct', 'craftbukkit-reflection', 'impl-reflection', 'api-private-reflection'}
                  else 1),
        'reaches': sorted(reach),
        'internal_strings_only': strings_internal and not reach & {'nms-direct', 'nms-reflection', 'craftbukkit-direct',
                                                                    'craftbukkit-reflection', 'impl-reflection'},
        'api_private_reflection': sorted(set(api_private)),
        'craftbukkit_refs': len(cb_c) + len(cb_f) + len(cb_m), 'craftbukkit_refs_on_1_20_1': len(cb_reach),
        'nms_refs': len(nms_c) + len(nms_f) + len(nms_m), 'nms_refs_on_1_20_1': len(nms_reach),
        'reflection_sites_craftbukkit': len(refl_cb), 'reflection_sites_nms': len(refl_nms),
        'reflection_methods': len(sink_methods),
        'paper_api_refs': paper['io/papermc/']['refs_classes'] + paper['com/destroystokyo/paper/']['refs_classes'],
        'bukkit_members_missing_in_spigot_api': len(missing),
        'adventure': adv['mode'],
        'patching': sorted(k for k in patch if k not in ('strings',)),
        'version_revisions': spig_set, 'has_v1_20_R1_adapter': 'v1_20_R1' in spig_set,
        'events': len(ev_out), 'bukkit_members': len(members),
    }
    out['scan']['seconds'] = round(time.time() - t_start, 2)
    return out


def summary_signature(summ, fields):
    out = {}
    for k, v in summ.items():
        out[('s',) + k] = (render(v['ret']) if v['ret'] is not None else None,
                           tuple(render(t[4]) if t[4] is not None else '' for t in v['tpl']),
                           tuple(tuple(render(a) for a in t[5]) for t in v['tpl']))
    for k, vs in fields.items():
        out[('f',) + k] = tuple(render(x) for x in vs)
    return out


def _tag_sort(t):
    m = re.match(r'v1_(\d+)_R(\d+)', t)
    return (int(m.group(1)), int(m.group(2))) if m else (0, 0)


def split_key(mk, kind):
    if kind == 'F':
        a = mk.index(':')
        head = mk[:a]
        o, n = head.rsplit('.', 1)
        return o, n + mk[a:]
    a = mk.index('(')
    head = mk[:a]
    o, n = head.rsplit('.', 1)
    return o, n + mk[a:]


def mk_ref(key, sites_, ncls, tags, ex, runs):
    pv = None
    m = re.match(r'(?:org/bukkit/craftbukkit|net/minecraft/server)/(v1_\d+_R\d+)/', key)
    if m:
        pv = m.group(1)
    frm = {(t or '-'): n for t, n in tags.items()}
    on = runs > 0
    r = {'ref': key, 'sites': sites_, 'classes': ncls, 'from': frm, 'on_1_20_1': on, 'ex': ex}
    if pv:
        r['pkg_version'] = pv
        r['on_1_20_1'] = on and pv == 'v1_20_R1'
    elif key.startswith('org/bukkit/craftbukkit/libs/'):
        r['pkg_version'] = 'libs'
    elif key.startswith('org/bukkit/craftbukkit/'):
        # unversioned CraftBukkit package: it exists only on Paper 1.20.5 and later
        r['pkg_version'] = None
        r['on_1_20_1'] = False
    return r


# Minecraft classes whose Spigot 1.20.1 name differs from the Mojang name, in both spellings.
_SPIGOT_NAMES = {'EntityPlayer', 'WorldServer', 'PlayerConnection', 'NBTTagCompound', 'NBTTagList', 'NBTBase',
                 'BlockPosition', 'MinecraftKey', 'DataWatcher', 'DataWatcherObject', 'EntityLiving', 'EntityHuman',
                 'IChatBaseComponent', 'ChatComponentText', 'IRegistry', 'World', 'NetworkManager', 'EnumHand',
                 'EnumItemSlot', 'EntityArmorStand', 'EntityTypes', 'IBlockData', 'TileEntity', 'MobEffect',
                 'MobEffectList', 'EntityInsentient', 'PathfinderGoal', 'PathfinderGoalSelector', 'ChunkProviderServer',
                 'PlayerChunkMap', 'DedicatedServer', 'EnumChatFormat', 'Vec3D', 'AxisAlignedBB', 'SoundEffect',
                 'EntityItem', 'EntityFallingBlock', 'EntityVillager', 'EntityZombie', 'ScoreboardServer', 'ChatMessage',
                 'ServerConnection', 'CommandListenerWrapper', 'ArgumentEntity', 'DynamicOpsNBT', 'GameProfileSerializer',
                 'MojangsonParser', 'NBTCompressedStreamTools', 'ResourceKey', 'IRegistryCustom', 'Containers'}
_MOJANG_NAMES = {'ServerPlayer', 'ServerLevel', 'ServerGamePacketListenerImpl', 'CompoundTag', 'ListTag', 'Tag',
                 'BlockPos', 'ResourceLocation', 'SynchedEntityData', 'EntityDataAccessor', 'LivingEntity', 'Player',
                 'Component', 'Registry', 'Level', 'Connection', 'InteractionHand', 'EquipmentSlot', 'ArmorStand',
                 'EntityType', 'BlockState', 'BlockEntity', 'MobEffectInstance', 'Mob', 'Goal', 'GoalSelector',
                 'ServerChunkCache', 'ChunkMap', 'DedicatedServer', 'ChatFormatting', 'Vec3', 'AABB', 'SoundEvent',
                 'ItemEntity', 'FallingBlockEntity', 'Villager', 'Zombie', 'ServerScoreboard', 'ServerConnectionListener',
                 'CommandSourceStack', 'EntityArgument', 'NbtOps', 'NbtUtils', 'TagParser', 'NbtIo', 'RegistryAccess',
                 'MenuType'}
_OBF_MEMBER = re.compile(r'^[a-z]{1,3}$')


def nms_naming(classes, members):
    sp, mj = Counter(), Counter()
    for r in classes:
        ref = r['ref']
        if re.match(r'net/minecraft/server/v1_', ref):
            continue          # pre-1.17 packages, Spigot names by construction
        simple = ref.rsplit('/', 1)[-1].split('$')[0]
        if simple in _SPIGOT_NAMES and simple not in _MOJANG_NAMES:
            sp[simple] += 1
        elif simple in _MOJANG_NAMES and simple not in _SPIGOT_NAMES:
            mj[simple] += 1
    obf = named = 0
    for r in members:
        ref = r['ref']
        if re.match(r'net/minecraft/server/v1_', ref):
            continue
        head = ref.split(':', 1)[0].split('(', 1)[0]
        nm = head.rsplit('.', 1)[-1]
        if nm in ('<init>', '<clinit>', 'values', 'valueOf', 'ordinal', 'name'):
            continue
        if _OBF_MEMBER.match(nm):
            obf += 1
        else:
            named += 1
    style = 'none'
    if sp and not mj:
        style = 'spigot'
    elif mj and not sp:
        style = 'mojang'
    elif sp and mj:
        style = 'mixed'
    return {'style': style, 'spigot_class_names': sum(sp.values()), 'mojang_class_names': sum(mj.values()),
            'spigot_examples': sorted(sp)[:8], 'mojang_examples': sorted(mj)[:8],
            'obfuscated_member_refs': obf, 'named_member_refs': named}


def ref_cat(name):
    if name.startswith('org/bukkit/craftbukkit/'):
        return 'craftbukkit'
    if name.startswith('net/minecraft/'):
        return 'nms'
    if name.startswith('com/mojang/'):
        return 'mojang'
    return None


_INFOS_BY_NAME = {}


def infos_by_name(infos):
    key = id(infos)
    d = _INFOS_BY_NAME.get(key)
    if d is None:
        _INFOS_BY_NAME.clear()
        d = {ci.name: ci for ci in infos.values()}
        _INFOS_BY_NAME[key] = d
    return d


def resolve_inherited(owner, name, desc, is_field, byname, api):
    """A member referenced through a jar class: the API member key it resolves to, or None."""
    member = name + (':' + desc if is_field else desc)
    c = owner
    for _ in range(40):
        ci = byname.get(c)
        if ci is None:
            break
        if is_field:
            if name in ci.fields and ci.fields[name] == desc:
                return None
        elif (name + desc) in ci.mdecl:
            return None
        c = ci.super
        if c is None:
            return None
    else:
        return None
    if not (c.startswith('org/bukkit/') or c.startswith('org/spigotmc/')) or c.startswith('org/bukkit/craftbukkit/'):
        return None
    st, decl, acc = api.resolve(c, member, is_field)
    if st == 'jdk':
        return None
    if st == 'api' and (acc & 0x0400):
        return None          # abstract in the API: the plugin implements it
    base = decl if st == 'api' else c
    return base + '.' + name + (':' + desc if is_field else desc)


def render_site(kind, o, n, d, recv, args):
    rec = {}
    if kind == 'lookup' or kind == 'lib':
        short = {'java/lang/Class': 'Class', 'java/lang/ClassLoader': 'ClassLoader', 'java/net/URLClassLoader': 'ClassLoader',
                 'java/lang/invoke/MethodHandles$Lookup': 'Lookup', 'java/lang/invoke/MethodHandles': 'MethodHandles'}
        rec['call'] = short.get(o, _short(o)) + '.' + n
        if o == 'java/lang/Class':
            if n == 'forName':
                tgt = args[0] if args else U
                rec['target'] = render(tgt)[:300]
                rec['kind'] = classify_target(rec['target'])
                return rec
            rec['target'] = render(recv)[:300] if recv is not None else '{?}'
            rec['kind'] = classify_target(rec['target'])
            if n in ('getDeclaredField', 'getField', 'getDeclaredMethod', 'getMethod'):
                rec['name'] = render(args[0])[:200] if args else '{?}'
            if n in ('getDeclaredMethod', 'getMethod'):
                rec['params'] = render_params(args[1] if len(args) > 1 else None)[:300]
            if n in ('getDeclaredConstructor', 'getConstructor'):
                rec['params'] = render_params(args[0] if args else None)[:300]
            return rec
        if n in ('loadClass', 'findClass') or (o == 'java/lang/invoke/MethodHandles$Lookup' and n == 'findClass'):
            rec['target'] = render(args[0] if args else U)[:300]
            rec['kind'] = classify_target(rec['target'])
            return rec
        if o == 'java/lang/invoke/MethodHandles$Lookup':
            if n.startswith('find'):
                rec['target'] = render(args[0] if args else U)[:300]
                rec['kind'] = classify_target(rec['target'])
                if n != 'findConstructor' and len(args) > 1:
                    rec['name'] = render(args[1])[:200]
                tp = args[-1] if args else None
                if tp is not None:
                    rec['type'] = (render_mt(tp) if isinstance(tp, tuple) and tp[0] == 'mt' else render(tp))[:300]
                return rec
            if n.startswith('unreflect'):
                r = render(args[0] if args else U)[:300]
                rec['target'] = r
                rec['kind'] = classify_target(r.split(' ', 1)[-1])
                return rec
            rec['target'] = render(args[0] if args else U)[:200]
            rec['kind'] = 'jdk'
            return rec
        if o == 'java/lang/invoke/MethodHandles':
            rec['target'] = render(args[0] if args else U)[:300]
            rec['kind'] = classify_target(rec['target'])
            return rec
        if kind == 'lib':
            rs = [render(a)[:200] for a in args]
            rec['args'] = rs
            rec['kind'] = 'unknown'
            for x in rs:
                c = classify_target(x)
                if c not in ('unknown', 'other'):
                    rec['kind'] = c
                    break
            return rec
        rec['target'] = render(args[0] if args else U)[:200]
        rec['kind'] = classify_target(rec['target'])
        return rec
    if kind == 'use':
        if recv is None or not isinstance(recv, tuple) or recv[0] not in ('fld', 'mth', 'ctor', 'mh', 'alt'):
            return None
        if not is_info(recv):
            return None
        rec['call'] = {'java/lang/reflect/Field': 'Field', 'java/lang/reflect/Method': 'Method',
                       'java/lang/reflect/Constructor': 'Constructor', 'java/lang/invoke/MethodHandle': 'MethodHandle'}[o] + '.' + n
        r = render(recv)[:300]
        rec['target'] = r
        inner = recv
        if isinstance(recv, tuple) and recv[0] == 'alt' and recv[1]:
            inner = recv[1][0]
        if isinstance(inner, tuple) and inner[0] in ('fld', 'mth', 'ctor', 'mh'):
            cls_v = inner[2] if inner[0] == 'mh' else inner[1]
            rec['kind'] = classify_target(render(cls_v))
        else:
            rec['kind'] = 'unknown'
        return rec
    if kind == 'event':
        rec['call'] = 'PluginManager.registerEvent'
        ev = args[0] if args else U
        rec['event'] = render(ev)[:200]
        rec['kind'] = 'bukkit-api'
        return rec
    return None


_PLACEHOLDER_RE = re.compile(r'\{[^{}]*\}|<[^<>]*>')


def _literal(s):
    s = s or ''
    for _ in range(4):
        s2 = _PLACEHOLDER_RE.sub('', s)
        if s2 == s:
            break
        s = s2
    return re.sub(r'[\s()|.,\[\]$#]+', '', s)


def api_public_name(api, dotted, name, is_field):
    """True when an API type or one of its supertypes has a public member of that name; None if unknown."""
    if '{' in name or '(' in name:
        return None
    t = dotted.replace('.', '/')
    if t not in api.model:
        parts = t.split('/')
        for k in range(len(parts) - 1, 0, -1):
            cand = '/'.join(parts[:k]) + '/' + '$'.join(parts[k:])
            if cand in api.model:
                t = cand
                break
        else:
            return None
    seen = set()
    stack = [t]
    while stack:
        c = stack.pop()
        if c in seen:
            continue
        seen.add(c)
        info = api.model.get(c)
        if info is None:
            continue
        tab = info['f'] if is_field else info['m']
        sep = ':' if is_field else '('
        for k, a in tab.items():
            if k.split(sep, 1)[0] == name and (a & 0x0001):
                return True
        if info['s']:
            stack.append(info['s'])
        stack.extend(info['i'])
    return False


def api_member_access(api, target, name, call):
    """'public' or 'non-public' for a reflected member of a Spigot API class named literally, else None."""
    if not target.startswith(('org.bukkit.', 'org.spigotmc.')) or '{' in target or '(' in target or '{' in name:
        return None
    t = target.replace('.', '/')
    info = api.model.get(t)
    if info is None:
        parts = t.split('/')
        for k in range(len(parts) - 1, 0, -1):
            cand = '/'.join(parts[:k]) + '/' + '$'.join(parts[k:])
            if cand in api.model:
                info = api.model[cand]
                break
    if info is None:
        return None
    if 'Field' in call or call in ('Lookup.findGetter', 'Lookup.findSetter', 'Lookup.findStaticGetter',
                                   'Lookup.findStaticSetter', 'Lookup.findVarHandle', 'Lookup.findStaticVarHandle'):
        accs = [a for k, a in info['f'].items() if k.split(':', 1)[0] == name]
    else:
        accs = [a for k, a in info['m'].items() if k.split('(', 1)[0] == name]
    if not accs:
        return None
    return 'public' if all(a & 0x0001 for a in accs) else 'non-public'


def site_informative(kind, rec):
    """A resolved site is worth reporting when it names a target or a member literally."""
    k = rec.get('kind')
    if kind == 'use':
        return k in ('craftbukkit', 'nms', 'bukkit-impl', 'bukkit-api', 'mojang', 'paper', 'adventure')
    if kind == 'event':
        return True
    if k not in ('unknown', None):
        return True
    return bool(_literal(rec.get('target', '')) or _literal(rec.get('name', '')))


def site_has_arg(kind, recv, args):
    if recv is not None and has_arg(recv):
        return True
    if kind == 'use':
        return False
    if kind == 'event':
        return bool(args) and has_arg(args[0])
    return any(has_arg(a) for a in args)


def method_wanted(cf, m, rnd, ctx, defined):
    """Decide from a quick scan whether a method is worth simulating."""
    code = m.code
    n = len(code)
    pc = 0
    vals, tags = cf.vals, cf.tags
    params, ret_t = method_params(m.desc)
    ret_interesting = ret_t in INTERESTING_DESCS
    has_ldc = False
    versrc = False
    try:
        while pc < n:
            op = code[pc]
            if 178 <= op <= 185:
                idx = (code[pc + 1] << 8) | code[pc + 2]
                o, nm, d = cf.ref(idx)
                if op >= 182:
                    if _sink_kind(o, nm) is not None:
                        return True
                    if (o, nm) in VERSRC:
                        versrc = True
                    if rnd > 0 and ctx.summary(o, nm, d) is not None:
                        return True
                else:
                    if op == 179 and d in INTERESTING_DESCS:
                        return True
                    if op == 181 and d in REFLECT_DESCS:
                        return True
                    if rnd > 0 and (op == 178 or op == 180) and (o, nm, d) in ctx.fields_prev:
                        return True
            elif op == 18 or op == 19:
                has_ldc = True
            pc += insn_len(code, pc)
    except (IndexError, struct.error):
        return False
    if ret_interesting and (has_ldc or versrc):
        return True
    return False


def analyze_class(cf, u, api, sink_methods, jar_strings, const_field_vals):
    ci = CInfo()
    ci.name = cf.name
    ci.super = cf.super
    ci.ifaces = cf.ifaces
    ci.access = cf.access
    ci.origin = u.origin
    ci.entry = u.entry
    ci.unit = u
    ci.major = cf.major
    ci.tag, ci.exact, ci.runs = class_version(cf.name)
    if not ci.tag and u.origin:
        ci.tag, ci.exact, ci.runs = class_version(u.origin)
    ci.fields = {fn: fd for fa, fn, fd in cf.fields}
    ci.mdecl = set(m.name + m.desc for m in cf.methods)
    tags, vals = cf.tags, cf.vals
    n = len(tags)
    # per-pool-entry info
    special = {}
    for i in range(1, n):
        t = tags[i]
        if t == 9 or t == 10 or t == 11:
            try:
                o, nm, d = cf.ref(i)
            except Exception:
                continue
            if t != 9:
                k = _sink_kind(o, nm)
                if k is not None:
                    special[i] = k
                elif (o, nm) in VERSRC:
                    special[i] = 'versrc'
            elif d in REFLECT_DESCS:
                special[i] = 'fput_r'
            elif d in INTERESTING_DESCS:
                special[i] = 'fput_s'
    # code pass
    sites = {}
    sim = False
    events = []
    for m in cf.methods:
        if m.annots:
            for a in m.annots:
                if a == 'Lorg/bukkit/event/EventHandler;':
                    events.append((m.name, m.desc))
                    break
        code = m.code
        if code is None:
            continue
        ldcs = []
        indys = []
        msinks = set()
        core = False
        pc = 0
        ln_code = len(code)
        try:
            while pc < ln_code:
                op = code[pc]
                k = REFKIND[op]
                if k:
                    if k == 3:
                        idx = code[pc + 1]
                        ldcs.append(idx)
                    else:
                        idx = (code[pc + 1] << 8) | code[pc + 2]
                        if k == 4:
                            ldcs.append(idx)
                        elif op == 186:
                            indys.append(idx)
                        else:
                            sites[idx] = sites.get(idx, 0) + 1
                            sp = special.get(idx)
                            if sp is not None:
                                if sp == 'fput_r':
                                    if op == 179 or op == 181:
                                        sim = True
                                elif sp == 'fput_s':
                                    if op == 179:
                                        sim = True
                                elif sp == 'versrc':
                                    sim = True
                                else:
                                    sim = True
                                    if sp == 'lookup' or sp == 'lib':
                                        o, nm, d = cf.ref(idx)
                                        msinks.add(('Lookup' if o.endswith('Lookup') else _short(o)) + '.' + nm)
                                        if (o, nm) in CORE_SINKS:
                                            core = True
                    if k == 1 and op != 186 and tags[idx] == 7:
                        pass
                ln = OPLEN[op]
                pc += ln if ln else insn_len(code, pc)
        except (IndexError, struct.error):
            pass
        # indy: bootstrap method handles count as uses of their member, and recipes are strings
        recipes = []
        for idx in indys:
            try:
                bi = vals[idx][0]
                mh, bargs = cf.bsm[bi]
            except Exception:
                continue
            for a in bargs:
                ta = tags[a]
                if ta == 15:
                    ref = vals[a][1]
                    sites[ref] = sites.get(ref, 0) + 1
                elif ta == 8 and msinks:
                    recipes.append(vals[vals[a]].replace('\x01', '{}').replace('\x02', '{c}'))
        if msinks:
            strs, clss = [], []
            for idx in ldcs:
                t = tags[idx]
                if t == 8:
                    s = vals[vals[idx]]
                    if s not in strs and len(strs) < 80:
                        strs.append(s[:300])
                elif t == 7:
                    c = vals[vals[idx]]
                    if c not in clss and len(clss) < 40:
                        clss.append(c)
            for r in recipes:
                if r not in strs and len(strs) < 80:
                    strs.append(r[:300])
            rec = {'in': cf.name + '.' + m.name + m.desc, 'sinks': sorted(msinks), 'core': core,
                   'strings': strs, 'class_constants': clss}
            if ci.tag:
                rec['v'] = ci.tag
            sink_methods.append(rec)
    ci.events = events
    ci.sim = sim
    # references from the pool
    crefs = {}
    mrefs = []
    for i in range(1, n):
        t = tags[i]
        if t == 7:
            c = elem_class(vals[vals[i]])
            if c:
                crefs[c] = crefs.get(c, 0) + sites.get(i, 0)
        elif t == 9 or t == 10 or t == 11:
            try:
                o, nm, d = cf.ref(i)
            except Exception:
                continue
            mrefs.append(('F' if t == 9 else ('I' if t == 11 else 'M'), o, nm, d, sites.get(i, 0)))
            for c in desc_classes(d):
                crefs.setdefault(c, 0)
        elif t == 8:
            s = vals[vals[i]]
            if s and len(s) < 2000:
                hits = match_string(s, api.simple)
                if hits:
                    e = jar_strings.get(s)
                    if e is None:
                        jar_strings[s] = e = [hits, set()]
                    e[1].add(cf.name)
                elif ('bStats' in s or 'bstats.org' in s):
                    e = jar_strings.get(s)
                    if e is None:
                        jar_strings[s] = e = [[], set()]
                    e[1].add(cf.name)
                elif s in ('sun.misc.Unsafe', 'theUnsafe', 'jdk.internal.misc.Unsafe') or s.startswith('jdk.internal.') \
                        or s.startswith('java.lang.instrument'):
                    e = jar_strings.get(s)
                    if e is None:
                        jar_strings[s] = e = [[], set()]
                    e[1].add(cf.name)
        elif t == 16:
            for c in desc_classes(vals[vals[i]]):
                crefs.setdefault(c, 0)
    for fa, fn, fd in cf.fields:
        for c in desc_classes(fd):
            crefs.setdefault(c, 0)
    for m in cf.methods:
        for c in desc_classes(m.desc):
            crefs.setdefault(c, 0)
    for s in [cf.super] + list(cf.ifaces):
        if s:
            crefs.setdefault(s, 0)
    crefs.pop(cf.name, None)
    ci.crefs = crefs
    ci.mrefs = mrefs
    for fname, fdesc, sval in cf.const_fields:
        const_field_vals[(cf.name, fname, fdesc)] = [('s', sval)]
    return ci


def detect_libs(infos, jar_strings):
    by_simple = defaultdict(list)
    for lib, roots, markers in LIBS:
        for mk in markers:
            grade = 'strong' if mk.startswith('!') else ('generic' if mk.startswith('?') else 'plain')
            mk = mk.lstrip('!?')
            by_simple[mk.rsplit('/', 1)[-1]].append((lib, roots, mk, grade))
    found = defaultdict(lambda: {'markers': set(), 'strong': False, 'plain': []})
    names = set(ci.name for ci in infos.values())
    for c in names:
        simple = c.rsplit('/', 1)[-1]
        cand = by_simple.get(simple)
        if not cand:
            continue
        for lib, roots, mk, grade in cand:
            if c == mk or c.endswith('/' + mk):
                root = c[:len(c) - len(mk)]
                f = found[(lib, root)]
                f['markers'].add(mk)
                f['strong'] = f['strong'] or grade == 'strong'
                f['roots'] = roots
                if grade != 'generic':
                    f['plain'].append(c)
    # bStats copies found by their strings, unless a marker root already covers the class
    marker_roots = [root for (lib, root), f in found.items() if lib == 'bstats' and len(f['markers']) >= 2]
    for s, (hits, classes) in jar_strings.items():
        if 'bstats.org' in s.lower():
            for c in classes:
                if any(c.startswith(r) for r in marker_roots):
                    continue
                if c.rsplit('/', 1)[-1].startswith('Metrics') or 'bstats' in c.lower():
                    f = found[('bstats', c.rsplit('/', 1)[0] + '/')]
                    f['markers'].add('string:' + s[:40])
                    f['roots'] = ['org/bstats/']
                    f['plain'].append(c)
    inst = {}
    for (lib, root), f in found.items():
        single = not (f['strong'] or len(f['markers']) >= 2)
        if single:
            for mc in f['plain']:
                inst[mc] = {'lib': lib, 'original': f['roots'][0], 'relocated': not any(mc.startswith(r) for r in f['roots']),
                            'single': True, 'markers': sorted(f['markers']), 'classes': 0}
            continue
        if root in inst and not inst[root]['single']:
            # two libraries claim the same root: keep the one with more markers
            if len(inst[root]['markers']) >= len(f['markers']):
                continue
        inst[root] = {'lib': lib, 'original': f['roots'][0], 'relocated': root not in f['roots'],
                      'single': False, 'markers': sorted(f['markers']), 'classes': 0}
    # count classes per instance (longest root wins)
    roots = sorted(inst.items(), key=lambda kv: -len(kv[0]))
    for c in names:
        for root, i in roots:
            if i['single']:
                if c == root or c.startswith(root + '$'):
                    i['classes'] += 1
                    break
            elif c.startswith(root):
                i['classes'] += 1
                break
    return inst


# --------------------------------------------------------------------------------------------
# Driver
# --------------------------------------------------------------------------------------------

_API = None


def _init_worker():
    global _API
    _API = API()


def _work(p):
    try:
        r = scan_jar(p, _API)
    except Exception as e:
        r = {'jar': os.path.basename(p.get('path', '?')), 'name': p.get('name'), 'source': p.get('source'),
             'error': '%s: %s' % (type(e).__name__, e), 'trace': traceback.format_exc()[-2000:]}
    return json.dumps(r, ensure_ascii=True, separators=(',', ':'), default=_json_default)


def _json_default(o):
    if isinstance(o, set):
        return sorted(o)
    if isinstance(o, (Arr, SB, New)):
        return render(o)
    return str(o)


def main():
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument('--dir', required=True)
    ap.add_argument('--spigot-api', required=True)
    ap.add_argument('--jobs', type=int, default=3)
    ap.add_argument('--only', nargs='*')
    ap.add_argument('--out')
    args = ap.parse_args()
    set_paths(args.dir, args.spigot_api, args.out)
    with open(CORPUS) as f:
        corpus = json.load(f)
    plugins = corpus['plugins']
    for p in plugins:
        p['path'] = os.path.join(JARS_DIR, os.path.basename(p['path']))
    if args.only:
        plugins = [p for p in plugins if any(s.lower() in (p.get('name') or '').lower() or s in p['path'] for s in args.only)]
    order = {p['path']: i for i, p in enumerate(plugins)}
    api = API()   # build caches once in the parent
    del api
    work = sorted(plugins, key=lambda p: -(p.get('bytes') or 0))
    tmp = OUT + '.part'
    results = {}
    t0 = time.time()
    if args.jobs <= 1:
        _init_worker()
        it = map(_work, work)
        pool = None
    else:
        import multiprocessing as mp
        pool = mp.get_context('fork').Pool(args.jobs, initializer=_init_worker, maxtasksperchild=40)
        it = pool.imap_unordered(_work, work, chunksize=1)
    with open(tmp, 'w') as f:
        for i, line in enumerate(it):
            f.write(line + '\n')
            f.flush()
            rec = json.loads(line)
            results[rec['jar']] = line
            if (i + 1) % 10 == 0 or i + 1 == len(work):
                print('%d/%d %.0fs %s' % (i + 1, len(work), time.time() - t0, rec.get('name')), flush=True)
    if pool is not None:
        pool.close()
        pool.join()
    by_jar = {os.path.basename(p['path']): p['path'] for p in plugins}
    lines = sorted(results.items(), key=lambda kv: order.get(by_jar.get(kv[0], ''), 1 << 30))
    with open(OUT, 'w') as f:
        for _, line in lines:
            f.write(line + '\n')
    os.remove(tmp)
    print('wrote %d jars to %s in %.0fs' % (len(lines), OUT, time.time() - t0))


if __name__ == '__main__':
    main()
