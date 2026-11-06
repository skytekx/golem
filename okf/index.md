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
| First milestone | The workspace, the JVM host, the bridge, the plugin runtime and the permissions, in work | [Plan](plan.md#work-plan) |

## Page map

| Page | What it holds |
| --- | --- |
| [Plan](plan.md) | The plan of Golem as the owner set it, with the later rulings |
| [Bridge](bridge.md) | The contract between the bridge mod and the Rust of Golem |
| [Change log](log.md) | What changed, oldest first |
| [Decisions](decisions.md) | Every ruling of the owner |
| [Open items](open-items.md) | What is still open |
