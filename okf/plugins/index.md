---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Spec
title: Plugins
description: The plugin runtime of Golem, the handler of the bridge that calls the Rust plugins on the server thread in their order, times every call against a budget, catches their panics and routes their commands.
tags: [plugins, runtime, budget, commands, bridge]
timestamp: 2026-11-06
---

# Plugins

A Golem plugin is Rust code that the plugin runtime calls on the server thread, for the events of
the [bridge](../bridge.md) and for the commands it declares. The runtime lives in the crate
`golem-plugin` and is the handler that `golem-bridge` hands every native method to. A plugin that
only observes the game uses the subscriptions of the GraphQL API instead, since a call on the
server thread holds up the tick.

Forge 47 calls `chat()` on a worker thread and `commandNames()` on the main thread at startup, as
the bridge page says. The handler sits behind one lock, so the runtime still runs one call at a
time: a call of another thread waits until the call before it ends, and a chat message that
arrives during a tick reaches the plugins after it. A call of a native from inside the handler
answers its pass value at once with a line on the log, since it would wait for itself.

| Part | Where | What it is |
| --- | --- | --- |
| `Plugin` | `golem_plugin::Plugin` | The trait of a plugin: its name, its commands, `on_event` and `on_command` |
| `Runtime` | `golem_plugin::Runtime` | The handler of the bridge, which calls the plugins and keeps their budgets |
| `Context` | `golem_plugin::Context` | What a plugin may do during its call: queue actions and read the budgets |
| `Budgets` | `golem_plugin::Budgets` | The shared handle on the time every plugin costs |
| Core plugin | `golem_plugin::CorePlugin` | The command `golem`, see [Core plugin](core.md) |

## Wiring

The program golem builds one runtime, adds its plugins and installs it before the JVM starts.
Its budget is `DEFAULT_BUDGET` unless `golem run --budget-ms` names another, and its source names
the commit that `build.rs` of golem reads from `GOLEM_COMMIT` or from `git rev-parse HEAD`, so a
build of a checkout names its commit. `Source::of_build()` of this crate reads `GOLEM_COMMIT`
alone and names the commit unknown without it.

```rust
let mut runtime = Runtime::new(budget);
runtime.add(Box::new(CorePlugin::new(Source {
    version: VERSION.to_owned(),
    repository: REPOSITORY.to_owned(),
    commit: COMMIT.to_owned(),
})));
let actions = golem_bridge::install(Box::new(runtime));
```

`install` moves the runtime into the global state of the bridge and answers the queue of actions,
which the signal handler of the host shares to queue `save-all flush` and `stop`. A second
`install` replaces the handler and keeps the queue with the actions in it. The handle from
`budgets_handle` stays valid after the runtime is installed, so the API reads the budgets through
it.

## Events

The runtime calls every plugin in the order of `add`, each with the event and a context.

| Event | Native method | Can be cancelled | Verdict of the runtime |
| --- | --- | --- | --- |
| `ServerStarting` | `serverStarting` | No | Pass |
| `ServerStarted` | `serverStarted` | No | Pass |
| `ServerStopping` | `serverStopping` | No | Pass |
| `ServerStopped` | `serverStopped` | No | Pass |
| `Tick` | `tick` | No | Pass |
| `PlayerJoined` | `playerJoined` | No | Pass |
| `PlayerLeft` | `playerLeft` | No | Pass |
| `Chat` | `chat` | Yes | Cancel at the first plugin that cancels, else pass |
| `Death` | `death` | No | Pass |
| `Respawn` | `respawn` | No | Pass |

`Event::Death` carries the `Location` where the player died: the dimension, the coordinates of
the feet and the rotation, so a plugin such as the port of `back` keeps the place without asking
the game.

For a chat message the first plugin that answers `Cancel` ends the chain: the plugins after it
never see the message, and the native answers true, so the mod cancels `ServerChatEvent`. Every
other event reaches every plugin, and the runtime answers a pass whatever the plugins answer.

A plugin that panics is caught with `catch_unwind`. The runtime logs the panic with the name of
the plugin and the event, counts it in the budget of the plugin and goes on as if the plugin
passed, so the plugins after it still see the event. A panic never reaches the bridge, and the
bridge would answer the pass value if one did.

## Budgets

Every call of a plugin, for an event or a command, is timed with `Instant` into the budget of that
plugin.

| Field | What it counts |
| --- | --- |
| `calls` | The calls of the plugin |
| `total_nanos` | The time of all calls, in nanoseconds |
| `max_nanos` | The longest call, in nanoseconds |
| `over_budget` | The calls that took longer than the budget of the runtime |
| `panics` | The calls that panicked |

The budget of one call is the argument of `Runtime::new`. `DEFAULT_BUDGET` is 1 ms, so many
plugins fit into a tick of 50 ms that the whole game shares. A call over the budget is logged at
most once a minute per plugin, `WARNING_INTERVAL`, so a plugin that is slow on every tick does not
fill the log, while `over_budget` keeps counting every such call.

