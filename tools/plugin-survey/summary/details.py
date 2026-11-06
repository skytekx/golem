# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# The counts of the page of the layer that no other step gives: the naming of Minecraft in the
# manifests and the references, the plugins others depend on, CraftBukkit's additions to
# Minecraft classes by member, the class names that mean different classes in Spigot and Mojang
# names, the flags of runtime patching and Java agents, plugin.yml libraries and the descriptors
# without api-version. Reads jars_facts.json.
#
#     python3 -I details.py <survey dir> <out dir> --srg <bukkit_srg.srg>
import collections
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common  # noqa: E402

P_ = common.paths(None, srg=True)
JF = P_.load('jars_facts.json')
DEPENDED = ('PlaceholderAPI', 'Vault', 'WorldGuard', 'ProtocolLib', 'Essentials', 'LuckPerms', 'Multiverse-Core',
            'WorldEdit', 'packetevents')
ADDITIONS = ('getBukkitEntity', 'getServer', 'nextContainerCounter', 'getWorld', 'server', 'getBukkitView',
             'getBukkitSender', 'toBukkitRecipe')

# members that vanilla has under the same Spigot name are no additions
spigot_members = collections.defaultdict(set)
spigot_to_mojang = {}
current = None
for line in open(P_.srg):
    if not line.strip():
        continue
    if not line[:1].isspace():
        words = line.split()
        current = words[0]
        if len(words) >= 2:
            spigot_to_mojang[words[0]] = words[1]
        continue
    words = line.split()
    spigot_members[current].add(words[-1] if '->' in words else words[0])

mojang_manifest = 0
styles = collections.Counter()
hard, soft = collections.Counter(), collections.Counter()
additions = collections.defaultdict(set)
patching = 0
agents = 0
libraries = 0
for r in P_.records():
    if (r['manifest'].get('paperweight-mappings-namespace') or {}).get('(jar)') == 'mojang':
        mojang_manifest += 1
    styles[r['minecraft']['naming'].get('style')] += 1
    for name in {d.lower() for d in r['depend'] or []}:
        hard[name] += 1
    for name in {d.lower() for d in r['softdepend'] or []}:
        soft[name] += 1
    for sec in ('fields', 'methods'):
        for e in r['minecraft'][sec]:
            if not e['on_1_20_1']:
                continue
            owner, _, rest = e['ref'].partition('.')
            name = re.split(r'[(:]', rest)[0]
            if name in ADDITIONS and name not in spigot_members.get(owner, set()):
                additions[owner.rsplit('/', 1)[1] + '.' + name].add(r['jar'])
    if r['verdict'].get('runtime_patching'):
        patching += 1
    if (r.get('patching') or {}).get('fable', {}).get('manifest_agent'):
        agents += 1
    if r['plugin_yml_libraries']:
        libraries += 1

print('jars that declare paperweight-mappings-namespace: mojang', mojang_manifest)
print('naming styles of Minecraft references', dict(styles))
print('depended on, hard plus soft', [(name, hard[name.lower()], soft[name.lower()]) for name in DEPENDED])
print('CraftBukkit additions to Minecraft classes, direct references on 1.20.1:')
for member, jars in sorted(additions.items(), key=lambda x: (-len(x[1]), x[0]))[:14]:
    print('   %-45s %3d' % (member, len(jars)))
clash = sorted(s for s, m in spigot_to_mojang.items() if s != m and s in set(spigot_to_mojang.values()))
print('mapping: classes', len(spigot_to_mojang), 'renamed', sum(1 for s, m in spigot_to_mojang.items() if s != m),
      'Spigot names that are another class in Mojang names', len(clash), [c.rsplit('/', 1)[1] for c in clash])
print('jars the judge pass flags for runtime patching', patching, ' jars that declare a Java agent', agents)
print('plugins with plugin.yml libraries', libraries)
print('plugin.yml without api-version', [v['name'] for v in JF.values()
                                         if v['descriptor'] == 'plugin.yml' and not v['descriptor_error']
                                         and v['descriptor_api_version'] is None])
