---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Plan
title: Golem, the Rust hybrid server
description: The plan for Golem as the owner set it on 2026-11-06, with the later rulings of that day: an Aosabi based Rust host that replaces Arclight, runs Forge 47.4.18 with every mod unchanged in an embedded JVM, adds a GraphQL API and ports EssentialsX, EssentialsXSpawn, LuckPerms and the Spigot world settings to Rust plugins.
tags: [server, golem, arclight, aosabi, rust, graphql, plugins, performance]
timestamp: 2026-11-07
---

# Golem, the Rust hybrid server

Golem replaces Arclight. It is a Rust program built on Aosabi that owns the server process, starts
the JVM inside itself and boots Minecraft 1.20.1 with Forge 47.4.18 and every mod of the pack
unchanged. Around the game it adds a GraphQL API, a plugin runtime in Rust and native code for the
hot paths of world generation. The Bukkit plugins the server uses today are ported to Golem plugins.

Every duration on this page is an estimate that nobody measured. The owner changed some rulings
later on 2026-11-06, and this page carries them: the license is the AGPL 3 only, the holder is
Florian Kleber, the tree follows REUSE, and commits are Conventional Commits in the form of the
kernel, signed off by the holder. [Decisions](decisions.md) lists each with its time, and
[Index](index.md) says what is built.

## Owner requirements

These come from the session of 2026-11-06 (claude.ai session `016dYAZDqtsLdVqTQQoTV4RM`).

- More performance for the modded server, with multicore use where possible.
- The server is written in Rust on Aosabi and replaces Arclight as a hybrid.
- Every Forge mod that works on Forge keeps working.
- A GraphQL API in addition to plugins.
- Bukkit plugins stay possible but must run more efficiently, and dropping the Bukkit layer is the
  better choice if it is clearly faster.
- The plugins in use are ported, and the Skylands stay as they are, because the datapack generator
  is already optimal.
- The project is called Golem and lives in the repository `skytekx/golem`.
- Quirks and security issues of the ported plugins are fixed during the port, not carried over.
- Every quirk and every security issue gets its own GitHub issue, and the pull request links all of
  them.
