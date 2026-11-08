---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Project
title: Golem
description: Knowledge base of Golem, the Rust hybrid server of SkyTekx, with its state, the parts that are built and a map of every page.
tags: [golem, server, rust, forge, overview]
timestamp: 2026-11-08
---

# Golem

Golem is the Rust hybrid server of SkyTekx. It owns the server process, starts the JVM inside
itself and boots Minecraft 1.20.1 with Forge 47.4.18 and every mod unchanged, and it replaces
Arclight. Around the game it adds plugins in Rust, and it will add a GraphQL API and native code
for world generation. This wiki holds the design and the operation of Golem itself. The pages about
the SkyTekx 3 server and its modpack stay in the
[wiki of the server](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/index.md), but
for [Modpack](modpack.md), which answers whether Golem runs that pack.

## State on 2026-11-07

| Area | State | Details |
| --- | --- | --- |
| License | AGPL 3 only, holder Florian Kleber, REUSE compliant, checked in CI. The ports keep the licenses of their upstreams | [Licensing](../Documentation/licensing.md) |
| Commits | Conventional Commits in the form of the kernel, checked in CI | [CONTRIBUTING.md](../CONTRIBUTING.md) |
| Tests | Jupyter notebooks run by papermill, as in Aosabi, the whole matrix in CI | [Testing](../Documentation/testing.md) |
| JVM host | Built: `golem run` boots Forge 47.4.18 unchanged in its embedded JVM and on a signal stops it through the server thread while it ticks and through `System.exit`, as java does, at every other stage, held by `03_launch_args` and `07_forge_boot` | [JVM host](jvm-host.md) |
| Bridge | Built: the host jar, the Forge mod and the 16 natives in Rust. The plugins ask the game on the server thread for the players, where a player is, the respawn point, the spawn, the held item, the heights and a safe place, queue commands, messages, teleports and names of the player list, decide the PermissionAPI of Forge through the handler `golem:permissions`, and judge a command by the permission level of its source. A plugin command named like a command of the game or of a mod takes the plain name, the lines of the natives and the plugins go to the log of the server through the logger `golem`, and a respawn says whether the player left the End. Held by `04_plugins`, `05_bridge_build` and `07_forge_boot` | [Bridge](bridge.md) |
| Plugin runtime | Built with the core plugin and its command `golem`, whose tools of operators read the game, teleport a player to a safe place and name them in the player list, held by `04_plugins` and `07_forge_boot` | [Plugins](plugins/index.md) |
| Permissions | Built as the crate `golem-permissions` with the seed of SkyTekx, held by `06_permissions`. The store holds the groups, nodes and players and hands them to the crate as the same TOML, held by `09_store`. No plugin answers the PermissionAPI with them yet, and `golem run` loads neither | [Permissions](plugins/permissions.md) |
| Store | Built: `golem-store` keeps the data of the plugins in one SQLite file, answers the server thread from its cache and writes on a thread of its own, held by `09_store`. `golem run` does not open it yet | [Store](store.md) |
| Modpack | Checked: `golem run` boots the 289 mods of the pack of the live server as java does and opens the world that Arclight wrote, held by `08_modpack_boot` | [Modpack](modpack.md) |
| Plugin API | Designed, not built: GraphQL on Aosabi, called in process through the executor, an equivalent of the whole Bukkit API that a coverage check holds to `spigot-api` 1.20.1 | [Plugin API](api.md), [API coverage](api-coverage.md) |
| Bukkit layer | Surveyed, not built: a survey of 503 plugins measures the three levels of the layer that runs Bukkit and Spigot plugins unchanged on the Golem API | [Bukkit layer](bukkit-layer.md) |
| Next | The ports of EssentialsX, EssentialsXSpawn and LuckPerms on the store, the store in `golem run`, the first area of the API and the decisions open for the owner | [Open items](open-items.md) |

## Page map

| Page | What it holds |
| --- | --- |
| [Plan](plan.md) | The plan of Golem as the owner set it, with the later rulings |
| [JVM host](jvm-host.md) | `golem run`, the launch as the java launcher reads it, the options Golem adds, the thread, the hooks, the stop path and the host jar |
| [Bridge](bridge.md) | The contract between the bridge mod and the Rust of Golem, and how the bridge is built and tested |
| [Plugins](plugins/index.md) | The plugin runtime: order, cancel chain, panics, budgets, threads, commands and tick statistics |
| [Core plugin](plugins/core.md) | The command `golem`: source, status, who runs a command, help and the tools of operators |
| [Permissions](plugins/permissions.md) | The permissions in place of LuckPerms: the format, the rules of resolution, the seed and the quirks they fix |
| [EssentialsX teleport port](plugins/essentials/teleport.md) | The spec of the teleport side of the port of EssentialsX 2.22.0 and EssentialsXSpawn 2.22.0, command by command, with its messages, config keys, nodes, stores and bridge needs and the quirks and security issues Golem fixes, under the GPL 3 or later of EssentialsX |
| [EssentialsX social port](plugins/essentials/social.md) | The spec of the messages, mail, ignore, info, AFK, suicide, economy and admin commands of the port of EssentialsX 2.22.0, in the same form and under the same license |
| [EssentialsX bridge needs](plugins/essentials/bridge-needs.md) | Every query, action, event and storage need of the port of EssentialsX and EssentialsXSpawn, held against what the bridge offers |
| [Store](store.md) | The SQLite store of the plugins: its design, threads, durability, schema, versions, API, refusals, the rules for the data of EssentialsX, its place in `golem run`, its cost and its tests |
| [Modpack](modpack.md) | Whether `golem run` runs the SkyTekx 3 modpack of the live server, what notebook 08 checks, its numbers and every difference against java and Arclight |
| [Plugin API](api.md) | The design of the GraphQL API of Golem on Aosabi, an equivalent of the whole Bukkit API: the schema, the client through the executor, the contract with the bridge, threads, events, remote clients, coverage, the Bukkit layer and the parts of the work |
| [API coverage](api-coverage.md) | The format of the mapping that holds the API to every member of spigot-api 1.20.1, its statuses, the three numbers and how CI holds them |
| [Bukkit layer](bukkit-layer.md) | The layer that runs Bukkit and Spigot plugins unchanged on the Golem API: what it is, the three levels a survey of 503 plugins measures, the stand-ins of CraftBukkit, the remapper, the needs outside the levels, what each work item completes and the decisions open for the owner |
| [Change log](log.md) | What changed, oldest first |
| [Decisions](decisions.md) | Every ruling of the owner |
| [Open items](open-items.md) | What is still open |
