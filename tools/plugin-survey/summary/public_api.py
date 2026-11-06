# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# Level 1: the public methods, fields and types of spigot-api 1.20.1 that the plugins use, counted
# at the declaring type, the events they listen to, and the org.bukkit members they use that
# spigot-api 1.20.1 lacks.
#
#     python3 -I public_api.py <survey dir> <out dir>
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
def resolve(t, key, is_field):
    """declaring API type of a member written against type t, walking supertypes"""
    seen = set(); stack = [t]
    while stack:
        c = stack.pop(0)
        if c in seen or c not in API:
            continue
        seen.add(c)
        d = API[c]['f' if is_field else 'm']
        if key in d:
            return c, d[key]
        stack.append(API[c]['s'])
        stack.extend(API[c]['i'])
    return None, None
pub_methods = {(c, k) for c, v in API.items() if v['a'] & 1 for k, a in v['m'].items() if a & 1 and not k.startswith('<clinit>')}
pub_fields = {(c, k) for c, v in API.items() if v['a'] & 1 for k, a in v['f'].items() if a & 1}
pub_types = {c for c, v in API.items() if v['a'] & 1}
used_m = collections.defaultdict(set); used_f = collections.defaultdict(set); used_t = collections.defaultdict(set)
missing = collections.defaultdict(set); missing_name = collections.defaultdict(set)
events = collections.defaultdict(set); ev_src = {}
listeners = set(); jars_missing = set()
for r in P_.records():
    jar = r['jar']; b = r['bukkit_api']
    for m in b['members']:
        t, _, rest = m.partition('.')
        is_field = ':' in rest.split('(')[0]
        key = rest.replace(':', ':', 1)
        c, acc = resolve(t, rest, is_field)
        if c is None:
            continue
        if is_field:
            if (c, rest) in pub_fields: used_f[(c, rest)].add(jar)
        else:
            if (c, rest) in pub_methods: used_m[(c, rest)].add(jar)
        used_t[c].add(jar)
    for t in b['types']:
        if t in pub_types:
            used_t[t].add(jar)
    for m in b['missing_in_spigot_api']:
        missing[m].add(jar)
        t, _, rest = m.partition('.')
        missing_name[t + '.' + rest.split('(')[0].split(':')[0]].add(jar)
        jars_missing.add(jar)
    for e, ev in b["events"].items():
        src = ev[1]
        events[e].add(jar); ev_src[e] = src
        listeners.add(jar)
print('public methods', len(pub_methods), 'used', len(used_m), 'by >=2', sum(1 for v in used_m.values() if len(v) >= 2), '>=10', sum(1 for v in used_m.values() if len(v) >= 10), '>=50', sum(1 for v in used_m.values() if len(v) >= 50))
print('public fields', len(pub_fields), 'used', len(used_f))
print('public types', len(pub_types), 'used (members or type refs)', len(used_t))
# events: concrete subclasses of Event in the inventory
def is_event(c):
    seen = set()
    while c and c in API and c not in seen:
        seen.add(c)
        if c == 'org/bukkit/event/Event':
            return True
        c = API[c]['s']
    return False
ev_all = [c for c in API if c != 'org/bukkit/event/Event' and is_event(c)]
ev_conc = [c for c in ev_all if not API[c]['a'] & 0x0400]
used_conc = [c for c in ev_conc if c in events]
print('event types', len(ev_all), 'concrete', len(ev_conc), 'concrete listened', len(used_conc), 'any listened', sum(1 for c in ev_all if c in events), 'jars listening to anything', len(listeners))
print('never listened concrete', sorted(c.rsplit('/', 1)[1] for c in ev_conc if c not in events))
print('missing members: by signature', len(missing), 'by name', len(missing_name), 'jars', len(jars_missing))
print('top missing', [(k.rsplit('/', 1)[1], len(v)) for k, v in sorted(missing_name.items(), key=lambda x: -len(x[1]))[:12]])
print('top events', [(k.rsplit('/', 1)[1], len(v)) for k, v in sorted(events.items(), key=lambda x: -len(x[1])) if ev_src.get(k) == 'spigot-api'][:8])
top = sorted(used_m.items(), key=lambda x: -len(x[1]))[:8]
print('top methods', [(c.rsplit('/', 1)[1] + '.' + k.split('(')[0], len(v)) for (c, k), v in top])
