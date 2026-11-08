---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Decisions
title: Decisions
description: Every ruling of the owner on Golem, with its day, its status and the page or file where it lives.
tags: [decisions, rulings, owner]
timestamp: 2026-11-08
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
| 2026-11-06 | The Golem plugin API is GraphQL on Aosabi: a plugin reads the game with queries, changes it with mutations and hears events through subscriptions, and the same schema serves tools and operators | In work: the design stands in [Plugin API](api.md), nothing of it is built | [Plan](plan.md#the-graphql-api) |
| 2026-11-06 | The Golem API covers an equivalent of the whole Bukkit API, Spigot included, so that every Bukkit or Spigot plugin can be converted to a Golem plugin | In work: the surface of spigot-api 1.20.1 is 1,247 types, 7,942 public methods, 5,900 fields and constants and 268 events, and a coverage check will hold the Golem API to it | [Plan](plan.md#bukkit-plugins) |
| 2026-11-06 | Every Golem plugin ported from a Bukkit plugin keeps the license of that plugin | Built: `golem-essentials` and `golem-essentials-spawn` under GPL-3.0-or-later, `golem-permissions` under MIT, checked by rule 9 of the license lint | [Licensing](../Documentation/licensing.md#6-the-ports-and-what-golem-never-carries) |
| 2026-11-06 | The Golem API is called in process through the executor of Aosabi, on the thread of the caller, so a call of the server thread runs on the server thread with no socket and no hop. graphtp over a Unix socket serves the clients outside the process | In force for the design of the API | [Plan](plan.md#the-graphql-api) |
| 2026-11-06 | Bukkit and Spigot plugins also run unchanged, through a Bukkit layer on the Golem API in the JVM of Forge, and every plugin must work: the public API, the internals of CraftBukkit that plugins reach, such as the permissible of `CraftHumanEntity`, the command map and its known commands, which the Golem API offers, and the classes of Minecraft that plugins call by their Spigot names. Golem's own plugins stay native Rust | In work: a survey of the plugins ranks the internals they reach | [Plan](plan.md#bukkit-plugins) |
| 2026-11-08 | The dress rehearsal runs on a fresh copy of the live server with an export of LuckPerms, and two or three players play on it on the LAN for an hour, since only players make the load that the comparison of Arclight and Golem needs | In force | [Plan](plan.md#verification), [Features](features.md#the-audits-the-comparison-and-the-rehearsal) |

## Answers of the agents

The agents answer these questions themselves, under the ruling that an agent answers its own
blocking questions instead of asking. Each stands until the owner rules otherwise.

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
| Which commit does `/golem source` name? | `GOLEM_COMMIT` at build time, else HEAD of the git checkout of Golem whose work tree is the workspace, with `-dirty` behind it when the files that make the binary differ from HEAD, else unknown. The link goes to the tree of that commit, and to the repository alone for an unknown or a dirty commit, since no tree of the repository is known to hold that source | [JVM host](jvm-host.md#golem-run), [Core plugin](plugins/core.md#source) |
| Where does the version of the bridge jars come from? | `[workspace.package]` of `Cargo.toml`, so Golem has one version | [Bridge](bridge.md) |
| Does a player's own node win over a group's? | Yes. The player's nodes decide before every group, so a player's wildcard deny takes away what a group grants by name, unlike LuckPerms | [Permissions](plugins/permissions.md) |
| How do groups of the same rank disagree? | The deny wins. A wildcard `a.*` matches below `a` and not `a` itself, as in LuckPerms | [Permissions](plugins/permissions.md) |
| What does the seed grant? | The 39 nodes of the live group default to every player, and `essentials.*` and `golem.*` to the group admin, which has no members yet | [Permissions](plugins/permissions.md) |
| In which order do the two branches of the bundle land, and may they be rebased? | The branch `port/wiring` first, then `port/bridge`, each rebased with its fixups and the unfinished `ca30d08` split in four. Neither branch is pushed, so the rebase rewrites no pushed history | [Features](features.md#the-tooling-ci-lints-and-pages) |
| Where do the ports of EssentialsX live? | In one crate `golem-essentials` for teleport, social, economy and admin, with `golem-essentials-spawn` beside it, both under GPL-3.0-or-later as `ports.toml` holds them | [Features](features.md#the-foundation-of-the-port-of-essentialsx) |
| Which sea level does `/depth` count from? | The value of the generator per dimension, 63 in the Overworld, 32 in the Nether and 0 in the End, until the bridge checks it against the sources of Minecraft 1.20.1 | [Features](features.md#the-bridge) |
| Where do the defaults of a port live? | In its config file under `config/`, with the values of the live server, read as the defaults that the plugin declares | [Features](features.md#the-plugin-runtime) |
| Which number does a player type to read or delete a mail? | The number of the list last shown, which the port maps to a stable id of the mail | [Features](features.md#social-info-and-status) |
| What does a player read when a teleport ends `unloaded`? | A text of the port, since no key of EssentialsX fits | [Features](features.md#teleport-and-spawn) |
| Does a link in a message become a link to click? | No. It stays plain text without a click action, and `ping` echoes plain text | [EssentialsX social port](plugins/essentials/social.md) |
| Do the commands of the ports complete with tab? | Yes, the names of players, homes and warps | [Features](features.md#the-plugin-runtime) |
| How is a balance with a fraction of a cent rounded? | Half to even to whole cents, as #106 says, where #79 reads otherwise | [Features](features.md#the-migration) |
| Does the migration read the H2 database of LuckPerms? | No. It reads the JSON export of `lp export`, and a reader of the H2 file is the fallback. Who runs the export and when waits for the owner | [Features](features.md#the-migration) |
| What happens to a key of a player file that the store has no place for? | The migration drops it by name in its report or fails, and never drops it in silence | [Features](features.md#the-migration) |
| How does the store get into the nightly backup? | `backup.sh` asks Golem for an online snapshot of the store after its check of free space and archives it with `world/`, so only a run that archives takes one. The owner commits that change of the server repository | [Features](features.md#the-store) |
| Are the structure seeds of Spigot ported? | No. They equal the vanilla salts, so Golem checks them on a generated region and sets them only if a structure moves | [Features](features.md#the-world-parity) |
| Does the switch wait for an item of Aosabi? | No. Each item has a fallback that Golem uses until it lands, and the switch path holds no item. Whether the ports run on `golem-plugin` before the API waits for the owner | [Features](features.md#what-golem-asks-of-aosabi) |
| Which release of Aosabi do the first measurements of the API run on? | The tagged release that Golem pins | [Features](features.md#the-graphql-api-of-golem) |
| How does Golem take Aosabi? | By path, from a checkout of a tagged release, with `golem-api` as a workspace of its own until Aosabi builds a guest that is a member of a larger workspace, and a line in `deny.toml` on why it allows the source. BSD-3-Clause joins the allowed licenses only if a feature pulls in rustls | [Features](features.md#the-graphql-api-of-golem) |
| How does a process plugin get its socket? | On stdin, which needs no unsafe code in the parent or the child | [Features](features.md#the-graphql-api-of-golem) |
| Which numbers and ports do the new notebooks take? | 12 to 19 for the switch, 20 to 38 for the API, 40 to 44 for the levels, 50 to 53 for the native code and 60 to 64 for the Bukkit layer. A notebook that boots a server takes the next free port in steps of 22 from 25699, and RCON takes the port plus 1 when a case tests it | [Features](features.md#the-tooling-ci-lints-and-pages) |
| Is #216 the rule of one server per host? | No. #216 locks the packages of the harness with their hashes. One Minecraft server runs per host, kept by a lock per host that every notebook which boots a server takes | [Features](features.md#the-tooling-ci-lints-and-pages) |
| Which machines run what? | A cloud container holds one agent and one server, a macOS test host runs the work on macOS, and the Mac mini serves players and runs no test. The dress rehearsal runs on a copy of the live server on the macOS test host, with players | [Features](features.md#the-audits-the-comparison-and-the-rehearsal) |
| Who changes the server repository? | Agents commit their patches as the rules of that repository say, and the owner pushes them | [Features](features.md#the-server-repository-and-the-day-of-the-switch) |
| How long do the files of Arclight stay on the Mac mini? | 30 days after the watch of the switch | [Features](features.md#the-server-repository-and-the-day-of-the-switch) |
| How many threads tick the levels at the switch? | One, the server thread, as under Arclight, so the comparison before the switch compares like with like | [Features](features.md#the-levels) |
| May one level thread wait for another? | No. A call into another level hops and gets a future, and a blocking hop between two level threads is refused, since two loops that wait for each other never wake | [Features](features.md#the-levels) |
| Does `isSameThread` of the server answer true on a level thread? | No. It keeps naming the server thread, so a mod on a level thread queues its work to the server executor, which runs it in the global work of the tick while the protected levels pause | [Features](features.md#the-levels) |
| Do the `ServerTickEvent` handlers of mods run once per level? | No. They run once per tick of the server, in the global work, so the networks of mods keep their pace | [Features](features.md#the-levels) |
| How do Bukkit plugins without the flag of Folia run once levels have threads of their own? | Under one lock that the global work holds too, with `isPrimaryThread` true while a thread holds it, so their events keep one order | [Features](features.md#the-bukkit-layer) |
| Does a lock per plugin answer the nested calls of the shared handle? | No. The lock is not reentrant, so the trait `Plugin` still needs its own answer for a nested call | [Features](features.md#the-plugin-runtime) |
| Do the watchdog and the degrade window count in the first minutes of a JVM? | No. They count nothing in the first minutes after a boot or after the JVM drops its compiled code | [Features](features.md#the-plugin-runtime) |
| Who stops the config watcher of NightConfig? | Under Arclight a release of skytekx-compat, under Golem the bridge, once the server has started, since configs change only while the JVM is gone | [Features](features.md#the-bridge) |
| Where does a count of the workers of world generation go? | Into the JVM array of `ServerMac.sh` before `-jar`, since a property after `-jar` reaches the program and not the JVM, and only once a recording of exploration shows the need | [Features](features.md#the-server-repository-and-the-day-of-the-switch) |
| Does a lower `KEEP` in `backup.sh` free room? | Not by itself. The prune runs only after a new archive is written, and the guard of free space skips the run before it, so room comes from a deletion by the owner | [Features](features.md#the-server-repository-and-the-day-of-the-switch) |
| Do the threads of Golem share their prepared operations? | Yes. One set serves every thread, with the ids given at enable, and Aosabi keeps a plan per thread | [Features](features.md#what-golem-asks-of-aosabi) |
| Is the budget of a first call measured once or per thread? | Per thread, since Aosabi keeps its plans per thread and each thread pays its own first call | [Features](features.md#the-graphql-api-of-golem) |
| Which stack do the threads that Golem makes get? | 8 MiB, as `golem-jvm` gives its thread | [Features](features.md#the-graphql-api-of-golem) |
| What does a remote subscriber get when its events overflow? | Every remote stream of events, chat and commands included, closes with `EVENTS_LOST` and the count of the lost events, and the streams of state, the tick and the positions keep the newest | [Features](features.md#the-graphql-api-of-golem) |
