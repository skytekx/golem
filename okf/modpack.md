---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Report
title: The SkyTekx 3 modpack under Golem
description: Whether golem run boots the real SkyTekx 3 modpack of the live server, every mod unchanged, as the java launcher and Arclight boot it, whether it opens the world of Arclight, what notebook 08 checks, the numbers it measured and every difference it found.
tags: [modpack, skytekx3, forge, arclight, mods, boot, tests]
timestamp: 2026-11-06
---

# The SkyTekx 3 modpack under Golem

Golem must run every mod of the live server unchanged. The notebook
`tests/notebooks/08_modpack_boot.ipynb` answers that for the pack of the server repository at the
commit that `tests/fetch.py` pins, and keeps the answer checked in CI. This page holds what it
checks, the numbers of its run and every difference between the java launcher, `golem run` and
Arclight that it found.

## The answer

Yes. `golem run` boots the 289 mods that the 265 jars of the pack hold, the jars nested in them
included, beside Minecraft and Forge, each in the same version as the java launcher, with no FATAL
line, no crash report, no ERROR line that java does not log too and the same warnings of the loader
as java. From the same seed it generates a world with the same spawn, the same datapacks and the
same folders of the dimensions, ticks at 20 TPS in the idle and stops with exit 0, every dimension
saved and no chunk that failed to write or read. It also opens the world that Arclight wrote, which
the switch of the live server needs. No mod of the pack names Bukkit, so none needs Arclight.
Arclight boots the same mods, with two conflicts of its own mixins with mods of the pack. Nothing
broke under golem, so nothing of Golem needed a fix.

