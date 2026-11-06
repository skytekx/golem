# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# Minecraft classes reached on 1.20.1 paths by the names written in the plugins, direct references
# with their descriptors and reflection targets, Spigot 1.20.1 names folded onto Mojang names
# through bukkit_srg.srg. Legacy flat names apart. Writes nmsrank.json.
#
#     python3 -I minecraft_rank.py <survey dir> <out dir> --srg <bukkit_srg.srg>
import collections
import json
import os
import re
import sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common  # noqa: E402
from common import tag_ok  # noqa: E402,F401

P_ = common.paths(None, srg=True)
sp2moj = {}
for line in open(P_.srg):
    if line[:1].isspace():
        continue
    a = line.split()
    if len(a) >= 2:
        sp2moj[a[0]] = a[1]
moj = set(sp2moj.values())
NAME = re.compile(r'net/minecraft/[\w/$]+')
LEG = re.compile(r'^net/minecraft/server/(?:v1_\d+_R\d+|\{[^}]*\})/')
plug = collections.defaultdict(set); direct = collections.defaultdict(set); refl = collections.defaultdict(set)
style = collections.Counter(); legacy_plugins = set(); unknown = collections.Counter(); any_on = set()
def canon(n):
    n = n.rstrip('/')
    if LEG.match(n):
        return None, 'legacy'
    if n in sp2moj:
        return sp2moj[n], ('same' if sp2moj[n] == n else 'spigot')
    if n in moj:
        return n, 'mojang'
    # inner or member tail: try trimming
    base = n.split('$')[0]
    if base in sp2moj:
        return sp2moj[base] + n[len(base):], 'spigot'
    if base in moj:
        return n, 'mojang'
    return n, 'unknown'
for r in P_.records():
    jar = r['jar']
    seen_d, seen_r = set(), set()
    for sec in ('classes', 'fields', 'methods'):
        for e in r['minecraft'][sec]:
            if not e['on_1_20_1']:
                continue
            owner = e['ref'].split('.')[0].split(':')[0]
            for n in {owner} | set(NAME.findall(e['ref'][len(owner):])):
                seen_d.add(n)
    for s in r['reflection']['sites']:
        if s['kind'] == 'nms' and not s.get('template') and tag_ok(s.get('v')):
            t = (s.get('target') or '').replace('.', '/')
            for n in NAME.findall(t):
                seen_r.add(n)
    hit = False
    for src, seen in (('d', seen_d), ('r', seen_r)):
        for n in seen:
            c, st = canon(n)
            if st == 'legacy':
                legacy_plugins.add(jar); continue
            if st == 'unknown':
                unknown[c] += 1
                # names of other versions (not in 1.20.1)
                continue
            hit = True
            plug[c].add(jar)
            (direct if src == 'd' else refl)[c].add(jar)
    if hit:
        any_on.add(jar)
rank = sorted(plug, key=lambda c: (-len(plug[c]), c))
unknown_top = sorted(unknown.items(), key=lambda x: (-x[1], x[0]))
P_.save('nmsrank.json', {'rank': [[c, len(plug[c]), len(direct[c]), len(refl[c])] for c in rank], 'unknown_top': unknown_top[:40]})
print('plugins reaching a 1.20.1 Minecraft class on 1.20.1 paths:', len(any_on), ' distinct 1.20.1 classes:', len(plug))
print('plugins with legacy flat-package lookups on 1.20.1 paths:', len(legacy_plugins))
inv = {v: k for k, v in sp2moj.items()}
for c in rank[:30]:
    print('%-75s %4d  direct %3d  refl %3d   spigot name %s' % (c, len(plug[c]), len(direct[c]), len(refl[c]), inv.get(c, c).rsplit('/', 1)[-1]))
print('names not in 1.20.1 (top):', [(k.rsplit('/', 1)[-1], v) for k, v in unknown_top[:12]])
