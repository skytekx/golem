---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Project
title: Golem
description: Knowledge base of Golem, the Rust hybrid server of SkyTekx, with its state, the parts that are built and a map of every page.
tags: [golem, server, rust, forge, overview]
timestamp: 2026-11-06
---

# Golem

Golem is the Rust hybrid server of SkyTekx. It owns the server process, starts the JVM inside
itself and boots Minecraft 1.20.1 with Forge 47.4.18 and every mod unchanged, and it replaces
Arclight. Around the game it adds plugins in Rust, a GraphQL API and native code for world
generation. This wiki holds the design and the operation of Golem itself. The pages about the
SkyTekx 3 server and its modpack stay in the
[wiki of the server](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/index.md).

## State on 2026-11-06

| Area | State | Details |
| --- | --- | --- |
| License | AGPL 3 only, holder Florian Kleber, REUSE compliant, checked in CI | [Licensing](../Documentation/licensing.md) |
| Commits | Conventional Commits in the form of the kernel, checked in CI | [CONTRIBUTING.md](../CONTRIBUTING.md) |
| Tests | Jupyter notebooks run by papermill, as in Aosabi | [Testing](../Documentation/testing.md) |
| JVM host | Built: `golem run` boots Forge 47.4.18 unchanged in its embedded JVM and stops it through the server thread, held by `03_launch_args` and `07_forge_boot` | [JVM host](jvm-host.md) |
| Bridge | Built: the host jar, the Forge mod and the 14 natives in Rust, held by `04_plugins`, `05_bridge_build` and `07_forge_boot` | [Bridge](bridge.md) |
| Plugin runtime | Built with the core plugin and its command `golem`, held by `04_plugins` and `07_forge_boot` | [Plugins](plugins/index.md) |
| Permissions | Built as a crate with the seed of SkyTekx, held by `06_permissions`, not yet wired to a command or the store | [Permissions](plugins/permissions.md) |
| Next | The adversarial review of the first milestone, CI for Rust, Gradle and the boot, then the GraphQL API and the ports | [Open items](open-items.md) |

## Page map

| Page | What it holds |
| --- | --- |
| [Plan](plan.md) | The plan of Golem as the owner set it, with the later rulings |
| [JVM host](jvm-host.md) | `golem run`, the launch as the java launcher reads it, the options Golem adds, the thread, the hooks, the stop path and the host jar |
| [Bridge](bridge.md) | The contract between the bridge mod and the Rust of Golem, and how the bridge is built and tested |
| [Plugins](plugins/index.md) | The plugin runtime: order, cancel chain, panics, budgets, threads, commands and tick statistics |
| [Core plugin](plugins/core.md) | The command `golem`: source, status and help |
| [Permissions](plugins/permissions.md) | The permissions in place of LuckPerms: the format, the rules of resolution, the seed and the quirks they fix |
| [Plugin API](api.md) | The design of the GraphQL API of Golem on Aosabi, an equivalent of the whole Bukkit API: the schema, the client through the executor, the contract with the bridge, threads, events, remote clients, coverage, the Bukkit layer and the work breakdown |
| [API coverage](api-coverage.md) | The format of the mapping that holds the API to every member of spigot-api 1.20.1, its statuses, the three numbers and how CI holds them |
| [Bukkit layer](bukkit-layer.md) | The layer that runs Bukkit and Spigot plugins unchanged on the Golem API: what it is, the three levels a survey of 503 plugins measures, the stand-ins of CraftBukkit, the remapper, the needs outside the levels, the order of work and the decisions open for the owner |
| [Change log](log.md) | What changed, oldest first |
| [Decisions](decisions.md) | Every ruling of the owner |
| [Open items](open-items.md) | What is still open |
