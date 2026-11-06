# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# Plugins that reach eight leading Minecraft classes on 1.20.1 under their names of 1.20.1, Spigot
# or Mojang, with the lookups of legacy flat package names apart.
#
#     python3 -I minecraft_classes.py <survey dir> <out dir>
import collections
import json
import os
import re
import sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common  # noqa: E402
from common import tag_ok  # noqa: E402,F401

P_ = common.paths(None)
Q = {
 'Packet': ['net/minecraft/network/protocol/Packet'],
 'Level (World)': ['net/minecraft/world/level/World', 'net/minecraft/world/level/Level'],
 'ServerLevel (WorldServer)': ['net/minecraft/server/level/WorldServer', 'net/minecraft/server/level/ServerLevel'],
 'ServerPlayer (EntityPlayer)': ['net/minecraft/server/level/EntityPlayer', 'net/minecraft/server/level/ServerPlayer'],
 'ItemStack': ['net/minecraft/world/item/ItemStack'],
 'CompoundTag (NBTTagCompound)': ['net/minecraft/nbt/NBTTagCompound', 'net/minecraft/nbt/CompoundTag'],
 'ServerGamePacketListenerImpl (PlayerConnection)': ['net/minecraft/server/network/PlayerConnection', 'net/minecraft/server/network/ServerGamePacketListenerImpl'],
 'Entity': ['net/minecraft/world/entity/Entity'],
}
legacy = collections.defaultdict(set)
res = {k: set() for k in Q}
for r in P_.records():
    cls_on = set()
    for sec in ('classes', 'fields', 'methods'):
        for e in r['minecraft'][sec]:
            if e['on_1_20_1'] and (sec != 'classes' or e.get('cp_class', True)):
                cls_on.add(e['ref'].split('.')[0].split(':')[0])
    txt = []
    for s in r['reflection']['sites']:
        if s['kind'] == 'nms' and not s.get('template') and tag_ok(s.get('v')):
            txt.append((s.get('target') or '').replace('.', '/'))
    for k, names in Q.items():
        if any(n in cls_on for n in names) or any(re.search(re.escape(n) + r'(?![\w$])', t) for t in txt for n in names):
            res[k].add(r['jar'])
        simple = names[0].rsplit('/', 1)[1]
        if any(re.search(r'net/minecraft/server/(?:v1_\d+_R\d+|\{ver\}|\{[^}]*\})/' + re.escape(simple) + r'(?![\w$])', t) for t in txt):
            legacy[k].add(r['jar'])
for k in Q:
    print('%-50s modern %3d   legacy flat-package lookups on 1.20.1 paths %3d   union %3d' % (k, len(res[k]), len(legacy[k]), len(res[k] | legacy[k])))
