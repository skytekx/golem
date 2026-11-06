# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# The stand-in groups of level 2: per plugin, the CraftBukkit classes and members and the
# internals of the API jar it reaches, sorted into the groups of the object they wrap, on all
# code paths and on the paths that run on 1.20.1. Writes groups.json.
#
#     python3 -I groups.py <survey dir> <out dir>
import collections
import json
import os
import re
import sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common  # noqa: E402
from common import tag_ok  # noqa: E402,F401

P_ = common.paths(None)
API = json.load(open(P_.api))
LOOKUP = ('Class.getDeclaredField', 'Class.getField', 'Class.getDeclaredMethod', 'Class.getMethod', 'Lookup.find',
          'Class.getDeclaredConstructor', 'Class.getConstructor')
IMPL = {'Server': 'CraftServer', 'plugin.PluginManager': '@SimplePluginManager', 'plugin.SimplePluginManager': '@SimplePluginManager', 'command.CommandMap': 'CraftCommandMap',
        'entity.Player': 'entity/CraftPlayer', 'entity.HumanEntity': 'entity/CraftHumanEntity', 'entity.Entity': 'entity/CraftEntity',
        'entity.LivingEntity': 'entity/CraftLivingEntity', 'World': 'CraftWorld', 'Chunk': 'CraftChunk',
        'inventory.meta.ItemMeta': 'inventory/CraftMetaItem', 'inventory.meta.SkullMeta': 'inventory/CraftMetaSkull',
        'inventory.ItemStack': 'inventory/CraftItemStack', 'inventory.Inventory': 'inventory/CraftInventory',
        'inventory.InventoryView': 'inventory/CraftInventoryView', 'boss.BossBar': 'boss/CraftBossBar', 'block.Skull': 'block/CraftSkull',
        'block.Block': 'block/CraftBlock', 'block.BlockState': 'block/CraftBlockState', 'OfflinePlayer': 'CraftOfflinePlayer',
        'scoreboard.Scoreboard': 'scoreboard/CraftScoreboard', 'map.MapView': 'map/CraftMapView', 'block.data.BlockData': 'block/data/CraftBlockData',
        'command.CommandSender': 'command/ServerCommandSender', 'command.ConsoleCommandSender': 'command/ServerCommandSender',
        'help.HelpMap': 'help/SimpleHelpMap', 'scheduler.BukkitScheduler': 'scheduler/CraftScheduler', 'inventory.ItemFactory': 'inventory/CraftItemFactory',
        'persistence.PersistentDataContainer': 'persistence/CraftPersistentDataContainer', 'entity.Item': 'entity/CraftItem',
        'entity.ArmorStand': 'entity/CraftArmorStand', 'UnsafeValues': 'util/CraftMagicNumbers'}
