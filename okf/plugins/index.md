---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Spec
title: Plugins
description: The plugin runtime of Golem, the handler of the bridge that calls the Rust plugins on the server thread in their order, times every call against a budget, catches their panics and routes their commands.
tags: [plugins, runtime, budget, commands, bridge]
timestamp: 2026-11-16
---

# Plugins

A Golem plugin is Rust code that the plugin runtime calls on the server thread, for the events of
the [bridge](../bridge.md) and for the commands it declares. The runtime lives in the crate
`golem-plugin` and is the handler that `golem-bridge` hands every native method to. A plugin that
only observes the game will use the subscriptions of the GraphQL API instead, since a call on the
server thread holds up the tick. The [Plugin API](../api.md) designs that API, and it is not built
yet.

Forge 47 calls `chat()` on a worker thread and `commandNames()` on the main thread at startup, as
the bridge page says. The handler sits behind one lock, so the runtime still runs one call at a
time: a call of another thread waits until the call before it ends, and a chat message that
arrives during a tick reaches the plugins after it. Once the server thread is known, a call of
another thread waits at most one second, `golem_bridge::host::OFF_THREAD_WAIT`, and then answers
its pass value, so the chat message passes unseen and a permission falls to its default resolver,
with a line on the log at powers of ten. A question of the server thread may wait for a worker of
the server, which may be the thread of that chat message, and a wait without end would keep both
waiting. A call of a native from inside the handler answers its pass value at once with a line on
the log, since it would wait for itself.

| Part | Where | What it is |
| --- | --- | --- |
| `Plugin` | `golem_plugin::Plugin` | The trait of a plugin: its name, its commands, `on_event` and `on_command` |
| `Runtime` | `golem_plugin::Runtime` | The handler of the bridge, which calls the plugins and keeps their budgets |
| `Context` | `golem_plugin::Context` | What a plugin may do during its call: queue actions, ask the game and read the budgets |
| `Budgets` | `golem_plugin::Budgets` | The shared handle on the time every plugin costs |
| Core plugin | `golem_plugin::CorePlugin` | The command `golem`, see [Core plugin](core.md) |

## Wiring

