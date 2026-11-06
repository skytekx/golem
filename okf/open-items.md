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
