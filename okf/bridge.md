---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Spec
title: Bridge
description: The contract between the Golem bridge mod in the JVM and the Rust of Golem, with the host class, its native methods, the wire form of the actions and the rules of the server thread.
tags: [bridge, jni, forge, contract]
timestamp: 2026-11-06
---

# Bridge

The bridge connects the game, which runs unchanged in the JVM, with the Rust of Golem. It has three
parts, and this page is the contract between them.

| Part | Where | What it is |
| --- | --- | --- |
| Host class | `bridge/host/`, the jar `golem-host.jar` | One Java class with the native methods, no Forge in it |
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
	public static native String[] drain();
}
```

`available()` is plain Java. It calls `version()` once and answers false when that throws an
`UnsatisfiedLinkError`, so the mod switches itself off with one warning on a server that Golem
does not run, instead of failing on every event.

| Native method | Called by the mod | Answer |
| --- | --- | --- |
| `version()` | Once, through `available()`, while Forge constructs the mod | The version of Golem, such as `0.1.0` |
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
| A player, also when another source runs the command as them | The UUID of the player | The name of the profile | The level of the source |
| The console and RCON: no entity at level 4 | Null | Null | 4 |
| Every other source: a command block, a function, a command that runs as an entity that is no player | Null | The name of the source, such as `@` for a command block | The level of the source |

The natives answer a player as `Sender::Player`, the console and RCON as `Sender::Console` and every
other source as `Sender::Other` with its name and level. A command that runs as a player through
`execute as` reaches the plugins as that player, as Bukkit hands it to a plugin, so a plugin judges
the player and not the source that made the command run. A function that runs at level 4 has no
entity and the rights of the console, so it reads as the console. The natives refuse the command of
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
Rust writes and `Double.parseDouble` and `Float.parseFloat` of Java read. A teleport does what the
command `tp` does: it keeps the chunk of the place loaded for a moment with the ticket
`post_teleport`, wakes a sleeping player, lets a rider dismount, stops the fall and wraps the yaw
to -180 to 180 and the pitch to -90 to 90, and `ServerPlayer.teleportTo` moves the player into
another dimension. It takes the place as it is, so a plugin that wants a safe place asks the game
for one first. A name of the player list is answered on `PlayerEvent.TabListNameFormat` after
every other listener, so it wins over the name of another mod, and `refreshTabListName` sends it
to every client. It holds until Golem clears it or the player leaves.

The mod logs an action of an unknown kind, or of a known kind with the wrong number of fields,
once and skips it, and so a teleport into a dimension the server does not have, once per
dimension, and to a place that is no finite number. Each action runs on its own, so an action that
throws, such as a message to a UUID that does not parse, is logged and the next action runs. The
host stops the server with the actions `command save-all flush` and `command stop`, so the stop
path is the same as an operator's at the console.

## The rules of the server thread

- Every native method runs on the thread that calls it. That is the server thread for every
  method of this page but three, as Forge 47.4.18 calls them and the notebook `05_bridge_build`
  records. `version()` runs on a thread of the mod loader, `modloading-worker-<n>`, while Forge
  constructs the mods. `commandNames()` runs on the main thread, `main`, when Forge builds the
  commands at the start, and on the server thread on a reload. `chat()` runs on a worker thread of
  Forge, `Worker-Main-<n>`, while the server thread goes on, since Forge fires `ServerChatEvent`
  there.
- A handler in Rust answers before the event goes on, so it must be quick. The plugin runtime times
  every call against its budget.
- Rust never touches game state from another thread. Whatever Rust wants done is an action. A
  handler of `chat()` runs beside the server thread, so it must be thread safe and may only answer
  and queue actions. Its answer still cancels the message.
- A native method never lets a panic of Rust cross into the JVM. It catches it, logs it and
  answers what a pass would answer: nothing, false, or an empty array.
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

- The host jar holds exactly one class, `com/skytekx/golem/host/Host.class`, of class file version
  61, and depends on `java.lang` alone. Its manifest names the module, the version of Golem and
  the license. Its public methods are the 14 natives of the Java block above, each with the
  descriptor that the notebook derives from that block and that `javap` confirms, and the plain
  `available()`.
- The mod jar holds the mods.toml and the `pack.mcmeta` of above, no class of the host package and
  no `.license` file. It listens for every event of the table and calls every native but
  `version()`, with the descriptors of the host jar. Every member of Minecraft it uses carries its
  SRG name, but the methods of `java.lang.Object`, and no official name of Mojang appears.

It then boots a copy of the Forge server of `tests/fetch.py` on port 25633 three times, through
the java launcher of the JDK with the argument files of the server, as `run.sh` does. Each boot
must reach `Done`, stop cleanly with no FATAL line and no crash report, and find the mod.

| Boot | The mod must |
| --- | --- |
| Without the host jar | Log one warning that the host class is missing, and nothing else |
| With `golem-host.jar` on the `-p` of a copied `unix_args.txt` | Log one warning that the natives are not registered |
| With the host jar and stand in natives | Call every native, run every action, and stop the server through the actions |

The stand in natives are a small C library that the notebook compiles and loads with
`-agentpath`, since HotSpot searches the symbols of an agent library for a native method that no
library of its class loader holds. A minimal offline client of Minecraft 1.20.1 joins, chats,
runs a command of the plugins, dies, respawns and leaves. The notebook records the thread of every
call, and the threads of
[the rules of the server thread](#the-rules-of-the-server-thread) come from it.

Related: [JVM host](jvm-host.md), [Plugins](plugins/index.md), [Index](index.md).
