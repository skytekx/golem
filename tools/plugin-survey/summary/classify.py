# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# One class per plugin on 1.20.1 paths, the first that applies: runtime patching, Paper only,
# level 3, level 2, level 1 with the internals of the API jar only, level 1. With the sources
# and the share of downloads. Reads levels.json and groups.json and writes classes.json.
#
#     python3 -I classify.py <survey dir> <out dir>
import collections
import json
import os
import re
import sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common  # noqa: E402
from common import tag_ok  # noqa: E402,F401

P_ = common.paths(None)
L = P_.load('levels.json')
G = P_.load('groups.json')
c = json.load(open(P_.corpus))
FREE = {'plugin manager', 'API jar other', 'identity (CraftServer class lookup only)', 'plugin classes (not internals)'}
w_all = {}; w_field = {}; src = {}; desc = {}
for p in c['plugins']:
    jar = os.path.basename(p['path'])
    per = collections.defaultdict(int)
    s0 = p['source'] if p['source'] != 'live' else ('modrinth' if 'modrinth.com' in (p.get('url') or '') else 'github')
    if p.get('downloads'): per[s0] = max(per[s0], p['downloads'])
    for a in p.get('also_in', []):
        if a.get('downloads'): per[a['source']] = max(per[a['source']], a['downloads'])
    w_all[jar] = sum(per.values()); src[jar] = p['source']
    d = p.get('downloads')
    if d is None:
        alts = [a.get('downloads') or 0 for a in p.get('also_in', [])]
        d = max(alts) if alts else 0
    w_field[jar] = d
    desc[jar] = 'paper-plugin.yml' if (p.get('paper_plugin_yml') and not p.get('plugin_yml')) else 'plugin.yml'
cls = {}
for jar, v in L.items():
    if v['name'] == 'Nova' and src[jar] == 'hangar':
        cls[jar] = '1 runtime patching of server classes'
    elif desc[jar] == 'paper-plugin.yml':
        cls[jar] = '2 Paper only: paper-plugin.yml without plugin.yml'
    elif v['level_on'] == 3:
        cls[jar] = '3 needs level 3 (Minecraft classes)'
    elif v['level_on'] == 2 and set(G[jar]['on']) - FREE:
        cls[jar] = '4 needs level 2 (CraftBukkit stand-ins)'
    elif v['level_on'] == 2:
        cls[jar] = '5 level 1 plus internals of the API jar only'
    else:
        cls[jar] = '6 level 1 only'
tw = sum(w_all.values()); tf = sum(w_field.values())
print('%-55s %5s %5s %5s %5s %5s %7s %10s %7s' % ('class', 'live', 'spig', 'modr', 'hang', 'jars', 'share', 'dl(all)', 'dlshare'))
for k in sorted(set(cls.values())):
    js = [j for j in cls if cls[j] == k]
    cc = collections.Counter(src[j] for j in js)
    print('%-55s %5d %5d %5d %5d %5d %6.1f%% %10d %6.1f%%' % (k, cc['live'], cc['spigot'], cc['modrinth'], cc['hangar'], len(js), 100.0 * len(js) / len(cls), sum(w_all[j] for j in js), 100.0 * sum(w_all[j] for j in js) / tw))
for lv in (1, 2, 3):
    js = [j for j in L if L[j]['level_on'] == lv]
    print('level %d on 1.20.1: %d jars, %.1f%% of downloads (all listings)' % (lv, len(js), 100.0 * sum(w_all[j] for j in js) / tw))
live = [(L[j]['name'], cls[j], L[j]['level_any'], L[j]['level_on']) for j in L if src[j] == 'live']
print(live)
# shares within each source's downloads
for s in ('spigot', 'modrinth', 'hangar'):
    js = [j for j in cls if src[j] == s]
    tot = sum(w_field[j] for j in js)
    print(s, {k[2:20]: round(100.0 * sum(w_field[j] for j in js if cls[j] == k) / tot, 1) for k in sorted(set(cls.values()))})
P_.save('classes.json', cls)
