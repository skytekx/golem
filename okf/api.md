---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Spec
title: Plugin API
description: The plugin API of Golem, one GraphQL schema on Aosabi that covers an equivalent of the whole Bukkit API of spigot-api 1.20.1, with native Rust plugins compiled into golem that reach it through the executor of Aosabi on the thread of the caller, the mapping that generates the schema, the contract and the client, the threads, the synchronous answer of cancellable events, remote clients, the coverage check and the work breakdown.
tags: [api, graphql, aosabi, plugins, bukkit, events, coverage]
timestamp: 2026-11-06
---

# Plugin API

The plugin API of Golem is one GraphQL schema that Aosabi serves inside the program `golem`. A
plugin reads the game with queries, changes it with mutations and hears events through
subscriptions, and tools and operators reach the same schema from outside the process. The schema
covers an equivalent of the whole Bukkit API of `spigot-api` 1.20.1-R0.1-20230921.163938-66, Spigot
included: 1,247 types, 7,942 public methods, 5,900 fields and constants and 268 events, so that
every Bukkit or Spigot plugin converts to a Golem plugin. Plugins are native Rust on Aosabi and use
nothing else, as the owner ruled on 2026-11-06 ([Decisions](decisions.md)).

This page is the one design that the two syntheses of the panel of 2026-11-06 settle into, each
point where they differed ruled on its evidence. Nothing of it is built yet. [Index](index.md) says
what is built, and [the work breakdown](#20-work-breakdown) says in which order the rest comes.
Every duration on this page is an estimate that nobody measured, and every measured number names
the spike or the notebook, the machine and the day. [API coverage](api-coverage.md) holds the exact
format of the mapping files.

## 1. The design in short

1. One schema. The crate `golem-api` is the Aosabi app of Golem, plain Rust from which the builder
   of Aosabi derives the SDL, committed as `api/schema.graphql`. One `Service` answers plugins,
   tools and operators.
2. Plugins are crates compiled into `golem`, as `ports.toml` already assumes. Golem loads no library
   and no jar at run time.
3. Every schema coordinate has one Rust function that holds its meaning, in the plain crate
   `golem-model`, or in `golem-events` for the fields of an event. The resolver in `golem-api` and
   the method of the plugin client `golem-client` both call it, so the two ways in cannot drift
   apart and no state of the game exists twice.
4. Every call of a plugin is an operation of the schema that runs through the executor of Aosabi in
   process, on the thread of the caller, as the owner ruled. A method of the client runs the
   canonical operation of one coordinate, a document of one field that the build validates and the
   runtime prepares at `enable`, through the `Service` of `golem-api`, whose resolver calls the
   coordinate function, so a Bukkit getter such as `getLocation()` costs one pass of the executor
   and one JNI call with primitive arguments and an out buffer. Until item A2 of Aosabi lands, that
   pass parses the text with validation off, and A2 then runs it as `Operation::Persisted` from the
   document table of Aosabi. A document of several fields pays the executor once for all of them.
   Golem writes no plan and no compiler of documents of its own.
5. One mapping file per Bukkit type, `api/bukkit/<package>/<Type>.toml`, keyed by the exact lines
   of `inventory.json`, names the coordinate of every member and how it is built: a Java expression
   from which a generator writes the contract call, the Java, the Rust stub, the coordinate
   function, the resolver and the client method, or a function written by hand where the meaning of
   Bukkit needs Rust. Most getters and setters need no hand written code.
6. The contract between Rust and the bridge mod is generated from the mapping and checked in. It
   replaces the strings joined by U+001F of the first game interface.
7. Game code runs on the server thread. Event handlers, commands, tab completion and sync tasks of
   plugins already run there inside a native call of the bridge, so a coordinate calls Java at once,
   with no queue and no wait. Other threads read the coordinates that are safe anywhere and a
   snapshot written once per tick, and hop to the server thread for the rest.
8. A cancellable event is a synchronous subscription. The mod writes the event into a frame and
   calls Rust once, the dispatcher runs the subscribers in the order of Bukkit with
   `ignoreCancelled`, each subscriber changes the frame through the mutations of the event, and the
   native answers the changes in the same call. Dispatch nests up to a depth of 16, as Bukkit events
   nest.
9. No thread of the game ever waits for a plugin outside a native call it made itself. The chain of
   the async chat event runs on a thread of Golem, composed onto the chat future of Forge, so no
   worker of world generation waits for the server thread, and the chains of the login and of the
   ping run on a thread of Golem as well, so no thread of Netty runs code of a plugin.
10. An event that nobody hears costs one array load, and the filters of its subscribers fold into
    interest tables that the Java hook checks before it writes a frame.
11. Remote clients see events after the chain, read only, from a copy of the frame and the tick
    snapshot, and never hold up the tick. Their operations run whole on the server thread within a
    budget per tick, each held before it runs to a cost estimate and an answer size, and while it
    runs to a deadline of its own, and none of them loads a chunk.
12. Plugins are contained: a budget per call, a deadline past which a slow call is named and, only
    for a plugin that asks for it, every further call into golem fails, a watchdog that names a
    hang, a degrade mode, a quarantine for the next start, and a ledger that releases everything a
    plugin owns when it is disabled, with no wait of the server thread.
13. Remote clients authenticate with a token or with the peer check of a Unix socket, and every
    coordinate is a permission node of `golem-permissions`, such as `golem.api.player.location`.
14. Coverage is three numbers that CI holds so that they only rise: mapped, present in the SDL, and
    proven by a document of the conformance corpus on a booted server.
15. Bukkit and Spigot plugins also run unchanged through a Bukkit layer in the JVM of Forge, as the
    owner ruled. For the public API the layer is one more client of this schema in process,
    generated from the same mapping, and for the internals of CraftBukkit and the classes of
    Minecraft it reaches the game beside the schema, see [section 17](#17-the-bukkit-layer). It is
    no form of a Golem plugin.
16. Measured on 2026-11-06: a document of seven fields costs about 13 µs through `Service::execute`
    of Aosabi with validation off and about 30 µs with the 27 rules, and a document of one field
    about 7 to 8 µs with validation off, against about 100 ns for a JNI call. That gap is the price
    of point 4, which item A2 and the records of 6.3 lower and the gates measure first.

## 2. What exists and what this design makes of it

| Part today | What it does | What it becomes |
| --- | --- | --- |
| The natives of `Host` in `golem-bridge` | Events as strings, a handler behind one lock, a native called from inside the handler answers its pass value | The natives stay as the entry, with `serverAboutToStart`, `event`, `complete`, `runHops`, `apiSlot`, `call` and the callback of a closure during a call added. The global lock goes, since Bukkit events are re-entrant, and dispatch nests on the server thread |
| Actions joined by U+001F, drained at the end of the tick | The only way Rust changes the game | A bounded queue for unit mutations from other threads. A mutation on the server thread runs at once, as in Bukkit |
| `Game.java` and `golem_bridge::Game`, the first game interface | Seven queries answered as strings joined by U+001F through `Host.connect(Game)`, the actions `teleport` and `listName`, the PermissionAPI handler and the level of a command sender | The seed of the mapping. Its calls move onto generated calls with primitive arguments, and `Context::game()` answers through the client until the ports leave it, see [section 16](#16-the-essentialsx-port-and-the-first-game-interface) |
| `golem-plugin::Plugin` | `on_event`, `on_command`, `on_permission`, 10 events, budgets | A plugin gets `load`, `enable` and `disable` and registers subscriptions, commands, tasks, permissions, services and documents in `enable`. Budgets stay, with a count per coordinate |
| `golem-permissions` | Groups, nodes, the seed | The resolver of `Permissible`, with attachments and defaults, the handler of the Forge PermissionAPI and the nodes of the API coordinates |
| `golem-store` | The SQLite store of the plugins | Also holds the hashes of the API tokens and the audit log |
| The schema sketch of [Plan](plan.md#the-graphql-api) | Six queries, seven mutations, five subscriptions | Folds into this schema: `runCommand` is `Mutation.serverDispatchCommand`, `teleport` is `Mutation.entityTeleport`, `chat` is `Subscription.asyncPlayerChat`, `ticks` is `Subscription.serverTick` |
| The notebooks `04_plugins`, `05_bridge_build`, `07_forge_boot`, `08_modpack_boot` | The runtime, the jars and the boots | Stay. The API adds the notebooks from `10` on |

## 3. Layers and crates

```text
 plugins compiled into golem                      tools, operators, process plugins
   golem-client: one method per coordinate          documents over GraphTP or HTTP
   ops.rs: documents with typed answers                       |
        |             | every call                 golem-remote: thread golem-api, carriers,
        |             v                            tokens, limits, event streams. Operations
        |      golem-api: the Aosabi Service  <---------+  go whole to the server thread
        |        roots, generated types, the schema parts of the ports
        v frame reads v resolvers
     golem-events: frames, dispatcher, chains, interest tables, the functions of event fields
     golem-model: one function per coordinate, value types, mirrors, the tick snapshot
     golem-game: ids, server token, generated stubs, buffers, hops              unsafe
        |
  ------ JNI ----------------------------------------------------------------------------
   bridge/host: the natives of Host and the generated contract interfaces
   bridge/mod:  generated and hand written Java over Minecraft and Forge, Mixins, frames
   the Bukkit layer: Java in the JVM of Forge, a client in process for level 1, section 17
   Minecraft 1.20.1, Forge 47.4.18, 265 mods, unchanged
```

| Crate | Holds | Unsafe | License |
| --- | --- | --- | --- |
| `golem` | The program, `golem run`, the list of compiled plugins | No | AGPL-3.0-only |
| `golem-jvm` | The JVM host, unchanged | Yes, the invocation API | AGPL-3.0-only |
| `golem-bridge` | The natives of `Host` and the `Handler` trait that the runtime fills | Yes, through the safe wrapper of `jni` | AGPL-3.0-only |
| `golem-game` | `EntityRef`, `WorldRef` and the other ids, `Server<'t>` the server token, the stubs that `contract gen` writes with method ids cached at `connect`, safe views of the frame stack and the out buffer of each thread, the hop queue and the waker | Yes, each block with `// SAFETY:` | AGPL-3.0-only |
| `golem-model` | The coordinate functions, generated for the Java expressions of the mapping and hand written where the meaning of Bukkit needs Rust, the value types `Location`, `Vector`, `BoundingBox`, `ItemStack`, `BlockData`, `Component`, the registry tables, the mirrors and the tick snapshot | No | AGPL-3.0-only |
| `golem-events` | The frame of every event, generated from the mapping, the functions of the event coordinates over the frames, the dispatcher, the chains, the wanted mask, the interest tables and subject sets, the copies for remote subscribers | No | AGPL-3.0-only |
| `golem-api` | The Aosabi app: hand written roots and types of Golem, generated object types, union enums, mutations and subscriptions over `golem-model` and `golem-events`, the schema parts of the ports mounted by path | No | AGPL-3.0-only, the mounted files keep the license of their port |
| `golem-client` | The plugin client, generated from the SDL: one trait per Bukkit interface, one method per coordinate, the casts, the value helpers that the mapping names `lib`, the runtime of documents, the tokens `Game` and `AsyncGame`, `PluginLock`, and under its feature `process` the GraphTP transport of a process plugin | No | AGPL-3.0-only |
| `golem-plugin` | The runtime: `Plugin`, `plugin.toml`, load order, the ledger, budgets, deadline, watchdog, degrade, quarantine, scheduler, services, the configuration library, conversations | No | AGPL-3.0-only |
| `golem-remote` | The API thread: Aosabi carriers for HTTP and GraphTP, tokens, limits, the hand over of operations to the server thread, event streams, supervised process plugins | No | AGPL-3.0-only |
| `golem-nbt` | Binary NBT and SNBT, for item meta, block states and persistent data | No | AGPL-3.0-only |
| `golem-permissions` | As today, plus attachments, defaults and the API nodes | No | MIT, the port of LuckPerms |
| `golem-store` | As today, plus the hashes of tokens and the audit log | No | AGPL-3.0-only |
| `golem-devtool` | New subcommands `contract gen`, `api inventory`, `api propose`, `api gen`, `api client`, `api ops`, `api sdl`, `api coverage`, `api slice`, `api smoke` and `api lib` | No | AGPL-3.0-only |

The crates depend on each other in this order and no crate names one above it: `golem-jvm`,
`golem-bridge`, `golem-game`, `golem-model`, `golem-events`, `golem-api`, `golem-client`,
`golem-plugin`, the plugin crates, `golem-remote`, `golem`. `golem-api` shows the state of the
runtime, such as the plugins and their budgets, through a table of providers that `golem-plugin`
fills at boot, so the schema never depends on the runtime.

Why the meaning sits below `golem-api`: the build of Aosabi compiles the glue of a plain crate into
a generated copy of the crate under `target/aosabi/`, a package of its own whose files are links to
the author's files (Rust guide of Aosabi, section 13, and `aosabi-build/src/source/copy.rs`). A
client that called the resolvers of the plain `golem-api` would link a second compiled copy of every
resolver and of every `static`, such as the snapshot. `golem-model` and `golem-events` are plain
crates that the glued copy and the client link once.

Unsafe code stays where CONTRIBUTING.md section 4 puts it, in `golem-jvm`, `golem-bridge` and
`golem-game`. `golem-game` hands the direct buffers out as byte slices, so the frames of
`golem-events` are safe code, and `golem-api` holds none, so the glue that Aosabi writes for it
keeps the rule as well.

## 4. The schema

### 4.1 How it is defined

`golem-api` is a plain Aosabi crate as the Rust guide of Aosabi describes it, with no macro, no
derive and no attribute of Aosabi. Four kinds of Rust make it up.

| Kind | Written by | Example |
| --- | --- | --- |
| Roots and the types of Golem itself | Hand | `Query::server`, `Query::entity(id)`, `Server.tps`, `Plugin.budget`, `World.safeLocation` |
| Object types of the concrete Bukkit types, each with every field of every interface it implements | `golem-devtool api gen` from `api/bukkit/` | `ZombieObject`, `Husk`, `Player`, `PlayerMoveEvent`, `StairsData` |
| Union enums, one per Bukkit interface over its concrete types, which Aosabi prints as interfaces | `api gen` | `enum LivingEntity { Player(Player), Husk(Husk), ... }` |
| Mutations and subscriptions, one per Bukkit setter, action and listenable event | `api gen` | `Mutation::entity_teleport`, `Subscription::player_move` |

The generated files are checked in, with a first line that names the command that wrote them, and CI
runs the generators again and fails on a difference, as it runs `reuse lint`. Aosabi's builder reads
them as the plain Rust they are. They are boilerplate that a command writes, in the sense of
principle 4 of Aosabi, and the meaning behind each field is one coordinate function.

```rust
// crates/golem-api/src/entity/zombie.rs, written by golem-devtool api gen
use aosabi::prelude::*;
use golem_model::coordinate as c;

/// A zombie that is no husk, drowned or zombie villager. Bukkit: org.bukkit.entity.Zombie.
pub struct ZombieObject {
    entity: golem_model::EntityRef,
}

impl ZombieObject {
    /// The UUID of the entity, the same across restarts and dimensions.
    pub fn id(&self) -> ID {
        ID::from(self.entity.uuid().to_string())
    }

    /// Where the feet of the entity are. Bukkit: `Entity.getLocation()`.
    pub fn location(&self) -> Result<Location> {
        Ok(c::entity::location(&server()?, self.entity)?.into())
    }

    /// Whether the zombie is turning into a drowned. Bukkit: `Zombie.isConverting()`.
    pub fn is_converting(&self) -> Result<bool> {
        Ok(c::zombie::is_converting(&server()?, self.entity)?)
    }
    // ... every field of Entity, Damageable, LivingEntity, Mob, Creature, Monster, Ageable, Zombie
}

/// Bukkit's org.bukkit.entity.Zombie: a zombie, a husk, a drowned or a zombie villager.
pub enum Zombie {
    ZombieObject(ZombieObject),
    Husk(Husk),
    Drowned(Drowned),
    ZombieVillager(ZombieVillager),
}
```

`server()` answers the server token of the thread, or the error `NOT_ON_SERVER_THREAD` on any other
thread, see [section 7](#7-threads). A resolver cannot take the token as a parameter, since Aosabi
would print it as an argument, so it reads it from the frame that the bridge opens for each native
call. The value types of the schema, such as `Location`, are declared in `golem-api` with `From`
from the types of `golem-model`, since Aosabi refuses a type of a path dependency, whose glue the
orphan rule of Rust forbids (`aosabi-build/src/source/glue.rs`).

The index registers the roots on one chain:

```rust
// crates/golem-api/src/lib.rs
pub mod parts;

pub fn app() -> impl IntoApp {
    App::new()
        .query(Query)
        .mutation(Mutation)
        .subscription(Subscription)
        .query(parts::essentials::EssentialsQuery)
        .mutation(parts::essentials::EssentialsMutation)
        .subscription(parts::essentials::EssentialsSubscription)
}
```

### 4.2 The schema parts of ports

A port that offers an API of its own, as EssentialsX offers its economy and its events, holds a
schema part in its own directory, `crates/<port>/schema/mod.rs`, under the license of its upstream,
since the events and the API of the upstream are its code, which only the port may translate
([Licensing](../Documentation/licensing.md) section 6). `golem-api` mounts it as a module with
`#[path]`, so its types are types of `golem-api`, as the builder requires, and the file stays below
the path of the port, as rule 9 of the license lint requires:

```rust
// crates/golem-api/src/parts/mod.rs
#[path = "../../../golem-essentials/schema/mod.rs"]
pub mod essentials;
```

The mount stands in `src/parts/mod.rs` and never in the crate root. The generated copy of Aosabi
copies the crate root into a real `src` directory and links every other entry, so a relative path in
a module under a linked directory resolves through the link to the port, while the same path in the
copied crate root resolves inside `target/aosabi/` and fails, as the spike of
[4.4](#44-the-spikes-behind-the-rules) showed.

A schema part declares types, roots and the provider trait its resolvers call, and names no crate
above `golem-events`. The port implements the provider trait and registers it in `enable`, as
`ServicesManager.register` does in Bukkit, so the schema part reaches the code of the port with no
cycle between the crates. A shared service, such as the economy of Vault, is a schema part of Golem
itself under `crates/golem-api/src/parts/economy.rs`, written fresh, whose coordinates stand in
`api/golem-own.toml`, so every economy provider answers one SDL. The events of a port are declared
in its schema part as event types, a subscription and a `Mutation.call<Event>`, and run through the
dispatcher of section 10 with a frame in Rust memory and no JNI.

Every committed file generated from a schema part stays with the port as well, since it derives
from the code of the port. `crates/<port>/schema/part.toml` lists the types and the root fields
that the part declares. `api sdl` prints them into `crates/<port>/schema/schema.graphql` and the
rest of the SDL into `api/schema.graphql`, and `api client` writes the methods and the canonical
operations of the part into `crates/<port>/src/client.rs`, each under the license of the port, so
`golem-client` holds the coordinates of the mapping and of `api/golem-own.toml` alone. A plugin that
calls the API of a port depends on the crate of the port, as a Bukkit plugin compiles against the
jar of EssentialsX. The copy that the build of Aosabi writes under `target/aosabi/` is build output
and never a file of the tree: it links the file of the port into the compiled `golem-api` as the
program `golem` links the crate of the port, the combination that section 13 of the GPL 3 and the
AGPL 3 allow, and every file keeps its own license.

### 4.3 The limits of the plain Rust path and the rules that meet them

| Limit of Aosabi at `6e507dc` | Where it is | The rule of Golem |
| --- | --- | --- |
| The builder reads inherent `impl` blocks only and no trait impl, so Rust states no inheritance | `aosabi-build/src/source/describe.rs` | Every object type carries every field of its interfaces, each delegating to the one coordinate function |
| An interface is made only from a union enum whose members share fields, and it implements no interface | `promote_unions_to_interfaces` and `check_if_interface_is_possible_for_union` in `aosabi-infer/src/schema_parser.rs` | One union enum per Bukkit interface over its concrete types. Every object type implements each interface of its ancestry by name, so a fragment on `LivingEntity` matches a field typed `Entity` |
| A union enum with one member prints as that member, and one that no field references leaves the SDL | The spikes of the panel on 2026-11-06 | Every union enum has at least two members: `<Name>Object` is the member for the instances whose most specific Bukkit type is the interface itself, modded entities of that base included. Every union enum is referenced: the entity unions by `World.entitiesOf`, which maps `getEntitiesByClass`, and every other union that no Bukkit signature references by the cast `Query.as<Name>(id:)` |
| An input type is named after the field and the argument that take it | Rust guide of Aosabi, section 4 | No argument takes a struct until item A3 gives inputs a shared name, so no document declares a variable of an input type that A3 would rename. Places are flat arguments (`world`, `x`, `y`, `z`, `yaw`, `pitch`), prefixed with the name of the parameter where a member takes two, such as `fromX` and `toX`, an item stack is the SNBT text of the stack as vanilla saves it, a block data is its text such as `minecraft:oak_stairs[facing=north]` |
| No custom scalar, only the markers `ID`, `Int`, `Float`, `Number`, `Date`, `File`, `JSON` and `Any` | `aosabi-build/src/source/describe.rs` | Ids are `ID`, keys of registries are `String` of the form `namespace:path`, NBT is SNBT text, a text component is `JSON`, raw bytes are base64 text, numbers of 64 bits are `Number` |
| A default of an argument does not reach the schema | Rust guide of Aosabi, section 3 | An optional argument is an `Option`, its description names the default and the body takes it |
| The plain path declares no directive | `aosabi-build/src/source` | No custom directive. Priority, `ignoreCancelled` and filters are arguments of the subscription field |
| An enum whose variants hold a value is refused as an input | `payload_of` in `aosabi-build/src/source/describe.rs` | No input that is one of several shapes and no `@oneOf`. An argument that Bukkit takes as one of several types is one optional argument per shape, and the body refuses all but one |
| No deprecation is printed | Quirk `Q07_NO_DEPRECATED` of `aosabi-infer` | The schema only grows within a major version of Golem, and a coordinate that a later one replaces keeps working with its description naming the replacement. Compatibility is judged by validating documents, see [section 11](#11-packaging-and-lifecycle) |
| A root is a unit struct of the crate, and a type of a path dependency is refused | `aosabi-build/src/source/entry.rs` and `glue.rs` | Every schema type lives in `golem-api`, and the schema parts of ports are mounted with `#[path]` |

### 4.4 The spikes behind the rules

The spike `api-design/work-final-opus/spike` of the scratchpad ran the source reader of Aosabi at
`6e507dc`, `aosabi-devtool source --requests`, over a crate shaped like `golem-api` on 2026-11-06 in
the cloud container of the panel. It showed four things.

- A module mounted with `#[path]` from the directory of another crate is read as a module of the
  crate, and its unit struct `EssentialsQuery` merged its field `essentialsBalance` into `Query`.
- The union enum `HumanEntity { Player, HumanEntityObject }`, referenced by `Query.asHumanEntity`,
  printed `interface HumanEntity`, and `Player` printed `implements HumanEntity & LivingEntity`.
- A struct used at one argument site printed as `input CallEntityDamageEventInput`, named after the
  field and the argument, which is why no argument takes a struct before A3.
- A resolver that calls a function of a path dependency and converts its value into a type of the
  crate with `From` builds, and an `Option<f32>` argument prints as a nullable `Float`.

That spike ran the source reader alone, with the mount in the crate root. The spike
`api-design/work-final/pathspike` rebuilt the layout that `aosabi-build/src/source/copy.rs` writes,
a real `src` directory with the crate root copied and every other entry a link, and compiled it with
rustc 1.95.0 on 2026-11-06 in the same container. A mount in `src/parts/mod.rs` resolved through the
link to the port. The same mount in the crate root failed with "couldn't read
`target/aosabi/api/src/../../port/schema/mod.rs`". Hence the mount rule of
[4.2](#42-the-schema-parts-of-ports).

### 4.5 What Golem asks of Aosabi

Each request is small, useful beyond Golem, and has a fallback that the design uses until it lands.

| Item | Request | Needed by | Fallback |
| --- | --- | --- | --- |
| A1 | An embedding entry: the glued copy of a plain crate as a library that a host program links, and an entry that answers the `Service` and the routes without starting `serve_app` | The first area, on the critical path | `golem` links the generated copy that `cargo aosabi build` writes under `target/aosabi/` as a path package, and if that fails `golem-remote` builds `Service::builder(SDL).resolvers(map)` over `api/schema.graphql` with a resolver map that `api gen` writes over the same functions |
| A2 | The document table of M12, `aosabi-kernel/src/document_table.rs`, which maps the `u32` id of `Operation::Persisted` to its generated resolver plan, the generator of those plans, and an entry on `Service` that runs an id with its variables on the thread of the caller, with the context and the ambient hooks that `execute` builds | Every call of a plugin, which pays a parse per call until it lands | Every operation, the canonical operations of the client included, runs through `Service::execute` with `validate: false`, which parses each time, about 7 to 8 µs for one field. `aosabi-core/src/service.rs` answers `Operation::Persisted` with "Persisted documents land at M12." today, so Golem writes no plan of its own in its place |
| A3 | A shared name for an input struct used at several sites, such as `LocationInput` and `ItemStackInput` | Inputs such as a place or an item stack | Flat arguments and text forms, as in 4.3 |
| A4 | Custom scalars on the plain path, such as `UUID`, `Key`, `SNBT` and `Component` | Clearer types | `ID`, `String` and `JSON`, with the form named in the description |
| A5 | A trait with supertraits read as an interface that implements its supertraits, and a struct that implements the trait as a type that implements the interface. The smallest form is a doc line `@implements Entity & Damageable` on a struct, read the way `@param` lines are, into the `implements` list that `aosabi-ir` already carries | Entities, block data, block states, meta, events | The union enum rules of 4.3 |
| A6 | GraphTP on Unix sockets in the facade, on a path and on a socket pair, `SOCK_SEQPACKET` on Linux and the stream form with prefixed packets on macOS, and a kqueue reactor for macOS. `homa::unix` builds on Linux alone today, `homa/src/unix.rs` stops any other build with `compile_error!`, so the item includes a build of its stream form for macOS | The clients outside the process, which reach the API over GraphTP on a Unix socket as the owner ruled, and process plugins | On Linux `golem-remote` drives `graphql-over-graphtp` over `homa::unix` itself, `SeqListener` on the path, as the card `test-graphtp` of `aosabi-devtool` does. On macOS it drives the same listener through `homa::stream::Framed`, the prefixed AoFrame of one Homa packet on a byte stream, which is `no_std` and builds on every target, over a newtype of the non blocking `std::os::unix::net::UnixStream` that implements `homa::stream::ByteStream` with the plain read and write of the standard library. That adapter is safe code of a few dozen lines, since a Rust program starts with `SIGPIPE` ignored, which gate 0 checks with the JVM running |
| A7 | The Rust GraphTP client of `roadmap/graphtp.md` item 4 | Process plugins | `golem-client` drives the client of `graphtp-homa`, which does no input or output of its own, over the socket pair of `homa::unix` on Linux and over `Framed` on `UnixStream::pair` of the standard library on macOS, with the adapter of A6 |
| A8 | The peer uid of a Unix socket on macOS, which `aosabi-serve/src/runtime/peer.rs` answers on the Linux cards alone | The principal `local` on the Mac mini | On macOS the modes of the socket and its directory are the boundary, and a token is required |

The engine plugin of `golem-remote` hands each remote operation to the server thread through the
`on_execute` seam of `aosabi-hooks`, whose `set_execute_fn` lets a plugin replace the execution with
one that answers later. The first area checks that the serving runtime of Aosabi accepts that answer
when the server thread wakes the API thread, see
[7.8](#78-remote-operations-run-whole-on-the-server-thread).

### 4.6 The size of the schema

`inventory.json` holds 171 interfaces in `org.bukkit.entity` and its subpackages, 168 in the tree of
`Entity` with `Entity` itself and 117 of them leaves, counted by `work-final-opus/scripts/counts.py`
on 2026-11-06. Flattened onto the leaves, the entity types print about 13,000 object fields, by the
count of the panel. Nobody has run the builder and the boot of Aosabi on a schema of that size, so
the gates of [section 20](#20-work-breakdown) do that before any area starts. If the builder fails
at that size, the fallback of A1 with `Service::builder(sdl)` and a generated resolver map keeps the
engine and the schema and gives up the plain Rust source of the generated part, and plugins are
untouched, since their calls go through the coordinate functions.

The SDL that the builder prints is committed as `api/schema.graphql`, and the notebook
`11_api_schema` fails when the printed schema differs from it, so every change of the schema is
visible in review.

## 5. How a plugin calls the schema

### 5.1 The client

`golem-devtool api client` writes `golem-client` from `api/schema.graphql` and the type lines of the
mapping, which carry the hierarchy of Bukkit that the SDL of the union enums lacks. It writes one
trait per Bukkit interface and one method per coordinate, and for every coordinate the canonical
operation, which it validates against the SDL and keeps in `api/coordinates.graphql`, with a struct
for its variables and a decoder of its answer. The body of a method runs its canonical operation
through the executor of Aosabi:

```rust
// crates/golem-client/src/entity/living_entity.rs, written by golem-devtool api client
pub trait LivingEntity: Entity + Damageable + Attributable + ProjectileSource {
    /// LivingEntity.eyeHeight, the canonical operation
    /// `query($id: ID!) { entity(id: $id) { ... on LivingEntity { eyeHeight } } }`.
    fn eye_height(&self, g: &Game<'_>) -> Result<f64> {
        g.run(&op::LIVING_ENTITY_EYE_HEIGHT, op::EntityId { id: self.id() })
    }

    /// Mutation.livingEntitySetAi, the canonical operation `mutation($e: ID!, $ai: Boolean!)
    /// { livingEntitySetAi(livingEntity: $e, ai: $ai) { __typename } }`.
    fn set_ai(&self, g: &Game<'_>, ai: bool) -> Result<()> {
        g.run(&op::MUTATION_LIVING_ENTITY_SET_AI, op::LivingEntitySetAi { e: self.id(), ai })
    }
    // ...
}
```

`op::LIVING_ENTITY_EYE_HEIGHT` holds the text, its SHA-256, the encoder of the variables and the
decoder of the answer, and `g.run` hands the prepared operation to the `Service` of `golem-api`,
whose resolver of `eyeHeight` calls `golem_model::coordinate::living_entity::eye_height`. No line of
the client calls a coordinate function itself.

The concrete types are `Copy` names, such as `PlayerRef` and `ZombieRef`. A cast of Java is the type
condition of GraphQL: `if (entity instanceof Ageable a)` becomes
`if let Some(a) = entity.as_ageable(g)`, which reads the concrete type of the entity, cached per
entity type, and checks it against the possible types of `Ageable`. A method exists in the client
only when its coordinate is in the SDL, and the build of `golem-client` fails otherwise, so the
client cannot reach anything the schema lacks and holds nothing beside it.

A subscription is built from its field, so the builder spells the coordinate and its arguments,
filters included:

```rust
enable.subscribe(
    Subscription::player_move()
        .priority(EventPriority::High)
        .ignore_cancelled(true)
        .changed_block(true),
    |g, event| { /* ... */ Ok(()) },
);
```

The fields of an event are reads of its frame, which is Rust memory under a borrow that ends with
the dispatch, so `event.from()` and `event.player()` take no token and cannot fail. The frame is the
value that the subscription delivers to its subscriber, as a remote subscriber receives the payload
of its stream, so a read of it is no call of the API and runs no operation. A setter of an event is
a call: it runs its mutation, such as `Mutation.playerMoveEventSetTo`, through the executor like
every other method, and answers a `Result`, since a `MONITOR` subscriber gets `EVENT_READ_ONLY`.

### 5.2 Documents

A plugin may keep documents in `ops/*.graphql`. `aosabi-query` writes no Rust client, so
`golem-devtool api ops` parses the documents with `aosabi-lang`, validates them with
`aosabi-validate` against `api/schema.graphql` at build time and writes `src/ops.rs` with the text,
the SHA-256 of the text, a struct for the variables and structs for the answer. At `enable` the
runtime prepares every document of the plugin once, checks its coordinates against the grants of
the plugin once and keeps it with its `u32` id. A call runs it on the server thread through the
`Service` of `golem-api`, through `Operation::Persisted` of the document table of A2 once it lands
and through `Service::execute` with `validate: false` until then, and the generated decoder reads
the answer into the structs. The same `ops.rs` builds against `golem-client` with the feature
`process`, where a call sends the text over GraphTP, so a plugin that only runs documents and
observes events moves into a process of its own by a change of its manifest.

A resolver that the generator writes for a record of the mapping, such as the commonly read fields
of a living entity, learns the fields a document selects through `ObjectLike::select` of
`aosabi-execute`, under its feature `host-reads`, and reads them with one JNI call. So a document
that reads ten fields of one player costs one crossing.

### 5.3 Operations and the executor

`g.run` counts the call of the operation in the budget of the plugin, records the coordinate for the
proof of [section 14](#14-coverage), checks the grant of the plugin once per operation and runs the
operation through the `Service` of `golem-api` on the thread of the caller. Until item A2 lands it
calls `Service::execute` with the text and `validate: false`, since the build validated the text
against the SDL, which parses on every call. Once A2 lands it runs `Operation::Persisted` with the
`u32` id under which the document table of Aosabi holds the plan that Aosabi generates, and nothing
changes for a plugin. The executor resolves the root field, such as `entity(id:)`, and then the one
field, whose resolver calls the coordinate function, so plugins, documents and tools reach the
game through the same resolvers and the same executor. A null at the root, such as an entity that
is gone, is the error `ENTITY_GONE` in the client. The notebook `14_api_smoke` runs the canonical
operation of every coordinate through the client on a booted server with generated arguments, and
checks the type of every answer and that the coordinate resolved. Golem writes no plan of an
operation and no compiler of documents: both are item A2 of Aosabi.

### 5.4 Which form a plugin uses

| Use | Form | Why |
| --- | --- | --- |
| Event handlers, commands, tab completion, sync tasks, everything hot | The client | One typed operation per Bukkit call, as the conversion maps it, and the fields of the frame as Rust memory with no call |
| Reads of many fields or objects at once, one consistent read inside one slot, code shared with tools, a plugin that may leave the process | Documents | One pass of the executor for all fields, one crossing per object through the records, one permission check, the same text a tool runs |
| Tools, operators and process plugins | Documents over HTTP or GraphTP | The only way in from outside the process |

### 5.5 The tokens

`Game<'t>` is what a handler receives on the server thread. It is neither `Send` nor `Sync`, it
lives only for the call, and it carries the plugin, its budget clock and the server token, so code
that compiles cannot reach the game from the wrong thread. `AsyncGame` is the `Send` handle of the
other threads, with the coordinates of the thread classes `any` and `snapshot` and the hops of
[7.5](#75-hops).

## 6. The contract with the bridge mod

### 6.1 One mapping entry writes every artifact

The mapping of a member is the one place a person writes how it reaches the game. A member whose
meaning is one read or one write of the game carries its Java as an expression over `it`, the object
the receiver names, and the names of its arguments:

```toml
# api/bukkit/org.bukkit.entity/Player.toml
[members."public abstract boolean getAllowFlight()"]
status = "schema"
target = "Player.allowFlight"
read = "it.getAbilities().mayfly"
doc = "Whether the player may fly, as in creative mode."

[members."public abstract void setAllowFlight(boolean)"]
status = "schema"
target = "Mutation.playerSetAllowFlight(player:, flight:)"
receiver = "player"
params = ["flight"]
write = '''
it.getAbilities().mayfly = flight;
if (!flight) it.getAbilities().flying = false;
it.onUpdateAbilities();'''
doc = "Allows or forbids flight, and ends a flight in progress when it forbids it."
```

| Artifact | Where it lands | What `read = "it.getAbilities().mayfly"` becomes |
| --- | --- | --- |
| The contract call | `bridge/contract/<area>.toml`, generated and checked in | `playerAllowFlight`, thread `server`, the arguments of an entity, an answer of one boolean |
| The Java interface method | `bridge/host/.../contract/Players.java` | `boolean playerAllowFlight(int id, int level, long most, long least)` |
| The Java implementation | `bridge/mod/.../gen/PlayersGen.java` | The lookup of the receiver and the expression, compiled by javac with every lint as an error, so a wrong expression fails the build of the mod at its line |
| The Rust stub | `crates/golem-game/src/contract/players.rs` | `player_allow_flight(&Server, EntityRef) -> Result<Option<bool>, JniFault>` with its method id cached at `connect` |
| The coordinate function | `crates/golem-model/src/coordinate/player.rs` | `allow_flight(&Server, EntityRef) -> Result<bool>`, with `ENTITY_GONE` on a miss |
| The resolver | `crates/golem-api/src/entity/*.rs` | The field `allowFlight` of every object type that implements `Player` |
| The client method | `crates/golem-client/src/entity/player.rs` | `fn allow_flight(&self, g: &Game<'_>) -> Result<bool>` of the trait `Player` |

A member whose Java needs more than an expression names `read = "hand"` or `write = "hand"`, a
method of `bridge/mod/.../game/<Area>Hand.java` that a person writes with the semantics of
CraftBukkit, read and not copied, and the generator checks that it exists with the signature the
entry implies. A member whose meaning needs logic in Rust, such as `Damageable.setHealth`, which
refuses a value outside zero and the maximum and kills at zero, names its function with `rust =`. A
member of an event names the field of its frame with `frame =` and needs no call at all. javac
compiles the generated Java against the official names of Mojang, and ForgeGradle reobfuscates it
to SRG with the rest of the mod. The coordinates of Golem's own, such as `World.safeLocation`, carry
the same keys in `api/golem-own.toml`, so the mapping and that file are the only hand edited sources
of the contract.

### 6.2 What `contract gen` writes

`api gen` writes `bridge/contract/<area>.toml` from the mapping, with every call, its thread, its
primitive arguments and the layout of its answer in the out buffer, every record and every event
with the layout of its frame. The files are checked in so that a reviewer sees the surface of the
JNI, and CI writes them again and fails on a difference. `contract gen` reads them and writes the
rest.

| Output | Where | What it is |
| --- | --- | --- |
| Java interfaces | `bridge/host/src/main/java/com/skytekx/golem/host/contract/` | `Entities`, `Players`, `Worlds`, `Blocks`, `Items`, `Inventories`, `Scoreboards`, ... with one method per call, such as `boolean entityLocation(int id, int level, long most, long least, ByteBuffer out)` |
| Java implementations of the expressions | `bridge/mod/src/main/java/com/skytekx/golem/bridge/gen/` | One final class per interface for the calls that the mapping generates, beside the hand written classes for the rest |
| Java frame writers | `bridge/mod/src/main/java/com/skytekx/golem/bridge/frames/` | One final class per event that writes and reads its frame at fixed offsets |
| Rust stubs | `crates/golem-game/src/contract/` | One function per call with its method id cached at `connect` |
| Rust frames | `crates/golem-events/src/frames/` | One struct per event with its reader and the writer of its changed fields, over the safe slices of `golem-game` |
| The contract hash | Both sides | The SHA-256 of the contract, which `Host.connect` compares |

The mod hands its implementations to `Host.connect`, as `Game.java` does today. The host jar keeps
no type of Forge or Minecraft, so it still sits in the boot layer.

### 6.3 Records and masked reads

A getter costs one JNI call, and a document that reads seven fields of one entity would cost seven.
Two forms cut that.

- A record per object kind, declared in the mapping file of the type with `record = "<name>"` and
  on each of its fields with `record = true`: `entityRecord`, `livingRecord`, `playerRecord`,
  `worldRecord`, `blockRecord`. Each is one call that writes the fields plugins read most into the
  out buffer, so a field in the record costs a buffer read once the record is read in that call.
- `ObjectLike::select` of `aosabi-execute`, behind its feature `host-reads`, hands a resolver the
  members a selection asks for before it resolves them, so a generated object type reads its
  selected fields with one masked call, `Entities.read(id, level, mask, out)`.

### 6.4 The checks

| Check | Where | Fails when |
| --- | --- | --- |
| The generated files are current | CI, the generators with a diff | Someone edits a generated file or a mapping entry without running the generators |
| The host jar matches the contract | Notebook `10_api_contract`, as `05_bridge_build` checks the natives today | A method of a contract interface has another descriptor than the contract derives |
| The mod implements every call | The same notebook, with javap of the mod jar | A call is missing or a frame writer is stale |
| Every expression compiles | The build of the bridge | An expression names a member that does not exist or is not accessible |
| Rust and the mod agree at boot | `Host.connect` compares the hash and resolves every method id with `GetMethodID` | A mod jar of another build. The mod then logs one line that names the first missing method and switches itself off, as on a stock Forge |
| A JNI call costs what the model says | `10_api_contract` times `entityRecord` and a getter with the out buffer on the booted test server | The cost leaves the band of [section 12](#12-performance) |
| Every coordinate answers on a real server | Notebook `14_api_smoke`, through the client as [5.3](#53-operations-and-the-executor) runs it | A coordinate throws, answers the wrong type or does not resolve |
| A call of the client costs what the model says | `10_api_contract` times a canonical operation through the `Service` on the booted test server | The cost leaves the band of [section 12](#12-performance) |

## 7. Threads

### 7.1 The threads and what they may do

| Thread | Who | May call |
| --- | --- | --- |
| The server thread of Minecraft | Ticks, Forge events, Mixin hooks, commands, tab completion, sync tasks, the API slot, hops | Every coordinate, at once |
| `golem-chat-<n>`, a pool of Golem | The chain of `AsyncPlayerChatEvent`, see [7.6](#76-the-chat-and-login-chains) | The coordinates of the classes `any` and `snapshot`, the frame of the chat event, hops |
| `golem-net-<n>`, a pool of Golem | The chains of `AsyncPlayerPreLoginEvent` and `ServerListPingEvent`, which hooks on the threads of Netty and of the login hand over, see [7.6](#76-the-chat-and-login-chains) | The same as `golem-chat` |
| The threads of Netty | Packets, the status answer and the login of vanilla | Nothing of a plugin: their hooks only hand frames to `golem-chat` and `golem-net` |
| `golem-async-<n>`, a pool of Golem | The async tasks of plugins, `runTaskAsynchronously` | The same as `golem-chat` |
| `golem-api` | HTTP and GraphTP of remote clients, authentication, limits, remote event streams | The classes `any` and `snapshot`. It hands every other operation to the server thread whole |
| `golem-watchdog` | The watchdog of [11.4](#114-containment) | Nothing of the game |
| The world generation threads of Minecraft | Chunk generators of plugins | The chunk buffer handed to the generator and the coordinates of the class `worldgen` |

Every coordinate has one thread class in its mapping entry. `server` is the default. `any` is what
is safe off the server thread: permissions, metadata, configuration, the registry tables, the
mirrors, every value type, and `CommandSender.sendMessage`, since the connection of vanilla queues
its packets from any thread. `snapshot` is a live read on the server thread and the value of the
[tick snapshot](#77-the-tick-snapshot) on any other thread, and the coverage tool holds every
`snapshot` member to a field of the snapshot record. The snapshot holds a record per online player
and per world alone, so off the server thread a `snapshot` coordinate answers from the record of its
receiver when there is one, and for any other receiver, such as a zombie for
`LivingEntity.remainingAir`, it answers `NOT_ON_SERVER_THREAD`, as a coordinate of the class
`server` does. CraftBukkit reads such getters off the server thread with no check, as
`CraftEntity.getLocation()` does, so Bukkit plugins read them from async tasks, and a value at most
one tick old is the closest equivalent that is also safe. `worldgen` is the chunk buffer of a
generator.

### 7.2 A coordinate on the server thread

```text
Minecraft server thread
  ServerGamePacketListenerImpl.handleMovePlayer (Mixin of the bridge mod)
    if Host.WANTED[PLAYER_MOVE] and the interest table passes       one array load, one test
      PlayerMoveFrame.write(frames, depth, player, from, to)       fixed offsets in the frame
      Host.event(PLAYER_MOVE, depth)                               one JNI call into golem-bridge
        golem-events: open the frame, run the chain by priority
          the closure of a plugin, with a Game token of this thread
            event.from(), event.player()                            reads of the frame, no call
            player.allow_flight(g)                                  a method of golem-client
              Service of golem-api: the executor of Aosabi runs the canonical operation
                resolver Player.allowFlight -> coordinate function -> stub of golem-game
                  CallBooleanMethodA(players, id, entity, level, ...)  no wait, no queue
            event.set_to(safe)                                      Mutation.playerMoveEventSetTo
              Service of golem-api -> writes the frame, marks to
        answer: the changed mask and the cancel flag
      PlayerMoveFrame.apply(frames, depth)                         Java moves the player
```

The JNIEnv of the thread is valid for the whole native call, and `golem-game` keeps it in a thread
local for that call alone. The stubs pass primitives and the buffers of the thread, so a call
creates no local reference and no string unless its answer is text.

### 7.3 Frames and buffers

Every thread that calls a native holds two direct buffers, which the mod allocates once per thread
and hands to `Host.bindBuffers`, and whose addresses `golem-game` reads once.

- The frame stack, 64 KiB. A frame of depth `d` starts where the frame of depth `d - 1` ends, so an
  event fired inside a handler gets a frame of its own and the outer frame stays as it was. The
  mapping gives every frame a fixed size of at most 1 KiB, and lists of variable length, such as the
  recipients of a chat message, the blocks of an explosion or the drops of a death, are read and
  written through calls of the contract on demand.
- The out buffers, 16 slots of 4 KiB, one per depth of the dispatch, which the mod hands over as
  16 direct slices of one buffer. A stub hands Java the slot of the depth it runs at and reads its
  answer into Rust before it returns. Every native that enters Rust from Java, `Host.event`,
  `Host.call` and the callback of a closure during a call, moves the thread to the next slot until
  it returns, and one that would pass the sixteenth answers `EVENT_DEPTH` as a deep event does. So a
  mutation whose Java re-enters Rust before it answers, as a teleport does through the inner
  `PlayerTeleportEvent`, `worldSpawnEntity` through the closure of [9.2](#92-entities) or
  `inventoryAddItem` through an inventory event, keeps its slot, and the getters that run inside
  write the slots above it, never a frame and never the slot of the outer call. The generated Java
  also writes its answer after its last call that can re-enter Rust, and the review of a method
  written by hand checks the same.

The notebook `13_api_events` holds the stack and the slots with nested events, a spawn closure and
getters inside handlers on a booted server.

### 7.4 Re-entrancy

Bukkit events nest: `player.teleport()` inside a move handler fires `PlayerTeleportEvent` before it
returns. Golem does the same. A mutation on the server thread calls Java at once, Java fires the
inner event, `Host.event` enters the dispatcher again on the same thread with the next frame of the
stack, and the outer handler goes on when the inner chain ends. The depth is capped at 16, and a
deeper event answers `EVENT_DEPTH`, passes as if nobody heard it and is logged once per event kind.
Every call of the client and every document run from inside a handler nests an agent of Aosabi the
same way, which the gates prove first. The registrations of the dispatcher are an immutable table
that an enable or a disable replaces as a whole, so dispatch reads it with no lock, and the global
lock of the handler of the first milestone goes.

### 7.5 Hops

A thread that is not the server thread reaches it through a hop, a closure that the server thread
runs.

| Call | From another thread | Bukkit |
| --- | --- | --- |
| `g.on_server(|g| ...)` | A future that the server thread completes. `.wait()` blocks for at most 5 seconds and answers `HOP_TIMEOUT` after that, and `PLUGIN_DISABLED` at once when the plugin disables, see [11.3](#113-the-ledger) | `callSyncMethod(...)` and its `get()` |
| `g.later(|g| ...)` | Queued, runs on the server thread within the tick, answers nothing | `runTask` from an async thread |
| A mutation of the class `server` whose answer is unit | Queued in the same bounded queue and answered at once. A mirror that the mutation changes is updated when the server thread applies it, not before | A sync task that sets something |
| A query of the class `server` | `NOT_ON_SERVER_THREAD` | The `AsyncCatcher` of CraftBukkit, which throws for the operations it guards |

The queue is bounded per plugin at 4,096 entries, and a full queue answers `QUEUE_FULL` and logs
once a minute. A hop does not wait for the end of the tick. The mod hands `Host` a waker at
`connect`, a `Runnable` that calls `server.execute(Host::runHops)`. A Rust thread that queues the
first hop of an empty queue calls the waker once through the JNI, attached as a daemon thread, and
the server thread runs `Host.runHops()` from its task queue, which `MinecraftServer` drains while it
waits for the next tick. So a hop waits at most for the rest of the current tick. A thread that
waits for a hop spins for at most 20 µs before it parks, since a parked wake measured a p50 of 21 to
26 µs and a spinning one 3 to 5 µs on the 4 core Xeon of the dev container on 2026-11-06
(`work-judge-runtime/oopo-rtt.txt`), and a longer spin would cost a core of the Mac mini.

### 7.6 The chat and login chains

Forge 47.4.18 fires `ServerChatEvent` inside the chat decorator of
`ForgeHooks.getServerChatSubmittedDecorator`, which answers
`CompletableFuture.supplyAsync(..., Util.backgroundExecutor())`. That pool is `Worker-Main`, as
`05_bridge_build` recorded, and it is also the executor that `MinecraftServer.createLevels` hands to
every `ServerLevel`, whose `ChunkMap` runs its mailbox `worldgen` on it (javap of the mapped Forge
jar, 2026-11-06). A plugin that waited there for the server thread would hold a thread of world
generation while the server thread may itself wait for a chunk.

So Golem keeps the rule of Bukkit, that the chat handlers run off the server thread and may wait for
it, on a thread of its own. A Mixin at the return of `MinecraftServer.getChatDecorator()` composes
the chain of `asyncPlayerChat` onto the future of Forge: when the decoration of Forge and the mods
is done, the hook writes the frame of the event, the player, the message, the format and the
recipients, and hands it to `golem-chat-<n>`, whose dispatcher runs the chain and completes the
future with the changed message, or with null when the event ends cancelled, which is the convention
of Forge for a cancelled message. No thread waits for that future. The chat chain of the connection
orders the messages of one player, as vanilla does. A message that a mod cancelled reaches the chain
cancelled, so a subscriber without `ignoreCancelled` sees it, and a subscriber that clears the
cancel sends the message of the frame. The recipients apply at the broadcast through a Mixin, since
Forge has no recipient list. Handlers get an `AsyncGame` and may wait for the server thread through
`g.on_server`, as an async task may.

Bukkit fires `AsyncPlayerPreLoginEvent` off the server thread and lets its handlers wait, while
vanilla 1.20.1 gives a login a thread of its own only in online mode: `handleKey` starts
`ServerLoginPacketListenerImpl$1`, which extends `Thread`, while in offline mode, which the test
server of the notebooks runs (`tests/fetch.py` writes `online-mode=false`), `handleHello` sets the
state `NEGOTIATING` on the thread of Netty that read the packet and the server thread finishes the
login in `tick` (javap of the mapped Forge jar, 2026-11-06). CraftBukkit, and so Arclight on the
live server, starts a thread of its own for the offline case. Golem runs the chain on its pool
`golem-net-<n>` in both modes. A Mixin at the point where the login holds its profile, at the end of
the authentication thread in online mode and in place of the state `NEGOTIATING` in `handleHello` in
offline mode, sets the state `AUTHENTICATING`, which `tick` leaves alone but for its timeout of 600
ticks, writes the name, the UUID and the address into the frame and hands it to `golem-net-<n>`.
When the chain ends, that thread sets `NEGOTIATING`, as the authentication thread of vanilla does,
or disconnects the connection with the kick message of the verdict. No thread of Netty and no server
thread waits, a handler may wait for the server thread as an async task may, and a chain that hangs
ends with the login timeout of vanilla. A login that reaches `golem-net` before the plugins of
`postworld` are enabled waits there for `serverStarting`, so no subscriber misses it.

`ServerListPingEvent` is asynchronous as well, spigot-api constructs it with `super(true)`, and
vanilla answers a status request on the thread of Netty in
`ServerStatusPacketListenerImpl.handleStatusRequest`, as the legacy ping in `LegacyQueryHandler`. A
Mixin at both hands the frame, the address, the MOTD, the counts and the icon, to `golem-net-<n>`
and sends the answer from the frame when the chain ends, through `Connection.send` or the context of
the channel, which any thread may write, so no thread of Netty runs code of a plugin. The pings have
a bounded queue of 256 apart from the logins, and a ping that finds it full gets the answer of
vanilla without the chain, so a flood of pings never holds up a login.

### 7.7 The tick snapshot

At the end of every tick the `tick` native reads one record per online player and per world in one
JNI call and publishes it as an immutable snapshot that any thread reads with no lock: the UUID, the
names, the world and the place, the game mode, the health, the food, the remaining air, the level
and the experience, the ping, the operator level and the time since the last action of every player,
and the time, the weather and the counts of every world, beside the TPS, the MSPT and the budgets of
the server. A coordinate of the class `snapshot` answers from it off the server thread for a
receiver that has a record, an online player or a loaded world, and answers `NOT_ON_SERVER_THREAD`
for any other receiver. Monitors and dashboards read `Query.server` and the player list from the
snapshot and cost the server thread nothing. The snapshot never grows into a cache of blocks or
inventories, since a stale block is a bug a plugin cannot see.

### 7.8 Remote operations run whole on the server thread

A document of a remote client could touch the game in every field, and one hop per field would cost
a tick per level of nesting. So the API thread never resolves a field of the class `server`. It
reads the request, authenticates it, applies the limits, and hands the whole operation to the server
thread through the `on_execute` seam: the prepared document, the variables, the principal and a
reply slot. An operation whose fields are all of the classes `any` and `snapshot` runs on the API
thread at once.

The server thread runs the waiting operations at two points, each with a budget.

- The API slot, `Host.apiSlot(budgetNanos)`, which the mod calls after `tick`, with 2 ms per tick by
  default.
- The serve point, `Host.runHops`, which the waker of [7.5](#75-hops) posts when the first operation
  arrives in an empty queue, with 1 ms per run, so an operation that arrives early in a tick does
  not wait for its end.

Both stop when their budget is spent and the rest waits for the next point. The budget is checked
between operations, so it cannot stop an operation that has started, and the bounds of
[13.4](#134-limits) keep every operation small instead: a cost estimate and an answer size before it
runs, no chunk load, and a deadline of its own while it runs. The total of both points then stays
within 2 ms per tick plus at most the one field that runs when the deadline of an operation passes,
and each remote principal has its own bucket of server time. The server thread writes the answer
bytes into the reply slot and wakes the API thread through a wake source registered with the reactor
of `aosabi-serve`, an `eventfd` on Linux and a pipe on macOS, since `Reactor::add` takes any
`BorrowedFd`. On Linux the epoll reactor of Aosabi sees the wake at once. On macOS the poll reactor
of Aosabi learns that a source is ready only by trying it, a hot source on every turn and a cold one
at the cold sweep, which its constant `COLD_CEIL_US` caps at 50 ms, so a remote answer there can
wait up to that sweep until item A6 brings a kqueue reactor. Plugins in the process never wait for
it.

### 7.9 World generation

A chunk generator of a plugin runs on the threads where Minecraft generates. The bridge registers a
`ChunkGenerator` type `golem:plugin` whose methods hand a buffer of block state ids for one chunk
section to the Rust generator and read it back. The generator never hops to the server thread and
never blocks on it, and generators of one plugin run in parallel, so a plugin that keeps shared
state guards it itself.

### 7.10 The locks of a plugin

With no global lock, a plugin guards its own state, as a Bukkit plugin does for its async handlers.
`golem-client` offers `PluginLock<T>`, a mutex whose guard is not `Send` and which counts the guards
a thread holds. A hop that would wait while the thread holds a guard answers `HOP_WITH_LOCK_HELD` at
once instead of waiting, so the deadlock of a thread that holds a lock and waits for the server
thread, while a handler of the same plugin waits for that lock on the server thread, cannot happen
with locks of the client. The `clippy.toml` of every plugin crate lists the locks of the standard
library and of common crates under `disallowed-types`, so a conversion turns `synchronized` and the
locks of Java into `PluginLock`. The watchdog names the deadlock of a lock it cannot see, see
[11.4](#114-containment).

## 8. Names

### 8.1 Objects in the schema and across the JNI

Rust never holds a Java object of the game. Every object has a small value as its name, which the
Java side turns back into the object with one lookup. Every id that is no UUID and no key has the
form `<kind>:<value>`.

| Object | In the schema | In Rust | Across the JNI | Java lookup | Lifetime |
| --- | --- | --- | --- | --- | --- |
| Entity, player included | `id: ID!`, the UUID, beside `uniqueId` and `entityId: Int!` of Bukkit | `EntityRef { id: i32, level: u16, uuid: Uuid }`, `Copy` | Two ints, the UUID as two longs for a miss | `ServerLevel.getEntity(int)` on the hinted level, then `ServerLevel.getEntity(UUID)` on every level | The run. A removed entity answers `ENTITY_GONE`. A mob that changes dimension gets a new runtime id and keeps its UUID, so a miss on the id falls back to the UUID, as CraftBukkit keeps one Bukkit entity across the change |
| Offline player | `OfflinePlayer.uniqueId` | `Uuid` | Two longs | `PlayerList.getPlayer(UUID)`, then `playerdata/<uuid>.dat` | Forever |
| World | `id: ID!`, the key of the dimension such as `minecraft:overworld`, `name`, `uid` | `WorldRef(u16)`, an index into the level table | One short | `ServerLevel[]`, filled when a level loads | Until the level unloads |
| Chunk | `World.chunkAt(x:, z:)` | `ChunkRef { world, x, z }` | A short and two ints | `ServerChunkCache` | A value |
| Block | `World.blockAt(x:, y:, z:)` with `world`, `x`, `y`, `z` | `BlockRef { world, pos: i64 }` | A short and `BlockPos.asLong()` | The level and the position | A value, a place and no object |
| Block data | `BlockData.asString: String!`, `stateId: Int!` for the run | `BlockData(u32)`, the id of `Block.BLOCK_STATE_REGISTRY` | One int | The state registry | The run. Rust reads the key and the properties of an id once |
| Block state | A snapshot with its tile NBT as SNBT | `BlockStateSnapshot { block, data, nbt }`, a value | The position, and NBT bytes on demand | `Level.getBlockEntity` | A value until `update` |
| Item stack | `ItemStack` with `type`, `amount`, `itemMeta`, `snbt`, and as an argument the SNBT of the stack | `ItemStack { item: ItemId, count: i32, tag: Option<Nbt> }`, a value | The item id, the count, and NBT bytes when the tag is not empty | `BuiltInRegistries.ITEM` | A value, never a live mirror |
| Inventory | `id: ID!` such as `player:<uuid>`, `entity:<uuid>`, `block:<world>/<x>/<y>/<z>`, `custom:<n>` | `InventoryRef` | A kind byte and two ints or longs | The inventory of the player, the container of the entity, the block entity, the table of custom inventories | As its holder |
| Inventory view | `id: ID!` such as `view:<uuid>/<container id>` | `ViewRef { player, container }` | Two ints | `player.containerMenu` with its id | Until the menu closes |
| Event | `id: ID!` such as `event:<depth>.<serial>` | `EventRef { depth, serial }` | The depth of the frame | The frame stack | The dispatch, then `EVENT_CLOSED` |
| Command sender that is no entity | `ConsoleCommandSender` with `id: ID!` `console`, `RemoteConsoleCommandSender` with `rcon:<n>`, `BlockCommandSender` with `commandblock:<world>/<x>/<y>/<z>`, `ProxiedCommandSender` with `proxied:<n>`, each with `name` and `permissionLevel`, and `Server.consoleSender` | `SenderRef`, an enum of `Entity(EntityRef)`, `Console`, `Rcon(u32)`, `CommandBlock(BlockRef)` and `Proxied(u32)`, `Copy` | A kind byte and the values of its kind | `MinecraftServer.createCommandSourceStack()` for the console, the source of RCON client `<n>`, the `BaseCommandBlock` of the block entity, and for a proxied sender the source that the native of the command handed in, kept for that call | The console the run, an RCON client its connection, a command block its block, a proxied sender the command call, then `SENDER_GONE` |
| Plugin | `Plugin.name` | `PluginId(u16)` | Never crosses | Rust only | The run |
| Objects that plugins make | `BossBar.id`, `Scoreboard.id`, `MapView.id`, `PermissionAttachment.id`, `Task.id` | Small typed ids | One int | Tables of Golem in the mod or in Rust | Until removed, or the plugin is disabled |

A command block in a minecart is the entity `CommandMinecart` and takes its UUID, and an entity
that runs a command is that entity as its sender. The fields of `Permissible` answer for the senders
that are no entity as CraftBukkit answers them: the console, an RCON client and a command block are
operators, `isOp` is true for them (`CraftConsoleCommandSender`, `CraftRemoteConsoleCommandSender`
and `CraftBlockCommandSender` in the Arclight jar), so a permission whose default is `OP` holds,
with their attachments above it, and a proxied sender answers as its caller, as
`ProxiedNativeCommandSender` does. `permissionLevel` is 4 for the console and RCON, 2 for a command
block, and the level of the source for a proxied sender. A plugin runs a command as the console with
`serverDispatchCommand(sender: "console", commandLine:)`, which the node
`golem.api.mutation.serverdispatchcommand.console` guards, and the mod turns the id back into its
source with the lookup of the table. `Sender::Console` and `Sender::Other` of the runtime of today
map onto these kinds when the first area moves the senders onto the contract.

World names keep the names Arclight gave the levels of the live server, so the homes, warps and
spawns that EssentialsX stored by world name keep working: the level name for the Overworld, the
level name with `_nether` and `_the_end` for the two others, and for every other dimension the rule
Arclight uses, which the usage audit of [Plan](plan.md#before-the-port) reads off the live server.
`World.uid` reads the `uid.dat` that Arclight wrote into each world folder and writes one where none
exists.

### 8.2 Registries and closed sets

Content of a registry is a `String` key in the schema, such as `minecraft:stone`, so the items,
blocks and entities of the 265 mods work, which a GraphQL enum could not hold. That covers
`Material`, `EntityType`, `Biome`, `Sound`, `Particle`, `Enchantment`, `PotionEffectType`,
`Attribute`, `Statistic`, `Art`, the variants of villagers, cats and frogs, the trims, the loot
tables and the structures. The mapping names the registry of each such type. Across the JNI a key is
the int id of its registry, and Rust keeps the tables of keys, read once at `serverAboutToStart`,
before any plugin loads and after the registries of the world are frozen, and again for the tags on
every `TagsUpdatedEvent`. `golem-client` has a
constant for every constant of Bukkit, such as `Material::GRASS_BLOCK`, which is the interned key
`minecraft:grass_block`, and `Query.material(key:): MaterialInfo` answers the methods of `Material`,
such as `isEdible` and `maxDurability`, from a table, so `Material::STONE.is_solid()` costs a table
read. Closed sets of Bukkit, such as `GameMode`, `BlockFace`, `EventPriority`, `TeleportCause`,
`DamageCause`, `InventoryType` and `ClickType`, are GraphQL enums with the constant names of Bukkit.

### 8.3 Type names

A GraphQL type keeps the simple name of its Bukkit type, and these rules make the names unique.
`inventory.json` holds 47 simple names that more than one package uses, counted by
`work-final-opus/scripts/counts.py` on 2026-11-06.

1. The legacy package `org.bukkit.material` gets no types. Its members map with `via`, see
   [section 14](#14-coverage), which removes 26 of the clashes.
2. Every type of `org.bukkit.block.data` and its subpackage but `BlockData` takes the suffix `Data`,
   so `org.bukkit.block.data.type.Chest` is `ChestData` and `Waterlogged` is `WaterloggedData`,
   while the block state `org.bukkit.block.Chest` keeps `Chest`.
3. Every type of `org.bukkit.inventory.meta` and its subpackages whose name does not end in `Meta`
   takes the suffix `Meta`, so the meta `Damageable` is `DamageableMeta`.
4. The types of `org.bukkit.generator.structure` take the prefix `Generator`, so
   `GeneratorStructure` and `GeneratorStructureType`.
5. An inner type keeps its own name where that name is unique, such as `DamageCause`, and is joined
   to the name of its outer type where it is not, such as `PlayerSpigot` and `BoatType`.
6. A type with subtypes is an interface, and the object type of its direct instances takes the
   suffix `Object`, such as `ZombieObject` and `EntityDamageEventObject`.
7. `api/names.toml` names every remaining exception, which after these rules is one,
   `org.bukkit.block.Structure` as `StructureBlock`, by `work-final/names.py` over the inventory on
   2026-11-06.

The generator refuses any name that two types would share, so a clash is a build error and never a
silent rename.

### 8.4 Member names

One rule decides every name: a read is a field of its declaring type, and a write is a root
mutation named after its declaring type and its method, whose first argument names the receiver
after its declaring type. A type with one instance, such as `Server` and the managers it answers,
`PluginManager`, `BukkitScheduler`, `ScoreboardManager`, `Messenger`, `HelpMap`, `ServicesManager`
and `StructureManager`, takes no receiver argument. Arguments take the names of the parameters in
the sources jar.

| Bukkit | Schema | Client | Example |
| --- | --- | --- | --- |
| A getter `getX()`, `isX()`, `hasX()` with no argument | Field `x`, `isX`, `hasX` of the declaring type, repeated on every implementing object type | `fn x(&self, g) -> Result<T>` | `getLocation` is `location`, `isDead` is `isDead` |
| A getter with arguments | A field with arguments named as in the sources jar | A method with the same arguments | `getNearbyEntities(x, y, z)` is `nearbyEntities(x:, y:, z:)` |
| A setter or an action | `Mutation.<declaringType><Method>` with the receiver as its first argument, answering the receiver or the result | `fn set_x(&self, g, ...) -> Result<()>` | `Damageable.setHealth` is `Mutation.damageableSetHealth(damageable:, health:)` |
| A method of `Server`, or a static of `Bukkit`, which delegates to `Server` | A field of `Server`, reached as `Query.server`, for a read, and `Mutation.server<Method>` for a write | A method of `g.server()` | `getOnlinePlayers` is `Server.onlinePlayers`, `createInventory` is `Mutation.serverCreateInventory`, `CommandMap.register` is `Mutation.commandMapRegister` |
| An event class a listener can name | `Subscription.<event without Event>` | A subscription of `enable` | `PlayerMoveEvent` is `Subscription.playerMove` |
| A setter of an event | `Mutation.<eventType><Method>`, which writes the frame | `event.set_x(...)` | `Mutation.playerMoveEventSetTo(playerMoveEvent:, ...)`, `Mutation.cancellableSetCancelled(cancellable:, cancel:)` |
| A constructor of an event with `callEvent` | `Mutation.call<Event>` | `g.events().call(...)` | `Mutation.callEntityRegainHealth` |
| A constructor of a value | A `lib` constructor of the client, and an argument in a text form | `ItemStack::new(Material::DIAMOND, 1)` | `new ItemStack(...)` |
| Overloads with one meaning | One coordinate with optional arguments | One method with `Option` arguments | `getBlockAt(int, int, int)` and `getBlockAt(Location)` are `World.blockAt` |
| An enum constant | An enum value or a registry key | A constant | `GameMode.CREATIVE`, `Material.STONE` is `minecraft:stone` |
| A pure helper of a value, `equals`, `hashCode`, `toString` and `clone` of values | No coordinate, a `lib` target | A method of the value type, `PartialEq`, `Hash`, `Display`, `Copy` | `Location.distance` |
| A mutable collection that a getter answers | The field, and a setter mutation for the whole collection | `recipients_mut()` | `Mutation.asyncPlayerChatEventSetRecipients` |
| A Java callback type that the game calls later | A synchronous subscription whose subscriber is a closure, see [section 10](#10-events) | A closure | `CommandExecutor`, `TabCompleter`, `MapRenderer`, `EventExecutor` |
| A functional argument that the member calls during the call itself, a `Predicate`, `Function` or `Consumer` of an element | The field without the function, with the filter arguments a document can state instead, such as `types:` | A method with a closure, which the client keeps in a slot of the thread for the operation and the bridge calls back for each element through the native of the spawn closure of [9.2](#92-entities), one crossing per element, in the order of CraftBukkit | `getNearbyEntities(Location, double, double, double, Predicate<Entity>)`, `rayTraceEntities(..., Predicate<Entity>)`, `generateTree(..., Predicate<BlockState>)`, `spawn(Location, Class, Consumer)` |
| `getLogger()` of `Plugin`, `Server` and `Bukkit`, which answer a `java.util.logging.Logger` | No coordinate, a `lib` target | `g.logger()`, the logger of the runtime, which writes to the log of golem with the name of the plugin as `PluginLogger` prefixes it, `SEVERE` as error, `WARNING` as warn, `INFO` as info, `CONFIG` and `FINE` as debug, `FINER` and `FINEST` as trace | `getLogger().info(...)` is `g.logger().info(...)` |
| A class argument, such as `getEntitiesByClass(Class<T>)` | A field with one list per interface | A generic method | `World.entitiesOf { zombies ageables }` |

Every mutation is a root field of `Mutation`, whose fields GraphQL runs one after the other, so a
document that sets two things sets them in its order, and no mutation hangs below a namespace
object, whose fields GraphQL would not order. The rules are what the generator writes into the
skeleton of the mapping. The mapping holds the exact coordinate of every member, so a rule is a
convenience and never the truth, and a person decides the 192 methods and 2 fields that the rules
leave open, by the count of the prototype `scripts/coverage_rules.py` of the panel over
`inventory.json` on 2026-11-06. Coordinates of Golem's own follow the same rule: reads of a world,
such as `safeLocation`, are fields of `World`, and the tick stream is `Subscription.serverTick`,
named like the `Server*` events.

## 9. The areas of the Bukkit API

Each area names the schema types, the main coordinates and how the game delivers them. A Mixin is a
hook of the bridge mod in the code of Minecraft, written by Golem against the official names of
Mojang and reobfuscated to SRG, at the point where Arclight hooks, under the clean room rule of
[section 15](#15-licensing-and-the-clean-room).

### 9.1 The server and the plugin manager

`Query.server: Server` holds what `Server` and `Bukkit` answer: `name`, `version`, `bukkitVersion`
(the value of spigot-api, `1.20.1-R0.1-SNAPSHOT`), `onlinePlayers`, `player(name:)`, `maxPlayers`,
`motd`, `worlds`, `defaultGameMode`, `allowFlight`, `viewDistance`, `ticksPerAnimalSpawns`,
`banList(type:)`, `whitelistedPlayers`, `operators`, `pluginManager`, `scheduler`,
`scoreboardManager`, `helpMap`, `messenger`, `servicesManager`, and Golem's own `tps`, `mspt`,
`source`, the budgets of the plugins and the tick statistics with the time in the API slot. Root
mutations: `serverDispatchCommand(sender:, commandLine:)`, `serverBroadcastMessage`,
`serverShutdown`, `serverSavePlayers`, `serverReloadData`, `serverSetWhitelist`, `banListAddBan`.
`pluginManager` lists the plugins with their state, `plugin(name:)` answers one, and
`Mutation.pluginManagerEnablePlugin` and `pluginManagerDisablePlugin` change it, see
[section 11](#11-packaging-and-lifecycle). A command reply goes to its sender as a system message,
as [Bridge](bridge.md#commands) says.

### 9.2 Entities

The 171 interfaces of `org.bukkit.entity` and its subpackages are GraphQL interfaces, and the object
types are the 117 leaves plus the `Object` types of the interfaces whose direct instances exist,
such as `ZombieObject`, `SpiderObject`, `SlimeObject`, `CowObject` and `ItemFrameObject`. A modded
entity, which has no Bukkit interface of its own, gets the object type of its nearest vanilla base:
an entity whose class extends `Zombie` is a `ZombieObject`, and one that extends an abstract base of
vanilla is the `Object` type of the Bukkit interface of that base, such as `MonsterObject` or
`AnimalsObject`, as Arclight wraps such entities in its generic classes. The type of an entity is
decided once per entity type from the Java class and kept in Rust.

`Query.entity(id:)`, `World.entities`, `World.livingEntities`, `World.entitiesOf` with one list per
entity interface for `getEntitiesByClass`, `World.nearbyEntities(x:, y:, z:, dx:, dy:, dz:)`.
Mutations of every setter, `entityTeleport`, `entityRemove`,
`worldSpawnEntity(world:, x:, y:, z:, type:, snbt:)`, `livingEntityAddPotionEffect`,
`attributeInstanceAddModifier`, `mobSetTarget`. The callback of
`World.spawn(Location, Class, Consumer)`, which runs before the entity joins the world, is a closure
of the client, `world.spawn(g, at, EntityType::ZOMBIE, |g, zombie| ...)`, which the bridge runs
between the creation and `addFreshEntity`. Attributes, potion effects and their types follow the
same rules, with the keys of their registries.

### 9.3 Worlds and chunks

`World` carries the 200 methods of Bukkit's `World` and the 30 of `RegionAccessor`: `name`, `uid`,
`environment`, `seed`, `time`, `fullTime` as `Number`, `weather`, `difficulty`, `spawnLocation`,
`gameRuleValue(rule:)`, `worldBorder`, `highestBlockYAt(x:, z:)`, `rayTraceBlocks(...)`,
`chunkAt(x:, z:)`, `isChunkLoaded(x:, z:)`, `loadedChunks`, `persistentDataContainer`, and Golem's
own `safeLocation(x:, y:, z:, radius:)` of the first game interface. Mutations: `worldSetTime`,
`worldSetStorm`, `worldSetGameRule`, `worldCreateExplosion`, `worldSpawnParticle`, `worldPlaySound`,
`worldLoadChunk`, `worldUnloadChunk`, `worldAddPluginChunkTicket`.

A read of Bukkit in an unloaded chunk loads it on the server thread, and so does Golem for a plugin
in the process, since plugins rely on it. Such coordinates carry `loads_chunk = true` in the
mapping, the runtime counts the loads per plugin, and a plugin that loads chunks this way gets one
log line a minute with the count. A remote operation never loads a chunk on the server thread, see
[13.4](#134-limits). `ChunkSnapshot` is a value that Rust copies in one call, the block state ids
and the biomes of the sections.

### 9.4 Blocks, block data and block states

`Block` is a place, as in Bukkit: `World.blockAt(x:, y:, z:)` answers it without loading anything,
and its fields `type`, `blockData`, `state`, `lightLevel`, `lightFromSky`, `biome`, `temperature`,
`isPassable`, `relative(face:, distance:)` and `drops(tool:)` read the game. Mutations:
`blockSetType(world:, x:, y:, z:, type:, applyPhysics:)`,
`blockSetBlockData(world:, x:, y:, z:, data:, applyPhysics:, ifType:)`, `blockBreakNaturally`.
`ifType` is a precondition: the write happens only while the block still holds that type, and the
answer says whether it did.

`BlockData` is an immutable value named by its state id. The 123 types of `org.bukkit.block.data`
and `org.bukkit.block.data.type` are interfaces and object types with the suffix `Data`, `BlockData`
itself aside, such as `AgeableData`, `DirectionalData`, `BisectedData`, `WaterloggedData`,
`StairsData`, `SlabData`, `DoorData` and `ChestData`, decided from the vanilla block class, so a
modded stair that extends `StairBlock` is `StairsData`. A modded block with properties of its own is
`BlockDataObject` with the generic `properties: [BlockProperty!]!`. `Query.createBlockData(text:)`
parses the text form that `Bukkit.createBlockData` takes. In the client
`data.with(Facing, BlockFace::North)` is pure Rust over the property table of the state.

`BlockState` is a snapshot, as in Bukkit, with the interfaces `TileState`, `Container`, `Lockable`,
`Nameable` and `Lootable` and one object type per block entity type of vanilla, `Chest`, `Furnace`,
`Sign`, `Banner`, `Beacon`, `CreatureSpawner`, `ShulkerBox` and the rest of the 40 subinterfaces,
with `ModTileState` for the block entities of mods. A snapshot holds the block data and the tile NBT
as SNBT. Edits are pure Rust, and
`Mutation.blockStateUpdate(world:, x:, y:, z:, state:, force:, applyPhysics:)` writes it back
through `BlockEntity.load` and `sendBlockUpdated`, as `BlockState.update` does.

### 9.5 Items, materials and meta

`ItemStack` is a value: `type`, `amount`, `maxStackSize`, `durability`, `enchantments`, `itemMeta`,
`snbt`. The 34 types of `org.bukkit.inventory.meta` and its subpackages, `ItemMeta`, `BookMeta`,
`SkullMeta`, `PotionMeta`, `LeatherArmorMeta`, `FireworkMeta`, `BannerMeta`, `MapMeta`,
`EnchantmentStorageMeta`, `CrossbowMeta`, `CompassMeta`, `BlockStateMeta`, `ArmorMeta`,
`SpawnEggMeta`, `DamageableMeta` and the rest, are read and written by `golem-nbt` in Rust from the
tag of the stack, in the layout CraftBukkit 1.20.1 writes, such as `display.Name` as a JSON text
component and the persistent data under `PublicBukkitValues`, so the items that Arclight wrote read
the same. Reading or building an item needs no JNI call. `Query.material(key:): MaterialInfo`
answers the methods of `Material` from the table of [8.2](#82-registries-and-closed-sets), and
`ItemFactory` is `lib` over the same table. The `LEGACY_` constants of `Material` are `via` their
current key.

### 9.6 Inventories and views

`Inventory` with `id`, `type`, `size`, `contents`, `item(index:)`, `viewers`, `holder`, `location`,
`maxStackSize`, `firstEmpty`, `contains(type:, amount:)`, and its subinterfaces `PlayerInventory`,
`CraftingInventory`, `FurnaceInventory`, `BrewerInventory`, `AnvilInventory`, `MerchantInventory`,
`HorseInventory`, `DoubleChestInventory` and the others. A container of a mod that only offers the
Forge capability `ITEM_HANDLER` is `ModInventory`. Mutations:
`inventorySetItem(inventory:, index:, item:, ifCurrent:)`,
`inventoryAddItem(inventory:, items:): [LeftoverItem!]!` with the leftovers that did not fit,
`inventoryRemoveItem(inventory:, items:): [LeftoverItem!]!` with the leftovers that could not be
removed, as `Inventory.removeItem` answers them in a map, `inventoryClear`,
`playerInventorySetHelmet` and the other slots, and Golem's own
`inventoryTransferItem(from:, to:, items:): [LeftoverItem!]!`. `ifCurrent` is the SNBT of the stack
the caller expects in the slot, so a remote client that read a slot in one operation cannot
duplicate an item by writing it in the next.

`InventoryView` is the open menu of a player: `topInventory`, `bottomInventory`, `title`, `type`,
`cursor`, `slotType(rawSlot:)`, `convertSlot(rawSlot:)`. Mutations: `humanEntityOpenInventory`,
`humanEntityCloseInventory`, `inventoryViewSetCursor`, `inventoryViewSetTitle`,
`humanEntityOpenMerchant`.

Menus of plugins are the most common use of inventories. `Mutation.serverCreateInventory(owner:,
type:, size:, title:)` makes a `SimpleContainer` in a table of the mod, with `owner` as the tag of a
custom holder, owned by the plugin and freed when it is disabled. A Bukkit plugin tells its menus
apart with `getHolder() instanceof MyMenu`, and Golem gives a custom inventory `holder:
CustomInventoryHolder` with the plugin and the tag, so that check becomes `inventory.custom_tag(g)?
== Some("my_menu")`. Clicks come from a Mixin at `AbstractContainerMenu.clicked` with the click type
that `handleContainerClick` read, and a cancel resends the menu with `sendAllDataToRemote`.

### 9.7 The scheduler

| Bukkit | Golem |
| --- | --- |
| `runTask`, `runTaskLater`, `runTaskTimer` and the `scheduleSync` forms | `Subscription.schedule(delay:, period:)`, whose subscriber is the closure of the task on the server thread: `g.scheduler().run_later(20, |g| ...)` |
| `runTaskAsynchronously` and the async timers | The pool `golem-async-<n>`, the closure gets an `AsyncGame` |
| `callSyncMethod` | `g.on_server(|g| ...)`, a future, see [7.5](#75-hops) |
| `cancelTask`, `cancelTasks(plugin)` | `Mutation.bukkitSchedulerCancelTask(taskId:)`, `Mutation.bukkitSchedulerCancelTasks(plugin:)`, and dropping the handle of the task |
| `isQueued`, `isCurrentlyRunning`, `getPendingTasks`, `getActiveWorkers` | `Server.scheduler { pendingTasks { id plugin sync delay period } activeWorkers { id plugin } }` |
| `BukkitRunnable` | A `lib` type of the client with the same methods |

Sync tasks run in the `tick` native after the plugins see the tick, in the order of their due tick
and then of their registration, as CraftBukkit runs them, in the same tick as Bukkit's and never a
tick late.

### 9.8 Commands and tab completion

`plugin.toml` declares the commands as `plugin.yml` does, with `aliases`, `description`, `usage`,
`permission` and `permission-message`. `Server.commandMap { knownCommands }`, Golem's own, since the
API of Bukkit reaches the command map only through CraftBukkit, lists every command with its plugin,
and `Mutation.commandMapRegister(fallbackPrefix:, name:, aliases:, description:, usage:,
permission:)` adds one at run time, as `CommandMap.register` does, and sends the command tree to
every player again.

The executor of a command is the subscriber of `Subscription.commandExecuted(name:)`, which answers
the `boolean` of `onCommand`, written as `Mutation.commandCallAnswer(call:, success:)` or, in Rust,
as the return value of the closure. Tab completion is the subscriber of
`Subscription.tabComplete(name:)`, which answers the suggestions. The mod registers each command as
today, a literal with a greedy string argument, with `.suggests(...)` calling the native `complete`,
which Minecraft runs on the server thread when the client asks, and with `.requires(...)` calling
the permission of the command, so a player sees only the commands they may run, as CraftBukkit sends
them. Brigadier evaluates `requires` only when it builds the tree it sends, so the tree follows
every change of permissions as [9.9](#99-permissions) says. A command may also declare its arguments
as a tree of Brigadier argument types in `plugin.toml`, which gives the client its own checks. A
process plugin completes through the future of Brigadier: `handleCustomCommandSuggestions` waits for
`getCompletionSuggestions` with `thenAccept`, so the suggestion provider answers a future that the
answer of the process completes, within 200 ms, else with the static tree of its `plugin.toml`, and
neither the server thread nor Netty waits.

A plugin command named like a vanilla command takes the plain name, and the vanilla command stays
reachable as `minecraft:<name>`, as CraftBukkit orders them: `CraftServer.enablePlugins` enables the
plugins first, registers the vanilla commands of the Spigot setting `commands.replace-commands`
under their plain name, and registers every other vanilla command with the fallback prefix
`minecraft`. Golem keeps that setting with Spigot's default, `setblock`, `summon`, `testforblock`
and `tellraw`, which closes the open item of [Open items](open-items.md) on such names. The level
of the source of a command, which the bridge passes since the first game interface, is
`CommandSender.permissionLevel`, Golem's own, and decides the default `OP` of a permission for a
source without a profile.

### 9.9 Permissions

`golem-permissions` stays the one source of truth. `Permissible.hasPermission(name:)`,
`isPermissionSet(name:)`, `effectivePermissions` and `isOp` are fields of every command sender,
answered in Rust from the tables of `golem-permissions`, which are rebuilt on every change, so they
cost no JNI call and run on any thread. `isOp` of a player is whether the list of operators holds
its profile, at any level, as `CraftPlayer.isOp()` answers it.
`Mutation.permissibleAddAttachment(permissible:, plugin:, ticks:)` answers a `PermissionAttachment`
with an `id`, and `permissionAttachmentSetPermission`, `permissionAttachmentUnsetPermission` and
`permissionAttachmentRemove` change it. Attachments are transient nodes that rank above the nodes of
the player, as the transient nodes of LuckPerms do, and go when the plugin is disabled or after
their ticks. `Server.pluginManager { permissions }` lists the permissions that plugins declared,
with `default` of `PermissionDefault` (`TRUE`, `FALSE`, `OP`, `NOT_OP`) and `children`, and a node
that no rule of the configuration decides takes that default, which rule 8 of `golem-permissions`
leaves to the caller. The handler of the Forge PermissionAPI answers from the same tables, so mods
see the same groups.

Every change of the effective permissions of an online player, a node or a group of
`golem-permissions`, an attachment set, unset, removed or expired, `setOp`, or a permission or a
default that a plugin registers or that a disable releases, sends that player the command tree
again through `Commands.sendCommands`, coalesced to one send per player at the end of the tick.
That is what the live server does: the command list updater of LuckPerms, whose port
`golem-permissions` is, calls `Player.updateCommands()` after every recalculation of a user or of a
group the user inherits, under its setting `update-client-command-list`, true by default
(`AbstractCommandListUpdater` and `BukkitCommandListUpdater` of LuckPerms), while CraftBukkit itself
resends only on `setOp`, through the operator list of vanilla, and offers `Player.updateCommands`,
which is `Mutation.playerUpdateCommands(player:)`.

### 9.10 Configuration

The 270 methods of `org.bukkit.configuration` are the library `golem_client::config`, which offers
Bukkit's `ConfigurationSection` with `getString`, `getInt`, `getList`, `isSet`, `set`,
`getConfigurationSection` and `save` over a tree, so a converted plugin keeps the calls of its
upstream. The files of a Golem plugin are TOML, which the workspace already reads with its `toml`
crate, and the migration tool of [Plan](plan.md#data-migration) converts the `config.yml` of a
plugin to TOML once, as it reads the rest of the plugin data once. The operator side is in the
schema: `Plugin.config: ConfigSection` with `keys(deep:)`, `value(path:): JSON` and `isSet(path:)`,
and the mutations `pluginConfigSet(plugin:, path:, value:)`, `pluginConfigSave(plugin:)` and
`pluginConfigReload(plugin:)`. The data of a plugin lives in `golem-data/plugins/<name>/` outside
the tracked tree.

### 9.11 Persistent data containers

`PersistentDataContainer` with `keys`, `has(key:, type:)` and `get(key:, type:): JSON`, on the
holders `Entity`, `ItemMeta`, `Chunk`, `World` and `TileState`, and
`Mutation.persistentDataContainerSet(persistentDataContainer:, key:, type:, value:)` and
`persistentDataContainerRemove`, where the id of the container names its holder. The data stays
exactly where CraftBukkit and so Arclight kept it on the live server.

| Holder | Tag |
| --- | --- |
| Entity | `BukkitValues` at the root of the NBT of the entity |
| Item | `PublicBukkitValues` in the tag of the stack |
| Block entity | `PublicBukkitValues` in the NBT of the block entity |
| Chunk | `ChunkBukkitValues` in the NBT of the chunk |
| World | `BukkitValues` in the level data |

Vanilla drops the tags it does not know when it loads an entity, a block entity, a chunk or the
level data, so the bridge carries these tags through `Entity.load` and `saveWithoutId`,
`BlockEntity.load` and `saveAdditional`, `ChunkSerializer.read` and `write` and the level data with
Mixins from the first day of the area, and the notebook of the area checks every key on a copy of
the live world before and after a save. Without them, the persistent data that Bukkit plugins
stored on the live server would vanish at the first save under Golem.

### 9.12 Metadata

Metadata is transient and lives in Rust only, keyed by the object, the key and the plugin, with
values of `JSON`. `Entity.metadata(key:)`, `Block.metadata(key:)` and `World.metadata(key:)` answer
`[MetadataValue!]!` with `plugin` and `value`, and `Mutation.metadatableSetMetadata` and
`metadatableRemoveMetadata` change it. The metadata of an entity goes with the entity, that of a
block with the world. A plugin that stored Java objects as metadata keeps a map of its own keyed by
`EntityRef`.

### 9.13 Scoreboards

The main scoreboard is the one of vanilla. Bukkit also lets a plugin give a player a scoreboard of
their own, which vanilla does not know and many sidebar plugins depend on. The mod keeps the
scoreboards of Golem in a table, sends the packets of a custom scoreboard to the players assigned to
it and keeps the packets of the main scoreboard from them, with a Mixin on the broadcast of
`ServerScoreboard`. Schema: `Scoreboard` with `id`, `objectives`, `teams`, `entries`,
`objective(name:)`, `team(name:)`, `scores(entry:)`, `Server.scoreboardManager { mainScoreboard }`,
and the mutations `scoreboardManagerNewScoreboard`, `scoreboardRegisterNewObjective`,
`objectiveSetDisplaySlot`, `scoreSetScore`, `teamAddEntry`, `teamSetPrefix` and
`playerSetScoreboard`.

### 9.14 Boss bars

`BossBar` and `KeyedBossBar` with `id`, `title`, `progress`, `color`, `style`, `flags`, `players`
and `isVisible`. `Mutation.serverCreateBossBar(title:, color:, style:, flags:)` makes a
`ServerBossEvent` owned by the plugin, and `serverCreateKeyedBossBar` one of `CustomBossEvents`,
which `/bossbar` sees. `bossBarSetProgress`, `bossBarAddPlayer`, `bossBarRemoveAll`. The bars of a
plugin go when it is disabled, and keyed bars stay, as vanilla keeps the bars of `/bossbar`.

### 9.15 Maps

`MapView` with `id`, `scale`, `centerX`, `centerZ`, `world`, `isLocked`, `isTrackingPosition` and
`renderers`, and the mutations `serverCreateMap(world:)`, `mapViewSetScale` and
`mapViewSetCenterX`. A `MapRenderer` is the subscriber of `Subscription.mapRender(map:)`, which the
bridge calls with a canvas of 128 by 128 colour bytes when vanilla builds the update packet of the
map for a player, as CraftBukkit calls its renderers there. `MapCanvas.setPixel`, `drawImage`,
`drawText` and the cursors are `lib` methods of that canvas. A remote client paints a static map
with Golem's own `Mutation.mapCanvasSetPixels(map:, x:, y:, width:, height:, colors:)`.

### 9.16 Chat components and messages

A text component is `JSON` that holds the text component of Minecraft, the form
`Component.Serializer` reads. The chat API of `net.md_5.bungee.api.chat`, `TextComponent`,
`TranslatableComponent`, `ClickEvent`, `HoverEvent` and `ComponentBuilder`, is a `lib` builder of
the client that writes that JSON. `CommandSender.sendMessage(String)` is
`Mutation.commandSenderSendMessage(commandSender:, message:)`, and the inner types `Spigot` of
`Player`, `Entity`, `CommandSender`, `Server`, `World` and `BookMeta` are object types such as
`PlayerSpigot`, reached by the field `spigot`, whose setters are mutations such as
`Mutation.playerSpigotSendMessage(playerSpigot:, position:, components:)`. `ChatColor` and
`translateAlternateColorCodes` are `lib`. Titles, action bars and the player list are
`playerSendTitle`, `playerSetPlayerListHeaderFooter` and `playerSetPlayerListName`. Text that a
plugin echoes from a player goes through `golem_client::text::strip_codes` by default, which closes
the open item on formatting codes.

### 9.17 Plugin messaging

`Messenger.registerOutgoingPluginChannel` and `registerIncomingPluginChannel` are
`Mutation.messengerRegisterOutgoingPluginChannel` and `messengerRegisterIncomingPluginChannel`, the
listener of an incoming channel is the subscriber of `Subscription.pluginMessage(channel:)` with
`player`, `channel` and `data` as base64, and `Player.sendPluginMessage` is
`Mutation.pluginMessageRecipientSendPluginMessage(pluginMessageRecipient:, source:, channel:,
message:)`. The mod opens each channel as an `EventNetworkChannel` of Forge with
`NetworkRegistry.acceptMissingOr`, so clients without the channel still join.

### 9.18 Services

A Bukkit service is an interface with providers, such as the economy of Vault. In Golem a service is
a provider trait declared in a schema part of [4.2](#42-the-schema-parts-of-ports), with providers
that plugins register with a priority, as `ServicesManager.register` does, and that other plugins
load through the client. `Server.servicesManager { registrations { service plugin priority } }`
lists them. The schema part makes the service reachable for tools as well, such as
`Query.economyBalance(player:)` and `Mutation.economyDeposit(player:, amount:)` over the provider
with the highest priority.

### 9.19 World creation and generators

`Mutation.serverCreateWorld(name:, environment:, type:, seed:, generator:, generateStructures:,
hardcore:)` answers a `World`, as `Bukkit.createWorld(WorldCreator)` does, and
`serverUnloadWorld(world:, save:)` unloads it. Vanilla has no dimension that comes into
being at run time, so the mod adds the `ServerLevel` to the levels of the server, writes its
`LevelStem` and sends the dimension to the clients, with Mixins where Arclight does the same. A
generator is a Rust trait object of a plugin, `ChunkGenerator` with `generate_noise`,
`generate_surface`, `generate_bedrock`, `generate_caves`, the `should_generate` flags and
`biome_provider`, which the generator type `golem:plugin` of the mod calls, see
[7.9](#79-world-generation). `World.generator` and `Server.generators` show what runs.

### 9.20 Conversations

The conversation API of Bukkit is plugin code on top of chat. Golem keeps it as a library of
`golem-plugin` with `ConversationFactory`, `Prompt` as a trait, `ValidatingPrompt` and the abandon
listeners, which takes the chat of a conversing player through a subscriber of `asyncPlayerChat` at
`LOWEST` that cancels the message. The state is in the schema: `Player.isConversing`,
`Mutation.conversableAcceptConversationInput` and `conversableAbandonConversation`.

### 9.21 Help

`Server.helpMap { helpTopics helpTopic(topicName:) }` with
`HelpTopic { name shortText fullText }`, filled from the commands and from the `help.toml` of the
operator, and the mutations `helpMapAddTopic` and `helpMapClear`.

### 9.22 Recipes, loot tables and advancements

`Server.recipesFor(result:)` and `recipe(recipeKey:)` answer `Recipe` with the object types
`ShapedRecipe`, `ShapelessRecipe`, `FurnaceRecipe`, `BlastingRecipe`, `SmokingRecipe`,
`CampfireRecipe`, `StonecuttingRecipe`, `SmithingTransformRecipe`, `SmithingTrimRecipe` and
`ModRecipe` for the recipe types of mods. `Server.lootTable(key:)` and the `Lootable` fields
`lootTable` and `seed` of chests, minecarts and mobs, `LootTable.populateLoot(random:, context:)`,
whose `random` is a seed, which rolls a table without touching the world, and
`Mutation.lootTableFillInventory`. `Server.advancement(key:)` with `criteria` and `display`,
`Player.advancementProgress(advancement:)` with `isDone`, `awardedCriteria`, `remainingCriteria` and
`dateAwarded(criteria:)`, and the mutations `advancementProgressAwardCriteria` and
`advancementProgressRevokeCriteria`.

`Mutation.serverAddRecipe` takes effect at once, as `Bukkit.addRecipe` does: `CraftServer.addRecipe`
calls `addToCraftingManager` of the recipe, which hands it to `RecipeManager.addRecipe`, a method
CraftBukkit adds, and sends no packet, so a player who joins later receives it with the recipe list
of the join (javap of the CraftBukkit classes of the Arclight jar, 2026-11-06). The mod adds the
recipe to the maps of `RecipeManager` with a Mixin that makes them writable, and keeps every recipe
of a plugin in a table of Golem that adds them again after each reload of the data packs, before
vanilla sends the recipes to the players, so they also survive `/reload`.
`serverRemoveRecipe(key:)`, `serverClearRecipes` and `serverResetRecipes` change the same maps and
that table. Bukkit adds no loot table and no advancement outside `UnsafeValues`, whose members are
`none`, so no virtual data pack is needed. `LootGenerateEvent` comes from a Mixin at
`LootTable.fill`, and `PlayerAdvancementDoneEvent` from Forge's `AdvancementEarnEvent`.

### 9.23 Structures

The templates of `org.bukkit.structure`: `Server.structureManager { structures
structure(structureKey:) }` with `palettes`, `entities` and `size`, and the mutations
`structurePlace` with the place, `includeEntities`, `structureRotation`, `mirror`, `palette`,
`integrity` and `random`, `structureManagerSaveStructure`, and `structureFill` with a box of the
world and `includeEntities`. The structures of world generation, `org.bukkit.generator.structure`,
are `World.locateNearestStructure(...)` and the registry keys of `GeneratorStructure`.

### 9.24 The rest

Bans and the whitelist over the lists of vanilla, player profiles and textures over `GameProfile`,
statistics over `ServerStatsCounter`, sounds, particles and effects, the world border, game rules,
fireworks, banners, raids, enchantments, `org.bukkit.util` with its vectors, bounding boxes, ray
traces and noise generators as `lib`, and the Spigot extras of `org.spigotmc` follow the same rules.
None of them needs a mechanism of its own beyond the rows of [8.4](#84-member-names) for functional
arguments and for the logger.

## 10. Events

### 10.1 The shape in the schema

`inventory.json` holds 268 event classes: 250 concrete, 180 of them cancellable, and 18 abstract,
counted by `work-final-opus/scripts/counts.py` on 2026-11-06. Each is a GraphQL type: an interface
where Bukkit has subclasses, such as `Event`, `PlayerEvent`, `EntityEvent`, `BlockEvent`,
`InventoryEvent`, `Cancellable` and `EntityDamageEvent`, with the `Object` type for the direct
instances of a concrete class with subclasses, and an object type otherwise. The fields of an event
type are its getters and those of its ancestors, `isCancelled` included, and its mutations are its
setters, 134 besides `setCancelled` over all events, so the coverage tool holds that every getter of
an event class is a field and every setter a mutation.

A listener of Bukkit may name 252 classes: the 250 concrete ones and the abstract
`InventoryInteractEvent` and `PlayerChannelEvent`, whose own or inherited handler list a
registration finds. Each of the 252 is a field of `Subscription`, named after the class without
`Event`, so a conversion never has to know which handler list a class uses. 225 classes declare
their own handler list, and a subscription to any other class joins the chain of the nearest handler
list in its class chain with a filter on the class, as the registration of Bukkit does, so
`Subscription.entityDamageByEntity` sits in the chain of `EntityDamageEvent` and the order between
the handlers of the parent and of the child stays that of Bukkit. A subscription to a class with
subclasses delivers them too, so `Subscription.entityDamage` answers `EntityDamageEvent`, whose
possible types include `EntityDamageByEntityEvent`, and `if (e instanceof
EntityDamageByEntityEvent)` is `... on EntityDamageByEntityEvent` or
`event.as_entity_damage_by_entity()` in the client.

Every subscription field takes `priority: EventPriority`, whose description names the default
`NORMAL`, `ignoreCancelled: Boolean`, false by default, and the filters of its event, see
[10.6](#106-what-an-event-costs). The facts of every event, its chain, its hook, its rate class,
its thread and its frame, stand in the `[event]` table of its mapping file, see
[API coverage](api-coverage.md), and `api gen` writes the overview `api/events.toml` from them.

### 10.2 Frames

The mod writes the fields of an event into the frame of its depth at the offsets that the mapping
gives, calls `Host.event(kind, depth)` once and reads the changed fields back. The Rust struct of
the frame, such as `PlayerMoveFrame` with `player`, `from`, `to`, `cancelled` and the mask
`changed`, is what the subscribers read and write through the client. A chain of any length costs
one JNI crossing each way, and reading `event.getFrom()` costs a read of Rust memory.

### 10.3 Order, priorities and `ignoreCancelled`

The dispatcher keeps one chain per handler list, sorted by priority, `LOWEST`, `LOW`, `NORMAL`,
`HIGH`, `HIGHEST` and `MONITOR`, and within a priority by the load order of the plugins and then the
order of registration, which is how CraftBukkit orders a handler list. Within a priority the
subscribers run one after the other, so a later one sees the changes of an earlier one and is
skipped with `ignoreCancelled` after an earlier cancel. Each call is timed against the budget of the
plugin, a panic is caught and counted, and the chain goes on as if the subscriber passed. `MONITOR`
subscribers get the frame read only, and a mutation of the event from them answers
`EVENT_READ_ONLY`. Bukkit only asks that in its documentation, and Golem holds it, which is a quirk
fix with its own issue.

The mod listens to Forge events at the priority `LOW` with `receiveCanceled = true`. The chain of
Golem starts with `cancelled` as the mods above left it, and its verdict is what the mods at
`LOWEST` see, which the `[event]` table records per event. The chat chain runs after every listener
of `ServerChatEvent`, see [7.6](#76-the-chat-and-login-chains).

### 10.4 The synchronous answer of a cancellable event

The Bukkit handler of a cancellable event answers before the game goes on, and the server thread
never waits for anything slow. Both hold because the subscriber is a Rust closure that the
dispatcher calls on the server thread inside `Host.event`. `event.set_cancelled(true)` is the
coordinate `Mutation.cancellableSetCancelled(cancellable:, cancel:)`, which writes the frame, and
the native answers the flag and the changed mask in the same call. A document run inside the handler
does the same through the mutation and the id of the event. Nothing in the path waits for another
thread or a socket, and no deadline cuts a converted handler short ([11.4](#114-containment)), so a
converted handler means what its Java meant, a slow one that loads a chunk included. The chains of
the chat, the login and the ping of [7.6](#76-the-chat-and-login-chains) run the same dispatcher on
the threads of Golem and answer through the chat future, the state of the login and the status
answer.

### 10.5 Calling events and the events of ports

`PluginManager.callEvent` of a Bukkit event, which EssentialsX uses in `/heal` to ask other plugins
whether the heal may happen, is `Mutation.call<Event>` with the arguments of the constructor, or
`g.events().call(...)` in the client: the chain runs synchronously for a frame in Rust memory and
the caller reads the result, cancelled or changed. An event class that a port defines for other
plugins, such as `AfkStatusChangeEvent` of EssentialsX, is declared in its schema part as an event
type, `Subscription.essentialsAfkStatusChange` and `Mutation.callEssentialsAfkStatusChange`, and
runs through the same dispatcher.

### 10.6 What an event costs

- The wanted mask. `Host.WANTED` is a `boolean[]` with one entry per event kind, which Rust writes
  through `SetBooleanArrayRegion` when the first subscriber of a kind arrives and when the last one
  leaves. Every hook tests its entry first, so an event that nobody hears costs one array load and
  builds no frame.
- Filters. Subscription fields take filters that the dispatcher reads from the frame before it calls
  a subscriber, such as `player:`, `world:`, `types:`, `causes:`, `changedBlock:` and `region:`. A
  literal conversion keeps the `if` of Java in Rust, and a filter is an optimization that the
  converter may add.
- Interest tables. The union of the filters that the `pushdown` of the event names, over all
  subscribers of a kind, is folded into arrays of the host class that the Java hook checks before
  it writes a frame: a bit set of the world indexes, bit sets of block state ids and item ids, sets
  of runtime ids, boxes of regions, and the flag of a change of block. The Java check is a
  necessary condition and Rust checks every filter exactly, since a union lets through more than
  each filter. So a move inside one block, the commonest event of the game, costs a compare in Java
  when every subscriber asked for `changedBlock: true`.
- Subject sets. A plugin keeps a named set of entities, such as `essentials:frozen`, through
  `Mutation.subjectSetAdd` and `subjectSetRemove`, and a filter
  `players: {set: "essentials:frozen"}` folds it into the interest table, so a protection check on a
  hot event reaches no closure for the players outside the set.

| Rate class | Events | In process | Remote and process plugins |
| --- | --- | --- | --- |
| A | Rare: join, quit, death, respawn, world load, plugin enable, advancements | Any subscriber | Any subscriber |
| B | Per action of a player: chat, interact, block break and place, inventory click, commands, consume | Any subscriber | Any subscriber |
| C | Frequent: move, damage, pickup, projectile hit, explosion, `PlayerToggle*`, `PlayerVelocityEvent`, `VehicleMoveEvent` | Any subscriber, logged once when it has no filter | Needs a filter, else `HOT_EVENT_NEEDS_FILTER` |
| D | Physics, fluid flow, redstone, hoppers, block form, grow and spread, leaves decay, entity targets, air change | Any subscriber, logged once a minute when it has no filter that Java checks | Refused without a filter that Java checks, `HOT_EVENT_NEEDS_FILTER` |

Bukkit hands every one of these events to any listener, and protection plugins listen to the class
D events without a filter, so the in process side refuses none of them and the mapping stays
mechanical. `/golem status` names a plugin that listens to a class C or D event without a filter.

### 10.7 Remote observers

A remote client and a process plugin observe. After the chain of an event that has remote
subscribers, the dispatcher copies the frame once into an owned snapshot, shared by every remote
subscriber, and hands it to the API thread, which runs each selection over the copy and the
[tick snapshot](#77-the-tick-snapshot). A field of the class `server` inside a remote subscription
is refused when the document is prepared, with `SERVER_FIELD_IN_STREAM`, so the client asks for it
with a query, and no field of an event is read later than the event. The server thread therefore
pays one copy per event and nothing per subscriber. A remote subscription with a priority other
than `MONITOR` is refused when it is prepared with the GraphTP status 403.1 Stream Not Allowed and
the code `REMOTE_PRIORITY`, never downgraded in silence, and `ignoreCancelled: true` drops the
events that ended cancelled. Each remote subscriber has a queue of at most 1,024 events and 1 MiB,
and a subscriber that falls behind gets `EVENTS_LOST` with the count and the end of its stream, so
it resubscribes and reads the state again. At 64 streams per principal that bounds the queues of a
principal at 64 MiB. Coalescing to the latest value through the hub of Aosabi is kept for streams
of state, such as `Subscription.serverTick`, where only the latest value matters.

### 10.8 Where the events come from

The Forge 47.4.18 universal jar holds 236 classes under `net/minecraftforge/event`, by the count of
the panel. A judgement from the two class lists, not a measurement, puts the 252 listenable events
of Bukkit into four hook classes, and the `[event]` table of each holds the class, the source and
the rate class, which the coverage tool checks for each of the 252.

| Class | Events | What the bridge mod does |
| --- | ---: | --- |
| F, Forge enough | 59 | A Forge listener, such as `BlockEvent.EntityPlaceEvent` for `blockPlace` |
| P, Forge and a Mixin | 51 | A Forge listener and a Mixin for what it lacks: Forge's `PlayerRespawnEvent` fires after the respawn, so `playerRespawnEventSetRespawnLocation` needs a Mixin in `PlayerList.respawn`, `BlockEvent.BreakEvent` lacks the drop flag, `ExplosionEvent.Detonate` carries no `@Cancelable`, so a cancel of `entityExplode` clears its block list and a Mixin in `ServerLevel.explode` sends no explosion packet, as CraftBukkit skips the blocks and the packet, and the chat chain composes onto the decorator of `ServerChatEvent` |
| M, Mixin alone | 137 | A Mixin at the place where CraftBukkit fires the event: `playerMove` in `handleMovePlayer`, `inventoryClick` in `handleContainerClick`, `signChange` in `updateSignText`, `blockPhysics` in `Level.neighborChanged`, `serverListPing` in `ServerStatusPacketListenerImpl.handleStatusRequest` and `LegacyQueryHandler`, handed to `golem-net` |
| G, Golem | 5 | `pluginEnable`, `pluginDisable`, `serviceRegister`, `serviceUnregister`, `broadcastMessage` |

For an event that Bukkit can cancel and whose hook is of the class F, the coverage tool checks
against the Forge jar that every Forge class in `forge` carries the annotation `Cancelable` of the
event bus, so an event that Forge cannot cancel moves to the class P with its Mixin. A cross check
of the design on 2026-11-06 found one such event by hand and moved `entityExplode` from F to P,
since `ExplosionEvent.Detonate` carries no such annotation while `ExplosionEvent.Start` does (javap
of the mapped Forge jar). The check finds any other one when the tables are written.

| Bukkit event | Source in Golem | Remark |
| --- | --- | --- |
| `PlayerJoinEvent` | Forge `PlayerLoggedInEvent` and a Mixin at the join message of `PlayerList.placeNewPlayer` | The message is settable in Bukkit |
| `PlayerQuitEvent` | Forge `PlayerLoggedOutEvent` and a Mixin at the quit message | |
| `AsyncPlayerChatEvent` | A Mixin at the return of `MinecraftServer.getChatDecorator()` that composes the chain onto the future of `ServerChatEvent`, and a Mixin at the broadcast for the recipients | Runs on `golem-chat-<n>`, Forge has no recipient list |
| `AsyncPlayerPreLoginEvent` | A Mixin at the end of the authentication thread of `ServerLoginPacketListenerImpl` in online mode and in the offline branch of `handleHello`, each handing the frame to `golem-net` | Vanilla has a thread of its own per login in online mode alone, see [7.6](#76-the-chat-and-login-chains) |
| `ServerListPingEvent` | A Mixin in `handleStatusRequest` and in `LegacyQueryHandler`, handing the frame to `golem-net` | Asynchronous in spigot-api, answered on Netty by vanilla |
| `PlayerCommandPreprocessEvent` | Mixin at `ServerGamePacketListenerImpl.handleChatCommand` before the parse | Forge's `CommandEvent` comes after the parse |
| `PlayerMoveEvent` | Mixin at `handleMovePlayer` | Rate class C |
| `PlayerTeleportEvent` | Mixin at `ServerPlayer.teleportTo` and the portals, Forge `EntityTeleportEvent` for pearls and commands | The causes of Bukkit need the call sites |
| `PlayerInteractEvent` | The Forge `PlayerInteractEvent` variants | `Action` and `useItemInHand` map to the Forge results |
| `BlockBreakEvent` | Forge `BlockEvent.BreakEvent` and a Mixin for `dropItems` | |
| `EntityDamageEvent` and its subclasses | Forge `LivingAttackEvent` and `LivingHurtEvent`, a Mixin at `Entity.hurt` for entities that are not living | The damage modifiers need the Mixin |
| `PlayerDeathEvent` | Forge `LivingDeathEvent` and `LivingDropsEvent`, Mixins for the death message, `keepInventory` and `keepLevel` | |
| `CreatureSpawnEvent` | Forge `MobSpawnEvent.FinalizeSpawn` and `EntityJoinLevelEvent`, a Mixin for the spawn reason | |
| `InventoryClickEvent` | Mixin at `AbstractContainerMenu.clicked` | No Forge event |
| `InventoryOpenEvent` | Forge `PlayerContainerEvent.Open` and a Mixin to cancel the open | Forge's open cannot be cancelled |
| `PlayerRespawnEvent` | Mixin at `PlayerList.respawn` before the place is chosen | Forge's event comes too late for `setRespawnLocation` |
| `BlockPhysicsEvent` | Mixin at `Level.neighborChanged` | Rate class D |

The Mixins of Golem inject at the head or the return of a method, or at one call inside it, and
never overwrite. They live in `bridge/mod/src/main/java/com/skytekx/golem/bridge/mixin/`, are listed
in `golem.mixins.json` and are remapped to SRG with their refmap by MixinGradle beside ForgeGradle,
with the Mixin 0.8.5 that Forge 47 ships. Each hook has one form: check the mask, check the interest
table, write the frame, call `Host.event`, read the outcome. The mods of the pack already inject at
2,569 points of the server code, by the scan of [Plan](plan.md#why-the-game-itself-stays-java), so
the notebook `18_api_hooks` boots the full pack of 265 mods under `golem run` and asserts that every
Mixin of the bridge applied and that every hook fires once in a scripted scene, before an area
merges.

## 11. Packaging and lifecycle

### 11.1 A plugin is a crate

```text
crates/golem-essentials/
  Cargo.toml        the license of the upstream, GPL-3.0-or-later, which ports.toml names
  plugin.toml       the translation of plugin.yml key by key, plus grants and limits
  src/lib.rs        pub fn plugin() -> Arc<dyn Plugin>
  schema/mod.rs     its schema part, mounted into golem-api, if it offers an API
  ops/*.graphql     its documents
  src/ops.rs        written by golem-devtool api ops
  resources/        config.toml, messages, the defaults it writes into its data folder
  clippy.toml       the lock rule of 7.10
```

```toml
# crates/golem-essentials/plugin.toml
name = "Essentials"
version = "2.22.0"
load = "postworld"
softdepend = ["Vault", "LuckPerms"]
grants = ["golem.api.*", "-golem.api.mutation.serverdispatchcommand.console"]

[commands.getpos]
aliases = ["coords", "egetpos", "position", "eposition", "whereami", "ewhereami", "getlocation",
    "egetlocation", "getloc", "egetloc"]
usage = "/<command> [player]"
description = "Get your current coordinates or those of a player."
permission = "essentials.getpos"

[permissions."essentials.getpos.others"]
default = "op"

[limits]
command-deadline-ms = 200
```

`golem` names the plugin crates it is built with as cargo features and lists their entries in
`crates/golem/src/plugins.rs`, and `config/golem.toml` names which of them are enabled at run time.
A plugin is compatible with a schema exactly when its documents and the canonical operations its
client methods use validate against that schema, which the build checks and the runtime checks
again at `enable`, so a schema change that breaks a plugin breaks its build. For documents outside
the workspace, the notebook `11_api_schema` diffs the schema against the last release and validates
the corpus of every known document against it.

### 11.2 The states of a plugin

| Step | When | What the runtime does |
| --- | --- | --- |
| Order | Before the JVM starts | Sorts the plugins by `depend`, `softdepend` and `loadbefore` as Bukkit does, and refuses a cycle or a missing `depend` with one line |
| Load | `serverAboutToStart`, a new native that the mod calls from Forge's `ServerAboutToStartEvent`, before `loadLevel` creates the worlds | `Plugin::load`, where a plugin registers world generators and services |
| Enable | `serverAboutToStart` after every load for `load = "startup"`, and `serverStarting` for `load = "postworld"`, the default, once the worlds are loaded | `Plugin::enable(&mut Enable)`, where it registers subscriptions, commands, tasks, permissions, documents, channels and recipes. A failing or panicking `enable` rolls back what it registered and marks the plugin failed |
| Disable | `serverStopping`, `Mutation.pluginManagerDisablePlugin` or `/golem plugin disable` | The ledger of 11.3 releases what the plugin owns, then `Plugin::disable` runs, and the server thread waits for nothing |
| Enable again | `Mutation.pluginManagerEnablePlugin` or `/golem plugin enable` | `enable` with the same plugin value, once the plugin has left `DISABLING`. There is no unload and no reload of code |

Forge 47.4.18 fires `ServerAboutToStartEvent` in `DedicatedServer.initServer` before `loadLevel`,
the server then logs `Done`, fires `ServerStartingEvent` at the end of `initServer`, and
`MinecraftServer.runServer` fires `ServerStartedEvent` after `initServer` returns and before the
first tick (javap of the mapped Forge jar, 2026-11-06, offsets 467, 491, 533 and 763 of
`initServer`). So a generator registered in `load` reaches every dimension that `loadLevel` creates,
as the `STARTUP` load of Bukkit does, and a plugin enabled at `serverStarting` sees the loaded
worlds, as `POSTWORLD` does. The players join only once the ticks run, since the server thread
places a player in `tick` of the login listener, so every plugin hears the first join, and a login
that reaches `golem-net` earlier waits for the enables, see [7.6](#76-the-chat-and-login-chains).
The `serverStarting` native of the first milestone and its doc comment in `Host.java` and
`golem_bridge::Event`, which say "before the worlds load", fire after them.

### 11.3 The ledger

Golem records every resource that a plugin makes through the schema in a ledger keyed by the plugin,
and releases it at disable in this order: its subscriptions, timers, command and completion handlers
first, so no event reaches a plugin that is disabling, then its subject sets, its commands with the
command trees sent again, its permission attachments with a recalculation, the permissions it
registered, its metadata, its custom inventories with their viewers closed, its boss bars but the
keyed ones, its scoreboards with the players moved back to the main one, its chunk tickets, its
plugin channels, its services and the plugins that depend on them, its map renderers, generators and
conversations. The recipes it added stay by default, since players may hold what they made, and go
with `plugins.keep-data-on-disable = false`. Then `disable` runs.

The server thread never waits for the async calls of a disabling plugin, since such a call may
itself wait in `g.on_server(...).wait()` for the server thread. From the start of the disable every
hop of the plugin, queued or new, answers `PLUGIN_DISABLED` at once, which ends every such wait, and
every further call of its async code into golem answers the same. The plugin stays `DISABLING` until
the count of its async calls in flight reaches zero, `enable` again is refused until then, and the
watchdog logs the plugin with the calls still in flight once `plugins.drain-ms`, 5,000 by default,
has passed. At the stop of the server, `serverStopped` waits for the calls in flight of every plugin
for at most `plugins.drain-ms`, after the worlds are saved and when no tick runs any more, so a
plugin may finish writing its data.

### 11.4 Containment

A compiled plugin is trusted code in the process. Golem stops what it can stop and names what it
cannot. Every value below is a setting of `config/golem.toml`.

| Layer | When | What happens |
| --- | --- | --- |
| Budget | Every call, 1 ms by default, `--budget-ms` | Timed into the budget of the plugin, `over_budget` counts it, a log line at most once a minute with the coordinate that cost the most, as today |
| Deadline | A call past its deadline: 5 ms for an event handler, 50 ms for a command, a tab completion or a sync task, and up to 1,000 ms for a command or a handler whose `plugin.toml` raises it | The call runs to its end, as a Bukkit handler does: the runtime logs, at most once a minute, the plugin, the handler and the coordinates that cost it the most, its chunk loads among them, and counts the call toward the degrade. A plugin written for Golem may set `deadline = "enforce"` under `[limits]`, and then every further call of that plugin into golem during a call past its deadline answers `DEADLINE` at once, so a handler that reads or writes the game returns within one call. `g.remaining()` lets a long loop check itself. The conversion rules never set it, since a Bukkit handler may take as long as it needs, such as the respawn handler of [18.3](#183-a-listener-registered-at-run-time-with-a-priority-from-the-configuration), which loads the chunk of a bed |
| Watchdog | A call past 1 s | `golem-watchdog` samples the current call of every thread through one atomic per thread every 50 ms and logs the plugin, the handler, the event and the time, with a dump of the Java threads through `DiagnosticCommand`, as `SIGQUIT` prints it. It names a deadlock when the server thread is in a call of a plugin and a thread of the same plugin waits for the server thread |
| Degrade | A plugin over budget in more than 10 percent of the last 1,000 ticks, or with 3 panics within a minute | Its synchronous subscriptions become observers at `MONITOR`, whose mutations of events are refused, `Plugin.state` says `DEGRADED`, and the log says so once, until an operator enables it again. Its commands still run |
| Quarantine | A call past 10 s | The plugin is written to `golem-data/quarantine.toml` with the reason, so the next start loads it but does not enable it until an operator runs `/golem plugin enable <name>` |
| The end | The tick stays stuck | Vanilla's `ServerWatchdog` ends the process at `max-tick-time`, 60 s by default, with its crash report, which now names the plugin. Golem adds no kill of its own |

A panic is caught with `catch_unwind` at every entry into plugin code, logged with the plugin and
the event, counted, and passed as if the plugin passed, as the runtime does today. Golem builds with
`panic = "unwind"`, which `17_plugin_lifecycle` checks, and a guard resets the ambient context of
Aosabi after a caught panic inside a document, which the gates of section 20 prove first. An async
call on `golem-async-<n>`, `golem-chat-<n>` or `golem-net-<n>` holds no tick, so it gets the warning
and the quarantine alone. `13_api_events` holds every layer to a case with test plugins that sleep,
loop, panic and deadlock.

### 11.5 Process plugins

A plugin that must be isolated is a native Rust binary built against `golem-client` with its feature
`process`, with the same `plugin.toml` and `kind = "process"`. `golem` starts it as a child with one
end of a socket pair on fd 3, `SOCK_SEQPACKET` through `homa::unix` on Linux and a stream pair of
the standard library with prefixed AoFrame through `homa::stream::Framed` on macOS, where
`homa::unix` does not build, so its identity is the process golem started, with no path and no
token, and restarts it with a growing pause when it ends. It speaks documents over GraphTP to the
API thread, under the principal `plugin:<name>`, whose grants of `plugin.toml` are a real boundary
here. It observes events as a remote client does, answers commands later through
`Mutation.commandSenderSendMessage`, completes through the future of
[9.8](#98-commands-and-tab-completion), and cannot answer a cancellable event, since that answer
belongs to code on the server thread. Only its documents move between the two kinds, since `Game`
exists in the process of golem alone. Process plugins use the items A6 and A7 of Aosabi once they
land and the fallbacks of [4.5](#45-what-golem-asks-of-aosabi) until then, on Linux and on macOS
alike. The notebook `19_api_process` runs one through its whole life.

## 12. Performance

### 12.1 What was measured

| What | Value | How |
| --- | --- | --- |
| `{__typename}` through `aosabi_core::Service::execute` | 8.8 to 10.2 µs with the 27 rules, 2.3 to 2.8 µs without | Spike `work-in-process-opus/spike-exec`, release with thin LTO, Aosabi `6e507dc`, 20,000 runs after 1,000 to warm up, 4 cores of an Intel Xeon at 2.10 GHz at a load average of 4 to 5, 2026-11-06 |
| `{player(name:"Steve"){health}}`, the same way | 15.9 to 23.9 µs with the rules, 6.9 to 8.3 µs without | The same spike |
| A query of seven fields, the same way | 30.4 to 30.7 µs with the rules, 13.0 to 13.2 µs without, of which the parse is 1.75 µs and the validation 10.7 µs | The same spike, rerun by a judge at a load average of 1 with 12.8 µs, 32.5 µs, 1.78 µs and 11.0 µs (`work-judge-runtime/ipo-spike.txt`) |
| The query of seven fields prepared once and run through `aosabi_execute::execute` over the parsed document | 5.4 to 6.5 µs | Spike `work-loadable-opus/spike`, release, the same machine, 2026-11-06, rerun in `work-judge-runtime/lo-spike.txt`. It runs outside the context of `Service`, so it bounds what item A2 can reach |
| One whole chat event through a plugin, the selection, the payload, the call and the change | 3.6 to 4.3 µs | The same spike |
| `hasPermission` | 138 ns for four nodes of the seed | `golem-devtool permissions --repeat`, release, [Permissions](plugins/permissions.md), 2026-11-06 |
| The wake of another thread | p50 21 to 26 µs parked, 3 to 5 µs spinning, a maximum of 1.9 to 3.7 ms | Spike `work-opus-oop/spike-rtt`, rerun by a judge at a load average of 1 (`work-judge-runtime/oopo-rtt.txt`), 2026-11-06 |

A JNI call costs from about 100 ns by the figure of the brief, which no spike of the panel measured
on the test server, so the gates of section 20 measure a stub with the out buffer there first, and
a canonical operation through the `Service`. One small document through the executor costs as much
as about 60 to 130 JNI calls, and a document of one field about 25 to 80. That ratio is the price of
the ruling that every call runs through the executor, and it is why A2 comes first among the items
of Aosabi, why a handler reads its event from the frame with no call, and why a read of many fields
of one object is one document through a record.

### 12.2 The cost model

| Operation | Path | Cost | Basis |
| --- | --- | --- | --- |
| A field of the frame of an event | A read of Rust memory | Nanoseconds | Design |
| A registry table, `MaterialInfo`, the type of an entity type | A table in Rust | About 10 ns | Design |
| A mirror or the tick snapshot | Rust, any thread | 10 to 50 ns | Design |
| `hasPermission` | The tables of `golem-permissions` | About 35 ns a node | Measured above |
| The JNI call of a live getter, `location`, `health`, `allowFlight` | One JNI call with primitives and the out buffer | 100 to 300 ns | The JNI figure of the brief and one lookup in Java, measured by `10_api_contract` |
| A method of the client, a getter or a setter of an event included | One canonical operation through the `Service` and its JNI call | About 7 to 8 µs until A2 lands, lower after | The document of one field of 12.1, measured again by `10_api_contract` |
| A record of one object | One JNI call that writes about 20 fields | 300 to 600 ns | Design, measured by `10_api_contract` |
| An event that nobody hears | `Host.WANTED` | One array load | Design |
| An event that the interest table filters out | One array load and one test in Java | Nanoseconds | Design |
| An event with subscribers | One JNI crossing and the frame | Under 1 µs plus the handlers | To be measured by `13_api_events` |
| A document of seven fields | `Service::execute` with `validate: false` | About 13 µs, about 6 µs once A2 lands | Measured above |
| A document of a remote client seen the first time | Parse, the 27 rules, execute | About 30 µs | Measured above |
| A hop from another thread | `server.execute` and the waker | At most the rest of the tick | Design |

Twenty players moving at 20 packets a second make 400 move events a second. With a subscriber that
reads two fields of the frame and makes one call of the client, that is about 400 times 9 µs, 3.6
ms a second or 0.18 ms a tick until A2 lands, where a document of seven fields per event would cost
about 400 times 13 µs, 5.2 ms a second. A converter that adds `.changed_block(true)` keeps the moves
inside one block in Java, the most common ones, and the cost falls by their share. A protection
plugin that calls the client on every event of the class D pays the executor per event, which
`13_api_events` measures before the areas of events start.

### 12.3 The rules that keep it there

- No document per event on the hot path. Handlers read the frame, which costs no call, and make one
  operation of one field per Bukkit call, which parses until A2 lands and runs a plan of Aosabi
  after.
- No string across the JNI unless the answer is text. Ids, places, registry ids and block state ids
  are numbers.
- A read of several fields of one object is one call, through a record or the masked read.
- No cache of live state across calls. What never changes during a run, registries, the properties
  of block states and the classes of entity types, is read once. What changes only through hooks
  that Golem owns, the mirrors, is kept in Rust and updated by those hooks on the server thread, and
  the rest is the tick snapshot, whose age is one tick and is documented.
- Budgets per plugin as today, with a count of calls per coordinate and timing sampled for one call
  in 64, so `/golem status` and `Plugin.budget` show which coordinate costs a plugin its time.
- The API slot and the serve point share 2 ms per tick, every remote principal has its bucket, and
  the bounds of [13.4](#134-limits) hold each operation to a cost and an answer size before it runs,
  to no chunk load and to a deadline of its own, so remote load stays within 2 ms of a tick plus at
  most the one field that runs when the deadline of an operation passes.

## 13. Remote clients

### 13.1 Transports

| Transport | Where | Who |
| --- | --- | --- |
| GraphTP on a Unix socket | `$XDG_RUNTIME_DIR/golem/api.graphtp` on Linux and `golem-data/run/api.graphtp` on macOS, the directory of mode 0700 and the socket of mode 0600, `SOCK_SEQPACKET` through `homa::unix` on Linux and a stream socket with prefixed AoFrame on macOS, through the adapter of [4.5](#45-what-golem-asks-of-aosabi) until item A6 lands, since `homa::unix` builds on Linux alone | Operators and tools on the host, as the owner ruled |
| GraphTP on a socket pair | fd 3 of a process plugin, see [11.5](#115-process-plugins) | Process plugins |
| HTTP | `127.0.0.1` only, reached from outside through the Cloudflare Tunnel of [Plan](plan.md#architecture), with `cors()` and `csrf()` of the facade over the one origin of the dashboard of the operator | Dashboards and tools that speak HTTP, the OpenID Connect login of operators |
| HTTPS | TLS 1.3 of Aosabi, only when an operator binds an address that is no loopback, which Golem refuses without TLS | Rare |

RCON stays during the switch and then goes, as the plan says.

### 13.2 Who is calling

A request carries a bearer token in the credential of GraphTP or in `Authorization`. `golem token
create --principal tool:dashboard --group tool` makes one, shows it once and keeps its SHA-256 in
`golem-store`, outside the tracked tree. On Linux a peer of the Unix socket with the uid of `golem`
and no token is the principal `local`, which the configuration maps to a group, `operator` by
default, and a peer of another user is closed with no byte, as the Unix carrier of Aosabi does for
HTTP. On macOS `aosabi-serve` reads no peer uid, so there the modes of the socket and its directory
are the boundary, which `golem` checks when it binds, and a token is required until item A8 lands.
Operators who log in through a browser use the OpenID Connect plugin of Aosabi. A process plugin is
the principal `plugin:<name>`. The identity reaches the resolvers through `use_identity` of
`aosabi-auth`, whose `Principal` carries the id and, as its roles, the groups of the principal.

### 13.3 What a principal may do

Principals are subjects of `golem-permissions` beside the players, so one file says what everyone
may do. The seed holds five groups: `metrics` reads `Query.server { tps mspt }` and
`Subscription.serverTick`, `monitor` every query of the classes `any` and `snapshot` and the
observing subscriptions, `tool` what its nodes grant, `operator` every coordinate and introspection,
and `admin` everything with the tokens and the principals. Every coordinate is a node, derived from
it mechanically: the node is `golem.api.` and the coordinate in lower case, since a segment of
`golem-permissions` holds no capital.

| Coordinate | Node |
| --- | --- |
| `Query.entity` | `golem.api.query.entity` |
| `Player.location` | `golem.api.player.location` |
| `Mutation.entityTeleport` | `golem.api.mutation.entityteleport` |
| `Subscription.playerMove` | `golem.api.subscription.playermove` |
| Introspection | `golem.introspect` |

The wildcards of `golem-permissions` apply, so a group with `golem.api.*` and
`-golem.api.mutation.*` reads everything and writes nothing, since the longer deny decides for every
mutation by rule 4 of that crate. Introspection stands outside `golem.api.`, since a wildcard
`a.*` matches below `a`, so a group that reads the API does not get introspection with it. For each
document the check collects every field of the selection, fragments included, and for a fragment on
an interface every possible type it can reach, and checks each node. The verdict is kept per
document, principal and revision of the permissions, so a prepared document is checked once.
Arguments that widen what a field may do have nodes of their own, such as
`golem.api.mutation.serverdispatchcommand.console`. A mutation of the class `server` that acts as a
player, such as `playerPerformCommand`, is resolved as that player, so the permissions of the player
apply as they would in the game. Every mutation of a remote principal goes to the audit log of
`golem-store` with the principal, the coordinate, the document id and a digest of its arguments.

A plugin compiled into golem is trusted code in the same process, which could do anything a Rust
crate can do. Its grants in `plugin.toml` are checked as a safety rail and as the record of what a
port needs, which the slice of [section 14](#14-coverage) reads, not as a security boundary, and
this page says so instead of pretending otherwise. For a process plugin the grants are the boundary.

### 13.4 Limits

Depth 12, 500 fields, lists bounded by a `limit:` argument of at most 1,000, at most 64 streams per
principal, an operation rate per principal, and a bucket of server time per principal, 2 ms per
second by default, charged with the measured time of each operation on the server thread. The
bucket is charged after the fact and the API slot checks its budget between operations, so four
bounds hold every remote operation before and while it runs, for every principal, `operator` and
the principal `local` included.

- A cost estimate. When the engine plugin of `golem-remote` prepares a document, it sums the cost of
  every field that the mapping gives it, 0 for a field of a frame, a mirror or the snapshot, 1 for a
  call of the contract, and the size of the record for a field of a record, and multiplies the cost
  below a list by the `limit:` of that list, or by the bound of the list where the field takes no
  `limit:`. A document above `remote.max-cost`, 2,000 by default, answers 400.5 Query Too Complex
  before it runs, so nested lists such as `worlds { entities { nearbyEntities } }` cannot multiply
  past it.
- An answer size. The same estimate sums the size of every leaf, a `ChunkSnapshot` at the size of
  its sections, and refuses a document whose answer would pass the answer buffer of the carrier,
  `remote.max-answer`, 1 MiB by default. The answer buffer of `graphql-over-graphtp` answers 500.1
  to an answer that passes it all the same, and the arena floor of the executor answers 503.1.
- No chunk load. A field with `loads_chunk = true` in a remote operation reads a loaded chunk alone
  and answers `CHUNK_NOT_LOADED` for any other, and `worldLoadChunk` of a remote principal adds a
  ticket and answers at once, so the chunk loads as a ticket loads it and `isChunkLoaded` tells when
  it is there. A remote operation never loads or generates a chunk on the server thread.
- A deadline per operation, `remote.operation-ms`, 2 ms by default. Past it every further field of
  the operation answers `DEADLINE`, so an operation overruns its point by at most the one field that
  runs when its deadline passes.

A bounded queue of 256 waiting operations answers `SERVER_BUSY` to an operation that waited longer
than one second. A document over a bound answers 400.5 Query Too Complex over GraphTP, the status
Aosabi defines. Text of a player never becomes part of a document, only of its variables.
`Allow::Only` of the carrier policy of `aosabi-kernel` lists the subscription fields each carrier
serves, so the HTTP carrier of the tunnel serves no event of the rate classes C and D.
`Server.budgets` shows the bucket of every principal, and the notebook `15_api_remote` holds every
line of this section to a case over both carriers.

### 13.5 Writes from outside

A remote operation runs whole within one run of the API slot or the serve point, so its reads and
its writes see one state of the game. Between two operations the game goes on, so the writes that
Bukkit code guards with a read take a precondition: `inventorySetItem(ifCurrent:)` and
`blockSetBlockData(ifType:)` answer `PRECONDITION_FAILED` when the slot or the block changed, and
`inventoryRemoveItem` and `inventoryTransferItem` answer what they could not remove or move. A
remote client can therefore not duplicate an item by reading in one operation and writing in the
next.

## 14. Coverage

[API coverage](api-coverage.md) gives the exact format of every file below, three filled examples
for each status and the cells of the notebook.

### 14.1 The files

- `api/spigot-api-1.20.1-R0.1-20230921.163938-66.json`, the inventory of the reference, which
  `golem-devtool api inventory` writes with javap from the jar that `tests/fetch.py` pins by its
  SHA-256 `d8aa356781e32896e7d78490813975cac7ca8d99126adc41c2836c56c68b9c3e`, with the sources jar
  for the names of the parameters.
- `api/bukkit/<package>/<Type>.toml`, one file per type of the inventory, 1,247 files, with one
  entry per method and per field, keyed by its line of the inventory byte for byte, so matching is a
  comparison of strings, and with the `[event]` table of every event class.
- `api/events.toml`, the overview of the 252 listenable events that `api gen` writes.
- `api/names.toml` with the exception of [8.3](#83-type-names), and `api/golem-own.toml` with every
  coordinate of the SDL that is no target of the mapping, each with its reason and its keys, such as
  `Server.tps`, `World.safeLocation`, `CommandSender.permissionLevel` and the casts
  `Query.as<Name>`.
- `api/schema.graphql`, the printed SDL of Golem's own schema, and `api/coordinates.graphql`, the
  canonical operations, while the SDL and the operations of the schema part of a port stand in the
  crate of the port under its license, as [4.2](#42-the-schema-parts-of-ports) says.
- `bridge/contract/*.toml`, the contract that `api gen` writes from the mapping.
- `api/coverage.lock`, the numbers of `main`.
- `tests/corpus/api/`, the conformance corpus: documents with variables and golden answers against
  the known world of the test server.

### 14.2 The statuses

| Status | Means | Example |
| --- | --- | --- |
| `schema` | The member is a coordinate of the SDL: a field, a root field, a type, an enum value or an argument | `Player.getAllowFlight()` is `Player.allowFlight` |
| `key` | The constant is a key of a registry of Minecraft | `Material.STONE` is `minecraft:stone` |
| `lib` | The member is a helper of `golem-client` with no game involved: a pure helper of a value, or the logger of the runtime | `Location.distance(Location)`, `Location.equals(Object)`, `Plugin.getLogger()` |
| `via` | A legacy member that a conversion rewrites to a current coordinate | `org.bukkit.material.Stairs.setInverted` through `StairsData.half` |
| `none` | A ruling that Golem does not offer it, with a reason of a closed list | `UnsafeValues.fromLegacy`, reason `internal` |
| `decide` | The rules propose a target and a person has not ruled yet | `Villager.zombify()` |

### 14.3 The three numbers

- Mapped: the member has a status other than `decide`. The target is every member.
- Present: a `schema` target exists in `api/schema.graphql` down to its arguments, a `key` exists in
  the registries report that the data generator of the server jar writes with `--reports`, which
  starts no server, a `lib` target is named in `lib_items.rs` of `golem-client`, which fails the
  build when a path is missing, a `via` target is present, and a `none` is present by its ruling. A
  coordinate exists in the SDL only when its implementation exists, since the generator writes no
  field for an entry without its Java, its Rust or its frame, so no field answers "not
  implemented".
- Proven: a `schema` target is selected by a document of the conformance corpus whose answer matched
  its golden answer on a booted test server, and the runtime recorded during that run that the
  coordinate resolved, from the resolve info of the executor and the counts of `g.run`, so a
  fragment never taken proves nothing. A `key` is proven by the registries of the booted server, a
  `lib` target by a cell of `golem-devtool api lib` with golden values, a `via` target with its
  target, and a `none` by its review.

The report prints the three numbers per status, per area, per package and over all 15,089 members
of the inventory: 1,247 types, 7,942 methods and 5,900 fields and constants, with the covered share
of all members, `none` included, beside the share of the members that are not `none`. The prototype
of the panel, `scripts/coverage_rules.py`, maps 14,834 of the members by rule alone, leaves 192
methods and 2 fields to decide, and leaves the 61 methods of the 50 anonymous classes to the reason
`anonymous`, so the first area starts with every member mapped by rule or listed for a person,
present at the size of the vertical slice and proven at zero.

### 14.4 The checks of the tool

`golem-devtool api coverage` reads the inventory, the mapping, `api/golem-own.toml`, the SDL, the
registries report, `lib_items.rs` and the record of proven coordinates, and checks that every line
of the inventory has exactly one entry and no entry names a line the inventory lacks, that every
status carries the keys its status needs, that every target is present or counted as mapped and not
present, that every listenable event has an `[event]` table, that every getter of an event class is
a field of its type, `isCancelled` included, and every setter a mutation, that every writable field
of a frame has its setter, that every cancellable event of the hook class F starts from Forge
classes that carry `Cancelable`, that every `snapshot` member names a field of a record that its
receiver type can have, that every member whose `on` narrows the receiver says with `otherwise`
what it does for the others, that every description that names a Bukkit member in backticks agrees
with the entry of that member, that every coordinate of the SDL is a target of some entry, a line of
`api/golem-own.toml` or a line of the `part.toml` of exactly one port, and that no two types share a
name.

### 14.5 The notebook and CI

The notebook `12_api_coverage` fetches the pinned jar and checks its SHA-256, writes the inventory
again and fails when it differs from the committed one, prints the SDL with the builder of Aosabi
and fails when it differs from `api/schema.graphql`, runs the checks of 14.4 and compares the
numbers with `api/coverage.lock`. CI runs it on every push and every pull request. A number below
the lock fails, and a number above it fails as well with the lines to write, so the lock always
equals the state of `main` and every rise is visible in the history, in the commit that made it. The
proven number comes from `16_api_corpus`, which boots the test server and runs in the notebooks job
of CI on every push and every pull request, and the lock of the proven number follows the same rule
in that job. The release notes of every version of golem carry the three numbers.
Existence is not equivalence: each area also has a notebook that holds its coordinates to the
semantics of CraftBukkit, such as the order of a chain with mixed priorities and `ignoreCancelled`,
the leftovers of `addItem` and `removeItem`, the default of a permission for an operator and the
nested teleport event.

`golem-devtool api slice <jar>` reads the constant pools of the classes of a plugin jar, resolves
each reference to the inventory through the type hierarchy, and prints the members the plugin needs
with their statuses and the share of them that is present and proven, so an operator reads off how
much of a plugin Golem hosts today and the work order of a port. `golem-devtool api propose` writes
the rule of the generator as the skeleton entry of a member that has none.

## 15. Licensing and the clean room

- The crates of Golem stay under the AGPL 3 only. A port keeps the license of its upstream, as
  `ports.toml` lists, and the schema part of a port is a file of the port under its license, which
  `golem-api` mounts by path. Every committed file generated from that part, its SDL, its canonical
  operations and its client, is written into the crate of the port under the same license, so no
  file of Golem's own crates takes code of an upstream, `golem-client` included. The generated copy
  under `target/aosabi/` is build output, never a file of the tree.
- Golem carries no code of Arclight or CraftBukkit and no javadoc text of spigot-api. Descriptions
  in the mapping and the schema are written in Golem's own words.
- The clean room rule for hooks: whoever reads Arclight for where a hook sits writes the class, the
  method and the point of injection into the `source` of the `[event]` table in their own words, and
  the Mixin is written from the code of vanilla in the official names and from the behaviour
  CraftBukkit shows and the corpus records. No line of Arclight or CraftBukkit is copied, translated
  or kept open beside the Mixin while it is written, and the review of every Mixin checks the table
  against the code. A Java method written by hand names in its review where its semantics were read.
- The inventory lists the signatures of a jar of the Bukkit project, which is under the GPL 3, while
  the pom of the jar names no license. `api/spigot-api-1.20.1-R0.1-20230921.163938-66.json`
  therefore takes the license of Bukkit through a `.license` file beside it and a line in NOTICE,
  and the jar itself stays in the cache of `tests/fetch.py` and out of the tree.
- Value types that Bukkit defines, such as `Location` and `Vector`, are written fresh in
  `golem-model` and `golem-client` from their documented behaviour.

## 16. The EssentialsX port and the first game interface

The first game interface that an agent builds now, the queries `players`, `location`, `respawn`,
`spawn`, `mainHand`, `heights` and `safeLocation`, the actions `teleport` and `listName`, the
handler of the Forge PermissionAPI and the level of a command sender, is the seed of this API. It
keeps its strings joined by U+001F until the first area moves it, and nothing of its meaning is
thrown away.

| Part of the first game interface | Becomes |
| --- | --- |
| `players()` | `Server.onlinePlayers`, the record `playerRecord` and the [tick snapshot](#77-the-tick-snapshot) |
| `location(uuid)` | `Entity.location` |
| `respawn(uuid)` | `Player.bedSpawnLocation`, with `loads_chunk = true` |
| `spawn(dimension)` | `World.spawnLocation` |
| `mainHand(uuid)` | `PlayerInventory.itemInMainHand` |
| `heights(dimension)` | `WorldInfo.minHeight` and `maxHeight` |
| `safeLocation(...)` | `World.safeLocation(x:, y:, z:, radius:)`, Golem's own, whose search the mod keeps as [Bridge](bridge.md) describes it and which the port of `LocationUtil.getSafeDestination` calls |
| The action `teleport` | `Mutation.entityTeleport` |
| The action `listName` | `Mutation.playerSetPlayerListName` |
| The PermissionAPI handler | Stays, answering from `golem-permissions` as `Permissible.hasPermission` does |
| The level of a sender | `CommandSender.permissionLevel`, Golem's own, and the default `OP` of a permission for a source without a profile |

The first area writes these as the first entries of the mapping and of `api/golem-own.toml`, and
`golem_bridge::Game` and `Context::game()` become a shim over `golem-client`, so `golem-essentials`,
`golem-essentials-spawn` and the notebooks `04_plugins`, `05_bridge_build` and `07_forge_boot` keep
working through the move. The scripted game of `golem-devtool` becomes the scripted contract that
answers the same questions in the notebooks. The shim and the wire of U+001F go when no port calls
them any more.

`golem-essentials` and `golem-essentials-spawn` are translations of the Java of EssentialsX 2.22.0
and EssentialsXSpawn 2.22.0 to clients of this schema, under the license of their upstream. Their
commands that need only the first game interface, such as `tpa`, `home`, `warp`, `spawn` and `back`,
go on now on the runtime of the first milestone and move to the client when the first area lands.
`golem-devtool api slice` over the jar of EssentialsX 2.22.0 gives the members the port needs, 756
methods, 211 constants and 42 events by the prototype `scripts/plugin_slice.py` of the panel on
2026-11-06, the jar of EssentialsXSpawn adds its own, and every area of the fan out builds that
slice first. The economy and the events of EssentialsX live in their schema part. The golden samples
of [Plan](plan.md#verification), which run EssentialsX on Arclight against the port on Golem, stay
the final judge, and every quirk the port fixes gets its issue, as CONTRIBUTING.md section 6 asks.

## 17. The Bukkit layer

The owner ruled on 2026-11-06 that Bukkit and Spigot plugins also run unchanged, through a Bukkit
layer in the JVM of Forge on top of this API, with the public API, the internals of CraftBukkit that
plugins reach and the classes of Minecraft that they call by their Spigot names
([Plan](plan.md#bukkit-plugins)). The layer has a page of its own, [Bukkit layer](bukkit-layer.md),
which a survey of 503 plugins orders. It is no form of a Golem plugin: every Golem plugin is native
Rust on Aosabi. This page fixes what the layer takes from the API, so the two stay one design.

- For the public API, level 1 of the layer, the layer is one more client in process. Its Java
  implementations of the interfaces of Bukkit, such as `Player` and `World`, are generated from the
  mapping files: a member of the status `schema` calls its coordinate through the native
  `Host.call(operation, depth)`, which runs the canonical operation through the executor of Aosabi
  on the thread of the caller, with the arguments and the answer in the buffers of
  [7.3](#73-frames-and-buffers). So the layer and the Rust plugins reach the same coordinate
  functions, and the mapping file is the one source for the schema, the client and the layer.
- A member of the status `lib` is the class of the reference jar itself, such as `Location` and
  `Vector`, which the layer loads as the API that plugins link, and a member of the status `via` is
  its rewrite in Java.
- A Bukkit listener is a subscriber of the same chain. The dispatcher calls the layer at the place
  of its priority with one JNI call, the layer builds the Bukkit event over the frame when the
  listener first reads it, and its setters write the frame through the mutations of the event, so
  the Rust plugins and the Bukkit plugins share one order.
- The internals of CraftBukkit that hold state of Golem, such as the permissible of
  `CraftHumanEntity` and the command map with its known commands, are coordinates of Golem's own in
  `api/golem-own.toml`, ranked by the survey, and the layer gives them the shapes the plugins
  expect. The other internals hand out objects of the game, as the paragraphs below say.
- The threads keep their rules. The layer runs the async tasks of Bukkit on its pool with the
  classes `any` and `snapshot`, and turns `NOT_ON_SERVER_THREAD` into the `IllegalStateException` of
  the `AsyncCatcher` of CraftBukkit.
- A call from the layer crosses the JNI twice, Java to Rust and Rust to Java, about 200 to 600 ns by
  the figure of the brief, plus one pass of the executor, about 7 to 8 µs until A2 lands, which the
  gates measure with the stub of [6.2](#62-what-contract-gen-writes).

The layer is a client of this schema for level 1 only. The survey of [Bukkit
layer](bukkit-layer.md), 503 plugins collected and counted by `tools/plugin-survey` on 2026-11-06,
shows that the stand-ins of level 2 mostly hand plugins live objects of Forge: on code that runs on
1.20.1, `CraftPlayer.getHandle` returns the `ServerPlayer` for 160 plugins,
`CraftItemStack.asNMSCopy` a Minecraft `ItemStack` for 130 and `CraftWorld.getHandle` the
`ServerLevel` for 83. 239 plugins call classes of Minecraft directly, level 3, which the remapper of
the layer maps onto the classes of Forge. For these plugins the layer reaches the game beside the
API and not through it: a plugin that holds a `ServerPlayer` changes the game with no coordinate, no
mutation and no frame of this API in between. So the state that Golem keeps for both ways is the
state of the game itself, or one object that both ways reach, and never a copy that only one way
updates.

- **One command registry.** Forge's Brigadier dispatcher, Golem's own commands and the Bukkit
  command map see one registry. The `knownCommands` of the Bukkit map is live, since 97 plugins read
  and write it by reflection, so a command that a plugin puts in or takes out is a command that
  players can or cannot run, and `syncCommands` resends the tree.
- **One wrapper per entity for its life.** Every entity has exactly one Bukkit wrapper from its
  spawn to its removal, and `getHandle` and the field `entity` of that wrapper return the live Forge
  entity, so a plugin that keeps a wrapper, compares wrappers or reads the field meets the entity
  that the API names.
- **Permission checks through the permissible in its field.** Every permission check of a Bukkit
  sender, also one that this API answers for it, goes through the `PermissibleBase` in the field
  `perm` of its `CraftHumanEntity`, `CraftEntity` or `ServerCommandSender`, which LuckPerms,
  GroupManager and PowerRanks replace. The answer of [9.9](#99-permissions) from the tables of
  `golem-permissions` holds while that field holds the permissible of the layer. How Golem's own
  checks ask a Bukkit permission plugin that owns the decision is open for the owner on
  [Bukkit layer](bukkit-layer.md#13-decisions-for-the-owner).
- **Persistent data on the same tags.** The `CraftPersistentDataContainer` of the layer is backed by
  the `CompoundTag` of its holder, whose raw map NBT-API reads, and the containers of
  [9.11](#911-persistent-data-containers) live on the same tags, so a value written one way reads
  the same the other way.
- **Mirrors hooked in the game.** A mirror of [12.3](#123-the-rules-that-keep-it-there) stays true
  only when its hook sits in the method of the game that changes the state, since a plugin with a
  handle calls that method without any coordinate.

## 18. Real plugins translated

The Java below is quoted from EssentialsX and EssentialsXSpawn 2.22.0 under the GPL 3 or later, the
license their ports keep. Each Rust line names the coordinate it reaches where that is not plain.

### 18.1 A hot event with a priority, `ignoreCancelled` and an event mutation

`EssentialsPlayerListener.onPlayerMove` holds frozen players in place and tracks the activity of the
others:

```java
@EventHandler(priority = EventPriority.HIGH, ignoreCancelled = true)
public void onPlayerMove(final PlayerMoveEvent event) {
    if (event.getFrom().getBlockX() == event.getTo().getBlockX()
            && event.getFrom().getBlockZ() == event.getTo().getBlockZ()
            && event.getFrom().getBlockY() == event.getTo().getBlockY()) {
        return;
    }
    final User user = ess.getUser(event.getPlayer());
    if (user.isFreeze()) {
        final Location from = event.getFrom();
        final Location to = event.getTo().clone();
        to.setX(from.getX());
        to.setY(from.getY());
        to.setZ(from.getZ());
        try {
            event.setTo(LocationUtil.getSafeDestination(ess, to));
        } catch (final Exception ex) {
            event.setTo(to);
        }
        return;
    }
    // ... the AFK branch with event.getPlayer().getAllowFlight() ...
}
```

```rust
// enable
enable.subscribe(
    Subscription::player_move().priority(EventPriority::High).ignore_cancelled(true),
    {
        let listener = Arc::clone(&self.player_listener);
        move |g, event| listener.on_player_move(g, event)
    },
);

fn on_player_move(&self, g: &Game<'_>, event: &mut PlayerMoveEvent<'_>) -> Result<()> {
    let (from, to) = (event.from(), event.to());       // PlayerMoveEvent.from and .to, the frame
    if from.block_x() == to.block_x()
        && from.block_z() == to.block_z()
        && from.block_y() == to.block_y()
    {
        return Ok(());
    }
    let user = self.ess.user(g, event.player())?;      // PlayerEvent.player, the frame
    if user.is_freeze() {
        let mut to = to;                                // Location is Copy, clone() goes away
        to.set_x(from.x());
        to.set_y(from.y());
        to.set_z(from.z());
        let safe = location_util::safe_destination(g, &self.ess, to).unwrap_or(to);
        return event.set_to(safe);                      // Mutation.playerMoveEventSetTo
    }
    // ... the AFK branch with event.player().allow_flight(g)?, Player.allowFlight, one JNI call
    Ok(())
}
```

A move inside one block, by far the most common, returns before any JNI call. A converter that knows
this adds `.changed_block(true)` to the subscription, and the interest table of
[10.6](#106-what-an-event-costs) then keeps those moves in Java. The `try` around
`getSafeDestination` becomes `unwrap_or`, since the Rust helper answers a `Result`.

### 18.2 An async event with a cancel and its recipients

`EssentialsPlayerListener.onPlayerChat` mutes players and applies the ignore lists:

```java
@EventHandler(priority = EventPriority.LOWEST)
public void onPlayerChat(final AsyncPlayerChatEvent event) {
    final User user = ess.getUser(event.getPlayer());
    if (user.isMuted()) {
        event.setCancelled(true);
        // ... user.sendTl("voiceSilenced...") by the reason and the time left ...
    }
    final Iterator<Player> it = event.getRecipients().iterator();
    while (it.hasNext()) {
        final User u = ess.getUser(it.next());
        if (u.isIgnoredPlayer(user)) {
            it.remove();
        }
    }
    user.updateActivityOnChat(true);
    user.setDisplayNick();
}
```

```rust
enable.subscribe_async(Subscription::async_player_chat().priority(EventPriority::Lowest), {
    let listener = Arc::clone(&self.player_listener);
    move |g, event| listener.on_player_chat(g, event)
});

fn on_player_chat(&self, g: &AsyncGame, event: &mut AsyncPlayerChatEvent<'_>) -> Result<()> {
    let user = self.ess.user_any(event.player());      // the user table of the port, any thread
    if user.is_muted() {
        event.set_cancelled(true)?;                     // Mutation.cancellableSetCancelled
        user.send_silenced_notice(g)?;                  // CommandSender.sendMessage, class any
    }
    event
        .recipients_mut()                               // AsyncPlayerChatEvent.recipients
        .retain(|recipient| !self.ess.user_any(*recipient).is_ignored_player(&user));
    user.update_activity_on_chat(true);
    user.set_display_nick(g)?;                          // Mutation.playerSetDisplayName, queued
    Ok(())
}
```

The chain runs on `golem-chat-<n>`, off the server thread as in Bukkit, and no thread of the game
waits for it. `playerSetDisplayName` is of the class `server` with a unit answer, so from the chat
thread it is queued, and the mirror of the display name changes when the server thread applies it.
A handler that needs a live read here may wait for it with `g.on_server`, as a Bukkit chat handler
may wait for `callSyncMethod`. The recipients are a list of the frame, which a Mixin at the
broadcast of the chat applies.

### 18.3 A listener registered at run time with a priority from the configuration

`EssentialsSpawn.onEnable` and `EssentialsSpawnPlayerListener.onPlayerRespawn`:

```java
final EventPriority respawnPriority = ess.getSettings().getRespawnPriority();
if (respawnPriority != null) {
    pluginManager.registerEvent(PlayerRespawnEvent.class, playerListener, respawnPriority,
        (ll, event) -> ((EssentialsSpawnPlayerListener) ll)
            .onPlayerRespawn((PlayerRespawnEvent) event), this);
}

void onPlayerRespawn(final PlayerRespawnEvent event) {
    final User user = ess.getUser(event.getPlayer());
    if (user.isJailed() && user.getJail() != null && !user.getJail().isEmpty()) {
        return;
    }
    if (ess.getSettings().getRespawnAtHome()) {
        Location respawnLocation = null;
        if (ess.getSettings().isRespawnAtBed()
                && (!event.isAnchorSpawn() || ess.getSettings().isRespawnAtAnchor())) {
            respawnLocation = user.getBase().getBedSpawnLocation();
        }
        final Location home = respawnLocation != null
                ? respawnLocation : user.getHome(user.getLocation());
        if (home != null) {
            event.setRespawnLocation(home);
            return;
        }
    }
    // ... random respawn, then the spawn of the group ...
}
```

```rust
// enable of golem-essentials-spawn
if let Some(priority) = self.ess.settings().respawn_priority() {
    let listener = Arc::clone(&self.listener);
    enable.subscribe(Subscription::player_respawn().priority(priority), move |g, event| {
        listener.on_player_respawn(g, event)
    });
}

fn on_player_respawn(&self, g: &Game<'_>, event: &mut PlayerRespawnEvent<'_>) -> Result<()> {
    let user = self.ess.user(g, event.player())?;
    if user.is_jailed() && user.jail().is_some_and(|jail| !jail.is_empty()) {
        return Ok(());
    }
    let settings = self.ess.settings();
    if settings.respawn_at_home() {
        let mut respawn = None;
        if settings.is_respawn_at_bed()
            && (!event.is_anchor_spawn() || settings.is_respawn_at_anchor())  // the frame
        {
            respawn = user.base().bed_spawn_location(g)?;  // Player.bedSpawnLocation, loads a chunk
        }
        let home = match respawn {
            Some(bed) => Some(bed),
            None => user.home(g, user.location(g)?)?,     // Entity.location
        };
        if let Some(home) = home {
            return event.set_respawn_location(home);  // Mutation.playerRespawnEventSetRespawnLocation
        }
    }
    // ... random respawn, then the spawn of the group ...
    Ok(())
}
```

`PluginManager.registerEvent(Class, Listener, EventPriority, EventExecutor, Plugin)` maps to
`Enable::subscribe` with the subscription field, its priority and the closure, the plugin being the
one that enables. The respawn hook is the Mixin in `PlayerList.respawn` of
[10.8](#108-where-the-events-come-from), since Forge's event comes too late for
`setRespawnLocation`. When the chunk of the bed is not loaded, `bed_spawn_location` loads it and the
handler runs past the deadline of 5 ms for a handler. It runs to its end, as in Bukkit, and
[11.4](#114-containment) names it in the log, so the converted handler sets the respawn location as
its Java does, with no change of `plugin.toml`.

### 18.4 A command that fires a Bukkit event

`Commandheal.healPlayer` asks the other plugins whether the heal may happen:

```java
final double amount = player.getMaxHealth() - player.getHealth();
final EntityRegainHealthEvent erhe =
        new EntityRegainHealthEvent(player, amount, RegainReason.CUSTOM);
ess.getServer().getPluginManager().callEvent(erhe);
if (erhe.isCancelled()) {
    throw new QuietAbortException();
}
double newAmount = player.getHealth() + erhe.getAmount();
if (newAmount > player.getMaxHealth()) {
    newAmount = player.getMaxHealth();
}
player.setHealth(newAmount);
player.setFoodLevel(20);
player.setFireTicks(0);
player.setRemainingAir(player.getMaximumAir());
```

```rust
let amount = player.max_health(g)? - player.health(g)?;     // Damageable.maxHealth, .health
let erhe = g.events().call_entity_regain_health(&player, amount, RegainReason::Custom)?;
if erhe.is_cancelled() {                                     // Mutation.callEntityRegainHealth
    return Err(QuietAbort.into());
}
let new_amount = (player.health(g)? + erhe.amount()).min(player.max_health(g)?);
player.set_health(g, new_amount)?;                           // Mutation.damageableSetHealth
player.set_food_level(g, 20)?;                               // Mutation.humanEntitySetFoodLevel
player.set_fire_ticks(g, 0)?;                                // Mutation.entitySetFireTicks
player.set_remaining_air(g, player.maximum_air(g)?)?;        // Mutation.livingEntitySetRemainingAir
```

The chain of `EntityRegainHealthEvent` runs synchronously inside the command, the Rust plugins and
the Bukkit layer in one order, and the answer carries the amount a later handler changed.

### 18.5 The same read from a tool

A dashboard reads what `/getpos` shows with a document over GraphTP, under a principal that holds
`golem.api.query.entity`, `golem.api.entity.location`, `golem.api.location.*` and
`golem.api.world.name`:

```graphql
query GetPos($id: ID!) {
  entity(id: $id) {
    location { world { name } blockX blockY blockZ yaw pitch }
  }
}
```

A plugin that keeps the same text in its `ops/getpos.graphql` gets the function
`ops::get_pos(g, GetPosVariables { id })` with typed answers, and runs it through the executor once
for the six fields.

## 19. What this design does not do, and its risks

- No library loaded at run time, no hot reload of plugin code, no scripting language. Plugins are
  native Rust on Aosabi.
- No socket, channel or thread between a plugin in the process and the API. A call is a call on the
  thread of the caller.
- No compiler of GraphQL documents and no plan of an operation in Golem. Every call runs through the
  executor of Aosabi, and the plans are item A2 of Aosabi.
- No blocking of the server thread on another thread, the network, the disk of a plugin or a remote
  client, and no deadline offered as a way to do it anyway. Remote clients and process plugins never
  answer a cancellable event.
- No thread of the game waiting for a plugin outside a native call of that thread. The chat chain
  runs on a thread of Golem.
- No action queue for a mutation made on the server thread. It runs at once, as in Bukkit.
- No live mirror of an `ItemStack` that writes through into an inventory, as `CraftItemStack` does
  for some stacks. A converted plugin writes the stack back.
- No Java global reference per entity, block or item, and no Java object held by Rust beyond the
  contract implementations and the buffers.
- No implicit cache of live state across calls, and no field in the schema that answers "not
  implemented".
- No custom directive, no input struct, and no custom scalar until Aosabi offers them.
- No rules or verdict caches of constant answers, no update mutation of every setter and no other
  coordinate of Golem's own that no Bukkit member and no part of the first game interface needs,
  until `13_api_events` measures a need. In process a closure costs nanoseconds, and the interest
  tables and subject sets keep hot events in Java.
- No legacy material types. `org.bukkit.material` maps through `via`.
- No code of Arclight or CraftBukkit and no javadoc text of spigot-api.
- No kill of a plugin by Golem. The containment names, refuses, degrades and quarantines, and
  vanilla's watchdog ends the process.

| Risk | Why it matters | What answers it |
| --- | --- | --- |
| The plain path of Aosabi at full size | About 13,000 entity fields and 171 union enums have never gone through the builder | Gate 0 builds a generated skeleton at full size first, with the fallback of A1 |
| The embedding of `golem-api` | Golem cannot link a glued plain crate into its own binary today | Item A1, with the fallback of linking the generated copy |
| The executor on every call | Every Bukkit call of a converted plugin pays one pass of the executor, about 7 to 8 µs until A2 lands, 25 to 80 times its JNI call | A2 first among the items of Aosabi, the frame and the records for hot reads, gate 0 and `10_api_contract` measuring a call of the client on the test server, and decision 1 of section 22 |
| Nesting and panics through Aosabi | A document inside a handler nests an agent of Aosabi, and an unwind may leave its ambient context behind | Gate 0 proves both, and a guard resets the context after a caught panic |
| Mixins in a heavily patched game | The mods inject at 2,569 points of the server code, and the hook class of every event is a judgement from two lists | Few injections, at heads, returns and single calls, gated by the mask, an `[event]` table per event before its area starts, and `18_api_hooks` on the full pack before each area merges |
| The chat future | The Mixin at `getChatDecorator` sits where chat mods may hook as well | `18_api_hooks` sends chat through every chat mod of the pack, and the open item records each mod that decorates chat |
| The data that Arclight wrote | Persistent data and world names live in the forms of CraftBukkit | The Mixins of 9.11 and the names of 8.1 land before the switch, checked on a copy of the live world |
| Re-entrancy and threads | The global lock goes, nested dispatch and hops come | The frame stack, the depth cap, `PluginLock`, the watchdog, and `13_api_events` with nested and cross thread cases |
| macOS on the live server | The poll reactor of Aosabi, `homa::unix` that builds on Linux alone, no `SOCK_SEQPACKET` on Unix sockets, no peer uid | The stream adapter of 4.5 over `homa::stream::Framed` and the standard library, the hot wake source, the modes of the socket and a token, gate 0 on the Mac mini, and items A6 and A8 |
| Remote load on the server thread | One operation runs to its end once it starts, and a read of Bukkit may load a chunk | The cost estimate, the answer size, no chunk load and the deadline per operation of 13.4, for every principal |
| The size of the work | 168 to 220 weeks of one engineer is the honest scale of every Bukkit plugin | The EssentialsX slice first and the three numbers as the measure of progress |
| The license of the reference | The inventory lists the signatures of a GPL jar | A `.license` file and a line in NOTICE, which the owner confirms |

## 20. Work breakdown

The areas stand in dependency order, in weeks of one engineer, as estimates that nobody measured.
The events are planned at half a day for an event that Forge covers, one to one and a half days for
one that needs Forge and a Mixin, and one and a half to two days for one that needs a Mixin alone,
against the 265 mods of the pack. Area 0 decides before code lands, area 1 is the trunk that every
later area builds on, and from area 3 on the areas run in parallel, each with its own files. Every
area includes its notebook, its wiki page, its mapping entries and its coverage rows, as
CONTRIBUTING.md asks. [API coverage](api-coverage.md) section 2.1 names the packages of the
inventory that each area owns.

One prerequisite lies outside this breakdown, and the area that needs it says so: the usage audit
of the Mac mini of [Plan](plan.md#before-the-port), which needs the live server and reads off it
the world names of 8.1. The full pack of 265 mods boots under `golem run`, as `08_modpack_boot`
shows, and the notebooks job of CI runs every notebook, the ones that boot Forge included.

| # | Area | Delivers | Proven by | Needs | Weeks |
| --- | --- | --- | --- | --- | ---: |
| 0 | Gates | The builder and the boot of Aosabi on a generated skeleton of the whole entity tree, with the time of the build, the size of the glue, the growth of the binary and the boot of the schema. A stub with the out buffer and a frame on the test server, which measures a JNI call, and one canonical operation through the `Service`, which measures a call of the client. A document nested in a handler and a panic unwound through the executor. The hand over of `on_execute` with the wake source on Linux and on macOS, and `graphql-over-graphtp` served on the Mac mini through the stream adapter of 4.5 with the JVM running. The mount of a schema part through the generated copy. One Mixin of Golem booted beside the 265 mods | The first cells of `10_api_contract`, `11_api_schema` and `18_api_hooks` | Nothing | 3 |
| 1 | The trunk | Everything that [section 21](#21-what-the-first-area-builds) lists: the mapping and its generators, the generated contract, `golem-game`, `golem-model`, the client, the coverage tool with its lock, the frames and a first dispatcher, the tick snapshot with the `tick` native that reads its records in one JNI call, a first runtime, the vertical slice, the first game interface on the contract | `10_api_contract`, `11_api_schema`, `12_api_coverage`, the first cells of `13_api_events`, `14_api_smoke` and `16_api_corpus`, and `04_plugins`, `05_bridge_build` and `07_forge_boot` unchanged | 0 | 17 to 22 |
| 2 | The items of Aosabi | A1 to A8 of [4.5](#45-what-golem-asks-of-aosabi), in the repository of Aosabi, A1 and A2 first, and A6 with a build of the stream form of `homa::unix` for macOS and a kqueue reactor | The notebooks of Aosabi | 0 | 7 to 12, in Aosabi |
| 3 | Remote clients | `golem-remote`: GraphTP on the Unix socket and on the socket pair, with the stream adapter of 4.5 on macOS, HTTP on loopback, tokens with `golem token create`, groups, the OpenID Connect login of 13.2, the nodes of the coordinates, the limits and buckets, the cost estimate, the answer size, the refusal of chunk loads and the deadline per operation of 13.4, `SERVER_BUSY`, the audit log, the preconditions, the remote streams, the carrier policies, the API slot and the serve point, the settings of `config/golem.toml` under `remote` | `15_api_remote` | 1. On macOS the stream adapter stands in for A6 and a token for A8 until they land | 5 to 7 |
| 4 | The event core | The whole dispatcher: chains per handler list with class filters, priorities, `ignoreCancelled`, `MONITOR` read only, nesting to 16, the chains of `golem-chat` and `golem-net`, the wanted mask, filters, interest tables, subject sets, rate classes, `call<Event>`, the events of ports, the copies for remote subscribers | `13_api_events` | 1 | 3 to 4 |
| 5 | The plugin runtime | The lifecycle with the native `serverAboutToStart` and the steps of 11.2, `plugin.toml`, the order, the ledger, a disable that never waits on the server thread, `Game` and `AsyncGame`, the hops, `PluginLock` and its lint, budgets per coordinate, the deadline, the watchdog, the degrade mode, the quarantine, the scheduler, services, the configuration library, metadata, conversations, the logger, `/golem status`, `/golem plugin enable` and `disable`, `config/golem.toml` with the settings of 11.4, the shim of the first game interface | `04_plugins` extended and `17_plugin_lifecycle` | 1 | 6 to 8 |
| 6 | Entities | The 171 interfaces with their 1,147 methods, `World.entitiesOf`, spawning, attributes, potion effects, the types of modded entities, the records | `20_api_entities` and the corpus | 1, 4 | 14 to 18 |
| 7 | Worlds and blocks | Worlds, chunks, blocks, block data, block states, ray traces, explosions, sounds, particles, the world border, game rules, the world names and `uid.dat` of 8.1 | `21_api_worlds` and the corpus, the names on a copy of the live world | 1, 4, and the usage audit, a prerequisite, for the names | 12 to 15 |
| 8 | Items and data | Materials and the registries, `golem-nbt`, item meta, enchantments, persistent data with the Mixins of [9.11](#911-persistent-data-containers) | `22_api_items` and the corpus, with the keys on a copy of the live world | 1 | 8 to 10 |
| 9 | Inventories | Inventories, views, custom inventories and menus, merchants, the preconditions of writes | `23_api_inventories` and the corpus | 8 | 6 to 8 |
| 10 | Commands and permissions | Commands with the order of vanilla names, tab completion with the futures of process plugins, the senders that are no entity of 8.1, permissions with attachments and defaults, the command tree sent again on every change of 9.9, the PermissionAPI handler, help | `24_api_commands` and `06_permissions` extended | 5 | 5 to 6 |
| 11 | Events of the classes F, P and G | About 115 events with their hooks, frames and setters, the chat future and the chains of `golem-net` of 7.6 among them | `18_api_hooks`, `13_api_events` and the corpus | 4, 6 to 9 | 17 to 22 |
| 12 | Events of the class M | About 137 events with their Mixins | The same | 4, 6 to 9 | 41 to 55 |
| 13 | Display | Scoreboards per player, boss bars, chat components, plugin messaging, titles, the player list | `25_api_display` | 6 | 7 to 9 |
| 14 | Content | Maps and their renderers, recipes added at once with the table that adds them again after a reload, loot tables, advancements, structures, bans, profiles, statistics | `26_api_content` | 7, 8 | 8 to 10 |
| 15 | World creation | Worlds made at run time, generators, biome providers | `27_api_worldgen` | 7 | 8 to 12 |
| 16 | The seam of the Bukkit layer | `Host.call`, the subscribers of the layer in the chain, the error of the `AsyncCatcher`, the generated Java of the layer from the mapping, for the page of the layer to build on | `28_api_layer` | 1, 4 | 2 to 3 |
| 17 | Process plugins | The socket pair on fd 3, spawn and restart, observation, commands answered later, completion through the future | `19_api_process` | 3 | 3 to 4 |
| 18 | The conversion kit | The guide of the rules of [8.4](#84-member-names), a cookbook of Bukkit idioms, `api slice` and `api propose`, the corpus grown with every area | `16_api_corpus` | 1 | 3 to 4 |

The sum is 168 to 220 weeks of Golem work and 7 to 12 weeks in Aosabi, about 19 to 25 months with
the two engineers of [Plan](plan.md#work-plan), and less once more engineers take areas of the fan
out. That is far more than the 28 to 40 weeks the plan gives Golem without the native code, because
every member of Bukkit gets its coordinate, its Java and its proof, and about 250 events get hooks
in a game that 265 mods patch.

The port of EssentialsX does not wait for all of it. After the areas 0, 1, 3, 4 and 5, about 34 to
44 weeks, every area of the fan out builds the slice of EssentialsX first, which this page expects
at 12 to 16 weeks, so the port runs against a real API after about 46 to 60 weeks of API work, about
five to seven months with two engineers, while its commands on the first game interface go on from
today. Area 3 needs no item of Aosabi on the Mac mini, since the stream adapter of 4.5 and a token
stand in for A6 and A8 there.

## 21. What the first area builds

The trunk builds every shared part once, so that each later area adds mapping entries, expressions,
hand written functions, Mixins and corpus documents in files of its own and never waits for another
area.

1. The mapping: the format of [API coverage](api-coverage.md) and its reader, `api inventory` from
   the pinned jar, the 1,247 skeleton files that the rules write from the inventory with the members
   to decide listed, `api/names.toml` with the rules of [8.3](#83-type-names) and the refusal of any
   clash, `api/golem-own.toml` with the first game interface.
2. The contract: `api gen` for `bridge/contract/*.toml`, `contract gen` with its six outputs,
   `golem-game` with the ids, the server token, the safe views of the frame stack and of the out
   slots per depth, `Host.bindBuffers`, the hash and the `GetMethodID` check at `connect`, the
   waker and the hop queue, the natives without the global lock, and the `tick` native that writes
   the tick snapshot of [7.7](#77-the-tick-snapshot) from one JNI call.
3. The schema: `golem-model` with the generated coordinate functions, `api gen` for object types,
   union enums with the rules of
   [4.3](#43-the-limits-of-the-plain-rust-path-and-the-rules-that-meet-them), mutations,
   subscriptions and the Java of the `read` and `write` expressions, the index of `golem-api`, the
   embedding through A1 or its fallback, `api sdl` and the committed `api/schema.graphql`, with
   the split of the parts of ports of [4.2](#42-the-schema-parts-of-ports).
4. The client: `api client` with the traits, the casts, the builders of subscriptions, the
   canonical operations validated at build time and prepared at `enable`, `g.run` through the
   `Service` with its count and its grant check, `api ops` with prepared documents, and `api lib`
   with `lib_items.rs`.
5. The proof: `api coverage` with every check of [14.4](#144-the-checks-of-the-tool) and
   `api/coverage.lock` at the starting numbers, `api smoke`, the runner of the corpus and its first
   documents, and the CI jobs of `12_api_coverage` and `16_api_corpus`, the second in the
   notebooks job of CI.
6. Events: the `[event]` tables, the generator of frames, and a dispatcher that runs one chain by
   priority with `ignoreCancelled` and a read only `MONITOR`.
7. A first runtime: `Plugin::enable` with `subscribe` and `command`, and the shim of the first game
   interface over the client.
8. The vertical slice, one of each kind through every layer, which every later area copies:
   `Entity.location` and `Player.allowFlight` with its setter from Java expressions,
   `Damageable.setHealth` from a hand written function, `World.name`, `Block.type`, `ItemStack` as a
   value through a first `golem-nbt`, `Inventory.item`, the event `playerMove` with `setTo` and the
   cancel, one command with tab completion, one `lib` helper, one `key`, one `via` and one `none`,
   each with its corpus document and its cell of `14_api_smoke`, and the first game interface moved
   onto the contract.

Each later area owns the mapping files of its packages, which the key `area` of a file names and the
table of [API coverage](api-coverage.md) section 2.1 assigns, the `[event]` tables in them, its
modules of hand written coordinate functions, its Java classes and Mixins, the mirrors of
[12.3](#123-the-rules-that-keep-it-there) that its hooks update, its corpus documents and its
notebook. Generated files are written by the generators and never merged by hand, and
`api/coverage.lock` is the one shared file, whose conflicts the tool resolves by running again. The
numbers per area in the lock show each area its progress.

## 22. Decisions for the owner to confirm

This page takes these decisions under the ruling that an agent answers its own blocking questions,
for the owner to confirm or overrule on [Decisions](decisions.md).

1. The client of section 5 is the schema spelled as Rust, generated from the SDL alone, and every
   method runs its canonical operation through the executor of Aosabi, as ruled. Two points go to
   the owner. First, a subscriber reads the frame of its event as the value its subscription
   delivers, with no operation, which this page reads as within the ruling, since the ruling names
   the calls into the API. Second, should the gates show that one pass of the executor per call,
   about 7 to 8 µs before A2, misses the budget of hot handlers, whether a method may call the
   coordinate function that its resolver calls without the executor. This page does not do that
   without such a ruling.
2. The chat chain runs on a thread of Golem composed onto the chat future of Forge, after every
   listener of `ServerChatEvent`, so that no worker of world generation waits for a plugin.
3. `inventory.json` enters the repository under the license of spigot-api with a `.license` file and
   a line in NOTICE.
4. A permission attachment of a plugin ranks above a player's stored node, as a transient node of
   LuckPerms does.
5. The defaults of 11.4: a budget of 1 ms, deadlines of 5 ms and 50 ms that name a slow call and
   stop its further calls only for a plugin that sets `deadline = "enforce"`, a warning at 1 s, a
   degrade at 10 percent of the last 1,000 ticks over budget or three panics a minute, a quarantine
   at 10 s, and the remote bounds of 13.4: a cost of 2,000, an answer of 1 MiB, no chunk load and
   2 ms per operation.
6. A plugin command named like a vanilla command takes the plain name, as CraftBukkit orders them,
   with Spigot's `commands.replace-commands` as the exception list.
7. The crates `golem-client` and `golem-plugin`, which every port links, stay AGPL-3.0-only like all
   of Golem's own code, and section 13 of the GPL 3 and the AGPL 3 covers the combination, as
   [Licensing](../Documentation/licensing.md) section 6 reads it for the ports today.

Related: [Plan](plan.md), [Bridge](bridge.md), [Plugins](plugins/index.md),
[Permissions](plugins/permissions.md), [API coverage](api-coverage.md), [Decisions](decisions.md),
[Index](index.md).
