# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# Shaded libraries as the reconciled data detects them: their carriers, and the carriers whose
# copy reaches CraftBukkit or Minecraft, on all code paths and on 1.20.1.
#
#     python3 -I libraries.py <survey dir> <out dir>
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
car = collections.defaultdict(set); car_entries = collections.Counter()
reach_any = collections.defaultdict(set); reach_on = collections.defaultdict(set); reach_on_nms = collections.defaultdict(set)
roots_single = collections.defaultdict(set)
for r in P_.records():
    jar = r['jar']
    roots = []
    for x in r['libraries']['shaded']:
        car[x['lib']].add(jar); car_entries[x['lib']] += 1
        if x.get('single_class'):
            roots_single[x['lib']].add(jar)
        roots.append((x['root'], x['lib']))
        if x['internal_class_refs'] and any(k in ('craftbukkit', 'nms') for k in x['internal_class_refs']):
            reach_any[x['lib']].add(jar)
    roots.sort(key=lambda t: -len(t[0]))
    def lib_of(cls):
        for root, lib in roots:
            if cls.startswith(root):
                return lib
        return None
    for s in r['reflection']['sites']:
        k = s['kind']
        if s.get('template'):
            continue
        if k in ('craftbukkit', 'nms') or (k == 'bukkit-impl' and s['call'].startswith(LOOKUP) and not s.get('api_public')):
            lib = lib_of(s['in'])
            if lib:
                reach_any[lib].add(jar)
                if tag_ok(s.get('v')):
                    reach_on[lib].add(jar)
                    if k == 'nms':
                        reach_on_nms[lib].add(jar)
    for g in ('craftbukkit', 'minecraft'):
        for sec in ('classes', 'fields', 'methods'):
            for e in r[g][sec]:
                for ex in e.get('ex', []):
                    lib = lib_of(ex)
                    if lib:
                        reach_any[lib].add(jar)
                        if e['on_1_20_1']:
                            reach_on[lib].add(jar)
                            if g == 'minecraft':
                                reach_on_nms[lib].add(jar)
rows = sorted(car, key=lambda l: (-len(reach_on[l]), -len(car[l])))
print('%-34s %8s %8s %10s %10s %10s' % ('lib', 'carriers', 'single', 'reach_any', 'reach_1201', 'nms_1201'))
for l in rows:
    if len(car[l]) >= 5 or reach_on[l]:
        print('%-34s %8d %8d %10d %10d %10d' % (l, len(car[l]), len(roots_single[l]), len(reach_any[l]), len(reach_on[l]), len(reach_on_nms[l])))
