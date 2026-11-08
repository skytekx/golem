---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Spec
title: API coverage
description: The exact format of the mapping that holds the Golem API to every member of spigot-api 1.20.1, one TOML file per type keyed by the lines of inventory.json, with the keys that generate the schema, the contract and the client, the event tables, the statuses with three filled examples of each, the other files of the check, the three numbers, the notebook that computes them and how CI holds them so that they only rise.
tags: [api, coverage, bukkit, mapping, graphql, ci]
timestamp: 2026-11-08
---

# API coverage

The Golem API covers an equivalent of the whole Bukkit API of `spigot-api`
1.20.1-R0.1-20230921.163938-66, as the owner ruled on 2026-11-06, and this page gives the exact
format of the files that hold it to that reference. The mapping files are at once the input of the
generators, which write the schema, the contract with the bridge mod and the plugin client from
them, and the input of the coverage check, so a coordinate cannot exist without its mapping entry
and the reverse. [Plugin API](api.md#14-coverage) says why the check is shaped this way. Nothing of
it is built yet. Every number on this page names the script, the day and the machine that counted
it.

## 1. The files

| Path | Written by | Holds |
| --- | --- | --- |
| `api/spigot-api-1.20.1-R0.1-20230921.163938-66.json` | `golem-devtool api inventory` | The inventory of the reference: every public type of the jar with its kind, its modifiers, its `extends`, its `implements`, its methods and its fields as `javap -public` prints them, 1,247 types, 7,942 methods and 5,900 fields and constants |
| `api/bukkit/<package>/<Type>.toml` | `golem-devtool api propose` once, then people | The mapping of one type of the inventory and of each of its members, 1,247 files, such as `api/bukkit/org.bukkit.entity/LivingEntity.toml` and `api/bukkit/org.bukkit/ChatColor$1.toml` |
| `api/events.toml` | `golem-devtool api gen` | The overview of the 252 listenable events, one row per `[event]` table of the mapping |
| `api/names.toml` | People | The names of GraphQL types that the rules of [Plugin API](api.md#83-type-names) do not give |
| `api/golem-own.toml` | People | Every coordinate of the SDL that is the target of no entry, with its reason and the same keys a member takes, and the fields of the tick snapshot |
| `api/schema.graphql` | `golem-devtool api sdl`, the builder of Aosabi over `golem-api` | The printed SDL of Golem's own schema, without the types and root fields of the parts of ports |
| `api/coordinates.graphql` | `golem-devtool api client` | The canonical operation of every coordinate of Golem's own schema |
| `crates/<port>/schema/part.toml` | People, under the license of the port | The types and the root fields that the schema part of the port declares |
| `crates/<port>/schema/schema.graphql`, `crates/<port>/src/client.rs` | `golem-devtool api sdl` and `api client`, under the license of the port | The printed SDL of the part, and its client methods and canonical operations, see [Plugin API](api.md#42-the-schema-parts-of-ports) |
| `bridge/contract/<area>.toml` | `golem-devtool api gen` | The calls, records and frames of the JNI that the mapping implies |
| `crates/golem-client/src/lib_items.rs` | `golem-devtool api lib` | The path of every `lib` target, named in a function that is never called, so a missing helper fails the build |
| `api/coverage.lock` | `golem-devtool api coverage --lock` | The three numbers of `main` |
| `tests/corpus/api/<area>/<name>.graphql` with `<name>.json` | People | The conformance corpus: a document, its variables and its golden answer against the known world of the test server |
| `target/api/coverage.json`, `target/api/proven.json` | The tool and `26_api_corpus` | Every member with its state, and the coordinates a passing corpus run resolved. Never committed |

Generated files carry a first line that names the command that wrote them, and CI runs the command
again and fails on a difference. The inventory is checked in with a `.license` file beside it that
gives it the license of Bukkit and a line in NOTICE, since it lists the signatures of a jar of the
Bukkit project. The jar itself stays in the cache of `tests/fetch.py`, pinned by its SHA-256.

| Jar | SHA-256 | SHA-1 of the Spigot repository |
| --- | --- | --- |
| `spigot-api-1.20.1-R0.1-20230921.163938-66.jar` | `d8aa356781e32896e7d78490813975cac7ca8d99126adc41c2836c56c68b9c3e` | `23c0dae3633c3dd31d4503969fa7f9f3d36840fb` |
| `spigot-api-1.20.1-R0.1-20230921.163938-66-sources.jar` | `292793da18cba4214c2254748c88ee7463ecf9aa7450a8ba64e39c9d4a6610bf` | `db707b55ae958245adecc7f15b611e1a571e7fe4` |

The sources jar gives the names of the parameters, which javap does not print. Golem reads it for
the names alone and copies no javadoc text of it.

## 2. The mapping file

A mapping file is TOML with the SPDX lines of every TOML file of Golem. Its path is the package of
the type as a directory and the simple binary name of the type as the file, so the inner class
`org.bukkit.entity.Player$Spigot` is `api/bukkit/org.bukkit.entity/Player$Spigot.toml`. Its top
level describes the type, its table `members` holds one table per method and per field, whose key is
the line of the inventory byte for byte, and an event class adds the table `event`. The check is a
comparison of strings, and TOML itself refuses a line that appears twice. No type of the inventory
holds the same line twice, by a count over `inventory.json` on 2026-11-06, and a bridge method, such
as `public java.lang.Object clone()` beside `public org.bukkit.Location clone()`, is a line of its
own. A key and the text of a Java expression are data and may be wider than 100 columns.

### 2.1 The keys of the type

| Key | Required | Value | Checked against |
| --- | --- | --- | --- |
| `type` | Always | The binary name of the type, as the `name` of the inventory | The inventory and the path of the file |
| `kind` | Always | `class` or `interface`, as the inventory prints it. An enum is a `class` and an annotation an `interface`, as javap prints them | The inventory |
| `area` | Always | The slug of the part of [Plugin API](api.md#20-the-parts-of-the-work) that owns the file, from the table below | The table below, and for an event class the hook class of its `[event]` table |
| `status` | Always | The status of the type itself, see section 3 | Section 3 |
| `target` | With `schema` and `via` | The GraphQL type, or for `via` the current type a conversion rewrites to | The SDL |
| `registry` | With `key`, and with `schema` for a type whose constants are keys | The ids of the registries of Minecraft its constants are keys of, such as `["minecraft:block", "minecraft:item"]` | The registries report |
| `on` | Where its members carry Java | The class of Minecraft that `it` names in the Java of its members, such as `net.minecraft.world.entity.LivingEntity` | javac |
| `record` | No | The name of the record of the contract that reads the fields of its members marked `record = true` in one call, such as `livingRecord` | The contract |
| `model` | Where its members name functions written by hand | The default module of `golem-model` for its `rust` paths | The build of `golem-model` |
| `reason` | With `none` and `decide` | See section 4 | The closed list |
| `why` | With `none` | The evidence of the ruling | Not empty |
| `doc` | With `schema` | The description in Golem's own words | Not empty, no javadoc text |

The tool reads the modifiers, `extends` and `implements` of a type from the inventory itself, so the
mapping file does not repeat them.

The slugs name the parts of the API that build members, with the packages of the inventory whose
files each owns. The 108 types of the package `org.bukkit` itself go by their subject, as
`api propose` writes them and a person reviews, such as `World` and `Location` to `worlds`,
`Material` to `items`, `GameMode` and `OfflinePlayer` to `entities`, `ChatColor` to `display`, and
`Server`, `Bukkit` and `UnsafeValues` to `runtime`. A member that another part builds names that
part with the key `area` of the member, such as the members of the vertical slice with `base`.

| Slug | Part of the API | Packages |
| --- | --- | --- |
| `base` | The shared base | No file. The members of the vertical slice name it |
| `event-core` | The event core | `org.bukkit.event`, and the abstract event classes of its subpackages, such as `PlayerEvent` |
| `runtime` | The plugin runtime | `org.bukkit.plugin`, `org.bukkit.plugin.java`, `org.bukkit.scheduler`, `org.bukkit.configuration` and its subpackages, `org.bukkit.conversations`, `org.bukkit.metadata`, `org.bukkit.util.io`, `org.spigotmc` |
| `entities` | Entities | `org.bukkit.entity` and its subpackages, `org.bukkit.attribute`, `org.bukkit.potion`, `org.bukkit.projectiles` |
| `worlds` | Worlds and blocks | `org.bukkit.block` and its subpackages, `org.bukkit.util` |
| `items` | Items and data | `org.bukkit.inventory.meta` and its subpackages, `org.bukkit.enchantments`, `org.bukkit.persistence`, `org.bukkit.material` and `org.bukkit.material.types` |
| `inventories` | Inventories | `org.bukkit.inventory` |
| `commands` | Commands and permissions | `org.bukkit.command`, `org.bukkit.command.defaults`, `org.bukkit.permissions`, `org.bukkit.util.permissions`, `org.bukkit.help` |
| `events-fpg` | The events of the classes F, P and G | The concrete event classes of the subpackages of `org.bukkit.event` and of `org.spigotmc.event` whose hook is of the class F, P or G |
| `events-m` | The events of the class M | Those whose hook is of the class M |
| `display` | Display | `org.bukkit.scoreboard`, `org.bukkit.boss`, `org.bukkit.plugin.messaging` |
| `content` | Content | `org.bukkit.map`, `org.bukkit.loot`, `org.bukkit.advancement`, `org.bukkit.structure`, `org.bukkit.ban`, `org.bukkit.profile`, `org.bukkit.inventory.recipe`, `org.bukkit.packs` |
| `worldgen` | World creation | `org.bukkit.generator` and its subpackage, `org.bukkit.util.noise` |

A more specific package wins over a shorter one, so `org.bukkit.inventory.meta` is `items` and
`org.bukkit.util.noise` is `worldgen`. An inner type goes with its outer type, such as
`PlayerTeleportEvent$TeleportCause` with its event, and a type of an event package that is no event
class, such as `org.bukkit.event.inventory.InventoryType` or `org.bukkit.event.block.Action`, goes
with `event-core`. The 65 packages of the inventory, counted on 2026-11-06, each have a slug by
these rules. The first measurements, the remote clients, the seam of the Bukkit layer, the process
plugins and the conversion kit build no member and own no file.

### 2.2 The keys of a member

| Key | Status | Value |
| --- | --- | --- |
| `status` | All | One of `schema`, `key`, `lib`, `via`, `none` and `decide` |
| `target` | `schema`, `key`, `lib`, `via` | A coordinate of section 2.3, a key such as `minecraft:stone`, or the Rust path of a value helper in `golem-client` |
| `thread` | `schema` | `level`, `global`, `any`, `snapshot` or `worldgen`, see [Plugin API](api.md#71-the-threads-and-what-they-may-do). `level` when absent |
| `snapshot` | `schema` with `thread = "snapshot"` | The field of the tick snapshot that answers off the thread of its level, as `api/golem-own.toml` names it, `player.<field>` or `world.<field>`. Off the thread of its level the member answers from the record of its receiver when the snapshot holds one, an online player or a loaded world, and `NOT_ON_SERVER_THREAD` for any other receiver, as a member of the class `level` does off its thread |
| `receiver` | `schema` with a mutation | The argument of the mutation that names the receiver, the declaring type in lower camel case, such as `damageable`. A type with one instance, such as `Server`, has none |
| `params` | `schema` with Java parameters | One element per Java parameter in order: the name of the argument it fills, or a list of names for a value that the schema takes flat, such as `["world", "x", "y", "z", "yaw", "pitch"]` for a `Location` |
| `read` | `schema`, a getter | A Java expression over `it`, the object of the receiver, and the names of `params`, whose value is the answer, or `"hand"` for a method of `bridge/mod/.../game/<Area>Hand.java` that a person writes |
| `write` | `schema`, a setter or an action | Java statements over `it` and the names of `params`, or `"hand"` |
| `rust` | `schema` | The path of the coordinate function written by hand in `golem-model`, where the meaning of Bukkit needs Rust |
| `frame` | `schema`, a member of an event class | The field of the frame of the event that it reads or writes, so no JNI call is made |
| `record` | `schema` | `true` for a field that the record of the type reads |
| `on` | `schema` with `read` or `write` | The class of `it` when it differs from the `on` of the type. A subclass of the `on` of the type narrows the receiver, and the generator writes the expression under `if (receiver instanceof <on> it)` with `otherwise` in the other branch |
| `otherwise` | `schema` with an `on` that narrows the receiver | What the member does for a receiver that is no instance of its `on`, as CraftBukkit does it: `"nothing"` for a write that does nothing, a Java expression for the answer of a read, such as `"false"`, or `"refuse"` for the error `WRONG_RECEIVER` |
| `loads_chunk` | `schema` | `true` for a read that may load a chunk, as Bukkit's does, which the runtime counts per plugin, and which a remote operation answers with `CHUNK_NOT_LOADED` for an unloaded chunk instead of loading it |
| `area` | Any | The part of the API that builds the member, where it differs from the part that owns the file |
| `registry` | `key`, and `via` with a key as its target | The ids of the registries that hold the key |
| `rewrite` | `via` | How a conversion rewrites a call of the member, in one sentence |
| `reason` | `none`, `decide` | A reason of the closed list of section 4 for `none`, the open question for `decide` |
| `why` | `none` | The evidence of the ruling, in one or two sentences |
| `proposal` | `decide` | The coordinate that the rules propose |
| `doc` | `schema`, `lib` | The description in Golem's own words, which reaches the SDL or the documentation of the client. It may name the Bukkit member in backticks, and the check then holds that name to this entry |

An entry with the status `schema` and none of `read`, `write`, `rust`, `frame` and `record` is
mapped and not present: the generator writes no coordinate for it, so the SDL never holds a field
that answers "not implemented". The generator writes the contract call and its Java from `read` and
`write`, checks that a method named `"hand"` exists with the signature the entry implies, and calls
the function that `rust` names.

### 2.3 Coordinates

| Form | Means | Example |
| --- | --- | --- |
| `Type` | A type of the SDL | `LivingEntity` |
| `Type.field` | A field with no argument | `LivingEntity.eyeHeight` |
| `Type.field(a:, b:)` | A field with the arguments the member fills. Every non null argument of the field must be named, and an overload names the optional ones it fills | `World.blockAt(x:, y:, z:)` |
| `Query.name(...)`, `Mutation.name(...)`, `Subscription.name(...)` | A root field with its arguments | `Mutation.damageableSetHealth(damageable:, health:)` |
| `Enum.VALUE` | A value of a GraphQL enum | `GameMode.CREATIVE` |

### 2.4 The table of an event

The mapping file of every event class that a listener can name, 252 of the 268 by the prototype
`counts.py` on 2026-11-06, holds one table `event`:

| Key | Value | Checked against |
| --- | --- | --- |
| `subscription` | The root field of the event, such as `Subscription.playerMove` | The SDL |
| `type` | The GraphQL type of the event | The SDL |
| `chain` | The class whose handler list the event joins: itself when it declares a static `getHandlerList`, else the nearest superclass that does | The inventory |
| `cancellable` | Whether the class or an ancestor implements `org.bukkit.event.Cancellable` | The inventory |
| `rate` | `A`, `B`, `C` or `D`, see [Plugin API](api.md#106-what-an-event-costs) | The closed list |
| `thread` | `level`, `global`, `chat`, `net` or `worldgen`, the thread its chain runs on: the thread of the level where the event happens, the thread of the global work of the tick, `golem-chat-<n>`, `golem-net-<n>` for the chains that the threads of Netty and of the login hand over, or a thread of world generation. While one server thread ticks every level, `level` and `global` both run on it | The closed list |
| `hook` | An inline table: `class` of `F` Forge enough, `P` Forge and a Mixin, `M` a Mixin alone or `G` fired by Golem, `forge` with the Forge event classes it starts from, `mixin` with the target in official names, and `source`, where the hook sits in Golem's own words under the clean room rule of [Plugin API](api.md#15-licensing-and-the-clean-room) | The Forge jar for `forge`, `28_api_hooks` for the rest |
| `frame` | The fields of the frame with their contract types, in the order of the offsets | The contract generator |
| `writable` | The fields of the frame a subscriber may change, each the `frame` of one setter among the members | The members |
| `filters` | The filter arguments of its subscription field, which the dispatcher checks from the frame | The SDL |
| `pushdown` | The filters whose coarse part the interest table of Java checks as well | A subset of `filters` |

An abstract event class that no listener names, such as `PlayerEvent`, has no table `event`. Its
members still map, since its getters are fields of every event type below it.

## 3. The statuses

| Status | Means | Required keys | Mapped | Present when | Proven when |
| --- | --- | --- | --- | --- | --- |
| `schema` | The member is a coordinate of the SDL | `target`, `doc`, and one of `read`, `write`, `rust`, `frame` and `record` to be present | Yes | The target exists in `api/schema.graphql` down to its arguments | A document of the corpus selects it, its answer matches the golden answer on a booted server, and the run recorded that the coordinate resolved |
| `key` | The constant is a key of a registry of Minecraft | `target`, `registry` | Yes | The key is in the registries report that the data generator of the server jar writes with `--reports`, under each registry it names | The registries of the booted test server hold it |
| `lib` | A helper of `golem-client` with no game involved: a pure helper of a value, the methods `equals`, `hashCode`, `toString` and `clone` of values included, or the logger of the runtime that `getLogger()` of `Plugin`, `Server` and `Bukkit` answers | `target`, `doc` | Yes | The path is named in `lib_items.rs` and `golem-client` builds | A cell of `golem-devtool api lib` matches its golden values |
| `via` | A legacy member that a conversion rewrites to a current coordinate or key | `target`, `rewrite` | Yes | Its target is present | Its target is proven |
| `none` | A ruling that Golem does not offer the member | `reason`, `why` | Yes | Always, by its ruling | By its review in the pull request that set it |
| `decide` | The rules propose a target and nobody ruled yet | `proposal`, `reason` | No | Never | Never |

There is no status for "planned" or "not yet". A `schema` entry whose coordinate is not in the SDL
is mapped and not present, which is the honest state of a member whose Java or Rust nobody wrote
yet, and the report says so.

## 4. The reasons of `none`

| Reason | Means | Example |
| --- | --- | --- |
| `anonymous` | A member of one of the 50 anonymous classes of the jar, with their 61 methods, which no plugin can name | `org.bukkit.ChatColor$1`, `public net.md_5.bungee.api.ChatColor asBungee()` |
| `java-only` | Machinery of Java that has no meaning in a Rust plugin: class loaders, plugin loaders, the `HandlerList` objects, Java serialization | `JavaPluginLoader.loadPlugin(File)` |
| `internal` | A member that Spigot marks as internal or unsafe for plugins | `UnsafeValues` |
| `replaced` | A mechanism of Golem replaces it, which `why` names | `CustomTimingsHandler.startTiming()`, which the budgets replace |
| `unsupported` | CraftBukkit 1.20.1 itself refuses it or answers nothing, which `why` cites | `Material.getId()`, which refuses every current material |

A new reason needs a change of this page and of the tool in the same pull request. The report
prints the count per reason.

## 5. Three examples of each status

The keys below are real lines of `inventory.json` of spigot-api 1.20.1-R0.1-20230921.163938-66,
copied from it on 2026-11-06.

### 5.1 `schema`

A getter from a Java expression, in `api/bukkit/org.bukkit.entity/LivingEntity.toml`:

```toml
[members."public abstract double getEyeHeight()"]
status = "schema"
target = "LivingEntity.eyeHeight"
read = "it.getEyeHeight()"
doc = "The height of the eyes above the feet, in blocks."
```

A setter whose meaning needs Rust, in `api/bukkit/org.bukkit.entity/Damageable.toml`:

```toml
[members."public abstract void setHealth(double)"]
status = "schema"
target = "Mutation.damageableSetHealth(damageable:, health:)"
receiver = "damageable"
params = ["health"]
rust = "golem_model::coordinate::damageable::set_health"
doc = "Sets the health. A value below zero or above the maximum is refused, and zero kills."
```

A setter of an event that writes the frame, in
`api/bukkit/org.bukkit.event.player/PlayerMoveEvent.toml`:

```toml
[members."public void setTo(org.bukkit.Location)"]
status = "schema"
target = "Mutation.playerMoveEventSetTo(playerMoveEvent:, world:, x:, y:, z:, yaw:, pitch:)"
receiver = "playerMoveEvent"
params = [["world", "x", "y", "z", "yaw", "pitch"]]
frame = "to"
doc = "Sets where the player moves to. The mod moves the player there when the chain ends."
```

### 5.2 `key`

The head of `api/bukkit/org.bukkit/Material.toml` and one constant:

```toml
type = "org.bukkit.Material"
kind = "class"
area = "items"
status = "schema"
target = "MaterialInfo"
registry = ["minecraft:block", "minecraft:item"]
doc = "What the game knows about a block or an item, read once from the registries."

[members."public static final org.bukkit.Material STONE"]
status = "key"
target = "minecraft:stone"
registry = ["minecraft:block", "minecraft:item"]
```

In `api/bukkit/org.bukkit.entity/EntityType.toml`:

```toml
[members."public static final org.bukkit.entity.EntityType ZOMBIE"]
status = "key"
target = "minecraft:zombie"
registry = ["minecraft:entity_type"]
```

In `api/bukkit/org.bukkit/Sound.toml`:

```toml
[members."public static final org.bukkit.Sound ENTITY_ZOMBIE_AMBIENT"]
status = "key"
target = "minecraft:entity.zombie.ambient"
registry = ["minecraft:sound_event"]
```

### 5.3 `lib`

In `api/bukkit/org.bukkit/Location.toml`:

```toml
[members."public double distance(org.bukkit.Location)"]
status = "lib"
target = "golem_client::value::Location::distance"
doc = "The distance to another location of the same world, in blocks. Another world is an error."
```

A method of `Object` on a value, in the same file:

```toml
[members."public boolean equals(java.lang.Object)"]
status = "lib"
target = "golem_client::value::Location::eq"
doc = "Two locations are equal when the world, the coordinates and the rotation are equal."
```

In `api/bukkit/org.bukkit/ChatColor.toml`:

```toml
[members."public static java.lang.String translateAlternateColorCodes(char, java.lang.String)"]
status = "lib"
target = "golem_client::text::translate_alternate_color_codes"
doc = "Turns a code character followed by a colour or format letter into the section sign form."
```

### 5.4 `via`

In `api/bukkit/org.bukkit.material/Stairs.toml`:

```toml
[members."public void setInverted(boolean)"]
status = "via"
target = "StairsData.half"
rewrite = "Set the half of the block data to TOP for true and BOTTOM for false, then write it."
```

In `api/bukkit/org.bukkit.block/Block.toml`:

```toml
[members."public abstract byte getData()"]
status = "via"
target = "Block.blockData"
rewrite = "Read the property that the old data value encoded from the block data."
```

In `api/bukkit/org.bukkit/Material.toml`:

```toml
[members."public static final org.bukkit.Material LEGACY_STONE"]
status = "via"
target = "minecraft:stone"
registry = ["minecraft:block", "minecraft:item"]
rewrite = "The legacy table of Bukkit maps LEGACY_STONE with the data value 0 to minecraft:stone."
```

### 5.5 `none`

In `api/bukkit/org.bukkit/UnsafeValues.toml`:

```toml
[members."public abstract org.bukkit.Material fromLegacy(org.bukkit.Material)"]
status = "none"
reason = "internal"
why = "Bukkit tells plugins not to use UnsafeValues, and each legacy constant has its via."
```

In `api/bukkit/org.bukkit.plugin.java/JavaPluginLoader.toml`:

```toml
[members."public org.bukkit.plugin.Plugin loadPlugin(java.io.File) throws org.bukkit.plugin.InvalidPluginException"]
status = "none"
reason = "java-only"
why = "Golem plugins are crates compiled into golem, and no jar is loaded at run time."
```

In `api/bukkit/org.bukkit/Material.toml`:

```toml
[members."public int getId()"]
status = "none"
reason = "unsupported"
why = "The sources jar checks the legacy flag first and refuses every current material."
```

### 5.6 `decide`

In `api/bukkit/org.bukkit.entity/Villager.toml`:

```toml
[members."public abstract org.bukkit.entity.ZombieVillager zombify()"]
status = "decide"
proposal = "Mutation.villagerZombify(villager:)"
reason = "A verb with no prefix of a getter or a setter, which the rules cannot class."
```

In `api/bukkit/org.bukkit.entity/Allay.toml`:

```toml
[members."public abstract org.bukkit.entity.Allay duplicateAllay()"]
status = "decide"
proposal = "Mutation.allayDuplicateAllay(allay:)"
reason = "A verb that spawns an entity and answers it, a mutation by its effect."
```

In `api/bukkit/org.bukkit/World.toml`:

```toml
[members."public abstract org.bukkit.util.RayTraceResult rayTraceBlocks(org.bukkit.Location, org.bukkit.util.Vector, double)"]
status = "decide"
proposal = "World.rayTraceBlocks(x:, y:, z:, directionX:, directionY:, directionZ:, maxDistance:)"
reason = "A read named by a verb, a field of World by the rule of reads, and it may load chunks."
```

## 6. Two whole files

The head of `api/bukkit/org.bukkit.entity/LivingEntity.toml`, with five of its 72 members:

```toml
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# The mapping of org.bukkit.entity.LivingEntity, written by golem-devtool api propose and
# reviewed by hand. Plugin API section 14 and API coverage say what each key means.

type = "org.bukkit.entity.LivingEntity"
kind = "interface"
area = "entities"
status = "schema"
target = "LivingEntity"
on = "net.minecraft.world.entity.LivingEntity"
record = "livingRecord"
model = "golem_model::coordinate::living_entity"
doc = "An entity with health, effects and equipment."

[members."public abstract double getEyeHeight()"]
status = "schema"
target = "LivingEntity.eyeHeight"
read = "it.getEyeHeight()"
doc = "The height of the eyes above the feet, in blocks."

[members."public abstract boolean hasLineOfSight(org.bukkit.entity.Entity)"]
status = "schema"
target = "LivingEntity.hasLineOfSight(other:)"
params = ["other"]
rust = "golem_model::coordinate::living_entity::has_line_of_sight"
doc = "Whether nothing blocks the view from the eyes of this entity to the other entity."

[members."public abstract int getRemainingAir()"]
status = "schema"
target = "LivingEntity.remainingAir"
thread = "snapshot"
snapshot = "player.remainingAir"
read = "it.getAirSupply()"
record = true
doc = "The ticks of air left under water. Off the thread of its level, for an online player, the value at the last tick end."

[members."public abstract int getMaximumAir()"]
status = "schema"
target = "LivingEntity.maximumAir"
read = "it.getMaxAirSupply()"
record = true
doc = "The ticks of air the entity holds when it surfaces."

[members."public abstract void setAI(boolean)"]
status = "schema"
target = "Mutation.livingEntitySetAi(livingEntity:, ai:)"
receiver = "livingEntity"
params = ["ai"]
on = "net.minecraft.world.entity.Mob"
write = "it.setNoAi(!ai);"
otherwise = "nothing"
doc = "Turns the brain of a mob on or off. An entity that is no mob ignores it, as in CraftBukkit."
```

The generator writes the Java of `setAI` as `if (entity instanceof Mob it) { it.setNoAi(!ai); }`
with an empty other branch, which is what `CraftLivingEntity.setAI` does, and `hasAI` carries
`otherwise = "false"`.

The head of `api/bukkit/org.bukkit.event.player/PlayerMoveEvent.toml` with its event table and two
getters:

```toml
type = "org.bukkit.event.player.PlayerMoveEvent"
kind = "class"
area = "events-m"
status = "schema"
target = "PlayerMoveEvent"
doc = "A player moved, by at least 1/256 of a block or 10 degrees, once per movement packet."

[event]
subscription = "Subscription.playerMove"
type = "PlayerMoveEvent"
chain = "org.bukkit.event.player.PlayerMoveEvent"
cancellable = true
rate = "C"
thread = "level"
hook = { class = "M", mixin = "ServerGamePacketListenerImpl#handleMovePlayer", source = "After the checks of the packet and before the move is applied" }
frame = ["player: entity", "from: location", "to: location"]
writable = ["from", "to"]
filters = ["player", "world", "changedBlock", "changedChunk", "region"]
pushdown = ["player", "world", "changedBlock", "changedChunk"]

[members."public org.bukkit.Location getFrom()"]
status = "schema"
target = "PlayerMoveEvent.from"
frame = "from"
doc = "Where the player stood before the move."

[members."public org.bukkit.Location getTo()"]
status = "schema"
target = "PlayerMoveEvent.to"
frame = "to"
doc = "Where the move ends, as the chain has set it so far."
```

## 7. The other files

### 7.1 The overview of the events

`api gen` writes `api/events.toml` with one row per `[event]` table, so a person sees every event
with its hook, its rate and its thread in one file and edits none of it there:

```toml
# written by golem-devtool api gen from the [event] tables of api/bukkit, do not edit

["org.bukkit.event.player.AsyncPlayerChatEvent"]
subscription = "Subscription.asyncPlayerChat"
hook = "P"
rate = "B"
thread = "chat"
chain = "org.bukkit.event.player.AsyncPlayerChatEvent"
```

### 7.2 Names and Golem's own coordinates

```toml
# api/names.toml
["org.bukkit.block.Structure"]
graphql = "StructureBlock"
why = "org.bukkit.structure.Structure keeps the name Structure."
```

`api/golem-own.toml` holds a table per coordinate with a `why` and the keys of a member, so its
Java and its Rust reach the contract and the client as a member's do, and the table `snapshot` with
the fields of the tick snapshot:

```toml
# api/golem-own.toml
["Server.tps"]
why = "The ticks per second of the window of tick statistics of Golem."
thread = "any"
rust = "golem_model::coordinate::server::tps"
doc = "The ticks per second over the last minute."

["World.safeLocation(x:, y:, z:, radius:)"]
why = "The safe place search of the first game interface, which the port of EssentialsX calls."
params = ["x", "y", "z", "radius"]
read = "hand"
loads_chunk = true
doc = "The nearest place within the radius where a player stands safely, or null."

["CommandSender.permissionLevel"]
why = "The operator level of the source of a command, 0 to 4, which vanilla and Forge decide by."
thread = "any"
rust = "golem_model::coordinate::command_sender::permission_level"
doc = "The operator level of the sender."

["HumanEntityObject"]
why = "Keeps HumanEntity an interface until Aosabi reads traits, and types a modded human entity."

[snapshot]
player = ["uuid", "name", "displayName", "world", "x", "y", "z", "yaw", "pitch", "gameMode",
    "health", "foodLevel", "remainingAir", "level", "exp", "ping", "permissionLevel", "idleMillis"]
world = ["time", "fullTime", "storm", "thundering", "playerCount", "entityCount", "loadedChunks"]
```

### 7.3 The value helpers

```rust
// crates/golem-client/src/lib_items.rs, written by golem-devtool api lib
/// The path of every `lib` target of the mapping.
pub const LIB_ITEMS: &[&str] = &[
    "golem_client::value::Location::distance",
    "golem_client::value::Location::eq",
    "golem_client::text::translate_alternate_color_codes",
    // ...
];

/// Names every path of `LIB_ITEMS`, so a helper that does not exist fails the build at its line.
#[allow(dead_code)]
fn named() {
    let _ = crate::value::Location::distance;
    let _ = <crate::value::Location as PartialEq>::eq;
    let _ = crate::text::translate_alternate_color_codes;
}
```

The pinned toolchain prints no rustdoc JSON without an unstable flag, so the build of the client
itself is the check, and the workspace holds no `#[test]`, as CONTRIBUTING.md section 5 asks.

### 7.4 The lock

```toml
# api/coverage.lock, written by golem-devtool api coverage --lock
reference = "spigot-api 1.20.1-R0.1-20230921.163938-66"
jar-sha256 = "d8aa356781e32896e7d78490813975cac7ca8d99126adc41c2836c56c68b9c3e"
schema-sha256 = "<the SHA-256 of api/schema.graphql>"

[all]
members = 15089
mapped = 14895
present = 0
proven = 0

[kind.methods]
members = 7942
mapped = 7750
present = 0
proven = 0

[status.decide]
members = 194
```

The lock holds one table of the same four keys for every kind, every status and every value of
`area`. The mapped numbers above are those that the rules of the prototype `coverage_rules.py` give
over the inventory on 2026-11-06, with the 61 methods of the anonymous classes ruled `none` and 192
methods and 2 fields left to decide. The first run of the coverage tool writes the real lock with
the numbers it counts.

### 7.5 A document of the corpus

```graphql
# tests/corpus/api/entities/air.graphql
# The scene of 26_api_corpus spawns an armor stand at a fixed UUID before the documents run.
query Air($id: ID!) {
  entity(id: $id) {
    ... on LivingEntity { remainingAir maximumAir }
  }
}
```

```json
{"variables": {"id": "00000000-0000-4000-8000-000000000001"},
 "answer": {"data": {"entity": {"remainingAir": 300, "maximumAir": 300}}}}
```

The `.json` file carries its SPDX lines in a `.license` file beside it, as REUSE asks for JSON.

## 8. The report

`golem-devtool api coverage` prints the report that CI shows, and writes it with every member and
its state to `target/api/coverage.json`:

```text
reference spigot-api 1.20.1-R0.1-20230921.163938-66, schema sha256 <...>
                      members   mapped  present   proven   covered of all
types                   1,247
methods                 7,942
fields and constants    5,900
all                    15,089
  schema
  key
  lib
  via
  none                          by reason: anonymous java-only internal replaced unsupported
  decide                  (not mapped)
listenable events         252
area entities
area worlds
package org.bukkit.entity
...
```

The column "covered of all" divides the members of the statuses `schema`, `key`, `lib` and `via`
that are present by every member, `none` included, beside the share of the members that are not
`none`.

## 9. The checks

`golem-devtool api coverage` fails when any of these breaks.

1. Every type, method and field of the inventory has exactly one entry, and no entry names a line
   the inventory lacks. A missing or a dangling one fails with the file and the line.
2. The path of every file and its `type` and `kind` agree with the inventory, and its `area` is a
   slug of the table of section 2.1 that owns its package, or for an event class the slug that the
   hook class of its table `event` gives.
3. Every entry carries the keys its status requires, and no key that its status forbids.
4. Every `schema` target is present or is counted mapped and not present, with its arguments,
   `receiver` and `params` agreeing with the field of the SDL.
5. Every `key` is in the registries report, under each registry it names.
6. Every `lib` path is in `lib_items.rs`, and `golem-client` builds.
7. Every `via` target is a coordinate or a key, present or counted not present.
8. Every `none` has a reason of the closed list of section 4 and a `why`.
9. Every listenable event, 252 by the prototype `counts.py` on 2026-11-06, has a table `event`
   with every key of section 2.4, its `chain` and `cancellable` agree with the inventory,
   and every field of `writable` is the `frame` of one of its setters and every setter writes a
   field of `writable`. An event with `cancellable = true` and a hook of the class `F` names in
   `forge` only classes that carry the annotation `net.minecraftforge.eventbus.api.Cancelable` in
   the Forge jar, so an event that Forge cannot cancel, such as `entityExplode` over
   `ExplosionEvent.Detonate`, is of the class `P` with its Mixin.
10. Every getter of an event class and of its ancestors is a field of its event type, `isCancelled`
    included, and every setter a mutation of it.
11. Every member with `thread = "snapshot"` names a field of the table `snapshot` of
    `api/golem-own.toml`, of a record that its receiver can have: `player.<field>` on `Player` and
    the types it implements, `world.<field>` on `World`. Off the thread of its level such a member
    answers `NOT_ON_SERVER_THREAD` for a receiver that has no record, so the check needs no other
    rule.
12. Every member whose `on` is a subclass of the `on` of its type carries `otherwise`, and no other
    member does.
13. Every description of the SDL that names a Bukkit member in backticks agrees with the entry of
    that member.
14. Every coordinate of the SDL is the target of some entry, a line of `api/golem-own.toml` or a
    line of the `part.toml` of exactly one port, and no `part.toml` lists a coordinate of the
    mapping or of `api/golem-own.toml`.
15. No two types share a GraphQL name, after the rules and `api/names.toml`.

## 10. The notebook `22_api_coverage`

| Cell | What it does |
| --- | --- |
| Parameters | The version of the reference, the SHA-256 of the jar and of the sources jar, the path of the cache |
| Fetch | `python3 -I tests/fetch.py --only spigot-api`, which downloads both jars from the Spigot repository and checks their SHA-256 |
| Inventory | `golem-devtool api inventory` writes the inventory again with the javap of the pinned JDK, and the cell fails when it differs from the committed file by one byte |
| Generated | `golem-devtool api gen`, `contract gen`, `api client` and `api lib` write their files again, and the cell fails on a difference |
| Schema | `golem-devtool api sdl` prints the SDL with the builder of Aosabi over `golem-api`, splits off the parts of ports by their `part.toml`, and the cell fails when the rest differs from `api/schema.graphql` or a part from the `schema.graphql` of its port |
| Registries | The data generator of the server jar of 1.20.1 writes its registries report with `--reports`, which starts no server |
| Coverage | `golem-devtool api coverage` runs the checks of section 9 and writes the report |
| Lock | The cell compares every number of the report with `api/coverage.lock` and prints the lines to write when they differ |
| Proven | The cell reads `target/api/proven.json` of the last run of `26_api_corpus` when the job has one, and otherwise carries the proven numbers of the lock |
| Slice | `golem-devtool api slice` over the jar of EssentialsX 2.22.0, which prints the members the port needs with their three states |

`26_api_corpus` boots the test server under `golem run` with the corpus plugin, runs every document
of `tests/corpus/api/` through the API, compares each answer with its golden answer, and writes
`target/api/proven.json` from the coordinates that the runtime recorded as resolved during the
passing documents, from the resolve info of the executor and the counts of `g.run`.

## 11. How CI holds the numbers

- The notebooks job of CI runs `22_api_coverage` on every push and every pull request. A mapped or
  present number below the lock fails, and a number above the lock fails as well with the lines to
  write, so the lock always equals the state of `main` and every rise lands in the commit that made
  it, with its numbers in the history.
- The same job runs `26_api_corpus`, which boots the test server, and holds the proven numbers to
  the lock by the same rule.
- The lock holds the numbers per status, per kind and per area, so a fall in one area cannot hide
  behind a rise in another.
- A pull request that changes a status to `none` names the ruling in its body, and one that lowers a
  number on purpose, such as a coordinate taken out of the SDL, changes the lock in the same commit
  and says why in the body of the commit.
- The release notes of every version of golem carry the three numbers of the lock.

Related: [Plugin API](api.md), [Bridge](bridge.md), [Testing](../Documentation/testing.md),
[Index](index.md).
