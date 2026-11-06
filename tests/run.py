# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
"""Golem harness: the runner that sweeps the notebook matrix.

Every test case of Golem is a notebook under tests/notebooks/, and this file runs the ones that
tests/matrix.yaml names through papermill, in order. It writes one executed notebook per entry
into tests/notebooks/out/ and a report.json with the outcome of every entry and, for a failure,
the cell and the error that stopped it.

    python tests/run.py
    python tests/run.py --only 01_license_lint
    python tests/run.py --list

The interpreter is the one of the harness environment, with the packages of tests/pyproject.toml
and the kernel golem installed. The exit status is 1 when any entry failed, and 2 when --only
names an entry the matrix does not hold.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

# The harness package sits beside this file. Bytecode stays out of the tree.
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))

from harness import QUIET, find_root  # noqa: E402

REPORT_SCHEMA = "golem-harness-report-1"
DEFAULT_KERNEL = "golem"
DEFAULT_CELL_TIMEOUT = 600

ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


def load_matrix(path: Path) -> list:
    """The entries of the matrix file, each with its id, notebook, parameters and timeout."""
    import yaml

    document = yaml.safe_load(path.read_text(encoding="utf-8")) or {}
    raw_entries = document.get("entries") if isinstance(document, dict) else document
    if not isinstance(raw_entries, list):
        raise ValueError("%s holds no list of entries" % path)
    entries, seen = [], set()
    for position, raw in enumerate(raw_entries):
        if not isinstance(raw, dict) or "notebook" not in raw:
            raise ValueError("entry %d of %s carries no notebook key" % (position, path))
        name = raw.get("id") or Path(raw["notebook"]).stem
        if name in seen:
            raise ValueError("the id %s appears twice in %s" % (name, path))
        seen.add(name)
        entries.append({
            "id": name,
            "notebook": raw["notebook"],
            "parameters": dict(raw.get("parameters") or {}),
            "cell_timeout": raw.get("cell_timeout") or DEFAULT_CELL_TIMEOUT,
        })
    return entries


def resolve_notebook(root: Path, notebook: str) -> Path:
    """A matrix entry names a repository relative path or a bare notebook name."""
    direct = root / notebook
    if direct.is_file():
        return direct
    name = notebook if notebook.endswith(".ipynb") else notebook + ".ipynb"
    return root / "tests" / "notebooks" / Path(name).name


def first_error(executed: Path):
    """The first error output of an executed notebook, as plain data, or None."""
    import nbformat

    try:
        notebook = nbformat.read(str(executed), as_version=4)
    except (OSError, ValueError):
        return None
    for index, cell in enumerate(notebook.cells):
        for output in cell.get("outputs") or []:
            if output.get("output_type") == "error":
                return {
                    "cell_index": index,
                    "ename": output.get("ename", ""),
                    "evalue": ANSI.sub("", output.get("evalue", "")),
                    "traceback": [ANSI.sub("", line) for line in output.get("traceback") or []],
                }
    return None


def run_entry(root: Path, entry: dict, kernel: str) -> dict:
    """Run one entry through papermill and return its line of the report."""
    import papermill

    source = resolve_notebook(root, entry["notebook"])
    out = root / "tests" / "notebooks" / "out"
    out.mkdir(parents=True, exist_ok=True)
    executed = out / ("%s.ipynb" % entry["id"])
    parameters = dict(entry["parameters"])
    parameters.setdefault("golem_root", str(root))
    started = time.monotonic()
    status, error = "passed", None
    try:
        papermill.execute_notebook(
            str(source),
            str(executed),
            parameters=parameters,
            kernel_name=kernel,
            cwd=str(root),
            execution_timeout=entry["cell_timeout"],
            progress_bar=False,
        )
    except Exception as failure:  # papermill raises its own error for a failing cell
        status = "failed"
        error = first_error(executed) or {"ename": type(failure).__name__, "evalue": str(failure)}
    return {
        "id": entry["id"],
        "notebook": str(source.relative_to(root)),
        "executed": str(executed.relative_to(root)),
        "status": status,
        "seconds": round(time.monotonic() - started, 3),
        "error": error,
    }


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Run the notebook matrix of Golem.")
    parser.add_argument("--matrix", type=Path, default=None, help="The matrix, by default tests/matrix.yaml.")
    parser.add_argument("--only", action="append", default=[], help="Run only the entry with this id. May repeat.")
    parser.add_argument("--kernel", default=DEFAULT_KERNEL, help="The Jupyter kernel, by default golem.")
    parser.add_argument("--list", action="store_true", help="Print the ids of the matrix and exit.")
    args = parser.parse_args(argv)

    root = find_root(Path(__file__).resolve().parent)
    entries = load_matrix(args.matrix or root / "tests" / "matrix.yaml")
    if args.list:
        for entry in entries:
            print(entry["id"])
        return 0
    if args.only:
        unknown = sorted(set(args.only) - {entry["id"] for entry in entries})
        if unknown:
            print("run: the matrix holds no entry %s" % ", ".join(unknown), file=sys.stderr)
            return 2
        entries = [entry for entry in entries if entry["id"] in args.only]

    os.environ.update(QUIET)
    os.environ["PYTHONPATH"] = os.pathsep.join(filter(None, [str(root / "tests"), os.environ.get("PYTHONPATH")]))
    results = []
    for entry in entries:
        result = run_entry(root, entry, args.kernel)
        results.append(result)
        line = "run: %-6s %s in %.1f s" % (result["status"], result["id"], result["seconds"])
        if result["error"]:
            line += " - %s: %s" % (result["error"]["ename"], result["error"]["evalue"].splitlines()[0] if result["error"]["evalue"] else "")
        print(line, flush=True)

    passed = sum(result["status"] == "passed" for result in results)
    report = {
        "schema": REPORT_SCHEMA,
        "finished": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "python": sys.version.split()[0],
        "passed": passed,
        "failed": len(results) - passed,
        "entries": results,
    }
    out = root / "tests" / "notebooks" / "out"
    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print("run: entries %d, passed %d, failed %d" % (len(results), passed, len(results) - passed))
    return 0 if passed == len(results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