`Budgets` is a cloneable handle on the table of the runtime. `limit` answers the budget of one
call and `snapshot` the name and the budget of every plugin in their order. The runtime takes the
lock of the table only to count a call, never while a plugin runs, so a plugin may read the table
during its own call through `Context::budgets`, as `/golem status` does.

## Commands

The runtime reads the commands of a plugin once, when the plugin is added. The first plugin that
declares a name owns it. A later plugin that declares the same name is logged and refused, and so
is a name that is empty or holds a space, a control character or a slash, since the mod registers
every name as a literal of Brigadier. `commandNames()` answers the names in the order of the
plugins and of their declarations.

| Case | Answer of `command()` |
| --- | --- |
| The name has an owner | The lines of `on_command` of the owner, timed into its budget |
| The name is unknown | `The command <name> is unknown.` |
| The owner panics | `The command <name> failed, and the log of the server holds the cause.` |

The sender of a command follows the source of the command, as [Bridge](../bridge.md#the-host-class)
says. A null `arguments` reads as the empty string.

| Sender | Source |
| --- | --- |
| `Sender::Player(player)` | A player, also when the console or a command block runs the command as them |
| `Sender::Console` | The console and RCON, with no entity at the level `MAX_LEVEL`, 4 |
| `Sender::Other { name, level }` | Every other source, such as a command block at level 2 or a command that runs as an armor stand |

A plugin judges a player by who they are, through `golem-permissions` and their operator level,
and every other sender by its level. A source that the mod passes wrong, a UUID without a name,
neither of them below level 4 or a level outside 0 to 4, is refused with an empty answer instead
of running as the console, which may do more than any other source.

## The context of a call

A plugin never touches the game. It queues actions through its context, and the mod runs them on
the server thread at the end of the tick, oldest first, in the wire form of
[Bridge](../bridge.md#actions).

| Helper | Action |
| --- | --- |
| `send_message(uuid, text)` | `message`, a system message to one player if they are online |
| `broadcast(text)` | `broadcast`, a system message to every player and the console |
| `run_command(command)` | `command`, run as the console, with a leading slash dropped |
| `actions()` | The queue itself, for what the helpers do not cover |
| `budgets()` | The budgets of the runtime that makes the call, or none outside a runtime |

A field of an action that holds U+001F, the separator of the wire form, has it replaced by a space,
and the first such replacement of the process is logged.

## Tick statistics

The native `tick(nanos)` records every tick with its duration and the instant of the call in a
window of the last 100 ticks, before the plugins see the tick. `golem_bridge::stats()` answers
`TickStats` from that window.

| Field | Value |
| --- | --- |
| `ticks` | The ticks since the start |
| `mspt` | The mean of the durations of the window, in milliseconds |
| `tps` | The intervals of the window divided by the seconds from its first to its last tick, at most 20, and 0 below 2 ticks |

A negative duration, which the mod never sends, counts as 0. The cap holds the TPS at 20 because
the server waits out the rest of a tick that ends early.

The duration runs from the first listener of `ServerTickEvent` at the start of a tick, the one of
the bridge, to its last at the end, so it covers what the mods do at both ends of the tick.
Minecraft stops its own clock of a tick, which `forge tps` reads, before the listeners at the end
run, so the MSPT of Golem reads higher. In the idle of the modpack the two read 1.96 and 1.60 ms,
see [Modpack](../modpack.md).

## The log

Every line of the bridge and the runtime goes to the standard error, which is the log of the
server, behind the prefix `[golem]`.

| Line | When |
| --- | --- |
| `the plugin <p> panicked on the event <e> with "<message>", so the runtime goes on as if it passed` | A plugin panics on an event |
| `the plugin <p> took <t> ms on the event <e>, over the budget of <b> ms, ...` | A call over the budget, once a minute per plugin |
| `the plugin <p> declares the command <c>, which the plugin <o> owns, so the runtime ignores it` | A duplicate command |
| `the native <n> panicked with "<message>", so it answers the pass value` | A panic that reached a native |
| `the native <n> got null for <field>, so it answers the pass value` | A null where the contract names a string |

## Tests

The notebook `tests/notebooks/04_plugins.ipynb` drives the runtime, the core plugin and the
functions behind the natives through `golem-devtool`. The subcommand `plugins` runs a script of
native calls with test plugins that broadcast, cancel, panic and sleep, `actions-encode` prints the
wire form of actions, `tick-stats` feeds ticks at given instants to a window and `natives` prints
the table that `register_natives` hands the JVM. The natives themselves run only inside a JVM:
`tests/notebooks/07_forge_boot.ipynb` boots the Forge server under `golem run` with the bridge mod,
so a player, the console and both stop paths go through the real natives.

Related: [Bridge](../bridge.md), [Core plugin](core.md), [Index](../index.md).
