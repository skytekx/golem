---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Spec
title: Core plugin
description: The core plugin of Golem and its command golem, which names the license and the source of the running build as section 13 of the AGPL asks, the TPS and MSPT of the server and the budget of every plugin.
tags: [plugins, core, command, agpl, source, status]
timestamp: 2026-11-06
---

# Core plugin

The core plugin is the plugin `golem-core` of the crate `golem-plugin`, `CorePlugin`. It answers the
command `golem` and logs once that the bridge is connected. The program golem adds it to the
[plugin runtime](index.md) before any other plugin, so it owns the name `golem`.

## The command golem

The command needs no permission, since section 13 of the AGPL gives every player the right to the
source, and the status names nothing that a player could misuse. The first word of the arguments
picks the subcommand, in any case, and the words after it are ignored.

| Command | Answer |
| --- | --- |
| `/golem` | The help |
| `/golem help` | The help |
| `/golem source` | The license, the version, the repository, the commit and the link to its source |
| `/golem status` | The TPS, the MSPT and the ticks, and the budget of every plugin |
| `/golem <other>` | `The subcommand <other> of /golem is unknown.` and the help |

The console runs the same command, without the slash, and sees the same lines.

## Source

Golem is licensed under the AGPL 3 only, and section 13 of the AGPL asks a changed Golem that
players use over the network to offer them its source. `/golem source` answers five lines.

```text
Golem 0.1.0 is free software under the GNU Affero General Public License 3.0 only, AGPL-3.0-only.
Section 13 of the AGPL gives every player of this server the right to the source of the Golem it runs.
Repository: https://github.com/skytekx/golem
Commit: 4b825dc642cb6eb9a060e54bf8d69288fbee4904
Source of this build: https://github.com/skytekx/golem/tree/4b825dc642cb6eb9a060e54bf8d69288fbee4904
```

The link is the repository followed by `/tree/` and the commit, with a trailing slash or `.git` of
the repository dropped. While the commit is unknown, the line `Commit: unknown` stands and the link
is the repository alone.

`Source::of_build()` gives the core plugin the version and the repository of the workspace and the
commit that the environment variable `GOLEM_COMMIT` names when the crate is built, so a release
build sets it to the output of `git rev-parse HEAD`. A build without it names the commit unknown.
`Source::lines()` answers the same five lines, so the program golem prints them for
`golem source` as well.

## Status

`/golem status` reads the tick statistics of the bridge, `golem_bridge::stats()`, and the budgets
of the runtime through the context of its call.

```text
Golem 0.1.0: TPS 20.00, MSPT 1.00, ticks 2
Plugins: 1, with a budget of 1.000 ms per call
golem-core: calls 5, total 0.002 ms, longest 0.001 ms, over budget 0, panics 0
```

The first line follows [Plugins](index.md#tick-statistics): the MSPT is the mean over the last 100
ticks and the TPS is capped at 20. One line per plugin follows, in the order of the runtime, with
the counts of [Plugins](index.md#budgets). The call of the status itself is counted after it
answers.

## Help

```text
Golem 0.1.0, the subcommands of /golem:
/golem source: the license of Golem and the source of this build
/golem status: the TPS, the MSPT and the time every plugin costs
/golem help: this help
```

## Events

On the first `ServerStarting` the core plugin logs one line, so the log of the server shows that
the bridge reaches the Rust of Golem and which versions it runs.

```text
[golem] the bridge is connected: Forge 47.4.18, Minecraft 1.20.1
```

A later `ServerStarting` logs nothing. Every event passes.

## Tests

The notebook `tests/notebooks/04_plugins.ipynb` checks every answer above in its section on the
core plugin: the five lines of the source with a known commit, with an unknown commit and of the
build, the help of `/golem` and `/golem help`, the error of an unknown subcommand, the first line
of the status against the statistics of the bridge, and the single line of the log.

Related: [Plugins](index.md), [Bridge](../bridge.md), [Licensing](../../Documentation/licensing.md).
