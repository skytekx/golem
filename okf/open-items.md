---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Backlog
title: Open items
description: What is still open on Golem, from the decisions that the owner has still to make to the parts not built yet and what remains open in the parts that are built.
tags: [open-items, backlog, todo]
timestamp: 2026-11-07
---

# Open items

## Open decisions

- Whether `skytekx/golem` stays public until the switch. While it is public and the live server
  runs EssentialsX, an exploitable security issue goes to a private advisory.
- Whether the economy of EssentialsX is in use at all, which the usage audit decides.
- Whether the Boolean nodes that mods ask through the PermissionAPI keep the answers that LuckPerms
  gave through Arclight, a node that nothing names false or true for an operator, or go back to
  the default resolvers of their mods, see
  [Permissions](plugins/permissions.md#arclight-forwards-every-permission-of-a-mod-to-luckperms)
  and issue [#105](https://github.com/skytekx/golem/issues/105). Until the owner records a decision
  in [Decisions](decisions.md), the port of LuckPerms keeps the answers of LuckPerms.
- The four decisions of the Bukkit layer: Paper, Java 21, the order of the injection of the
  permissible, and how `golem-permissions` asks a Bukkit permission plugin, see
  [Bukkit layer](bukkit-layer.md#13-decisions-for-the-owner).

Whether a Bukkit layer is ever wanted is answered by the ruling of 2026-11-06 in
[Decisions](decisions.md): Bukkit and Spigot plugins run unchanged through a Bukkit layer on the
Golem API.

## Not built yet

- The GraphQL API on Aosabi, which [Plugin API](api.md) designs, the Bukkit layer, which
  [Bukkit layer](bukkit-layer.md) surveys, the ports of EssentialsX, EssentialsXSpawn and the
  Spigot world settings, the migration tool and the native acceleration of world generation. See
  [Plan](plan.md#work-plan).
- The usage audit and the sign audit of the Mac mini, and the four way comparison of
  [Plan](plan.md#before-the-port), which need the live server.

## Open in what is built

- `golem run` does not open the store yet. The steps that wire it in, with the close in the exit
  hook of `golem-jvm`, stand in [Store](store.md#the-place-in-the-program-golem).
- `golem-permissions` answers no `on_permission` yet. A port of LuckPerms answers every Boolean
  node of an online player as LuckPerms did through Arclight, see the open decision above, and
  the importer of a LuckPerms export, the members of the group admin and a comparison of the seed
  with an `lp export` of the Mac mini are open.
- `/golem status` does not name the permission handler that Forge took. The warning that the mod
  logs on `ServerStartingEvent` when Forge takes another handler than `golem:permissions` is the
  only sign. Naming it needs an eighth question of `Game` or a native, which changes the contract
  of [Bridge](bridge.md#the-game) and the mapping of [Plugin API](api.md) section 16.
- Actions that a plugin queues on `serverStopping()` or `serverStopped()` never run, since no tick
  follows to drain them. The mod could drain once in its stopping listener, while the players are
  still there.
- The plugin runtime treats `serverStopping()` as the end of every open session, since a player
  still connected at the stop may get no `PlayerLoggedOutEvent`.
- A plugin command named like a vanilla command merges into the vanilla node of Brigadier. The
  port of EssentialsX needs a rule for it.
- Text that a plugin echoes from a player passes formatting codes into `Component.literal`, so the
  ports strip them.
- `GOLEM_COMMIT` in the release build, so that a binary built outside a checkout still names its
  commit.
- The launcher on macOS, where the path of `libjvm.dylib` comes from reading the sources alone.
- Notebook 08 compares the spawn, the datapacks and the folders of the dimensions of the worlds
  under java, golem and Arclight, not the chunks in them, so a change in the generation under
  golem would pass, see [Modpack](modpack.md#what-is-not-checked). A comparison of the content of
  chunks is open.
- The repository of the server holds skytekx-compat 2.1.3 and marks Nostalgic Tweaks as required
  on the server, while the live server runs 2.1.6 without Nostalgic Tweaks. Once the repository
  holds the live pack, the pin of `tests/fetch.py` moves to it, see issue
  [#102](https://github.com/skytekx/golem/issues/102).
- Notebook 08 fails a boot of golem on a call of a plugin over its budget of 1 ms. On a loaded
  machine the core plugin might exceed it with no fault of its own, which would fail the notebook.
