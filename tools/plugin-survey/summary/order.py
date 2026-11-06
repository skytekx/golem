# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# The requirement set of every plugin and the greedy order of work: each step takes the work item
# that completes the most plugins, and a plugin works once every item it needs is done. A plugin
# that patches server classes, carries no code for 1.20.1, names a main class its jar lacks or
# ships a plugin.yml that YAML refuses stays out, since it fails on Spigot 1.20.1 as well. Reads
# levels.json, groups.json and jars_facts.json and writes order.json.
#
#     python3 -I order.py <survey dir> <out dir> --srg <bukkit_srg.srg>
import collections
import json
import os
import re
import sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common  # noqa: E402
from common import tag_ok  # noqa: E402,F401

P_ = common.paths(None, srg=True)
L = P_.load('levels.json')
G = P_.load('groups.json')
JF = P_.load('jars_facts.json')
SUP = {'1.13', '1.14', '1.15', '1.16', '1.17', '1.18', '1.19', '1.20'}
FREE = {'plugin manager', 'API jar other', 'identity (CraftServer class lookup only)', 'plugin classes (not internals)'}   # free when the stock API classes run and CraftServer sits in v1_20_R1
LIKELY_PAPER = {'FeatherMorph', 'FreedomChat', 'Simple Voice Chat Discord Bridge', 'Typewriter'}
ADD = {'getBukkitEntity', 'getBukkitView', 'getBukkitSender', 'getBukkitStack', 'nextContainerCounter', 'toBukkitRecipe', 'getCraftServer',
       'getBukkitLivingEntity', 'bukkitEntity', 'craftAttributes', 'getWorld', 'getServer', 'server', 'getBukkitCreature', 'getCraftWorld',
       'spawnReason', 'persistentDataContainer', 'getBukkitInventory', 'getBukkitRecipe', 'transaction', 'getContents', 'getViewers', 'getOwner',
       'onOpen', 'onClose', 'setMaxStackSize', 'getLocation', 'checkReachable', 'title', 'bukkitPickUpLoot', 'addEntity', 'collidableExemptions'}
# members of that list that vanilla also has, by owner, are excluded below through the 1.20.1 mapping
sp_members = collections.defaultdict(set)
cur = None
for line in open(P_.srg):
    if not line[:1].isspace():
        cur = line.split()[0]
    else:
        s = line.strip()
        if '->' in s:
            sp_members[cur].add(s.split()[-1])
        else:
            sp_members[cur].add(s.split()[0])
NETTY = ('net/minecraft/network/NetworkManager', 'net/minecraft/network/Connection', 'net/minecraft/server/network/ServerConnection', 'net/minecraft/server/network/ServerConnectionListener')
needs = {}; names = {}; additions = collections.Counter()
for r in P_.records():
    jar = r['jar']; names[jar] = r['name']
    nd = {'level 1'}
    if (JF[jar]['main_major'] or 0) > 61:
        nd.add('Java 21 or 25')
    # the api-version as Spigot reads the descriptor, by its byte order mark and YAML 1.1
    av = r['api_version'] if JF[jar]['descriptor_error'] else JF[jar]['descriptor_api_version']
    if av is not None and r['descriptor'] == 'plugin.yml' and str(av) not in SUP:
        nd.add('api-version above 1.20, or 1.20 read as 1.2')
    if r['descriptor'] == 'paper-plugin.yml':
        nd.add('Paper loader and API')
    if r['name'] in LIKELY_PAPER:
        nd.add('Paper loader and API')
    for g in G[jar]['on']:
        if g not in FREE:
            nd.add('L2 ' + g)
    if L[jar]['level_on'] == 3:
        nd.add('L3 remapper')
    # CraftBukkit additions to Minecraft classes, and the Netty pipeline
    add = False; netty = False
    for sec in ('fields', 'methods'):
        for e in r['minecraft'][sec]:
            if not e['on_1_20_1']:
                continue
            owner, _, rest = e['ref'].partition('.')
            nm = re.split(r'[(:]', rest)[0]
            anyowner = {'getBukkitEntity', 'getBukkitView', 'getBukkitSender', 'getBukkitStack', 'nextContainerCounter', 'toBukkitRecipe',
                        'getCraftServer', 'getBukkitLivingEntity', 'bukkitEntity', 'craftAttributes', 'getBukkitCreature', 'getBukkitInventory', 'getCraftWorld'}
            ok = nm in anyowner or (nm == 'getWorld' and owner.endswith(('/World', '/WorldServer', '/Level', '/ServerLevel'))) or \
                (nm in ('getServer', 'server') and owner.endswith(('/MinecraftServer', '/DedicatedServer')))
            if ok and nm not in sp_members.get(owner, set()):
                add = True; additions[nm] += 1
    for sec in ('classes', 'fields', 'methods'):
        for e in r['minecraft'][sec]:
            if e['on_1_20_1'] and e['ref'].startswith(NETTY):
                netty = True
    for s in r['reflection']['sites']:
        if s['kind'] == 'nms' and not s.get('template') and tag_ok(s.get('v')):
            t = (s.get('target') or '').replace('.', '/')
            if any(n in t for n in NETTY):
                netty = True
            nm = s.get('name')
            if nm in ADD and nm in ('getBukkitEntity', 'getBukkitView', 'getBukkitSender', 'nextContainerCounter', 'getBukkitStack', 'toBukkitRecipe', 'getBukkitLivingEntity', 'bukkitEntity'):
                add = True
    if add:
        nd.add('L3 CraftBukkit additions to Minecraft classes')
    if netty:
        nd.add('L3 Netty pipeline')
    if r['name'].startswith('Nova') and r['source'] == 'hangar':
        nd.add('patching of server classes')
    if r['name'].startswith(('GadgetsMenu', '\u2728Phoenix Crates')):
        nd.add('no adapter for 1.20.1')
    if not JF[jar]['main_found']:
        nd.add('no main class in the jar')
    if JF[jar]['descriptor_error']:
        nd.add('a plugin.yml that YAML refuses')
    needs[jar] = nd
units = collections.Counter(u for nd in needs.values() for u in nd)
works = set(); done = set(); order = []
OUT_OF_REACH = {'patching of server classes', 'no adapter for 1.20.1', 'no main class in the jar',
                'a plugin.yml that YAML refuses'}
todo = set(units) - OUT_OF_REACH
while todo:
    best = max(sorted(todo), key=lambda u: (sum(1 for j, nd in needs.items() if j not in works and nd <= done | {u}), units[u]))
    done.add(best); todo.discard(best)
    new = {j for j, nd in needs.items() if nd <= done} - works
    works |= new
    order.append((best, units[best], len(new), len(works)))
P_.save('order.json', {'needs': {j: sorted(v) for j, v in needs.items()}, 'order': order})
for i, (u, need, new, tot) in enumerate(order):
    print('%2d  %-50s need %3d  completes %3d  working %3d (%.1f %%)' % (i + 1, u, need, new, tot, 100.0 * tot / len(needs)))
print('left:', [names[j] for j in needs if j not in works])
print('additions found by name:', additions.most_common(12))
