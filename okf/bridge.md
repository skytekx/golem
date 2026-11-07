---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Spec
title: Bridge
description: The contract between the Golem bridge mod in the JVM and the Rust of Golem, with the host class, its native methods, the game that Rust may ask, the permission handler of Forge, the wire form of the actions and the rules of the server thread.
tags: [bridge, jni, forge, contract]
timestamp: 2026-11-06
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
	public static native void respawn(String uuid, String name, String dimension);
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
| `serverStarting(forge, minecraft)` | On `ServerStartingEvent` | None |
| `serverStarted()` | On `ServerStartedEvent` | None |
| `serverStopping()` | On `ServerStoppingEvent` | None |
| `serverStopped()` | On `ServerStoppedEvent` | None |
| `tick(nanos)` | At the end of every server tick, with the nanoseconds since its start | None |
| `playerJoined(uuid, name)` | On `PlayerLoggedInEvent` | None |
| `playerLeft(uuid, name)` | On `PlayerLoggedOutEvent` | None |
| `chat(uuid, name, message)` | On `ServerChatEvent`, with the raw text, on a worker thread | True cancels the event |
| `death(uuid, name, cause, dimension, x, y, z, yaw, pitch)` | On `LivingDeathEvent` of a `ServerPlayer`, with the id of the damage type and where the player died | None |
| `respawn(uuid, name, dimension)` | On `PlayerRespawnEvent`, with the id of the dimension | None |
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
| A player who runs the command themself | The UUID of the player | The name of the profile | The level of the source |
| The console and RCON: no entity at level 4 | Null | Null | 4 |
| Every other source: a command block, a function, a command that another source runs as an entity through `execute as`, a player included | Null | The name of the source, such as `@` for a command block, or of the entity it runs as | The level of the source |

The natives answer a player as `Sender::Player` with the level of the source, the console and RCON
as `Sender::Console` and every other source as `Sender::Other` with its name and level. The mod
tells a player who runs a command themself apart from a source that runs it as them through the
field `source` of `CommandSourceStack`, which Forge makes public: `execute as` changes the entity
of the stack and keeps that field and the level of the source that made the command run. So a
command that a command block, a function, the console or another player runs as a player reaches
the plugins as that source, named after the player, at its own level, and a source at level 2
never borrows the rights of a player of level 4. Vanilla judges such a command by the level of the
source as well, and on the live server Arclight handed a Bukkit plugin `getBukkitSender` of the
stack, which reads the same field, so a plugin there saw the console or the command block and never
the player. A plugin judges a `Sender::Player` by the lower of the level of the source and the
rights of the player, since the game may run a command of a player at another level, such as a
function of an advancement at level 2. A function that runs at level 4 has no entity and the rights
of the console, so it reads as the console. The natives refuse the command of
a source that the mod passes wrong with an empty answer instead of guessing who runs it: a UUID
without a name, neither of them below level 4, or a level outside 0 to 4.

## Commands

The mod registers every name of `commandNames()` with Brigadier as a literal that runs on its own
and as the same literal with one argument `arguments` of the type greedy string. Both run
`command`, and the mod sends every line of the answer to the source of the command as a system
message, with `sendSystemMessage`, so the game rule `sendCommandFeedback` does not hide it, as it
hides the feedback of `sendSuccess`. That keeps the offer of the source that `/golem source` makes
to every player. A silent source still sees no line, since `sendSystemMessage` of Minecraft 1.20.1
drops every message to a source whose output is suppressed, such as a function. The answer goes to
the player of the source when it has one, so a command that runs as a player through `execute as`
answers that player, and else to the source itself, such as the console or a command block. A
command of a plugin needs no permission level in Brigadier: the plugin checks the sender itself,
through `golem-permissions`.