The program golem builds one runtime, gives it the store of the plugins and the resolver of the
permissions of that store, adds its plugins and installs it before the JVM starts, as
[Store](../store.md#the-place-in-the-program-golem) says. A set of the store that
`golem-permissions` refuses stops the start with `golem: the permissions of the store <path> do not
load: <reason>`. Its budget is `DEFAULT_BUDGET` unless `golem run --budget-ms` names another, and
its source names the commit that `build.rs` of golem reads from `GOLEM_COMMIT` or from HEAD of the
git checkout of Golem itself, with `-dirty` behind it when the files that make the binary differ
from HEAD, as [JVM host](../jvm-host.md#golem-run) says. `Source::of_build()` of this crate reads
`GOLEM_COMMIT` alone and names the commit unknown without it.

```rust
let resolver = Resolver::from_store(&store)?;
let mut runtime = Runtime::new(budget);
runtime.set_store(store.clone());
runtime.set_permissions(resolver.clone());
runtime.add(Box::new(CorePlugin::new(Source {
    version: VERSION.to_owned(),
    repository: REPOSITORY.to_owned(),
    commit: COMMIT.to_owned(),
})));
runtime.add(Box::new(PermissionsPlugin::new(resolver)));
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
| `Teleported` | `teleported` | No | Pass |
| `TeleportFailed` | `teleportFailed` | No | Pass |

`Event::Death` carries the `Location` where the player died: the dimension, the coordinates of
the feet and the rotation, so a plugin such as the port of `back` keeps the place without asking
the game. `Event::Teleported` carries the id of a teleport that a plugin queued, the UUID and
where the player was and is, so the port of `back` keeps the place before a move that worked, and
`Event::TeleportFailed` the id, the UUID and the `TeleportFailure`, as
[Bridge](../bridge.md#the-end-of-a-teleport) names the reasons. Every plugin hears both, whichever
plugin queued the teleport, and tells its own teleports by their ids.

For a chat message the first plugin that answers `Cancel` ends the chain: the plugins after it
never see the message, and the native answers true, so the mod cancels `ServerChatEvent`. Every
other event reaches every plugin, and the runtime answers a pass whatever the plugins answer.

A plugin that panics is caught with `catch_unwind`. The runtime counts the panic in the budget of
the plugin and goes on as if the plugin passed, so the plugins after it still see the event. A
panic never reaches the bridge, and the bridge would answer the pass value if one did. The runtime
logs the panics of a plugin with its name, the event and the message the first time, the tenth,
the hundredth and at every further power of ten, as the guard of the mod logs a fault, so a plugin
that panics on every tick cannot fill the log, while the budget counts every panic. The first
`Runtime` of a process installs a panic hook that stays silent while a call of a plugin runs on
the thread and hands every other panic to the hook it replaced, so the hook of the standard
library prints neither the place of such a panic nor, under `RUST_BACKTRACE`, its backtrace.

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
plugins and of their declarations. A name that the game or a mod holds goes to the plugin, and the
command it replaces stays as `minecraft:<name>`, but for `setblock`, `summon`, `testforblock` and
`tellraw`, which keep the command of the game, as [Bridge](../bridge.md#commands) says.

| Case | Answer of `command()` |
| --- | --- |
| The name has an owner | The lines of `on_command` of the owner, timed into its budget |
| The name is unknown | `The command <name> is unknown.` |
| The owner panics | `The command <name> failed, and the log of the server holds the cause.` |

The sender of a command follows the source of the command, as [Bridge](../bridge.md#the-host-class)
says. A null `arguments` reads as the empty string.

| Sender | Source |
| --- | --- |
| `Sender::Player { player, level }` | The player who made the command run, at the level of the source: the one who typed it or clicked the sign that runs it, also when it runs as another entity through `execute as` |
| `Sender::Console` | The console and RCON, with no entity at the level `MAX_LEVEL`, 4 |
| `Sender::Other { name, level }` | Every other source, such as a command block at level 2, or a command that the console or a command block runs as an armor stand or as a player through `execute as`, named after that entity |

A plugin judges a sender with `Context::check_permission` and `Context::has_permission`, see [the
check of a sender](permissions.md#the-check-of-a-sender): a player by their nodes in
`golem-permissions`, also when their command runs as another entity, and every other sender by its
level alone, with no node of any player, also when it runs as a player through `execute as`. So a
source at level 2 never borrows the rights of a player, as in vanilla, where `execute as` keeps the
level of the source, and as on the live server, where Arclight handed a Bukkit plugin the source and
not the player. A source that the mod passes wrong, a UUID without a name, neither of them below
level 4 or a level outside 0 to 4, is refused with an empty answer instead of running as the
console, which may do more than any other source.

## Permissions

A mod that asks the PermissionAPI of Forge for a node of the type Boolean reaches the runtime
through the native `permission`, as [Bridge](../bridge.md#permissions) says. The runtime asks the
plugins in their order with `on_permission(uuid, node, context)`, and the first plugin that
answers `Some` decides. A plugin that answers `None` leaves the node to the next one, and a
plugin that panics is logged, counted and passed over. When no plugin answers, the node falls to
its default resolver, which is what the default handler of Forge would answer. Every call is
timed into the budget of its plugin, as a command is. Mods ask on any thread, so a plugin answers
a permission from what it holds and never waits for the game. The native carries no dynamic
context of Forge, such as the dimension of a check, so the answer of a plugin holds in every
context in which a mod asks the node, and only the default resolver sees the context.

The program golem adds the plugin `golem-permissions`, `PermissionsPlugin`, after the core plugin.
It answers every Boolean node of an online player as LuckPerms 5.5.71 answered it through Arclight
on the live server, with `Permissions::has_permission` of the resolver: a node that a rule names
as the rule says, after it lowercased the node, and a node that nothing names true for an operator
and false for every other player, see
[Permissions](permissions.md#arclight-forwards-every-permission-of-a-mod-to-luckperms). An operator
is a player whose operator level is 1 or more, which every player on the list of operators has,
as `isOp` of Bukkit answers it, unless that list gives them the level 0. A player who is not online
gets `None`, so the node falls to its default resolver, as Arclight left `getOfflinePermission`
to the handler it wrapped. The native does not say whether Forge asked `getPermission` or
`getOfflinePermission`, so the plugin tells an online player by the game on the server thread and,
on every other thread, by the operator levels it keeps from the joins and the leaves and reads
from the players once a second while one is online, so a change of the operator level reaches a
check of another thread within a second. It asks the game nothing off the server thread.

The plugin also owns `/luckperms` and its aliases, the commands of [the port of
LuckPerms](permissions.md#the-commands-of-luckperms). A command reads the set of the store and its
generation as one, after it loaded the set when the generation of the store moved, runs on it,
writes every entry it changed to the store with `put_permission_group`, `remove_permission_group`
and `put_player_permissions`, in its order, and installs the changed permissions in the resolver
under the generation of before its writes, so the next check, also of the same tick, sees the
change, and the reload that the next tick wakes finds the same set. It writes through the handle of
the store that never waits, which the resolver keeps and which is the only one that it hands out,
and it asks `Store::room` for all of its changes before the first. So a command makes all of its
changes or none and never waits for the writer of the store: where the store has no room, the
command changes nothing, the plugin logs that, and the sender reads `[LP] The store of Golem takes
no change now, so nothing changed. Run the command again later.`. While the set of the store does
not load, the resolver keeps an older set for the checks, so a command changes nothing and answers
`[LP] The permissions of Golem do not load, so no command runs: <reason>.` to the console and to a
sender who holds a node of the commands by that older set. Every other sender reads the line without
its reason, since the reason can name a group, a node or the UUID of a player. When the store
refuses a change, the plugin logs why, loads the set of the store again and sends the line of
LuckPerms for a failed save, such as `[LP] There was an error whilst saving user data for Alex.`. On
every tick the plugin hands the resolver a look at the generation of the store, so a change that
another writer put into the store reaches the checks once the thread `golem-permissions` loaded it.

## The context of a call

A plugin never changes the game itself. It queues actions through its context, and the mod runs
them on the server thread at the end of the tick, oldest first, in the wire form of
[Bridge](../bridge.md#actions). It may ask the game during its call through `game()`, which
answers through the trait `golem_bridge::Game` of [Bridge](../bridge.md#the-game).

| Helper | Action |
| --- | --- |
| `send_message(uuid, text)` | `message`, a system message to one player if they are online |
| `broadcast(text)` | `broadcast`, a system message to every player and the console |
| `send_text(uuid, text)` | `styledMessage`, the `Text` of [Bridge](../bridge.md#styled-text) as a system message to one player if they are online, refused with a line on the log above `MAX_JSON` or `MAX_DEPTH`, and answers whether it queued it |
| `broadcast_text(text, hidden)` | `styledBroadcast`, the `Text` to every player but the UUIDs of `hidden` and to the console, refused as `send_text` refuses |
| `run_command(command)` | `command`, run as the console, with a leading slash dropped, whose answer reaches the log of the server and no operator in the game |
| `teleport(uuid, location)` | `teleport`, unless a coordinate or an angle is no finite number, the block lies outside the bounds of the world that `tp` takes, or the server stops, from `serverStopping`, or from `serverStopped` when the game did not announce the stop, to the next `serverStarting`, which is logged and refused, and answers whether it queued one |
| `queue_teleport(uuid, location, safe_radius)` | `teleport` with the next id of the process and the radius of the search for a safe place, held to 8, or `None` for the place as it is, refused as `teleport` refuses, and answers the id that the events `Teleported` and `TeleportFailed` carry, each id once, unless the JVM is halted or killed before, see [Bridge](../bridge.md#the-end-of-a-teleport) |
| `set_list_name(uuid, name)` | `listName`, the name of the player in the player list, cleared by an empty name |
| `clear_list_name(uuid)` | `listName` with an empty name, which clears it |
| `actions()` | The queue itself, for what the helpers do not cover |
| `game()` | The game, which answers on the server thread and answers none on any other thread |
| `budgets()` | The budgets of the runtime that makes the call, or none outside a runtime |
| `store()` | The store of the plugins that the program gave the runtime with `Runtime::set_store`, or none outside a runtime or without one. Its reads answer from its cache and its changes queue for its writer, so a call of the server thread waits for no disk. It is the handle that never waits for the writer either: a change that finds the writer as far behind as the store lets it fall answers `StoreError::Behind` and changes nothing, so a command tells its sender that the server is busy, and one that makes several changes asks `room` first. A plugin changes the store inside its calls alone, on no thread of its own, so that the answer of `room` holds, see [Store](../store.md#the-threads) |
| `check_permission(sender, node)` | Whether the sender holds the node, as `PermissionAnswer` with `granted`, `set`, which says whether a rule decided, and the `reason`, by the rules of [the check of a sender](permissions.md#the-check-of-a-sender) |
| `has_permission(sender, node)` | The same answer without the reason, for a player one call of `Permissions::value`, which allocates nothing, for the server thread |
| `permissions()` | The `Resolver` of the permissions that the program gave the runtime with `Runtime::set_permissions`, or none. Its `store()` is the handle of the store that never waits, as `store()` of the context is, and the resolver hands out no other |

Once every plugin saw `ServerStopped`, the runtime flushes the store, outside the budget of every
plugin, so every change of the plugins is in the file before the JVM ends, also when nothing closes
the store after it. A flush that fails is logged.

A field of an action that holds U+001F, the separator of the wire form, has it replaced by a space,
and the first such replacement of the process is logged.

## The game

`Context::game` answers what the game holds while the call runs on the server thread: every event
but a chat message and `ServerStopped`, every command and a permission that a mod asks on the
server thread. On any other thread, such as for a chat message, every question answers none, and
the log names the first such question of the process. On `ServerStopped` every question answers
none as well, since Minecraft closed every level before that event. Every answer is `None` when
the game cannot answer, and an answer that may be empty is an `Option` inside, so `?` takes the
outer one.

| Question | Answer |
| --- | --- |
| `players()` | Every player online as `OnlinePlayer`: the UUID, the name, the display name, the name in the player list, the ping in milliseconds, the `GameMode`, the operator level and the milliseconds since the last action |
| `player(who)` | The player online of that name or UUID in any case, or `Some(None)` |
| `location(uuid)` | The `Location` of an online player |
| `respawn(uuid)` | The respawn point that the player holds as a `RespawnPoint`, its `BlockLocation`, its angle and whether `/spawnpoint` forced it, which a bed, a respawn anchor or `/spawnpoint` set and which Minecraft checks only on a respawn, or `Some(None)` |
| `respawn_place(uuid)` | The `RespawnPlace` where the player would respawn now: `At` with the place and the yaw, `Nowhere` for the world spawn, or `Unloaded` when a chunk around the point is not loaded |
| `spawn(dimension)` | The shared spawn of a dimension as a `Spawn`, its block and angle |
| `main_hand(uuid)` | The `Item` in the main hand, its id and count, or `Some(None)` for an empty hand |
| `heights(dimension)` | The `Heights` of a dimension, the lowest block and the y above the highest |
| `safe_location(target, radius)` | The `SafePlace` of the search within the radius, at most 8: `Found` with the nearest safe place, with the dimension and the rotation of the target, `Nowhere`, or `Unloaded` when a chunk it must read is not loaded, since a question loads no chunk |

A question reads the game at once, through the JNI, and its time counts into the budget of the
plugin that asks, so a plugin asks what it needs and keeps no answer longer than its call, since
the game goes on. A test of a plugin makes its context with `Context::new`, whose game answers
nothing, or with `Context::with_game` and a game of its own.

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

The duration runs from the listener of the bridge of the highest priority on `ServerTickEvent` at
the start of a tick to its listener of the lowest priority at the end, so it covers what most mods
do at both ends of the tick. Forge runs the listeners of one priority in the order they registered
in, so another listener of the highest or the lowest priority may fall outside it, such as the
listener of the lowest priority of Citadel in the modpack. The bridge does its own work after it
reports a tick: the native `tick`, in which the plugins see the tick, `drain` and the actions. That
work delays the next tick as a listener does, so the bridge adds its time to the duration of the
next tick, and a plugin that is slow on a tick shows in the MSPT one tick later. Minecraft stops its
own clock of a tick, which `forge tps` reads, before the listeners at the end run, so the MSPT of
Golem reads higher. In the idle of the modpack the two read 1.96 and 1.60 ms, see
[Modpack](../modpack.md).

## The log

Every line that the bridge and the runtime log while a native runs, which is every line of a
running server but those of the host, goes through `log` of the game that the mod connected to the
logger `golem` of the server at the level `INFO`, so it stands in `logs/latest.log` and on the
console, as [Bridge](../bridge.md#the-game) says. A line before the JVM starts, such as a command
that the runtime refuses when `golem run` installs it, and a line of a tool without a JVM, such as
`golem-devtool`, go to the standard error behind the prefix `[golem]`.

| Line | When |
| --- | --- |
| `the plugin <p> panicked on the event <e> with "<message>", so the runtime goes on as if it passed, <count> times so far` | A plugin panics on an event, a command or a permission, at powers of ten per plugin |
| `the plugin <p> took <t> ms on the event <e>, over the budget of <b> ms, ...` | A call over the budget, once a minute per plugin |
| `the plugin <p> declares the command <c>, which the plugin <o> owns, so the runtime ignores it` | A duplicate command |
| `the native <n> panicked with "<message>", so it answers the pass value` | A panic that reached a native |
| `the native <n> got null for <field>, so it answers the pass value` | A null where the contract names a string |
| `the native <n> waited 1000 ms for the handler, which another call holds, so it answers the pass value, <count> times so far` | A call of another thread than the server thread that found the handler taken for a second, at powers of ten |

## The ports

A port of a Bukkit plugin is a Golem plugin that keeps the license of its upstream, as
[Licensing](../../Documentation/licensing.md) says. The port of LuckPerms is
[Permissions](permissions.md). The spec of the port of EssentialsX 2.22.0 and EssentialsXSpawn
2.22.0 quotes EssentialsX and keeps its license, the GPL 3 or later, in three pages:
[EssentialsX teleport port](essentials/teleport.md) for the teleport, home, warp and spawn
commands, [EssentialsX social port](essentials/social.md) for the messages, mail, info, AFK,
economy and admin commands, and [EssentialsX bridge needs](essentials/bridge-needs.md) for what
the ports need from the bridge.

## Tests

The notebook `tests/notebooks/04_plugins.ipynb` drives the runtime, the core plugin and the
functions behind the natives through `golem-devtool`. The subcommand `plugins` runs a script of
native calls with test plugins that broadcast, cancel, panic, sleep and reach a store that the
script names through the context, `actions-encode` prints the wire form of actions, `tick-stats`
feeds ticks at given instants to a window and `natives` prints the table that `register_natives`
hands the JVM. The natives themselves run only inside a JVM: `tests/notebooks/07_forge_boot.ipynb`
boots the Forge server under `golem run` with the bridge mod, so a player, the console and both stop
paths go through the real natives.

Related: [Bridge](../bridge.md), [Core plugin](core.md), [Permissions](permissions.md),
[EssentialsX teleport port](essentials/teleport.md), [Index](../index.md).
