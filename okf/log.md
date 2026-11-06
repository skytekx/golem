---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Log
title: Change log
description: Chronological change log of Golem from the plan of 2026-11-06 on.
tags: [log, history]
timestamp: 2026-11-06
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
