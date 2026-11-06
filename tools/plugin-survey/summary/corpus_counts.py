# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# The corpus by source: the jars kept, dropped as duplicates and skipped, their bytes, and the
# downloads counted two ways, once per site over every listing of a plugin and only for the
# listing the jar came from. Reads corpus.json alone.
#
#     python3 -I corpus_counts.py <survey dir> <out dir>
import collections
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common  # noqa: E402

P_ = common.paths(__doc__)
c = json.load(open(P_.corpus))
P = c['plugins']
print('kept', len(P), collections.Counter(p['source'] for p in P))
# downloads: field only
f_only = sum(p.get('downloads') or 0 for p in P)
print('sum downloads field of kept jars', f_only)
# the listing the jar came from, else the largest other listing
fab = 0
for p in P:
    d = p.get('downloads')
    if d is None:
        alts = [a.get('downloads') or 0 for a in p.get('also_in', [])]
        d = max(alts) if alts else 0
    fab += d
print('the listing of the jar', fab)
# all listings, one per site
allw = 0
persite = collections.Counter()
for p in P:
    per = collections.defaultdict(int)
    src = p['source']
    if src == 'live':
        src = 'modrinth' if 'modrinth.com' in (p.get('url') or '') else 'github'
    if p.get('downloads'):
        per[src] = max(per[src], p['downloads'])
    for a in p.get('also_in', []):
        if a.get('downloads'):
            per[a['source']] = max(per[a['source']], a['downloads'])
    allw += sum(per.values())
    for k, v in per.items():
        persite[k] += v
print('all listings one per site', allw, dict(persite))
print('also_in total entries', sum(len(p.get('also_in', [])) for p in P),
      'plugins with also_in', sum(1 for p in P if p.get('also_in')))
for p in P:
    if p['source'] == 'live':
        print('live', p['name'], p['version'], p.get('downloads'), (p.get('url') or '')[:80],
              [(a['source'], a.get('downloads')) for a in p.get('also_in', [])])
print('generated', c['generated'])
for k, v in c['counts'].items():
    print(k, {kk: vv for kk, vv in v.items() if kk != 'skip_reasons'})
for k, v in c['counts'].items():
    if v.get('skip_reasons'):
        print(k, 'skips', v['skip_reasons'])
