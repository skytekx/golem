<!-- SPDX-License-Identifier: AGPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com> -->
# Testing

Every test of Golem is a Jupyter notebook in Python under
[tests/notebooks/](../tests/notebooks/), run by papermill, as in
[Aosabi](https://github.com/netsnek/aosabi/blob/main/Documentation/guides/conformance.md). A
notebook asks one question, drives the programs under test through subprocesses, and asserts on
what they print, write and return. A failing run explains itself in the cells of its notebook,
and the executed copy shows every output on the way to the failure.

## 1. The parts

| Path | What it is |
| --- | --- |
| `tests/notebooks/` | The notebooks, one test case each, named `NN_topic.ipynb` |
| `tests/matrix.yaml` | The entries the runner sweeps, in order, with their parameters |
| `tests/run.py` | The runner, papermill over the kernel `golem` |
| `tests/harness/` | What the notebooks share: where the repository is and how a process runs |
| `tests/pyproject.toml` | The exact pins of the Python packages the harness needs |
| `tests/notebooks/out/` | The executed notebooks and `report.json` of a run, never committed |

## 2. Running the notebooks

The harness runs in an environment of its own under `tests/.venv`, which git ignores:

```
python3 -m venv tests/.venv
tests/.venv/bin/python -m pip install papermill==2.7.0 nbclient==0.11.0 nbformat==5.11.1 \
    ipykernel==7.3.0 jupyter_client==8.10.0 pyyaml==6.0.3 reuse==6.3.0 commitizen==4.19.2
tests/.venv/bin/python -m ipykernel install --sys-prefix --name golem --display-name golem

tests/.venv/bin/python tests/run.py
tests/.venv/bin/python tests/run.py --only 01_license_lint
tests/.venv/bin/python tests/run.py --list
```

The pins are the ones in `tests/pyproject.toml`, and CI reads them from there. The runner prints
one line per entry and a summary, writes `tests/notebooks/out/<id>.ipynb` for every entry and
`tests/notebooks/out/report.json` with the outcome of each, and exits with 1 when any entry
failed. For a failure the report names the cell and the error that stopped it.

CI runs the whole matrix on every push and every pull request, and keeps the executed notebooks
of a failed run as an artifact for a week.

## 3. Writing a notebook

A notebook follows the form of the notebooks of Aosabi.

1. A first Markdown cell that states the question the notebook answers and what it compares.
2. A code cell tagged `parameters`, which papermill overrides. It holds `golem_root`, the tree
   under test, and the pins of the notebook. The runner passes `golem_root` itself.
3. A setup cell that finds the harness and prints the versions it rests on:

   ```python
   for _base in [Path.cwd(), *Path.cwd().parents]:
       if (_base / "tests" / "harness" / "__init__.py").is_file():
           sys.path.insert(0, str(_base / "tests"))
           break

   from harness import paths, run
   ```

4. One section per part of the question, each a Markdown cell that says what the next cell
   checks and a code cell that asserts it, with a message that says what went wrong.
5. A last cell that prints the verdict. A run that reaches it passed.

A notebook is committed without outputs, writes only below `tests/notebooks/out/` or a temporary
directory, and prints repository relative paths, so no machine path enters a committed file. It
gets an entry in `tests/matrix.yaml`. A notebook is JSON and cannot hold a comment, so its SPDX
tags stand in a `.license` file beside it, as [licensing.md](licensing.md) describes.

A fix or a feature adds or extends the notebook cell that proves it.

## 4. The notebooks

| Notebook | Question |
| --- | --- |
| `01_license_lint` | Does the license lint pass this tree, refuse every break of its eight rules, and agree with `reuse lint`? |
| `02_commit_lint` | Does the commit lint pass the history, refuse every break of its eight rules, and agree with Commitizen? |
