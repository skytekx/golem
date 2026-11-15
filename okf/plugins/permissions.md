---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Spec
title: Permissions
description: The permissions of Golem in place of LuckPerms 5.5.71, with the format of the tracked configuration, the rules of resolution in their order, the seed of SkyTekx and how it maps from LuckPerms, and the quirks of the live server that the one source of truth fixes.
tags: [plugins, permissions, luckperms, essentials, config, quirks]
timestamp: 2026-11-16
---

# Permissions

The crate `golem-permissions` is the one permission system of Golem. It takes the place of LuckPerms
5.5.71 and of the permission switches of EssentialsX. As the port of LuckPerms it keeps the MIT
license of LuckPerms and names its holders, lucko (Luck) and the LuckPerms contributors, beside the
holder of Golem, see [Licensing](../../Documentation/licensing.md). The groups and nodes are a
tracked TOML file, [config/permissions.toml](../../config/permissions.toml), and every decision
names the rule that made it. The [store](../store.md#permissions) holds the same groups and nodes
and hands them to the crate as TOML of the same format, and `golem run` seeds it from
`config/permissions.toml` of the server, or from the seed of the Golem repository, as
[Store](../store.md#the-place-in-the-program-golem) says. The runtime of the plugins resolves every
check with the crate over the set of the store, which it loads again when the store changes, and a
plugin asks it whether a sender holds a node, as [the check of a sender](#the-check-of-a-sender)
says. The plugin `golem-permissions` of the runtime answers the PermissionAPI of Forge with it and
runs [the commands of LuckPerms](#the-commands-of-luckperms).

The notebook `tests/notebooks/06_permissions.ipynb` holds every rule on this page to a case.

## The configuration

```toml
[groups.default]
nodes = ["essentials.home", "essentials.signs.use.*"]

[groups.admin]
weight = 100
inherits = ["default"]
nodes = ["essentials.*", "golem.*"]

[players.00000000-0000-4000-8000-000000000001]
groups = ["admin"]
nodes = ["-essentials.suicide"]
```

| Table | Key | Type | Left out | What it is |
| --- | --- | --- | --- | --- |
| `groups.<name>` | `weight` | Integer of 32 bits | 0 | The rank of the group, a higher weight first. At 0 a group that a player holds directly shares the rank of `default`, see [where Golem differs](#where-golem-differs-from-luckperms) |
| `groups.<name>` | `inherits` | List of group names | Empty | The groups whose nodes the group takes on |
| `groups.<name>` | `nodes` | List of nodes | Empty | What the group grants and denies |
| `players.<uuid>` | `groups` | List of group names | Empty | The groups of the player besides the default group |
| `players.<uuid>` | `nodes` | List of nodes | Empty | What the player holds of their own |

- A node is segments joined by dots. A segment holds `a` to `z`, `0` to `9`, `_` and `-` and
  starts with one of the first three. `essentials.home` is a node, `Essentials.Home` and
  `essentials..home` are not.
- A node that ends in `.*` is a wildcard for every node below its prefix, at any depth.
  `essentials.signs.use.*` names `essentials.signs.use.buy`, but not `essentials.signs.use` itself.
  The node `*` alone names every node. A `*` anywhere else is refused.
- A leading `-` denies what the node names. This is the only form of a false value. There is no
  table form with `value = false`, so a node has one spelling and every deny stands in the list
  where a reader looks for it.
- A group name is one segment, such as `default` or `admin`.
- A player is their UUID in lower case with its four dashes. The configuration takes no name of a
  player, since a name changes and the UUID does not.
- The group `default` must exist, and every player is in it, also a player the file does not
  name. Naming it in the groups of a player is allowed and changes nothing.

Loading refuses a configuration with one line that names where it breaks a rule, as a TOML path or
as a line and a column. Nothing is skipped.

| What | Example of the message |
| --- | --- |
| Bad TOML | ``line 1, column 16: invalid table header, expected `.`, `]` `` |
| A key nobody knows | ``line 2, column 1: unknown field `weigth`, expected one of `weight`, `inherits`, `nodes` `` |
| A value of the wrong type | `line 2, column 10: invalid type: string "high", expected i32` |
| No default group | `groups.default: the group is missing, and every player is in it` |
| A bad group name | `groups.Admin: the group name "Admin" holds the character 'A', but a segment takes only a to z, 0 to 9, _ and -` |
| A bad node | `groups.default.nodes: the node "essentials.*.home" holds a * that is not a wildcard, which is * alone or .* at the end` |
| A node twice in one list | `groups.default.nodes: the node "essentials.home" is named twice` |
| An unknown group in `inherits` | `groups.admin.inherits: the group "moderator" is not defined` |
| A group twice in one list | `groups.admin.inherits: the group "default" is named twice` |
| An inheritance cycle | `groups.c.inherits: the inheritance a, b, c, a is a cycle` |
| A group that inherits itself | `groups.a.inherits: the group inherits from itself` |
| A player key that is no UUID | `players.steve: the key is not a UUID in lower case with its four dashes, such as 00000000-0000-4000-8000-000000000001` |
| An unknown group of a player | `players.<uuid>.groups: the group "vip" is not defined` |

The notebook holds 39 such refusals to their exact message.

## The rules of resolution

A check asks whether the player with a UUID holds one node. The rules decide in this order.

1. A check names a player by UUID, in either case, and one node with no wildcard and no `-`.
   Anything else is undefined, with a reason that says what is wrong.
2. The nodes of the player come first. When one of them matches, it decides, whatever the groups
   say.
3. Then the groups of the player, the default group among them, in ranks: a higher weight first,
   and at the same weight a closer group first. A group the player is in directly has depth 1, and
   every step of inheritance adds one, along the shortest path. The first rank in which any group
   matches decides, even when a group of a lower rank names the node more specifically.
4. Within one source, the player or one group, the most specific match decides: the node itself,
   then the wildcard with the longest prefix, then `*`. The order of the nodes in the file plays
   no part.
5. A wildcard `a.*` matches every node below `a` and not `a` itself, as in LuckPerms.
6. When one source grants and denies the same node, which is the only way to meet an equal
   specificity within one source, the deny wins.
7. Groups of the same weight and depth share a rank and do not outrank each other. Each decides by
   rules 4 to 6, and when they disagree, the deny wins, also against a grant that names the node
   more specifically. When they agree, the first of the rank by name gives the reason.
8. When no source matches, the node is undefined. A caller treats undefined as not granted, unless
   it has a default of its own, as a node of the Forge PermissionAPI has.

The reason of a decision is one of these sentences.

| Rule | Reason |
| --- | --- |
| 1 | `The player "steve" is not a UUID`, `The node "essentials.*" is a wildcard, and a check names one node` |
| 2 | `The player denies essentials.*` |
| 3, 4, 5 | `Group default grants essentials.signs.use.*` |
| 6 | `Group default grants and denies essentials.home, and the deny wins` |
| 7 | `Group blue denies essentials.fly and group red of the same rank grants essentials.fly, and the deny wins` |
| 8 | `No rule names essentials.fly` |

### Where Golem differs from LuckPerms

LuckPerms merges the nodes of the player and of every group they inherit into one map, then looks
for the node itself in that map and only then for a wildcard, in the source of its Bukkit
calculator (`BukkitCalculatorFactory.java` lines 59 and 70 on the master branch of
`LuckPerms/LuckPerms`, read on 2026-11-06). A node named exactly in any source therefore beats a
wildcard in every source. Golem asks the sources one by one, so a wildcard deny of a player takes
away a node that a group grants by name, and a group of a higher rank decides before a lower one.
The section on quirks below has the case.

A group without a weight has the weight 0 of the group `default`, and a player holds both at depth
1, so they share a rank, and by rule 7 a deny of `default` beats a grant of that group, also a grant
by name against a wildcard deny. With `-worldedit.*` in `default` and `worldedit.wand` in a group
`builder` without a weight, a player of `builder` is denied `worldedit.wand` with the reason
`Group default denies worldedit.* and group builder of the same rank grants worldedit.wand, and the
deny wins`. LuckPerms grants the node, since it finds the node itself before any wildcard. A group
that must win over a deny of `default` needs a weight above 0, also when it comes from a LuckPerms
export without a node `weight.<n>`.

On Bukkit, LuckPerms also answers a node that nothing names with the default of the plugin and,
for an operator, with true (lines 80 to 85 of the same file). The default of the plugin comes
first, so an operator gets false for a node that the `plugin.yml` of its plugin marks
`default: false`, such as `essentials.keepinv`. The rules of Golem know no operator and no default
of a plugin. An undefined node is undefined, and the group `admin` of the seed stands for the
operators, so it denies such nodes by name. `Permissions::has_permission` alone takes whether the
player is an operator, for the answer that LuckPerms gave on Bukkit, see [The API](#the-api).

The format has no contexts such as a world or a server, no expiry and no meta such as a prefix.
The importer of a LuckPerms export, which the data migration of the
[plan](../plan.md#data-migration) names, therefore refuses a node that carries them, with the node
it cannot place.

## The API

| Item | What it answers |
| --- | --- |
| `Permissions::from_toml(text)` | The permissions of a configuration, or the `ConfigError` that names where it breaks a rule |
| `Permissions::check(uuid, node)` | A `Decision` with `value`, true, false or none, and the `reason` |
| `Permissions::value(uuid, node)` | The value alone, without building the reason, for the server thread |
| `Permissions::has_permission(uuid, node, operator)` | What LuckPerms 5.5.71 answered `Player.hasPermission` on Bukkit for a node that no plugin declares. The node is lowercased by the rules of Unicode and looked up as LuckPerms looks it up, the text as it stands and then the wildcards above each of its dots, whatever characters it holds, so a wildcard also decides a node that the format refuses, such as one with a slash. A node that no rule names is true for an operator and false for every other player |
| `Permissions::groups_of(uuid)` | The groups of a player in the search order, each with weight, depth and rank |
| `Permissions::group_nodes(name)` | The nodes of a group as the file writes them, a deny with its `-` |
| `Permissions::group_names()` | The names of the groups, sorted |
| `Config`, `GroupConfig`, `PlayerConfig` | The configuration as data: every group with its weight, its parents and its nodes, and every player with their groups and nodes |
| `Permissions::from_config(config)` | The permissions of a `Config`, held to every rule as `from_toml` holds a file |
| `Permissions::config()` | The `Config` that the permissions were loaded from |
| `command::execute(permissions, label, arguments, host)` | The `Outcome` of a command of `/luckperms`: its lines, the entries it changed, the permissions after the change and the line of a failed save, see [The commands of LuckPerms](#the-commands-of-luckperms) |
| `DEFAULT_GROUP` | The name `default` |

`golem-devtool permissions --config <file> --player <uuid> --node <node>...` prints
`{"decisions": [{"node", "value", "reason", "hasPermission"}]}`, where `hasPermission` holds the
answer of `has_permission` for a player and for an operator, and
`golem-devtool permissions-groups --config <file> --player <uuid>` prints the groups of a player
with their weight, depth, rank and nodes. A broken configuration goes to standard error with the
exit status 1.

## The check of a sender

The runtime of the plugins holds a `Resolver`, the permissions of the store loaded into this crate,
and every plugin asks it through its context whether the sender of a command holds a node, with
`Context::check_permission`, which answers whether the sender holds it, whether a rule decided and
the reason, and `Context::has_permission`, which answers the first alone, for a player with one
call of `value`. The senders are those of [Plugins](index.md#commands).

| Sender | Holds | Reason |
| --- | --- | --- |
| The console and RCON | Every node | `The console holds every node` |
| Another source at the level 2 and more, such as a command block or a function | Every node | `The source @ runs at the level 2, and a source that is no player holds every node from the level 2` |
| Another source below the level 2 | No node | `The source Server runs at the level 1, and a source that is no player holds no node below the level 2` |
| A player | What the rules of resolution grant them, a node that no rule names not | The reason of the decision, such as `Group admin grants golem.*` |

- A player is judged by their nodes and groups alone. The player is the one who made the command
  run, so a command that a player types is judged by the nodes of that player, also when it runs as
  another entity through `execute as`. Their operator level and the level of the source play no
  part, so an operator of the game holds no node that the permissions do not grant them, and the
  group `admin` of the seed stands for the operators, as above. A plugin may add a rule of its own
  where no rule names a node, as the core plugin leaves its tools of operators with the operators of
  the game, see [Core plugin](core.md#the-command-golem).
- A source that is no player and runs a command as a player through `execute as`, such as the
  console or a command block, is another source at its own level, so it never holds a node of that
  player, and a source at the level 2 holds every node whoever it runs as. The level 2 is the level
  of the game masters of Minecraft, at which command blocks and functions run.
- A node that no rule names is not held, and `set` is false then, as `isPermissionSet` of Bukkit
  answers. A port whose upstream declares a node with the default true, such as
  `essentials.back.onteleport` of EssentialsX, grants it when no rule decided.
- A runtime without permissions, such as a test of a plugin, grants a player no node.

The program `golem` builds the resolver from the store after the seed, so a set of the store that
this crate refuses stops the start with `golem: the permissions of the store <path> do not load:
<reason>`, before the JVM exists. A change of the store moves its generation. The plugin
`golem-permissions` hands `Resolver::refresh` a look at the generation on every tick, which costs
one lock of the store, and `refresh` wakes the thread `golem-permissions`, which reads the set and
its generation under one lock of the store, loads it and swaps it in. A check never waits for that
reload and answers with the set the resolver holds until then. A set that this crate refuses, such
as one whose inheritance is a cycle, is logged once with its generation, `the permissions of the
store <path> at the generation 3 do not load, so the checks keep the permissions they hold:
<reason>`, and the checks keep the set they hold.

## The commands of LuckPerms

The module `command` of this crate is the port of the command `/luckperms` of LuckPerms 5.5.71 with
its aliases `/lp`, `/perm`, `/perms`, `/permission` and `/permissions`, as the `plugin.yml` of
LuckPerms names them, by which operators read and change the permissions in game and on the
console. The wiki of the live server names `lp group default permission set <node>` and `unset
<node>` over RCON and `/lp editor` in game as the ways its operators change the set. The commands
keep their names, the convenience aliases of LuckPerms, such as `/lp u Alex p set <node>` for
`/lp user Alex permission set <node>`, their nodes and their English texts, those of
`luckperms_en.properties` and `Message.java`, behind the prefix `[LP] `.

| Command | Node | What it does |
| --- | --- | --- |
| `/lp` | Any of them | The version, and the way to the help or why the sender may use none |
| `/lp help` and an unknown word | Any of them | The main commands the sender holds |
| `/lp user <user> info` | `luckperms.user.info` | The UUID, its kind, whether the player is online, and their parents |
| `/lp user <user> permission info [page]` | `luckperms.user.permission.info` | The nodes of the player, 19 to a page |
| `/lp user <user> permission set <node> [true\|false]` | `luckperms.user.permission.set` | Sets a node, in place of the node of the other value |
| `/lp user <user> permission unset <node>` | `luckperms.user.permission.unset` | Unsets a node, of either value |
| `/lp user <user> permission check <node>` | `luckperms.user.permission.check` | The nodes that name it, directly, inherited and as a wildcard, and the decision with its reason |
| `/lp user <user> permission clear` | `luckperms.user.permission.clear` | Unsets every node of the player |
| `/lp user <user> parent info [page]` | `luckperms.user.parent.info` | The groups of the player, `default` last |
| `/lp user <user> parent set <group>` | `luckperms.user.parent.set` | Leaves the player in that group alone, beside `default` |
| `/lp user <user> parent add <group>` | `luckperms.user.parent.add` | Adds a group |
| `/lp user <user> parent remove <group>` | `luckperms.user.parent.remove` | Removes a group |
| `/lp user <user> parent clear` | `luckperms.user.parent.clear` | Removes every group but `default` |
| `/lp user <user> clear` | `luckperms.user.clear` | Removes every node and every group |
| `/lp group <group> ...` | `luckperms.group.<subcommand>` | `info`, `permission`, `parent` and `clear` as for a user |
| `/lp group <group> setweight <weight>` | `luckperms.group.setweight` | Sets the weight |
| `/lp group <group> listmembers [page]` | `luckperms.group.listmembers` | The players and groups that inherit from the group, 15 to a page |
| `/lp creategroup <group> [weight]` | `luckperms.creategroup` | Creates a group, in lower case |
| `/lp deletegroup <group>` | `luckperms.deletegroup` | Deletes a group but `default` |
| `/lp listgroups [page]` | `luckperms.listgroups` | The groups by weight, then by name |

A sender may run a command when they hold its node by [the check of a
sender](#the-check-of-a-sender), so the console runs every command. The group `admin` of the seed
grants `luckperms.*`, so the console puts an operator into it with `lp user <name> parent add
admin`, and from then on that operator changes the permissions in game.

A command builds the change on the configuration that it is given, loads the result into this
crate with `Permissions::from_config`, which checks every rule of the format, and answers the
entries it changed and the permissions to install, so its caller writes the entries to the store
and installs the result at once. A change that breaks a rule changes nothing and answers `The
permissions of Golem refuse the change, so nothing changed: <reason>.`, such as a parent that closes
a cycle of inheritance. The module names no store and no runtime, so the crate keeps its license
and its callers. The plugin of the permissions in the runtime runs the commands, writes the entries
to the store and installs the result, so the next check sees the change, see
[Plugins](index.md#permissions).

Where the port differs from LuckPerms:

- Golem has no contexts, no expiry, no meta, no tracks, no log, no web editor and no import or
  export of LuckPerms, so a context in the arguments, such as `server=survival`, answers `The
  permissions of Golem have no contexts, so the port takes no server=survival.`, a node of meta,
  such as `prefix.10.x`, says that Golem does not hold it, a display name of `creategroup` answers
  that the groups of Golem have none, and every other command of LuckPerms, such as `/lp editor`,
  `/lp user <user> meta` or `permission settemp`, answers `Golem does not port /lp editor of
  LuckPerms 5.5.71, see okf/plugins/permissions.md.`. Every text names the context `global`, as
  LuckPerms names an empty set of contexts.
- The port has no option `commands-allow-op`, so a player runs a command by its node alone, whatever
  their operator level.
- The bridge sends a line as plain text, so the colors of LuckPerms are gone, and an entry of
  `permission info` writes a denied node with its leading `-`, as the configuration writes it, where
  LuckPerms shows it in red. The help and the usage name only the arguments that the port takes, so
  `[context...]` and the sort mode are gone.
- `user info` and `group info` show no contextual data, the prefix, the suffix and the meta of
  LuckPerms, which Golem does not hold, and a group without a weight shows the weight 0, where
  LuckPerms shows `None`, since the format holds a weight for every group.
- A node is held to [the format](#the-configuration), so a node that it refuses, such as
  `Essentials.Fly`, answers why, where LuckPerms takes any text. A node `group.<name>` adds or
  removes a parent, as in LuckPerms, but the group must exist, and a denied parent does not exist in
  Golem and is refused.
- The name of a group is one segment of the format, where LuckPerms takes any text of 1 to 36
  characters without a space, so `creategroup bad.name` answers `bad.name is not a valid group
  name.`. Every other command that names such a group answers as LuckPerms answers for a group that
  does not exist.
- The port asks for the subcommand of `permission` and of `parent` before it looks for the user or
  the group that the command names. So for a player whom the server does not know, `/lp user Steve
  permission`, `/lp user Steve permission nothing` and `/lp user Steve permission set` answer the
  list of the subcommands, `Command not recognised.` and the usage, where LuckPerms looks for the
  user first and answers that it could not be found, and the same holds for a group that does not
  exist. The name of a group has no bound of 36 characters, so `creategroup` makes a group with a
  longer name. And the help and the usage list the commands that the port takes and no other.
- Every player is in the group `default`, so `parent remove default` keeps it and says so, `parent
  info` names it last, and `listmembers` of `default` names every player with an entry of the
  configuration.
- `deletegroup` removes the group from every group and player that names it, since the store keeps
  no entry that names a group that is gone, where LuckPerms leaves such entries behind unless its
  flag `--update-parent-lists` is given. The port takes the flag and does the same without it, see
  issue [#231](https://github.com/skytekx/golem/issues/231).
- A user is a UUID, with or without its dashes, or the name of a player who is online or whom the
  store knows. The store knows a player once the port of EssentialsX records their join, so until
  then the command takes the UUID of a player who is offline.
- `/lp` names `Running LuckPerms v5.5.71, as ported to Golem.`, and `permission check` names the
  processor `golem-permissions` and the reason of the decision as its cause.
- The store takes a node and the name of a group of at most 256 bytes, see
  [Store](../store.md#refusals). A command with a longer one answers its line and then the line
  of LuckPerms for a failed save, the plugin logs why the store refused, and nothing changes.

`golem-devtool permissions-command <script.json>` runs commands on a configuration with a sender
and the players a server knows, and prints for each its lines, its changes, the configuration
after it and the line of a failed save, so `06_permissions` holds every command of the table above
to its texts and to its node without a store and without a JVM.

## The cost of a check

Loading builds one table per source, with the exact nodes, the prefixes of the wildcards and `*`
apart, and the search order of every player, shared by the players with the same groups. A check
then reads the UUID, finds the player in a hash map and asks each source at most one lookup per
segment of the node, until a source matches. `value` allocates nothing, also for a player or a
node that a check may not name, and the notebook counts the allocations of its checks to hold it to
that.

The notebook times the same check of a player in three groups in a configuration with that player
alone and in one with 200 groups and 5000 players. In two runs on 2026-11-06, in the cloud
container of the session with an Intel Xeon at 2.10 GHz, the debug build of the devtool took 3.1
and 3.5 microseconds per check in the small configuration and 4.0 and 3.5 microseconds in the
large one, 1.29 and 0.99 times as long. A release build took 138 nanoseconds per check of four
nodes of the seed for a player of the group default, measured with
`golem-devtool permissions --repeat 1000000` on the same machine and day.

Loading searches the inheritance for cycles with a stack of its own, so a chain of 100000 groups
loads, and the notebook holds a check to the end of such a chain. In the debug build each load of
that chain took about 2.4 seconds on the same machine and day.

## The seed of SkyTekx

[config/permissions.toml](../../config/permissions.toml) holds two groups and no player. The UUID
of a player is player data, which stays out of the repository, so the store says who is in the
group `admin`.

The nodes keep the names of EssentialsX, such as `essentials.home`, and of LuckPerms, such as
`luckperms.user.permission.set`, so that a node of a LuckPerms export maps one to one and every
ported command keeps the node a player and an operator know. The commands of Golem itself take
nodes under `golem.`, named after the command, such as `golem.player` for `/golem player`. The
command `perm` that changes the permissions is the alias `/perm` of the port of LuckPerms, so it
takes the nodes of LuckPerms.

The group `default` holds the 39 nodes that the group `default` of LuckPerms on the Mac mini holds,
as the wiki of the server lists them in
[Plugins](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/modpack/plugins.md#permissions),
each with the prefix `essentials.`.

| Area | Nodes |
| --- | --- |
| Teleport requests | `tpa`, `tpaccept`, `tpdeny`, `tpahere`, `tpacancel`, `tpauto` |
| Homes | `home`, `home.bed`, `sethome`, `sethome.multiple`, `delhome` |
| Return | `back`, `back.ondeath` |
| Spawn | `spawn` |
| Warps | `warp`, `warp.list` |
| Messages | `msg`, `mail`, `mail.send`, `ignore`, `me` |
| Status | `afk` |
| Information | `list`, `seen`, `realname`, `motd`, `rules`, `help`, `helpop`, `ping`, `getpos`, `compass`, `depth` |
| Other | `suicide` |
| Economy | `balance`, `balancetop`, `pay`, `worth` |
| Signs | `signs.use.*` |

The group `admin` has the weight 100, inherits `default` and grants `essentials.*`, `golem.*` and
`luckperms.*`, the nodes of the commands of LuckPerms that change the permissions.
The live server has no such group. Its admin commands stay with the operators, who hold them
through the defaults of Bukkit, as the same page says. An operator there holds none of the 23
nodes that the `plugin.yml` of EssentialsX 2.22.0 marks `default: false`, switches that a player
gets only by hand, so the group denies 22 of them by name, every one but `essentials.back.ondeath`,
which the group `default` grants. They are `afk.auto`, `balancetop.exclude`, `ban.exempt`,
`chat.ignoreexempt`, `exempt`, `home.compass`, `invsee.preventmodify`, `keepinv`, `keepxp`,
`kick.exempt`, `kit.exemptdelay`, `mute.exempt`, `near.exclude`, `nick.allowunsafe`,
`nick.hideprefix`, `sethome.bed`, `silentjoin`, `silentjoin.vanish`, `silentquit`, `sudo.exempt`,
`tempban.exempt` and `vanish.pickup`, each with the prefix `essentials.`. Golem has no child
permissions, so the children of `essentials.exempt` stand in the list by name. The notebook
checks that a player of the group `default` holds each of the 39 nodes and none of the admin
commands, such as `essentials.setspawn`, `essentials.eco`, `essentials.gamemode`, `golem.player`
and `luckperms.user.permission.set`,
that a player of the group `admin` holds them, and that the admin holds none of the 22 switches.

What nobody knows yet, and the comments of the file say so too:

- Whether the H2 database on the Mac mini holds exactly these 39 nodes. No export of it exists, so
  the seed is the set the wiki names. The same wiki counts 38 nodes in its open points (line 125
  of `okf/modpack/plugins.md`) and in its change log (line 369 of `okf/log.md`), both written
  before `essentials.home.bed` joined on 2026-11-02.
- Which admin commands the operators really use. The usage audit of the
  [plan](../plan.md#before-the-port) answers that, and the group `admin` may then grant less.
- How many homes `essentials.sethome.multiple` allows. That number is a setting of the
  `config.yml` of Essentials on the Mac mini, which the tracked tree of the server does not hold,
  so the port of the homes decides it.

### From LuckPerms

| In a LuckPerms export | In the configuration |
| --- | --- |
| A group | A table `groups.<name>` |
| A node with the value true | The node in `nodes` |
| A node with the value false | The node with a leading `-` in `nodes` |
| A node `group.<name>` of a group | The name in `inherits` |
| A node `group.<name>` of a user | The name in `groups` of the player, unless it is `default` |
| A node `weight.<n>` of a group | `weight = n` |
| A user | A table `players.<uuid>` |
| A node with a context, an expiry or meta | Refused by the importer, see above |

## Quirks of today

### The player commands of Essentials do nothing

The config of Essentials keeps `use-bukkit-permissions: true`, and its list `player-commands`,
which gives commands to every player, only takes effect when that option is false, as the server
wiki records in `okf/modpack/plugins.md` lines 55 to 59 of `skytekx3-1201-server`, from
`plugins/Essentials/config.yml` lines 265 to 269 and 351 to 354. The configuration says one thing
and the server does another.

Golem has one permission system and no switch that turns a second one on or off. A command of a
plugin needs no permission level in Brigadier, and the plugin judges the sender itself, as
[Bridge](../bridge.md#commands) says. A plugin checks the sender through `golem-permissions`, as
[the check of a sender](#the-check-of-a-sender) says, so the nodes of the store seeded from
`config/permissions.toml` are the only place that grants a command of a port to a player. The tools
of operators of `/golem` take the nodes `golem.<name>` that way and stay with the operators of the
game where no rule names the node, see [Core plugin](core.md#the-command-golem).

Issue: [#1](https://github.com/skytekx/golem/issues/1).

### The permissions live only in a database on one host

LuckPerms keeps the group `default` and its nodes in an H2 database in `plugins/LuckPerms/` on the
Mac mini (`okf/modpack/plugins.md` line 64), and a new host needs the jar and the nodes again
(line 125). Nobody can review the set in git, and the wiki does not agree with itself on its size,
39 nodes at line 65 and 38 at line 125.

Golem keeps the groups and nodes in the tracked file `config/permissions.toml`, which seeds the
store, and the notebook 06_permissions compares it with the list of the wiki on every run.

Issue: [#2](https://github.com/skytekx/golem/issues/2).

### A wildcard deny of a player loses to a node of a group

In LuckPerms, a node that a group grants by name beats a wildcard that the player denies, since
the exact lookup over every source comes before the wildcard lookup (`BukkitCalculatorFactory.java`
lines 59 and 70). An operator who takes `essentials.*` from a player with
`lp user <name> permission set essentials.* false` leaves them every command that the group
`default` names, `/home` among them.

Golem asks the nodes of the player before any group, so `-essentials.*` on a player denies
`essentials.home` with the reason `The player denies essentials.*`. The notebook holds the case in
its section on the player before the groups.

Issue: [#3](https://github.com/skytekx/golem/issues/3).

### Arclight forwards every permission of a mod to LuckPerms

The live server sets `forward-permission=true` in `arclight.conf` (line 31 of
`skytekx3-1201-server`), so Arclight wraps the handler of the Forge PermissionAPI that Forge takes,
`forge:default_handler` of `defaultconfigs/forge-server.toml` (line 17), and answers every Boolean
node of an online player with `hasPermission` of Bukkit. LuckPerms therefore decides every node
that a mod asks, such as `voicechat.speak` of Simple Voice Chat for every packet of a microphone:
a node that a group names as the group says, and a node that nothing names false, or true for an
operator. The default resolver of the node, which the mod meant as its default, is never asked.
Mekanism, Nature's Compass, Ars Magica Legacy, Balm, CC: Tweaked, JEI, spark and Simple Voice Chat
ask such nodes.

Under Golem the handler `golem:permissions` asks the plugins and falls to the default resolver of
the node when no plugin answers, as [Bridge](../bridge.md#permissions) says. A default resolver
often grants a node to every player or to an operator level, so who may speak in voice chat or
use spark would change without a word. So the plugin `golem-permissions` of the runtime answers
every Boolean node of an online player through `on_permission` as LuckPerms did on the live
server, with `Permissions::has_permission`, a node that nothing names included, and leaves a
player who is not online to the default resolver, as [Plugins](index.md#permissions) says, until
the owner decides and records which nodes go back to the defaults of their mods. The notebook
`04_plugins` holds the plugin to those answers on the server thread and beside it, and
`07_forge_boot` holds a selector of a player who is no operator, which the default resolver of
Forge refuses below the level 2, to the node that the console sets and unsets with the port of
LuckPerms.

Issue: [#105](https://github.com/skytekx/golem/issues/105).

Related: [Plan](../plan.md#luckperms-5571), [Bridge](../bridge.md), [Index](../index.md).