- Golem is licensed under the GNU AGPL 3 only, written into every source file, see
  [License](#license).
- Commits are Conventional Commits as Commitizen writes them, in the form of the Linux kernel, and
  the code follows the kernel style with SPDX license lines, see
  [Commit and code style](#commit-and-code-style).
- Test cases are Jupyter notebooks, as in Aosabi.
- Every commit is pushed to `skytekx/golem` as soon as it exists, so that no code is lost when a dev
  environment ends, for example because its usage runs out.
- Golem keeps its own OKF wiki.

## Why the game itself stays Java

Forge mods are not written against an API. They subclass vanilla classes, read their private fields
through access transformers and inject code into the middle of vanilla methods through Mixins. A
scan of all 216 server jars of `modrinth.index.json`, the 49 jars of `local-mods/`, the Forge
47.4.18 binary patches and the Arclight jar against the Mojang mappings of the 1.20.1 server gave
these numbers on 2026-11-06.

| Measured | Value |
|---|---|
| Top level vanilla classes in the 1.20.1 server | 3,568 (5,460 with inner classes) |
| Classes that Forge patches | 494 |
| Classes that Arclight patches | 507 |
| Classes that the mods patch with Mixins or access transformers | 855 |
| Classes that at least one of the three patches | 1,205 (34 percent) |
| Classes that nobody patches | 2,363 (66 percent) |
| Classes that Arclight alone patches | 164 |
| Mixin injection points into server side vanilla code | 2,569 |
| Mods that patch `ServerLevel`, `Entity`, `MinecraftServer` and `ChunkMap` | 26, 39, 15 and 11 |

A patched class must stay real Java bytecode with the exact fields and instructions the patches
target, or the mods fail to load. So Minecraft, Forge and the mods run unchanged inside a JVM that
Golem embeds. Golem moves to Rust what sits around the game, plus the unpatched hot spots.

The scan only sees declared patches. It misses Mixins that a mod switches off at runtime and access
through reflection. The scripts and results were kept in the session scratchpad and are not in the
repo yet.

## Architecture

```text
Golem process (Rust, an Aosabi service)
  GraphQL API          queries, mutations, subscriptions, auth, TLS
  Plugin runtime       Golem plugins in Rust: essentials, spawn, permissions, world settings
  Native library       noise and density functions, region file compression
  JVM host             starts and stops the JVM through the JNI invocation API
    Embedded JVM (Java 17)
      Minecraft 1.20.1 and Forge 47.4.18, unchanged
      Every mod of the pack, unchanged
      Golem bridge mod (Java): Forge events to Rust, actions from Rust on the server thread
```

The parts in detail.

- The JVM host starts Java 17 in the Golem process with the same arguments that Forge's
  `unix_args.txt` gives the stock launcher and calls `cpw.mods.bootstraplauncher.BootstrapLauncher`
  on a dedicated thread. On macOS the JVM must not run on the main thread. The host registers the
  native methods of the bridge mod itself, so no separate library has to be loaded.
- The stop path keeps the rule of [Arclight
  runtime](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/server/arclight.md):
  Golem never stops the server with a signal. It queues `save-all flush` and `stop` on the server
  thread and waits until the JVM has returned. Golem passes `-Xrs` so that the JVM leaves the signal
  handling to the host.
- The bridge mod is a small Forge mod. It listens on the Forge event bus for joins, leaves, chat,
  commands, deaths, respawns, block interaction and server ticks and hands them to Rust. It drains a
  queue of actions from Rust at the end of every server tick, so Rust never touches game state from
  another thread.
- The plugin runtime calls Golem plugins on the server thread for events that can be cancelled or
  changed, with a time budget per call that the API reports. Forge 47 fires the chat event on a
  worker thread beside the server thread, so the runtime runs one call at a time behind a lock, as
  [Plugins](plugins/index.md) says. Plugins that only observe use the
  subscriptions of the GraphQL API instead.
- The GraphQL API runs on Aosabi and replaces RCON for operators and tools. It binds to localhost
  and reaches the outside only through the Cloudflare Tunnel with authentication through
  `aosabi-auth`. RCON stays available during the switch.
- The native library holds the world generation work of [Native acceleration](#native-acceleration).

## Where the code lives

Golem lives in its own repository, `skytekx/golem`, as the owner decided on 2026-11-06. It names
Aosabi by path or git as long as the Aosabi crates are not on a registry. Aosabi promises no unsafe
block in its own code and puts embedded targets first ([Aosabi
principles](https://github.com/netsnek/aosabi/blob/main/Documentation/principles.md)). A JVM host
needs unsafe JNI calls and only runs on servers, so it does not belong into the Aosabi core. The
bridge mod lives in the same repository as a Gradle subproject.

## License

Golem is licensed under the GNU Affero General Public License 3.0 only, SPDX `AGPL-3.0-only`, as the
owner decided on 2026-11-06 after a comparison with the GPL 2, the GPL 3 and the LGPL, and then
narrowed from "or later" to "only" the same day. The copyright holder is Florian Kleber, reached at
`opensource@skytekx.com`. [Documentation/licensing.md](../Documentation/licensing.md) holds the
rules as they are built.

- The AGPL is the only one of them that reaches a server that is run but never distributed. Whoever
  runs a changed Golem for players must offer those players its source. Under the GPL 2 or 3 the
  changes could stay private.
- The AGPL 3 takes Apache-2.0 code, so Aosabi works as it is. Aosabi is MIT, but some of its files
  are Apache-2.0, the code it takes from Pylon in `aosabi-core`, `aosabi-serve`, `aosabi-query` and
  `aosabi-build` ([Aosabi
  licensing](https://github.com/netsnek/aosabi/blob/main/Documentation/licensing.md)), and its TLS
  stack uses `ring`, which is under Apache-2.0 and ISC terms. A license that is GPL 2 only could not
  take those parts.
- Forge is LGPL-2.1 and LuckPerms is MIT. Both may be combined with AGPL code.
- EssentialsX is GPL-3.0, which is compatible, but Golem still does not copy or translate its code.
  The port is written from the documented behaviour and from tests against the running plugin.
  Arclight and CraftBukkit code is not used either.
- Golem never ships Minecraft. The server jar comes from Mojang's download as today.
- "Only" keeps Golem on the AGPL 3 of 2007, so a later AGPL of the Free Software Foundation does
  not apply to Golem by itself.

Golem follows its own license on the SkyTekx server. The command `/golem source` names the
repository and the exact commit that runs, so every player can get the source of the running
version. The server list text and the field `server.source` of the GraphQL API will name them as
well once they are built.

The license is written into the code itself.

- The file `LICENSE` holds the full text of the AGPL 3 from gnu.org, unchanged.
- Every source file names the license on its first line with an SPDX identifier, as the Linux kernel
  does, and carries no other license text. A file that cannot hold a comment, such as JSON or an
  image, is covered by an entry in `REUSE.toml`.

<!-- REUSE-IgnoreStart -->

```text
// SPDX-License-Identifier: AGPL-3.0-only             Rust, Java, C, JavaScript
# SPDX-License-Identifier: AGPL-3.0-only              TOML, YAML, shell, Python
<!-- SPDX-License-Identifier: AGPL-3.0-only -->       Markdown outside the wiki
```

<!-- REUSE-IgnoreEnd -->

Each SPDX line is followed by the `SPDX-FileCopyrightText` line of REUSE that names the holder.

- The `license` field of every `Cargo.toml` and the Gradle build of the bridge mod say
  `AGPL-3.0-only`. The wiki pages carry both tags as YAML comments right after the opening `---` of
  their front matter, so that REUSE reads them.
- The folder `LICENSES/` holds the text of every license a dependency or a copied file brings, named
  by its SPDX identifier, and `NOTICE` names each such project.
- CI runs `reuse lint` and the license lint of Golem so that no file lacks its SPDX lines, and
  `cargo deny` so that no dependency has an incompatible license.

A later ruling of the same day: every Golem plugin that ports a Bukkit plugin keeps the license of
that plugin and names its holders. So `golem-essentials` and `golem-essentials-spawn` are under the
GPL 3 or later of EssentialsX 2.22.0, and `golem-permissions` is under the MIT license of LuckPerms.
They combine with the AGPL code of Golem under section 13 of the GPL 3 and the AGPL 3.
`ports.toml` lists the ports, and [Documentation/licensing.md](../Documentation/licensing.md)
section 6 has the rule.

This is a reading of the licenses and not legal advice.

## Commit and code style

Golem follows the Linux kernel's rules for commits and code, from its
`Documentation/process/submitting-patches.rst` and `Documentation/process/coding-style.rst`, with
the subject of a Conventional Commit as Commitizen writes it. [CONTRIBUTING.md](../CONTRIBUTING.md)
holds the rules as they are checked.

### Commits

- One logical change per commit. On `main` every commit builds and passes the tests on its own, so
  that `git bisect` works.
- The subject is `type(scope): summary` and says what the commit does in the imperative, in lower
  case, without a period at the end and in 72 characters at most. Examples are
  `fix(essentials): refuse negative amounts in pay`, `feat(bridge): queue teleports on the server
  thread` and `feat(api): add the source field to Server`.
- A blank line follows, then a body wrapped at 72 columns that explains the problem first, then why
  this change solves it, then how. The diff already shows what changed.
- Trailers close the message in the kernel's form.

```text
fix(essentials): refuse negative amounts in pay

The pay command must never move money from the receiver to the sender.
Golem parses every amount as whole cents and refuses anything that is
not above zero before it touches an account.

Closes: https://github.com/skytekx/golem/issues/12
Signed-off-by: Florian Kleber <opensource@skytekx.com>
```

- Every commit carries `Signed-off-by` under the Developer Certificate of Origin, in the name of the
  holder, also a commit that an AI agent writes, as the owner ruled on 2026-11-06.
- A fix of an earlier Golem commit carries `Fixes: <first 12 characters of the hash> ("<subject>")`.
  A commit that solves an issue carries `Closes:` with the issue URL, and `Link:` points to further
  discussion. `Reported-by`, `Reviewed-by`, `Tested-by` and `Co-developed-by` are used as the kernel
  uses them.
- CI checks every subject, the body width and the trailers, and refuses commits without
  `Signed-off-by`.

### Code

- The first line of every file is its SPDX line, see [License](#license).
- Rust is formatted with `rustfmt` and passes `clippy` with warnings as errors, as the Rust code of
  the kernel does. Every `unsafe` block, which the JNI host needs, carries a `// SAFETY:` comment
  that says why it is sound, as the kernel requires.
- C, if any is ever needed, follows the kernel coding style with tabs of 8 columns and the kernel's
  `.clang-format`. The Java of the bridge mod takes the same rules where they fit: tabs, K&R braces,
  lines of at most 100 columns, short functions and no clever tricks.
- Comments say why, not what. Names are plain words, not abbreviations.

## Repository rules

These hold for everyone who works on `skytekx/golem`, human or agent.

- Push every commit at once, the same minute it is made. This is the most important rule of the
  repository. Dev environments end without warning: a cloud container is reclaimed, a session times
  out, or the usage or credit of an agent runs out in the middle of the work. Whatever is only local
  at that moment is gone.
- Commit often, at every step that works or that would hurt to redo, not only at the end of a task.
  Uncommitted work is lost the same way.
- Work happens on a personal work branch that is pushed after every commit. On that branch commits
  may be work in progress that does not build yet. Before a merge into `main` the branch is cleaned
  into commits of [Commit and code style](#commit-and-code-style), and only then may it be force
  pushed with `--force-with-lease`, and only on that own branch.
- If a push fails because of the network, retry it at once and keep retrying with growing pauses. A
  push that still fails is the first thing to report, before any other work goes on.
- An agent ends no session and no answer with an unpushed commit or uncommitted changes. Before it
  stops it checks that `git status` is clean and that `git log @{u}..` is empty.
- The repository ships an optional `post-commit` hook in `tools/hooks/` that pushes after every
  commit, for people who want it automatic.
- Keep the OKF wiki in `okf/` of the Golem repository, in the same Open Knowledge Format as this
  wiki: front matter with `type`, `title`, `description`, `tags` and `timestamp`, an `index.md` with
  the state and a page map, a change log, a decisions page and open items. A change that alters
  behaviour, config, the API, a ported command or a plugin's data updates the matching wiki page in
  the same pull request. Every quirk and security issue of [Quirks and
  security](#quirks-and-security) gets a line on the wiki page of its plugin with the link to its
  issue.
- The Golem wiki holds the design and the operation of Golem itself. This wiki keeps the pages about
  the SkyTekx 3 server and links to the Golem wiki.

## Quirks and security

The port is a chance to fix what the plugins do wrong today. Golem does not copy a behaviour only
because EssentialsX, LuckPerms, Spigot or Arclight have it.

- A quirk is a behaviour that surprises players or operators, breaks a documented intent or needs a
  workaround. A security issue is anything that lets a player or an outsider get access, data,
  items, money or control they should not have, or take the server down.
- Every quirk and every security issue found while porting gets its own GitHub issue in
  `skytekx/golem` before the fix, with the label `quirk` or `security`. The issue states the old
  behaviour, with evidence such as a log line, a config line or a test against the running plugin,
  and the behaviour Golem has instead.
- Every pull request that ports or changes a plugin lists all issues it fixes in a section of its
  body, one link per issue with `Fixes #n`. A pull request without that section does not merge. A
  pull request that found no quirk and no security issue says so in that section.
- As long as the live server still runs EssentialsX and the repository is public, a security issue
  that is exploitable on the live server is filed as a private security advisory of the repository
  instead of a public issue. The pull request links the advisory in the same section, and the
  advisory is published after the switch.

These are known at the start. Each one gets its issue when the work begins.

| Area | Quirk or security issue today | Golem |
|---|---|---|
| Permissions | The option `use-bukkit-permissions: true` makes the `player-commands` list of Essentials do nothing, so the config says one thing and the server does another ([Plugins](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/modpack/plugins.md#permissions)) | One permission system with one source of truth |
| Permissions | LuckPerms and its 39 nodes exist only in an H2 database on the Mac mini, so a new host has none ([Plugins](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/modpack/plugins.md#open-points)) | Groups and nodes as a tracked config file that seeds the database |
| Homes | Without `home.bed`, `/home` of a player with no home ignores the bed and goes to the spawn ([Plugins](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/modpack/plugins.md#permissions)) | The bed comes first by default |
| Data | The player data of Essentials sits in `plugins/`, which git does not ignore ([Plugins](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/modpack/plugins.md#files)) | The Golem store lives outside the tracked tree and is ignored |
| Telemetry | bStats and the update check are on ([Plugins](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/modpack/plugins.md#metrics-and-update-checks)) | No telemetry |
| Entities | Spigot's activation range left items hanging in the air until misc was set to 0 ([Floating items](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/server/arclight.md#floating-items)) | Items are exempt by design |
| Stop path | A signal stop and Arclight's shutdown thread could work on chunks at the same time ([Arclight runtime](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/server/arclight.md)) | Golem stops only through the server thread |
| Remote control | RCON sends its password in clear text and must never be exposed | The GraphQL API with authentication and TLS replaces it |

The port also checks these classes of issues and files what it finds: chat and name formatting codes
that fake system messages, spam through `tpa`, `msg` and `mail` without limits, money that goes
negative or rounds wrong in `pay` and `eco`, teleports into unsafe or unloaded places, and API calls
without rate limits.

## The GraphQL API

A later ruling of 2026-11-06 makes this schema the plugin API of Golem: a plugin reads the game with
queries, changes it with mutations and hears events through subscriptions, and the schema covers an
equivalent of the whole Bukkit API, see [Bukkit plugins](#bukkit-plugins). The schema below is the
first sketch for operators and tools, before that ruling. Plugins call it in process through the
executor of Aosabi, on their own thread, and clients outside the process reach it over graphtp.

A first schema, to be refined while the bridge grows.

```graphql
type Query {
  server: Server!
  players: [Player!]!
  player(name: String!): Player
  dimensions: [Dimension!]!
  plugins: [Plugin!]!
  permissions(group: String!): [String!]!
}

type Mutation {
  runCommand(command: String!): CommandResult!
  teleport(player: String!, dimension: String!, x: Float!, y: Float!, z: Float!): Boolean!
  setGameMode(player: String!, mode: GameMode!): Boolean!
  kick(player: String!, reason: String): Boolean!
  setPermission(group: String!, node: String!, value: Boolean!): Boolean!
  saveAll: Boolean!
  stop: Boolean!
}

type Subscription {
  chat: ChatMessage!
  joins: Player!
  leaves: Player!
  deaths: DeathEvent!
  ticks: TickStats!
}
```

The type `Server` carries TPS and MSPT, the uptime, the Forge and mod versions and the time budget
of every plugin. Aosabi sends subscriptions as server sent events over HTTP, see the Aosabi Rust
guide, section 11.

## Bukkit plugins

A later ruling of 2026-11-06 replaces the scope of this section: the Golem API covers an equivalent
of the whole Bukkit API, Spigot included, so that every Bukkit or Spigot plugin can be converted to
a Golem plugin, not only the ones in use. The reference is `spigot-api` 1.20.1 R0.1, the last build
of 2023-09-21, with 1,247 types, 7,942 public methods, 5,900 fields and constants and 268 events. A
coverage check maps every part of it to its equivalent in Golem.

A further ruling of the same day adds a Bukkit layer on the Golem API, so that Bukkit and Spigot
plugins also run unchanged, and every plugin must work. The layer is Java in the JVM of Forge and
reaches the Golem API in process. It covers three levels:

1. The public API of Bukkit and Spigot, mapped through the coverage check.
2. The internals of CraftBukkit that plugins reach by reflection or by name, such as the
   permissible field `perm` of `CraftHumanEntity` that LuckPerms 5.5.71 replaces
   (`PermissibleInjector.java:66`), the command map and its known commands. The Golem API offers
   what they need, and the layer gives them the shapes the plugins expect.
3. The classes of Minecraft that plugins call by their Spigot names, which the layer maps onto the
   classes of Forge in the same JVM.

A survey of the plugins that run on Spigot 1.20.1 ranks the internals they reach, so the layer is
built in the order that makes the most plugins work, see [Bukkit layer](bukkit-layer.md).

The text below is the plan before those rulings.

The Bukkit plugins in use are ported, so Golem needs no Bukkit layer for this server. A Bukkit layer
stays an option for later. It has to be Java, because plugins are Java, and it would build Bukkit
events only for event types that a plugin listens to and create the Bukkit wrapper objects only when
a plugin asks for them. That is 6 to 12 months of work and only worth it if the owner wants other
Bukkit plugins. The further ruling above replaces this paragraph: the layer is wanted, and every
plugin must work through it.

Dropping Bukkit is likely to pay off more through the mods it allows than through Arclight's own
overhead. Arclight does not appear as a source in the worldgen profile of
[Performance](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/server/performance.md),
but Radium crashed Arclight on 2026-06-27
([Performance](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/server/performance.md)).
Without Arclight the server can try Radium or Canary and ServerCore again. The comparison under
[Verification](#verification) measures both effects before the switch.

## Plugin port

The plugins come from
[Plugins](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/modpack/plugins.md). The
Golem plugins keep the command names and aliases players know, so nobody has to learn new commands.

### EssentialsX 2.22.0

The default group on the Mac mini holds 39 EssentialsX nodes. Admin commands stay with operators.
Which admin commands operators really use is not known. The usage audit under
[Before the port](#before-the-port) answers that before the admin list is built.

| Area | Commands and nodes | Golem plugin |
|---|---|---|
| Teleport requests | Commands `tpa`, `tpahere`, `tpaccept`, `tpdeny`, `tpacancel`, `tpauto` | Plugin `golem-essentials` with a request table, a timeout and a cooldown |
| Return | Nodes `back` and `back.ondeath` | Last location per player, written on teleport and on death through the bridge |
| Homes | Commands `home`, `home.bed`, `sethome`, `sethome.multiple`, `delhome` | Homes per player with a limit per group. A player without a home goes to the bed and then to the spawn, as `spawn-if-no-home` does today |
| Warps | Commands `warp`, `warp.list`, admin `setwarp` and `delwarp` | Warps with an optional node per warp |
| Messages | Commands `msg`, `mail`, `mail.send`, `ignore`, `me`, `helpop` | Private messages, reply, offline mail, an ignore list. Ops get `helpop` messages |
| Info | Commands `list`, `seen`, `realname`, `motd`, `rules`, `help`, `ping`, `getpos`, `compass`, `depth` | Read only commands. The commands `motd` and `rules` read text files |
| Status | Command `afk` | AFK flag with automatic AFK after idle time and a mark in the player list |
| Other | Command `suicide` | Kills the player through the vanilla damage source so mods see a normal death |
| Economy | Commands `balance`, `balancetop`, `pay`, `worth`, admin `eco` | Accounts per player with the values of `worth.yml`, exposed through the GraphQL API |
| Signs | Node `signs.use.*` | Only built if the sign audit finds Essentials signs in the world |
| Admin | Command `gm` and its forms `gmc`, `gms`, `gma`, `gmsp`, and the others the audit finds | Thin wrappers over vanilla commands where vanilla has one |

The kits of `kits.yml` are the sample kits and no group may use them, so they are not ported. The
random teleport of `tpr.yml` is not in the default group and is only ported if the audit shows use.

### EssentialsXSpawn 2.22.0

The port is `spawn` and `setspawn` with the alias `espawn`. Both listeners are off today
(`respawn-listener-priority: none` and `spawn-join-listener-priority: none`), so respawns and first
joins stay vanilla and Golem adds no listener for them. Until an operator sets a spawn, `spawn`
leads to the world spawn of the Overworld.

### LuckPerms 5.5.71

The port is `golem-permissions` with groups, inheritance, nodes with wildcards, values per player
and a store in SQLite next to the world. It registers itself as the permission handler of Forge's
PermissionAPI, so mods that ask Forge for permissions see the same groups. Operators change groups
in game with `/perm`, over the GraphQL API, or during the switch over RCON.

### Telemetry

bStats, the MCStats leftover and the EssentialsX update check are not ported. That follows the
owner's no telemetry rule that
[Plugins](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/modpack/plugins.md#metrics-and-update-checks)
names.

### Data migration

A migration tool in Golem reads the plugin data once with the server stopped and writes the Golem
store.

| Source on the Mac mini | Target |
|---|---|
| Files `plugins/Essentials/userdata/*.yml` with homes, last and logout location, balance, mail, ignore list, nickname | Players, homes, back locations, accounts, mail and ignore lists |
| Folder `plugins/Essentials/warps/` | Warps |
| The spawn of EssentialsXSpawn | Spawn |
| Files `plugins/Essentials/worth.yml`, `motd.txt` and the rules text | Golem config and text files |
| LuckPerms export from `lp export` | Groups, nodes and player values |

The tool prints counts per kind and fails on any entry it cannot place. The player data holds
personal data, so it stays out of git as today, see
[Secrets](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/server/secrets.md).

EssentialsX holds values that the store does not take as they are, and
[Store](store.md#data-from-essentialsx) gives the rules for them. A balance with a fraction of a
cent rounds half to even to whole cents, the account opens with the rounded balance as its starting
balance, and the report names every account that changed. A balance below zero is refused and named
with its debt, which an operator settles in EssentialsX before the migration runs again, see issue
[#106](https://github.com/skytekx/golem/issues/106). The names of homes, warps and spawn groups that
EssentialsX folded go over as they are, and each port folds a name as its plugin does, see issue
[#107](https://github.com/skytekx/golem/issues/107).

## Spigot world settings

Arclight applies `spigot.yml` and `bukkit.yml`. Most values there equal vanilla and need no port.
That holds for the spawn limits and `ticks-per` of `bukkit.yml`, the growth modifiers, the hunger
values, the despawn rates, the hopper values and the structure seeds. These differ from vanilla and
move into the plugin `golem-world`.

| Setting | Value today | Port |
|---|---|---|
| Entity activation range | Animals 32, monsters 32, raiders 48, misc 0, inactive villagers tick | Bridge hook on entity ticks with the same ranges. Misc stays at 0 so that items never skip ticks, see [Floating items](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/server/arclight.md#floating-items) |
| Entity tracking range | Players 48, animals 48, monsters 48, misc 32, display 128, other 64 | Bridge hook on the tracking range per entity type |
| Merge radius | Items 0.5, experience 3.0 | Experience merge radius through the bridge. Items match vanilla |
| Mob spawn range | 6 chunks | Bridge hook on the natural spawn radius |
| Max tick time | 50 ms for block entities and entities | Ported only if the load test shows that it ever triggers |
| Max TNT per tick | 100 | Bridge hook |

Before the switch a pregenerated test region is compared between Arclight and Golem to prove that
the structure seeds of `spigot.yml` were the vanilla salts. If a structure moves, Golem sets the
salts.

## Native acceleration

This part follows the hot spots of
[Performance](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/server/performance.md).
Chunk generation spends 50 to 62 percent of its time in vanilla code, most of it in terrain noise.
The scan shows that no Forge patch, no Arclight patch and no mod touches `SimplexNoise`,
`ImprovedNoise`, `NormalNoise`, `Aquifer`, `Mth` or `RegionFile`.

- Noise in Rust. The Java classes keep their exact signatures and call the Rust library in batches
  of a whole chunk. Results must match Java bit for bit, so the Rust code avoids fused multiply add
  and fast math. `PerlinNoise` is patched by Aether Redux and keeps its patched method in Java.
- Density functions compiled to Rust per dimension. Types that mods add, such as those of Draconic
  Evolution, stay Java callbacks. The Skylands datapack stays the single source and runs through the
  compiled path without changes.
- Region file compression in Rust with libdeflate, in the same format. `NbtIo` is patched by Forge
  and stays Java.
- Pathfinding in Rust only if the load test shows that it costs time. `PathFinder` is unpatched but
  reads world state from patched classes all the time, so it may not pay off.

C2MEF, the Forge port of C2ME, spreads chunk work over several cores and stacks with the native
code. It is tested in the measurement step. The expected gain of the native code is roughly 1.2 to
1.6 times faster chunk generation per core. That number is a judgement, not a measurement.

## Verification

- A comparison with the same world copy, the same pregen and 20 simulated players: Arclight as
  today, plain Forge, plain Forge with Radium or Canary and ServerCore, and later Golem. Each run
  records Spark profiles, GC logs, MSPT and chunks per second.
- Golden samples for the plugin port. Scenarios such as `tpa` with timeout, `back` after death,
  homes across dimensions, mail to an offline player and `pay` run on Arclight with EssentialsX and
  on Golem, and the resulting positions, messages and stored data must match.
- Golden samples for the native code. A recorder logs inputs and outputs of the target methods
  during real generation in every dimension, and a pregenerated region must be identical block for
  block between vanilla and Golem.
- A clean Golem boot reaches `Done` with 0 FATAL lines and no crash report. The Essentials warning
  about the hybrid server leaves the boot error baseline of [Logs and
  crashes](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/server/logs-and-crashes.md#boot-error-baseline).

## Work plan

Two engineers. Durations in weeks of one engineer.

| Work | Content | Effort |
|---|---|---|
| Before the port | Load test setup, the four way comparison, command usage audit from the Mac mini logs, sign audit of the world | 2 to 3 |
| JVM host | Golem boots unchanged Forge with the full mod set and the Halloween world, stop path, logs, crash recovery, launchd and systemd units | 3 to 4 |
| Bridge mod | Events to Rust, action queue on the server thread, native method registration | 3 to 4 |
| GraphQL API | Schema above, auth, server sent events, the Cloudflare Tunnel route | 3 to 5 |
| Plugin runtime | Rust plugin trait, server thread calls with time budgets, config and SQLite store | 3 to 4 |
| Permissions | Plugin `golem-permissions`, Forge PermissionAPI handler, LuckPerms import | 3 to 4 |
| Essentials port | Plugin `golem-essentials` and the spawn commands, economy, migration tool, golden sample scenarios, the issues of [Quirks and security](#quirks-and-security) | 7 to 10 |
| World settings | Plugin `golem-world` with activation range, tracking range and the other hooks | 2 to 3 |
| Switch | Test copy run, migration on the Mac mini, launchers, docs in this wiki | 2 to 3 |
| Native acceleration | Golden sample recorder, noise, density functions, region files | 20 to 32 |

Golem without the native code is 28 to 40 weeks of work, about 4 to 5 months with two engineers. The
native code runs in parallel and ends about 8 to 10 months after the start.

### Before the port

- Grep the Mac mini logs for `issued server command` to list the Essentials commands players and
  operators really use.
- Scan the world's region files for signs whose first line is an Essentials sign tag such as
  `[Buy]`, `[Sell]`, `[Warp]` or `[Balance]`.
- Run the four way comparison above and record the numbers on
  [Performance](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/server/performance.md).

## Risks

- Forge and some mods behave differently when Arclight is missing. The comparison run catches this.
- Mods that ask Forge's PermissionAPI may expect nodes that `golem-permissions` must list first.
- A crash in the JVM takes the Golem process down with it. The launchd and systemd units restart the
  whole process, as they do today.
- The JNI boundary costs time per call. All native calls run in batches, and the golden sample run
  also measures speed.
- Golem stays bound to Minecraft 1.20.1 and Forge 47. A move to another version means new scans and
  a new bridge.

## Open decisions

- Whether `skytekx/golem` is private until the switch, which would let security issues be public
  issues there from the start.
- Whether a Bukkit layer is ever wanted for plugins beyond the ported ones. The further ruling of
  2026-11-06 answers it, see [Bukkit plugins](#bukkit-plugins).
- Whether the economy of EssentialsX is in use at all, which the usage audit decides.

Related: [Arclight
runtime](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/server/arclight.md),
[Plugins](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/modpack/plugins.md),
[Performance and worldgen
profiling](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/server/performance.md),
[Mac mini](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/server/macos.md), [Logs
and
crashes](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/server/logs-and-crashes.md).
