---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Log
title: Change log
description: Chronological change log of Golem from the plan of 2026-11-06 on.
tags: [log, history]
timestamp: 2026-11-07
---

# Change log

Oldest first. Times are UTC.

## 2026-11-06

- The owner set the plan of Golem and the repository `skytekx/golem`. See [Plan](plan.md).
- The license went in: the AGPL 3 only, every file with its SPDX lines, the holder Florian Kleber,
  REUSE, and the license lint with its notebook `01_license_lint`.
- The commit rules went in: Commitizen in `.cz.toml`, the commit lint for the rules of the kernel,
  the hooks under `tools/hooks/`, the notebook `02_commit_lint` and CONTRIBUTING.md. The history
  of the branch was rewritten into that form.
- A spike booted Forge 47.4.18 on Temurin 17.0.21 through the JNI invocation API inside a Rust
  process, reached `Done` in 26 s, stopped it through the server thread and saw `DestroyJavaVM`
  return 0. A second spike proved that a Forge mod calls a native method of a class on the module
  path that the host registered. See [Bridge](bridge.md).
- The Cargo workspace of the first milestone went in, with the crates `golem`, `golem-jvm`,
  `golem-bridge`, `golem-plugin`, `golem-permissions` and `golem-devtool`.
- Four agents built the parts of the first milestone in parallel, each in a work tree of its own,
  and their commits went onto the branch one part after the other:
  - The JVM host: `golem-jvm` reads the argument files and the launch of a Forge server as the java
    launcher of JDK 17 does and runs it in a JVM on the thread `golem-jvm`, and `golem run` boots
    the server of a directory with `-Xrs` and the host jar. Notebook `03_launch_args`. See
    [JVM host](jvm-host.md).
  - The natives and the plugins: `golem-bridge` answers the 14 natives and registers them,
    `golem-plugin` runs the plugins with budgets, and the core plugin answers `golem`. Notebook
    `04_plugins`. See [Plugins](plugins/index.md) and [Core plugin](plugins/core.md).
  - The Java bridge: the Gradle build of `golem-host.jar` and of the Forge mod, which stays off
    without Golem. Notebook `05_bridge_build`, which found that Forge 47 calls `chat()` on a worker
    thread and `commandNames()` on the main thread at startup. See [Bridge](bridge.md).
  - The permissions: `golem-permissions` with its strict format and its rules, and the seed of
    SkyTekx in `config/permissions.toml`. Notebook `06_permissions`. See
    [Permissions](plugins/permissions.md).
- The merge showed that a chat message on the worker thread passed every plugin while a tick held
  the lock of the handler. The natives now wait for a call of another thread and refuse only a
  call from inside the handler, and `04_plugins` holds both.
- A boot without the bridge mod showed that no signal stopped the server. A signal without a
  connected bridge now ends the JVM through `System.exit` and the shutdown hook of the server, as
  the java launcher does.
- `07_forge_boot` boots Forge under `golem run` with the real natives, a player of the harness and
  both stop paths. It passed on the first run.
- The commit lint now refuses trailers outside the last paragraph and looks up the commit that a
  Fixes trailer names, after a Fixes hash that named no commit went out. The history was rewritten
  again so that every trailer stands in the last block.
- CI builds and lints the crates, runs cargo deny and runs all seven notebooks, with the downloads
  of `tests/fetch.py` and caches for cargo, the downloads and Gradle. The first green run took six
  minutes for the notebooks.
