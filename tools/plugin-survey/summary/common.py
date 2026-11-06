# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# What the summary steps of the plugin survey share: their arguments, the rule that decides
# whether code under a version tag runs on Minecraft 1.20.1, and the readers of their inputs.
#
# Every step takes the survey dir and an out dir. The survey dir holds corpus.json, jars/ and
# the scan records. The out dir takes the JSON files that one step hands to the next.
import argparse
import json
import os
import re

EXACT = re.compile(r'^v?(\d+)_(\d+)_R(\d+)$')
BARE = re.compile(r'^v?(\d+)_(\d+)$')
YEAR = re.compile(r'^v?(2\d|3\d)_(\d+)(_\d+)?$')


def vt(s):
    p = [int(x) for x in re.findall(r'\d+', s)][:3]
    return tuple(p + [0] * (3 - len(p)))


def tag_ok(t):
    """Whether code under this version tag can run on 1.20.1 (v1_20_R1)."""
    if not t or t == '-':
        return True
    if YEAR.match(t):
        return False
    m = EXACT.match(t)
    if m:
        return (int(m.group(1)), int(m.group(2)), int(m.group(3))) == (1, 20, 1)
    m = BARE.match(t)
    if m:
        return (int(m.group(1)), int(m.group(2))) == (1, 20)
    for op, f in (('>=', lambda a: a <= (1, 20, 1)), ('<=', lambda a: a >= (1, 20, 1)),
                  ('>', lambda a: a < (1, 20, 1)), ('<', lambda a: a > (1, 20, 1))):
        if t.startswith(op):
            return f(vt(t[len(op):]))
    return True


class Paths:
    """The inputs and outputs of a step."""

    def __init__(self, args):
        self.survey = os.path.abspath(args.survey)
        self.out = os.path.abspath(args.out)
        os.makedirs(self.out, exist_ok=True)
        self.corpus = os.path.join(self.survey, 'corpus.json')
        self.jars = os.path.join(self.survey, 'jars')
        self.data = os.path.abspath(args.data) if args.data else os.path.join(self.survey, 'reconciled.jsonl')
        self.api = os.path.abspath(args.api) if args.api else os.path.join(self.survey, 'api_model.json')
        self.srg = os.path.abspath(args.srg) if args.srg else None

    def output(self, name):
        return os.path.join(self.out, name)

    def jar(self, plugin):
        """The jar of a corpus entry, inside the jars dir of the survey."""
        return os.path.join(self.jars, os.path.basename(plugin['path']))

    def records(self):
        """The scan records, one jar at a time."""
        with open(self.data) as f:
            for line in f:
                yield json.loads(line)

    def load(self, name):
        with open(self.output(name)) as f:
            return json.load(f)

    def save(self, name, value):
        with open(self.output(name), 'w') as f:
            json.dump(value, f)


def paths(description, srg=False):
    """Parse the arguments of a step."""
    ap = argparse.ArgumentParser(description=description)
    ap.add_argument('survey', help='the survey dir with corpus.json, jars/ and the scan records')
    ap.add_argument('out', help='the dir for the JSON that the steps hand on')
    ap.add_argument('--data', help='the scan records, by default <survey>/reconciled.jsonl')
    ap.add_argument('--api', help='the model of spigot-api 1.20.1, by default <survey>/api_model.json')
    ap.add_argument('--srg', required=srg, help="bukkit_srg.srg of Arclight 1.20.1, Spigot names to Mojang names")
    return Paths(ap.parse_args())
