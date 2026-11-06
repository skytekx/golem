# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# The level of every plugin, on all code paths and on the paths that run on 1.20.1. A plugin is
# level 3 when it reaches Minecraft classes, level 2 when it reaches CraftBukkit classes, the
# runtime class behind an API object by a member lookup or a non-public member of the API jar,
# and level 1 otherwise. Writes levels.json.
#
#     python3 -I levels.py <survey dir> <out dir>
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
          'MethodHandles.privateLookupIn', 'Class.getDeclaredConstructor', 'Class.getConstructor')
def api_nonpublic(target, name, call):
    if not target or not name or '{' in target or '(' in target or '{' in name:
        return None
    t = target.replace('.', '/')
    info = API.get(t)
    if info is None:
        parts = t.split('/')
        for k in range(len(parts) - 1, 0, -1):
            cand = '/'.join(parts[:k]) + '/' + '$'.join(parts[k:])
            if cand in API:
                info, t = API[cand], cand
                break
    if info is None:
        return None
    if 'Field' in call or 'Getter' in call or 'Setter' in call or 'VarHandle' in call:
        accs = [a for k, a in info['f'].items() if k.split(':', 1)[0] == name]
    else:
        accs = [a for k, a in info['m'].items() if k.split('(', 1)[0] == name]
    if not accs:
        return None
    return None if all(a & 1 for a in accs) else t.replace('/', '.') + '#' + name
out = {}
for r in P_.records():
    any_, on, on_tmpl = set(), set(), set()
    apipriv_any, apipriv_on = set(), set()
    for g, tag in (('craftbukkit', 'cb-direct'), ('minecraft', 'nms-direct')):
        for sec in ('classes', 'fields', 'methods'):
            for e in r[g][sec]:
                if sec == 'classes' and not e.get('cp_class', True):
                    continue
                if g == 'craftbukkit' and '/craftbukkit/libs/' in e['ref']:
                    continue
                any_.add(tag)
                if e['on_1_20_1']:
                    on.add(tag)
    for s in r['reflection']['sites']:
        k = s['kind']; call = s['call']
        t = None
        if k == 'craftbukkit':
            t = 'cb-refl'
        elif k == 'nms':
            t = 'nms-refl'
        elif k == 'bukkit-impl':
            if call.startswith(LOOKUP) and not s.get('api_public'):
                t = 'impl-refl'
        elif k == 'bukkit-api':
            if call.startswith(LOOKUP):
                m = api_nonpublic(s.get('target'), s.get('name'), call)
                if m:
                    t = 'api-private'
                    if not s.get('template'):
                        apipriv_any.add(m)
                        if tag_ok(s.get('v')):
                            apipriv_on.add(m)
        if not t:
            continue
        ok = tag_ok(s.get('v'))
        if ok:
            on_tmpl.add(t)
        if s.get('template'):
            continue
        any_.add(t)
        if ok:
            on.add(t)
    def lvl(rs):
        if rs & {'nms-direct', 'nms-refl'}:
            return 3
        return 2 if rs else 1
    out[r['jar']] = {'name': r['name'], 'source': r['source'], 'judge_level': r['summary']['level'],
                     'level_any': lvl(any_), 'level_on': lvl(on), 'level_on_with_templates': lvl(on | on_tmpl),
                     'reach_any': sorted(any_), 'reach_on': sorted(on), 'reach_on_tmpl': sorted(on | on_tmpl),
                     'apipriv_any': sorted(apipriv_any), 'apipriv_on': sorted(apipriv_on),
                     'judge_apipriv': r['summary']['api_private_reflection'], 'judge_reaches': r['summary']['reaches']}
P_.save('levels.json', out)
C = collections.Counter
print('judge level', sorted(C(v['judge_level'] for v in out.values()).items()))
print('mine any', sorted(C(v['level_any'] for v in out.values()).items()), 'mismatch judge', sum(1 for v in out.values() if v['level_any'] != v['judge_level']))
print('mine on 1.20.1', sorted(C(v['level_on'] for v in out.values()).items()))
print('mine on 1.20.1 incl templates', sorted(C(v['level_on_with_templates'] for v in out.values()).items()))
mm = [(v['name'], v['judge_level'], v['level_any'], v['reach_any'], v['judge_reaches']) for v in out.values() if v['level_any'] != v['judge_level']]
for x in mm[:20]: print('MISMATCH', x)
print('rises with templates', [(v['name'], v['level_on'], v['level_on_with_templates']) for v in out.values() if v['level_on_with_templates'] != v['level_on']])
