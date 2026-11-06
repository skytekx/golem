---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Backlog
title: Open items
description: What is still open on Golem, from the open decisions of the plan to the parts not built yet.
tags: [open-items, backlog, todo]
timestamp: 2026-11-06
---

# Open items

## Open decisions of the plan

- Whether `skytekx/golem` stays public until the switch. While it is public and the live server
  runs EssentialsX, an exploitable security issue goes to a private advisory.
- Whether a Bukkit layer is ever wanted for plugins beyond the ported ones.
- Whether the economy of EssentialsX is in use at all, which the usage audit decides.

## Not built yet

- The GraphQL API on Aosabi, the SQLite store, the ports of EssentialsX, EssentialsXSpawn and the
  Spigot world settings, the migration tool and the native acceleration of world generation. See
  [Plan](plan.md#work-plan).
- The usage audit and the sign audit of the Mac mini, and the four way comparison of
  [Plan](plan.md#before-the-port), which need the live server.

## The first milestone

- CI for the Rust workspace, cargo deny, the downloads of `tests/fetch.py` and the notebooks that
  build or boot Golem, with caches for cargo, Gradle and the downloads.
- The findings of the adversarial review of the first milestone, and an issue for every quirk the
  agents recorded, as CONTRIBUTING.md section 6 asks.
- `GOLEM_COMMIT` in the release build, so that a binary built outside a checkout still names its
  commit.
- The plugin runtime treats `serverStopping()` as the end of every open session, since a player
  still connected at the stop may get no `PlayerLoggedOutEvent`.
- A plugin command named like a vanilla command merges into the vanilla node of Brigadier. The
  port of EssentialsX needs a rule for it.
- Text that a plugin echoes from a player passes formatting codes into `Component.literal`, so the
  ports strip them.
- The SQLite store of the permissions, the importer of a LuckPerms export, the handler of the Forge
  `PermissionAPI`, the members of the group admin, and a comparison of the seed with an `lp export`
  of the Mac mini.
- The launcher on macOS, where the path of `libjvm.dylib` comes from reading the sources alone.
