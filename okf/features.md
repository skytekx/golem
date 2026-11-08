---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Backlog
title: Features
description: Every feature of Golem, from the work of the hand over to the switch of the live server, the plugin API, the Bukkit layer, the native code and the items Golem asks of Aosabi, each with its source, its state and its notebook.
tags: [features, backlog, golem]
timestamp: 2026-11-08
---

# Features

This page lists every feature of Golem by what it does, one row each. The rows come from these
sources as they stand on 2026-11-07: the parts of the work of [Plugin API](api.md) sections 20 to
22, the parts of [Bukkit layer](bukkit-layer.md), the specs of the port of EssentialsX,
[Plan](plan.md), [Open items](open-items.md), the open issues of `skytekx/golem` and the items that
Golem asks of Aosabi.

Three of the columns work as follows.

- The column Source names the page and section, the issue, the commit, or the file and line that the
  feature comes from. A path after Golem is a file of this repository at `e4da1d2`. Teleport, Social
  and Bridge needs are the specs [EssentialsX teleport port](plugins/essentials/teleport.md),
  [EssentialsX social port](plugins/essentials/social.md) and [EssentialsX bridge
  needs](plugins/essentials/bridge-needs.md). Permissions, Plugins, Core plugin, Store, Bridge, JVM
  host, Modpack, Plan and Open items are the pages of this wiki of those names, API is [Plugin
  API](api.md) and Layer is [Bukkit layer](bukkit-layer.md). SRV is the server repository
  `skytekx/skytekx3-1201-server`, read at `3fa5ae68` and, where a row says master, at its master
  `1b9fad9b8f3f`. Aosabi is `netsnek/aosabi` at `6e507dc`, its release 0.1.0, and "the branch" is
  its branch `claude/wasm-webgpu-rust-rewrite-8o2855` at `4361ee4`. A commit "on port/wiring" or "on
  port/bridge" lives in the git bundle `handover/port.bundle` and not yet on a pushed branch. This
  page as the source marks a feature that no other page, issue or file names yet, and "a decision
  open with the owner" marks a feature whose default stands until the owner rules.
- The column State holds one of ten values. A Built feature is on the pushed branch at `e4da1d2`, a
  Handover feature on a branch of the bundle, and a Part feature built in part. A Specified feature
  is written in a spec of a port or in [Store](store.md), a Designed one in [Plugin API](api.md),
  and a Surveyed one measured by the survey of [Bukkit layer](bukkit-layer.md). An Open feature has
  no page beyond this one. A Held feature waits for a decision of the owner, and the default of the
  decision stands meanwhile. A None row has nothing to build and says why. An Out feature lands in
  another repository, the server repository or Aosabi.
- The column Notebook names the notebook that holds the row once it lands. The notebooks 01 to 11
  exist, and 12 to 19 are new for the switch: `12_essentials_teleport`, `13_essentials_social`,
  `14_essentials_economy`, `15_essentials_admin`, `16_migration`, `17_macos_host`,
  `18_world_settings` and `19_switch_rehearsal`. The numbers 20 to 38 hold the API, from
  `20_api_contract` to `38_api_layer`, as [Plugin API](api.md) section 20 names them. The numbers 40
  to 44 hold the levels: `40_level_ticks`, `41_level_budgets`, `42_level_loops`, `43_level_handoff`
  and `44_mod_state_audit`. The numbers 50 to 53 hold the native code and 60 to 64 the Bukkit layer.
  A notebook of Aosabi carries the name of its repository.

Every open issue of `skytekx/golem` stands in the column Source of at least one row. Issue #36
appears by its number alone. The closed issues record pushed fixes and need no row.

## Counts