known = {}
def g_of(cls, mem):
    """group of a CraftBukkit class path (version segment removed) or '@API' class and member"""
    c = cls; m = mem or ''
    if m in ('perm', 'blockPermInst', 'getPermissibleBase') or c in ('@PermissibleBase', '@PermissionAttachment', '@Permission') or (c == '@SimplePluginManager' and m in ('permissions', 'permSubs', 'defaultPerms', 'defSubs')):
        return 'permissible'
    if m in ('profile', 'setProfile', 'getProfile', 'serializedProfile') or c.startswith('profile/') or c in ('CraftOfflinePlayer',) and 'rofile' in m:
        return 'profiles'
    if c in ('CraftServer',) and m in ('commandMap', 'getCommandMap', 'syncCommands', 'helpMap', 'getHelpMap') or c.startswith(('command/', 'help/')) or c in ('CraftCommandMap', '@SimpleCommandMap', '@PluginCommand', '@Command', '@FormattedCommandAlias') or (c == '@SimplePluginManager' and m == 'commandMap'):
        return 'commands'
    if c == 'CraftServer':
        return 'server'
    if c.startswith('@'):
        if c in ('@SimplePluginManager', '@JavaPlugin', '@PluginClassLoader', '@RegisteredListener', '@JavaPluginLoader', '@PluginDescriptionFile', '@HandlerList', '@StandardMessenger', '@PluginLogger', '@LibraryLoader'):
            return 'plugin manager'
        return 'API jar other'
    if c.startswith('entity/') or c in ('CraftOfflinePlayer',):
        return 'entities'
    if c.startswith(('inventory/CraftItemStack', 'inventory/CraftMeta', 'inventory/SerializableMeta', 'inventory/CraftItemFactory', 'persistence/')):
        return 'items'
    if c.startswith('inventory/'):
        return 'inventories'
    if c in ('CraftWorld', 'CraftChunk', 'CraftWorldBorder') or c.startswith(('block/', 'generator/')) or c in ('util/CraftLocation', 'util/RandomSourceWrapper', 'util/CraftRayTraceResult', 'CraftFluidCollisionMode'):
        return 'worlds'
    if c.startswith(('util/CraftMagicNumbers', 'util/CraftNamespacedKey', 'enchantments/', 'potion/', 'attribute/', 'legacy/', 'util/CraftLegacy', 'tag/', 'util/CraftSpawnCategory')) or c in ('CraftRegistry', 'CraftParticle', 'CraftSound', 'CraftArt', 'CraftEquipmentSlot', 'CraftEffect', 'CraftStatistic', 'CraftLootTable'):
        return 'conversions'
    if c.startswith('boss/'):
        return 'boss bars'
    if c.startswith('util/CraftChatMessage'):
        return 'chat'
    if c.startswith('event/'):
        return 'event factory'
    if c.startswith('map/'):
        return 'maps'
    if c.startswith('scheduler/'):
        return 'scheduler'
    if c.startswith('scoreboard/'):
        return 'scoreboards'
    if c.startswith('libs/'):
        return None
    return 'other'
CBPATH = re.compile(r'org/bukkit/craftbukkit/(?:v\d+_\d+_R\d+/)?([\w/$]+?)(?:\.([\w$<>]+))?(?:[(:]|$)')
def cb_from_direct(ref):
    m = CBPATH.match(ref)
    return (m.group(1), m.group(2)) if m else (None, None)
SUBPK = r'(?:entity|inventory|block|block\.data|command|help|util|boss|persistence|event|map|scheduler|scoreboard|potion|enchantments|attribute|generator|profile|legacy|tag|metadata|inventory\.view)'
TOK = re.compile(r'(?:(' + SUBPK + r')\.)?\b([A-Z][\w$]*)')
def site_member(s):
    if s.get('name') and '{' not in s['name']:
        return s['name']
    t = (s.get('target') or '').strip()
    t = re.sub(r'\(.*\)\s*$', '', t).rstrip('.')
    t = t.split(' ')[-1]
    return t.rsplit('.', 1)[-1] if '.' in t else None
recs = list(P_.records())
for r in recs:
    for sec in ('classes', 'fields', 'methods'):
        for e in r['craftbukkit'][sec]:
            c, _ = cb_from_direct(e['ref'])
            if c:
                known[c.split('/')[-1]] = c
for extra in ('CraftOfflinePlayer', 'CraftCommandMap', 'inventory/CraftMetaItem', 'inventory/CraftMetaSkull', 'scheduler/CraftScheduler',
              'inventory/CraftItemFactory', 'inventory/CraftMetaBook', 'block/CraftSign', 'util/CraftChatMessage', 'CraftEffect'):
    known[extra.split('/')[-1]] = extra
def cb_classes_in(target):
    out = []
    # the scan writes a char appended to a name by its code, so '{pkg}' + '.' + 'CraftServer' reads
    # {pkg}46CraftServer, and 46 is the dot
    target = re.sub(r'([})])46(?=[A-Za-z])', r'\1.', target)
    t = target.split('(')[0] if not target.startswith('(') else target
    for pk, name in TOK.findall(target):
        base = name.split('$')[0]
        if base in known:
            out.append(known[base])
        elif base.startswith('Craft'):
            out.append(((pk.replace('.', '/') + '/') if pk else '') + base)
    return out
