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
| `tests/harness/` | What the notebooks share: where the repository is, how a process runs, and a player of Minecraft 1.20.1 in `minecraft.py` |
| `tests/fetch.py` | The downloads of the notebooks that boot Forge: Temurin 17, the Forge server and Gradle, each checked by its SHA-256. With `--only modpack` it builds the SkyTekx 3 modpack of the live server from a pinned commit of its repository instead: every jar of the index checked by its size and the SHA-512 of the index, the Arclight jar by its SHA-256, and the 31 libraries of the `installer.json` of that jar by their SHA-256 |
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

The notebooks that build or boot Golem need more than Python. `cargo` of the toolchain in
`rust-toolchain.toml` builds `golem` for 03, 07 and 08 and `golem-devtool` for 03, 04, 06, 08 and
09. `golem-devtool` takes `golem-store`, whose bundled SQLite the build compiles with the C compiler
of the host, so every notebook that builds it needs a C compiler as `cc`, and so does 05 for its
stand in natives. `tests/fetch.py` downloads Temurin 17.0.21, installs the Forge server
1.20.1-47.4.18 and unpacks Gradle 8.14.3 under `tests/.cache/`, which git ignores, for 03, 05, 07
and 08. The first build of the bridge downloads and remaps Minecraft into the Gradle cache of the
user, which takes minutes, and later builds take seconds:

```
python3 -I tests/fetch.py
```

Notebook 08 boots the modpack of the live server, which it needs fetched first:

```
python3 -I tests/fetch.py --only modpack
```

The fetch takes the repository of the server at the pinned commit without a work tree, and the jars
of the pack and the libraries of Arclight from GitHub, Modrinth, Maven Central and the repository of
Sponge. `du` measured the result at about 1.4 GB under `tests/.cache/modpack` and
`tests/.cache/skytekx3-1201-server` on 2026-11-07 in the cloud container of the session. The four
boots of the notebook run one after the other, since each held about 7.5 GiB of resident memory,
and the whole notebook took 19 minutes in its run of 2026-11-06 in the same container, see
[Modpack](../okf/modpack.md#running-it). The parameter `arclight = False` leaves out the boot under
Arclight and the boot of golem on its world.

Notebook 09 needs the module `sqlite3` of the Python of the harness with SQLite 3.37 or newer, since
it writes rows into a copy of a store, whose tables are `STRICT`, and builds the files and logs of
other applications that the store must refuse.

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
directory, and prints repository relative paths, so no machine path enters a committed file. The
builds it runs are the exception: cargo writes its target directory, `target/` unless
`CARGO_TARGET_DIR` moves it, and Gradle fills its cache in `~/.gradle` unless `GRADLE_USER_HOME`
moves it, as every build of the tree does. It gets an entry in `tests/matrix.yaml` and a row in the
table of section 4, which the docs lint `.github/scripts/docs_lint.py` checks beside the prose of
this page. A notebook is JSON and cannot hold a comment, so its SPDX tags stand in a `.license` file
beside it, as [licensing.md](licensing.md) describes.

A fix or a feature adds or extends the notebook cell that proves it.

## 4. The notebooks

| Notebook | Question |
| --- | --- |
| `01_license_lint` | Does the license lint pass this tree, refuse every break of its nine rules, and agree with `reuse lint`? |
| `02_commit_lint` | Does the commit lint pass the history, refuse every break of its eight rules, and agree with Commitizen? |
| `03_launch_args` | Does the JVM host read the launch of a Forge server as the java launcher of Temurin 17 reads it, and does a program run in its embedded JVM as it runs under java? |
| `04_plugins` | Do the 16 natives of the bridge match `okf/bridge.md`, and do the plugin runtime, the core plugin with the tools of operators, the chain of plugins that decide a permission, a port that answers a permission as LuckPerms did through Arclight, the questions of the plugins to a scripted game, the queue, the tick statistics and calls from other threads answer as `okf/plugins/` says, without a JVM? |
| `05_bridge_build` | Does the Gradle build of `bridge/` give the host jar and the reobfuscated mod of `okf/bridge.md`, does the mod stay off with one warning on a stock Forge without the host jar or its natives, and with stand in natives does it call every native, answer every question of the game on the server thread and none from `serverStopped` on, and run every action, and does a fourth boot on a world whose `forge-server.toml` names `forge:default_handler` log one warning? |
| `06_permissions` | Does `golem-permissions` refuse every break of its format, decide every check by the rules of `okf/plugins/permissions.md` in their order, and does the seed grant the 39 nodes of the live group default and no admin command? |
| `07_forge_boot` | Does `golem run` boot Forge with the bridge connected to the real natives, answer `golem` on the console and to a player, tell the sources of a command apart, read the game with the respawn point as Minecraft stores it, teleport and name a player through the tools of operators, hold the search for a safe place to each of its rules, answer through the JNI without Minecraft when a question waits, throws, does not read or names 2000 players, and stop on a signal with the worlds saved, with and without the bridge mod? |
| `08_modpack_boot` | Does `golem run` boot the 289 mods of the SkyTekx 3 modpack of the live server as the java launcher does, with the same mods, errors and warnings of the loader, 20 TPS and a clean stop, and open the world that Arclight wrote, and does `tests/fetch.py --only modpack` hold to its docstring? |
| `09_store` | Does `golem-store` keep the schema of `okf/store.md`, answer and refuse every call, keep a prefix of its changes after a kill, a failed writer and a panic, refuse every file that is no store without changing it, bound the changes that wait for the writer, and answer the calls of the server thread inside their budget? |
