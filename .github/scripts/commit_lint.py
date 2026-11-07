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
   kernel or of this repository, and no other line. No earlier paragraph is made of trailers,
   since git and the kernel read the last paragraph alone as the block.
6. The block holds Signed-off-by with a name and an address in angle brackets, and the last
   trailer is a Signed-off-by, as the kernel orders them.
7. A Fixes trailer names at least 12 characters of a hash and the subject in parentheses and
   quotes, and Link and Closes name a URL. In a git repository the hash names one of its commits
   and the quoted subject is the subject of that commit. A shallow clone may lack the commit, so
   there a hash it does not hold passes.
8. No trailer appears twice with the same value.

A commit of a range is judged by the message that git stored, every line of it. A message file is
judged as git will store it, as git commit and git merge hand it to the commit-msg hook: cut at the
cut line of git commit --verbose, which the diff follows, and without the lines that open with the
comment character of core.commentChar, # unless it is set, which git drops from a message that was
edited unless commit.cleanup keeps them. Git tells the hook that no editor ran with GIT_EDITOR set
to :, and then keeps those lines.

Merge commits are left out, since git writes their message: a range holds no merge, and the hook
judges no message while git makes a merge, in git merge or in a git commit that concludes one. A
shallow clone cuts the parents of the commits at its edge, so there a commit whose message git
writes for a merge, such as the merge of a pull request that CI checks out, is left out as well.

With no range, the commits are those of HEAD that origin/main does not hold, or every commit of
HEAD while origin/main does not exist. Every failure is printed with the short hash and the
subject, and the exit status is 1 when any rule fails.
"""

from __future__ import annotations

import argparse
import os
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


#: The subject git gives a merge commit, and GitHub the merge of a pull request.
MERGE_SUBJECT = re.compile(r"^Merge (branch |remote-tracking branch |tag |pull request |commit |[0-9a-f]{7,40} into )")


def merging(root: Path, message_file: Path) -> bool:
    """Whether git makes a merge, so that the hook judges no message: git merge hands the hook its
    MERGE_MSG, and a git commit that concludes a merge runs while MERGE_HEAD exists."""
    if message_file.name == "MERGE_MSG":
        return True
    probe = subprocess.run(["git", "-C", str(root), "rev-parse", "--quiet", "--verify", "MERGE_HEAD"],
                           capture_output=True, text=True, check=False)
    return probe.returncode == 0


def shallow_edge(root: Path) -> set[str]:
    """The commits at the edge of a shallow clone, whose parents git does not hold."""
    path = Path(git(root, "rev-parse", "--git-path", "shallow").strip())
    path = path if path.is_absolute() else root / path
    return set(path.read_text().split()) if path.is_file() else set()


def commits(root: Path, revisions: str) -> list[tuple[str, str]]:
    """The hashes and messages of the commits of `revisions`, oldest first, merges left out."""
    output = git(root, "log", "--no-merges", "--reverse", "--format=%H%x00%B%x1e", revisions)
    edge = shallow_edge(root)
    found = []
    for record in output.split("\x1e"):
        record = record.lstrip("\n")
        if record:
            commit, _, message = record.partition("\x00")
            # A merge at the edge of a shallow clone has lost its parents, so --no-merges keeps it.
            if commit in edge and MERGE_SUBJECT.match(message):
                continue
            found.append((commit, message))
    return found


#: What the resolver of a shallow clone answers for a hash that the clone does not hold.
UNKNOWN = object()


def resolver(root: Path):
    """A function that answers the subject of the commit a hash names in `root`, None for no
    commit, or UNKNOWN in a shallow clone that lacks it, or no function at all when `root` is no
    git repository."""
    probe = subprocess.run(["git", "-C", str(root), "rev-parse", "--is-shallow-repository"], capture_output=True,
                           text=True, check=False)
    if probe.returncode != 0:
        return None
    shallow = probe.stdout.strip() == "true"

    def subject_of(abbreviated: str):
        found = subprocess.run(["git", "-C", str(root), "log", "-1", "--format=%s", abbreviated + "^{commit}", "--"],
                               capture_output=True, text=True, check=False)
        verified = subprocess.run(["git", "-C", str(root), "rev-parse", "--verify", "--quiet",
                                   abbreviated + "^{commit}"], capture_output=True, text=True, check=False)
        if found.returncode != 0 or verified.returncode != 0:
            return UNKNOWN if shallow else None
        return found.stdout.rstrip("\n")

    return subject_of


def known_trailer(line: str) -> bool:
    """True for a line Token: value whose token is a trailer of the kernel or of this repository."""
    match = TRAILER.match(line)
    return bool(match) and match.group("token").lower() in (token.lower() for token in TRAILERS)


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


#: The cut line that git commit --verbose writes after the comment character, above the diff. Git
#: drops it and everything below it, and so does the cleanup mode scissors.
CUT_LINE = "------------------------ >8 ------------------------"
#: The comment characters that git tries in this order for core.commentChar auto.
AUTO_COMMENT = "#;@!$%^&|:"


def git_config(root: Path, pattern: str):
    """The last value of the git configuration of `root` whose key matches `pattern`, or None."""
    found = subprocess.run(["git", "-C", str(root), "config", "--get-regexp", pattern], capture_output=True,
                           text=True, check=False)
    values = found.stdout.splitlines() if found.returncode == 0 else []
    return values[-1].partition(" ")[2] if values else None


def comment_string(root: Path, text: str) -> str:
    """The string that opens a comment line of `text`, as core.commentChar or core.commentString of
    `root` sets it, or # when neither does."""
    value = git_config(root, r"^core\.comment(char|string)$")
    if not value:
        return "#"
    if value.lower() != "auto":
        return value
    # With auto git takes the first candidate that no line of the message opens with, and writes its
    # own lines with it: the cut line, and its hints and the status at the end of the file.
    cut = re.search(r"^(.) %s$" % re.escape(CUT_LINE), text, re.MULTILINE)
    if cut and cut.group(1) in AUTO_COMMENT:
        return cut.group(1)
    lines = [line for line in text.splitlines() if line.strip()]
    if lines and lines[-1][0] in AUTO_COMMENT and lines[-1][1:2] in ("", " ", "\t"):
        return lines[-1][0]
    if "#" not in text:
        return "#"
    taken = {line[:1] for line in text.splitlines()}
    return next((candidate for candidate in AUTO_COMMENT if candidate not in taken), "#")


