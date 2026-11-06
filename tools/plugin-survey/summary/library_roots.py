# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# The roots of shaded libraries in every jar, found by their marker classes in the entry names,
# nested jars included, even when relocated. Writes libroots.json.
#
# The jars are untrusted data. This step reads only their entry names, in memory, and runs
# nothing.
#
#     python3 -I library_roots.py <survey dir> <out dir>
import collections
import io
import json
import os
import re
import sys
import zipfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common  # noqa: E402
from common import tag_ok  # noqa: E402,F401

P_ = common.paths(None)
c = json.load(open(P_.corpus))
# (library, regex on the class path, group 1 = root)
MARK = [
 ('adventure-platform-bukkit', r'^(.*?platform/bukkit/)(?:MinecraftReflection|BukkitAudiences|CraftBukkitFacet)\.class$'),
 ('xseries', r'^(.*/)XMaterial\.class$'),
 ('nbt-api', r'^(.*/)NBTItem\.class$'),
 ('acf', r'^(.*/)BukkitCommandManager\.class$'),
 ('cloud', r'^(.*?cloud/)(?:.*/)?(?:BukkitCommandManager|CommandManager)\.class$'),
 ('lamp', r'^(.*?revxrsal/commands/)'),
 ('commodore', r'^(.*?commodore/)Commodore\.class$'),
 ('commandapi', r'^(.*?commandapi/)CommandAPI\.class$'),
 ('anvilgui', r'^(.*?anvilgui/)AnvilGUI\.class$'),
 ('packetevents', r'^(.*?packetevents/)'),
 ('triumph-gui', r'^(.*/)guis/BaseGui\.class$'),
 ('fastboard', r'^(.*/)FastBoard(?:Base)?\.class$'),
 ('bstats', r'^(.*?bstats/)'),
 ('bstats', r'^(.*/)Metrics\$(?:MetricsBase|CustomChart|SimplePie)\.class$'),
 ('protocollib-api', r'^(com/comphenix/protocol/)'),
 ('nbt-api', r'^(.*?nbtapi/)'),
 ('foliaLib', r'^(.*?tcoded/folialib/)'),
 ('axapi', r'^(.*?axapi/)'),
]
MARK = [(l, re.compile(p)) for l, p in MARK]
res = {}
def walk(zf, prefix, depth, roots, counts):
    for n in zf.namelist():
        if n.endswith('.class'):
            counts[0] += 1
            for lib, rx in MARK:
                m = rx.match(n)
                if m:
                    roots.setdefault(lib, set()).add(prefix + m.group(1))
        elif n.endswith('.jar') and depth < 2:
            try:
                info = zf.getinfo(n)
                if info.file_size < 60 * 1024 * 1024:
                    with zf.open(info) as fh:
                        walk(zipfile.ZipFile(io.BytesIO(fh.read())), prefix + n + '!', depth + 1, roots, counts)
            except Exception:
                pass
for p in c['plugins']:
    jar = os.path.basename(p['path'])
    roots = {}; counts = [0]
    with zipfile.ZipFile(P_.jar(p)) as zf:
        walk(zf, '', 0, roots, counts)
    # keep the shortest roots only (drop roots nested in another root of the same library)
    keep = {}
    for lib, rs in roots.items():
        rs = sorted(rs, key=lambda r: (len(r), r))
        out = []
        for r_ in rs:
            if not any(r_.startswith(o) for o in out):
                out.append(r_)
        keep[lib] = out
    res[jar] = keep
P_.save('libroots.json', res)
cnt = collections.Counter(l for v in res.values() for l in v)
print(sorted(cnt.items(), key=lambda x: -x[1]))
