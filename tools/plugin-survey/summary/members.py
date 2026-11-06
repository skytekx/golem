# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# The plugins that reach the named members of CraftBukkit and of the API jar, directly and by
# reflection, on all code paths and on the paths that run on 1.20.1.
#
#     python3 -I members.py <survey dir> <out dir>
import collections
import json
import os
import re
import sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common  # noqa: E402
from common import tag_ok  # noqa: E402,F401

P_ = common.paths(None)
M = {
 'CraftPlayer.getHandle': (r'CraftPlayer\b|<class of org\.bukkit\.entity\.Player>', 'getHandle'),
 'CraftEntity.getHandle': (r'CraftEntity\b|<class of org\.bukkit\.entity\.Entity>', 'getHandle'),
 'CraftWorld.getHandle': (r'CraftWorld\b|<class of org\.bukkit\.World>', 'getHandle'),
 'CraftItemStack.asNMSCopy': (r'CraftItemStack\b', 'asNMSCopy'),
 'CraftItemStack.asCraftMirror': (r'CraftItemStack\b', 'asCraftMirror'),
 'CraftItemStack.asBukkitCopy': (r'CraftItemStack\b', 'asBukkitCopy'),
 'SimpleCommandMap.knownCommands': (r'SimpleCommandMap|CraftCommandMap|<class of org\.bukkit\.command\.(CommandMap|SimpleCommandMap)>', 'knownCommands'),
 'CraftServer.commandMap': (r'CraftServer\b|<class of org\.bukkit\.Server>', 'commandMap'),
 'CraftServer.getCommandMap': (r'CraftServer\b|<class of org\.bukkit\.Server>', 'getCommandMap'),
 'CraftServer.syncCommands': (r'CraftServer\b|<class of org\.bukkit\.Server>', 'syncCommands'),
 'CraftServer.getServer': (r'CraftServer\b|<class of org\.bukkit\.Server>', 'getServer'),
 'skull profile field (CraftMetaSkull or CraftMetaItem)': (r'CraftMetaSkull|CraftMetaItem\b|<class of org\.bukkit\.inventory\.meta\.(SkullMeta|ItemMeta)>', 'profile'),
 'CraftMetaSkull.profile named': (r'CraftMetaSkull|<class of org\.bukkit\.inventory\.meta\.SkullMeta>', 'profile'),
 'CraftMetaItem.profile named': (r'CraftMetaItem\b|<class of org\.bukkit\.inventory\.meta\.ItemMeta>', 'profile'),
 'SimplePluginManager.commandMap': (r'SimplePluginManager|<class of org\.bukkit\.plugin\.PluginManager>', 'commandMap'),
 'SimplePluginManager.dependencyGraph': (r'SimplePluginManager|<class of org\.bukkit\.plugin\.PluginManager>', 'dependencyGraph'),
 'CraftBossBar.handle': (r'CraftBossBar|<class of org\.bukkit\.boss\.BossBar>', 'handle'),
 'CraftHumanEntity.perm': (r'CraftHumanEntity|<class of org\.bukkit\.entity\.(HumanEntity|Player)>', 'perm'),
 'CraftEntity.perm': (r'CraftEntity\b|<class of org\.bukkit\.entity\.Entity>', 'perm'),
 'ServerCommandSender.perm': (r'ServerCommandSender', 'perm'),
 'CraftRegistry.getMinecraftRegistry': (r'CraftRegistry', 'getMinecraftRegistry'),
 'CraftMagicNumbers.getMaterial': (r'CraftMagicNumbers', 'getMaterial'),
 'Material.BY_NAME': (r'org\.bukkit\.Material\b|org/bukkit/Material\b', 'BY_NAME'),
}
MEMRE = re.compile(r'[.#]([A-Za-z_$][\w$]*)(?:\(|\.?$)')
def site_text(s):
    return (s.get('target') or '') + ' ' + (s.get('name') or '')
def site_member(s):
    if s.get('name') and '{' not in s['name']:
        return s['name']
    t = (s.get('target') or '').strip()
    t = re.sub(r'\(.*\)$', '', t).rstrip('.')
    return t.rsplit('.', 1)[-1] if '.' in t else None
res = {k: {'any': set(), 'on': set(), 'direct_on': set(), 'refl_on': set()} for k in M}
for r in P_.records():
    jar = r['jar']
    directs = []
    for sec in ('fields', 'methods'):
        for e in r['craftbukkit'][sec]:
            directs.append(e)
    for k, (cre, mem) in M.items():
        for e in directs:
            ref = e['ref']
            cls, _, rest = ref.partition('.')
            name = re.split(r'[(:]', rest)[0]
            if name == mem and re.search(cre.replace('\\.', '[./]'), cls.replace('/', '.')):
                res[k]['any'].add(jar)
                if e['on_1_20_1']:
                    res[k]['on'].add(jar); res[k]['direct_on'].add(jar)
        for s in r['reflection']['sites']:
            if s['kind'] not in ('craftbukkit', 'bukkit-impl', 'bukkit-api') or s.get('template'):
                continue
            if site_member(s) != mem:
                continue
            if not re.search(cre, site_text(s)):
                continue
            res[k]['any'].add(jar)
            if tag_ok(s.get('v')):
                res[k]['on'].add(jar); res[k]['refl_on'].add(jar)
    # direct API-field references (protected fields reached by subclasses)
    for m in r['bukkit_api']['members']:
        for k, (cre, mem) in M.items():
            cls, _, rest = m.partition('.')
            if re.split(r'[(:]', rest)[0] == mem and re.search(cre.replace('\\.', '[./]'), cls.replace('/', '.')):
                res[k]['any'].add(jar)
                res[k]['on'].add(jar); res[k]['direct_on'].add(jar)
for k, v in res.items():
    print('%-55s any %3d  on1.20.1 %3d  direct %3d  refl %3d' % (k, len(v['any']), len(v['on']), len(v['direct_on']), len(v['refl_on'])))