The answer has limits, listed under [What is not checked](#what-is-not-checked). Most of all no
player joins, since the client of the harness is a vanilla client, which the pack refuses.

Golem runs Forge 47.4.18 unchanged, so it runs the Forge mods that run on Forge 47.4.18. It runs no
Bukkit plugin, since it has no Bukkit layer. The plugins in use on the live server, EssentialsX,
EssentialsXSpawn and LuckPerms, are ported to Golem plugins instead, see
[Plan](plan.md#plugin-port), and of those only the permissions exist so far, see
[Permissions](plugins/permissions.md).

## The pack

`python3 -I tests/fetch.py --only modpack` builds the pack in `tests/.cache/modpack` from commit
`3fa5ae681d12` of `skytekx/skytekx3-1201-server`, pack 3.1.3 for Minecraft 1.20.1 and Forge
47.4.18.

| Part | What it holds |
| --- | --- |
| `mods/` | The 216 jars of `modrinth.index.json` that the server requires, from `cdn.modrinth.com` and `github.com`, which sends a release on to `release-assets.githubusercontent.com`, over https alone and each checked by its size and its SHA-512, and the 49 jars of `local-mods/`. The 16 entries that the server does not support stay out |
| `config/`, `defaultconfigs/`, `kubejs/` | The configuration of the mods and the KubeJS scripts of the server |
| `world/datapacks/` | `skytekx3_fixes`, `skytekx3_skylands` and `skytekx3_skylands_forge` |
| `server.properties.example`, `systemd/skytekx3.service` | The settings of the live server and the options of its JVM |
| The Arclight jar, `arclight.conf`, `bukkit.yml`, `spigot.yml`, `commands.yml` | Arclight as the live server runs it, its jar checked by its SHA-256 |
| `libraries/` | The 31 libraries that the `installer.json` of the Arclight jar names, 29 from Maven Central and the two of Configurate from the repository of Sponge, each pinned by its SHA-256 and checked against the SHA-1 of `installer.json`, so Arclight downloads nothing at its start |
| `tools/patch_mods.py` | The patches of the live server for 14 jars, 11 of the index and Traverse, BiblioCraft and Simply Jetpacks 2 of `local-mods/` |
| `modpack.json` | The commit, every jar with its source and its SHA-512, and the Arclight jar with its SHA-256, written last |

The script takes every file from the commit itself, so a checkout that `--server-repo` names may
hold other work, and removes from the pack whatever the pin does not name, such as the Arclight jar
of an earlier pin. The jars stay as their authors published them, and the notebook patches its own
copies with `tools/patch_mods.py`, as the live server patches its `mods/`.

The pack is the one that the repository describes. It is not the `mods/` of the Mac mini jar for
jar, as the page of the mod set in the wiki of the server says. The live server moved Nostalgic
Tweaks out of `mods/` by hand, while the index still marks it as required on the server, and it
runs skytekx-compat 2.1.6, while `local-mods/` holds 2.1.3. Issue
[#102](https://github.com/skytekx/golem/issues/102) asks the repository to hold the live pack, so
that the pin can move to it.

## What the notebook checks

The notebook copies the Forge server of `tests/fetch.py` and the pack into a temporary directory and
boots it from a fresh world three times, then a fourth time under golem on the world of the third.
`server.properties` takes every setting of the live server, the level type with the Skylands
datapacks among them, but binds 127.0.0.1 on port 25677, runs offline with a view and simulation
distance of 4 and RCON off, and seeds the world with `golem`, so every boot starts from the same
seed. Every boot runs with a heap of 6 GiB.

| Boot | Started as | Stopped by |
| --- | --- | --- |
| java | `java @user_jvm_args.txt @.../unix_args.txt nogui`, as `run.sh` | `stop` on the console |
| golem | `golem run --host-jar golem-host.jar -- nogui`, with the bridge mod in `mods/` | `SIGTERM` |
| Arclight | `java -Xmx6G -jar arclight-forge-...-safeiter2.jar nogui`, as `ServerLinux.sh` | `stop` on the console |
| Golem on the world of Arclight | `golem run` on the copy that Arclight left, with the bridge mod added | `SIGTERM` |

| Section | What must hold |
| --- | --- |
| The pack | `modpack.json` names the pinned commit, and the 216 and 49 jars match their SHA-512 |
| The options of the live JVM | The 23 options of the systemd unit reach the launch that `golem run` reads, unchanged and in order |
| No mod needs Bukkit | The scan reads the 265 jars as `tools/patch_mods.py` patched them, and the jars nested in them. No class and no `mods.toml` names Bukkit, CraftBukkit, Spigot, BungeeCord, Paper or Mohist, and of the coremods, mixin configs and refmaps only three name one of them or Arclight: the coremod `exhaustion.js` of Twilight Forest names Arclight and Spigot in its comments alone, and the mixin config and the refmap of Supplementaries list its mixin for Arclight. Only two mixins of Supplementaries name Arclight, the annotation of each decides which of the two applies, only the agent entry point of the debug probes of Kotlin installs a handler of a signal through the Java API, and only spark and Simple Voice Chat ship native libraries |
| Each boot | `Done`, the answer of `forge tps` after the listeners of `ServerStartedEvent`, `forge mods`, the server list, a vanilla login, six samples of `forge tps`, one at every 10 s of a 60 s idle, under golem `golem source` and `golem status`, then the stop and its exit status |
| The same mods | Java loads every jar of the pack from `mods/`, but the jar of Kotlin for Forge, a library of jars of JarJar that `forge mods` does not list, and 289 mods beside Minecraft and Forge. Golem loads every mod of java in the same version from the same file, and only its bridge `golem` beside them. The server list names the same mods and marks `golem` as needed on the server alone. Arclight adds only `arclight` |
| Errors and warnings | No crash report and no FATAL line. Every ERROR line of the other boots also appears under java or stands in the list of known lines with its reason. The WARN lines of the loader, Forge loading, JarJar, the registries, ModLauncher and Mixin, are the same under golem as under java, but for the version check of Forge, which asks the network |
| Idle, stop and world | 19 TPS or more in the idle under java and golem, the same spawn and the same datapacks, with `mod:golem` and `mod:arclight` as the only additions, the same folders of the dimensions in the world, every dimension saved at the stop, where every boot logs the save of each level that java saves and writes `level.dat` after the command to stop, and no failure of the storage of the world under any boot, the 35 messages of 10 classes of Minecraft that the notebook pins against the jars of the server |
| Golem itself | Golem logs its banner and the four lines of a clean boot alone: the mod connected the game, the bridge connected, the stop on `SIGTERM` and the end. A panic of a plugin or a native, or a call over its budget, fails the boot |
| Golem on the world of Arclight | `Done`, the bridge connected, the lines of a clean boot of golem alone, the mods of the golem boot, the spawn of the world, no ERROR line that java does not log, and exit 0 |
| Arclight | It downloads no library, its two conflicts with mixins of the pack appear under Arclight and not under golem, its stop logs no line of the flush, and its two mixins of the save hold what makes it skip those lines |

The list of known ERROR lines holds the codec noise of new sculk sensors, which the wiki of the
server records as random during chunk generation. The race of the IOWorker of vanilla in a chunk
save, which the wiki records as random too, is no known line. A failed write leaves a truncated
entry in the region file, and vanilla still logs `All dimensions are saved` after it, so every
message with which Minecraft reports a failed read or write of a chunk, its entities, its points of
interest, `level.dat` or the saved data fails the check of the world under every boot, java and
Arclight included: `Failed to store chunk` and `Failed to read chunk` of the IOWorker, `Failed to
save chunk` and `Couldn't load chunk` of the chunk map, the damage that a region file finds and the
others that the notebook pins. It reads the strings of those 10 classes from the jars of the
server and fails when one holds a message that it does not pin. None of those lines stood among
the ERROR lines of the runs of the day.

## The numbers

The last run of the notebook on 2026-11-06 in the cloud container of the session, with 4 CPUs and
16 GB shared with another agent at a load average of 4 to 10, measured these. Each boot ran once.

| Measured | java | golem | Arclight | Golem on the world of Arclight |
| --- | ---: | ---: | ---: | ---: |
| `Done`, seconds from the start of the process | 156 | 128 | 213 | 174 |
| The line `Done`, seconds of the load of the worlds | 19.7 | 22.1 | 45.9 | 7.2 |
| Started, seconds to the first answer of the console | 177 | 148 | 250 | 193 |
| Mod files, Minecraft and Forge among them | 290 | 291 | 291 | 291 |
| Mods with their versions, Minecraft and Forge among them | 291 | 292 | 292 | 292 |
| Mods and channels in the server list | 291 and 188 | 292 and 188 | 292 and 188 | 292 and 188 |
| Jars that `tools/patch_mods.py` patched | 14 | 14 | 14 | Those of Arclight |
| FATAL lines | 0 | 0 | 0 | 0 |
| ERROR lines | 4 | 4 | 4 | 3 |
| WARN lines | 2,267 | 2,266 | 2,277 | 567 |
| WARN lines of the loader | 52 | 52 | 59 | 52 |
| Crash reports | 0 | 0 | 0 | 0 |
| TPS in the idle, mean and lowest of `forge tps` | 20.00, 20.00 | 20.00, 20.00 | 20.00, 20.00 | 20.00, 20.00 |
| MSPT in the idle, mean and highest of `forge tps` | 1.76, 2.76 | 1.60, 2.48 | 2.08, 3.61 | 1.75, 2.65 |
| TPS and MSPT in the idle, mean of `golem status` | None | 19.99, 1.96 | None | 20.00, 2.20 |
| Resident memory at the end of the idle, MiB | 7,543 | 7,562 | 7,562 | 7,124 |
| Seconds of the stop, and the exit status | 4, 0 | 3, 0 | 5, 0 | 4, 0 |
| Spawn | 160 79 -736 | 160 79 -736 | 160 79 -736 | 160 79 -736 |
| Datapacks enabled | 309 | 310 | 310 | 310 |

The four ERROR lines are the same under the first three boots: the client mixin of AdvancedAE for
`ModelBakery`, which Forge refuses on a dedicated server, the backup of the configuration of
Nostalgic Tweaks, the reflection of AllTheLeaks into Citadel, and the network registry of Forge,
which lists the channels that turn away the vanilla client of the notebook. The fourth boot lacks
the line of Nostalgic Tweaks, whose backup the boot before it left. Without the vanilla client and
without Nostalgic Tweaks, which the live server moved out, the two ERROR lines of the baseline of
the server wiki remain, the `ModelBakery` mixin of AdvancedAE and the line of AllTheLeaks, as the
page [Melting](https://github.com/skytekx/skytekx3-1201-server/blob/master/okf/content/melting.md)
of that wiki records them, and on the live server the warning of EssentialsX about the hybrid. A
first boot writes 1,705 WARN lines of the configuration of Forge, which fills in new files. The
fourth boot starts on the files that the third left, and Forge still corrects `ticex-common.toml`
there with two of those lines, as it does at every start of the live server: TiCEX defines
`avaritia.condensingDropProbability` with a default of the type Float, the parser of TOML reads the
0.003 of the file back as a Double, which the check of `define` refuses, so Forge logs that the file
is not correct and corrects the key from 0.003 to its default, 0.003, and writes the same value
again. The pack can fix it with a rule of `tools/patch_mods.py` that makes the default a Double, or
TiCEX upstream, see issue [#104](https://github.com/skytekx/golem/issues/104).

Golem reached `Done` before java in all six pairs of boots of the day: 128 against 156 seconds in
the last run of the notebook, 137 against 206, 158 against 180 and 125 against 150 in the runs
before, 128 against 142 in a boot by hand, and 111 against 133 in a pair where golem booted first.
Java with `-Xrs` reached `Done` after 170 seconds and without it after 173 in a pair of its own, so
the option that Golem adds is not the cause. The machine was shared, so these times are no
measurement of speed, and why golem boots sooner is not known.

An earlier run of the same day gave the boots the options of the systemd unit, Aikar's flags, with
the heap at 6 GiB and without `-Xms` and `-XX:+AlwaysPreTouch`. All three booted the pack with them
as well, `Done` after 150, 125 and 147 seconds, with the same ERROR lines and 20 TPS. That run
stopped at the check of the datapacks, which did not know `mod:golem` yet. The resident memory was
about 7.5 GiB with those options and without them, so the pack itself fills the heap.

## Every difference between java and golem

| Difference | Why | Effect |
| --- | --- | --- |
| The mod `golem` and its datapack `mod:golem` | The bridge is a Forge mod with a `pack.mcmeta`, as [Bridge](bridge.md) says | One more mod and one more datapack. The server list marks the mod as needed on the server alone, so a client without it joins |
| `-Xrs` and `golem-host.jar` on the module path | [JVM host](jvm-host.md#the-options-golem-adds) | No class of the pack installs a handler of a signal through the Java API but the agent entry point of Kotlin. The native async-profiler of spark installs a handler of `SIGPROF` and wraps the handler of `SIGSEGV` of the JVM when the background profiler of spark starts, at every boot. `-Xrs` leaves `SIGINT`, `SIGTERM`, `SIGHUP` and `SIGQUIT` alone to the host, and golem puts `SIGSEGV` and `SIGBUS` back to their default before it creates the JVM, so the JVM owns them as under java, and `-Xrs` takes nothing from a mod. Spark logs the same lines under java and golem. Spark reads the options of the JVM through `getInputArguments` for its report, so the report would list `-Xrs` and the hooks `exit` and `abort` beside the options of the server, which nobody ran under golem yet |
| `SIGTERM` stops the server | [JVM host](jvm-host.md#the-stop-path) | The stop of the pack took 3 seconds and saved every dimension, as `stop` on the console under java |
| The MSPT of `golem status` | The bridge times a tick from its listener of the highest priority on `ServerTickEvent` to its listener of the lowest priority, so the time covers most listeners of the mods at the end of the tick, which the clock of Minecraft that `forge tps` reads leaves out, see [Plugins](plugins/index.md#tick-statistics) | `golem status` reads about a fifth higher than `forge tps` in the idle, 1.96 against 1.60 ms |

Nothing else differed. The mods, their versions, the ERROR lines, the warnings of the loader, the
spawn, the datapacks, the folders of the world, the server list and the refusal of a vanilla
client were the same.

## Golem on the world of Arclight

The fourth boot ran golem on the world that Arclight had just written, as the switch of the live
server will. It reached `Done` with the 292 mods of the golem boot, connected the bridge, kept the
spawn, logged no ERROR line that java does not log, and stopped with exit 0 and every dimension
saved. Arclight keeps the layout of vanilla, so the data of the dimensions lies in the same 28
folders under every loader: `world/`, `world/DIM-1`, `world/DIM1` and
`world/dimensions/<namespace>/<name>`.

Golem logged 28 WARN lines on that world that it did not log on its own world, in the run of the
day. Two come from the switch: Forge names the mod that the world was saved with and that is gone,
`arclight (version 1.20.1-1.0.6-SNAPSHOT-6de9fec -> MISSING)`, and Minecraft drops the datapack
`mod:arclight`. One is the overload of the shared machine,
`Can't keep up! Is the server overloaded?`, and one names a loot function that Advanced Loot Info
met without a tooltip, `slimeknights.mantle.loot.function.SetFluidLootFunction`. The other 24 differ
from lines of the golem boot only in the order of a hash set, which changes from run to run:
AlmostUnified lists its invalid tags in another order, and in 23 lines Redux names another tag in
which it found a biome twice. The notebook sorts the items of a list and drops the tag of Redux
before it compares, and asserts what remains against the pinned list of those four lines, the two of
the switch required. The world that golem saved names `mod:golem` and no longer `mod:arclight` among
its datapacks.

## Every difference against Arclight

| Difference | Evidence | Under golem |
| --- | --- | --- |
| The redirect `arclight$mutePotion` of Arclight wins over `apoth_sunderingHasEffect` of Apothic Attributes in `LivingEntity`, so Sundering adds no damage, see issue [#99](https://github.com/skytekx/golem/issues/99) | The mixin WARN `@Redirect conflict. Skipping attributeslib.mixins.json:LivingEntityMixin` under Arclight alone | The mixin of Apothic Attributes applies, so Sundering adds its damage, as on plain Forge. skytekx-compat 2.1.6 on the live server adds that damage back on Arclight |
| The constant of the repair cost of the anvil that Arclight raises for Bukkit goes to Apotheosis, see issue [#100](https://github.com/skytekx/golem/issues/100) | The mixin WARN `@ModifyConstant conflict. Skipping mixins.arclight.core.json:world.inventory.RepairContainerMixin` under Arclight alone | No mixin of Arclight exists, and Apotheosis lifts the cap of the anvil as it does under Arclight |
| Arclight weaves its own mixins into Minecraft | Three more static binding WARNs of Mixin, two of Arclight and one of ModernFix, and WARNs of Arclight about level stems it does not know and about the test level of Supplementaries | None |
| The version in the server list reads `forge arclight/Trials 1.20.1` | The answer of the status | `1.20.1`, as vanilla |
| Arclight downloads the 31 libraries of its `installer.json` at the first start from `arclight.hypertention.cn` and `repo.spongepowered.org`, checked by SHA-1, see issue [#101](https://github.com/skytekx/golem/issues/101) | The `installer.json` and the class `Mirrors` of the jar. The notebook puts the libraries in place, so its boot downloads nothing | Golem downloads nothing at the start |
| The Bukkit layer applies `spigot.yml` and `bukkit.yml`: the activation and tracking ranges, the merge radius of items and experience, the spawn limits | The log of Arclight names them at the start, `Entity Activation Range: An 32 / Mo 32 / Ra 48 / Mi 0`, see [Plan](plan.md#spigot-world-settings) | The values of vanilla until the plugin `golem-world` ports them |
| At the stop Arclight logs the save of each of the 27 levels, but neither `All chunks are saved` for each level nor `All dimensions are saved`, which vanilla logs once it has flushed them, see issue [#103](https://github.com/skytekx/golem/issues/103) | The log of the stop, and the mixins of the save of Arclight, which the notebook reads with `javap`: `arclight$skipSave` cancels `saveAllChunks` of `MinecraftServer` at its call of `overworld()`, after the save of each level, so it skips the world border and the custom boss events in the data of the world, `level.dat` and both lines, and `arclight$saveLevelDat` does the first three at the end of the save of the overworld instead, as CraftBukkit does | Both lines, for the `save-all flush` and for the `stop`. The notebook checks the stop of every boot by the save of each level and a `level.dat` written after the command to stop as well, which stand for the lines under Arclight |
| The live server stops over RCON, since a signal starts the shutdown thread of Arclight beside the server thread | The systemd unit and issue [#4](https://github.com/skytekx/golem/issues/4) | `SIGTERM` stops through the server thread |

Supplementaries carries `CompatCreeperArclightMixin`, which it applies only when the creeper mixin
of Arclight is loaded, and its plain `CreeperMixin` otherwise, so it works without Arclight as it
is meant to.

## What is not checked

- No player joins. The pack needs a Forge client with its mods, and the client of the harness is a
  vanilla client, so the notebook checks the server list and the refusal alone.
- The pack runs for a minute of idle, with no pregen, no load and no play. The comparison of the
  [Plan](plan.md#verification) with 20 simulated players stays open.
- The pack is the one of the repository, not the `mods/` of the Mac mini, see
  [The pack](#the-pack).
- No Bukkit plugin runs under Arclight in the notebook, since the server repository tracks none,
  and the world of the fourth boot is a fresh one of Arclight, not the world of the live server.
- The terrain. The notebook compares the spawn, the datapacks and the folders of the dimensions of
  the worlds, not the chunks in them, so a change in the generation under golem, such as a mixin
  that applies otherwise or another order of the biome modifiers, would pass.
- Each boot ran once on a shared machine, so the times are single samples.

## Running it

```text
python3 -I tests/fetch.py
python3 -I tests/fetch.py --only modpack
tests/.venv/bin/python tests/run.py --only 08_modpack_boot
```

The run took 19 minutes, and a boot holds about 7.5 GiB of resident memory, so the notebook runs
one server at a time. The parameter `arclight = False` leaves out the boot under Arclight and the
boot on its world. CI fetches the pack after the plain fetch and caches it by the pinned commit,
saved right after the fetch, so a run whose notebooks fail keeps it. The notebook keeps the log and
the console of each boot in `tests/notebooks/out/08_modpack_boot/`.

Related: [JVM host](jvm-host.md), [Bridge](bridge.md), [Plugins](plugins/index.md),
[Plan](plan.md), [Index](index.md).