res = {}
for r in recs:
    jar = r['jar']
    hits_on, hits_any = collections.defaultdict(set), collections.defaultdict(set)
    def add(g, on, key):
        if g is None:
            return
        hits_any[g].add(key)
        if on:
            hits_on[g].add(key)
    for sec in ('classes', 'fields', 'methods'):
        for e in r['craftbukkit'][sec]:
            if sec == 'classes' and not e.get('cp_class', True):
                continue
            c, m = cb_from_direct(e['ref'])
            if not c:
                continue
            add(g_of(c, m), e['on_1_20_1'], c + '.' + (m or ''))
    for s in r['reflection']['sites']:
        k = s['kind']
        if s.get('template'):
            continue
        on = tag_ok(s.get('v'))
        mem = site_member(s)
        tgt = s.get('target') or ''
        if k == 'craftbukkit':
            cs = cb_classes_in(tgt)
            for c in cs:
                m2 = mem if mem and mem != c.split('/')[-1] and not mem.startswith(('|', '{')) and '>' not in mem else None
                if c == 'CraftServer' and not m2:
                    add('identity (CraftServer class lookup only)', on, 'CraftServer')
                    continue
                add(g_of(c, m2), on, c + '.' + (m2 or ''))
        elif k == 'bukkit-impl' and s['call'].startswith(LOOKUP) and s['call'] != 'Class.forName' and not s.get('api_public'):
            m_ = re.search(r'<class of org\.bukkit\.([\w.$]+)>', tgt)
            api = m_.group(1) if m_ else None
            c = IMPL.get(api)
            if api and api.startswith(('plugin.Plugin', 'plugin.java.JavaPlugin', 'event.', 'configuration.', 'inventory.InventoryHolder', 'inventory.Recipe', 'command.CommandExecutor', 'command.TabCompleter', 'command.Command')) and c is None:
                add('plugin classes (not internals)', on, 'impl:' + api + '.' + (mem or ''))
                continue
            if c is None:
                c = 'impl:' + (api or '?')
            if c.startswith('@'):
                add(g_of(c, mem), on, c + '.' + (mem or ''))
            elif c.startswith('impl:'):
                add('other', on, c + '.' + (mem or ''))
            else:
                add(g_of(c, mem), on, c + '.' + (mem or ''))
        elif k == 'bukkit-api' and s['call'].startswith(('Class.getDeclared', 'Lookup.find', 'Class.getField', 'Class.getMethod', 'MethodHandles.privateLookupIn')):
            t = tgt.replace('.', '/')
            if '{' in t or '(' in t or not t.startswith('org/bukkit/'):
                continue
            info = API.get(t)
            if info is None:
                continue
            simple = t.rsplit('/', 1)[-1]
            if 'Constructor' in s['call'] or (s.get('name') == '<init>'):
                ctors = [a for kk, a in info['m'].items() if kk.startswith('<init>')]
                if ctors and not all(a & 1 for a in ctors):
                    add(g_of('@' + simple, '<init>'), on, '@' + simple + '.<init>')
                continue
            if not mem:
                continue
            if 'Field' in s['call'] or 'Getter' in s['call'] or 'Setter' in s['call'] or 'VarHandle' in s['call']:
                accs = [a for kk, a in info['f'].items() if kk.split(':', 1)[0] == mem]
            else:
                accs = [a for kk, a in info['m'].items() if kk.split('(', 1)[0] == mem]
            if accs and not all(a & 1 for a in accs):
                add(g_of('@' + simple, mem), on, '@' + simple + '.' + mem)
    res[jar] = {'name': r['name'], 'on': {g: sorted(v) for g, v in hits_on.items()}, 'any': {g: sorted(v) for g, v in hits_any.items()}}
P_.save('groups.json', res)
G = collections.Counter(); GA = collections.Counter()
for v in res.values():
    G.update(v['on'].keys()); GA.update(v['any'].keys())
print('plugins reaching any group on 1.20.1:', sum(1 for v in res.values() if v['on']))
for g, n in G.most_common():
    print('%-16s on %3d  any %3d' % (g, n, GA[g]))
