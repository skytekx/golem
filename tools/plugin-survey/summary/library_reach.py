# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# Shaded libraries with NBT-API's utils merged into NBT-API, XSeries held to its own classes and
# bStats counted from the entry names: their carriers, the copies that reach internals on
# 1.20.1, the plugins whose internal access on 1.20.1 lies only inside shaded libraries, and the
# level 3 plugins whose Minecraft access lies only there. Reads libroots.json and levels.json.
#
#     python3 -I library_reach.py <survey dir> <out dir>
import collections
import json
import os
import re
import sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common  # noqa: E402
from common import tag_ok  # noqa: E402,F401

P_ = common.paths(None)
LOOKUP = ('Class.getDeclaredField', 'Class.getField', 'Class.getDeclaredMethod', 'Class.getMethod', 'Lookup.find')
names = P_.load('libroots.json')
XS_SUB = {'reflection', 'profiles', 'particles', 'messages', 'base', 'art', 'skull', 'abstractions', 'minecraft', 'jvm', 'asm', 'aggregate'}
XS_NAMES = {'SkullUtils', 'ReflectionUtils', 'NMSExtras', 'Titles', 'ActionBar', 'SkullCacheListener', 'NoteBlockMusic', 'XReflection'}
def is_xseries(cls, root):
    rest = cls[len(root):]
    parts = rest.split('/')
    if len(parts) > 1:
        return parts[0] in XS_SUB
    simple = parts[0].split('.')[0].split('$')[0]
    return bool(re.match(r'^X[A-Z]', simple)) or simple in XS_NAMES
LIBS = {}
carry = collections.defaultdict(set); reach = collections.defaultdict(set); reach_nms = collections.defaultdict(set)
only_lib = collections.defaultdict(set); only_any_lib = set(); internal_on = set(); jar_name = {}
for r in P_.records():
    jar = r['jar']; jar_name[jar] = r['name']
    roots = []
    for x in r['libraries']['shaded']:
        lib = x['lib']
        if lib == 'bstats':
            continue
        if lib == 'foundation':
            parent = [y for y in r['libraries']['shaded'] if y['lib'] == 'nbt-api' and x['root'].startswith(y['root'])]
            if parent:
                lib = 'nbt-api'
        if x.get('single_class') and lib in ('anvilgui', 'fastboard', 'xseries'):
            # a single class of that name may be the plugin's own code, which stays counted
            pass
        roots.append((x['root'], lib))
        carry[lib].add(jar)
    if 'bstats' in names.get(jar, {}):
        carry['bstats (entry names)'].add(jar)
        if any('bstats/' in rt for rt in names[jar]['bstats']):
            carry['bstats (package path)'].add(jar)
    roots.sort(key=lambda t: -len(t[0]))
    def lib_of(cls):
        for root, lib in roots:
            if cls.startswith(root):
                if lib == 'xseries' and not is_xseries(cls, root):
                    return None
                return lib
        return None
    where = []   # (lib or None, is_nms) of each internal access that runs on 1.20.1
    for s in r['reflection']['sites']:
        k = s['kind']
        if s.get('template') or not tag_ok(s.get('v')):
            continue
        if k in ('craftbukkit', 'nms') or (k == 'bukkit-impl' and s['call'].startswith(LOOKUP) and not s.get('api_public')):
            where.append((lib_of(s['in']), k == 'nms'))
    for g in ('craftbukkit', 'minecraft'):
        for sec in ('classes', 'fields', 'methods'):
            for e in r[g][sec]:
                if not e['on_1_20_1'] or (sec == 'classes' and not e.get('cp_class', True)):
                    continue
                if g == 'craftbukkit' and '/craftbukkit/libs/' in e['ref']:
                    continue
                ls = {lib_of(x) for x in e.get('ex', [])}
                complete = len(e.get('ex', [])) >= e.get('classes', 1)
                if not complete:
                    ls.add(None)
                for l in ls:
                    where.append((l, g == 'minecraft'))
    if where:
        internal_on.add(jar)
        for l, nms in where:
            if l:
                reach[l].add(jar)
                if nms:
                    reach_nms[l].add(jar)
        libs = {l for l, _ in where}
        if None not in libs:
            only_any_lib.add(jar)
            if len(libs) == 1:
                only_lib[next(iter(libs))].add(jar)
print('plugins with internal access on 1.20.1', len(internal_on))
print('plugins whose internal access on 1.20.1 is only inside shaded libraries', len(only_any_lib))
print('%-28s %8s %12s %12s %14s' % ('library', 'carriers', 'reach 1.20.1', 'Minecraft', 'only internals'))
for l in sorted(carry, key=lambda l: (-len(reach[l]), -len(carry[l]))):
    if len(carry[l]) >= 5 or reach[l]:
        print('%-28s %8d %12d %12d %14d' % (l, len(carry[l]), len(reach[l]), len(reach_nms[l]), len(only_lib[l])))
by_set = collections.Counter(tuple(sorted({l for l in only_lib if j in only_lib[l]})) or ('several',) for j in only_any_lib)
print('only via libs, by lib set:', sorted(by_set.items(), key=lambda x: (-x[1], x[0])))
# level 3 only because of a library: Minecraft access on 1.20.1 lies only inside library roots
L = P_.load('levels.json')
nms_lib_only = collections.defaultdict(set); cnt = 0
for r in P_.records():
    jar = r['jar']
    if L[jar]['level_on'] != 3:
        continue
    roots = []
    for x in r['libraries']['shaded']:
        lib = x['lib']
        if lib == 'bstats':
            continue
        if lib == 'foundation' and any(y['lib'] == 'nbt-api' and x['root'].startswith(y['root']) for y in r['libraries']['shaded']):
            lib = 'nbt-api'
        roots.append((x['root'], lib))
    roots.sort(key=lambda t: -len(t[0]))
    def lib_of(cls):
        for root, lib in roots:
            if cls.startswith(root):
                if lib == 'xseries' and not is_xseries(cls, root):
                    return None
                return lib
        return None
    libs = set()
    for s in r['reflection']['sites']:
        if s['kind'] == 'nms' and not s.get('template') and tag_ok(s.get('v')):
            libs.add(lib_of(s['in']))
    for sec in ('classes', 'fields', 'methods'):
        for e in r['minecraft'][sec]:
            if e['on_1_20_1'] and (sec != 'classes' or e.get('cp_class', True)):
                ls = {lib_of(x) for x in e.get('ex', [])}
                if len(e.get('ex', [])) < e.get('classes', 1):
                    ls.add(None)
                libs |= ls
    if None not in libs:
        cnt += 1
        for l in libs:
            nms_lib_only[l].add(jar)
        if r['name'] in ('EssentialsX',):
            print('EssentialsX level 3 only through', libs)
print('level-3 plugins whose Minecraft access on 1.20.1 is only inside shaded libraries:', cnt)
print(sorted(((l, len(v)) for l, v in nms_lib_only.items()), key=lambda x: -x[1]))
