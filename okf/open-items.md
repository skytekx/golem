---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Backlog
title: Open items
description: What is still open on Golem, from the decisions that the owner has still to make to the parts not built yet and what remains open in the parts that are built.
tags: [open-items, backlog, todo]
timestamp: 2026-11-08
---

# Open items

A number such as H03 names a finding of the bug hunt of 2026-11-06, see [Change log](log.md).

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
- Whether golem drops `-Xrs` and takes `SIGINT`, `SIGTERM` and `SIGHUP` through handlers of the JVM
  in the host jar, with a new native, so that HotSpot keeps `SIGQUIT` and creates an attach socket
  that a cleaner removed again, which goes against the plan, where Golem passes `-Xrs` (H03). Until
  then a line of tmpfiles.d keeps the socket on Linux, see [JVM host](jvm-host.md#the-stop-path).
  macOS has no tmpfiles.d and cleans the temporary folder of the user by age, so a touch of the
  socket by golem once a day on every host is planned, which keeps it without that decision.
- Whether a command of a mod that a plugin command names moves to `minecraft:<name>` as a command of
  the game does, as the bridge moves it and as Arclight wrapped the commands of the mods on the live
  server, while decision 6 of [Plugin API](api.md#22-decisions-for-the-owner-to-confirm) names the
  commands of the game alone (H14).
- Whether the harness locks every package that it installs with its hashes, those that its eight
  pins pull in included, and pins the patch release of Python, or keeps the pins of the eight
  packages it names alone, as the comment of `tests/pyproject.toml` states (H45).
- Whether CI runs once for a push to a branch with an open pull request, where the push and the pull
  request each run the whole of CI (H46). Running the push on main and the tags alone takes CI from
  a work branch without a pull request, and one group of concurrency for both events cancels a run
  of an earlier commit that still runs.
- Whether the commit lint refuses a `Signed-off-by` that does not name the holder, which would also
  refuse the sign off of an outside contributor (H53). Until then the review checks the name, as
  [Decisions](decisions.md) says, and notebook 10 holds that row to the lint.
- How wide a tab counts for the 100 columns of the Java of the bridge, which section 4 of
  CONTRIBUTING.md does not say (H58). At a tab of 8 columns, as in the kernel, 65 lines of the Java
  under `bridge/` are wider than 100, and at a tab of 4 none is, as `expand` counts them on
  2026-11-07.
- Whether the group `default` ranks below the groups that a player holds directly, since a group
  without a weight shares its rank, so a wildcard deny of `default` beats a grant by name of such a
  group, unlike LuckPerms (H35). A lower rank would answer as LuckPerms there, and otherwise than
  LuckPerms where `default` denies a node by name and such a group grants its wildcard, see
  [Permissions](plugins/permissions.md#where-golem-differs-from-luckperms).

Whether a Bukkit layer is ever wanted is answered by the ruling of 2026-11-06 in
[Decisions](decisions.md): Bukkit and Spigot plugins run unchanged through a Bukkit layer on the
Golem API.

## Not built yet

- The GraphQL API on Aosabi, which [Plugin API](api.md) designs, the Bukkit layer, which
  [Bukkit layer](bukkit-layer.md) surveys, the ports of EssentialsX, EssentialsXSpawn and the
  Spigot world settings, the migration tool and the native acceleration of world generation. See
  [Features](features.md).
- The usage audit and the sign audit, which read copies of the logs and of the world of the Mac
  mini, and the comparison of Arclight and Golem under [Plan](plan.md#verification), which needs a
  copy of the live world.

## Open in what is built

- `golem run` does not open the store yet. The steps that wire it in, with the close in the exit
  hook of `golem-jvm`, stand in [Store](store.md#the-place-in-the-program-golem).
- `golem run` hands the plugins no permissions. The program `golem` links no `golem-permissions` and
  the context of a plugin offers no check of a node, so a plugin judges a sender by the level of the
  source and the operator level of the player alone, and an edit of `config/permissions.toml`
  changes nothing on a running server, see [Permissions](plugins/permissions.md). Wiring them in
  loads the file or the store and gives the context a check of `golem-permissions`.
- `golem-permissions` answers no `on_permission` yet. A port of LuckPerms answers every Boolean
  node of an online player as LuckPerms did through Arclight, see the open decision above, and
  the importer of a LuckPerms export, the members of the group admin and a comparison of the seed
  with an `lp export` of the Mac mini are open.
- `/golem status` does not name the permission handler that Forge took, issue
  [#141](https://github.com/skytekx/golem/issues/141). The warning that the mod logs on
  `ServerStartingEvent` when Forge takes another handler than `golem:permissions` is the only sign.
  Naming it needs an eighth question of `Game` beside `log` or a native, which changes the contract
  of [Bridge](bridge.md#the-game) and the mapping of [Plugin API](api.md) section 16.
- Actions that a plugin queues on `serverStopping()` or `serverStopped()` never run, since no tick
  follows to drain them. The mod could drain once in its stopping listener, while the players are
  still there.
- The plugin runtime treats `serverStopping()` as the end of every open session, since a player
  still connected at the stop may get no `PlayerLoggedOutEvent`.
- The names that keep the command of the game when a plugin command takes them, `setblock`,
  `summon`, `testforblock` and `tellraw`, are a constant of the mod until Golem has a configuration
  that stands for `commands.replace-commands` of Spigot, see [Bridge](bridge.md#commands).
- A mod that registers a command at the lowest priority after the bridge still merges it into the
  node of a plugin command of that name. When the bridge moves a command aside, it points the
  aliases of the root that redirect to it at the moved command, and no redirect deeper in the tree.
- The mod logs every line that Golem logs through the game at the level `INFO`, since a line carries
  no level, so a panic of a plugin or a call over its budget reads in `logs/latest.log` as the lines
  of a clean boot do.
- Text that a plugin echoes from a player passes formatting codes into the line that the mod builds
  with `ForgeHooks.newChatWithLinks`, so the ports strip them.
- `GOLEM_COMMIT` in the release build, so that a binary built outside a checkout still names its
  commit. A build outside the checkout of Golem names the commit unknown, and a build whose files
  differ from HEAD names `<hash>-dirty`, and `/golem source` gives the repository alone as the link
  of both.
- The launcher on macOS takes `lib/server/libjvm.dylib` of the Java home
  (`crates/golem-jvm/src/lib.rs` 160 to 167), and no run of CI on macOS proves it yet, see
  [Features](features.md#the-jvm-host-and-macos).
- golem logs the line about a shutdown signal that the process started with ignored before its
  banner (`crates/golem/src/main.rs` 492 to 504 and 604 to 610), so a launch under `nohup`, or by
  any parent that ignores `SIGHUP`, fails the check of a clean boot in notebook 08, which expects
  the banner first. The banner is planned as the first line on every host.
- `tests/fetch.py` takes only the `.jar` files of `local-mods/` and fetches from
  `cdn.modrinth.com` and `github.com` alone, while the master branch of the server repository
  keeps a pointer file in `local-mods/` for a jar whose author allows it in a modpack but no repost
  elsewhere, so a pack of master lacks that jar. Reading the pointer as the tool of the server
  repository reads it is planned, with the size and the SHA-1 checked and the jar kept out of git
  and out of every public artifact.
- The pin of `tests/fetch.py` names a commit that lies on a side branch of the server repository
  and not in the history of its master, and the fetch by hash fails once that branch goes. The pin
  is planned to move to the pack commit of 3.1.4 on master, with a check that every later pin is
  reachable from master.
- The case of notebook 08 on the names of a refusal in NFC and NFD assumes Linux. git on macOS
  precomposes the names of files, so the refusal names both in NFC there.
- Notebook 08 compares the spawn, the datapacks and the folders of the dimensions of the worlds
  under java, golem and Arclight, not the chunks in them, so a change in the generation under
  golem would pass, see [Modpack](modpack.md#what-is-not-checked). A comparison of the content of
  chunks is open.
- The pack of the server repository keeps skytekx-compat 2.1.3, as the server repository records
  it, while the live server runs 2.1.6, and its index still marks Nostalgic Tweaks as required on
  the server, which the live server moved out of `mods/` by hand. A place for jars that the server
  alone loads, which holds the compat of the server and the bridge jar, is planned in the server
  repository, see issue [#102](https://github.com/skytekx/golem/issues/102).
- The first call of `/golem player`, `/golem world` and `/golem teleport`, the tools of operators
  that ask the game through the JNI, goes over the default budget of 1 ms. A boot of stock Forge
  47.4.18 with the bridge under `golem run`, in the cloud container of the session on 2026-11-07,
  that runs the commands of notebook 07 one at a time with `/golem status` after each counts 3 calls
  over the budget, one for each of the three, the longest `/golem player Tester` of an operator at
  6.546 ms. The call of the core plugin on `serverStarting`, which logs a line through the game,
  takes 0.260 ms on that boot.
- Notebook 08 fails a boot of golem on a call of a plugin over its budget of 1 ms. It runs only
  `/golem source` and `/golem status`, which ask the game nothing. On its run with the fixes of the
  bug hunt, in the cloud container of the session on 2026-11-07, the longest call of the core plugin
  took 0.210 ms on the boot under golem and 0.241 ms on the boot of golem on the world of Arclight,
  as `/golem status` counts them. A loaded machine might still push such a call over the budget with
  no fault of the core plugin and fail the notebook.
- A half install of Forge or Gradle that `tests/fetch.py` left in its place, before it installed
  beside the place, still passes as in place, since the proof is `unix_args.txt` or `bin/gradle` in
  that place, so such a cache of the downloads is removed by hand once.
- Two proxies of rustup that add the components at once fail with the conflict on `bin/cargo-fmt`,
  but no run outside CI reproduced the state of runs 47, 60 and 67, where every cargo after the
  first failed the same way. The step that installs the components before any cargo leaves the
  proxies nothing to add in either case.
- The commit-msg hook sees the configuration of git and the message file, not the options of
  `git commit`, so `--cleanup=whitespace`, `--cleanup=verbatim` and `--no-verbose` on the command
  line count only through the configuration, and the hook may judge such a message otherwise than
  git stores it. The range lint of CI judges the message that git stored.
- No issue records a finding of the bug hunt yet, while #8 to #27 and #108 to #172 record those of
  the reviews before it. H28, the switches of EssentialsX in the group admin, and H35, the rank of a
  group without a weight, are quirks of the port of LuckPerms, which section 6 of CONTRIBUTING.md
  asks an issue for, and their lines on [Permissions](plugins/permissions.md) wait for the numbers.
