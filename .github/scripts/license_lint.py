#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only AND MIT
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
"""Check that every file of the tree states its license and its holder, and that every license
has its text.

    python3 -I .github/scripts/license_lint.py
    python3 -I .github/scripts/license_lint.py --root path/to/tree

Derived from Aosabi 0.1.0, .github/scripts/license_lint.py, MIT. Changed by the Golem authors.

Golem states licenses in SPDX tags and keeps its tree compliant with REUSE, whose tool reuse
lint runs beside this script in CI. This script checks what REUSE leaves open. A file names its
license in its tag, an SPDX-License-Identifier line, which stands on the first line of the file
as the Linux kernel places it, or on the second under a #! line or under the --- that opens the
front matter of a page. LICENSES/ holds the text of every license a file names, as
LICENSES/<identifier>.txt. A tag anywhere else, such as an example in a document, is no tag of
the file. The rules are these.

1. LICENSE at the root holds the GNU AGPL 3, byte for byte the same as
   LICENSES/AGPL-3.0-only.txt, so GitHub reads it as the license of the repository.
2. Every tag holds a valid SPDX expression, license identifiers joined by AND, OR and WITH, with
   the operators in capitals.
3. Outside third_party/, every license an expression names has its text under LICENSES/, and
   every file under LICENSES/ is a text named <identifier>.txt, as REUSE asks.
4. Every Cargo.toml outside third_party/ states a license, its own or the workspace's, every
   license field of a Cargo.toml or a package.json follows rules 2 and 3, and every license-file
   field names a file that exists.
5. Every package under third_party/ keeps a license file of its own.
6. Every file outside LICENSE, LICENSES/, NOTICE and third_party/ carries a tag in its place.
   A file that cannot carry the tag, such as JSON or an image, takes it from a file beside it
   whose name adds .license, as REUSE does. NOTICE takes its license and its holders from the
   annotation of REUSE.toml that names its path, with the precedence override, since the
   copyright lines that NOTICE quotes would otherwise count as its own.
7. NOTICE exists at the root when a file outside third_party/ names a license other than the
   AGPL, since such a file carries the work of another project that NOTICE must name.
8. Every file that rule 6 covers names its copyright holders in SPDX-FileCopyrightText lines
   among its first lines, in the .license file beside it, or for NOTICE in its annotation, and
   one of them names Florian Kleber, the holder of Golem.
9. A port keeps the license of the plugin it ports. ports.toml at the root names every port
   with its name, its path, its upstream, which is the name of the plugin and its version, its
   source, its license and the holders of the upstream. The path holds files of the tree and the
   Cargo.toml of a package of that name, which states the license itself, and the license has
   its text under LICENSES/. Every file below the path carries exactly that license and a
   copyright line for each of those holders beside the one of Florian Kleber. NOTICE has a
   section headed by the name of the plugin, underlined with =, whose Version line starts with
   the version, whose License line is the license and whose Files line names the path.

Unlike Aosabi, Golem has no default license for a file without a tag: rule 6 covers every file,
not only the shipped sources.

The files are the ones git tracks when the root is a git work tree. Otherwise, as in an exported
source archive, they are every file below the root outside the directories that the .gitignore of
Golem names: the build output, the environment, the downloads and the executed notebooks of the
harness and the checkpoints of an editor. Every failure is printed with its path, and the exit
status is 1 when any rule fails.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path, PurePosixPath

#: The root of the tree, two levels above this file.
ROOT = Path(__file__).resolve().parents[2]

#: The lines read of a file, enough for its tags and for the tags of a REUSE file beside it.
HEAD_LINES = 40
#: The bytes read to find the tag and to tell a binary file, which holds a NUL among them.
HEAD_BYTES = 8192

# REUSE-IgnoreStart
#: The comment markers a tag may follow, and those that may close its line.
COMMENT_OPEN = r"(?://[/!]?|/\*+|\*|#+|--|<!--|;+|%+|\{#|\(\*|\.\.)?"
COMMENT_CLOSE = r"(?:\*/|-->|#\}|\*\))?"
#: A tag counts when it opens a line, after the comment marker of the language at most, so a
#: sentence that mentions the tag in prose is no tag.
TAG_LINE = re.compile(
    r"^\s*" + COMMENT_OPEN + r"\s*SPDX-License-Identifier:(?P<expression>.*?)\s*" + COMMENT_CLOSE + r"\s*$"
)
#: A copyright line in the form of REUSE.
COPYRIGHT_LINE = re.compile(
    r"^\s*" + COMMENT_OPEN + r"\s*SPDX-FileCopyrightText:(?P<holder>.*?)\s*" + COMMENT_CLOSE + r"\s*$"
)
# REUSE-IgnoreEnd
#: The copyright holder of Golem, whom every file of Golem names.
GOLEM_HOLDER = "Florian Kleber"
#: The tokens of an expression.
TOKEN = re.compile(r"\(|\)|[^\s()]+")
#: A license identifier, an exception identifier, or a reference to a license of the project.
IDENTIFIER = re.compile(r"(?:DocumentRef-[A-Za-z0-9.-]+:)?(?:LicenseRef-)?[A-Za-z0-9.-]+\+?")
OPERATORS = ("AND", "OR", "WITH")

#: The license of Golem's own code, and the text under LICENSES/ that LICENSE must equal.
GOLEM_LICENSE = "AGPL-3.0-only"
GOLEM_TEXT = PurePosixPath("LICENSES/AGPL-3.0-only.txt")
#: The sentences that make LICENSE the GNU AGPL 3.
AGPL_SENTENCES = (
    "GNU AFFERO GENERAL PUBLIC LICENSE",
    "Version 3, 19 November 2007",
    "13. Remote Network Interaction; Use with the GNU General Public License.",
)

#: The table of the ports, which rule 9 reads.
PORTS = PurePosixPath("ports.toml")
#: The version that ends the upstream of a port, after the name of the plugin and a space.
VERSION = re.compile(r"[0-9][0-9A-Za-z.+-]*")
#: A line of a section of NOTICE that names one of its fields.
NOTICE_FIELD = re.compile(r"^(Project|URL|Version|Files|License): (.*)$")

#: The names of a license file of a package under third_party/.
LICENSE_FILE = re.compile(r"^(?:LICEN[CS]E|COPYING|UNLICENSE)(?:[-._].*)?$", re.I)

#: The notices of the work of others, and the file of REUSE that gives NOTICE its tags.
NOTICE = PurePosixPath("NOTICE")
REUSE_TOML = PurePosixPath("REUSE.toml")

#: The files rule 6 leaves out. LICENSE is the license itself, and REUSE.toml gives NOTICE its
#: license and holder, since its text quotes the notices of other projects.
UNTAGGED = {PurePosixPath("LICENSE"), NOTICE}

#: The directories skipped at any depth when the root is no git work tree: build output,
#: dependency trees, environments and the checkpoints of an editor, as .gitignore names them.
SKIP_DIRS = {".git", "target", "node_modules", ".venv", "__pycache__", "build", ".gradle", ".ipynb_checkpoints"}
#: The directories of the harness skipped by their path from the root when the root is no git
#: work tree: the downloads of tests/fetch.py and the executed notebooks of tests/run.py.
SKIP_PATHS = {PurePosixPath("tests/.cache"), PurePosixPath("tests/notebooks/out")}


class Failure(Exception):
    """A failure of the lint itself, printed without a traceback."""


def tracked_files(root: Path) -> list[PurePosixPath]:
    """The files of the tree as paths relative to the root."""
    if (root / ".git").exists():
        result = subprocess.run(["git", "-C", str(root), "ls-files", "-z"], capture_output=True, check=False)
        if result.returncode != 0:
            raise Failure("git ls-files failed in %s: %s" % (root, result.stderr.decode(errors="replace").strip()))
        names = [name for name in result.stdout.decode("utf-8", errors="surrogateescape").split("\0") if name]
        return sorted(PurePosixPath(name) for name in names if not os.path.islink(root / name))
    files = []
    for directory, subdirectories, names in os.walk(root):
        relative = PurePosixPath(Path(directory).relative_to(root).as_posix())
        subdirectories[:] = sorted(name for name in subdirectories
                                   if name not in SKIP_DIRS and relative / name not in SKIP_PATHS)
        for name in names:
            if not os.path.islink(os.path.join(directory, name)):
                files.append(relative / name if str(relative) != "." else PurePosixPath(name))
    return sorted(files)


def head(root: Path, path: PurePosixPath) -> list[str] | None:
    """The first lines of a text file, or None for a binary file."""
    try:
        with open(root / path, "rb") as stream:
            data = stream.read(HEAD_BYTES)
    except OSError:
        return None
    if b"\0" in data:
        return None
    return data.decode("utf-8", errors="replace").splitlines()[:HEAD_LINES]


def find_tag(lines: list[str], placed: bool = True) -> str | None:
    """The expression of the tag among the first `lines` of a file, or None.

    With `placed`, the tag counts on the first line alone, or on the second under a #! line or
    under the --- that opens a front matter. Without it, as for a REUSE file beside another
    file, any of the lines may hold it.
    """
    if placed:
        lines = lines[1:2] if lines and (lines[0].startswith("#!") or lines[0].rstrip() == "---") else lines[:1]
    for line in lines:
        match = TAG_LINE.match(line)
        if match:
            return match.group("expression").strip()
    return None


def find_copyrights(lines: list[str]) -> list[str]:
    """The copyright holders that the first `lines` of a file name."""
    return [match.group("holder").strip() for match in map(COPYRIGHT_LINE.match, lines) if match]


def parse_expression(text: str) -> tuple[list[str], list[str]]:
    """The licenses and exceptions of an SPDX expression, or ValueError when it is not one."""
    tokens = TOKEN.findall(text)
    if not tokens:
        raise ValueError("the expression is empty")
    for token in tokens:
        if token.lower() in ("and", "or", "with") and token not in OPERATORS:
            raise ValueError("the operator %r must be written in capitals" % token)
    licenses, exceptions = [], []
    position = 0

    def peek():
        return tokens[position] if position < len(tokens) else None

    def take():
        nonlocal position
        if position >= len(tokens):
            raise ValueError("the expression ends early")
        position += 1
        return tokens[position - 1]

    def atom():
        token = take()
        if token == "(":
            disjunction()
            if take() != ")":
                raise ValueError("a parenthesis is not closed")
            return
        if token in OPERATORS or token == ")" or not IDENTIFIER.fullmatch(token):
            raise ValueError("%r is no license identifier" % token)
        licenses.append(token[:-1] if token.endswith("+") else token)

    def exception():
        atom()
        if peek() == "WITH":
            take()
            token = take()
            if token in OPERATORS or token in ("(", ")") or not IDENTIFIER.fullmatch(token):
                raise ValueError("%r is no exception identifier" % token)
            exceptions.append(token)

    def conjunction():
        exception()
        while peek() == "AND":
            take()
            exception()

    def disjunction():
        conjunction()
        while peek() == "OR":
            take()
            conjunction()

    disjunction()
    if position != len(tokens):
        raise ValueError("%r follows a complete expression" % tokens[position])
    return licenses, exceptions


def license_texts(files: list[PurePosixPath], fail) -> dict:
    """The identifiers whose texts lie under LICENSES/, each with its file, as REUSE names them."""
    known = {}
    for path in files:
        if path.parts[0] != "LICENSES":
            continue
        if len(path.parts) != 2 or path.suffix != ".txt" or not IDENTIFIER.fullmatch(path.stem):
            fail(path, "rule 3: a file under LICENSES/ must be a text named <identifier>.txt, as REUSE asks")
            continue
        known.setdefault(path.stem, path)
    return known


def notice_annotation(root: Path, present: set, fail) -> dict | None:
    """The annotation of REUSE.toml that names the path NOTICE, or None after a failure of rule 6."""
    if REUSE_TOML not in present:
        fail(NOTICE, "rule 6: NOTICE carries no tag, and the root holds no REUSE.toml to give it one")
        return None
    try:
        table = tomllib.loads((root / REUSE_TOML).read_text(encoding="utf-8"))
    except (OSError, ValueError) as broken:
        fail(REUSE_TOML, "rule 6: REUSE.toml does not parse: %s" % broken)
        return None
    for annotation in table.get("annotations") or []:
        paths = annotation.get("path") if isinstance(annotation, dict) else None
        if paths == str(NOTICE) or (isinstance(paths, list) and str(NOTICE) in paths):
            return annotation
    fail(NOTICE, "rule 6: NOTICE carries no tag, and no annotation of REUSE.toml names its path")
    return None


def strings(value) -> list[str]:
    """A value of REUSE.toml that is a string or a list of strings, as a list."""
    if isinstance(value, str):
        return [value]
    return [item for item in value if isinstance(item, str)] if isinstance(value, list) else []


def load_ports(root: Path, fail) -> list[dict]:
    """The ports of ports.toml, each checked for its fields, or no port when the file is absent."""
    try:
        table = tomllib.loads((root / PORTS).read_text(encoding="utf-8"))
    except FileNotFoundError:
        return []
    except (OSError, ValueError) as broken:
        fail(PORTS, "rule 9: the table of the ports does not parse: %s" % broken)
        return []
    ports = []
    for index, port in enumerate(table.get("port") or []):
        where = "%s port %d" % (PORTS, index + 1)
        missing = [field for field in ("name", "path", "upstream", "source", "license", "holders") if not port.get(field)]
        if missing:
            fail(where, "rule 9: the port lacks %s" % ", ".join(missing))
            continue
        if not isinstance(port["holders"], list) or not all(isinstance(holder, str) for holder in port["holders"]):
            fail(where, "rule 9: holders is no list of strings")
            continue
        try:
            licenses, _ = parse_expression(port["license"])
        except ValueError as broken:
            fail(where, "rule 9: the license %r is no SPDX expression, %s" % (port["license"], broken))
            continue
        ports.append(dict(port, path=PurePosixPath(port["path"]), licenses=licenses, where=where))
    return ports


def notice_sections(text: str) -> dict:
    """The sections of NOTICE by their heading, a line underlined with = as long as it, each with
    the fields its lines name, such as Version and License."""
    lines = text.splitlines()
    sections, current = {}, None
    for index, line in enumerate(lines):
        if line.strip() and index + 1 < len(lines) and lines[index + 1] == "=" * len(line):
            current = sections.setdefault(line, {})
            continue
        match = NOTICE_FIELD.match(line)
        if current is not None and match:
            current.setdefault(match.group(1), match.group(2).strip())
    return sections


def check_notice(port: dict, sections: dict, fail) -> None:
    """Rule 9: NOTICE has the section of the upstream of a port, with its version, its license
    and the path of the port."""
    project, _, version = port["upstream"].rpartition(" ")
    if not project or not VERSION.fullmatch(version):
        fail(port["where"], "rule 9: the upstream %r names no version after the name of the plugin" % port["upstream"])
        return
    section = sections.get(project)
    if section is None:
        fail("NOTICE", "rule 9: NOTICE has no section headed %s, the upstream of %s" % (project, port["name"]))
        return
    if not re.match(re.escape(version) + r"(?:,|$)", section.get("Version", "")):
        fail("NOTICE", "rule 9: the section %s gives the version %r, not %s, the version that %s ports"
             % (project, section.get("Version"), version, port["name"]))
    if section.get("License") != port["license"]:
        fail("NOTICE", "rule 9: the section %s gives the license %r, not %s, the license of %s"
             % (project, section.get("License"), port["license"], port["name"]))
    if str(port["path"]) not in [entry.strip() for entry in section.get("Files", "").split(",")]:
        fail("NOTICE", "rule 9: the section %s does not name %s among its files" % (project, port["path"]))


def check_ports(root: Path, files: list[PurePosixPath], present: set, known: dict, fail) -> None:
    """Rule 9: every file of a port keeps the license and the holders of its upstream."""
    ports = load_ports(root, fail)
    if not ports:
        return
    try:
        sections = notice_sections((root / "NOTICE").read_text(encoding="utf-8"))
    except OSError:
        sections = {}
    for port in ports:
        check_notice(port, sections, fail)
        for identifier in port["licenses"]:
            if identifier not in known:
                fail(port["where"], "rule 9: the license %s of the port has no text under LICENSES/" % identifier)
        owned = [path for path in files if port["path"] in path.parents]
        if not owned:
            fail(port["where"], "rule 9: the path %s of the port %s holds no file of the tree" % (port["path"], port["name"]))
            continue
        for path in owned:
            if path.name.endswith(".license"):
                continue
            lines = head(root, path)
            expression = find_tag(lines) if lines is not None else None
            source = path
            if expression is None:
                sidecar = PurePosixPath(str(path) + ".license")
                lines = head(root, sidecar) if sidecar in present else None
                expression = find_tag(lines, placed=False) if lines is not None else None
                source = sidecar
            if expression is None:
                continue  # Rule 6 reports the missing tag.
            if expression.strip() != port["license"]:
                fail(source, "rule 9: a file of the port %s carries %s, not %s, the license of %s"
                     % (port["name"], expression.strip(), port["license"], port["upstream"]))
            holders = find_copyrights(lines)
            for holder in port["holders"]:
                if not any(holder in line for line in holders):
                    fail(source, "rule 9: no copyright line names %s, a holder of %s" % (holder, port["upstream"]))
        manifest = port["path"] / "Cargo.toml"
        if manifest not in present:
            fail(manifest, "rule 9: the port %s has no Cargo.toml at its path" % port["name"])
            continue
        try:
            package = tomllib.loads((root / manifest).read_text(encoding="utf-8")).get("package") or {}
        except (OSError, ValueError):
            continue  # Rule 4 reports it.
        if package.get("name") != port["name"]:
            fail(manifest, "rule 9: the package is named %r, not %s as ports.toml names the port"
                 % (package.get("name"), port["name"]))
        if package.get("license") != port["license"]:
            fail(manifest, "rule 9: the port states the license %r, not %s, the license of %s"
                 % (package.get("license"), port["license"], port["upstream"]))


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Check the licenses of the tree.")
    parser.add_argument("--root", type=Path, default=ROOT, help="The root of the tree, by default the repository.")
    args = parser.parse_args(argv)
    root = args.root.resolve()
    failures = []

    def fail(path, message):
        failures.append("%s: %s" % (path, message))

    try:
        files = tracked_files(root)
    except Failure as broken:
        print("license_lint: %s" % broken, file=sys.stderr)
        return 1
    known = license_texts(files, fail)
    present = set(files)

    # Rule 1.
    try:
        text = (root / "LICENSE").read_bytes()
        for sentence in AGPL_SENTENCES:
            if sentence.encode() not in text:
                fail("LICENSE", "rule 1: the GNU AGPL 3 lacks the sentence %r" % sentence)
        try:
            if (root / GOLEM_TEXT).read_bytes() != text:
                fail("LICENSE", "rule 1: LICENSE differs from %s" % GOLEM_TEXT)
        except OSError:
            fail(GOLEM_TEXT, "rule 1: the text of the license of Golem is missing")
    except OSError:
        fail("LICENSE", "rule 1: the root holds no LICENSE")

    # Rules 2, 3, 6 and 8, and what rule 7 needs.
    tagged = 0
    foreign = []

    def check_expression(path, expression, origin):
        try:
            licenses, exceptions = parse_expression(expression)
        except ValueError as broken:
            fail(path, "rule 2: %s %r is no SPDX expression, %s" % (origin, expression, broken))
            return
        if path.parts[0] == "third_party":
            return
        for identifier in licenses:
            if identifier not in known:
                fail(path, "rule 3: %s names %s, whose text is not under LICENSES/" % (origin, identifier))
            if not identifier.startswith("AGPL-3.0"):
                foreign.append(path)
        for identifier in exceptions:
            if identifier not in known:
                fail(path, "rule 3: %s names the exception %s, whose text is not under LICENSES/" % (origin, identifier))

    def check_copyright(path, holders):
        if not holders:
            fail(path, "rule 8: the file names no copyright holder in an SPDX-FileCopyrightText line")
        elif not any(GOLEM_HOLDER in holder for holder in holders):
            fail(path, "rule 8: no copyright line names %s, the holder of Golem" % GOLEM_HOLDER)

    for path in files:
        if path.parts[0] == "LICENSES":
            continue
        lines = head(root, path)
        expression = find_tag(lines) if lines is not None else None
        if expression is not None:
            tagged += 1
            check_expression(path, expression, "the tag")
            if path.parts[0] != "third_party":
                check_copyright(path, find_copyrights(lines))
            continue
        if path in UNTAGGED or path.parts[0] == "third_party" or path.name.endswith(".license"):
            continue
        sidecar = PurePosixPath(str(path) + ".license")
        sidecar_lines = head(root, sidecar) if sidecar in present else None
        sidecar_expression = find_tag(sidecar_lines, placed=False) if sidecar_lines is not None else None
        if sidecar_expression is not None:
            check_expression(sidecar, sidecar_expression, "the tag")
            check_copyright(sidecar, find_copyrights(sidecar_lines))
            continue
        fail(path, "rule 6: the file carries no tag on its first line, or on its second under #! or ---, and no %s" % sidecar.name)

    # Rules 6 and 8 for NOTICE, which takes its tags from REUSE.toml.
    if NOTICE in present:
        annotation = notice_annotation(root, present, fail)
        if annotation is not None:
            if annotation.get("precedence") != "override":
                fail(NOTICE, "rule 6: the annotation of NOTICE in REUSE.toml takes the precedence %r, not override"
                     % annotation.get("precedence", "closest"))
            expressions = strings(annotation.get("SPDX-License-Identifier"))
            if not expressions:
                fail(NOTICE, "rule 6: the annotation of NOTICE in REUSE.toml states no SPDX-License-Identifier")
            for expression in expressions:
                check_expression(REUSE_TOML, expression, "the license of NOTICE")
            check_copyright(NOTICE, strings(annotation.get("SPDX-FileCopyrightText")))

    # Rule 4.
    for path in files:
        if path.parts[0] == "third_party" or "node_modules" in path.parts:
            continue
        if path.name == "Cargo.toml":
            try:
                manifest = tomllib.loads((root / path).read_text(encoding="utf-8"))
            except (OSError, ValueError) as broken:
                fail(path, "rule 4: the manifest does not parse: %s" % broken)
                continue
            package = manifest.get("package")
            if isinstance(package, dict) and "license" not in package and "license-file" not in package:
                fail(path, "rule 4: the package states no license")
            for table, origin in ((package or {}, "the field license"),
                                  ((manifest.get("workspace") or {}).get("package") or {}, "the workspace license")):
                value = table.get("license")
                if isinstance(value, str):
                    check_expression(path, value, origin)
                elif value is not None and value != {"workspace": True}:
                    fail(path, "rule 4: %s is neither a string nor inherited from the workspace" % origin)
                license_file = table.get("license-file")
                if isinstance(license_file, str) and not (root / path.parent / license_file).is_file():
                    fail(path, "rule 4: the field license-file names %s, which does not exist" % license_file)
        elif path.name == "package.json":
            try:
                package = json.loads((root / path).read_text(encoding="utf-8"))
            except (OSError, ValueError) as broken:
                fail(path, "rule 4: the package does not parse: %s" % broken)
                continue
            value = package.get("license") if isinstance(package, dict) else None
            if value is None:
                continue
            if not isinstance(value, str):
                fail(path, "rule 4: the field license is not a string")
            elif value.startswith("SEE LICENSE IN "):
                if not (root / path.parent / value[len("SEE LICENSE IN "):]).is_file():
                    fail(path, "rule 4: the field license points to a file that does not exist")
            else:
                check_expression(path, value, "the field license")

    # Rule 5.
    for path in files:
        if path.parts[0] != "third_party" or path.name not in ("Cargo.toml", "package.json", "build.gradle", "build.gradle.kts"):
            continue
        if "node_modules" in path.parts:
            continue
        if not any(other.parent == path.parent and LICENSE_FILE.match(other.name) for other in files):
            fail(path.parent, "rule 5: the package under third_party/ keeps no license file of its own")

    # Rule 7.
    if foreign and not (root / "NOTICE").is_file():
        fail("NOTICE", "rule 7: %s names a license other than the AGPL and the root holds no NOTICE" % foreign[0])

    # Rule 9.
    check_ports(root, files, present, known, fail)

    # A file beside another file may fail as itself and as the other file's .license.
    failures = list(dict.fromkeys(failures))
    for line in failures:
        print("license_lint: %s" % line)
    print("license_lint: files %d, with a license %d, license texts %d, failures %d"
          % (len(files), tagged, len(known), len(failures)))
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