| Area | Built | Handover | Part | Specified | Designed | Surveyed | Open | Held | None | Out | All |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [The JVM host and macOS](#the-jvm-host-and-macos) | 1 | 0 | 1 | 0 | 0 | 0 | 13 | 1 | 0 | 0 | 16 |
| [The bridge](#the-bridge) | 1 | 5 | 1 | 5 | 0 | 0 | 10 | 2 | 0 | 0 | 24 |
| [The plugin runtime](#the-plugin-runtime) | 0 | 1 | 0 | 0 | 1 | 0 | 11 | 0 | 0 | 0 | 13 |
| [The permissions and the port of LuckPerms](#the-permissions-and-the-port-of-luckperms) | 0 | 4 | 2 | 0 | 0 | 0 | 6 | 4 | 0 | 0 | 16 |
| [The store](#the-store) | 0 | 1 | 2 | 0 | 0 | 0 | 5 | 0 | 0 | 0 | 8 |
| [The foundation of the port of EssentialsX](#the-foundation-of-the-port-of-essentialsx) | 0 | 0 | 1 | 7 | 1 | 0 | 0 | 0 | 0 | 0 | 9 |
| [Teleport and spawn](#teleport-and-spawn) | 0 | 0 | 1 | 24 | 0 | 0 | 2 | 2 | 1 | 0 | 30 |
| [Social, info and status](#social-info-and-status) | 0 | 0 | 0 | 19 | 0 | 0 | 0 | 7 | 2 | 0 | 28 |
| [The economy](#the-economy) | 0 | 0 | 0 | 9 | 0 | 0 | 0 | 3 | 0 | 0 | 12 |
| [The commands of operators](#the-commands-of-operators) | 0 | 0 | 0 | 4 | 0 | 0 | 1 | 1 | 1 | 0 | 7 |
| [The migration](#the-migration) | 0 | 0 | 0 | 6 | 0 | 0 | 5 | 1 | 0 | 0 | 12 |
| [The server repository and the day of the switch](#the-server-repository-and-the-day-of-the-switch) | 0 | 0 | 0 | 0 | 1 | 0 | 3 | 1 | 1 | 19 | 25 |
| [The audits, the comparison and the rehearsal](#the-audits-the-comparison-and-the-rehearsal) | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 1 | 0 | 0 | 18 |
| [The world parity](#the-world-parity) | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 2 | 0 | 0 | 19 |
| [The levels](#the-levels) | 0 | 0 | 0 | 0 | 0 | 0 | 16 | 2 | 0 | 0 | 18 |
| [The GraphQL API of Golem](#the-graphql-api-of-golem) | 0 | 0 | 0 | 0 | 20 | 0 | 8 | 1 | 0 | 1 | 30 |
| [The Bukkit layer](#the-bukkit-layer) | 0 | 0 | 0 | 0 | 0 | 25 | 4 | 1 | 0 | 0 | 30 |
| [The native code and what follows](#the-native-code-and-what-follows) | 0 | 0 | 0 | 0 | 1 | 0 | 6 | 1 | 0 | 0 | 8 |
| [What Golem asks of Aosabi](#what-golem-asks-of-aosabi) | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 32 | 32 |
| [The tooling, CI, lints and pages](#the-tooling-ci-lints-and-pages) | 1 | 0 | 3 | 0 | 0 | 0 | 15 | 0 | 1 | 0 | 20 |
| All | 3 | 11 | 11 | 74 | 24 | 25 | 139 | 30 | 6 | 52 | 375 |

## The JVM host and macOS

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| The launch and the stop path: `golem run` boots Forge 47.4.18 unchanged in its embedded JVM as the java launcher reads `unix_args.txt` and stops it through the server thread, and the pull request that names the quirks #4 to #7 closes them | JVM host, issues #4, #5, #6, #7, commits 975ea85, cf5be98, 175ba9f, 3cd89bf, 4d59755 | Built | 03, 07 |
| golem logs its banner as the first line on every host, before the line about a shutdown signal that the process started with ignored, so a launch under `nohup` passes the check of a clean boot | Golem crates/golem/src/main.rs 492 to 504 and 604 to 610, Modpack (what the notebook checks) | Open | 03, 08 |
| The build of `golem` and `golem-devtool` for aarch64-apple-darwin on a macOS test host, with the path `lib/server/libjvm.dylib` of `JavaHome::libjvm` proven by a run | Open items (the launcher on macOS), crates/golem-jvm/src/lib.rs 160 to 167 | Open | 03, 17 |
| A smoke run on a macOS test host: notebook 03 and one boot of stock Forge under `golem run`, every finding filed as an issue | This page | Open | 03, 07 |
| Notebooks 04 to 09 on macOS: the `.so` builds of 05 and 06 as `.dylib`, the cells of 07 on the attach socket `/tmp/.java_pid`, and the reads of `/proc` in 08 and in the devtool | Notebooks 05, 06, 07, 08, crates/golem-devtool/src/store.rs | Open | 04 to 09, 17 |
| The live pack boots under `golem run` on a macOS test host with the heap and the flags of the Mac mini, with the time to `Done` and the resident memory recorded against the 32 to 35 s and the 10.1 to 10.7 GB of Arclight | SRV okf/server/macos.md 86 to 93 and 216 to 224 | Open | 17 |
| The signature of the binary, the ad hoc one of the linker or one made for it, checked by `codesign -v`, and the `libjvm.dylib` of Temurin loaded under it, or the entitlements it needs recorded | This page | Open | 17 |
| The lowest version of macOS that the binary names as its minimum, at or below the version of the live host, checked beside the signature | This page | Open | 17 |
| A stop that hangs escalates: a second signal, or a bound below the `ExitTimeOut` of 180 s of the plist, ends the JVM through `System.exit` after the save it can still make | JVM host (the stop path), SRV macos/com.skytekx.server.plist 45 to 46 | Open | 07, 17 |
| The attach socket under `-Xrs` kept from a cleaner by age: golem detects a removed socket and says so today, and a touch of the socket once a day on Linux and on macOS keeps it, while `-Xrs` stays | Issue #175, commits fe9ad9e and 7b2a72a, Golem crates/golem/src/main.rs 365 to 410 | Part | 07, 17 |
| The commit of a release build made outside a checkout, set through `GOLEM_COMMIT`, so `/golem source` on the Mac mini names it | Open items (GOLEM_COMMIT) | Open | 03 |
| A release artifact for the Mac mini, built and signed on a macOS test host, so the Mac mini needs no toolchain and no room for a build | SRV okf/log.md 717 to 718 | Open | 17 |
| A macOS job in CI on a hosted arm64 runner for the notebooks that boot no pack | Golem `.github/workflows/ci.yml` 59, 86, 131, 176 and 210, every job on `ubuntu-latest`, a decision open with the owner | Open | CI, 11 |
| The option `--budget-ms` takes a fraction of a millisecond, as the devtool does | Golem crates/golem/src/main.rs 132 to 136, crates/golem-devtool/src/bridge.rs 140 | Open | 03, 04 |
| A unit of systemd for a Linux host beside the job of launchd | Plan (work plan, JVM host) | Open | None |
| The count of workers for world generation, `-Dmax.bg.threads`, set by the launcher of Golem from a setting whose default leaves the value of vanilla, the processors less one, once a recording of exploration shows the need | SRV ServerMac.sh 28 to 42, vanilla 1.20.1 `Util` | Held | 03, 17 |

## The bridge

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| The result of a teleport: an id, a safe radius of 0 to 8, chunks held by a ticket, and the events `teleported` and `teleportFailed` with eight reasons, with the three reasons that 05 does not assert yet added | Bridge needs (need 1), issues #28, #39, #43, #44, #63, #65, commit 35dc669 on port/bridge | Handover | 04, 05 |
| Styled text built in Rust and sent as a component, and a broadcast that skips hidden players | Bridge needs (need 2), issues #35, #64, #76, commit 699bb0b on port/bridge | Handover | 04, 05 |
| The small queries: the respawn point with its angle and forced flag, the respawn place, the inventory, flight, the dimensions with sea level and day time, and the most players, split from one unfinished commit into four, with the natives counted 18 | Bridge needs (need 4), issues #28, #86, #91, commit ca30d08 on port/bridge | Handover | 04, 05, 07 |
| The game still answers during `playerLeft`, with the quit position | Bridge needs (need 5), issues #55, #89, commit ca30d08 on port/bridge | Handover | 05, 07 |
| The block of notebook 05 set where its chunk is loaded | Commit c80d6b6 on port/bridge | Handover | 05 |
| The tools of operators move onto the teleport ids, so `/golem teleport` reports the end of the teleport in place of whether it is queued | HANDOVER.md section 3, crates/golem-plugin/src/operator.rs on port/bridge | Open | 07 |
| The sea level that the bridge answers per dimension checked against the sources of Minecraft 1.20.1 | Commit ca30d08 on port/bridge (GameQueries.java 181 to 182), Social (depth) | Open | 05, 07 |
| Chat to a subset of players: a chat verdict with a filter of recipients, so ignore works and the signature of the message stays | Bridge needs (need 6), issue #72, Social Q31 | Specified | 05, 07 |
| A kill with a damage type through the damage path of vanilla | Bridge needs (need 6), issue #94 | Specified | 05, 07 |
| A requirement per plugin command that asks `golem-permissions`, and the command tree sent again when the permissions of a player change | Bridge needs (need 3), issue #96 | Specified | 05, 07 |
| Kick and ban as actions with a reason and a source, the ban entry and the whitelist flag as queries, and suggestions per command | Bridge needs (need 9), issues #31, #92, #93, #95 | Specified | 05, 07 |
| Actions that set the game mode and kill with the notices that vanilla sends to operators, since the action `command` tells no operator what a command answers | Issue #93, commit a79fb59 | Open | 05, 07 |
| A cancellable attack event, for the 4 s of immunity after a teleport or a respawn and the 5 s after a login | Bridge needs (need 8), issue #61, Teleport lines 414 to 428 | Specified | 05, 07 |
| Events for an automatic AFK: interact, attack, bow, fishing and every command of a player | Bridge needs (need 7), issue #87 | Held | 05 |
| The permission handler that Forge takes, named by `/golem status` | Issue #141, commit 368ab11 | Part | 05, 07 |
| A plugin command takes the name of a command of the game or of a mod, as for vanilla | Issue #186, commit aa75076 | Built | 05 |
| Actions that a plugin queues on `serverStopping` or `serverStopped` run before the JVM ends, drained once by the stopping listener of the mod | Open items (actions on serverStopping) | Open | 04, 07 |
| Text that a player typed stays plain when a plugin echoes it, with its formatting codes stripped | Open items (echoed text) | Open | 05, 13 |
| The lines of Golem keep their level in the log, so a panic of a plugin reads as an error | Open items (the level INFO) | Open | 05, 07 |
| A mod that registers a command after the bridge no longer merges into a plugin command of that name, and the redirects deeper in the tree follow a command that the bridge moves aside | Open items (a mod that registers a command) | Open | 07 |
| Passengers dismounted and ejected and a found place centred at a teleport, as Teleport 8.2 asks, and, as a need still to be confirmed, a teleport without a radius kept inside the world border | Teleport 8.2 and the key `teleport-passenger-dismount` | Open | 05, 07 |
| A check of the exact place, so a stand on a slab, a chest, farmland or a path passes, as a mode of the safe search | Issue #47, Teleport Q12 | Open | 05, 07 |
| A login verdict event for the texts of a ban and of a full server | Social (listeners), a decision open with the owner | Held | 05 |
| The bridge stops the config watcher of NightConfig once the server has started, since configs change only with the JVM gone and the watcher polls with a park of 1 µs | SRV AGENTS.md 42 to 46, NightConfig 3.6.4 `FileWatcher` and its constant `SLEEP_TIME_NANOS` | Open | 07 |

## The plugin runtime

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| The store and the permissions in the context of every plugin, and the flush on `ServerStopped` | Commits f26d511 and eb21a7e on port/wiring | Handover | 04, 07 |
| The tools of operators of `/golem` stay with every operator of level 2 outside the group admin, as the pages say | Core plugin, crates/golem-plugin/src/operator.rs 69 to 76 on port/wiring | Open | 04, 07 |
| A plugin declares its default nodes, which the resolver reads below the nodes of a player, the four true nodes of the teleport side among them | Bridge needs 135 to 141, API 11.1, crates/golem-permissions/src/lib.rs 84 to 85 | Open | 04, 06 |
| A second plugin of one name refused by `Runtime::add`, since the ledger and the quarantine of API 11.3 and 11.4 key by the name | Golem crates/golem-plugin/src/runtime.rs 244 to 266 | Open | 04 |
| A registry of plugins outside `main.rs` that enables and disables them | Golem crates/golem/src/main.rs | Open | 04 |
| A command spec with aliases, a description, a usage and a node, read by help and by the command tree | Golem crates/golem-plugin/src/lib.rs 66 to 72, issue #32, API 9.8 | Open | 04 |
| Tab completion of the names of players, homes and warps for the commands of the ports | Decisions (answers of the agents) | Open | 07, 12 |
| The trait `Plugin` meets the shared handle of API 11.1 and 7.4, which nests calls and so, as those sections read, needs `&self`, while it takes `&mut self` behind one lock today | Golem crates/golem-plugin/src/lib.rs 211 to 237 | Open | 04 |
| A config file per plugin under `config/<plugin>.toml`, read at install, with the values of the live server as its defaults | Plugins | Open | 04 |
| A warm start: the game queries and the plans of a plugin built at install, so its first call stays inside the budget of 1 ms, which three tools of operators exceed at up to 6.546 ms | Open items (the first call) | Open | 07, 08 |
| The report of the budget per command and a deadline of 200 ms for a command of a port | API 11.1 | Designed | 04 |
| Sessions still open at the stop end at `serverStopping`, and the ports write the quit position and the last time seen there | Open items (serverStopping ends every session) | Open | 04, 13 |
| A warm up rule: no degrade or quarantine count in the first minutes after a boot or after the JVM drops its compiled code, which a first start of a recording can cause | API 11.4 and 22.5 | Open | 04 |

## The permissions and the port of LuckPerms

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| The store in `golem run`: opened, seeded, flushed and closed on every stop path, with exit 1 on a close that fails | Commits eab8cd7 and c1c1015 on port/wiring, issues #1, #2 | Handover | 03, 07, 08 |
| Every sender checked for a node: the console holds every node, another source from level 2, a player through `golem-permissions` | Commit eb21a7e on port/wiring, issue #1 | Handover | 04, 07 |
| The PermissionAPI of Forge answers every Boolean node of a mod as LuckPerms answered it through Arclight | Commits 2b8ec15 and 9b5749a on port/wiring, issue #105 | Handover | 04, 06, 07 |
| The commands of LuckPerms 5.5.71: user, group, creategroup, deletegroup, listgroups and 15 subcommands, with `/perm` as an alias | Commits 063b980 and 9b5749a on port/wiring, Plan (LuckPerms 5.5.71) | Handover | 06, 07 |
| The cases the bundle lacks: setweight, parent set and clear, permission clear, clear, creategroup of a group that exists, an unknown subcommand, a second page, the aliases g, t and i, UUID versions 3 and 2, `/lp` in game, the level map, the PermissionAPI for an operator and a node that nothing names, a refused write with its log line, the flush on stop, the bridge under `golem run` and a close that fails | Permissions, the branch port/wiring of the bundle | Open | 04, 06, 07 |
| The two quirks of LuckPerms filed as issues before the bundle lands: `/lp deletegroup` leaves `group.<name>` on its members without `--update-parent-lists`, and `commands-allow-op` | Open items on port/wiring | Open | 06 |
| A group without a weight ties with `default`, kept and documented | Issue #206, commits e12b336 and d340aec | Part | 06 |
| A source that is no player needs level 4 for the permission commands and for `/golem` and level 2 for the rest, so a command block cannot grant the group admin | Open items on port/wiring, a decision open with the owner | Held | 06, 07 |
| The members of the group admin after the switch | Open items (the members of the group admin) | Held | 06 |
| The PermissionAPI compared with the golden values of the answers that LuckPerms gives through Arclight, read on a test copy | Issue #105, Permissions (Arclight forwards every permission of a mod to LuckPerms) | Open | 06, 07 |
| The paths of `/lp` that are not ported yet and that the usage audit shows in use | Permissions | Held | 06 |
| A refusal of `/lp editor` that names the commands to use | SRV okf/modpack/plugins.md 70 to 72, a decision open with the owner | Held | 06 |
| A changed set of `/lp` loaded off the server thread | Open items on port/wiring | Open | 04 |
| An operator entry of level 0 and the offline flag of `getOfflinePermission` answered and recorded | Open items on port/wiring | Open | 04 |
| The cost of a check of the PermissionAPI measured, the hottest node first | API 16 | Open | 07 |
| The quirks #1, #2 and #3 closed by the pull request of the port that names them, with #3 read again against the tag 5.5.71 | Issues #1, #2, #3, commit 50be86c | Part | 06 |

## The store

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| Notebook 09 repaired for the field `hasPermission` and run | Commit 8e3e405 on port/wiring | Handover | 09 |
| Schema 2 with its migration from version 1: the toggles, the last teleport time, mail expiry and the legacy flag, the last owner of a warp, past names and settings per player | Store (versions), Bridge needs (storage), Social (storage) | Open | 09 |
| Mail in the store: the shown mails marked read, a stable id per mail, and expired mails dropped before the numbering | Issues #73, #74 | Open | 09 |
| A change never holds the server thread: at `MAX_PENDING` it is deferred or dropped with a warning, where a burst waits up to 274 ms in debug and 151 ms in release today | Store lines 634 to 644, API 19 | Open | 09 |
| Balances in whole cents and never below zero, one transaction per transfer and per bulk change | Issues #79, #80, #81, #106, commit 041569a | Part | 09 |
| Names: one rule per kind, a warp name that reads as a page refused when it is set, and the store keeps the bytes | Issues #35, #46, #53, #60, #107 | Part | 09 |
| An online snapshot of the store that opens, taken by Golem when `backup.sh` asks for it after its check of free space, since the store holds an exclusive lock while Golem runs | Store lines 108 and 126 to 128, SRV macos/backup.sh 53 to 66 and 76 | Open | 09 |
| A warning when the disk runs low, before a full disk stops the writer of the store | Store lines 131 to 140, SRV okf/log.md 717 to 718 | Open | 09, 17 |

## The foundation of the port of EssentialsX

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| The crates `golem-essentials` and `golem-essentials-spawn`, which hold their license and their upstream alone today, get the modules dispatch, text, matching, config, teleport, social, economy and admin | The file ports.toml, crates/golem-essentials/src/lib.rs, Decisions (2026-11-06, a port keeps the license of its plugin) | Part | 12, 13 |
| The dispatcher: the node `essentials.<name>` of the command, `onlyPlayers`, the usage on a missing argument, `errorWithMessage`, and costs charged by the teleport | Teleport 3.1, Social (dispatch) | Specified | 12, 13 |
| Usage and help texts shipped with every description, so no Java error reaches a player | Issue #32 | Specified | 12 |
| The templates of MiniMessage parsed once into `golem_bridge::Text`, the two colours, placeholders escaped, numbers without grouping, and the place shown where a text promises one | Teleport 1.2 and 3.2, issues #35, #64, #77 | Specified | 12, 13 |
| Players matched by exact name or UUID, online through the game and offline through the store, and never by a substring | Teleport, Plugins, Store | Specified | 12 |
| The four true defaults of the teleport side and the mapping of dimension ids in nodes such as `minecraft.the_nether` | Teleport 2.2 and 8.5 | Specified | 06, 12 |
| The config of the port from the values of the live `config.yml`, with a negative or unparsable cost refused at load | Teleport 3.1, issue #34 | Specified | 12 |
| A broadcast of `afk` and of `suicide` at most once per interval per player, beside the limit of chat commands that vanilla keeps | Social Q5 | Specified | 13 |
| The ports land on the trait of `golem-plugin` for the switch, with their calls of the game behind one interface, and move onto `golem-client` through the shim of the plugin runtime of the API | API 11, 16 and 21 item 7, a decision open with the owner | Designed | 12 to 15, 27 |

## Teleport and spawn

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| The queue of requests keyed by UUID: expiry on the tick, which tells the requester, a drop on the quit of either side, and the oldest dropped at `tpa-max-requests` | Teleport 3.6, issues #37, #55 | Specified | 12 |
| The command `tpa` refuses a request to oneself with a reason | Issue #40 | Specified | 12 |
| The command `tpahere` names the dimension of the requester, and an accept moves the player to the live position after the safety search | Issue #33 | Specified | 12 |
| The command `tpaccept`: confirmation after every check, the message to the player who moves, the cost, cooldown and warmup of the mover, the world rule of the mover, and a refused request removed | Issues #38, #39, #54, #56, #70 | Specified | 12 |
| The command `tpdeny` removes every request it denies, so `/tpdeny *` never loops on the server thread | Issue #41 | Specified | 12 |
| The command `tpacancel` says when there is no request and counts only valid requests | Issue #42 | Specified | 12 |
| The command `tpauto` with its flag in the store, and exactly one player for the form with others | Issue #57, Teleport 4.6 | Specified | 12 |
| The command `tptoggle` with its flag in the store | Teleport 4.7 | Specified | 12 |
| One teleport path: an id, the safe radius, chunks by ticket, the text of success and the charge after the end event, the bypass type per teleport, and cooldown and warmup on the tick, checked before and at the end | Teleport 3.5, issues #30, #62, #63, #65, #69 | Specified | 12 |
| The safety search in the mod with the checks of vanilla, the real height range, a bounded box and a clear refusal | Issue #28, Teleport 3.5, SafeLocation.java of the bridge | Part | 05, 12 |
| The command `back`: the last location from the `from` of `teleported` after success, the message after success, and a death location kept with its dimension | Issues #43, #44, #50 | Specified | 12 |
| The command `back` writes no flight and no speed on a change of dimension | Issue #67 | Specified | 12 |
| A death by `suicide` sets no back point | Issue #97 | Specified | 12, 13 |
| The command `home`: an unknown or broken name is an error that names it, and a bare `/home` goes to the home named home | Issues #29, #52 | Specified | 12 |
| The command `home` without a home goes to the bed and then to the point of `/spawn` | Issue #45 | Specified | 12 |
| No home addressed by number, and the list of homes in one order | Issue #48 | Specified | 12 |
| The command `sethome`: one name rule, the limit of the owner, an error in place of a silent rename, and a home on a slab or a chest allowed through the check of the exact place in the safe search | Issues #46, #47, #51 | Specified | 12 |
| The command `delhome` asks for a confirmation for `*` | Issue #49 | Specified | 12 |
| The command `warp`: pages from 1, errors to the sender, and names as literal text | Issues #68, #35 | Specified | 12 |
| The command `setwarp` refuses a name that reads as a page | Issue #53 | Specified | 12 |
| The command `delwarp` | Teleport 5.7 | Specified | 12 |
| The command `spawn` and its alias `espawn`: the missing dimension named, and a group as one segment of a node | Issues #58, #60, Teleport 6.1, Plan (EssentialsXSpawn 2.22.0) | Specified | 12 |
| The command `setspawn` in the Overworld sets the world spawn too | Issue #59, Teleport 6.2 | Specified | 12 |
| The listeners of respawn and of first join stay off as on the live server, so respawns and first joins stay vanilla | Teleport 6.3, Plan (EssentialsXSpawn 2.22.0) | Specified | 12 |
| The compass that pointed at the first home is dropped, so it points at the world spawn as in vanilla | Issue #66, Teleport Q32 | None | 12 |
| No combat tag, while the 4 s of immunity after a teleport or a respawn and the 5 s after a login stay through the attack event | Issue #61, Teleport lines 414 to 428, a decision open with the owner | Specified | 12 |
| A text of the port for a teleport that waited 600 ticks and ended `unloaded`, since no key of EssentialsX fits | Bridge (the end of a teleport) on port/bridge | Open | 12 |
| A teleport of Waystones sets no back point, as on the live server with `register-back-in-listener: false`, and a test copy shows whether the 4 s of immunity follow it | Teleport line 93, SRV okf/modpack/client.md 63 | Open | 12 |
| The command `tpr`, ported only if the usage audit shows use | Plan (EssentialsX 2.22.0) | Held | 12 |
| The held rows S1, S2 and S8 of the teleport spec and the issue with the label security, their content withheld | Teleport 7.1, issue #36 | Held | 12 |

## Social, info and status

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| The command `msg` echoes as usual and delivers nothing to a player who ignores the sender | Issue #71 | Specified | 13 |
| The command `r` with its reply target and the timeout of 180 s of the live config | Social (r) | Specified | 13 |
| A dot in `msg`, `mail` and `me` stays as typed, and a link stays plain text | Issue #76 | Specified | 13 |
| The command `mail`: stable ids, only the shown mails marked read, expired mails dropped before the numbering, `sendtemp` through the expiry column, at most 1000 characters, and the number a player types mapped from the last list shown | Issues #71, #73, #74 | Specified | 13 |
| The command `ignore` with its list in the store | Social (ignore) | Specified | 13 |
| The command `me` skips players who ignore the sender and reveals no hidden sender | Issue #72 | Specified | 13 |
| The command `helpop` stays silent under a mute, with a limit per sender | Issue #75 | Specified | 13 |
| The command `list` grouped by the primary group, with the most players | Issue #91 | Specified | 13 |
| The command `seen` with one rule for past names, the quit position, and no stored address | Issue #89, a decision open with the owner | Specified | 13 |
| The command `realname` | Social (realname) | Specified | 13 |
| The commands `motd` and `rules` show texts the port ships, never a file error, with the keywords of the live texts | Issue #90 | Specified | 13 |
| The command `help` lists the Golem commands a player may use, and the command tree hides the rest | Issue #96 | Specified | 13 |
| The command `ping` echoes plain text | Social (ping), issue #76 | Specified | 13 |
| The command `getpos` prints plain integers in every locale | Issue #77 | Specified | 13 |
| The command `compass` with its bearing | Social (compass) | Specified | 13 |
| The command `depth` against the sea level of the dimension | Social (depth) | Specified | 13 |
| The command `afk` by hand alone: no automatic AFK, and the message shows for players allowed to set one | Issues #87, #88 | Specified | 13 |
| The command `suicide` kills through the damage path of vanilla, which a protection can cancel, with a limit per player | Issue #94 | Specified | 13 |
| The rule of EssentialsX for the milk bucket is not ported | Issue #98 | None | 13 |
| The join flow: the MOTD, the notice of mail once per cooldown, the name and past names, and the time of login | Social (motd) | Specified | 13 |
| Operators and AFK players count for sleeping as on the live server, so nothing is built | Social Q37 | None | 13 |
| The command `msgtoggle`, ported only where the usage audit shows use, else dropped with its fields named in the report of the migration | Social (dependent commands), a decision open with the owner | Held | 13 |
| The command `rtoggle`, under the same rule | Social (dependent commands), a decision open with the owner | Held | 13 |
| The command `socialspy`, under the same rule | Social (dependent commands), a decision open with the owner | Held | 13 |
| The command `mute`, under the same rule | Social (dependent commands), a decision open with the owner | Held | 13 |
| The command `nick`, under the same rule | Social (dependent commands), a decision open with the owner | Held | 13 |
| The command `vanish`, under the same rule | Social (dependent commands), a decision open with the owner | Held | 13 |
| The held rows Q1, Q6 and Q23 of the social spec, their content withheld | Social (quirks and security) | Held | 13 |

## The economy

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| The whole economy, ported only if the usage audit shows a player using `pay`, `balance` or `eco`, else dropped with the balances migrated for the record | Open items (whether the economy is in use) | Held | 14 |
| The core of the money in whole cents: the most money, no balance below zero and no loan, so loans end with the port | Social (economy), Store lines 511 to 514 | Specified | 14 |
| Amounts as plain decimals with at most the scale of the currency, without grouping and without a suffix such as `k` | Issue #78 | Specified | 14 |
| No payment below one cent | Issue #79 | Specified | 14 |
| The command `balance`, which names the missing node for the balance of another player | Issue #85 | Specified | 14 |
| The command `balancetop` | Social (balancetop) | Specified | 14 |
| The command `pay` in one transaction, with 0 and oneself refused | Issues #81, #83 | Specified | 14 |
| The command `payconfirmtoggle`, with amounts compared by value | Issue #84 | Specified | 14 |
| The command `eco`: `take` never makes a loan, a percentage works or is refused, and 0 is refused | Issues #80, #82, #83 | Specified | 14 |
| The command `worth` keyed by registry id over the whole inventory, with every name that maps to nothing reported | Issue #86 | Specified | 14 |
| The command `paytoggle`, ported only where the usage audit shows use | Social (dependent commands), a decision open with the owner | Held | 14 |
| The signs `[Buy]`, `[Sell]` and `[Balance]`, ported only if the sign audit shows them in use, since every sign of EssentialsX is off in the live config | Plan (EssentialsX 2.22.0, signs), Bridge needs (signs) | Held | 14 |

## The commands of operators

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| The command `gamemode` and its forms `gmc`, `gms`, `gma` and `gmsp`, for oneself and for another player, with the notices of vanilla to operators | Issue #93, Plan (EssentialsX 2.22.0, admin) | Specified | 15 |
| The command `kick` with a plain source, where only a typed `\n` breaks a line of the reason | Issue #95 | Specified | 15 |
| The command `ban` with a known player, a plain source and the notice | Issues #92, #95 | Specified | 15 |
| The names of operators are not dark red in texts, and no colour goes into stored data | Issue #31 | Specified | 15 |
| Every command of operators that the usage audit finds and no spec covers, ported or recorded as an intended difference | Plan (EssentialsX 2.22.0, admin) | Open | 15, 19 |
| The kits of `kits.yml` are not ported, since no group may use them | Plan (EssentialsX 2.22.0) | None | None |
| The command `togglejail`, ported only where the usage audit shows use | Social (dependent commands), a decision open with the owner | Held | 15 |

## The migration

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| The tool `golem migrate` reads `userdata/*.yml` into players, homes, back and quit locations, accounts, mail, ignore lists and nicknames, prints counts per kind and fails on any entry it cannot place | Plan (data migration) | Specified | 16 |
| The discipline of a run: the server stopped, the same result twice, a store that holds data refused unless told to replace it, and a report kept beside the copy | Plan (data migration) | Open | 16 |
| Balances rounded half to even to whole cents, a balance below zero refused and named with its debt, and a report of every account that changed | Issues #106, #79, commits 35379c3, 6c89e40, b7098e1 | Specified | 16 |
| Names that EssentialsX folded kept as they are, and a warp name outside the charset or one that reads as a page put on a list to rename | Issues #107, #35, #46, #53 | Specified | 16 |
| The folder `warps/` and the spawn of EssentialsXSpawn | Plan (data migration), Teleport line 1526 | Specified | 16 |
| The files `worth.yml`, `motd.txt` and the rules text into the config and text files of Golem, with every name of `worth.yml` that maps to nothing reported | Issue #86, Plan (data migration) | Specified | 16 |
| The export of LuckPerms into groups, nodes and player values, a node with a context, an expiry or meta refused, and the result compared with the seed and with `lp export` | Issues #2, #3, Permissions lines 248 to 259 | Specified | 16 |
| Every key of a player file placed or dropped by name in the report, with `usermap.bin` and `uuids.bin` covered by the table of players | Social (storage keys) | Open | 16 |
| A source for the first join, the time of the first login or the player file of the world, with the source recorded | Social (storage) | Open | 16 |
| The world map of Arclight: each Bukkit world name of a stored location turned into its dimension id | Teleport 8.4 | Open | 16 |
| A reverse export of homes, warps, balances and mail into the files of EssentialsX for a rollback | A decision open with the owner | Held | 16 |
| A dry run on a copy of the data of the Mac mini, compared with `compare.rs` of the store | Store (compare.rs) | Open | 16, 19 |

## The server repository and the day of the switch

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| The file `.gitignore` of the server repository names `golem-store.db` with its `-wal` and `-shm` | SRV .gitignore 1 to 65 | Out | None |
| The sync to the Mac mini excludes the store, its log and `plugins/LuckPerms/`, and the checkout holds the bridge jar in `mods/`, so `--delete` keeps it | SRV macos/sync-to-mac.sh 71 to 74 | Out | None |
| A place in the server repository for jars that the server alone loads, holding the compat of the server and the bridge jar, which the deploy puts into `mods/` of the Mac mini and of the checkout, while the pack keeps skytekx-compat 2.1.3 as the server repository records it, and Nostalgic Tweaks out of the server as the live server runs it | Issue #102, SRV okf/content/compat-mod.md 29 to 32 and macos/sync-to-mac.sh 72 to 78 at master | Out | 08 |
| The file `ticex-common.toml`, which Forge corrects at every start, settled by a rule of `tools/patch_mods.py` or upstream | Issue #104, commit f7c0a48 | Out | 08 |
| The command `skytekx3ctl stop` waits on a Golem process, and `status` adds `golem status` | SRV macos/skytekx3ctl 23 to 42 | Out | 17 |
| The job of launchd keeps its label `com.skytekx.server` and runs a start script that finds Java 17 with `/usr/libexec/java_home -F -v 17` and runs `golem run` by `exec`, so golem is the process of the job and gets its `SIGTERM`, with standard out in a log file so a dump of `SIGQUIT` survives, and `KeepAlive`, `ExitTimeOut` 180, `ProcessType` Interactive and the limit of 65,536 files kept | SRV macos/com.skytekx.server.plist 8 to 64, SRV ServerMac.sh 10 and 44 to 82 at master | Out | 17 |
| The file `user_jvm_args.txt` with the heap and the flags of the JVM array of `ServerMac.sh` 28 to 42 without `-jar`, headless included, beside a plain Forge 47.4.18 install with its `unix_args.txt`, which the owner installs before the day of the switch if none sits beside Arclight | SRV ServerMac.sh 10 to 42 at master, Golem crates/golem-jvm/src/launch.rs 550 to 559 | Out | 17 |
| The script `install.sh` checks for a Forge 47.4.18 install with its `unix_args.txt` and installs the binary of golem, the host jar and the bridge jar as it installs the others | SRV macos/install.sh, Golem crates/golem-jvm/src/launch.rs 485 to 510 | Out | 17 |
| The script `backup.sh` asks Golem for the snapshot of the store after its check of free space and archives it with `world/` | SRV macos/backup.sh 53 to 66 and 76 | Out | 17 |
| The handler `golem:permissions` in `defaultconfigs/forge-server.toml` and in the `forge-server.toml` of the world, each edited with the JVM gone | SRV defaultconfigs/forge-server.toml 17, Bridge (permissions) | Out | 19 |
| The script of the switch and its runbook with the steps of the day of the switch, where the owner runs the steps that need sudo | SRV macos/skytekx3ctl 19 to 38 | Out | 19 |
| The runbook of the rollback: the old program of the job, the handler set back with the JVM gone, the reverse export, and the two commands of `/datapack` with nobody online | SRV okf/content/boot-fixes.md 93 to 111 | Out | 19 |
| The fixes pack made the last datapack again with nobody online after the first boot, since `mod:golem` takes the place of `mod:arclight` | SRV okf/content/boot-fixes.md 93 to 111, Modpack lines 200 to 201 | Open | 19 |
| The watch after the switch: a daily report of the run count of launchd, the counts of ERROR and FATAL against the baseline, `crash-reports/`, TPS, resident memory, the store and its backup | This page | Open | None |
| Arclight, its jar, `plugins/` and `ServerMac.sh` kept on the Mac mini for 30 days after the watch | Decisions (answers of the agents) | Open | None |
| The server wiki links the Golem wiki, holds the new launcher, and announces before the switch the behaviour that the quirk fixes change | This page | Out | None |
| Room freed by the owner on the disk of the Mac mini before pregeneration and the next backups, since the log of the server records it 98 percent full with 4.7 GB free on 2026-11-04 and the nightly backup needs the newest archive plus 4 GiB free | SRV okf/log.md 717 to 718, macos/backup.sh 10 and 53 to 66 | Out | None |
| RCON replaced by the remote API over the Unix socket for `skytekx3ctl` and `backup.sh`, while RCON stays through the switch | Plan (the GraphQL API) | Designed | 25 |
| The three quirks that need nothing of Golem, closed by the pull request that names them | Issues #100, #101, #103 | None | 08 |
| skytekx-compat 2.1.7 stops the config watcher of NightConfig once the server has started, as a jar in `server-mods/` that the server alone loads, with the server list at "2.0.0" and the pack at 2.1.3, deployed at a restart with nobody online after a boot test on a laptop copy and a dry run of the sync | SRV okf/content/compat-mod.md 29 to 32, AGENTS.md 36 to 46 | Out | None |
| An override of `spm:world/tick` in the fixes pack that checks each item entity once and ticks only the special ones, after the split of the tick functions, tested on a laptop copy and loaded by `minecraft:reload` with nobody online | SRV world/datapacks/skytekx3_fixes, AGENTS.md 23 to 24 and 58 | Out | None |
| Pregeneration with Chunky of the Nether and the Twilight Forest to radius 1000, one level at a time at night with nobody online, once room is freed on the disk | SRV AGENTS.md 99 to 101, okf/server/world-reset.md 74 | Out | None |
| `-Dmax.bg.threads` as an element of the JVM array of `ServerMac.sh` before `-jar`, only if a recording of exploration shows the server thread slowed while the workers generate | SRV ServerMac.sh 28 to 42 | Held | None |
| `skytekx3ctl` and `backup.sh` send the vanilla commands with the prefix `minecraft:` until the remote API takes the place of RCON in them | SRV macos/skytekx3ctl 40 and 46, macos/backup.sh 22 and 68 to 72 | Out | None |
| The nightly backup checked against its guard of free space, the newest archive plus 4 GiB, and a skipped run reported, not only logged | SRV macos/backup.sh 10 and 53 to 66 | Out | None |

## The audits, the comparison and the rehearsal

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| The usage audit: every `issued server command` line of the logs since the server started, counted per command, with no player name in any file | Plan (before the port), CONTRIBUTING.md section 9 | Open | 16 |
| The sign audit of the region files for `[Buy]`, `[Sell]`, `[Warp]` and `[Balance]` | Plan (before the port) | Open | 16 |
| The facts the specs leave open, read from a copy: `warps/`, the spawn file, the rules text, `enabledSigns`, the locale of the JVM, the count of homes, balances below zero or with a fraction of a cent, the size of `plugins/LuckPerms/`, what `/depth` prints, and whether the nightly backup still runs | Teleport line 1526, Social lines 2046 and 2050, Permissions lines 244 to 246 | Open | None |
| The answers that LuckPerms gives today through Arclight for the Boolean nodes of mods, `voicechat.speak` among them, read on a test copy as golden values | Permissions lines 307 to 325 | Open | 06 |
| Golden samples on Arclight and on Golem: `tpa` with a timeout, `back` after a death, homes across dimensions, mail to an offline player and `pay`, with a list of the intended differences per issue | Plan (verification) | Open | 19 |
| A load of two or three players on the LAN for an hour at a time the owner picks, for the rehearsal and the comparison before the switch, as the owner ruled, since the client of the harness sends no movement, loads no chunk and answers the login of Forge as a vanilla client, which the live pack turns away | Decisions (2026-11-08), Golem tests/harness/minecraft.py 13 to 21, Modpack lines 137 to 140 | Open | 19 |
| Arclight against Golem on one world copy with one pregen under the load of two or three players on the LAN: TPS, MSPT, resident memory, chunks per second, Spark profiles and GC logs | Plan (verification), Decisions (2026-11-08) | Open | 19 |
| The rest of the four way comparison after the switch: plain Forge, and plain Forge with Radium or Canary and ServerCore | Plan (verification) | Open | 19 |
| A clean boot of the live pack under Golem: `Done` with 0 FATAL and no crash report, and the warning of Essentials about a hybrid server gone from the baseline of boot errors | Plan (verification) | Open | 08, 19 |
| Sundering counted once, with the damage test of the iron golem run under Golem | Issue #99 | Open | 08, 19 |
| A comparison of the content of chunks between java, golem and Arclight | Open items (notebook 08 compares no chunks) | Open | 08 |
| The rehearsal on a macOS test host with the live `mods/`, the migrated store and clients over the LAN: migration, switch, play, rollback and switch again, each timed, with the switch inside 20 minutes of downtime and the rollback inside 15, targets still to be confirmed | Plan (work plan, switch) | Open | 19 |
| The rollback rehearsed: Arclight on the world that Golem has saved, the handler set back with the JVM gone, and the two commands of `/datapack` | SRV okf/content/boot-fixes.md 93 to 111 | Open | 19 |
| A dress rehearsal on the Mac mini itself at a planned restart, on an APFS clone | A decision open with the owner | Held | 19 |
| A recording of exploration on the live server: one player explores new chunks outside the Skylands for ten minutes while another player stays in the Skylands, with one recording of two minutes, read for whether the Skylands slow while the workers of world generation are busy | This page | Open | None |
| The split of the share of the tick functions between `spm:world/tick` and `skytekx3:travel/tick` with the profiler of vanilla, `minecraft:perf start` and `stop`, at a quiet hour, which also settles the cost of the global work of each tick | SRV AGENTS.md 58 and 61 | Open | None |
| The definition of MSPT p95 for the bounds of the switch: the p95 of the 5 s samples of the mean of 100 ticks, one world copy, one player script, no recording started inside the window, per level and overall | Golem crates/golem-bridge/src/ticks.rs 14, Plan (verification) | Open | 19 |
| The comparison before the switch reports MSPT per level beside the overall figure | Plan (verification) | Open | 19 |

## The world parity

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| The plugin `golem-world`, with the values of the live `spigot.yml` as its defaults | Plan (Spigot world settings), a decision open with the owner | Open | 18 |
| The entity activation range: animals 32, monsters 32, raiders 48, misc 0, and inactive villagers ticking | SRV spigot.yml 120 to 126 | Open | 18 |
| The entity tracking range: players, animals and monsters 48, misc 32, display 128 and other 64 | SRV spigot.yml 127 to 133 | Open | 18 |
| The merge radius of experience at 3.0, with items as in vanilla | SRV spigot.yml 56 to 58 | Open | 18 |
| The mob spawn range of 6 chunks | SRV spigot.yml 59 | Open | 18 |
| At most 100 TNT per tick | SRV spigot.yml 116 | Open | 18 |
| The max tick time of 50 ms for block entities and entities, ported only if the load test shows that it triggers | SRV spigot.yml 117 to 119, Plan (Spigot world settings) | Held | 18 |
| The goal selector updated every second tick | SRV arclight.conf 71 to 74 | Open | 18 |
| An answer to the async catcher of Arclight, which runs world access from other threads on the server thread | SRV arclight.conf 13 to 19 | Open | 18 |
| Chunks unloaded every 600 ticks | SRV bukkit.yml 21 to 22 | Open | 18 |
| The multiplier of 10.0 for moving too quickly | SRV spigot.yml 33 | Open | 18 |
| Four threads of Netty | SRV spigot.yml 22 | Open | 18 |
| A login throttle | SRV bukkit.yml 7, a decision open with the owner | Held | 18 |
| A log line per issued command, so the usage audits go on after the switch | SRV spigot.yml 46 | Open | 18 |
| The structure seeds checked before the switch on a region generated under Arclight and under Golem, with the salts set by Golem only if a structure moves | Plan (Spigot world settings), SRV spigot.yml 93 to 107 | Open | 18 |
| The persistent data of Bukkit kept or checked: `BukkitValues` of entities and of the level, `PublicBukkitValues` of items and block entities and `ChunkBukkitValues`, which vanilla drops, through the Mixins of API 9.11 or a check on a copy of the live world that no tag in use is lost | API 9.11 and 19 | Open | 18 |
| The world names of Arclight and `uid.dat` kept, checked on a copy of the live world | API 8.1 and 19 | Open | 18 |
| The names that keep the command of the game when a plugin command takes them, `setblock`, `summon`, `testforblock` and `tellraw`, read from a setting of `golem-world` that stands for `commands.replace-commands` in place of a constant of the mod | SRV spigot.yml 36 to 40, Open items (the names that keep the command of the game), Bridge (commands) | Open | 18 |
| The spawn periods of `ticks-per` in bukkit.yml (monsters and the water, axolotl and ambient kinds 1, animals 400, the autosave 6000) checked against vanilla and kept as settings of `golem-world` | SRV bukkit.yml 23 to 31 | Open | 18 |

## The levels

These rows keep a busy level from slowing the others: windows, hooks, queues and a lock per plugin
on the one server thread first, then budgets per level, then the loop of a protected level of its
own, which stays on a test server until it keeps the pace of the protected level with the live mods.

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| A tick window per level beside the window of the server, fed by the tick hooks per level, so Golem reports MSPT per level as `forge tps` does | Golem crates/golem-bridge/src/ticks.rs 14 to 91 | Open | 40_level_ticks |
| Tick hooks per level: `LevelTickEvent` START and END beside `ServerTickEvent`, and a native `tick` that names the level | Golem bridge ServerEvents.java 96 to 123, bridge Host.java 103 | Open | 40 |
| One action queue per level, drained at the end of that level's tick, beside the global queue for commands, broadcasts and teleports across levels | Golem crates/golem-bridge/src/actions.rs 188 to 231 | Open | 40 |
| A teleport across levels as a hand off with a completion: the source level removes the entity at the end of its tick, the target level finds the place or the portal and adds it at the start of its next tick, and the caller learns the outcome | Golem bridge Actions.java 189 to 214, API 7.5 | Open | 43_level_handoff |
| A lock per plugin in place of the one lock of the handler, so a call on one level never waits for another plugin. The lock is not reentrant, so the nested calls of the shared handle of API 11.1 keep their own answer | Golem crates/golem-bridge/src/host.rs 101 to 106 and 624 to 652, API 7.10 | Open | 04, 40 |
| A scheduler in the shape of Folia: level, entity, global and async, delays in ticks of the running level, an entity task that follows its entity and retires with it, and a manifest flag for plugins that run without their lock | API 9.7 | Open | 40 |
| A budget per level for the chunk work of the server thread, promotion and sending per player and tick, deferred first when a level is over its budget | Vanilla 1.20.1 `ChunkMap` and `ServerChunkCache` | Open | 41_level_budgets |
| A spawn budget per level: a level over its budget spawns at a lower rate while the caps stay, and entity and block entity ticks never skip, read from the spawn settings of `golem-world` | SRV bukkit.yml 13 to 31 | Open | 41 |
| A quota of workers per level for world generation, so one explorer takes at most a share of the pool, 2 by default until a measurement sets it, under the total that the launcher sets | Vanilla 1.20.1 `Util` and its property `max.bg.threads` | Open | 41 |
| The watchdog, the degrade window and the cost model of API 11.4 and 12.2 per level, with a soft cap of 10 ms per level and tick and no cap for a protected level, starting values until a measurement sets them | API 11.4 and 12.2 | Open | 41 |
| The audit of mod state: which mods of the pack keep state shared across levels, where they tick it and how they handle packets, read by a scan of the class files of the jars on a laptop copy, never by running a jar | Modpack, SRV arclight.conf 13 to 19 | Open | 44_mod_state_audit |
| A detector for tests that records every access of a level thread to state it does not own | SRV arclight.conf 13 to 19, the async catcher of Arclight as its model | Open | 44 |
| The dispatcher: levels on a fixed pool of 2 to 3 tick threads by their recent cost, a protected level on a thread of its own, the global work on the server thread, once the audit of mod state passes | SRV okf/server/macos.md 21 | Open | 42_level_loops |
| The global work of each tick: tick functions, `ServerTickEvent` handlers once per tick, the tasks queued on the server executor, commands, chat, the player list, the autosave and the stop path, run while it holds the lock of each protected level | This page | Open | 42 |
| The chunk cache of a decoupled level bound to the thread of that level, its executor the waker of the level | Vanilla 1.20.1 `ServerChunkCache`, its fields `f_8330_` and `f_8332_` | Open | 42 |
| The connections and packets of the players of a decoupled level handled on its thread, while the packets of mods stay in the global work, which runs while that level's loop pauses under its lock, or go to that level's executor when the audit ties them to it, a design that the audit of mod state is still to confirm | This page | Open | 42 |
| The decoupled loop of a protected level behind a flag, on a test server only and never on the live server, after the audit, the detector, the dispatcher, the global work, the chunk cache and the packets, until the protected level keeps its quiet MSPT while another level runs ticks of 40 ms with the live mods | This page | Held | 42 |
| The QoS class of the tick threads on macOS, so they stay on performance cores, once a measurement shows the need, starting from the `ProcessType` of `Interactive` that the job of launchd sets today | SRV macos/com.skytekx.server.plist 47 to 48 | Held | 17, 42 |

## The GraphQL API of Golem

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| The first measurements: the builder on a generated skeleton of the whole entity tree, a stub with the out buffer and a frame, one canonical operation through the `Service`, a document nested in a handler and a panic unwound through the executor, the hand over of `on_execute` with a wake source on Linux and macOS, GraphTP through the stream adapter with the JVM running on a macOS test host, the mount of a part, one Mixin beside the mods, and the first call measured beside the steady one | API 20 | Designed | 20_api_contract, 21_api_schema, 28_api_hooks |
| The shared base: the mapping and its generators, the contract, `golem-game`, `golem-model`, `golem-api` over the embedding or its fallback, `api client` and `g.run`, the coverage lock, a first dispatcher and runtime, and the vertical slice | API 20 and 21 | Designed | 20, 21, 22_api_coverage, 23_api_events, 24_api_smoke, 26_api_corpus |
| The items that Golem asks of Aosabi, held by the rows of [What Golem asks of Aosabi](#what-golem-asks-of-aosabi) | API 20 and 4.5 | Out | Aosabi |
| The remote clients: GraphTP on the Unix socket and the socket pair, HTTP on loopback, tokens, groups, the OpenID Connect login, nodes, limits, buckets, cost, answer size, no chunk load, the deadline, `SERVER_BUSY`, the audit log, streams and the serve point | API 13 and 20 | Designed | 25_api_remote |
| The event core: chains, priorities, `ignoreCancelled`, `MONITOR`, nesting to 16, `golem-chat` and `golem-net`, masks, filters, interest tables, rates, `call<Event>`, the events of ports and remote copies | API 10 and 20 | Designed | 23_api_events |
| The plugin runtime: the lifecycle, `plugin.toml`, the order, the ledger, `Game` and `AsyncGame`, hops, `PluginLock`, budgets, the deadline, the watchdog, the degrade mode, the quarantine, the scheduler, services, the configuration and the shim over the client | API 11 and 20 | Designed | 04, 27_plugin_lifecycle |
| Entities: 171 interfaces with 1,147 methods, `entitiesOf`, spawning, attributes, effects, modded types and records | API 9 and 20 | Designed | 30_api_entities |
| Worlds and blocks, with the world names of Arclight and `uid.dat` | API 8.1 and 20 | Designed | 31_api_worlds |
| Items and data: materials, registries, `golem-nbt`, meta, enchantments and persistent data | API 9.11 and 20 | Designed | 32_api_items |
| Inventories, views, menus, merchants and the preconditions of writes | API 20 | Designed | 33_api_inventories |
| Commands and permissions: the order of vanilla names, completion, senders that are no entity, attachments above the stored nodes, defaults, the tree sent again, the PermissionAPI and help | API 9.8, 9.9, 22.4 and 20 | Designed | 34_api_commands, 06 |
| About 115 events of the hook classes F, P and G, with the chat future and the chains of `golem-net` | API 10.8 and 20 | Designed | 28_api_hooks, 23 |
| About 137 events of the hook class M with their Mixins | API 10.8 and 20 | Designed | 28, 23 |
| Display: scoreboards, boss bars, components, messaging, titles and the player list | API 20 | Designed | 35_api_display |
| Content: maps, recipes, loot, advancements, structures, bans, profiles and statistics | API 20 | Designed | 36_api_content |
| Worlds made at run time, generators and biome providers | API 20 | Designed | 37_api_worldgen |
| The seam of the Bukkit layer: `Host.call`, the subscribers of the layer, the error of the `AsyncCatcher` and the generated Java | API 17 and 20 | Designed | 38_api_layer |
| Process plugins over a socket handed to them on stdin, with spawn, restart and late answers | API 11.5 and 20 | Designed | 29_api_process |
| The conversion kit: the guide of 8.4, the cookbook, `api slice`, `api propose` and the corpus per part of the API | API 20 | Designed | 26_api_corpus |
| The slice of EssentialsX: the ports moved from the trait onto `golem-client`, the 66 members that the ports call beside the 756 methods, 211 constants and 42 events of the jar, their part of the schema under their licence, and the golden samples run again | API 4.2, 16 and 20, Bridge needs lines 52 to 110 | Designed | 12 to 16, 27 |
| The page of the API caught up on the commands of mods, in 22.6 and 9.8, as the bridge treats them | API 9.8 and 22 | Open | 10 |
| The field `server.source` and the text of the server list name the source, as section 13 of the AGPL asks | Plan (license) | Open | 24_api_smoke, 07 |
| The decisions 22.1 to 22.7 recorded as rulings once the owner confirms them | API 22 | Held | None |
| The dependency of Golem on Aosabi: a path to a checkout of a tagged release, `golem-api` as a workspace of its own until a release of Aosabi takes a guest that is a member of a larger workspace, and a line in `deny.toml` on why it allows the source | Plan (where the code lives), the file deny.toml | Open | 01, CI |
| The budget of a first call set from the number that the first measurements of the API take per thread, since each thread pays its own first call | Open items (the first call) | Open | 20 |
| Every thread of Golem that runs the executor gets the state that the engine keeps per thread on its first call, and the threads that Golem makes have stacks of 8 MiB | Golem crates/golem-jvm/src/invocation.rs 57 | Open | 20 |
| The thread classes `level(dim)` and `global` of API 7.1, the hops `on_level(dim)` and `on_global` with no blocking hop between level threads, and the remote operations of each level through the API slot | API 7.1, 7.5 and 7.8 | Designed | 20 |
| A tick snapshot per level, beside the one snapshot of the server of API 7.7 | API 7.7 | Open | 20 |
| Game queries answered on the thread of their level, a query about a player on the thread of the player's level | Golem bridge GameQueries.java 176 to 185 | Open | 20 |
| Typed subscriptions for remote clients: events as object types with their subtypes as abstract types, a stream that falls behind ended with `EVENTS_LOST` and the count, chat and commands included, so that the client resubscribes and reads the state again, and the newest value kept for the tick and positions | API 10.1, 10.7 and 13 | Open | 23, 25 |

## The Bukkit layer

The counts of working plugins add up in the order of the survey of [Bukkit layer](bukkit-layer.md)
section 9.

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| Level 1: spigot-api 1.20.1 unchanged, its events, the plugin loader and `CraftServer` in `v1_20_R1` with the strings of Spigot, 136 plugins working after it | Layer 9 | Surveyed | 60_bukkit_level1 |
| A runtime of Java 21 or 25 for the classes of plugins or a downgrade at load, 157 working | Layer 9 and 13 | Surveyed | 60 |
| An `api-version` above 1.20 and the misread 1.2 accepted, 180 working | Layer 9 | Surveyed | 60 |
| The command map, registration and help of level 2, 196 working | Layer 9 | Surveyed | 61_bukkit_level2 |
| The handles of entities and players of level 2, 217 working | Layer 9 | Surveyed | 61 |
| The remapper of level 3 from Spigot names to the classes of Mojang with SRG members, reflection answered, classes made at run time and class files of Java 25, 248 working | Layer 5 and 9 | Surveyed | 62_bukkit_level3 |
| The game profiles of level 2, 265 working | Layer 9 | Surveyed | 61 |
| Item stacks, meta and persistent data of level 2, 282 working | Layer 9 | Surveyed | 61 |
| Worlds, chunks, blocks and block data of level 2, 309 working | Layer 9 | Surveyed | 61 |
| The handle of the server of level 2, 319 working | Layer 9 | Surveyed | 61 |
| The conversions of level 2, with a rule for modded blocks and items that have no `Material`, 335 working | Layer 9 | Surveyed | 61 |
| The additions of CraftBukkit to the classes of Minecraft, level 3, 353 working | Layer 9 | Surveyed | 62 |
| The boss bars of level 2, 392 working, EssentialsX among them | Layer 9 | Surveyed | 61 |
| The other classes of CraftBukkit of level 2, 409 working | Layer 9 | Surveyed | 61 |
| The chat conversions of level 2, 422 working | Layer 9 | Surveyed | 61 |
| The pipeline of Netty of level 3, 440 working | Layer 9 | Surveyed | 62 |
| Inventories and menus of level 2, 456 working | Layer 9 | Surveyed | 61 |
| The loader and the API of Paper, all of Paper or none, built after the work of Spigot, which Paper follows, 471 working in the order of the survey | Layer 9 and 13 | Surveyed | 63_bukkit_paper |
| The event factory of level 2, 485 working | Layer 9 | Surveyed | 61 |
| The maps of level 2, 490 working | Layer 9 | Surveyed | 61 |
| The scheduler of level 2, 494 working | Layer 9 | Surveyed | 61 |
| The injection of permissibles of level 2, 497 working, LuckPerms among them, with no early injection and checks asked of the plugin that owns the decision | Layer 9 and 13 | Surveyed | 61 |
| The scoreboards of level 2, 498 working | Layer 9 | Surveyed | 61 |
| The corpus of 503 jars run again on the layer, with the five out of reach recorded | Layer 2 and 10 | Surveyed | 64_bukkit_corpus |
| The mapping at run time once Arclight is gone: where the SRG files come from, under which terms, and a clean room for the stand ins of CraftBukkit | Layer 5, a decision open with the owner | Held | 62 |
| The tools of the survey repaired: the scan records that `details.py` reads, the count of agents, the depth of nested jars and the tagged descriptors | Tools tools/plugin-survey | Open | None |
| The efficiency requirement of the owner: the cost of a call of level 1 measured against Arclight, since dropping the layer is the better choice if that is clearly faster | Plan (owner requirements) | Open | 64 |
| Mods that detect Bukkit by its classes keep their path | Layer | Open | 61 |
| The 49 plugins that name `libraries` get them through the Maven Resolver that the layer ships | Layer 8.3 | Surveyed | 60 |
| The Bukkit layer on level threads: a plugin that declares the support of Folia in the API of Paper takes the schedulers of the levels, and every other plugin shares one lock that the global work holds too, under which `isPrimaryThread` answers true | API 7.1 and 7.5, Layer 5.2 and 13 | Open | 60 to 64, 42 |

## The native code and what follows

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| Noise in Rust, bit for bit, called in batches of a chunk, with `PerlinNoise` kept in Java | Plan (native acceleration) | Open | 50_native_noise |
| Density functions compiled to Rust per dimension, with the types of mods as Java callbacks and the Skylands datapack unchanged | Plan (native acceleration) | Open | 51_native_density |
| Region files compressed with libdeflate in the same format | Plan (native acceleration) | Open | 52_native_region |
| The recorder of golden samples per dimension, and a pregenerated region equal block for block between vanilla and Golem | Plan (verification) | Open | 53_native_golden |
| Pathfinding in Rust, only if the load test shows a cost | Plan (native acceleration) | Held | 53 |
| C2MEF measured with the native code | Plan (native acceleration) | Open | 53 |
| Radium, Canary and ServerCore tried again without Arclight | Plan (Bukkit plugins) | Open | 19 |
| A public API through the Cloudflare Tunnel, with tokens and the login of operators through OpenID Connect | Plan (architecture), API 13.1 and 13.2 | Designed | 25 |

## What Golem asks of Aosabi

Each row is a need of Golem that Aosabi, a project of its own, may meet in a tagged release. [Plugin
API](api.md#45-what-golem-asks-of-aosabi) section 4.5 holds what each item serves and the fallback
of Golem until it lands.

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| A tagged release of Aosabi that holds the embedding entry of the branch, which Golem pins in place of the branch | The branch | Out | Aosabi |
| `Embedded` hands out its `Service` beside a typed `execute` whose validation the caller chooses, with `answer` kept for hosts of JSON, and the same typed `execute` serves a document run inside the handler of a cancellable event, for its synchronous answer | The branch: crates/aosabi/src/embed.rs 70 to 73, crates/aosabi-core/src/embed.rs 80 and 83, and API 10.4 | Out | Aosabi, 20 |
| A principal, headers or a value of the host handed into the context of a call, so two engines live in one process | API 4.5 and 7.8 | Out | Aosabi |
| An async run over `prepare_request`, `agent.drive` and `finish_request` that never spins on a foreign wait | API 4.5 | Out | Aosabi |
| The served boot without `serve_app` and without argv, with routes, plugins and the seam of `on_execute`, which returns the booted app and its listeners, and a stop | Aosabi crates/aosabi-serve/src/serve.rs 414 to 655 | Out | Aosabi |
| Subscriptions inside a host as a stream that the host polls with no wait or that wakes its host, or through a callback that a pump of the host runs, with cancel, whose events may be object types | API 10.7 | Out | Aosabi |
| A guest that is a member of a larger workspace, or the exclusion documented | Aosabi crates/aosabi-cli/src/codes.rs 232 to 239 (AO-C0103) | Out | Aosabi |
| A host card for aarch64-apple-darwin and for the triples of Linux | Aosabi crates/aosabi-cli/src/cards/builtin.rs | Out | Aosabi |
| A prepared operation that every thread shares, `Service::prepare` and `execute_prepared` over the public `KeptDocument` with an async form, where Aosabi keeps a plan per thread and frees it with the operation, and Golem keeps one set for all its threads and gives its `u32` ids at enable | Aosabi crates/aosabi-execute/src/plan.rs 272, crates/aosabi-core/src/service.rs 21 and 352 to 357 | Out | Aosabi, 20 |
| A cache of documents behind `Service::execute` and the embed answer, for a host that sends one text again and again | Aosabi crates/aosabi-cache | Out | Aosabi |
| One input name per struct, as an option of the chain of the entry that a host opts into, so other hosts keep their names by default | Aosabi crates/aosabi-build/src/source/names.rs 105 to 123, Documentation/guides/rust.md 259 to 261 | Out | Aosabi, 21 |
| A newtype with one unnamed field read as a custom scalar and checked through `TryFrom` on input, for `UUID`, `Key`, `SNBT` and `Component` | Aosabi crates/aosabi-build/src/source/reach.rs 540 to 549 | Out | Aosabi, 21 |
| The doc line `@implements` read into the implements list, then traits with supertraits as interfaces | Aosabi crates/aosabi-build/src/source/jsdoc.rs 793 to 812 | Out | Aosabi, 21 |
| The stream half of `homa::unix` on every Unix target, with the send flags per target and `SO_NOSIGPIPE` | Aosabi crates/homa/src/unix.rs 93 to 106 | Out | Aosabi |
| One task loop for every carrier of one endpoint | API 4.5 | Out | Aosabi |
| Host entries for GraphTP on the path of a Unix socket and on a handed stream, with a `Fetch` | Aosabi crates/aosabi/src/lib.rs 875 | Out | Aosabi |
| The directory of the socket from the host, with the length of the path checked | API 4.5 | Out | Aosabi |
| A reactor on macOS that learns readiness, the safe poll behind a feature that a host opts into | Aosabi crates/aosabi-serve/src/runtime/reactor.rs 61 and 325, CHANGELOG.md 47 | Out | Aosabi |
| A wake of the runtime from another thread | API 7.8 | Out | Aosabi |
| A read of `Framed` large enough for the remote clients of Golem, once a measurement shows the need | Aosabi crates/homa/src/stream.rs 430 | Out | Aosabi |
| The Rust client of GraphTP, a std driver in a blocking and a task mode over the engine of `Link`, with the socket of a process plugin handed over on stdin | Aosabi crates/graphtp-wasm/src/link.rs 223 to 459, API 11.5 | Out | Aosabi, 29 |
| The peer uid of a Unix socket on macOS through `getpeereid`, for a socket that the host hands over as well | Aosabi crates/aosabi-serve/src/runtime/peer.rs 73 to 84 | Out | Aosabi |
| `ObjectLike::select` on the plain path, so the records and the masked reads of the API work | Aosabi crates/aosabi-execute/src/value/object.rs 244 and 272 to 273 | Out | Aosabi, 20 |
| Status, substatus and content type on the embed answer, and a `Fetch` over the embedded engine | API 4.5 | Out | Aosabi |
| The bound of one message of Homa, 1,000,000 bytes, reachable through the facade, so a host bounds an answer before it sends it, while only `graphql-over-graphtp` names it today | Aosabi crates/graphql-over-graphtp/src/limits.rs 66 | Out | Aosabi |
| What the remote API of Golem builds on: a bound of 64 streams per principal, an OpenID Connect plugin that serves any provider, beside the public `aosabi-auth`, for the login of operators, and stable error codes such as `NOT_ON_SERVER_THREAD` | API 4.1, 10.7 and 13.2 | Out | Aosabi |
| The state that the engine keeps per thread, the clock, the environment, the logger and the fetch of the host among it, set up by the engine on the first call of each thread, so a host calls from any thread | API 7.1 | Out | Aosabi |
| Typed values on the hot path: variables from Rust values and the answer read as a value, with no JSON text between | This page | Out | Aosabi, 20 |
| Subscriptions whose events are object types stream them, with the selection set run over every event as graphql-js does, and a getter or a method of an event object resolves as it does in a query | Aosabi crates/aosabi-compat/src/lib.rs 136 to 144 | Out | Aosabi |
| Abstract types as the items of a stream, resolved per event | This page | Out | Aosabi |
| An overflow policy per topic or per listen: keep the newest, drop the oldest, or end the stream with an error that carries a code and the count, beside a depth per topic and a bound of bytes per subscriber, opt in | This page | Out | Aosabi |
| The order across the streams of one client stated and tested as the order in which entries become ready, and a client that needs one order reads one stream or merges by a field of its events | This page | Out | Aosabi |

## The tooling, CI, lints and pages

| Feature | Source | State | Notebook |
| --- | --- | --- | --- |
| The commits of the bundle carry `Co-Authored-By` and `Claude-Session`, the type wip is gone, and every commit builds and passes on its own | HANDOVER.md section 2 rule 5 | Open | 02 |
| A lock of every package of the harness with hashes, with markers for macOS, `--require-hashes`, a tool that writes the lock again, and the patch release of Python | Issue #216, commit ec39475 | Part | 11, CI |
| CI keeps its triggers | Issue #217 | None | CI |
| Rule 6 of the commit lint takes any name in the sign off, and the review checks the name | Issue #223, commit 2c3d5d4 | Part | 02 |
| A tab of 8 columns for the Java of the bridge, with 65 lines rewrapped and the width named in CONTRIBUTING.md section 4 | Issue #228, commits 956ad76, 0770222, 1897775 | Part | 10 |
| The pages caught up: the stale item of Open items on the bug hunt, the rows Other and Admin of Plan, 18 natives in the index, Bridge needs against `aa75076`, the seed row of Decisions, and the rule for the commands of mods in Bridge and API 22.6 | Open items, Plan (EssentialsX 2.22.0), issue #186 | Open | 10 |
| This list kept current: every commit that lands a feature changes the state of its rows, and a quirk keeps its line on the page of its plugin | CONTRIBUTING.md section 7, HANDOVER.md section 5 | Built | 10 |
| The rule of notebook numbers, ports and the lock of one server per host written into Testing: 12 to 19 for the switch, 20 to 38 for the API, 40 to 44 the levels, 50 to 53 native, 60 to 64 the layer, ports in steps of 22 from 25699, and one lock per host that every notebook which boots a server takes | Decisions (answers of the agents), HANDOVER.md section 8 | Open | 10 |
| The page `HANDOVER.md` and the folder `handover/` deleted once the bundle has landed | HANDOVER.md line 9 | Open | 10 |
| The docs lint reads the Markdown cells of the notebooks for the 100 columns, or the rule says that it does not | Golem .github/scripts/docs_lint.py | Open | 10 |
| The two rules of history reconciled in CONTRIBUTING.md section 1: pushed commits are never rewritten, so the clean up before a merge into master and its `--force-with-lease` touch only commits that are not pushed yet, and how work done beside other work is pushed waits for the ruling of the owner | CONTRIBUTING.md lines 18 to 20, HANDOVER.md section 2 rule 5 | Open | 02 |
| A half install of Forge or Gradle that `tests/fetch.py` leaves behind is found, not taken as in place | Open items (a half install) | Open | 11 |
| The conflict of two proxies of rustup on `bin/cargo-fmt` reproduced or closed, with the step that installs the components before any cargo kept | Open items (two proxies of rustup) | Open | 11 |
| The commit-msg hook judges the options of `git commit` as git stores the message, or the page says where it cannot | Open items (the commit-msg hook) | Open | 02 |
| Notebook 08 tells a call over the budget on a loaded machine from a fault of a plugin | Open items (notebook 08 fails a boot) | Open | 08 |
| The harness reads the pointer files of `local-mods/` with the keys of the tool of the server repository, allows the host of each pointer for that file alone, checks the size and the SHA-1, records the SHA-512, and keeps the jar out of git and out of every artifact of CI | SRV tools/fetch_local_mods.py at master, Golem tests/fetch.py 117, 475 and 614 | Open | 08, 11 |
| The pin of `tests/fetch.py` moved to the pack commit of 3.1.4 on master, `dc03fb4e6365`, with the counts of its pack in notebook 08, and a check that every later pin is reachable from master | Golem tests/fetch.py 110 to 111 and 354 to 372, SRV master | Open | 08 |
| The case of notebook 08 on the names of a refusal compares both names after NFC and accepts either form, since git on macOS precomposes the names of files | Golem tests/fetch.py 381 to 388, notebook 08 | Open | 08 |
| Notebook 08 reads the options of the live JVM from the JVM array of `ServerMac.sh` on macOS and from the unit of systemd on Linux | SRV ServerMac.sh 28 to 42 at master, Modpack (what the notebook checks) | Open | 08 |
| Notebook 08 names the two mods of master that name Arclight, Supplementaries 3.1.44 and ActuallyAdditions 1.1.52-skytekx.1, and checks that each takes its plain path under golem | Modpack (the answer), SRV master | Open | 08 |

## Keeping the list current

Every commit that lands a feature changes the state of its rows, and the counts at the head of this
page with them. A feature found later gets a row of its area in the commit that adds it. A row is
never deleted: a feature that a ruling drops keeps its row with the state None and the ruling in its
source. A row of another project leaves the page, since the page lists only the features of Golem
and what Golem needs. A quirk issue stays open until the pull request of its port lists it with
`Fixes #n` in its body, as section 6 of CONTRIBUTING.md asks, and its row turns Built in that pull
request. Notebook 10 holds this page to the docs lint.
