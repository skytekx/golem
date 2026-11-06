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
	public static native void death(String uuid, String name, String cause);
	public static native void respawn(String uuid, String name, String dimension);
	public static native String[] commandNames();
	public static native String[] command(String name, String senderUuid, String senderName,
			String arguments);
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
| `death(uuid, name, cause)` | On `LivingDeathEvent` of a `ServerPlayer`, with the id of the damage type | None |
| `respawn(uuid, name, dimension)` | On `PlayerRespawnEvent`, with the id of the dimension | None |
| `commandNames()` | On every `RegisterCommandsEvent`, the first before `serverStarting` | The names of the commands of the plugins |
| `command(name, uuid, playerName, arguments)` | When a command of `commandNames()` runs | The lines the sender sees |
| `drain()` | At the end of every server tick, after `tick` | The queued actions, oldest first |

A UUID is in its usual form with dashes. For a command of the console, `senderUuid` and
`senderName` are null. `arguments` is everything after the name and one space, or the empty string.

## Commands

The mod registers every name of `commandNames()` with Brigadier as a literal that runs on its own
and as the same literal with one argument `arguments` of the type greedy string. Both run
`command`, and the mod sends every line of the answer to the source of the command as a system
message. A command of a plugin needs no permission level in Brigadier: the plugin checks the
sender itself, through `golem-permissions`.

Forge builds the commands when it loads the data packs: once at the start, before
`ServerStartingEvent`, and again on every `/reload`. So `commandNames()` answers before
`serverStarting()`, and the plugins must know their commands by then. A name that is empty or holds
whitespace cannot be typed as the first word of a command, so the mod skips it with a warning. A
command block, a function and every other source that is not a player run as the console, with a
null sender. When `command` throws, the sender reads that the command failed and the log of the
server holds the exception.

## Actions

`drain()` answers each action as one string, its fields joined by the character U+001F, the unit
separator, which no field may hold. The first field is the kind.

| Kind | Fields after the kind | What the mod does on the server thread |
| --- | --- | --- |
| `command` | The command without its slash | Runs it as the console, through the command source stack of the server |
| `message` | The UUID of a player, the text | Sends the text as a system message to that player, if they are online |
| `broadcast` | The text | Sends the text as a system message to every player and the console |

The mod logs an action of an unknown kind, or of a known kind with the wrong number of fields,
once and skips it. Each action runs on its own, so an action that throws, such as a message to a
UUID that does not parse, is logged and the next action runs. The host stops the server with the
actions `command save-all flush` and `command stop`, so the stop path is the same as an operator's
at the console.

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

Related: [JVM host](jvm-host.md), [Plugins](plugins/index.md), [Index](index.md).
