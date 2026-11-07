---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Spec
title: Core plugin
description: The core plugin of Golem and its command golem, which names the license and the source of the running build as section 13 of the AGPL asks, the TPS and MSPT of the server, the budget of every plugin, who runs a command, and the tools of operators that read the game, teleport a player to a safe place and name them in the player list.
tags: [plugins, core, command, agpl, source, status, operators, game]
timestamp: 2026-11-07
---

# Core plugin

The core plugin is the plugin `golem-core` of the crate `golem-plugin`, `CorePlugin`. It answers the
command `golem` and logs once that the bridge is connected. The program golem adds it to the
[plugin runtime](index.md) before any other plugin, so it owns the name `golem`.

## The command golem

The subcommands `source`, `status`, `whoami` and `help` need no permission, since section 13 of
the AGPL gives every player the right to the source, and they name nothing that a player could
misuse. The tools of operators read the game and change it, so only the console, a player of
operator level 2 and more and another source of level 2 and more may run them, `OPERATOR_LEVEL`.
The first word of the arguments picks the subcommand, in any case, and the words after it are its
arguments, which `source`, `status`, `whoami` and `help` ignore, while `listname` takes its text
as it is typed.

| Command | Who | Answer |
| --- | --- | --- |
| `/golem` | Everyone | The help |
| `/golem help` | Everyone | The help |
| `/golem source` | Everyone | The license, the version, the repository, the commit and the link to its source |
| `/golem status` | Everyone | The TPS, the MSPT and the ticks, and the budget of every plugin |
| `/golem whoami` | Everyone | Who Golem sees run the command, and at which level |
| `/golem player <name>` | Operators | Where a player is and how they play |
| `/golem world [<dimension>]` | Operators | The spawn and the heights of a dimension, by default the Overworld |
| `/golem teleport <name> <dimension> <x> <y> <z>` | Operators | A teleport to the nearest safe place |
| `/golem listname <name> [<text>]` | Operators | The name of a player in the player list, cleared without a text |
| `/golem <other>` | Everyone | `The subcommand <other> of /golem is unknown.` and the help |