- The issues [#1](https://github.com/skytekx/golem/issues/1) to
  [#7](https://github.com/skytekx/golem/issues/7) record the quirks that the first milestone fixes:
  three of the permissions, the stop path of Arclight and three of the java launcher.
- An adversarial review of the merged milestone, six reviewers with a skeptic each, confirmed 20
  findings and refuted 4. Each confirmed one is fixed with a notebook case where it had a behaviour:
  the lock of the handler, `SIGSEGV` and `SIGBUS` chained to the handler of Rust, `SIGQUIT` that
  ended the server, `SIGHUP` under nohup, the answers of plugin commands that `sendCommandFeedback`
  hid, `-J-server` and a source file in upper case, a merge commit at the edge of a shallow clone,
  the target directory of cargo, the include of macOS, the missing steps of CI and the pages that
  said what was no longer true.
- Three parts went onto the branch `integrate/foundation`, each built by an agent of its own:
  - The check of the modpack: `tests/fetch.py --only modpack` builds the SkyTekx 3 modpack of the
    live server from a pinned commit of its repository, and `08_modpack_boot` boots it under java,
    golem and Arclight and then under golem on the world that Arclight wrote. Golem booted the 289
    mods as java did, so nothing of Golem needed a fix. See [Modpack](modpack.md).
  - The game access: the bridge passes the permission level of the source of a command, reports
    where a player died, teleports players and names them in the player list, answers the
    PermissionAPI of Forge through the handler `golem:permissions`, and lets the plugins ask the
    game on the server thread, with the natives `connect` and `permission`, 16 in all. The core
    plugin gives operators the tools of `/golem` that ask the game. See [Bridge](bridge.md) and
    [Core plugin](plugins/core.md).
  - The store: `golem-store` keeps the data of the plugins in one SQLite file, `golem-devtool
    store` drives it from a JSON script, and `09_store` holds it to its schema, its refusals and
    its crashes. See [Store](store.md).
- The owner ruled that plugins call the Golem API in process through the executor of Aosabi, and
  that Bukkit and Spigot plugins run unchanged through a Bukkit layer on the API. See
  [Decisions](decisions.md). A design panel on Opus and Fable wrote the plugin API and the format
  of its coverage check. See [Plugin API](api.md) and [API coverage](api-coverage.md).
- A survey of 503 plugins that run on Spigot 1.20.1 measured the three levels of the Bukkit layer,
  and `tools/plugin-survey/` rebuilds it. See [Bukkit layer](bukkit-layer.md).
- An adversarial review of the three parts, reviewers on Opus and on Fable and a judge for each
  part, confirmed 29 findings of the check of the modpack, 22 of the game access and 26 of the
  store, and refuted 4, 5 and 4.
- The fixes of the check of the modpack went in, each with its notebook case: the fetch starts a
  broken resume over, refuses a malformed index with one line, follows redirects only over https
  to known hosts, keeps only what the pin names and fetches the 31 libraries of Arclight, so its
  boot downloads nothing, and CI saves the downloads and the pack right after the fetch. Notebook
  08 asserts that java loads every jar, pins the lines of golem itself and its warnings on the
  world of Arclight, takes a fixed count of samples of the TPS, reads a decimal comma, gives the
  voice chat of each copy a free port, keeps the logs of a failed boot, scans the coremods and
  mixin configs, finds the native handlers of signals, fails every boot on a failure of the
  storage of the world and checks the stop of every boot by its saves.
- The issues [#99](https://github.com/skytekx/golem/issues/99) to
  [#104](https://github.com/skytekx/golem/issues/104) record the quirks that the check found: the
  two conflicts of mixins of Arclight with Apothic Attributes and Apotheosis, the libraries that
  Arclight downloads from a third party mirror, the pack of the server repository that differs
  from the mods of the live server, the save of Arclight that skips the tail of vanilla, and the
  key of TiCEX that Forge corrects at every start.

## 2026-11-07

- The fixes of the game access went in, each with its notebook case: the search for a safe place
  reads loaded chunks alone and keeps out portals, cobwebs, the bedrock roof of the Nether and the
  edge of the world border, a teleport runs as the command `tp` runs it, an infinite number
  travels as Java reads it, a call of another thread waits at most one second for the handler, a
  failing question prints its stack trace once, a command run as a player is judged by the source
  that ran it, the game answers nothing from `serverStopped` on, an operator level beyond 0 to 4
  reads as the nearest one, a player whose answer throws leaves the others, and the mod warns when
  Forge takes another permission handler. The core plugin says that the game gives no answer
  instead of refusing, and `/golem listname` keeps the text as the operator typed it. The
  scripted game of the devtool answers as the game of the mod. The pages name the respawn point
  as the point that the player holds, say that a permission answer holds in every context and
  record that Arclight hands every Boolean node of a mod to LuckPerms.
- The fixes of the store went in, each with its notebook case: the permissions and their
  generation are one read, a check copies the cache and queues its read under one lock, copies
  pointers instead of rows and sums the log of an account without an overflow, every close answers
  as the first, a second open in the same process is named, the open reads what a file is before
  it may write to it, a coordinate of -0.0 is 0.0, the changes that wait for the writer are
  bounded, a failed writer says which changes it loses and gives its reason to every later call,
  `PermissionSet::from_toml` refuses a key that `golem-permissions` refuses, and
  `libsqlite3-sys` is pinned exactly. The devtool refuses a weight that is no integer, stops
  without a panic when its output closes and closes a store once. The page states how the
  migration carries the balances and the names of EssentialsX and what a change costs the server
  thread, a burst that waits for the writer included.
- Open in the game access stays that `/golem status` does not name the permission handler that
  Forge took. See [Open items](open-items.md).
- The issues [#105](https://github.com/skytekx/golem/issues/105) to
  [#107](https://github.com/skytekx/golem/issues/107) record the quirks that the reviews of the
  game access and of the store found: Arclight hands every Boolean permission of a mod to
  LuckPerms, and EssentialsX keeps balances with fractions of a cent and below zero and folds the
  names of homes, warps and spawn groups, which the store holds otherwise. The issues #102 to #104
  now say what notebook 08 checks.
- The shared pages follow the merge: [Testing](../Documentation/testing.md) names what every
  notebook needs and asks, and the index, the open items and this log name the parts that are
  built.
