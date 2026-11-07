---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Spec
title: Bridge
description: The contract between the Golem bridge mod in the JVM and the Rust of Golem, with the host class, its native methods, the game that Rust may ask, the permission handler of Forge, the wire form of the actions and the rules of the server thread.
tags: [bridge, jni, forge, contract]
timestamp: 2026-11-16
---

# Bridge

The bridge connects the game, which runs unchanged in the JVM, with the Rust of Golem. It has three
parts, and this page is the contract between them.

| Part | Where | What it is |
| --- | --- | --- |
| Host class | `bridge/host/`, the jar `golem-host.jar` | The Java class with the native methods and the interface `Game`, no Forge in them |
| Bridge mod | `bridge/mod/`, the jar `golem-bridge-<version>.jar` in `mods/` | A Forge mod that listens on the event bus and calls the host class |
| Natives | The crate `golem-bridge` | The Rust functions behind the native methods, registered by the host |

## How the parts find each other

The host class lives in its own jar so that it can sit on the module path of the JVM, outside the
layers that Forge builds. The JVM host of Golem adds `golem-host.jar` to the `--module-path` of the
launch. The jar has no `module-info.class` and names itself `com.skytekx.golem.host` with the
manifest attribute `Automatic-Module-Name`, so it becomes an automatic module of the boot layer that
exports its package. Forge mods are automatic modules of the game layer, which read every module of
the layers below them, so the bridge mod sees the host class.

The JVM host creates the JVM, finds `com/skytekx/golem/host/Host` through the system class loader,
registers the native methods of this page on it with `RegisterNatives` and only then calls the main
class of Forge. No library is loaded from Java, and no symbol is looked up by name.

A spike on 2026-11-06 proved the path on Forge 47.4.18 and Temurin 17.0.21: a mod built against the
Forge jars alone called a native method of such a class from its constructor and got the answer of
the host.

## The host class

```java
package com.skytekx.golem.host;

public final class Host {
	/** True when a Golem host registered the native methods, false on a stock Forge. */
	public static boolean available();

	public static native String version();
	public static native void connect(Game game);
	public static native void serverStarting(String forgeVersion, String minecraftVersion);
	public static native void serverStarted();
	public static native void serverStopping();
	public static native void serverStopped();
	public static native void tick(long nanos);
	public static native void playerJoined(String uuid, String name);
	public static native void playerLeft(String uuid, String name);
	public static native boolean chat(String uuid, String name, String message);
	public static native void death(String uuid, String name, String cause, String dimension,
			double x, double y, double z, float yaw, float pitch);
	public static native void respawn(String uuid, String name, String dimension,
			boolean endConquered);
	public static native void teleported(long id, String uuid, String from, String to);
	public static native void teleportFailed(long id, String uuid, String reason);
	public static native String[] commandNames();
	public static native String[] command(String name, String senderUuid, String senderName,
			int level, String arguments);
	public static native int permission(String uuid, String node);
	public static native String[] drain();

	public static final int PERMISSION_TRUE = 1;
	public static final int PERMISSION_FALSE = 0;
	public static final int PERMISSION_NONE = -1;
}
```

`available()` is plain Java. It calls `version()` once and answers false when that throws an
`UnsatisfiedLinkError`, so the mod switches itself off with one warning on a server that Golem
does not run, instead of failing on every event.