The console runs the same command, without the slash, and sees the same lines. A source below the
level reads `Only the console and operators of level 2 and more may run /golem <name>.`. A player
is judged by the lower of their operator level as the game answers it and the level of the source,
and a command that another source runs as a player through `execute as` is judged by the level of
that source alone, as [Plugins](index.md#commands) says.

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
is the repository alone. A commit that ends in `-dirty` names a build whose files differ from that
commit, as the build script of the program golem names it, so the tree of the commit is not its
source and the link is the repository alone as well.

The program golem gives the core plugin the version and the repository of the workspace and the
commit that its `build.rs` takes from the environment variable `GOLEM_COMMIT`, else from
`git rev-parse HEAD`, else names unknown, see [JVM host](../jvm-host.md#golem-run). So a build in a
clone of the repository names its commit without the variable. `Source::of_build()` is a helper
for other hosts of the core plugin: it reads `GOLEM_COMMIT` alone when `golem-plugin` is built, and
without it names the commit unknown. `Source::lines()` answers the five lines above, which
`/golem source` sends. The program golem prints the same version, repository and commit for
`golem source` as three lines of a key and a value, `version <v>`, `repository <url>` and
`commit <hash>`, for scripts to read.

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

## Who runs a command

`/golem whoami` names the sender of [Plugins](index.md#commands) as Golem sees them, one line:

| Sender | Answer |
| --- | --- |
| The console and RCON | `You run /golem as the console.` |
| A player | `You run /golem as the player Tester at the operator level 0.`, and `, from a source at the level 2` before the full stop when the game runs their command at another level than their operator level |
| A player whose operator level the game does not answer | `You run /golem as the player Tester from a source at the level 0.`, which names the level of the source alone |
| Another source | `You run /golem as @ at the level 2.` for a command block, or the name of the entity it runs as through `execute as`, a player included |

A command block keeps the line as its last output, which `data get block <x> <y> <z> LastOutput`
shows, and a function sees no line, since the game hides the answers to a silent source.

## The tools of operators

`/golem player <name>` takes the name or the UUID of a player online, in any case, and answers
six lines from the game:

```text
Tester (f3d28cb0-7225-3cb1-baeb-2dadd2be89ae): minecraft:overworld -21.50 63.00 -0.50, yaw 0.0, pitch 0.0
Game mode survival, ping 0 ms, operator level 4, idle 1.2 s
Display name: Tester
List name: none
Respawn point: minecraft:overworld 10 64 10
Main hand: 3 minecraft:diamond
```

The respawn point is the one that the player holds, which a bed, a respawn anchor or `/spawnpoint`
set, as Minecraft stores it. Minecraft checks its block only when the player respawns, so the line
may name a bed that is gone, and a respawn that finds the block gone or blocked clears the point,
which the line then names `none`.

`/golem world [<dimension>]` answers the shared spawn with its angle and the heights of the blocks,
the lowest y and the y above the highest:

```text
minecraft:the_nether: spawn 4 70 4 at the angle 0.0, blocks from y 0 below y 256
```

`/golem teleport <name> <dimension> <x> <y> <z>` asks the game for the nearest safe place within 3
blocks of the target, `SAFE_RADIUS`, with the search of [Bridge](../bridge.md#the-game), and queues
the teleport there with the rotation the player has, also into another dimension. It answers
`Queued a teleport of Tester to minecraft:the_nether -0.50 100.00 0.50, yaw 0.0, pitch 0.0.`, since
the mod runs the teleport at the end of the tick, where the player may have left or a mod may cancel
it, or `No safe place within 3 blocks of <target>, so Tester stays.` when there is none. It refuses
a coordinate that is no number, and a target outside the bounds of the world that `tp` takes, x and
z from -30000000 to below 30000000 and y from -20000000 to below 20000000. The search loads no
chunk, so a target whose chunks are not loaded answers `The chunks near <target> are not loaded,
so Tester stays, since the search for a safe place loads no chunk.`, and the teleport works once a
player nearby or `/forceload add` loads them. It is the teleport of an operator across dimensions
that never lands a player in a wall, in lava or above the void.

`/golem listname <name> [<text>]` queues the name of the player in the player list, which holds
until the player leaves, or its clearing without a text. One space after the name separates it
from the text, which the tool takes as it is typed, runs of spaces included, so
`/golem listname Tester [AFK]   Tester` shows three spaces. It answers
`The player list names Tester [AFK] Tester.` or `The player list shows the name of Tester again.`

Each tool answers `No player <name> is online.` for a player who is not, names a dimension the
server does not have, and answers its usage for words it cannot read. The tools ask the game on
the server thread alone, so a tool answers `The game gives no answer now, so try again.` when the
game cannot answer, also to a player whose operator level the game does not answer, in place of
the refusal.

## Help

```text
Golem 0.1.0, the subcommands of /golem:
/golem source: the license of Golem and the source of this build
/golem status: the TPS, the MSPT and the time every plugin costs
/golem whoami: who Golem sees run the command, and at which level
/golem player <name>: where a player is and how they play, for operators
/golem world [<dimension>]: the spawn and the heights of a dimension, for operators
/golem teleport <name> <dimension> <x> <y> <z>: a teleport to the nearest safe place, for operators
/golem listname <name> [<text>]: the name of a player in the player list, for operators
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
core plugin: the five lines of the source with a known, an unknown and a dirty commit and of the
build, the help of `/golem` and `/golem help`, the error of an unknown subcommand, the first line
of the status against the statistics of the bridge, and the single line of the log. Its section on
the tools of operators checks `whoami` for every sender, the refusal below level 2 and every
answer and action of `player`, `world`, `teleport` and `listname` against a scripted game. The
notebook `tests/notebooks/07_forge_boot.ipynb` runs them under `golem run` against the real game:
`whoami` for the console, the player, an armor stand and a command block, `player` and `world`
against what the console set, a teleport onto a block of magma in the Nether that lands on the
nearest obsidian, a teleport into solid obsidian that finds no place, a teleport to a chunk that
was never generated, which answers that it is not loaded and generates nothing, and the name in
the player list as the client sees it.

Related: [Plugins](index.md), [Bridge](../bridge.md), [Licensing](../../Documentation/licensing.md).