def stripspace(text: str, comment=None) -> str:
    """`text` as git stripspace leaves it: without white space at the end of a line, without empty
    lines at the start, at the end or after an empty line, and without the lines that open with
    `comment` when it is given."""
    kept, empty = [], False
    for line in text.split("\n"):
        if comment is not None and line.startswith(comment):
            continue
        line = line.rstrip(" \t\n\v\f\r")
        if not line:
            empty = bool(kept)
            continue
        if empty:
            kept.append("")
            empty = False
        kept.append(line)
    return "".join(line + "\n" for line in kept)


def stored(text: str, root: Path, environment) -> str:
    """The message that git stores from `text`, the file that it hands to the commit-msg hook in
    `root`, with `environment` the environment of the hook."""
    comment = comment_string(root, text)
    cut = "%s %s\n" % (comment, CUT_LINE)
    if text.startswith(cut):
        text = ""
    elif "\n" + cut in text:
        text = text[:text.index("\n" + cut) + 1]
    cleanup = (git_config(root, r"^commit\.cleanup$") or "default").lower()
    if cleanup == "verbatim":
        return text
    if cleanup not in ("strip", "whitespace", "scissors"):
        cleanup = "whitespace" if environment.get("GIT_EDITOR") == ":" else "strip"
    return stripspace(text, comment if cleanup == "strip" else None)


def check_message(message: str, subject_of=None) -> list[str]:
    """The failures of one message as git stores it, each naming its rule. `subject_of` answers the
    subject of the commit that a hash of a Fixes trailer names, when the lint runs in a repository."""
    failures = []
    lines = [line.rstrip() for line in message.splitlines()]
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

    for group in body:
        if all(known_trailer(line) for line in group):
            failures.append("rule 5: the paragraph %r holds trailers apart from the last block" % group[0])

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
        elif canonical == "Fixes" and subject_of is not None:
            abbreviated, quoted = value.split(" ", 1)
            subject = subject_of(abbreviated)
            if subject is UNKNOWN:
                pass
            elif subject is None:
                failures.append("rule 7: Fixes names %s, which is no commit of the repository" % abbreviated)
            elif quoted != '("%s")' % subject:
                failures.append('rule 7: Fixes names %s, whose subject is "%s"' % (abbreviated, subject))
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

    subject_of = resolver(args.root)
    if args.message_file:
        if merging(args.root, args.message_file):
            return 0
        text = args.message_file.read_text(encoding="utf-8")
        failures = check_message(stored(text, args.root, os.environ), subject_of)
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
        failures = check_message(message, subject_of)
        subject = message.splitlines()[0] if message.strip() else ""
        for failure in failures:
            print("commit_lint: %s %s: %s" % (commit[:12], subject, failure))
        failed += bool(failures)
    print("commit_lint: range %s, commits %d, failed %d" % (revisions, len(found), failed))
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
