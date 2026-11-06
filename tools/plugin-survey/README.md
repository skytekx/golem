<!-- SPDX-License-Identifier: AGPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com> -->
# Plugin survey

These tools rebuild the survey of Bukkit and Spigot plugins that orders the Bukkit layer of Golem,
see [Bukkit layer](../../okf/bukkit-layer.md). They collect a corpus of plugin jars that target
Minecraft 1.20.1, read every class file of every jar, and count what the plugins reach: the public
API of Bukkit and Spigot, the internals of CraftBukkit and the classes of Minecraft. Every tool is
Python on the standard library alone and runs with `python3 -I`.

| File | What it does |
| --- | --- |
| `corpus.py` | Builds the corpus: lists the plugins of Spigot, Modrinth and Hangar, downloads the newest file for 1.20.1 of each, drops duplicates and writes `corpus.json` |
| `plugin_yaml.py` | Reads `plugin.yml` and `paper-plugin.yml` as Spigot reads them, by the byte order mark and YAML 1.1, for `corpus.py` and `summary/jar_facts.py` |
| `scan.py` | The scanner: one JSON record per jar with every reference to CraftBukkit, Minecraft, Mojang libraries, Paper and the Bukkit API, the reflection sites with their resolved targets, the version adapters, the shaded libraries and the level |
| `summary/` | The steps that count the records into the numbers of the page of the layer |

## The jars are untrusted data

The jars come from public sites and are untrusted data. The tools never run them, never put them
on a class path and never extract them. `corpus.py` streams each download to a file whose name it
chooses itself and reads it in memory with `zipfile`. `scan.py`, `summary/jar_facts.py` and
`summary/library_roots.py` read the jars in memory with `zipfile`, nested jars included, parse the
class files themselves and never write a file named from inside an archive. `scan.py` calls
`javap` only on classes of the JDK, to list the members that the types of spigot-api inherit from
it. Run the tools with `python3 -I`, keep the survey dir apart from the tools, and pass paths as
arguments, so that no file of a jar can stand in for a module of Python.

## What it reaches

Only `corpus.py` reaches the network:

| Host | What for |
| --- | --- |
| `api.spiget.org`, `cdn.spiget.org` | The list of free Spigot resources and their files |
| `api.modrinth.com`, `cdn.modrinth.com` | The search of Modrinth, the versions and their files, and LuckPerms for the live server |
| `hangar.papermc.io`, `hangarcdn.papermc.io` | The projects of Hangar, their versions and their files |
| `github.com`, `release-assets.githubusercontent.com` | EssentialsX and EssentialsXSpawn 2.22.0 from their release, and Hangar files hosted there |
| The host of an external Hangar link | `hangar_ext` follows an external link when it names a file, in the corpus of 2026-11-06 on `download.geysermc.org` and on `www.dropbox.com` with its download host |

It sends the user agent `golem-plugin-survey/0.1`, waits 0.3 s between two requests to one host,
follows redirects itself so that it paces each hop, retries three times, skips a jar over 60 MiB
and stops before the survey dir passes about 2.8 GiB. `scan.py` and the summary steps reach
nothing.

## Inputs beside the corpus

- `spigot-api-1.20.1-R0.1-20230921.163938-66.jar`, the last build of spigot-api 1.20.1, from the
  snapshot repository of Spigot on `hub.spigotmc.org`. `scan.py` builds its model of the API from
  it and caches it as `api_model.json` in the survey dir.
- `javap` of a JDK on the `PATH`, for the members of the JDK that the API inherits. `scan.py`
  caches them as `jdk_members.json`.
- `bukkit_srg.srg` from the jar of Arclight 1.20.1, which maps the Spigot 1.20.1 names of Minecraft
  to its Mojang class names and SRG member names. The survey of 2026-11-06 takes it from
  `arclight-1.20.1-1.0.6-SNAPSHOT-6de9fec`. `summary/order.py`, `summary/minecraft_rank.py`,
  `summary/details.py` and `summary/mapping_lookup.py` read it.