Forge builds the commands when it loads the data packs: once at the start, before
`ServerStartingEvent`, and again on every `/reload`. So `commandNames()` answers before
`serverStarting()`, and the plugins must know their commands by then. A name that is empty or holds
whitespace cannot be typed as the first word of a command, so the mod skips it with a warning.
When `command` throws, the sender reads that the command failed and the log of the
server holds the exception.

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
	String spawn(String dimension);
	String mainHand(String uuid);
	String heights(String dimension);
	String safeLocation(String dimension, double x, double y, double z, int radius);
}
```

An answer is its fields joined by U+001F, as an action is, null when the game cannot answer, and
the empty string where the table names an empty answer. A field that holds U+001F, such as a
display name that a mod made, has it replaced by a space.

| Method | Fields of the answer | Null | Empty |
| --- | --- | --- | --- |
| `players()` | One answer per player: the UUID, the name of the profile, the display name, the ping in milliseconds, the game mode, the permission level, the milliseconds since the last action, the name in the player list | Never on the server thread | The name in the list, while the client shows the name of the profile |
| `location(uuid)` | The dimension, `x`, `y`, `z` of the feet, `yaw`, `pitch` | The player is not online | Never |
| `respawn(uuid)` | The dimension and `x`, `y`, `z` of the respawn point that the player holds, which a bed, a respawn anchor or `/spawnpoint` set, as it is stored, since Minecraft checks its block only when the player respawns | The player is not online | The player has no respawn point |
| `spawn(dimension)` | `x`, `y`, `z` of the shared spawn and its angle | The server has no such dimension | Never |
| `mainHand(uuid)` | The registry id of the item and the count | The player is not online | The hand is empty |
| `heights(dimension)` | The lowest y of a block and the y above the highest | The server has no such dimension | Never |
| `safeLocation(dimension, x, y, z, radius)` | `x`, `y`, `z` of the feet in the middle of the block, or the one field `unloaded` when the search met a chunk that is not loaded | The server has no such dimension, or the target is no finite number | No place within the radius is safe |

The game mode is the name of the game, `survival`, `creative`, `adventure` or `spectator`. The
permission level is the operator level of the profile from `getProfilePermissions`, which neither
`ops.json` nor `op-permission-level` of `server.properties` bounds, so Golem reads a level above 4
as 4 and one below 0 as 0, with one line on the log, instead of dropping the player. The
time since the last action is `Util.getMillis()` less `getLastActionTime()`, which the game
resets when the player moves, chats, uses an item or clicks, so a plugin marks a player who is
away. The shared spawn of the Nether and the End is the one of the Overworld, since Minecraft
keeps one spawn for the worlds of a server. The registry id of an item comes from the registry of
Forge, such as `minecraft:diamond`.

`safeLocation` answers the nearest place near the block of the target where a player stands
safely, with the search that the mod writes itself and copies from no plugin. A place is safe when
the block below the feet carries a player on its full top face and is no block that hurts, the
blocks of the feet and the head have no collision shape, hold no fluid, are no portal of the tag
`minecraft:portals`, the nether portal, the end portal and the end gateway, which would carry the
player away, no cobweb, which would hold them, and do not hurt, the ground lies at or above the
lowest block of the dimension, the head below its top, and a player in the middle of the block
inside the world border, with half the width of a player, 0.3 blocks, to spare, since the border
of `/worldborder center` runs through the middle of a block. A
target below the logical height of its dimension, the lowest block plus `logical_height` of the
type of the dimension, keeps the head below that height too, as EssentialsX does, so a search
from under the bedrock roof of the Nether, whose logical height ends at y 128 while its blocks go
up to y 256, never lands on top of the roof. A target at or above it searches up to the top.
The blocks that hurt are fire, soul fire, campfires, cactus, magma, sweet berry bushes, wither
roses, powder snow, pointed dripstone and lava, and every block, of a mod as well, whose type of
path for the mobs of Forge, `getBlockPathType`, names a danger, such as `DAMAGE_FIRE`,
`DAMAGE_OTHER` or `LAVA`, or is `POWDER_SNOW` or `DAMAGE_CAUTIOUS`, as the mobs avoid it. The search looks at every block
within the sphere of the radius around the block of the target, at most 8 blocks, in this order:
the nearest first, then the higher, then the lower `x`, then the lower `z`, so a world always gives
the same answer. It reads at most 2109 places, and only those of loaded chunks: at the first
place in its order whose chunk is not loaded, before it found a safe one, it stops and answers
`unloaded`, so an answer with a place is always the answer of the whole search. A question never
loads a chunk, since a load on the server thread waits for the workers of the server, which may
wait for the handler of Golem that asks, and so could stall the tick or hang the server. A
plugin that wants a place in a chunk that is not loaded asks again once a player or a ticket of
the game loads it.

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
selector. Mods ask on any thread, so the native runs beside the server thread as well.

## Actions

`drain()` answers each action as one string, its fields joined by the character U+001F, the unit
separator, which no field may hold. The first field is the kind.

| Kind | Fields after the kind | What the mod does on the server thread |
| --- | --- | --- |
| `command` | The command without its slash | Runs it as the console, through the command source stack of the server |
| `message` | The UUID of a player, the text | Sends the text as a system message to that player, if they are online |
| `broadcast` | The text | Sends the text as a system message to every player and the console |
| `teleport` | The UUID of a player, the dimension, `x`, `y`, `z`, `yaw`, `pitch` | Teleports the player there, if they are online, also into another dimension |
| `listName` | The UUID of a player, the name | Shows the name for the player in the player list, if they are online, or clears it when the name is empty |

A number travels as the shortest decimal that reads back as the same number, which `Display` of
Rust writes and `Double.parseDouble` and `Float.parseFloat` of Java read, and an infinity as Java
spells it, `Infinity` or `-Infinity`, since `Display` writes `inf`, which Java does not read. A
teleport does what the
command `tp` does, step by step as `TeleportCommand.performTeleport` of Minecraft 1.20.1: it posts
`EntityTeleportEvent.TeleportCommand` of Forge, so a mod may cancel it or move its target, skips a
place outside the bounds of the world, `Level.isInSpawnableBounds`, which are x and z from
-30000000 to below 30000000 and y from -20000000 to below 20000000, wraps the yaw to -180 to 180
and the pitch to -90 to 90, and calls the `teleportTo` of `tp`, which keeps the chunk of the place
loaded for a moment with the ticket `post_teleport`, lets a rider dismount, wakes a sleeping
player, turns the head, keeps the camera of a spectator within one dimension and moves the player
into another dimension, which a mod may cancel on `EntityTravelToDimensionEvent`. The fall stops
when the player is in the dimension of the place afterwards. It takes the place as it is, so a
plugin that wants a safe place asks the game for one first. A name of the player list is answered on `PlayerEvent.TabListNameFormat` after
every other listener, so it wins over the name of another mod, and `refreshTabListName` sends it
to every client. It holds until Golem clears it or the player leaves.

The mod logs an action of an unknown kind, or of a known kind with the wrong number of fields,
once and skips it, and so a teleport into a dimension the server does not have, once per
dimension, to a place whose numbers do not read, that is no finite number or that lies outside
the bounds of the world, and a teleport that a mod cancelled. Each action runs on its own, so an action that
throws, such as a message to a UUID that does not parse, is logged and the next action runs. The
host stops the server with the actions `command save-all flush` and `command stop`, so the stop
path is the same as an operator's at the console.

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
  public methods of `Host` are the 16 natives of the first Java block above, each with the
  descriptor that the notebook derives from that block and that `javap` confirms, and the plain
  `available()`, and those of `Game` are the 7 abstract methods of the block of
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
server thread and once from the worker thread of a chat message, where it answers null. The
notebook records the thread of every call, and the threads of
[the rules of the server thread](#the-rules-of-the-server-thread) come from it.

Related: [JVM host](jvm-host.md), [Plugins](plugins/index.md), [Index](index.md).
