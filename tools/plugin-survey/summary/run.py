# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# Runs every summary step of the plugin survey in order and keeps what each prints as
# <out dir>/<step>.txt beside the JSON the steps hand on.
#
#     python3 -I run.py <survey dir> <out dir> --srg <bukkit_srg.srg> [--data FILE] [--api FILE]
import argparse
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
STEPS = ('corpus_counts', 'jar_facts', 'levels', 'members', 'groups', 'library_roots', 'libraries',
         'library_reach', 'minecraft_classes', 'minecraft_rank', 'public_api', 'order', 'classify', 'details')

ap = argparse.ArgumentParser()
ap.add_argument('survey')
ap.add_argument('out')
ap.add_argument('--srg', required=True)
ap.add_argument('--data')
ap.add_argument('--api')
args = ap.parse_args()
os.makedirs(args.out, exist_ok=True)
extra = ['--srg', args.srg]
for flag in ('data', 'api'):
    if getattr(args, flag):
        extra += ['--' + flag, getattr(args, flag)]
for step in STEPS:
    with open(os.path.join(args.out, step + '.txt'), 'w') as log:
        result = subprocess.run([sys.executable, '-I', os.path.join(HERE, step + '.py'), args.survey, args.out] + extra,
                                stdout=log, stderr=subprocess.STDOUT)
    print('%-18s %s' % (step, 'ok' if result.returncode == 0 else 'failed, see %s.txt' % step), flush=True)
    if result.returncode != 0:
        sys.exit(1)
