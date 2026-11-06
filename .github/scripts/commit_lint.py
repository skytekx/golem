#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
"""Check that every commit message is a Conventional Commit in the form of the Linux kernel.

    python3 -I .github/scripts/commit_lint.py
    python3 -I .github/scripts/commit_lint.py --range origin/main..HEAD
    python3 -I .github/scripts/commit_lint.py --message-file .git/COMMIT_EDITMSG
    python3 -I .github/scripts/commit_lint.py --root path/to/repository

CONTRIBUTING.md states the form, and .cz.toml gives Commitizen the same subject. The rules are
these.

1. The subject is type(scope): summary, or type(scope)!: summary for a breaking change. The type
   is one of feat, fix, docs, refactor, perf, test, build, ci, chore and revert. The summary
   starts with a lower case letter or a digit and ends without a full stop.
2. The subject is at most 72 characters long.
3. An empty line follows the subject, then a body of prose, so a commit says why it exists.
4. Every line of the body is at most 72 columns wide, but a line that holds a URL alone, a
   trailer, or an indented quote such as a log line.
5. The last paragraph is the block of trailers, each a line Token: value with a token of the
   kernel or of this repository, and no other line.
6. The block holds Signed-off-by with a name and an address in angle brackets, and the last
   trailer is a Signed-off-by, as the kernel orders them.
7. A Fixes trailer names at least 12 characters of a hash and the subject in parentheses and
   quotes, and Link and Closes name a URL.
8. No trailer appears twice with the same value.

Merge commits are left out, since git writes their message. With no range, the commits are those
of HEAD that origin/main does not hold, or every commit of HEAD while origin/main does not exist.
Every failure is printed with the short hash and the subject, and the exit status is 1 when any
rule fails.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

#: The root of the tree, two levels above this file.
ROOT = Path(__file__).resolve().parents[2]

TYPES = ("feat", "fix", "docs", "refactor", "perf", "test", "build", "ci", "chore", "revert")
SUBJECT = re.compile(
    r"^(?P<type>[a-z]+)\((?P<scope>[A-Za-z0-9_./-]+)\)(?P<breaking>!)?: (?P<summary>.+)$"
)
SUBJECT_LIMIT = 72
BODY_LIMIT = 72

#: The trailers of the kernel's submitting-patches.rst, and those this repository adds: the
#: Closes of a GitHub issue, the Co-Authored-By of GitHub and the session link of an agent.
TRAILERS = (
    "Signed-off-by", "Co-developed-by", "Acked-by", "Reviewed-by", "Tested-by", "Reported-by",
    "Suggested-by", "Cc", "Fixes", "Link", "Closes", "Co-Authored-By", "Claude-Session",
)
TRAILER = re.compile(r"^(?P<token>[A-Za-z][A-Za-z-]*): (?P<value>\S.*)$")
PERSON = re.compile(r"^[^<>]+ <[^<>@\s]+@[^<>@\s]+>$")
FIXES = re.compile(r'^[0-9a-f]{12,40} \("[^\n]+"\)$')
URL = re.compile(r"^https?://\S+$")


class Failure(Exception):
    """A failure of the lint itself, printed without a traceback."""


def git(root: Path, *args: str) -> str:
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True, check=False)
    if result.returncode != 0:
        raise Failure("git %s failed: %s" % (" ".join(args), result.stderr.strip()))
    return result.stdout


def default_range(root: Path) -> str:
    """origin/main..HEAD, or HEAD alone while origin/main does not exist."""
    probe = subprocess.run(["git", "-C", str(root), "rev-parse", "--verify", "--quiet", "origin/main"],
                           capture_output=True, text=True, check=False)
    return "origin/main..HEAD" if probe.returncode == 0 else "HEAD"


def commits(root: Path, revisions: str) -> list[tuple[str, str]]:
    """The hashes and messages of the commits of `revisions`, oldest first, merges left out."""
    output = git(root, "log", "--no-merges", "--reverse", "--format=%H%x00%B%x1e", revisions)
    found = []
    for record in output.split("\x1e"):
        record = record.lstrip("\n")
        if record:
            commit, _, message = record.partition("\x00")
            found.append((commit, message))
    return found


def paragraphs(lines: list[str]) -> list[list[str]]:
    """The lines grouped at empty lines."""
    groups, current = [], []
    for line in lines:
        if line.strip():
            current.append(line)
        elif current:
            groups.append(current)
            current = []
    if current:
        groups.append(current)
    return groups


def check_message(message: str) -> list[str]:
    """The failures of one message, each naming its rule."""
    failures = []
    # Git drops the lines of a commit message that open with #, so the lint reads it the same way.
    lines = [line.rstrip() for line in message.splitlines() if not line.startswith("#")]
    while lines and not lines[-1]:
        lines.pop()
    if not lines:
        return ["rule 3: the message is empty"]
    subject = lines[0]

    match = SUBJECT.match(subject)
    if not match:
        failures.append("rule 1: the subject is not type(scope): summary")
    else:
        if match.group("type") not in TYPES:
            failures.append("rule 1: the type %s is none of %s" % (match.group("type"), ", ".join(TYPES)))
        summary = match.group("summary")
        if not re.match(r"[a-z0-9]", summary):
            failures.append("rule 1: the summary starts with neither a lower case letter nor a digit")
        if summary.endswith("."):
            failures.append("rule 1: the summary ends with a full stop")
        if summary != summary.strip():
            failures.append("rule 1: the summary starts or ends with white space")
    if len(subject) > SUBJECT_LIMIT:
        failures.append("rule 2: the subject is %d characters long, more than %d" % (len(subject), SUBJECT_LIMIT))

    if len(lines) < 2 or lines[1]:
        failures.append("rule 3: no empty line follows the subject")
    groups = paragraphs(lines[1:])
    trailer_block = groups[-1] if groups and all(TRAILER.match(line) for line in groups[-1]) else []
    body = groups[:-1] if trailer_block else groups
    if not body:
        failures.append("rule 3: the message holds no body of prose")

    for number, line in enumerate(lines[1:], start=2):
        if len(line) <= BODY_LIMIT or URL.match(line.strip()) or TRAILER.match(line) or line.startswith((" ", "\t")):
            continue
        failures.append("rule 4: line %d is %d columns wide, more than %d" % (number, len(line), BODY_LIMIT))

    if not trailer_block:
        failures.append("rule 5: the last paragraph is no block of trailers")
        last = groups[-1] if groups else []
        stray = [line for line in last if TRAILER.match(line)]
        if stray:
            failures.append("rule 5: the last paragraph mixes trailers with other lines")
        failures.append("rule 6: the message carries no Signed-off-by")
        return failures

    seen = set()
    for line in trailer_block:
        trailer = TRAILER.match(line)
        token, value = trailer.group("token"), trailer.group("value")
        canonical = next((known for known in TRAILERS if known.lower() == token.lower()), None)
        if canonical is None:
            failures.append("rule 5: the trailer %s is none of the kernel or of this repository" % token)
        elif canonical != token:
            failures.append("rule 5: the trailer %s is spelled %s" % (token, canonical))
        if canonical in ("Signed-off-by", "Co-developed-by", "Co-Authored-By", "Reviewed-by", "Tested-by",
                         "Acked-by", "Reported-by", "Suggested-by") and not PERSON.match(value):
            failures.append("rule 6: the trailer %s names no Name <address>" % token)
        if canonical == "Fixes" and not FIXES.match(value):
            failures.append('rule 7: Fixes names no hash of 12 characters and ("subject")')
        if canonical in ("Link", "Closes") and not URL.match(value):
            failures.append("rule 7: the trailer %s names no URL" % token)
        if (token.lower(), value) in seen:
            failures.append("rule 8: the trailer %s appears twice with the same value" % token)
        seen.add((token.lower(), value))

    tokens = [TRAILER.match(line).group("token").lower() for line in trailer_block]
    if "signed-off-by" not in tokens:
        failures.append("rule 6: the message carries no Signed-off-by")
    elif tokens[-1] != "signed-off-by":
        failures.append("rule 6: the last trailer is no Signed-off-by")
    return failures


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Check the commit messages of a range.")
    source = parser.add_mutually_exclusive_group()
    source.add_argument("--range", dest="revisions", help="The commits to check, by default origin/main..HEAD.")
    source.add_argument("--message-file", type=Path, help="Check one message from a file, as a commit-msg hook.")
    parser.add_argument("--root", type=Path, default=ROOT, help="The repository, by default this one.")
    args = parser.parse_args(argv)

    if args.message_file:
        failures = check_message(args.message_file.read_text(encoding="utf-8"))
        for failure in failures:
            print("commit_lint: %s: %s" % (args.message_file, failure))
        return 1 if failures else 0

    try:
        revisions = args.revisions or default_range(args.root)
        found = commits(args.root, revisions)
    except Failure as broken:
        print("commit_lint: %s" % broken, file=sys.stderr)
        return 1
    failed = 0
    for commit, message in found:
        failures = check_message(message)
        subject = message.splitlines()[0] if message.strip() else ""
        for failure in failures:
            print("commit_lint: %s %s: %s" % (commit[:12], subject, failure))
        failed += bool(failures)
    print("commit_lint: range %s, commits %d, failed %d" % (revisions, len(found), failed))
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