## Build the corpus

```sh
python3 -I corpus.py <dir> live
python3 -I corpus.py <dir> spigot        # 250 kept by default
python3 -I corpus.py <dir> modrinth      # 150
python3 -I corpus.py <dir> hangar        # 100
python3 -I corpus.py <dir> hangar_ext    # the external Hangar links that name a jar
python3 -I corpus.py <dir> index         # writes <dir>/corpus.json
```

`<dir>` takes `jars/`, the log of every decision per source in `work/<source>.jsonl` and
`corpus.json`. A step that stops resumes where it stopped, and `redup` reads every kept jar again
and applies the rule for duplicates to the kept set once more.

## Scan

```sh
python3 -I scan.py --dir <dir> --spigot-api <spigot-api jar> --jobs 3 --out <dir>/scan.jsonl
```

`scan.py` writes one record per jar in the format that the reconciled data of the survey keeps.
The reconciled data of the survey of 2026-11-06, `reconciled.jsonl`, adds the corrections of a
judge pass that compares these records with those of a second, independent scanner, each
correction with its reason. The second scanner and the judge pass stay outside the repository, so
a scan of these tools gives the records without those corrections. Over the corpus of 2026-11-06,
`scan.py` with 3 jobs takes about two and a half minutes on the machine of the survey and gives the
records of the survey again, up to the time it measures and the order of a few lists.

## Count

```sh
python3 -I summary/run.py <dir> <out> --srg <bukkit_srg.srg> [--data <records>] [--api <api model>]
```

`run.py` runs every step in order and keeps what each prints as `<out>/<step>.txt`, beside the JSON
that the steps hand on. `--data` names the records, `<dir>/reconciled.jsonl` by default, so the
records of `scan.py` count with `--data <dir>/scan.jsonl`. `--api` names the model of spigot-api,
`<dir>/api_model.json` by default. Each step also runs alone with the same arguments.

| Step | Writes | What it counts |
| --- | --- | --- |
| `corpus_counts.py` | | The corpus by source, the duplicates, the skips and the two download totals |
| `jar_facts.py` | `jars_facts.json` | The Java version of the main class and of all class files, the entry dates, and the descriptor as Spigot reads it: its `api-version`, whether YAML refuses it and whether its main class exists |
| `levels.py` | `levels.json` | The level of every plugin on all code paths and on 1.20.1 |
| `members.py` | | The named members of CraftBukkit and of the API jar, directly and by reflection |
| `groups.py` | `groups.json` | The stand-in groups of level 2 per plugin |
| `library_roots.py` | `libroots.json` | The roots of shaded libraries by their marker classes |
| `libraries.py` | | The shaded libraries as the reconciled data detects them |
| `library_reach.py` | | The carriers of each library, their reach on 1.20.1, and the plugins whose internal access lies only in libraries |
| `minecraft_classes.py` | | Eight leading Minecraft classes under their names of 1.20.1 and the legacy lookups |
| `minecraft_rank.py` | `nmsrank.json` | The Minecraft classes by the names written in the plugins |
| `public_api.py` | | The use of the public API, the events and the members spigot-api 1.20.1 lacks |
| `order.py` | `order.json` | The requirement set of every plugin and the greedy order of work |
| `classify.py` | `classes.json` | One class per plugin |
| `details.py` | | The naming of Minecraft, the plugins others depend on, CraftBukkit's additions to Minecraft classes, the patching flags, plugin.yml libraries and the descriptors without `api-version` |

`mapping_lookup.py <bukkit_srg.srg> <class>[#<member>]` looks single names up in the mapping.

Rerun on 2026-11-06 over the survey of that day, its `reconciled.jsonl` and the API model of its
judge pass, the steps give the counts of the page of the layer in about 40 seconds.

## What stays out of the repository

The jars, `corpus.json`, the logs under `work/`, the records and every output of the steps stay in
the survey dir and never enter the repository, nor does any code of a plugin.
