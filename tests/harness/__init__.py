# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
"""Golem harness: the Python helpers every notebook imports.

Every test case of Golem is a notebook executed by papermill, as in Aosabi. A notebook drives
the programs under test through subprocesses and checks what they print, write and return, so
a failing run explains itself in the cells of its notebook.

A notebook imports the package with the tests directory of the repository on the path:

    import sys
    from pathlib import Path

    for _base in [Path.cwd(), *Path.cwd().parents]:
        if (_base / "tests" / "harness" / "__init__.py").is_file():
            sys.path.insert(0, str(_base / "tests"))
            break

    from harness import paths, run

tests/run.py also exports the tests directory on PYTHONPATH, so under the runner the loop
above finds the package on the first try.
"""

from __future__ import annotations

import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

#: The file whose presence marks the root of the repository.
ROOT_MARKER = Path("tests") / "harness" / "__init__.py"

#: The environment every process of a notebook runs with, so that no tool sends telemetry or
#: asks a question.
QUIET = {"DO_NOT_TRACK": "1", "PYTHONDONTWRITEBYTECODE": "1", "LC_ALL": "C.UTF-8"}


class RootNotFound(RuntimeError):
    """No ancestor of the starting directory holds tests/harness/."""


@dataclass(frozen=True)
class Paths:
    """The directories a notebook needs, all absolute, all derived from the root."""

    root: Path
    tests: Path
    notebooks: Path
    out: Path

    def rel(self, path) -> str:
        """`path` as a repository relative string, which is what a cell prints."""
        try:
            return str(Path(path).resolve().relative_to(self.root))
        except ValueError:
            return str(Path(path))


def find_root(start=None) -> Path:
    """Walk up from `start` to the root of the repository."""
    probe = Path(start or os.getcwd()).resolve()
    for directory in [probe, *probe.parents]:
        if (directory / ROOT_MARKER).is_file():
            return directory
    raise RootNotFound("no ancestor of %s holds %s" % (probe, ROOT_MARKER))


def paths(root=None) -> Paths:
    """The directories of the repository at `root`, or of the one around the working directory."""
    base = find_root(root or None) if root in (None, "", ".") else Path(root).resolve()
    tests = base / "tests"
    return Paths(root=base, tests=tests, notebooks=tests / "notebooks", out=tests / "notebooks" / "out")


@dataclass(frozen=True)
class Result:
    """What a process returned and printed."""

    argv: list
    returncode: int
    stdout: str
    stderr: str


def run(argv, cwd=None, timeout=120, check=False) -> Result:
    """Run `argv` with the quiet environment and return what it printed.

    The interpreter of the kernel stands in for python3, so a script runs on the Python the
    harness pinned. With `check`, a non zero exit raises with both streams in the message.
    """
    argv = [sys.executable if part == "python3" else str(part) for part in argv]
    environment = dict(os.environ)
    environment.update(QUIET)
    completed = subprocess.run(
        argv, cwd=cwd, env=environment, capture_output=True, text=True, timeout=timeout, check=False
    )
    result = Result(argv, completed.returncode, completed.stdout, completed.stderr)
    if check and result.returncode != 0:
        raise AssertionError(
            "%s exited %d\n--- stdout\n%s--- stderr\n%s" % (argv, result.returncode, result.stdout, result.stderr)
        )
    return result
