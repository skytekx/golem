---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Decisions
title: Decisions
description: Every ruling of the owner on Golem, with its day, its status and the page or file where it lives.
tags: [decisions, rulings, owner]
timestamp: 2026-11-07
---

# Decisions

This page lists what the owner ruled on Golem and where each ruling lives. The rulings of the plan
itself are on [Plan](plan.md#owner-requirements).

| Day | Ruling | Status | Where |
| --- | --- | --- | --- |
| 2026-11-06 | Golem is licensed under the AGPL 3, written into every file | Built | [Licensing](../Documentation/licensing.md) |
| 2026-11-06 | The AGPL 3 only, not "or later" | Built | [Licensing](../Documentation/licensing.md#1-why-the-agpl-3-only) |
| 2026-11-06 | The license setup follows the folder structure of Aosabi | Built, with the flat `LICENSES/` that REUSE asks for | [Licensing](../Documentation/licensing.md#4-the-license-texts) |
| 2026-11-06 | The copyright holder is Florian Kleber with the address `opensource@skytekx.com` | Built, checked by the license lint | [Licensing](../Documentation/licensing.md#2-the-copyright-holder) |
| 2026-11-06 | The tree follows REUSE as long as it does not conflict with SPDX | Built, `reuse lint` in CI | [Licensing](../Documentation/licensing.md#9-the-lints) |
| 2026-11-06 | Every commit is signed off in the name of the holder, also an agent's | Built. The commit lint checks that a `Signed-off-by` with a name and an address closes the trailers, not whose name it is, so the review checks that it names the holder | [CONTRIBUTING.md](../CONTRIBUTING.md#2-signed-off-by) |
| 2026-11-06 | Commits are authored with the GitHub account of the owner and name the agent as co-author | Built | [CONTRIBUTING.md](../CONTRIBUTING.md#2-signed-off-by) |
| 2026-11-06 | Commits are Conventional Commits as Commitizen writes them, in the form of the Linux kernel | Built, checked in CI | [CONTRIBUTING.md](../CONTRIBUTING.md#3-commit-messages) |
| 2026-11-06 | Test cases are Jupyter notebooks, as in Aosabi | Built | [Testing](../Documentation/testing.md) |
| 2026-11-06 | An agent answers its own blocking questions instead of asking | In force | This page records the answers it gave |
| 2026-11-06 | Agents work on Opus and Fable side by side: every design is written and judged by both models, and every build is checked by the other model before it is merged | In force | This page and [Change log](log.md) |
| 2026-11-06 | Plugins are native Rust on Aosabi, with no TypeScript and no WebAssembly | In force: the design of the plugin API weighs only native Rust plugins, compiled in, as processes over graphtp, or as libraries loaded at runtime | [Plan](plan.md#the-graphql-api) |
| 2026-11-06 | The Golem plugin API is GraphQL on Aosabi: a plugin reads the game with queries, changes it with mutations and hears events through subscriptions, and the same schema serves tools and operators | In work: a design panel decides the architecture | [Plan](plan.md#the-graphql-api) |
| 2026-11-06 | The Golem API covers an equivalent of the whole Bukkit API, Spigot included, so that every Bukkit or Spigot plugin can be converted to a Golem plugin | In work: the surface of spigot-api 1.20.1 is 1,247 types, 7,942 public methods, 5,900 fields and constants and 268 events, and a coverage check will hold the Golem API to it | [Plan](plan.md#bukkit-plugins) |
| 2026-11-06 | Every Golem plugin ported from a Bukkit plugin keeps the license of that plugin | Built: `golem-essentials` and `golem-essentials-spawn` under GPL-3.0-or-later, `golem-permissions` under MIT, checked by rule 9 of the license lint | [Licensing](../Documentation/licensing.md#6-the-ports-and-what-golem-never-carries) |
| 2026-11-06 | The Golem API is called in process through the executor of Aosabi, on the thread of the caller, so a call of the server thread runs on the server thread with no socket and no hop. graphtp over a Unix socket serves the clients outside the process | In force for the design of the API | [Plan](plan.md#the-graphql-api) |
| 2026-11-06 | Bukkit and Spigot plugins also run unchanged, through a Bukkit layer on the Golem API in the JVM of Forge, and every plugin must work: the public API, the internals of CraftBukkit that plugins reach, such as the permissible of `CraftHumanEntity`, the command map and its known commands, which the Golem API offers, and the classes of Minecraft that plugins call by their Spigot names. Golem's own plugins stay native Rust | In work: a survey of the plugins ranks the internals they reach | [Plan](plan.md#bukkit-plugins) |

## Answers of the agents

The agents of the first milestone answered these questions themselves, under the last ruling
above. Each stands until the owner rules otherwise.

| Question | Answer | Where |
| --- | --- | --- |
| How closely does `golem run` follow the java launcher? | Byte for byte, quirks included, so an argument file that `run.sh` accepts starts the same server. A NUL byte in an argument file is refused instead of cutting the argument | [JVM host](jvm-host.md) |
| Which environment reaches the launch? | `JAVA_TOOL_OPTIONS` and `_JAVA_OPTIONS`, which the JVM reads itself. `JDK_JAVA_OPTIONS` and `CLASSPATH` of the launcher are ignored, so the launch is the files of the server alone | [JVM host](jvm-host.md) |
| How does a signal stop the server? | While the server ticks, from its first tick until it stopped, the first signal queues `save-all flush` and `stop`, which the bridge mod runs as the console at the end of the next tick, and a later signal changes nothing, since a second stop would only race the first. A server thread that hangs keeps that stop waiting until `SIGKILL`, as under java, where the shutdown hook of the server waits for the server thread without a bound. No tick drains the queue without the bridge, before the first tick and after the stop, so a signal then ends the JVM as the java launcher does, through `System.exit` with 128 plus the signal, and the shutdown hook of the server saves the worlds once Minecraft added it. The rule that Golem never stops through a signal is about Arclight, whose shutdown thread worked on chunks beside the server thread. The hook of vanilla Forge only lets the server thread stop and save by itself, and without a tick that drains the queue the only other way out is `SIGKILL`, which loses what the server did not save | [JVM host](jvm-host.md#the-stop-path) |
| What does `SIGQUIT` do under `-Xrs`? | It prints the dump of every thread and stops nothing, as under java. `jcmd` attaches without it while the socket `/tmp/.java_pid<pid>` of the JVM exists. Unlike java, a removed socket does not come back until golem restarts, since `-Xrs` takes away the signal thread of HotSpot that would create it again, so a `SIGQUIT` of a tool that finds the socket gone prints no dump and logs that it is gone. A line `x /tmp/.java_pid*` in a file of `/etc/tmpfiles.d` keeps the socket from a cleaner of `/tmp`, and giving the signal back to HotSpot waits for the owner | [JVM host](jvm-host.md#the-stop-path) |
| What does a shutdown signal do that golem started with ignored? | Nothing, as under java, so `nohup` keeps the server up on a hangup | [JVM host](jvm-host.md#the-stop-path) |
| What does a native do when another thread holds the handler? | It waits, since Forge calls `chat()` beside the server thread. Only a call from inside the handler answers the pass value at once | [Plugins](plugins/index.md) |
| How long is the budget of one plugin call? | 1 ms by default, `--budget-ms` changes it | [Plugins](plugins/index.md) |
| May a command run as the console when the mod sends half a sender? | No. A sender with only one of the UUID and the name is refused | [Bridge](bridge.md) |
| Which commit does `/golem source` name? | `GOLEM_COMMIT` at build time, else `git rev-parse HEAD`, else unknown, with the link to the tree of that commit | [Core plugin](plugins/core.md) |
| Where does the version of the bridge jars come from? | `[workspace.package]` of `Cargo.toml`, so Golem has one version | [Bridge](bridge.md) |
| Does a player's own node win over a group's? | Yes. The player's nodes decide before every group, so a player's wildcard deny takes away what a group grants by name, unlike LuckPerms | [Permissions](plugins/permissions.md) |
| How do groups of the same rank disagree? | The deny wins. A wildcard `a.*` matches below `a` and not `a` itself, as in LuckPerms | [Permissions](plugins/permissions.md) |
| What does the seed grant? | The 39 nodes of the live group default to every player, and `essentials.*` and `golem.*` to the group admin, which has no members yet | [Permissions](plugins/permissions.md) |
