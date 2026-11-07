#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
"""Check the prose of the documentation against the rules of CONTRIBUTING.md section 4, and that
the index of the wiki and the page on testing know every page and every notebook.

    python3 -I .github/scripts/docs_lint.py
    python3 -I .github/scripts/docs_lint.py --root path/to/tree

The lint reads the Markdown files of the wiki, okf/**/*.md, of Documentation/**/*.md, and
CONTRIBUTING.md and README.md at the root, and no other file. In a file it leaves out the front
matter that a page of the wiki opens with, fenced code blocks, code spans, HTML comments, URLs and
the targets of links, and a list item is read without its marker. The rules are these.

1. No semicolon.
2. No dash as punctuation: no en dash or em dash, no hyphen with a space on either side, and no
   double hyphen between two words.
3. No exclamation mark at the end of a word.
4. People are they and them: no he, him, his, himself, she, her, hers or herself.
5. A line is at most 100 columns long. A row of a table, a line that is only code and a line that
   holds a token longer than 60 columns, such as a long link, may be longer.
6. okf/index.md links every Markdown page under okf/ but itself, so its page map names every page.
7. Every entry of tests/matrix.yaml has a row in the table of section 4 of Documentation/testing.md,
   named by the id of the entry, which is its notebook without the suffix when it names no id.

Every failure is printed as the path, the line and the rule, and the exit status is 1 when any rule
fails. The lint uses the standard library alone, so CI runs it before it installs anything.
"""

from __future__ import annotations

import argparse
import posixpath
import re
import sys
from pathlib import Path, PurePosixPath

#: The root of the tree, two levels above this file.
ROOT = Path(__file__).resolve().parents[2]

PRONOUN = re.compile(r"\b(he|him|his|himself|she|her|hers|herself)\b", re.I)
DASH = re.compile(r"(\s[-–—]\s)|—|–|(?<=\w)--(?=\w)")
LIST_MARKER = re.compile(r"^\s*([-*+]|\d+\.)\s")
LINK = re.compile(r"\]\(([^)\s]+)\)")
WIDTH = 100
LONG_TOKEN = 60


def scrub(line: str) -> str:
    """The prose of a line, without code spans, link targets, URLs and HTML comments."""
    line = re.sub(r"`[^`]*`", "", line)
    line = re.sub(r"\]\([^)]*\)", "]", line)
    line = re.sub(r"https?://\S+", "", line)
    line = re.sub(r"<!--.*?-->", "", line)
    return line


def documents(root: Path) -> list[PurePosixPath]:
    """The files the lint reads, as paths relative to the root, sorted."""
    found = []
    for folder in ("okf", "Documentation"):
        base = root / folder
        if base.is_dir():
            found += [path for path in base.rglob("*.md") if path.is_file()]
    found += [root / name for name in ("CONTRIBUTING.md", "README.md") if (root / name).is_file()]
    return sorted(PurePosixPath(path.relative_to(root).as_posix()) for path in found)


def prose(root: Path, name: PurePosixPath, fail) -> None:
    """Rules 1 to 5 for one file."""
    lines = (root / name).read_text(encoding="utf-8").split("\n")
    fence = False
    front = 0
    for number, raw in enumerate(lines, 1):
        if number == 1 and raw == "---":
            front = 1
            continue
        if front == 1:
            if raw == "---":
                front = 2
            continue
        if raw.lstrip().startswith("```"):
            fence = not fence
            continue
        if fence:
            continue
        text = scrub(LIST_MARKER.sub(" ", raw))
        if ";" in text:
            fail(name, number, 1, "a semicolon")
        if DASH.search(text):
            fail(name, number, 2, "a dash as punctuation")
        if re.search(r"!(\s|$)", text):
            fail(name, number, 3, "an exclamation mark")
        pronoun = PRONOUN.search(text)
        if pronoun:
            fail(name, number, 4, "the pronoun %r, where people are they and them" % pronoun.group(0))
        unbreakable = not text.strip() or max(len(token) for token in raw.split()) > LONG_TOKEN
        if len(raw) > WIDTH and not raw.lstrip().startswith("|") and not unbreakable:
            fail(name, number, 5, "%d columns, more than %d" % (len(raw), WIDTH))


def page_map(root: Path, names: list[PurePosixPath], fail) -> None:
    """Rule 6: the index of the wiki links every page of the wiki."""
    index = PurePosixPath("okf/index.md")
    pages = [name for name in names if name.parts[0] == "okf" and name != index]
    if not pages:
        return
    if index not in names:
        fail(index, 1, 6, "the wiki has pages but no index")
        return
    lines = (root / index).read_text(encoding="utf-8").split("\n")
    linked = set()
    for line in lines:
        for target in LINK.findall(line):
            path = target.split("#", 1)[0]
            if path and not re.match(r"[a-z][a-z0-9+.-]*:", path):
                linked.add(PurePosixPath(posixpath.normpath(posixpath.join("okf", path))))
    where = next((number for number, line in enumerate(lines, 1) if line.strip() == "## Page map"), 1)
    for page in pages:
        if page not in linked:
            fail(index, where, 6, "links no %s, so the page map misses a page" % page)


def matrix_rows(root: Path, fail) -> None:
    """Rule 7: the page on testing names every entry of the matrix in its section 4."""
    matrix = root / "tests" / "matrix.yaml"
    if not matrix.is_file():
        return
    entries = []
    for number, line in enumerate(matrix.read_text(encoding="utf-8").split("\n"), 1):
        start = re.match(r"\s*-\s+notebook:\s*['\"]?([^'\"\s#]+)", line)
        named = re.match(r"\s+id:\s*['\"]?([^'\"\s#]+)", line)
        if start:
            entries.append([number, PurePosixPath(start.group(1)).name.removesuffix(".ipynb")])
        elif named and entries:
            entries[-1][1] = named.group(1)
    rows = set()
    testing = root / "Documentation" / "testing.md"
    if testing.is_file():
        section = False
        for line in testing.read_text(encoding="utf-8").split("\n"):
            if line.startswith("## "):
                section = bool(re.match(r"## 4\.\s", line))
            elif section:
                row = re.match(r"\|\s*`([^`]+)`\s*\|", line)
                if row:
                    rows.add(row.group(1))
    for number, name in entries:
        if name not in rows:
            fail(PurePosixPath("tests/matrix.yaml"), number, 7,
                 "the entry %s has no row in section 4 of Documentation/testing.md" % name)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Check the prose and the maps of the documentation.")
    parser.add_argument("--root", type=Path, default=ROOT, help="The root of the tree, by default the repository.")
    args = parser.parse_args(argv)
    root = args.root.resolve()
    failures = []

    def fail(path, line, rule, message):
        failures.append("%s:%d: rule %d: %s" % (path, line, rule, message))

    names = documents(root)
    for name in names:
        prose(root, name, fail)
    page_map(root, names, fail)
    matrix_rows(root, fail)
    for failure in failures:
        print("docs_lint: %s" % failure)
    print("docs_lint: files %d, failures %d" % (len(names), len(failures)))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
