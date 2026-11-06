# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# Looks Spigot 1.20.1 class and member names up in bukkit_srg.srg of Arclight 1.20.1. A class line
# holds the Spigot and the Mojang name, a method line under it, indented by a tab, the Spigot name,
# the descriptor and the SRG name, and a field line, indented by spaces, the type, the SRG name and
# the Spigot name after an arrow. A query is a class, or a class, a hash and a member, optionally
# followed by a space and the start of its descriptor or type.
#
#     python3 -I mapping_lookup.py <bukkit_srg.srg> 'net/minecraft/server/level/EntityPlayer#c'
import sys

if len(sys.argv) < 3:
    raise SystemExit("usage: mapping_lookup.py <bukkit_srg.srg> <class>[#<member>[ <descriptor start>]]...")
classes = {}
members = None
for line in open(sys.argv[1]):
    if not line.strip():
        continue
    if not line[:1].isspace():
        spigot, _, mojang = line.strip().partition(' ')
        members = classes.setdefault(spigot, {'mojang': mojang, 'members': []})['members']
        continue
    if members is None:
        continue
    words = line.split()
    if '->' in words:
        # a field: type, SRG name, arrow, Spigot name
        members.append((words[-1], words[0], words[1]))
    elif len(words) >= 3:
        # a method: Spigot name, descriptor, SRG name
        members.append((words[0], words[1], words[2]))
for query in sys.argv[2:]:
    name, _, member = query.partition('#')
    if name not in classes:
        print(query, '-> class missing')
        continue
    if not member:
        print(query, '-> class present, Mojang name', classes[name]['mojang'])
        continue
    member, _, start = member.partition(' ')
    hits = [m for m in classes[name]['members'] if m[0] == member and m[1].startswith(start)]
    print(query, '->', ['%s %s is %s' % hit for hit in hits[:4]] if hits else 'member missing')
