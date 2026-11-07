---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Spec
title: Permissions
description: The permissions of Golem in place of LuckPerms 5.5.71, with the format of the tracked configuration, the rules of resolution in their order, the seed of SkyTekx and how it maps from LuckPerms, and the quirks of the live server that the one source of truth fixes.
tags: [plugins, permissions, luckperms, essentials, config, quirks]
timestamp: 2026-11-06
---

# Permissions

The crate `golem-permissions` is the one permission system of Golem. It takes the place of
LuckPerms 5.5.71 and of the permission switches of EssentialsX. As the port of LuckPerms it keeps
the MIT license of LuckPerms and names its holders, lucko (Luck) and the LuckPerms contributors,
beside the holder of Golem, see [Licensing](../../Documentation/licensing.md). The groups and nodes are a tracked
TOML file, [config/permissions.toml](../../config/permissions.toml), and every decision names the
rule that made it. A plugin asks it whether a player holds a node, and so will the handler of the
Forge PermissionAPI. The store in SQLite comes later and is seeded from the same file. Today the
crate reads the TOML.

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
| `groups.<name>` | `weight` | Integer of 32 bits | 0 | The rank of the group, a higher weight first |
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

On Bukkit, LuckPerms also answers a node that nothing names with the default of the plugin and,
for an operator, with true (lines 80 to 85 of the same file). Golem knows no operator in its
permissions and no default of a plugin. An undefined node is undefined, and the group `admin` of
the seed stands for the operators.

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
| `Permissions::groups_of(uuid)` | The groups of a player in the search order, each with weight, depth and rank |
| `Permissions::group_nodes(name)` | The nodes of a group as the file writes them, a deny with its `-` |
| `Permissions::group_names()` | The names of the groups, sorted |
| `DEFAULT_GROUP` | The name `default` |

`golem-devtool permissions --config <file> --player <uuid> --node <node>...` prints
`{"decisions": [{"node", "value", "reason"}]}`, and `golem-devtool permissions-groups --config
<file> --player <uuid>` prints the groups of a player with their weight, depth, rank and nodes. A
broken configuration goes to standard error with the exit status 1.

## The cost of a check

Loading builds one table per source, with the exact nodes, the prefixes of the wildcards and `*`
apart, and the search order of every player, shared by the players with the same groups. A check
then reads the UUID, finds the player in a hash map and asks each source at most one lookup per
segment of the node, until a source matches. `value` allocates nothing.

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

The nodes keep the names of EssentialsX, such as `essentials.home`, so that a node of a LuckPerms
export maps one to one and every ported command keeps the node a player and an operator know. The
commands of Golem itself take nodes under `golem.`, named after the command, such as `golem.perm`
for the command `perm` that will change the permissions.

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

The group `admin` has the weight 100, inherits `default` and grants `essentials.*` and `golem.*`.
The live server has no such group. Its admin commands stay with the operators, who hold them
through the defaults of Bukkit, as the same page says. The notebook checks that a player of the
group `default` holds each of the 39 nodes and none of the admin commands, such as
`essentials.setspawn`, `essentials.eco`, `essentials.gamemode` and `golem.perm`, and that a player
of the group `admin` holds them.

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
plugin needs no permission level in Brigadier, and the plugin checks the sender through
`golem-permissions`, as [Bridge](../bridge.md#commands) says, so the nodes of
`config/permissions.toml` and of the store seeded from it are the only place that grants a
command.

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
use spark would change without a word. When this crate answers the PermissionAPI through
`on_permission`, it answers every Boolean node of an online player as LuckPerms did on the live
server, a node that nothing names included, unless the owner decides and records which nodes go
back to the defaults of their mods. The notebook `04_plugins` holds a plugin that answers that way
to its answers, beside one that leaves the node to the default resolver.

Related: [Plan](../plan.md#luckperms-5571), [Bridge](../bridge.md), [Index](../index.md).