| Native method | Called by the mod | Answer |
| --- | --- | --- |
| `version()` | Once, through `available()`, while Forge constructs the mod | The version of Golem, such as `0.1.0` |
| `connect(game)` | Once, right after `available()`, while Forge constructs the mod | None. Golem keeps the game, see [The game](#the-game) |
| `serverStarting(forge, minecraft)` | On `ServerStartingEvent`, which Forge fires at the end of `initServer`, once the worlds are loaded | None |
| `serverStarted()` | On `ServerStartedEvent` | None |
| `serverStopping()` | On `ServerStoppingEvent` | None |
| `serverStopped()` | On `ServerStoppedEvent` | None |
| `tick(nanos)` | At the end of every server tick, with the nanoseconds since its start and the time the bridge took after the report of the tick before, see [Plugins](plugins/index.md#tick-statistics) | None |
| `playerJoined(uuid, name)` | On `PlayerLoggedInEvent` | None |
| `playerLeft(uuid, name)` | On `PlayerLoggedOutEvent` | None |
| `chat(uuid, name, message)` | On `ServerChatEvent`, with the raw text, on a worker thread | True cancels the event |
| `death(uuid, name, cause, dimension, x, y, z, yaw, pitch)` | On `LivingDeathEvent` of a `ServerPlayer`, with the id of the damage type and where the player died | None |
| `respawn(uuid, name, dimension, endConquered)` | On `PlayerRespawnEvent`, with the id of the dimension and `isEndConquered` of the event, which is true when the player left the End through its exit portal and did not die | None |
| `teleported(id, uuid, from, to)` | When a teleport of Golem moved the player, with its id and where the player was and is, see [The end of a teleport](#the-end-of-a-teleport) | None |
| `teleportFailed(id, uuid, reason)` | When a teleport of Golem ended without a move or threw, with its id and the reason | None |
| `commandNames()` | On every `RegisterCommandsEvent`, the first before `serverStarting` | The names of the commands of the plugins |
| `command(name, uuid, senderName, level, arguments)` | When a command of `commandNames()` runs | The lines the sender sees |
| `permission(uuid, node)` | When a mod asks the PermissionAPI of Forge for a node of the type Boolean, on any thread | `PERMISSION_TRUE`, `PERMISSION_FALSE`, or `PERMISSION_NONE` for the default resolver of the node |
| `drain()` | At the end of every server tick, after `tick` | The queued actions, oldest first |

A UUID is in its usual form with dashes. A dimension is the id of its level, such as
`minecraft:the_nether`. A location is the dimension, the coordinates `x`, `y` and `z` of the feet
and the rotation in degrees, `yaw` around the vertical axis and `pitch` up and down, as the game
keeps them for the entity, so the place of a death is where a plugin sends the player back to.
`arguments` is everything after the name and one space, or the empty string. The source of a
command is one of three, and `level` is its permission level from 0 to 4, which the mod reads with
`hasPermission`, since Minecraft keeps the level private.

| Source | `senderUuid` | `senderName` | `level` |
| --- | --- | --- | --- |
| A player who made the command run: the one who typed it or clicked the sign that runs it, also when it runs as another entity through `execute as` | The UUID of the player | The name of the profile | The level of the source |
| The console and RCON: no entity at level 4 | Null | Null | 4 |
| Every other source: a command block, a function, a command that a source which is no player runs as an entity through `execute as`, a player included | Null | The name of the source, such as `@` for a command block, or of the entity it runs as | The level of the source |

The natives answer a player as `Sender::Player` with the level of the source, the console and RCON
as `Sender::Console` and every other source as `Sender::Other` with its name and level. The mod
reads who made a command run from the field `source` of `CommandSourceStack`, which Forge makes
public: the player who typed the command, the server, or whatever else made the stack, such as a
command block. `execute as` changes the entity of the stack and keeps that field and the level of
the source. So a command that a player typed reaches the plugins as that player, whatever entity it
runs as, and a command that a command block, a function or the console runs as a player reaches them
as that source, named after the player, at its own level, so a source at level 2 never borrows the
rights of a player of level 4. Vanilla judges a command by the level of the source as well, and on
the live server Arclight handed a Bukkit plugin `getBukkitSender` of the stack, which reads the same
field, so a plugin there saw the console or the command block and never the player that the command
ran as. A plugin judges a `Sender::Player` by the lower of the level of the source and the rights of
the player, since the game may run a command of a player at another level, such as a function of an
advancement at level 2. A function that runs at level 4 has no entity and the rights of the console,
so it reads as the console. The natives refuse the command of a source that the mod passes wrong
with an empty answer instead of guessing who runs it: a UUID without a name, neither of them below
level 4, or a level outside 0 to 4.

The command that the click of a sign runs has no player in that field: the game gives it a source
that takes no line and runs it as the player who clicked, at level 2, and `execute as` then changes
that entity. So the mod keeps who clicked with the command. It listens to `CommandEvent` of Forge,
which holds the stack that a command starts with, and when that stack has this shape and the block
entity of a sign hands the command to the game, it puts a source of its own in the place of the one
of the game, which takes no line either and holds the player who clicked. So the plain command of a
sign reaches the plugins as the player who clicked, at the level of the sign, as on the live server,
where Arclight names the player who clicked a sign as the sender of a plugin command that the sign
runs. `execute as` keeps that source as it keeps every source, so the command of a sign reaches the
plugins as the player who clicked whatever entity it runs as. The mod knows the sign by the code
that hands the command to the game, the first class below `Commands` on the stack of the thread.
Every other stack whose source takes no line stays another source at its level, as does a command
that another command starts while the click runs.

## Commands

The mod registers every name of `commandNames()` with Brigadier as a literal that runs on its own
and as the same literal with one argument `arguments` of the type greedy string. Both run
`command`, and the mod sends every line of the answer to what made the command run, as the game
sends the feedback of its own commands: to the player who typed the command or clicked the sign that
runs it, to the console, to the client of RCON, or into the last output of a command block, whatever
entity the command runs as. So a command that the console, a command block or another player runs as
a player through `execute as` writes no line into the chat of that player. The mod takes
`sendSuccess` of the game for it, on a copy of the stack whose source accepts every line and hands
it on, so the game rule `sendCommandFeedback` does not hide an answer, as it hides the feedback of a
command of the game. That keeps the offer of the source that `/golem source` makes to every player.
A silent source still sees no line, since Minecraft 1.20.1 drops the feedback of a source whose
output is suppressed, such as a function. A command of a plugin needs no permission level in
Brigadier: the plugin judges the sender itself, a player by the nodes that `golem-permissions`
grants them and every other source by its level, see
[Permissions](plugins/permissions.md#the-check-of-a-sender).

The client of Minecraft 1.20.1 makes no link of an address in a system message and copies no text
of the chat, so a player would type the link to the source that `/golem source` offers by hand.
The mod builds every line of an answer, of a `message` and of a `broadcast` with
`ForgeHooks.newChatWithLinks` of Forge, as Forge builds the links of a chat message: an address
with its scheme, such as `https://`, opens on a click and shows underlined in blue, and an address
without a scheme stays plain text, so no word turns into a link that a plugin did not mean.

Forge builds the commands when it loads the data packs: once at the start, before
`ServerStartingEvent`, and again on every `/reload`. So `commandNames()` answers before
`serverStarting()`, and the plugins must know their commands by then. A name that is empty or holds
whitespace cannot be typed as the first word of a command, so the mod skips it with a warning.
When `command` throws, the sender reads that the command failed and the log of the
server holds the exception.

The mod registers the commands of the plugins last, at the lowest priority of
`RegisterCommandsEvent`, so the commands of the game and of the mods are in the tree by then. A
plugin command named like one of them takes the plain name, as CraftBukkit orders them and as
[Plugin API](api.md#98-commands-and-tab-completion) designs it. The mod moves the command it
replaces to `minecraft:<name>` with its permission level, its arguments and its redirect, points
every alias of the root that redirects to it, such as `tell` to `msg`, at the moved command, and
logs one line. A mod command moves the same way, as on the live server, where Arclight wrapped every
command of the tree, those of the mods included, under the prefix `minecraft`. The names of
`commands.replace-commands` of `spigot.yml`, as Spigot sets them by default and the live server
keeps them, `setblock`, `summon`, `testforblock` and `tellraw`, keep the command of the game, and
the mod skips the plugin command with a warning. Brigadier merges a literal into the node of the
same name, which keeps the permission level and the arguments of that node, so a plugin command
merged into `/seed` would answer no player below level 2 and lose every call with arguments to the
arguments of the game. So the mod removes the node from the tree first, through the two private
maps of the root of Brigadier, which has no call for it. When it cannot, or when the tree holds
`minecraft:<name>` already, it skips the plugin command with a warning and the game keeps its
command. A mod that registers a command at the lowest priority after the bridge still merges it
into the node of the plugin.

## The game

While a native of the server thread runs, the handler in Rust may ask the game. The mod hands
Golem an object of the interface `Game` of the host jar with `connect`, and Golem keeps a global
reference to it and calls its methods through the JNI with the `JNIEnv` of the running native, so
every question is read on the server thread, inside the call of the mod.

```java
package com.skytekx.golem.host;

public interface Game {
	String[] players();
	String location(String uuid);
	String respawn(String uuid);
	String respawnPlace(String uuid);
	String spawn(String dimension);
	String mainHand(String uuid);
	String heights(String dimension);
	String safeLocation(String dimension, double x, double y, double z, int radius);
	void log(String line);
}
```

An answer is its fields joined by U+001F, as an action is, null when the game cannot answer, and
the empty string where the table names an empty answer. A field that holds U+001F, such as a
display name that a mod made, has it replaced by a space.

| Method | Fields of the answer | Null | Empty |
| --- | --- | --- | --- |
| `players()` | One answer per player: the UUID, the name of the profile, the display name, the ping in milliseconds, the game mode, the permission level, the milliseconds since the last action, the name in the player list, and no answer for a player whose answer throws, such as in a listener of a mod for the display name, which the guard logs | Never on the server thread | The name in the list, while the client shows the name of the profile |
| `location(uuid)` | The dimension, `x`, `y`, `z` of the feet, `yaw`, `pitch` | The player is not online | Never |
| `respawn(uuid)` | The dimension and `x`, `y`, `z` of the respawn point that the player holds, which a bed, a respawn anchor or `/spawnpoint` set, its angle and whether `/spawnpoint` forced it, `true` or `false`, as it is stored, since Minecraft checks its block only when the player respawns | The player is not online | The player has no respawn point |
| `respawnPlace(uuid)` | The dimension, `x`, `y`, `z` of the feet and the yaw where the player would respawn now, see below, or the one field `unloaded` when a chunk within 3 blocks of the point is not loaded | The player is not online | The player holds no point, or the point holds no respawn now, so the game would respawn them at the world spawn |
| `spawn(dimension)` | `x`, `y`, `z` of the shared spawn and its angle | The server has no such dimension | Never |
| `mainHand(uuid)` | The registry id of the item and the count | The player is not online | The hand is empty |
| `heights(dimension)` | The lowest y of a block and the y above the highest | The server has no such dimension | Never |
| `safeLocation(dimension, x, y, z, radius)` | `x`, `y`, `z` of the feet in the middle of the block, or the one field `unloaded` when the search met a chunk that is not loaded | The server has no such dimension, or the target is no finite number | No place within the radius is safe |

`log(line)` is no question. Golem calls it for every line that it logs while a native runs, on the
thread of that native, which may be any thread of the table of natives, and the mod logs the line
with its logger `golem` at the level `INFO`. So the lines of the natives and the plugins, such as a
panic of a plugin or that the bridge is connected, stand in `logs/latest.log` and on the console
beside the lines of the mod. A Java exception that is pending on the thread waits aside for the
call and is thrown again after it. A line that Golem logs outside a native, such as the lines of
the host before the JVM starts, on a signal and at the end, a line before `connect`, and a line
whose call throws go to the standard error behind the prefix `[golem]`, see
[JVM host](jvm-host.md).

The game mode is the name of the game, `survival`, `creative`, `adventure` or `spectator`. The
permission level is the operator level of the profile from `getProfilePermissions`, which neither
`ops.json` nor `op-permission-level` of `server.properties` bounds, so Golem reads a level above 4
as 4 and one below 0 as 0, with one line on the log, instead of dropping the player. The
time since the last action is `Util.getMillis()` less `getLastActionTime()`, which the game
resets when the player moves, chats, uses an item or clicks, so a plugin marks a player who is
away. The shared spawn of the Nether and the End is the one of the Overworld, since Minecraft
keeps one spawn for the worlds of a server. The registry id of an item comes from the registry of
Forge, such as `minecraft:diamond`.

`respawnPlace` resolves the respawn point as `PlayerList.respawn` of Minecraft 1.20.1 does, with
`Player.findRespawnPositionAndUseSpawnBlock` for a player who lives, so a respawn anchor keeps its
charge: a bed needs room beside it, an anchor a charge, which `/spawnpoint` does not ask, and a
point that `/spawnpoint` forced needs room for the player, and Forge lets a bed of a mod answer for
itself. The yaw faces a bed or an anchor as a respawn turns the player, and is the angle of the
point otherwise, and the place is lifted by whole blocks while the box of a player collides
there, as the game lifts a player after a respawn. The question reads the blocks within 3 blocks
of the point alone, and answers `unloaded` when a chunk of them is not loaded, since a question
never loads a chunk. So the port of EssentialsX tells a bed that is gone from one that stands, as
`getBedSpawnLocation` of Bukkit does, without a respawn.

`safeLocation` answers the nearest place near the block of the target where a player stands safely,
with the search that the mod writes itself and copies from no plugin. A place is safe when the block
below the feet carries a player on its full top face and is no block that hurts, the blocks of the
feet and the head have no collision shape, hold no fluid, are no portal of the tag
`minecraft:portals`, the nether portal, the end portal and the end gateway, which would carry the
player away, no cobweb, which would hold them, and do not hurt, the ground lies at or above the
lowest block of the dimension, the head below its top, and a player in the middle of the block
inside the world border, with half the width of a player, 0.3 blocks, to spare, since the border of
`/worldborder center` runs through the middle of a block. A target below the logical height of its
dimension, the lowest block plus `logical_height` of the type of the dimension, keeps the head below
that height too, as EssentialsX does, so a search from under the bedrock roof of the Nether, whose
logical height ends at y 128 while its blocks go up to y 256, never lands on top of the roof. A
target at or above it searches up to the top. The blocks that hurt are fire, soul fire, campfires,
cactus, magma, sweet berry bushes, wither roses, powder snow, pointed dripstone and lava, and every
block, of a mod as well, whose type of path for the mobs of Forge, `getBlockPathType`, names a
danger, such as `DAMAGE_FIRE`, `DAMAGE_OTHER` or `LAVA`, or is `POWDER_SNOW` or `DAMAGE_CAUTIOUS`,
as the mobs avoid it. The search looks at every block within the sphere of the radius around the
block of the target, at most 8 blocks, in this order: the nearest first, then the higher, then the
lower `x`, then the lower `z`, so a world always gives the same answer. It
reads at most 2109 places, and only those of loaded chunks: at the first place in its order whose
chunk is not loaded, before it found a safe one, it stops and answers `unloaded`, so an answer with
a place is always the answer of the whole search. A question never loads a chunk, since a load on
the server thread waits for the workers of the server, which may wait for the handler of Golem that
asks, and so could stall the tick or hang the server. A plugin that wants a place in a chunk that is
not loaded asks again once a player or a ticket of the game loads it.

The Rust side asks through the trait `golem_bridge::Game`, which the handler gets on every call.

| Implementation | Where | Answers |
| --- | --- | --- |
| The JNI | `golem-bridge`, the natives | The object of `connect`, asked with the `JNIEnv` of the native |
| `NoGame` | `golem-bridge` | Nothing, before the mod connects a game and for a call without one |
| The scripted game | `golem-devtool` | What a script of `04_plugins` names |

Every implementation reaches the handler behind the guard `ServerThread` of `golem_bridge::host`.
The server thread is the thread of the first `serverStarting()`, and a question on any other
thread, such as from a handler of `chat()`, answers none without asking the mod. The first such
question of the process is logged. A question runs in a local frame of its own, so a native that
asks many questions leaves no local reference behind. A Java exception or error that a question
throws, such as a `StackOverflowError` of a listener of a mod, is taken and cleared, so it never
crosses into the caller of the native, and the question answers none. A question that fails and
an answer that does not read are logged the first time, the tenth, the hundredth and every
further power of ten, with the class and the message of what the question threw, and the stack
trace of a question is printed for its first failure alone. A native whose own call of the JNI
fails is logged the same way. The object of the mod checks the thread a second time with
`isSameThread` of the server and answers null on any other thread, so the game is read on its own
thread alone, whatever Golem does. It answers null from `ServerStoppedEvent` on as well, also on
the server thread, since Minecraft closed every level and its workers before that event, so a
question would read closed chunks or wait for a worker that never answers.

## Permissions

Forge 47 asks one permission handler whether a player holds a node, when a mod calls
`PermissionAPI.getPermission`. The mod offers the handler `golem:permissions` on
`PermissionGatherEvent.Handler`, which Forge posts while the server starts, before
`ServerStartingEvent`. Forge takes the handler that the key `permissionHandler` of the section
`server` of `forge-server.toml` names, so a server under Golem must name it there:

```toml
[server]
	permissionHandler = "golem:permissions"
```

`forge-server.toml` is a configuration of the world, in `world/serverconfig/`, and Forge copies it
from `defaultconfigs/` of the server when a world has none, so a new world takes the handler from
`defaultconfigs/forge-server.toml`. Without the key, or on a server whose mod is off, Forge uses its
default handler `forge:default_handler` and logs that it could not find `golem:permissions`. A
world taken over from a server without Golem keeps its own file, which names
`forge:default_handler`, and then Forge logs no more than an info line for the handler it takes.
So the mod asks `PermissionAPI.getActivePermissionHandler` on `ServerStartingEvent`, right after
Forge chose the handler, and when that is not `golem:permissions` it logs one warning that names
the handler and the file of the world, since no plugin decides a permission of a mod then.

| Node | What the handler answers |
| --- | --- |
| Type Boolean, the native answers `PERMISSION_TRUE` or `PERMISSION_FALSE` | That answer |
| Type Boolean, the native answers anything else or throws | What the default resolver of the node answers, as under the default handler |
| Another type: Integer, String or Component | What the default resolver of the node answers |

The handler asks for an online player with `getPermission` and for an offline one with
`getOfflinePermission`, both by the UUID, and the name of the node is that of `getNodeName()`,
such as `forge.use_entity_selectors`, which Forge asks for a player below level 2 who uses a
selector. Mods ask on any thread, so the native runs beside the server thread as well. A mod may
ask a node with a dynamic context of Forge, such as the dimension or the block that the check is
about, and the native takes the UUID and the name of the node alone. So the answer of a plugin
holds in every context, as the answer of LuckPerms through Arclight did on the live server, which
asked `hasPermission` with the name of the node alone, and only the default resolver of a node
that no plugin decides gets the context.

The live server decides these nodes otherwise. Its `arclight.conf` sets `forward-permission=true`,
so Arclight wraps the handler that Forge takes, `forge:default_handler` of its
`defaultconfigs/forge-server.toml`, and answers every Boolean node of an online player with
`hasPermission` of Bukkit. LuckPerms decides every node that a mod asks, a node that it does not
name false unless a processor of LuckPerms, such as the one for operators, grants it, and the
default resolver of a node is never asked. The mods of the modpack that ask such nodes are
Mekanism, Nature's Compass, Ars Magica Legacy, Balm, CC: Tweaked, JEI, spark and Simple Voice
Chat, which asks `voicechat.speak` for every packet of a microphone. The handler of Golem falls to
the default resolver when no plugin answers, which often grants a node to every player or to an
operator level, so the plugin of the permissions of `golem run`, the port of LuckPerms, answers
these nodes for an online player as LuckPerms did, see
[Permissions](plugins/permissions.md#arclight-forwards-every-permission-of-a-mod-to-luckperms).

## Actions

`drain()` answers each action as one string, its fields joined by the character U+001F, the unit
separator, which no field may hold. The first field is the kind.

| Kind | Fields after the kind | What the mod does on the server thread |
| --- | --- | --- |
| `command` | The command without its slash | Runs it as the console, with the level and the name of the console, whose answer reaches the console and the log of the server but no operator in the game, see below |
| `message` | The UUID of a player, the text | Sends the text as a system message to that player, if they are online |
| `broadcast` | The text | Sends the text as a system message to every player and the console |
| `styledMessage` | The UUID of a player, the JSON of a styled text | Sends the text as a system message to that player, if they are online, see [Styled text](#styled-text) |
| `styledBroadcast` | The UUIDs of the players who do not get it, joined by commas, the JSON of a styled text | Sends the text as a system message to every other player and to the console |
| `teleport` | The id, the UUID of a player, the dimension, `x`, `y`, `z`, `yaw`, `pitch` and the radius of the search for a safe place, empty for none | Teleports the player there, or to the nearest safe place within the radius once the chunks of the search are loaded, also into another dimension, and reports how it ended, see [The end of a teleport](#the-end-of-a-teleport) |
| `listName` | The UUID of a player, the name | Shows the name for the player in the player list, if they are online, or clears it when the name is empty |

A command of Golem runs with a source of its own in place of the console of the server, so the
feedback of `sendSuccess` goes to the console and the log of the server, while no operator in the
game reads it. A command typed at the console tells every operator online what it did, under
`broadcast-console-to-ops` of `server.properties`, so a plugin that acts through commands, such as
a port of `/home`, would show every teleport of every player to every operator. The stop path of
Golem, `save-all flush` and `stop`, stays on the console and in the log as before.

A number travels as the shortest decimal that reads back as the same number, which `Display` of Rust
writes and `Double.parseDouble` and `Float.parseFloat` of Java read, and an infinity as Java spells
it, `Infinity` or `-Infinity`, since `Display` writes `inf`, which Java does not read. A teleport
does what the command `tp` does, step by step as `TeleportCommand.performTeleport` of Minecraft
1.20.1: it posts `EntityTeleportEvent.TeleportCommand` of Forge, so a mod may cancel it or move its
target, skips a place outside the bounds of the world, `Level.isInSpawnableBounds`, which are x and
z from -30000000 to below 30000000 and y from -20000000 to below 20000000, wraps the yaw to -180 to
180 and the pitch to -90 to 90, and calls the `teleportTo` of `tp`, which keeps the chunk of the
place loaded for a moment with the ticket `post_teleport`, lets a rider dismount, wakes a sleeping
player, turns the head, keeps the camera of a spectator within one dimension and moves the player
into another dimension, which a mod may cancel on `EntityTravelToDimensionEvent`. The fall stops
when the player is in the dimension of the place afterwards. It takes the place as it is, so a
plugin that wants a safe place asks the game for one first. A name of the player list is answered on
`PlayerEvent.TabListNameFormat` after every other listener, so it wins over the name of another mod,
and `refreshTabListName` sends it to every client. It holds until Golem clears it or the player
leaves.

The mod logs an action of an unknown kind, or of a known kind with the wrong number of fields, once
and skips it, and so a teleport whose id does not read, a teleport into a dimension the server does
not have, once per dimension, to a place whose numbers do not read, that is no finite number or
that lies outside the bounds of the world, with a radius outside 0 to 8, and a teleport that a mod
cancelled. Each action runs on its own, so an action that throws, such as a message to a UUID that
does not parse, is logged and the next action runs. The host stops the server with the actions
`command save-all flush` and `command stop`, so the stop path is the same as an operator's at the
console.

### Styled text

The plain `message` and `broadcast` carry text as it is, which the mod turns into a component with
`ForgeHooks.newChatWithLinks`, so a colour needs a formatting code with `§` in the text. A styled
text carries the JSON of a text component of Minecraft 1.20.1, the form that
`Component.Serializer` reads, which [Plugin API](api.md#916-chat-components-and-messages) names
the form of a text component of the API. A plugin builds it in Rust with `golem_bridge::Text`, and
the mod reads it with `Component.Serializer.fromJson`, as the game reads the text of `tellraw`.

| Part of a text | In Rust | In the JSON |
| --- | --- | --- |
| The words of a plugin, newlines included | `Text::plain(text)` | `text` |
| What a player wrote or named, kept literal | `Text::player(text)` | `text`, without formatting codes, control characters and direction controls |
| A key of the language of the client with its arguments | `Text::translate(key, with)` | `translate` and `with` |
| One of the 16 colours of the game or any colour | `color(Color::Gold)`, `color(Color::Rgb(0xff, 0xaa, 0x00))` | `color` as `gold` or `#ffaa00` |
| The decorations | `bold`, `italic`, `underlined`, `strikethrough`, `obfuscated` | The keys of the same names |
| A click | `click(Click::OpenUrl(...))`, `RunCommand`, `SuggestCommand`, `CopyToClipboard` | `clickEvent` with `open_url`, `run_command`, `suggest_command` or `copy_to_clipboard` |
| A hover | `hover(text)` | `hoverEvent` with `show_text` and `contents` |
| Text for a click with shift | `insertion(text)` | `insertion` |
| The texts after it, which take its style where they set none | `append(text)` | `extra` |

What a player wrote or named, such as a name, a chat line or the name of a home, enters a text
through `Text::player` as a text of its own, so it can neither close the style around it nor open
one. `strip_codes` takes out of it every `§` with the character after it, as the client of Minecraft
hides both when it draws a text, every control character that the game refuses in a chat line, below
U+0020 and U+007F, a newline among them, so no player starts a line of their own, and every
direction control of Unicode, U+202A to U+202E and U+2066 to U+2069. The game itself refuses a `§`
and a control character in what a player types, so a chat line or an argument of a command passes
unchanged but for a direction control, and a display name that a mod coloured reads as the client
draws it without its colour. The JSON escapes the quote, the backslash, every control character and
U+2028 and U+2029, so it holds no U+001F of the wire form and is as long as the JSON that the game
writes for the same text. The mod adds no link to a styled text, so a styled text links where its
plugin set a click, and a player cannot make one.

The context of a plugin refuses with a line on the log a text whose JSON holds more than
262144 UTF-16 units, `MAX_JSON`, the bound of the packet of a system message, which the game would
otherwise drop on its way to the client, and a text whose texts sit more than 32 deep inside each
other, `MAX_DEPTH`, through `extra`, `with` and the hover, so no text costs a client a deep
recursion. The mod logs once and skips a styled action whose JSON does not read, and reads a part
of the hidden players that is no UUID as nobody. The client of Minecraft 1.20.1 opens an address of
`http` and `https` alone, and while its prompt for links is on, as it is by default, only after the
player confirms it, and runs a command of a click only when it starts with a slash and takes no
signed argument, such as the message of `msg`.

The texts of EssentialsX 2.22.0 are MiniMessage in `messages.properties`, with the colours
`<primary>` and `<secondary>` of `message-colors`, `#ffaa00` and `#ff5555` live, the 16 colours of
the game, `<reset>`, the decorations `<b>`, `<i>` and `<st>`, the click `<click:open_url:"{0}">`
and newlines, and the arguments `{0}` to `{n}` of `MessageFormat`. The port parses each template
once into a tree of `Text`: a run of words becomes `Text::plain` with the style of the tags open
around it, a tag of a colour or a decoration sets the style of the runs inside it and `<reset>`
closes every tag, `<click:...>` becomes a `Click` and `<hover:show_text:...>` a hover, and an
argument becomes a text of its own, `Text::player` for what a player wrote or named and a text
that the port built for the rest, such as a link, so no argument is ever parsed as MiniMessage, as
EssentialsX escapes its arguments. The value of a click that holds an argument takes the argument
as plain text, which the JSON escapes. The parser belongs to the port, not to the bridge, see
[EssentialsX bridge needs](plugins/essentials/bridge-needs.md).

### The end of a teleport

Every teleport carries an id, which `Actions::teleport` of `golem-bridge` hands out once per process
from 1 on, so the plugin that queued it learns how it ended. Every id gets exactly one report, on
the server thread, unless the JVM is halted or killed before, as the end of this section says. The
mod reports through one of two natives, at the end of the tick that ended the teleport as a rule,
and for a teleport that waits also when its player leaves or the server stops. The natives
themselves report a teleport that never reached the mod, as the end of this section says.

| Native | When | Fields |
| --- | --- | --- |
| `teleported(id, uuid, from, to)` | The player moved | `from` is where the player was right before the move and `to` where they are after it, each in the form of the answer of `location`, so `to` holds what a safe search or a mod made of the target |
| `teleportFailed(id, uuid, reason)` | The teleport ended without a move, or threw | The reason, one word of the table below, and the UUID as the action carried it |

| Reason | Why the teleport ended |
| --- | --- |
| `offline` | No player of the UUID is online when the teleport runs, the UUID does not parse, the player left while the teleport waited for its chunks, or a mod made the player leave inside the move, before the game moved them |
| `dimension` | The server has no dimension of the id |
| `place` | A number does not read or is no finite number, the radius lies outside 0 to 8, or the place lies outside the bounds of the world, also after a mod moved it |
| `cancelled` | A mod cancelled `EntityTeleportEvent.TeleportCommand`, or the travel into another dimension on `EntityTravelToDimensionEvent` |
| `unsafe` | The search found no safe place within its radius |
| `unloaded` | The chunks of the search did not load within 600 ticks, 30 seconds at 20 TPS |
| `stopped` | The server stopped while the teleport waited, in the mod for its chunks or in the queue of Golem for its tick, or an error ended the server from inside an action of the drain that handed the teleport to the mod, before its turn |
| `error` | The teleport threw. The guard of the mod logs what it catches, and the game goes on. An error that the guard lets pass ends the server, and the teleport that ran ends with this reason before. Either leaves the place of the player unknown, since a mod may throw behind the move. The natives also end a teleport with it whose report does not read, a null or a place that is none, under the UUID of the report or an empty one, where the id reads |

A teleport without a radius moves the player at once, as `tp` does. A teleport with a radius
searches near its target as `safeLocation` searches, and moves the player to the middle of the
block it found, with the rotation of the target. The search reads loaded chunks alone, so the mod
first checks that every chunk of the columns within the radius of the block of the target is
loaded, at most four. When one is not, it adds a ticket of its own, `golem_teleport` at the level
of a full chunk, for each of them, keyed by the id, so two teleports never take each other's
ticket away, and the game loads or generates the chunks on its workers over the next ticks, while
no tick waits for them. At the end of every tick the mod looks again, before it runs the new
actions, searches once the chunks are loaded, removes its tickets and moves the player, and after
600 ticks it gives up with `unloaded`. The ticket runs out by itself 20 ticks after that, should a
fault keep the mod from removing it. So a far home or a warp whose chunks no player keeps loaded
finds its safe place, which `safeLocation` answers `unloaded` for.

The teleports of one player run in the order of their actions: a teleport of a player who has one
that waits for its chunks waits behind it, also when it has no radius, and the reports of one player
come in that order. When the player leaves, every teleport of theirs that waits in the mod ends with
`offline` before `playerLeft` reaches Golem, and one that still stands in the queue of Golem then
ends with `offline` at the end of that tick, behind `playerLeft`. A mod may make a player leave
inside the move of a teleport: the teleports that wait behind it end on the spot, and the one that
runs ends when its move returns, behind them and behind `playerLeft`, with `offline` when the player
is gone behind `EntityTeleportEvent.TeleportCommand`, where the mod moves nobody.

On `ServerStoppingEvent` every teleport that waits in the mod ends with `stopped` before
`serverStopping` reaches Golem. No tick drains the queue of Golem from there on, so the native
`serverStopping()` ends every teleport that still waits in that queue the same way, oldest first,
before the plugins hear of the stop, and the queue takes no teleport until the next
`serverStarting()`. A server whose loop ends by a fault stops without `ServerStoppingEvent`, and
Forge still posts `ServerStoppedEvent`. There the mod and the native `serverStopped()` end what
still waits in the same way, before the plugins hear that the server stopped, and behind a stop that
the game announced nothing waits, so nothing is reported twice. The game removes its players before
`ServerStoppedEvent`, and a player whose connection has closed by then leaves there, so the
teleports that wait in the mod for that player end with `offline`, as at every leave. Such a fault
may fly out of an action of the bridge itself: the guard of the mod catches an exception and a
linkage error, and every other error of the JVM, such as a stack that overflows in a listener of
another mod, passes it and ends the loop of the server. The teleport that ran stands in no queue
then, and the actions of that drain behind it never run. So the mod reports `error` for the teleport
that ran and `stopped` for every teleport of that drain behind it, at once and so before the
teleports that wait in the mod end, and throws the error on. So every id gets its report on every
way that the game ends its server while the JVM still runs the mod. A JVM that is halted, as the
watchdog of the dedicated server halts it, or killed, runs nothing, so no report comes then, and no
teleport outlives the process. The Rust side also refuses a place that is no finite number or
outside the bounds of the world before it queues a teleport, as
[Plugins](plugins/index.md#the-context-of-a-call) says, so `place` names a fault of the mod or a mod
that moved the target there.

## The rules of the server thread

- Every native method runs on the thread that calls it. That is the server thread for every
  method of this page but five, as Forge 47.4.18 calls them and the notebook `05_bridge_build`
  records. `version()` and `connect()` run on a thread of the mod loader, `modloading-worker-<n>`,
  while Forge constructs the mods. `commandNames()` runs on the main thread, `main`, when Forge
  builds the commands at the start, and on the server thread on a reload. `chat()` runs on a
  worker thread of Forge, `Worker-Main-<n>`, while the server thread goes on, since Forge fires
  `ServerChatEvent` there. `permission()` runs on the thread of the mod that asks Forge, which is
  the server thread for the selectors of a command.
- A handler in Rust answers before the event goes on, so it must be quick. The plugin runtime times
  every call against its budget.
- The handler runs one call at a time. The server thread waits for a call of another thread as
  long as it runs, since such a call gets no answer from the game and so never waits for the
  server thread. A call of another thread waits at most one second for a call of the server
  thread, and then answers what a pass would answer, since a question of the server thread may
  wait for a worker of the server, such as the one that runs a chat message.
- The mod listens to `ServerChatEvent` and `LivingDeathEvent` at the priority `LOWEST`, after every
  other mod, so Golem hears only what really happens, and to the end of the tick at `LOWEST` and to
  its start at `HIGHEST`. [Plugin API](api.md#103-order-priorities-and-ignorecancelled) changes
  these rules once its shared base moves this contract onto the generated hooks: its listeners take
  the priority `LOW` with `receiveCanceled = true`, so the mods at `LOWEST` see what the plugins
  decided, and an event fired inside a handler enters the dispatcher again on the same thread, in
  place of the one call at a time and the wait of one second above, see [Plugin
  API](api.md#74-re-entrancy).
- Rust never touches game state from another thread. Whatever Rust wants done is an action, and
  it reads the game only through `Game` while a native of the server thread runs. A handler of
  `chat()` runs beside the server thread, so it must be thread safe and may only answer and queue
  actions, and every question it asks the game answers none. Its answer still cancels the
  message.
- A native method never lets a panic of Rust or a Java exception of a question to the game cross
  into the JVM. It catches the panic, logs it and answers what a pass would answer: nothing,
  false, or an empty array.
- The mod catches every exception of its own around a native call, logs it with the logger
  `golem` and lets the game go on, so a fault of the bridge never stops the server. A call that
  fails again and again logs its first failure with the stack trace and after that only its tenth,
  its hundredth and every further power of ten, so a native that fails on every tick cannot fill
  the log.

## The bridge and the Plugin API

The questions of `Game`, the actions `teleport` and `listName`, the permission handler and the
level of a command sender are the first game interface of [Plugin
API](api.md#16-the-essentialsx-port-and-the-first-game-interface) and the seed of its mapping. They
keep their fields joined by U+001F until the shared base of the Plugin API moves them onto the
generated contract, where each question becomes a coordinate of the schema and `golem_bridge::Game`
and `Context::game()` become a shim over the client of the schema. This page describes the contract
as it is built, and Plugin API the contract it becomes.

## Building the bridge

The host jar and the mod are one Gradle build under `bridge/`, with the projects `host` and `mod`.
It runs with the system Gradle 8.14.3 and a JDK 17 and carries no Gradle wrapper, whose jar is a
binary under another license. `python3 -I tests/fetch.py` downloads both into `tests/.cache/`. The
path of the JDK differs per machine, so it is passed on the command line and never committed:

```text
JAVA_HOME=$PWD/tests/.cache/jdk-17.0.21+9 tests/.cache/gradle-8.14.3/bin/gradle -p bridge \
    -Porg.gradle.java.installations.paths=$PWD/tests/.cache/jdk-17.0.21+9 build
```

`--offline` builds without the network once the Gradle cache is warm. `bridge/gradle.properties`
holds only what holds on every machine: the heap of Gradle, no daemon, and no download of a JDK.

| Part | Version | Set in |
| --- | --- | --- |
| Gradle | 8.14.3, without a wrapper | The command line |
| JDK | Temurin 17.0.21, every class compiled with `--release 17` | `bridge/build.gradle` |
| ForgeGradle | `[6.0.24,6.2)`, the range of the MDK of Forge 47, 6.0.54 on 2026-11-06 | `bridge/mod/build.gradle` |
| Forge | 1.20.1-47.4.18, mappings `official` 1.20.1 | `bridge/mod/build.gradle` |
| The jars | The version of `[workspace.package]` in `Cargo.toml` | `bridge/build.gradle` |

The build compiles with every lint of javac as an error, writes the version, the vendor and
`Bundle-License: AGPL-3.0-only` into the manifest of each jar and keeps no file times in them.

| Jar | Lands in | Holds |
| --- | --- | --- |
| `golem-host.jar` | `bridge/host/build/libs/` | The class `Host` and `Automatic-Module-Name: com.skytekx.golem.host` |
| `golem-bridge-<version>.jar` | `bridge/mod/build/libs/` | The mod `golem`, reobfuscated to the SRG names |

ForgeGradle compiles the mod against Minecraft in the official names of Mojang, and its task
`reobfJar` rewrites the jar in place to the SRG names that the server runs with, so a call of
`MinecraftServer.getCommands()` reads `m_129892_` in the jar. The mod takes the host project at
compile time alone, so the host class never enters the mod jar. The JVM host of Golem puts
`golem-host.jar` on the module path and the mod jar in `mods/`. The mods.toml takes the version
from the manifest, names the license `AGPL-3.0-only` and sets `displayTest` to
`IGNORE_SERVER_VERSION`, since a client needs no copy of the mod, and `pack.mcmeta` with pack
format 15 keeps Forge from warning about missing pack metadata.

The first build on a machine downloads Minecraft and the tools of Forge into the Gradle cache of
the user and remaps them. That took 5 minutes 20 seconds in a spike in the dev container of
2026-11-06, and a later build of a fresh copy of `bridge/` took 15 seconds in `05_bridge_build` on
the same machine that day.

### What the notebook checks

The notebook `tests/notebooks/05_bridge_build.ipynb` builds a copy of `bridge/` and checks the
jars against this page.

- The host jar holds exactly two classes, `com/skytekx/golem/host/Host.class` and the interface
  `com/skytekx/golem/host/Game.class`, of class file version 61, and depends on its package and
  `java.lang` alone. Its manifest names the module, the version of Golem and the license. The
  public methods of `Host` are the 18 natives of the first Java block above, each with the
  descriptor that the notebook derives from that block and that `javap` confirms, and the plain
  `available()`, and those of `Game` are the 9 abstract methods of the block of
  [The game](#the-game).
- The mod jar holds the mods.toml and the `pack.mcmeta` of above, no class of the host package and
  no `.license` file. It listens for every event of the table and calls every native but
  `version()`, with the descriptors of the host jar. Every member of Minecraft it uses carries its
  SRG name, but the methods of `java.lang.Object`, the methods that Forge adds to a class of
  Minecraft, the constants of an enum and the methods that a class takes from Brigadier, and no
  official name of Mojang appears.

It then boots a copy of the Forge server of `tests/fetch.py` on port 25633 four times, through
the java launcher of the JDK with the argument files of the server, as `run.sh` does. Each boot
must reach `Done`, stop cleanly with no FATAL line and no crash report, and find the mod.

| Boot | The mod must |
| --- | --- |
| Without the host jar | Log one warning that the host class is missing, and nothing else, while Forge falls back to its own permission handler |
| With `golem-host.jar` on the `-p` of a copied `unix_args.txt` | Log one warning that the natives are not registered, while Forge falls back to its own permission handler |
| With the host jar and stand in natives | Call every native, answer every question of the game, run every action, decide `forge.use_entity_selectors` through the native, and stop the server through the actions |
| The same, once `forge-server.toml` of the world names `forge:default_handler` | Log one warning that Forge answers the PermissionAPI with that handler, which names the file of the world |

The stand in natives are a small C library that the notebook compiles and loads with
`-agentpath`, since HotSpot searches the symbols of an agent library for a native method that no
library of its class loader holds. A minimal offline client of Minecraft 1.20.1 joins, chats,
runs a command of the plugins, uses a selector, dies, respawns, is teleported into the Nether and
named in the player list, and leaves, and the console runs the command as the player and as an
armor stand. The stand in natives ask the object of `connect` every question of the game on the
server thread and once from the worker thread of a chat message, where it answers null. They queue
teleports with ids: into the Nether, into a dimension the server does not have, to places that are
no number, onto the roof of the Nether 4000 blocks away, whose chunks no player loaded and whose
search lands on the bedrock at y 128 once they are, and behind it above the Overworld 3000 blocks
away, where no place is safe, for a UUID of nobody online and one that does not parse, into chunks
that no player loaded for a player who is kicked in the same tick, and for a second player online
at the stop. Every teleport must end once, on the server thread, in the order of the teleports of
its player, with the place or the reason of the table of
[The end of a teleport](#the-end-of-a-teleport), the kicked one before `playerLeft` and the one of
the stop before `serverStopping`. While a
second client is online, the natives take the game mode from the first player for one question of
the players, so that the answer of that player throws, and `players()` must answer the second
player alone, with one error of the guard. The natives name `seed` and `teleport`, which the game
holds, and `setblock`, which it keeps: the player runs the `seed` of the plugin below the level of
the game, the console reaches the commands of the game as `minecraft:seed` and
`minecraft:teleport` and through the alias `tp`, and `setblock` stays the command of the game with
a warning. The notebook records the thread of every call, and the threads of
[the rules of the server thread](#the-rules-of-the-server-thread) come from it.

Related: [JVM host](jvm-host.md), [Plugins](plugins/index.md), [Index](index.md).
