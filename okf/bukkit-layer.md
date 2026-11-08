---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Spec
title: Bukkit layer
description: The Bukkit layer that runs Bukkit and Spigot plugins unchanged in the JVM of Forge on the Golem API, with the three levels it serves as a survey of 503 plugins measures them, the stand-ins of CraftBukkit and what the API offers behind each, the remapper of Minecraft names and the answers it gives to probes, the shaded libraries, the needs outside the levels, what each work item completes, the plugins no layer runs, the blind spots, how to reproduce the survey and the decisions open for the owner.
tags: [bukkit, spigot, craftbukkit, layer, plugins, remapper, survey, paper]
timestamp: 2026-11-08
---

# Bukkit layer

The Bukkit layer runs Bukkit and Spigot plugins unchanged on Golem, and every plugin must work, as
the owner ruled on 2026-11-06 ([Decisions](decisions.md), [Plan](plan.md#bukkit-plugins)). Golem's
own plugins stay native Rust, and the layer is no form of a Golem plugin.
[Plugin API](api.md#17-the-bukkit-layer) section 17 fixes what the layer takes from the Golem API.
This page says what the layer is, what it must serve by the measure of a survey of 503 plugins,
and in which order it is built.

## 1. What the layer is

The layer is Java in the JVM of Forge 47.4.18, beside Minecraft 1.20.1, the mods and the bridge
mod. It has five parts.

1. **The stock classes of spigot-api 1.20.1, unchanged.** The layer loads the classes of
   `spigot-api` 1.20.1-R0.1-20230921.163938-66 as they are: `SimplePluginManager`,
   `JavaPluginLoader`, `PluginClassLoader`, `LibraryLoader`, `SimpleCommandMap`, `SimpleHelpMap`,
   `PermissibleBase`, `JavaPlugin`, the events and the value classes. Every private field that
   plugins reach in them then exists with its name and its type, such as
   `SimplePluginManager.dependencyGraph` (55 plugins) and `SimpleCommandMap.knownCommands` (97),
   and none of them costs any work.
2. **The CraftBukkit shapes in `org.bukkit.craftbukkit.v1_20_R1`.** `CraftServer`, `CraftPlayer`,
   `CraftWorld`, `CraftItemStack`, `CraftMagicNumbers` and the other classes that plugins reach
   carry the class names, the package, the class hierarchy and real, non-final fields with the
   names and types of CraftBukkit 1.20.1, because plugins find them by name and walk their
   superclasses with `getDeclaredField`. Behind them stand the Golem API and the live objects of
   Forge, see section 4.
3. **A remapper of plugin bytecode in `UnsafeValues.processClass`.** `PluginClassLoader` passes
   every class of a plugin through `UnsafeValues.processClass` before it defines it (line 170 of the
   1.20.1 source), and the layer's `CraftMagicNumbers` implements `UnsafeValues`. The remapper
   rewrites the Spigot 1.20.1 names of Minecraft to the names Forge runs, see section 5.
4. **The `api-version` check in `UnsafeValues.checkSupported`.** `JavaPluginLoader` asks
   `checkSupported` about the descriptor of every plugin before it loads it (line 141 of the 1.20.1
   source), which is where the layer accepts the versions that CraftBukkit 1.20.1 refuses, see
   section 8.2.
5. **The answers of Spigot 1.20.1.** The version strings, the data version, the mappings version,
   the CraftBukkit package `v1_20_R1` and every lookup that fails on Spigot 1.20.1 answer as they
   do there, because plugins choose their code path from them, see section 5.2.

## 2. The survey and how it measures

The counts on this page come from a survey of 503 plugin jars collected on 2026-11-06, the newest
file of each plugin that targets 1.20.1:

| Source | How plugins are chosen | Kept | Dropped as duplicates | Skipped | Size |
| --- | --- | ---: | ---: | ---: | ---: |
| Spigot | Free resources through Spiget, most downloaded first, kept when `testedVersions` lists 1.20 or 1.20.x or the newest tested version is 1.20 or later, latest file | 250 | 1 | 373 | 282 MB |
| Modrinth | Plugins with game version 1.20.1 and loader bukkit, spigot, paper or purpur, most downloaded first, newest release for 1.20.1, primary file | 150 | 25 | 0 | 559 MB |
| Hangar | Projects with platform Paper, most downloaded first, newest release whose platforms hold 1.20.1, else 1.20, the 100 most downloaded that pass | 100 | 79 | 170 | 267 MB |
| Live | The plugins of the live server: EssentialsX 2.22.0 and EssentialsXSpawn 2.22.0 from the GitHub release, LuckPerms-Bukkit 5.5.71 from Modrinth | 3 | 0 | 0 | 6 MB |
| All | | 503 | 105 | 543 | 1.11 GB |

`tools/plugin-survey/corpus.py` builds the corpus and `tools/plugin-survey/scan.py` reads every
class file of every jar, nested jars included. A second, independent scanner reads the same jars,
and a reconciling pass joins the two scans into one record per jar in the format of `scan.py`,
`reconciled.jsonl`. The summary steps in `tools/plugin-survey/summary/` count the reconciled
records, and the scans and the counts are of 2026-11-06. Where a count comes instead from one of two
independent analyses of the same records of 2026-11-06, called the first and the second analysis
below, the text names the analysis. Section 12 says how to run the tools.

- **Counts are jars.** Every count is plugins of this corpus, unless the text names members,
  classes or downloads.
- **On 1.20.1.** A direct reference counts when the class that holds it runs on 1.20.1 by its
  version tag, and a versioned CraftBukkit package counts only if it is `v1_20_R1`. A reflection
  site counts when the class of its method runs on 1.20.1 and its target resolves. Version checks
  inside methods are not evaluated, so every count on 1.20.1 is an upper bound.
- **Downloads.** The corpus has 374,500,083 downloads when every listing of a plugin counts once
  per site (Spigot 103.5 M, Modrinth 268.0 M, Hangar 3.0 M) and 339,584,892 when only the listing
  that the jar is from counts. Two Modrinth projects that are mods first, VeinMiner (100.4 M) and
  Simple Voice Chat (75.8 M), weigh 47 % of all downloads, so this page plans by jars and gives
  downloads as context only.
- **References.** The public API is the spigot-api jar above: 1,247 types, 7,942 public methods,
  5,900 fields and constants and 268 event classes, 250 of them concrete. The Spigot 1.20.1 names
  of Minecraft come from `bukkit_srg.srg` of Arclight 1.20.1, 5,045 classes, 3,818 of them named
  differently from Mojang, and Forge's `srg_to_official_1.20.1.tsrg` maps SRG member names to
  Mojang names.
- **Descriptors.** 491 jars have a `plugin.yml`, 28 a `paper-plugin.yml` and 12 only a
  `paper-plugin.yml`. The builds of the 412 jars with real entry dates run from 2020-07-18 to
  2026-11-06 with the median at 2025-09-01, so most files are newer than 1.20.1 and built to run on
  many versions.

## 3. The three levels

A plugin is level 3 when it reaches classes of Minecraft, level 2 when it reaches classes of
CraftBukkit, the runtime class behind an API object by a member lookup or a non-public member of the
API jar, and level 1 otherwise. The summary step `levels.py` gives these levels for all 503 jars,
equal to the levels of the reconciling pass on all code paths.

| Level | What the plugin reaches | All code paths | On 1.20.1 | Share of downloads on 1.20.1 |
| --- | --- | ---: | ---: | ---: |
| 1 | The public API of Bukkit and Spigot only | 180 | 183 | 19.8 % |
| 2 | CraftBukkit, the runtime classes behind the API or internals of the API jar | 75 | 81 | 9.6 % |
| 3 | Classes of Minecraft | 248 | 239 | 70.6 % |

Counting only code that runs on 1.20.1 moves 9 plugins down from level 3 and none up, so version
adapters thin out little of the work: 239 of the 248 level 3 plugins reach Minecraft in code that
runs on this server.

One class per plugin, the first that applies, on 1.20.1 (`classify.py`):

| Class | Live | Spigot | Modrinth | Hangar | Plugins | Share of downloads |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Runtime patching of server classes | 0 | 0 | 0 | 1 | 1 | 0.0 % |
| Paper only: `paper-plugin.yml` without `plugin.yml` | 0 | 1 | 5 | 6 | 12 | 0.3 % |
| Needs level 3 | 1 | 114 | 67 | 51 | 233 | 70.3 % |
| Needs level 2 | 1 | 47 | 19 | 8 | 75 | 8.6 % |
| Level 1 with the stock API classes: level 2 by the rule, but only internals of the API jar, a lookup of the `CraftServer` class or reflection on plugin classes | 0 | 5 | 1 | 0 | 6 | 1.1 % |
| Level 1 only | 1 | 83 | 58 | 34 | 176 | 19.7 % |

Of the live plugins, EssentialsX is level 3, LuckPerms level 2 and EssentialsXSpawn level 1.

## 4. Level 2: the stand-ins of CraftBukkit

### 4.1 The members plugins reach

Plugins on 1.20.1 by `members.py`, directly and by reflection. Members of the API jar, such as
`SimpleCommandMap.knownCommands`, stand with the CraftBukkit members because plugins reach them the
same way. A row that names the first or the second analysis comes from that analysis.

| Member | Plugins | Directly | By reflection |
| --- | ---: | ---: | ---: |
| `CraftPlayer.getHandle` | 160 | 58 | 122 |
| `CraftItemStack.asNMSCopy` | 130 | 48 | 102 |
| `CraftEntity.getHandle` | 97 | 32 | 76 |
| `SimpleCommandMap.knownCommands`, 94 through that class and 3 through the runtime class of the command map | 97 | 15 | 97 |
| `CraftWorld.getHandle` | 83 | 52 | 42 |
| `CraftServer.commandMap` (field) | 77 | 0 | 77 |
| `CraftItemStack.asCraftMirror` | 70 | 14 | 61 |
| `CraftItemStack.asBukkitCopy` | 58 | 27 | 33 |
| `CraftMetaSkull.profile` (field, reached through the runtime class of the item meta) | 58 | 0 | 58 |
| `CraftServer.getCommandMap` | 57 | 10 | 47 |
| `CraftBossBar.handle` (field) | 56 | 0 | 56 |
| `SimplePluginManager.dependencyGraph` (field) | 55 | 0 | 55 |
| `CraftRegistry.getMinecraftRegistry` | 41 | 0 | 41 |
| `CraftServer.getServer` | 38 | 24 | 16 |
| `CraftMagicNumbers.getMaterial` | 29 | 10 | 20 |
| `PluginCommand` protected constructor (first analysis) | 26 | 0 | 26 |
| `VanillaCommandWrapper.getListener` (first analysis) | 25 | 6 | 19 |
| `Material.BY_NAME` (private field of the API) | 24 | 0 | 24 |
| `SimplePluginManager.commandMap` (field) | 24 | 0 | 24 |
| `CraftServer.getMinecraftVersion`, a Paper method that plugins probe (second analysis) | 22 | 0 | 22 |
| `CraftServer.syncCommands` | 18 | 8 | 10 |
| `CraftServer.console` (field, second analysis) | 17 | 0 | 17 |
| `CraftPlayer.entity` (field, second analysis) | 15 | 0 | 15 |
| `CraftHumanEntity.perm` (field) | 2 | 0 | 2 |

Some members that plugins reach do not exist in CraftBukkit 1.20.1, as a check against the
CraftBukkit `v1_20_R1` classes in Arclight's `common.jar` shows: `CraftServer.getMinecraftVersion`
(22 plugins), `CraftItemStack.asBukkitMirror` (7), `CraftWorld.getChunkAtAsync` (6),
`getScheduler`, `teleportAsync`, `showTitle` and `getTPS`. They are Paper members. On Spigot 1.20.1
the lookups fail and the plugins take their Spigot path, so the layer offers the surface of
CraftBukkit 1.20.1 exactly and adds Paper members only with the whole of Paper, see section 8.4.

### 4.2 The groups, ranked

The stand-ins grouped by the object they wrap, by `groups.py`, ranked by plugins on 1.20.1 with all
code paths in brackets. 306 plugins reach at least one group on 1.20.1. The member counts beside the
groups come from section 4.1 and, where it lacks a member, from the second analysis.

| Rank | Group | Plugins on 1.20.1 | Leading members (plugins) |
| ---: | --- | ---: | --- |
| 1 | Entity and player handles | 228 (235) | `CraftPlayer.getHandle` 160, `CraftEntity.getHandle` 97, `CraftPlayer.entity` 15, `CraftLivingEntity.getHandle` 14, `addChannel` 7 |
| 2 | Item stacks, item meta and persistent data | 169 (175) | `asNMSCopy` 130, `asCraftMirror` 70, `asBukkitCopy` 58, `CraftItemStack.handle` 11, `CraftMetaItem.displayName` 11, `lore`, `CraftPersistentDataContainer` 26 |
| 3 | Worlds, chunks, blocks and block data | 161 (167) | `CraftWorld.getHandle` 83, `CraftBlockData.getState` 21, `fromData` 13, `CraftChunk.getHandle` 8, `getTileEntityAt`, `addEntity` |
| 4 | Command map, registration and help | 140 (140) | `knownCommands` 97, `CraftServer.commandMap` 77, `getCommandMap` 57, `VanillaCommandWrapper` 27, `BukkitCommandWrapper` 26, `PluginCommand` constructor 26, `SimplePluginManager.commandMap` 24, `syncCommands` 18, `SimpleHelpMap.helpTopics` 7 |
| 5 | Conversions between Bukkit and Minecraft values | 101 (109) | `CraftRegistry.getMinecraftRegistry` 41, `CraftMagicNumbers.getMaterial` 29, `getBlock`, `getItem`, `INSTANCE`, `CraftParticle.toNMS` 17, `CraftNamespacedKey`, `CraftSound`, `CraftEnchantment`, `CraftPotionUtil` |
| 6 | Server handle and server state | 84 (87) | `CraftServer.getServer` 38, `console` 17, `getHandle` 7, `warningState` 5, `worlds`, and 64 more plugins that only look the `CraftServer` class up, mostly to read its package |
| 7 | Game profiles of skulls and players | 76 (78) | `CraftMetaSkull.profile` 58, `setProfile`, `CraftSkull.profile` 7, `CraftPlayer.getProfile` 11, `CraftPlayerProfile` |
| 8 | Boss bars | 56 (57) | `CraftBossBar.handle` 56 |
| 9 | Chat messages | 31 (47) | `CraftChatMessage.fromJSON` 12, `fromString` 11, `fromStringOrNull`, `toJSON` |
| 10 | Inventories, containers and menus | 28 (30) | `CraftInventoryView.getTopInventory` 17, `CraftContainer.getNotchInventoryType` 7, `CraftInventory.getInventory` 5 |
| 11 | Other CraftBukkit classes | 21 (25) | Handles of single entity types, advancements, chunk snapshots, damage sources |
| 12 | Event factory | 15 (23) | `CraftEventFactory.handleInventoryCloseEvent` 12 |
| 13 | Maps | 5 (5) | `CraftMapView`, `RenderData` |
| 14 | Scheduler | 4 (4) | `CraftScheduler` and `CraftTask` internals |
| 15 | Permissible injection | 3 (3) | `CraftHumanEntity.perm` 2, `CraftEntity.perm` 1, `ServerCommandSender.perm` and `blockPermInst` 1, the maps of `PermissibleBase` and `SimplePluginManager` |
| 16 | Scoreboards | 1 (2) | `CraftScoreboard` |
| free | Internals of the plugin manager and the loader | 85 (85) | `dependencyGraph` 55, `JavaPlugin.getFile` 15, the fields of `PluginClassLoader`, `RegisteredListener.executor`, `lookupNames`, `plugins` |
| free | Other internals of the API jar | 36 (36) | `Material.BY_NAME` 24, `Enchantment.acceptingNew` 4, `YamlConfiguration.yamlDumperOptions` 4 |

The two free rows live in the spigot-api jar and come with its stock classes.

### 4.3 What the Golem API offers behind each group

Most members of level 2 hand out an object of Minecraft: `CraftPlayer.getHandle` returns the
`ServerPlayer`, `asNMSCopy` a Minecraft `ItemStack`, `getMinecraftRegistry` the `RegistryAccess`,
`CraftServer.getServer` the `DedicatedServer`. What the plugins then do with that object is level 3.
So the stand-ins hand plugins the live objects of Forge, and level 2 pays off only once level 3
stands behind it. This is why the layer reaches the game beside the API as well as through it,
which [Plugin API](api.md#17-the-bukkit-layer) section 17 records.

1. **Entity and player handles (228).** `getHandle()` and the field `entity` return the live Forge
   `ServerPlayer` or `Entity`. The Golem API gives the layer the in-process Forge object of every
   entity, lookup by UUID and by network id, and spawn and removal events, so that exactly one
   wrapper lives as long as its entity. It also gives plugin message channels (`addChannel`) and a
   resend of the inventory (`updateInventory`). The hierarchy is real: `CraftPlayer` extends
   `CraftHumanEntity` extends `CraftLivingEntity` extends `CraftEntity`.
2. **Item stacks, item meta and persistent data (169).** `CraftItemStack` keeps its `handle` field
   and `asNMSCopy`, `asBukkitCopy`, `asCraftMirror` and `asCraftCopy`. A mirror shares the live
   Minecraft stack, so a change shows on both sides. `CraftMetaItem` keeps `displayName` and `lore`
   as JSON strings, as CraftBukkit 1.20.1 does. `CraftPersistentDataContainer` is backed by a
   `CompoundTag`, because NBT-API reads its raw map, and Golem's own persistent data lives on the
   same tags. The Golem API converts a Bukkit item and a Forge item in both directions with the
   full NBT of 1.20.1.
3. **Worlds, chunks, blocks and block data (161).** `CraftWorld.getHandle()` returns the
   `ServerLevel` and `CraftChunk.getHandle()` the `LevelChunk`. `CraftBlockData.getState()` and
   `fromData()` convert to and from `BlockState`. The Golem API keeps a map from each Bukkit world,
   by name and UUID, to its level, maps block data to block states for every block, modded ones
   included, reaches block entities, and spawns a prepared Minecraft entity with a Bukkit spawn
   reason.
4. **Command map, registration and help (140).** `CraftServer.commandMap` and `getCommandMap()`
   return a `CraftCommandMap` that extends `SimpleCommandMap`, and its `knownCommands` is live: a
   plugin that puts a command in or takes one out changes what players can run. The Golem API keeps
   one command registry that Forge's Brigadier dispatcher, Golem's own commands and the Bukkit
   command map all see, completes tab from it and resends the command tree to players
   (`syncCommands`). `VanillaCommandWrapper.getListener` returns a `CommandSourceStack` for a Bukkit
   sender, and `BukkitCommandWrapper` bridges Bukkit commands into Brigadier for cloud, Lamp,
   commodore and CommandAPI. The protected `PluginCommand` constructor and
   `SimpleHelpMap.helpTopics` come with the stock classes.
5. **Conversions (101).** `CraftMagicNumbers` with `getItem`, `getBlock`, `getMaterial`,
   `INSTANCE`, `getDataVersion` and `getMappingsVersion`, which return the values of Spigot 1.20.1
   because plugins compare them, and `CraftRegistry`, `CraftParticle`, `CraftSound`,
   `CraftNamespacedKey`, `CraftEnchantment`, `CraftPotionUtil` and `CraftAttributeMap`. The Golem
   API supplies the registries of the server under Bukkit keys and the `RegistryAccess`. The
   registries of Forge also hold modded blocks and items that the `Material` enum of spigot-api
   lacks, so the conversions need a rule for them.
6. **Server handle (84, and 64 class lookups).** `getServer()` returns the `MinecraftServer`, a
   `DedicatedServer`, `getHandle()` the `PlayerList`, and the fields `console`, `worlds` and
   `warningState` exist. The class sits in `org.bukkit.craftbukkit.v1_20_R1` with the version
   strings, data version and mappings version of Spigot 1.20.1, because plugins derive class names
   and adapters from them.
7. **Game profiles (76).** The `profile` fields of `CraftMetaSkull` and `CraftSkull` and the
   profiles of `CraftPlayer` and `CraftOfflinePlayer` hold an authlib `GameProfile`. The Golem API
   reads and writes skull owners with their texture property.
8. **Boss bars (56).** `CraftBossBar.handle` holds the `ServerBossEvent` behind each bar. Almost
   every reader is adventure-platform-bukkit, which sends that object's packets itself.
9. **Chat (31).** Static conversions between legacy text, JSON and `Component`, which the layer
   builds on Forge's `Component.Serializer` without new API.
10. **Inventories and menus (28).** The `Container` and the `AbstractContainerMenu` behind a Bukkit
    inventory and view, and the menu type of a view.
11. **Event factory (15).** `handleInventoryCloseEvent`, which AnvilGUI calls, and a long tail of
    single calls. The layer's own event factory keeps these names and fires the Bukkit events.
12. **Maps (5), scheduler (4), scoreboards (1) and the other classes (21).** The long tail, one or
    two plugins per class.
13. **Permissible injection (3: GroupManager, LuckPerms, PowerRanks).** `CraftHumanEntity.perm`,
    `CraftEntity.perm`, `ServerCommandSender.perm` and `blockPermInst` are non-final fields of type
    `PermissibleBase`, and the maps `permissions`, `permSubs` and `defaultPerms` of
    `PermissibleBase` and `SimplePluginManager` are the stock ones. Every permission check of a
    Bukkit sender goes through the object in that field. When a Bukkit permission plugin owns the
    decision, Golem's own checks in `golem-permissions` ask it too, or the server has two sources of
    truth, see section 13.

### 4.4 The plugins of the live server

- **LuckPerms 5.5.71** reaches `CraftHumanEntity.perm`, `CraftEntity.perm` and `getPermissibleBase`,
  `ServerCommandSender.perm` and `blockPermInst`, the maps `permissions`, `permSubs`,
  `defaultPerms`, `commandMap` and `dependencyGraph` of `SimplePluginManager`, and `attachments` and
  `permissions` of `PermissibleBase`. Nothing in it touches Minecraft. It needs the command map, the
  entity and the permissible groups and works once every item of section 9 down to the injection of
  permissibles stands.
- **EssentialsX 2.22.0** reaches `CraftServer.commandMap`, `syncCommands` and `knownCommands`,
  `dependencyGraph`, `CraftPlayer.getHandle`, `CraftEntity.getHandle`, `CraftItemStack`,
  `CraftWorld`, `CraftRegistry`, `CraftBossBar.handle` and `OfflinePlayer.setBanned` on the runtime
  class. Its own `net.ess3.nms.refl` code reaches `Language`, the Spigot `LocaleLanguage`, and
  `MinecraftServer`, and its legacy `net.minecraft.server.{version}` lookups must fail on 1.20.1 so
  that it falls back to the API. Its shaded adventure-platform-bukkit sends title, boss bar, book
  and tab list packets. It works once every item of section 9 down to the boss bars stands, so
  "every plugin works" is a statement about level 3 even for the plugins of this server.
- **EssentialsXSpawn 2.22.0** needs level 1 only and works once level 1 stands.

## 5. Level 3: the remapper

239 plugins reach classes of Minecraft on 1.20.1, 222 by reflection and 95 directly (first
analysis), and a typical one reaches most areas of the game at once. The classes already exist in
the JVM of Forge, under Mojang class names and SRG member names, so level 3 is a remapper and not a
set of stand-ins.

### 5.1 What the remapper does

1. **It rewrites plugin bytecode at load** from the Spigot 1.20.1 names to the names Forge 47
   runs, Mojang class names with SRG member names: `EntityPlayer.c` is `ServerPlayer.connection`,
   which Forge holds as `f_8906_`. Spigot renames 3,818 of the 5,045 classes and leaves most members
   obfuscated.
2. **It answers reflection in Spigot names.** `Class.forName`, `getDeclaredField`, `getMethod`,
   method handles and `getName` see Spigot 1.20.1 names, because 222 plugins reach Minecraft by
   reflection and build the names at run time.
3. **It knows the namespace of each class.** 10 class names mean different classes in Spigot and in
   Mojang names: `net.minecraft.core.Registry` is Mojang's `IdMap` in Spigot names and Mojang's
   `Registry` in Mojang names, and the same holds for `MobEffect`, `Fluid`, `ChunkMap`, `Path`,
   `WorldData`, `Position`, `ParticleType`, `TextFilter` and `ResourceManager`. 44 jars declare
   `paperweight-mappings-namespace: mojang` in their manifest and the reconciling pass finds both
   naming styles in 60 jars (`details.py`), so the remapper decides per class, from the manifest and
   the version tag.
4. **It maps Mojang member names as well.** Code written in Mojang names, such as DamageIndicator,
   ServerListPlus and the Mojang adapters of many plugins, finds Forge's class names but not its
   SRG member names.
5. **It covers overrides and classes defined at run time.** 40 plugins extend Minecraft classes, 13
   of them `ContainerAnvil` and 8 `PathfinderGoal` (first analysis), so their overriding methods
   need the Spigot member names mapped too. Big Doors, WolfyUtils, Denizen and TrainCarts define
   classes against Minecraft at run time.
6. **It keeps the pass of CraftBukkit.** The `processClass` of CraftBukkit rewrites the `Material`
   references of a plugin without `api-version` to the legacy materials, and the remapper runs that
   pass too, for the 6 plugins whose `plugin.yml` names no `api-version`.
7. **It reads class files up to Java 25**, since 9 plugins ship their main class for Java 25.

### 5.2 Faithful answers to probes

313 plugins have internal access on 1.20.1 (`library_reach.py`). 69 of them carry code for
`v1_20_R1` that runs (first analysis), and only 9 reach internals through that code alone (second
analysis): BedWars1058, BetterHud, CratesPlus, DamageIndicator, DiceFurniture, DualWield,
SilkSpawners, Staff+ and TerraformGenerator. The other 60 mix it with version independent code, and
244 carry no code for `v1_20_R1` at all. They splice the version into names at run time, use
thresholds such as "1.17 or later", or rely on shaded libraries that do. So the layer answers every
probe exactly as Spigot 1.20.1 answers it, and a wrong answer sends a plugin down a branch written
for another server:

- **Class not found** for legacy flat names such as `net.minecraft.server.v1_20_R1.DataWatcher`,
  for names of newer versions such as `ServerCommonPacketListenerImpl` of 1.20.2, and for the
  classes of Paper and Folia, such as `com.destroystokyo.paper.PaperConfig` and the classes of
  `io.papermc.paper.threadedregions`, while the layer serves no Paper.
- **No such method or field** for the Paper members of section 4.1, such as
  `CraftServer.getMinecraftVersion`, and for the Folia schedulers on the API types, which 107
  plugins carry code for.
- **The version of Spigot 1.20.1** from `Bukkit.getVersion`, `getBukkitVersion`, the package name of
  `CraftServer`, `CraftMagicNumbers.getDataVersion` and `getMappingsVersion`, since 194 plugins
  build internal class names from that package (first analysis).
- **The Spigot type of every lookup.** A field read by name has the CraftBukkit type, so a plugin
  that casts `CraftServer.console` to `DedicatedServer` or `CraftBossBar.handle` to
  `ServerBossEvent` gets that object.

### 5.3 The parts around the remapper

- **CraftBukkit's additions to Minecraft classes.** CraftBukkit patches members into Minecraft
  classes that Forge lacks. The leading ones by direct reference on 1.20.1:
  `EntityPlayer.getBukkitEntity` 24, `MinecraftServer.getServer` 24, `Entity.getBukkitEntity` 23,
  `EntityPlayer.nextContainerCounter` 21, `WorldServer.getWorld` 17, `World.getWorld` 16,
  `MinecraftServer.server` 9, `Container.getBukkitView` 8, `EntityHuman.getBukkitEntity` 8,
  `CommandListenerWrapper.getBukkitSender` 7 and `IRecipe.toBukkitRecipe` 7 (`details.py`). 78
  plugins reference such members directly (first analysis, an upper bound) and 116 when reflection
  by name counts (`order.py`). The layer adds them by Mixin or redirects the calls while it remaps.
- **The Netty pipeline of the player connection.** 29 plugins reach `Connection`, the Spigot
  `NetworkManager`, or `ServerConnectionListener` on 1.20.1, and packet libraries insert their
  handlers into the channel pipeline by name. ProtocolLib and PacketEvents are among them, and 60
  and 13 other plugins depend on them (`details.py`). The vanilla pipeline with its handler names
  stays as it is on Spigot.
- **Packets with readable fields.** Plugins build packets with their constructors and read and
  write their fields by name.

### 5.4 Areas, classes and shapes

Every area of the game is reached by 34 to 162 plugins, and a level 3 plugin reaches most of them,
so the areas are no separate units of work. The two analyses draw the borders differently:

| Area | First analysis | Second analysis |
| --- | ---: | ---: |
| Packets | 162 | 158 |
| Registries, holders and resource keys | 161 | 143 |
| Items and item stacks | 156 | 158 |
| Levels, chunks and blocks | 153 | 158 |
| NBT | 145 | 142 |
| Player connection and Netty | 141 | 149 |
| Chat components | 139 | 139 |
| Entities | 136 | 147 |
| The server and commands | 110 | 102 and 39 |
| Entity data (`SynchedEntityData`) | 88 | 88 |
| Sounds and particles | 75 | 71 |
| Menus and containers | 34 | 36 |

The most reached classes. "Written" counts plugins that name the class on a 1.20.1 path by a name
that exists in 1.20.1 (`minecraft_rank.py`). The analyses also count legacy names, descriptors and
the return types of calls, so their counts are mostly higher.

| Mojang name | Spigot 1.20.1 name | Written | First analysis | Second analysis |
| --- | --- | ---: | ---: | ---: |
| `ItemStack` | `ItemStack` | 133 | 153 | 150 |
| `CompoundTag` | `NBTTagCompound` | 124 | 139 | 136 |
| `ServerGamePacketListenerImpl` | `PlayerConnection` | 121 | 132 | 129 |
| `Component` | `IChatBaseComponent` | 116 | 129 | 129 |
| `Entity` | `Entity` | 125 | 128 | 129 |
| `ResourceLocation` | `MinecraftKey` | 104 | 127 | 128 |
| `ServerLevel` | `WorldServer` | 125 | 123 | 130 |
| `Packet` | `Packet` | 123 | 118 | 139 |
| `Level` | `World` | 118 | 113 | 126 |
| `ResourceKey` | `ResourceKey` | 116 | 113 | 117 |
| `Registry` | `IRegistry` | at least 88 | 111 | 111 |
| `ServerPlayer` | `EntityPlayer` | 82 | 108 | 99 |
| `BlockPos` | `BlockPosition` | 91 | 98 | 97 |
| `MinecraftServer` | `MinecraftServer` | 82 | 95 | 93 |
| `NbtIo` | `NBTCompressedStreamTools` | 80 | 90 | 86 |
| `SynchedEntityData` | `DataWatcher` | 40 | 87 | 87 |
| `ClientboundSetEntityDataPacket` | `PacketPlayOutEntityMetadata` | 38 | 87 | 85 |

The analyses count 1,583 (first analysis) and 1,809 (second analysis) distinct Minecraft classes on
1.20.1, and the written names that exist in 1.20.1 cover 1,270. By the second analysis, over the 222
plugins that reach these classes, the 430 most reached classes make half of them whole, 1,119 make
80 % whole and 1,682 make 95 % whole. So the remapper maps all of Minecraft and not a chosen part.

Five shapes carry most member references, counted directly (first analysis) and with reflection by
name (second analysis):

1. **Build a packet and send it** through the player's connection: `EntityPlayer.c`
   (`connection`) 57 plugins directly and 62 with reflection, `PlayerConnection.a(Packet)` (`send`)
   56 and 59, and the constructors of the entity data, destroy, teleport, title, tab list and
   custom payload packets.
2. **Read and write NBT on items:** `ItemStack.v` (`getTag`) 27, `setTag`,
   `NBTTagCompound.a(String, String)` (`putString`) 25, `contains`, `ItemStack.of`.
3. **Look up registries:** `new MinecraftKey` 97 with reflection, `ResourceKey.create`,
   `IRegistry.getOptional`, `wrapAsHolder`, `BuiltInRegistries`.
4. **Build components from JSON:** `ChatSerializer.a(String)` (`fromJson`) 29.
5. **Write entity data** for holograms, NPCs and disguises: `getDataWatcher`, the writes of
   `DataWatcher`, `setInvisible`, `setLocation`.

## 6. Shaded libraries that reach internals

Libraries are detected by their marker classes, even when relocated (`library_roots.py` and
`library_reach.py`). "Reaches on 1.20.1" counts carriers whose copy reaches CraftBukkit or Minecraft
in code that runs on 1.20.1, and "only internal access" counts plugins whose whole internal access
on 1.20.1 lies in that library.

| Library | Carriers | Reaches on 1.20.1 | Reaches Minecraft | Only internal access | What it reaches |
| --- | ---: | ---: | ---: | ---: | --- |
| adventure-platform-bukkit | 54 | 54 | 54 | 12 | `CraftPlayer.getHandle`, the connection's `send`, `CraftBossBar.handle`, `asNMSCopy`, `CraftRegistry`, `dependencyGraph`, chat, title, tab list, boss bar, book and sound packets, `LocaleLanguage` |
| XSeries | 40 | 26 | 6 | 5 | `CraftPlayer`, skull profiles, `Material.BY_NAME`, `CraftMagicNumbers`, title and particle packets |
| NBT-API | 29 | 22 | 22 | 2 | `CraftItemStack`, `CraftMetaItem`, `CraftPersistentDataContainer`, `CraftWorld`, `CraftEntity`, the whole NBT family, `NbtIo`, `MojangsonParser`, data fixers |
| cloud | 19 | 19 | 19 | 6 | `VanillaCommandWrapper`, `CommandDispatcher`, `CommandSourceStack`, the Brigadier argument types |
| AnvilGUI | 19 | 14 | 14 | 0 | `CraftEventFactory.handleInventoryCloseEvent`, `ContainerAnvil`, `ContainerAccess`, `PacketPlayOutOpenWindow` |
| ACF | 14 | 14 | 0 | 7 | `commandMap`, `knownCommands`, the player locale |
| Lamp | 11 | 11 | 11 | 1 | The command map and the Brigadier argument types |
| commodore | 9 | 9 | 9 | 0 | Brigadier registration through `CraftServer` and `BukkitCommandWrapper` |
| triumph-gui | 8 | 8 | 8 | 1 | Item meta and `NBTTagCompound` |
| FastBoard | 7 | 4 | 4 | 1 | The scoreboard packets |
| CommandAPI | 6 | 6 | 6 | 0 | The Brigadier dispatcher and about 130 Minecraft classes |
| PacketEvents | 6 | 6 | 6 | 1 | The connection, `ServerCommonPacketListenerImpl`, the Netty pipeline |

- 50 plugins reach internals on 1.20.1 only through shaded libraries: 12 through
  adventure-platform-bukkit alone, 7 through ACF, 6 through cloud, 5 through XSeries, 2 through
  NBT-API, 7 through another single library and 11 through several.
- 55 level 3 plugins reach Minecraft on 1.20.1 only inside shaded libraries, 31 of them through
  adventure-platform-bukkit, 12 through NBT-API, 8 through cloud, 4 through Lamp and 3 through
  PacketEvents. A stand-in that makes the lookups of one library succeed serves many plugins at
  once.
- **adventure-platform-bukkit is one cluster.** Once the remapper and the main stand-ins exist,
  `CraftBossBar.handle` is the last missing piece for 39 of its 54 carriers, EssentialsX among them.
- Libraries that many plugins carry but that reach no internals on 1.20.1: bStats (193 relocated
  copies, about 285 counting the single file copy of its `Metrics` class), FoliaLib 27,
  kotlin-stdlib 24, libby 17 and aikar-timings 15. PaperLib is carried by 41 and reaches internals
  in one copy. The Adventure API, its serializers and MiniMessage are carried by 54 to 83 plugins
  and reach almost nothing themselves.
- The plugins others depend on most, hard plus soft (`details.py`): PlaceholderAPI 4 plus 204,
  Vault 11 plus 130,
  WorldGuard 8 plus 104, ProtocolLib 9 plus 51, Essentials 1 plus 50, LuckPerms 5 plus 44,
  Multiverse-Core 4 plus 35 and WorldEdit 11 plus 34, all of them in the corpus. ProtocolLib is a
  heavy level 3 plugin, so its Netty and packet needs unlock a second ring of plugins that the scan
  sees only as depending on it.

## 7. Level 1: the public API

Counted at the declaring type (`public_api.py` and the first analysis), the corpus uses 4,983 of the
7,942 public methods (62.7 %), 3,621 of the 5,900 public fields and constants (61.4 %) and names
about 1,060 of the 1,247 types (1,057 by the count of the second analysis, 1,064 by
`public_api.py`). 4,010 methods are used by two or more plugins, 1,902 by ten or more and 606 by
fifty or more. Taking methods in order of use, the level 1 plugins need the 2,904 most used before
half of them are fully served, 4,401 for 90 % and 4,965 for all of them (first analysis), so level 1
is complete or nothing, and the coverage check of [API coverage](api-coverage.md) holds the Golem
API to all 7,942.

- **The most used methods** by plugins: `JavaPlugin.<init>` 494, `PluginManager.registerEvents` 473,
  `PlayerEvent.getPlayer` 458, `Permissible.hasPermission` 440, `CommandSender.sendMessage` 435,
  `Bukkit.getServer` 434, `Bukkit.getScheduler` 430, `PluginDescriptionFile.getVersion` 426. Plugin
  classes extend or implement `JavaPlugin` 494, `Listener` 479, `CommandExecutor` 260, `Event` 211,
  `BukkitRunnable` 194, `Cancellable` 193, `TabCompleter` 187 and `InventoryHolder` 113 (second
  analysis).
- **Events.** 476 plugins listen to events, 475 to an event of spigot-api. They listen to 231 of
  the 250 concrete events (92.4 %) and 233 of the 267 event types below `Event`. The most heard are
  PlayerJoinEvent 379, PlayerQuitEvent 326, InventoryClickEvent 245, PlayerInteractEvent 223,
  PlayerMoveEvent 194 and BlockBreakEvent 189. No plugin listens to 19 of them, among them
  BellRingEvent, PlayerPreLoginEvent, BroadcastMessageEvent and MapInitializeEvent. The rule of the
  plan to build Bukkit events only for the types a plugin listens to pays off most on
  PlayerMoveEvent and the inventory events.
- **Members spigot-api 1.20.1 lacks.** 329 plugins use 1,651 such members, 1,517 distinct names.
  The top ones are Folia and Paper API behind checks: `Entity.getScheduler` 107,
  `Server.getGlobalRegionScheduler` 90, `Entity.teleportAsync` 81, `Server.getAsyncScheduler` 81,
  `getRegionScheduler` 73, `World.getChunkAtAsync` 61, and the Adventure overloads of
  `sendMessage`, `ItemMeta.displayName` and `lore` with 55 to 61 each. On Spigot 1.20.1 these calls
  never run.

## 8. Needs outside the three levels

### 8.1 Java 21 and 25 class files

Forge 47 targets Java 17, and a JVM refuses a class file newer than itself with an
`UnsupportedClassVersionError`. By `jar_facts.py`, the main class of 87 plugins is newer than Java
17: 76 for Java 21, 9 for Java 25 and 2 for Java 18 or 19. 139 jars hold class files newer than
Java 17 somewhere, multi-release entries and code for other versions included. There are two ways
out:

1. **The server on Java 21**, which runs the 76 and the 2 and leaves the 9 of Java 25. Forge
   47.4.18 and the 265 mods of the pack are built and tested for Java 17, so whether the pack runs
   on Java 21 is a measurement with the pack, through the clean boot and the comparison runs of
   [Plan](plan.md#verification), and not an assumption. Java 25 for the remaining 9 is a second
   such measurement.
2. **A downgrade at load**: the remapper rewrites the class files of a plugin to Java 17 in the same
   `processClass` hook, so the server stays on Java 17. A downgrade also has to replace the classes
   and methods of the newer JDK that a plugin calls, which is a translation of its own.

Either way the remapper reads class files up to Java 25.

### 8.2 The api-version cases

`CraftMagicNumbers` of `v1_20_R1` supports the `api-version` values 1.13 to 1.20 and refuses the
rest with "Unsupported API version". 40 `plugin.yml` files carry a value it refuses
(`jar_facts.py` and `order.py`):

- 29 name a newer version: 1.21 (22), 1.21.3 (3), 1.21.10 (2), 1.20.5 (1) and 26.1 (1). One of them,
  Chunklock, writes its `plugin.yml` in UTF-16 with a byte order mark, which SnakeYAML reads.
- 11 write `api-version: 1.20` unquoted, which YAML reads as the number 1.2, so CraftBukkit 1.20.1
  refuses them as "Unsupported API version 1.2".

The layer's `checkSupported` accepts both and reads 1.2 as 1.20. A plugin that names a newer
version gets the API of 1.20.1, and its own version checks decide what it does with it. 6
`plugin.yml` files name no `api-version` (`details.py`) and load as legacy plugins, see section
5.1.

### 8.3 plugin.yml libraries

49 plugins list Maven libraries under `libraries` in their `plugin.yml` (`details.py`). The stock
`LibraryLoader` of spigot-api resolves them through Maven Resolver, which the server puts on its
class path, and downloads them from Maven Central when the plugin loads. The layer ships Maven
Resolver beside the stock classes, and the server reaches Maven Central or a mirror of it. Big Doors
loads ByteBuddy this way.

### 8.4 Paper

The JVM resolves a reference when the code runs, so a call of Paper API in a method body fails only
on that path, and a static scan cannot tell a guarded option from a hard need. The survey gives
three tiers:

- **Certain (12):** `paper-plugin.yml` without `plugin.yml`, which no Spigot loader loads: HeadDB,
  FancyNpcs, FancyHolograms, Bending, AdvancedServerList, AuthMeVelocity, Worlds,
  ArmorStandEditor-Reborn, akropolis, CommandControl, Tweaks and Networks.
- **Likely (4):** named by the estimates of both analyses: FeatherMorph, FreedomChat, Simple Voice
  Chat Discord Bridge and Typewriter.
- **Possible (40 more):** named by one estimate only. The rule of the second analysis, a Paper
  reference outside any Paper adapter class and no `Class.forName` guard anywhere, adds 20, some of
  them guarded at run time, such as the ACF command framework inside SpawnTP and Ultra SetHome. The
  rule of the first analysis, an author who lists only Paper, Folia or Purpur on Modrinth and own
  code that calls Paper API, adds 20 more. A listing names what the author tested and not what the
  code needs, so the list of the first analysis is the weaker one.

Beyond the tiers, 241 plugins call Paper API from the server on 1.20.1 paths, 172 of them from their
own code and the rest through shaded libraries, 157 use Adventure from the server and 128 listen to
a Paper event (both analyses), and 329 use `org.bukkit` members that spigot-api 1.20.1 lacks, mostly
of Paper and Folia, see section 7. On Spigot 1.20.1 those paths stay dark. A layer that offers Paper
lights them all, and one that offers part of Paper sends plugins down Paper paths it cannot serve,
so Paper is a tier of its own, all of it or none of it, see section 13.

## 9. What each work item completes

The work items stand in the order in which each completes the most plugins that the items above it
left, and a plugin counts as working when every item it needs is done (`order.py`). The counts
assume that guarded calls take their fallback path, and the five plugins of section 10 stay out.

| Work item | Plugins that need it | Completed by this item | Working | Share |
| --- | ---: | ---: | ---: | ---: |
| Level 1: spigot-api 1.20.1 unchanged, its events, the plugin loader, `CraftServer` in `v1_20_R1` with Spigot's version strings | 503 | 136 | 136 | 27.0 % |
| A Java 21 or 25 runtime for plugin classes, or a downgrade of their class files to Java 17 at load | 87 | 21 | 157 | 31.2 % |
| Accepting `api-version` above 1.20 and the misread 1.2 | 40 | 23 | 180 | 35.8 % |
| Level 2: command map, registration and help | 140 | 16 | 196 | 39.0 % |
| Level 2: entity and player handles | 228 | 21 | 217 | 43.1 % |
| Level 3: the remapper | 239 | 31 | 248 | 49.3 % |
| Level 2: game profiles | 76 | 17 | 265 | 52.7 % |
| Level 2: item stacks, item meta and persistent data | 169 | 17 | 282 | 56.1 % |
| Level 2: worlds, chunks, blocks and block data | 161 | 27 | 309 | 61.4 % |
| Level 2: server handle | 84 | 10 | 319 | 63.4 % |
| Level 2: conversions | 101 | 16 | 335 | 66.6 % |
| Level 3: CraftBukkit's additions to Minecraft classes | 116 | 18 | 353 | 70.2 % |
| Level 2: boss bars, all 39 completed carriers of adventure-platform-bukkit | 56 | 39 | 392 | 77.9 % |
| Level 2: other CraftBukkit classes | 21 | 17 | 409 | 81.3 % |
| Level 2: chat | 31 | 13 | 422 | 83.9 % |
| Level 3: the Netty pipeline | 29 | 18 | 440 | 87.5 % |
| Level 2: inventories and menus | 28 | 16 | 456 | 90.7 % |
| The Paper loader and Paper API for the 12 certain and 4 likely Paper plugins | 16 | 15 | 471 | 93.6 % |
| Level 2: event factory | 15 | 14 | 485 | 96.4 % |
| Level 2: maps | 5 | 5 | 490 | 97.4 % |
| Level 2: scheduler | 4 | 4 | 494 | 98.2 % |
| Level 2: permissible injection | 3 | 3 | 497 | 98.8 % |
| Level 2: scoreboards | 1 | 1 | 498 | 99.0 % |

EssentialsXSpawn works once level 1 stands, EssentialsX once every item down to the boss bars stands
and LuckPerms once every item down to the injection of permissibles stands.

- **The internals of the API jar cost nothing** when the stock classes of spigot-api run unchanged,
  which is why level 1 completes 136 plugins and not fewer. Of the 183 level 1 plugins, 9 still do
  not work once the items down to the `api-version` check stand, the 7 with a `paper-plugin.yml` and
  ClearInventory and Random Death Drop, while 6 plugins of level 2 whose internals the stock classes
  give work by then.
- **The Java runtime and the `api-version` check come before any internal.** Together they complete
  44 plugins that need nothing else.
- **Level 2 stand-ins and the remapper go together.** The handles complete few plugins on their own
  (21 with the entity and player handles), because what plugins do with a handle is level 3. Once
  the remapper stands, each stand-in group completes plugins whose Minecraft part already works.
- **Boss bars are one cluster.** The item of the boss bars completes 39 plugins at once, all of them
  carriers of adventure-platform-bukkit whose last missing piece is `CraftBossBar.handle`.
- **Permissible injection comes late by count, not by value.** It completes 3 plugins, but LuckPerms
  runs on this server today, and once the entity stand-ins exist its cost is small, see section 13.
- The first analysis gives the same shape, 135 plugins after level 1 and 498 after 24 items. The
  second analysis completes 165 plugins with level 1 because it counts neither the Java runtime nor
  the `api-version` check, then goes flat for fourteen items because it treats every area of
  Minecraft as a unit of its own, while one remapper serves all areas at once.

## 10. Plugins no layer runs identically

**Five plugins cannot run as Spigot 1.20.1 plugins on any layer**, since they fail on Spigot 1.20.1
too:

- **Nova** patches Minecraft classes at run time with `Instrumentation.redefineClasses` and ASM
  transformers in its `xyz.xenondevs.nova.transformer` package. It is also among the possible Paper
  needs.
- **GadgetsMenu** carries NMS packages for `v1_8_R3` up to `v1_19_R3` and from `v1_20_R3` on, but
  none for `v1_20_R1` or `v1_20_R2`. The reconciled flag `has_v1_20_R1` is set only because of an
  ambiguous tag `v1_20` on one class.
- **Phoenix Crates** carries NMS code only for `v1_20_R4` and later, and also writes
  `api-version: 1.20` unquoted.
- **ClearInventory** names `me.joeal.clearinventory.Metrics.ClearInventory` as its main class in
  `plugin.yml`, a class the jar lacks, so every loader fails with "Cannot find main class".
- **Random Death Drop** opens its `plugin.yml` with a stray scalar, `'PLUGIN'`, before the mapping,
  which YAML refuses, so SnakeYAML in Spigot rejects the descriptor.

A layer could run GadgetsMenu and Phoenix Crates only by posing as another version, and Random
Death Drop only by reading descriptors more leniently than Spigot, which is no longer identical.

**Uncertain (2).** BottledExp has adapters up to `v1_13_R2`, and its untagged `MerchantHandler` is
compiled against `org.bukkit.craftbukkit.v1_21_R3`, so its merchant feature fails on Spigot 1.20.1
while the rest may run. Enchantment Solution has one tagged adapter, `v1_16_R3`, and ten numbered,
untagged classes `AnvilGUI_1` to `AnvilGUI_10` in the package names of 1.17, and its author tested
it on 1.20, so it probably runs. The first analysis puts both under "cannot run".

**At risk, but servable (4).** Big Doors generates subclasses with ByteBuddy, WolfyUtils container
classes with javassist, Denizen its property and packet helpers with ASM, and TrainCarts its classes
through BKCommonLib's Mountiplex. They run if the remapper also covers classes defined at run time
and the names their generators look up. RPGItems generates ByteBuddy proxies of its own classes
only, and StormSzafka references `java.lang.instrument` from obfuscated code for an unknown purpose.
The reconciling pass flags 140 jars for patching (`details.py`), but the rest is library code that
never touches server classes, and no jar declares a Java agent in its manifest.

**No limits of the layer.**

- AuctionHouse, with an adapter only for `v1_20_R3`, ReviveMe, with `v1_13_R1` to `v1_18_R1`, and
  Morph, up to `v1_17_R1`, run on the API only on any 1.20.1 server, and their internal features are
  off on Spigot 1.20.1 as well.
- The 12 `paper-plugin.yml` plugins and the 40 plugins with a refused `api-version` do not load on
  Spigot 1.20.1, and the layer can load them.
- DamageIndicator and ServerListPlus use Mojang names. They fail on Spigot 1.20.1 and may run better
  on Golem once Mojang member names map to SRG.

## 11. Blind spots

- Version and platform checks inside methods are not evaluated, so every count on 1.20.1 is an
  upper bound, and a guarded Paper call looks the same as an unguarded one.
- Names built from configuration files or hidden by string encryption are invisible. StormSzafka
  and Player Auctions are obfuscated.
- The corpus holds the newest file of each plugin. Older files of the same plugins often target
  1.20.1 and Java 17 more closely.
- Download counts are not comparable across sites, and two mod-first Modrinth projects weigh 47 % of
  them.
- The list of CraftBukkit's additions to Minecraft classes is an upper bound. The check of
  CraftBukkit members against 1.20.1 uses Arclight's copy of CraftBukkit, which may differ from
  Spigot in a few places.
- Libraries are detected by marker classes, and a library that the shading tool minimised can lose
  its markers.
- The counts are of jars as they are and not of plugins at run time. A plugin that only one other
  plugin loads, an addon, needs that plugin too, which the scan sees only as a dependency.

## 12. Reproduce

The tools in `tools/plugin-survey/` rebuild the survey with the standard library of Python alone,
run with `python3 -I`, and treat every jar as untrusted data that they read in memory and never run.
Their `README.md` gives the commands and the hosts they reach.

1. `corpus.py <dir> live`, `spigot`, `modrinth`, `hangar`, `hangar_ext`, then `index` builds the
   corpus in `<dir>` and writes `corpus.json`. It reaches Spiget, Modrinth, Hangar, GitHub and the
   hosts that Hangar projects link their files to.
2. `scan.py --dir <dir> --spigot-api <jar>` writes one record per jar in the format of
   `reconciled.jsonl`. It needs the spigot-api jar above and `javap` of a JDK for the members the
   API inherits from the JDK.
3. `summary/run.py <dir> <out> --srg <bukkit_srg.srg>` runs the summary steps over the records and
   writes what each prints and hands on into `<out>`.

The reconciled data of 2026-11-06 adds to the records of `scan.py` the corrections of the
reconciling pass that compares them with a second scanner. The second scanner, the reconciling pass
and the two analyses are not in the repository, so the counts this page marks with the first or the
second analysis come from the survey folder of that day, and so does the check of CraftBukkit
members against Arclight's classes in section 4.1. Rerun on the reconciled data of 2026-11-06, the
summary steps give every other count on this page. Run on the records of `scan.py` alone, they give
the counts without the corrections of the reconciling pass.

## 13. Decisions for the owner

1. **Paper.** Whether the layer serves Paper plugins and the Paper API. Without Paper, the 12
   `paper-plugin.yml` plugins and the 4 likely hard needs do not run, and at most 40 more lose a
   feature or fail on a path, while every other plugin takes its Spigot path, as on Spigot 1.20.1.
   With Paper, the layer offers all of it, the plugin loader of `paper-plugin.yml`, Adventure from
   the server, the Paper events and the Folia schedulers, because a part of Paper turns the probes
   of section 5.2 into paths it cannot serve.
2. **Java 21.** Whether Java 21 class files run on the server on Java 21, which must be measured
   with the pack first, or through a downgrade at load that keeps the server on Java 17. Java 25
   class files need Java 25 or the downgrade either way.
3. **Permissible injection.** Whether it moves forward in the ranking of section 9, ahead of items
   that complete more plugins, because LuckPerms runs on the live server today. It completes 3
   plugins, and its cost is small once the stand-ins of the entity and player handles exist.
4. **Who decides a permission.** How Golem's own permission checks in `golem-permissions` ask a
   Bukkit permission plugin when one owns the decision, through the `PermissibleBase` in the `perm`
   field of the sender, so that the server keeps one source of truth.

Related: [Plugin API](api.md), [API coverage](api-coverage.md), [Plan](plan.md#bukkit-plugins),
[Decisions](decisions.md), [Permissions](plugins/permissions.md), [Index](index.md).
