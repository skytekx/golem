# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# Facts that only the jar itself gives: the class file version of the main class and the highest
# of any class file, nested jars included, the newest entry date, and the descriptor as Spigot
# reads it, with its api-version, whether YAML refuses it and whether the main class it names
# exists. Writes jars_facts.json.
#
#     python3 -I jar_facts.py <survey dir> <out dir>
#
# The jars are untrusted data. This step reads them in memory with zipfile, extracts nothing
# and runs nothing.
import collections
import io
import json
import os
import struct
import sys
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(1, os.path.dirname(HERE))
import common  # noqa: E402
import plugin_yaml  # noqa: E402

P_ = common.paths(None)
c = json.load(open(P_.corpus))
res = {}


def walk(zf, depth, acc, main_entry):
    for info in zf.infolist():
        n = info.filename
        if info.is_dir():
            continue
        dt = info.date_time
        if dt[0] >= 2000:
            acc['dates'].append('%04d-%02d-%02d' % dt[:3])
        if n.endswith('.class'):
            with zf.open(info) as fh:
                head = fh.read(8)
            if len(head) == 8 and head[:4] == b'\xca\xfe\xba\xbe':
                major = struct.unpack('>H', head[6:8])[0]
                acc['majors'][major] += 1
                if n == main_entry:
                    acc['main'].append(major)
        elif n.endswith('.jar') and depth < 2 and info.file_size < 60 * 1024 * 1024:
            try:
                with zf.open(info) as fh:
                    data = fh.read()
                walk(zipfile.ZipFile(io.BytesIO(data)), depth + 1, acc, main_entry)
            except Exception:
                acc['nested_errors'] += 1


def read_descriptor(zf):
    """The api-version, the main class and the error of the descriptor, as Spigot reads it."""
    names = set(zf.namelist())
    name = 'plugin.yml' if 'plugin.yml' in names else 'paper-plugin.yml' if 'paper-plugin.yml' in names else None
    if name is None:
        return None, None, None, 'no descriptor'
    try:
        doc = plugin_yaml.load(zf.read(name))
    except plugin_yaml.DescriptorError as e:
        return name, None, None, str(e)
    if not isinstance(doc, dict):
        return name, None, None, 'the descriptor is not a mapping'
    api = doc.get('api-version')
    main = doc.get('main')
    return name, None if api is None else str(api), None if main is None else str(main), None


import warnings  # noqa: E402

warnings.simplefilter('ignore')
for p in c['plugins']:
    jar = os.path.basename(p['path'])
    with zipfile.ZipFile(P_.jar(p)) as zf:
        name, api, main, error = read_descriptor(zf)
        main_class = p.get('main') or main
        main_entry = (main_class or '').replace('.', '/') + '.class'
        acc = {'dates': [], 'majors': collections.Counter(), 'main': [], 'nested_errors': 0}
        walk(zf, 0, acc, main_entry)
    d = sorted(acc['dates'])
    res[jar] = {'name': p['name'], 'source': p['source'], 'main_major': min(acc['main']) if acc['main'] else None,
                'max_major': max(acc['majors']) if acc['majors'] else None, 'majors': dict(acc['majors']),
                'newest_entry': d[-1] if d else None, 'nested_errors': acc['nested_errors'],
                'descriptor': name, 'descriptor_api_version': api, 'descriptor_error': error,
                'main': main_class, 'main_found': bool(acc['main'])}
P_.save('jars_facts.json', res)
majors = collections.Counter(v['main_major'] for v in res.values())
print('jars', len(res))
print('class file major of the main class', sorted(majors.items(), key=lambda x: (x[0] is None, x[0] or 0)))
print('main class above Java 17', sum(1 for v in res.values() if (v['main_major'] or 0) > 61),
      ' any class file above Java 17', sum(1 for v in res.values() if (v['max_major'] or 0) > 61))
print('main class missing from the jar', [v['name'] for v in res.values() if v['main'] and not v['main_found']])
print('descriptor that YAML refuses', [(v['name'], v['descriptor_error']) for v in res.values() if v['descriptor_error']])
dates = sorted(v['newest_entry'] for v in res.values() if v['newest_entry'])
print('jars with entry dates', len(dates), 'from', dates[0], 'to', dates[-1], 'median', dates[len(dates) // 2],
      'by year', sorted(collections.Counter(x[:4] for x in dates).items()))
