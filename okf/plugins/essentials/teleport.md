---
# SPDX-License-Identifier: GPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
# SPDX-FileCopyrightText: 2011 Essentials Team
# SPDX-FileCopyrightText: EssentialsX contributors
type: Spec
title: EssentialsX teleport port
description: The teleport side of EssentialsX 2.22.0 and EssentialsXSpawn 2.22.0 as the Java code does it with the live config, command by command, with every message, config key, permission node, store and bridge need, and the quirks and security issues Golem fixes instead of copying.
tags: [plugins, essentials, essentialsx, spawn, teleport, tpa, home, warp, back, port, quirks]
timestamp: 2026-11-07
---

# EssentialsX teleport port

This page specifies the teleport side of the port of EssentialsX 2.22.0 and EssentialsXSpawn 2.22.0
to the Golem plugins `golem-essentials` and `golem-essentials-spawn`: the commands `tpa`, `tpahere`,
`tpaccept`, `tpdeny`, `tpacancel`, `tpauto`, `back`, `home`, `sethome`, `delhome`, `warp`,
`setwarp`, `delwarp`, `spawn` and `setspawn`, the command `tptoggle` that writes the flag the
requests read, the two listeners of EssentialsXSpawn, and the core they share.

The page translates the logic of EssentialsX and quotes its English texts, so it carries the
license of EssentialsX, the GNU GPL 3.0 or later, and names its holders beside the holder of Golem,
as the crates `golem-essentials` and `golem-essentials-spawn` do (their `src/lib.rs`, lines 1 to
19). Golem fixes the quirks and security issues of section 7 instead of copying them, as
[Plan](../../plan.md#quirks-and-security) says.

## 1 Sources and conventions

The source is the checkout of EssentialsX 2.22.0 at commit
`100f575293afff1c2bc8e1468ce1cffc153319fc`, the tag
[2.22.0](https://github.com/EssentialsX/Essentials/tree/2.22.0) of EssentialsX. Every citation is
`file:line` of that checkout. The file names are unique, so the directory is left
out. The section Sources at the end lists the sources outside the checkout and the citations that
are checked against it.

| Files | Directory |
| --- | --- |
| `AsyncTeleport.java`, `AsyncTimedTeleport.java`, `User.java`, `UserData.java`, `IUser.java`, `Warps.java`, `Settings.java`, `Trade.java`, `Essentials.java`, `I18n.java`, `EssentialsPlayerListener.java`, `EssentialsEntityListener.java`, `PlayerTarget.java`, `LocationTarget.java`, `PlayerExtension.java`, `Console.java` | `Essentials/src/main/java/com/earth2me/essentials/` |
| `Command<name>.java`, `EssentialsCommand.java`, `EssentialsToggleCommand.java`, `PlayerNotFoundException.java`, `WarpNotFoundException.java`, `NotEnoughArgumentsException.java` | `Essentials/src/main/java/com/earth2me/essentials/commands/` |
| `LocationUtil.java`, `StringUtil.java`, `NumberUtil.java`, `DateUtil.java`, `EnumUtil.java` | `Essentials/src/main/java/com/earth2me/essentials/utils/` |
| `UserConfigHolder.java`, `LazyLocation.java`, `LocationTypeSerializer.java`, `EssentialsConfiguration.java` | `Essentials/src/main/java/com/earth2me/essentials/config/` and its folders `holders/`, `entities/`, `serializers/` |
| `ModernUserMap.java`, `ModernUUIDCache.java` | `Essentials/src/main/java/com/earth2me/essentials/userstorage/` |
| `PermissionsHandler.java`, `PermissionsDefaults.java`, `SuperpermsHandler.java`, `AbstractVaultHandler.java`, `ModernVaultHandler.java`, `LuckPermsHandler.java` | `Essentials/src/main/java/com/earth2me/essentials/perm/` and `perm/impl/` |
| `SpigotAdventureFacet.java`, `AdventureUtil.java` | `Essentials/src/main/java/com/earth2me/essentials/adventure/` |
| `plugin.yml`, `config.yml`, `messages.properties` | `Essentials/src/main/resources/` |
| `Commandspawn.java`, `Commandsetspawn.java`, `SpawnStorage.java`, `EssentialsSpawn.java`, `EssentialsSpawnPlayerListener.java` | `EssentialsSpawn/src/main/java/com/earth2me/essentials/spawn/` |
| `spawn/plugin.yml` | `EssentialsSpawn/src/main/resources/plugin.yml` |

Some behaviour lives outside the checkout, in CraftBukkit, PaperLib, LuckPerms or vanilla
Minecraft. Such a fact is marked *outside the checkout* and needs a test against the live server
before the port relies on it. Section 9 lists them.

### 1.1 The live config

The live server runs the default `config.yml` of 2.22.0
([plugins.md](https://github.com/skytekx/skytekx3-1201-server/blob/3fa5ae681d12233fe81003fbfe5a2d3c529747fc/okf/modpack/plugins.md#L37)
line 37), so the defaults below are the live settings, with one exception: the default file sets
`respawn-listener-priority: high` (`config.yml:1273`) and `spawn-join-listener-priority: high`
(`config.yml:1279`), while plugins.md lines 89 to 91 say both are `none` on the live server. This
page follows plugins.md, so both listeners are off. The live file is not in any checkout on this
machine, so the two values are unverified (section 9).

### 1.2 Text of the messages

A message key below is quoted as its raw value in `messages.properties`. The port reads it as Java
does: the properties escapes `\:`, `\!` and `\n` become `:`, `!` and a line break, and `''`
becomes `'` (`I18n.java:88-98`, `java.text.MessageFormat`). The tags `<primary>` and `<secondary>`
are the colors `message-colors.primary: '#ffaa00'` and `message-colors.secondary: '#ff5555'`
(`config.yml:479-481`, `SpigotAdventureFacet.java:48-55`, `Settings.java:2222-2237`). The other
tags are MiniMessage (`<dark_red>`, `<yellow>`, `<gray>`, `<white>`, `<i>`, `<st>`, `<reset>`).

## 2 The live setting

### 2.1 Config keys of this part

Every key below is at its default on the live server. Keys that `config.yml` does not hold use the
default in the code.

| Key | Live value | Where set | Read at | What it does here |
| --- | --- | --- | --- | --- |
| `teleport-safety` | `true` | `config.yml:92` | `Settings.java:277-284` | Unsafe destinations are moved to the nearest safe spot |
| `force-disable-teleport-safety` | `false` | `config.yml:96` | `Settings.java:286-293` | With `teleport-safety`, skips every check. With or without it, it also turns off the clamp to the world border and `teleport-to-center`, and a safe target is reached by `Player.teleport` (`AsyncTeleport.java:184`, `192-194`, `209-211`) |
| `force-safe-teleport-location` | `false` | `config.yml:100` | `Settings.java:296-298` | Applies the safety search also to creative, spectator and god mode |
| `is-water-safe` | `false` | `config.yml:104` | `Settings.java:2074-2084`, `LocationUtil.java:88-94` | Water counts as a block the player cannot stand in |
| `teleport-passenger-dismount` | `true` | `config.yml:109` | `Settings.java:301-303` | Passengers are ejected before a teleport |
| `force-passenger-teleportation` | `false` (not in the file) | code only | `Settings.java:306-308` | Teleports with passengers |
| `teleport-cooldown` | `0` | `config.yml:112` | `Settings.java:527-529` | Seconds between two teleports |
| `teleport-delay` | `0` | `config.yml:116` | `Settings.java:311-313` | Warmup in seconds before a teleport |
| `teleport-invulnerability` | `4` | `config.yml:120` | `Settings.java:1501-1517` | Seconds without PvP damage after a command or plugin teleport and after a respawn |
| `teleport-to-center` | `true` | `config.yml:123` | `Settings.java:1776-1778` | x and z become the center of the block |
| `world-teleport-permissions` | `false` | `config.yml:629` | `Settings.java:1425-1427` | Needs `essentials.worlds.<world>` to cross worlds with tpa, tpahere, back |
| `world-home-permissions` | `false` | `config.yml:808` | `Settings.java:1430-1432` | The same for `/home` |
| `register-back-in-listener` | `false` | `config.yml:655` | `Settings.java:1444-1446` | Teleports of other plugins would set the back location |
| `per-warp-permission` | `false` | `config.yml:440` | `Settings.java:784-786` | Needs `essentials.warps.<warp>` per warp |
| `tpa-accept-cancellation` | `120` | `config.yml:757` | `Settings.java:1492-1494` | Seconds until a request expires, 0 for never |
| `tpa-max-requests` | `5` | `config.yml:761` | `Settings.java:1497-1499` | Pending requests per target |
| `sethome-multiple` | `default: 3`, `vip: 5`, `staff: 10` | `config.yml:822-825` | `Settings.java:196-223` | Home limits per set |
| `spawn-if-no-home` | `true` | `config.yml:835` | `Settings.java:2151-2153` | `/home` without homes goes to the bed or the world spawn |
| `confirm-home-overwrite` | `false` | `config.yml:838` | `Settings.java:2156-2158` | A second `/sethome` is needed to overwrite |
| `compass-towards-home-perm` | `false` | `config.yml:831` | `Settings.java:2038-2040`, `EssentialsPlayerListener.java:618-633` | The compass points at a home without a check of `essentials.home.compass` (section 5.8) |
| `update-bed-at-daytime` | `true` | `config.yml:803` | `Settings.java:191-193`, `EssentialsPlayerListener.java:970-985` | A right click on a bed that is not occupied, in a world of environment `NORMAL`, sets the bed spawn of a holder of `essentials.sethome.bed` without sleeping. The comment of `config.yml:801-802` says it has no effect on 1.15 and later, but from 1.15 on only the message `bedSet` is skipped (`EssentialsPlayerListener.java:979-982`). The node is `default: false` (`plugin.yml:1195-1197`), so nobody holds it, operators included, and nothing is ported for the live state |
| `command-costs` | empty | `config.yml:855-860` | `Settings.java:413-456` | Every cost is 0 |
| `min-money` | `-10000` | `config.yml:877` | `User.java:353-362` | Lowest balance with `essentials.eco.loan` |
| `safe-usermap-names` | `true` | `config.yml:773` | `Settings.java:2086-2093` | Names are looked up lowercased, other characters as `_` |
| `nickname-prefix` | `'~'` | `config.yml:34` | `Settings.java:508-518` | Prefix of nicknames in display names |
| `ops-name-color` | `'4'` | `config.yml:30` | `Settings.java:551-574` | Operators' display names start with `§4` |
| `hide-displayname-in-vanish` | `true` | `config.yml:60` | `User.java:587-590` | Vanished players show their account name |
| `change-displayname` | `true` | `config.yml:63` | `User.java:568-584` | Essentials sets the display name |
| `verbose-command-usages` | `true` | `config.yml:188` | `Essentials.java:833-847` | Long usage text on missing arguments |
| `disabled-commands` | empty | `config.yml:182-184` | `Essentials.java:788-795` | No command is disabled |
| `send-info-after-death` | `false` | `config.yml:571` | `Settings.java:2161-2163` | No death position message |
| `death-messages` | `true` | `config.yml:558` | `Settings.java:1377-1379` | The death handler keeps the vanilla death message, and false would clear it (`EssentialsEntityListener.java:174-176`, section 5.1) |
| `teleport-when-freed` | `back` | `config.yml:749` | `Settings.java:2010-2024` | A player freed from jail goes to their last location through `AsyncTeleport.back(future)`, the one reader of `lastlocation` besides `/back` (section 3.7). Jail itself is an admin command outside this page |
| `login-attack-delay` | `5` | `config.yml:659` | `Settings.java:1519-1526`, `EssentialsEntityListener.java:77-80` | For 5 seconds after their login, the hits of a player on another player are cancelled (section 3.5), beside the invulnerability |
| `world-change-fly-reset` | `true` | `config.yml:235` | `Settings.java:1975-1977` | A dimension change takes flight from a player in survival or adventure without `essentials.fly` (section 3.5) |
| `world-change-preserve-flying` | `true` | `config.yml:240` | `Settings.java:1980-1982` | A holder of `essentials.fly` gets their flight back after a dimension change |
| `world-change-speed-reset` | `true` | `config.yml:251` | `Settings.java:1990-1992` | A dimension change resets walk and fly speed |
| `max-walk-speed`, `max-fly-speed` | `0.8`, `0.8` | `config.yml:664`, `665` | `Settings.java:1545-1548`, `1539-1542` | The clamp of a holder of `essentials.speed` after a dimension change, unless they hold `essentials.speed.bypass` |
| `respawn-listener-priority` | `none` live, `high` in the file | `config.yml:1273` | `Settings.java:1457-1483`, `EssentialsSpawn.java:43-47` | The respawn listener is not registered |
| `spawn-join-listener-priority` | `none` live, `high` in the file | `config.yml:1279` | `Settings.java:1486-1489`, `EssentialsSpawn.java:49-53` | The join listener is not registered |
| `respawn-at-home`, `respawn-at-home-bed`, `respawn-at-anchor` | `false`, `true`, `false` | `config.yml:1282`, `1286`, `1289` | `EssentialsSpawnPlayerListener.java:44-69` | Only read by the respawn listener, which is off |
| `newbies.announce-format`, `newbies.spawnpoint`, `newbies.kit` | `'&dWelcome {DISPLAYNAME}&d to the server!'`, `newbies`, `tools` | `config.yml:1254`, `1259`, `1265` | `EssentialsSpawnPlayerListener.java:113-139` | Only read by the join listener, which is off |
| `random-spawn-location`, `random-respawn-location` | `"none"` | `config.yml:1296`, `1300` | `EssentialsSpawnPlayerListener.java:70`, `111` | Off |
| `spawn-on-join` | `false` | `config.yml:1303` | `EssentialsSpawnPlayerListener.java:86-104` | Off |

### 2.2 How a node is checked on the live server

- Every check goes through `User.isAuthorized` (`User.java:137-148`), which answers false for an
  offline player (`User.java:184-187`) and asks the permissions handler.
- The Mac mini has LuckPerms but no Vault. `LuckPermsHandler` and both Vault handlers need Vault
  (`LuckPermsHandler.java:106-118`, `ModernVaultHandler.java:26-28`,
  `AbstractVaultHandler.java:112-119`), so `PermissionsHandler.checkPermissions` picks
  `SuperpermsHandler` (`PermissionsHandler.java:175-197`), which accepts LuckPerms by name
  (`SuperpermsHandler.java:164-181`) and logs that it runs without Vault
  (`PermissionsHandler.java:223-229`).
- `SuperpermsHandler.hasPermission` emulates wildcards: it asks Bukkit for the node, then for
  `a.b.*`, `a.*` and `*`, and takes the first that is set or that `plugin.yml` denies to operators
  (`SuperpermsHandler.java:96-118`).
- LuckPerms answers a node that its data does not name with the Bukkit default of the node, which
  for a node that no plugin registered is "operators only" (*outside the checkout*, as
  [Permissions](../permissions.md#where-golem-differs-from-luckperms) of Golem notes). So operators
  hold every node that `plugin.yml` does not set to `default: false`, and the nodes with
  `default: true` are held by everyone.
- The group of a player for `/spawn` is `default` for everyone, operators included:
  `SuperpermsHandler.getGroup` answers null (`SuperpermsHandler.java:36-38`) and
  `PermissionsHandler.getGroup` replaces null with `default` (`PermissionsHandler.java:27`,
  `39-47`).

Golem's `golem-permissions` knows no plugin default and no operator (rule 8 of
[The rules of resolution](../permissions.md#the-rules-of-resolution) and
[Where Golem differs from LuckPerms](../permissions.md#where-golem-differs-from-luckperms)). The port therefore has to treat the four nodes with a
default of true as defaults of its own: `essentials.back.onteleport` (`plugin.yml:649-651`),
`essentials.back.into.<world>`, which Essentials registers with `PermissionDefault.TRUE` for every
loaded world (`PermissionsDefaults.java:17-33`, called at `Essentials.java:438-440` and on every
world load at `Essentials.java:1376-1378`), and `essentials.teleport.cooldown.bypass.tpa` and
`essentials.teleport.cooldown.bypass.back` (`plugin.yml:1561-1567`), or it drops the split of the
cooldown bypass per type. Only the parent `essentials.teleport.cooldown.bypass` has no default and
goes to operators. With a cooldown above 0, operators skip it for all three types and players for
none (`AsyncTeleport.java:85-102`).

### 2.3 Nodes of this part and the group default

The group `default` holds 39 nodes (plugins.md lines 64 to 69, seeded in Golem's
`config/permissions.toml`). Of them, 16 belong to this part.

| Node | Checked at | Held by the group `default` |
| --- | --- | --- |
| `essentials.tpa` | dispatcher (`Essentials.java:807`), `Commandtpaccept.java:93` | Yes |
| `essentials.tpahere` | dispatcher, `Commandtpaccept.java:89` | Yes |
| `essentials.tpaall` | `Commandtpaccept.java:89` (accepts a tpahere request in its place) | No |
| `essentials.tpaccept` | dispatcher, target of `Commandtpa.java:30`, `Commandtpahere.java:26` | Yes |
| `essentials.tpdeny` | dispatcher | Yes |
| `essentials.tpacancel` | dispatcher, hint of `Commandtpa.java:76` | Yes |
| `essentials.tpauto` | dispatcher | Yes |
| `essentials.tpauto.others` | `Commandtpauto.java:9`, `EssentialsToggleCommand.java:23`, `28` | No |
| `essentials.tptoggle`, `essentials.tptoggle.others` | dispatcher, `Commandtptoggle.java:9` | No, operators |
| `essentials.back` | dispatcher | Yes |
| `essentials.back.others` | `Commandback.java:20` | No |
| `essentials.back.ondeath` | `EssentialsEntityListener.java:170` (`default: false`, `plugin.yml:646-648`) | Yes |
| `essentials.back.onteleport` | `AsyncTeleport.java:179` (`default: true`, `plugin.yml:649-651`) | Everyone, through the plugin default |
| `essentials.back.into.<world>` | `Commandback.java:58` (registered true, `PermissionsDefaults.java:23-33`) | Everyone, through the registered default |
| `essentials.home` | dispatcher | Yes |
| `essentials.home.bed` | `Commandhome.java:45`, `79`, `102` | Yes |
| `essentials.home.others` | `Commandhome.java:34` | No |
| `essentials.home.compass` | `EssentialsPlayerListener.java:621`, only with `compass-towards-home-perm` (off) | No, nobody (`default: false`, `plugin.yml:888-890`) |
| `essentials.sethome` | dispatcher | Yes |
| `essentials.sethome.multiple` | `Settings.java:205` | Yes |
| `essentials.sethome.multiple.<set>` (`default`, `vip`, `staff`) | `Settings.java:209-216` | No, so the limit is 3 |
| `essentials.sethome.multiple.unlimited` | `Commandsethome.java:90`, `Commandhome.java:125` | No |
| `essentials.sethome.others` | `Commandsethome.java:39` | No |
| `essentials.sethome.bed` | `EssentialsPlayerListener.java:977` (`default: false`, `plugin.yml:1195-1197`) | No, nobody |
| `essentials.delhome` | dispatcher | Yes |
| `essentials.delhome.others` | `Commanddelhome.java:57` | No |
| `essentials.warp` | dispatcher | Yes |
| `essentials.warp.list` | `Commandwarp.java:31` | Yes |
| `essentials.warp.others`, `essentials.warp.otherplayers` | `Commandwarp.java:39` | No |
| `essentials.warps.<warp>` | `Commandwarp.java:95`, `104`, only with `per-warp-permission` | No (off) |
| `essentials.setwarp` | dispatcher | No |
| `essentials.warp.overwrite.<safe name>` | `Commandsetwarp.java:46` | No |
| `essentials.delwarp` | dispatcher | No |
| `essentials.spawn` | dispatcher (`EssentialsSpawn.java:76` with prefix `essentials.`) | Yes |
| `essentials.spawn.others` | `Commandspawn.java:28` | No |
| `essentials.setspawn` | dispatcher | No |
| `essentials.spawn-on-join.exempt` | `EssentialsSpawnPlayerListener.java:90` (`default: false`) | No, listener off |
| `essentials.teleport.timer.bypass` | `AsyncTeleport.java:285`, `335-337`, `378` | No |
| `essentials.teleport.timer.move` | `AsyncTimedTeleport.java:56` | No |
| `essentials.teleport.cooldown.bypass` | `AsyncTeleport.java:87-101` | No, operators (no default) |
| `essentials.teleport.cooldown.bypass.back`, `.tpa` | `AsyncTeleport.java:87-101` (`default: true`, `plugin.yml:1561-1567`) | Everyone, through the plugin default |
| `essentials.worlds.<world>` | `Commandtpa.java:36`, `Commandtpahere.java:32`, `Commandtpaccept.java:89`, `93`, `Commandback.java:54`, `Commandhome.java:139` | No, and the rule is off |
| `essentials.fly`, `essentials.speed`, `essentials.speed.bypass` | `EssentialsPlayerListener.java:531`, `745`, `912`, `919-929` | No, operators |
| `essentials.pvpdelay.exempt` | `EssentialsEntityListener.java:78` | No, operators |
| `essentials.vanish.interact` | `User.java:1014-1016`, name lookups | No |
| `essentials.chat.ignoreexempt` | `User.java:1096-1098` (`default: false`, `plugin.yml:706-708`) | No |
| `essentials.nocommandcost.all`, `essentials.nocommandcost.<command>` | `Trade.java:380` | No, and no cost is set |
| `essentials.eco.loan` | `User.java:358` | No |
| `essentials.jail.allow.<command>` | `Essentials.java:813` | No, only for jailed players |

## 3 The shared core

### 3.1 Command dispatch

`Essentials.onCommandEssentials` (`Essentials.java:728-866`) runs every command of both plugins.

1. A command of another plugin with the same label wins unless the label is overridden
   (`Essentials.java:730-751`). Golem has no other plugin, so this falls away.
2. A player who runs any Essentials command but `mail` itself and holds `essentials.mail` is told
   about unread mail, while `mail` is not disabled (`Essentials.java:777-779`). Not in this part.
3. A disabled command answers `commandDisabled` (`Essentials.java:788-795`). None is disabled.
4. A player without `essentials.<name>`, where `<name>` is the command's own name and not the alias
   (`User.java:128-135`), gets `noAccessCommand` and the log gets `deniedAccessCommand`
   (`Essentials.java:807-811`).
5. A jailed player needs `essentials.jail.allow.<name>` (`Essentials.java:813-820`).
6. The command runs. `EssentialsCommand.run` for a player checks the cost `Trade(<name>)`, runs, and
   charges it after the run, unless the command throws `NoChargeException`
   (`EssentialsCommand.java:168-173`, `Essentials.java:830-831`). Every teleporting command throws
   it and lets the teleport charge.
7. A command that only implements the player form answers the console with `onlyPlayers`
   (`EssentialsCommand.java:184-186`).
8. `NotEnoughArgumentsException` prints the usage (`Essentials.java:832-854`): with
   `verbose-command-usages` the lines `commandHelpLine1` with the label, `commandHelpLine2` with
   `<name>CommandDescription`, `commandHelpLine3`, and one `commandHelpLineUsage` per
   `<name>CommandUsage<n>` with `<command>` replaced by the label. Required arguments `<...>` are
   wrapped in `commandArgumentRequired`, optional ones `[...]` in `commandArgumentOptional`
   (`EssentialsCommand.java:44`, `51-75`). A command without usage keys, as `spawn` and `setspawn`,
   prints the `description` and `usage` of `plugin.yml` instead (`Essentials.java:845-846`).
9. Any other exception becomes `errorWithMessage` (`Essentials.java:855-861`, `891-901`): a
   `TranslatableException` as its translated text, any other exception as its raw message. Every
   error key of this page therefore reaches the player as `Error: <text>`.

| Key | Line | Text |
| --- | --- | --- |
| `noAccessCommand` | 857 | `<dark_red>You do not have access to that command.` |
| `deniedAccessCommand` | 225 | `<secondary>{0} <dark_red>was denied access to command.` (log) |
| `onlyPlayers` | 909 | `<dark_red>Only in-game players can use <secondary>{0}<dark_red>.` |
| `commandDisabled` | 150 | `<secondary>The command<primary> {0}<secondary> is disabled.` |
| `errorWithMessage` | 347 | `<secondary>Error\:<dark_red> {0}` |
| `commandHelpLine1` | 153 | `<primary>Command Help\: <white>/{0}` |
| `commandHelpLine2` | 154 | `<primary>Description\: <white>{0}` |
| `commandHelpLine3` | 155 | `<primary>Usage(s);` |
| `commandHelpLineUsage` | 157 | `{0} <primary>- {1}` |
| `commandArgumentRequired`, `commandArgumentOptional`, `commandArgumentOr` | 148, 146, 147 | `<yellow>`, `<gray>`, `<secondary>` |
| `playerNotFound` | 950 | `<dark_red>Player not found.` |
| `notEnoughMoney` | 897 | `<dark_red>You do not have sufficient funds.` |

### 3.2 Messages and their placeholders

- `sendTl` translates the key, skips a text that is blank, and parses it as MiniMessage
  (`User.java:1127-1135`).
- Each placeholder that is not a number or date is escaped for MiniMessage tags and then converted
  from legacy `§` codes to MiniMessage (`I18n.java:171-193`), so a `§` code in a name becomes a
  color, but a tag in a name stays text. Numbers are left to `MessageFormat` (`I18n.java:196-209`),
  which groups digits, so 1234 prints as `1,234`.
- A placeholder wrapped in `AdventureUtil.parsed` is inserted as MiniMessage without escaping
  (`I18n.java:187-189`, `AdventureUtil.java:46-61`). In this part that is the list of homes
  (`Commandhome.java:109`) and the text inside `errorWithMessage` (`Essentials.java:894`).
- Several calls pass placeholders that the English text does not use, for example `teleporting`
  gets the world and the block position (`AsyncTeleport.java:130`) and prints only
  `Teleporting...`.

### 3.3 Player matching

The commands find a player through the variants of `EssentialsCommand.getPlayer`
(`EssentialsCommand.java:113-165`), which all end in `Essentials.matchUser`
(`Essentials.java:985-1064`).

| Variant | Lines | Hidden players | Offline players | Used by |
| --- | --- | --- | --- | --- |
| `getPlayer(server, user, args, pos)` | `EssentialsCommand.java:135-141` | Only with `essentials.vanish.interact` | No | `tpa`, `tpahere`, `tpaccept <name>`, `tpdeny <name>`, `tpacancel <name>`, `warp <warp> <player>`, `spawn <player>` |
| `getPlayer(server, args, pos, true, false)` | `EssentialsCommand.java:144-146` | Yes | No | `back <player>`, console `warp`, console `spawn` |
| `getPlayer(server, args, pos, true, true)` | same | Yes | Yes | `home <player>:`, `delhome <player>:` |
| `getPlayer(server, term, true, true)` | `EssentialsCommand.java:159-161` | Yes | Yes | `sethome <player>:` |
| `server.matchPlayer(term)` directly | `EssentialsToggleCommand.java:44-68` | Skipped without `vanish.interact` | No | `tpauto <player>`, `tptoggle <player>` |

`getPlayer(..., args, pos, ...)` throws `NotEnoughArgumentsException` when the argument is missing
and `playerNotFound` when it is empty (`EssentialsCommand.java:148-156`). `matchUser` then does:

1. `@p` or `@s` with a player as source answers that player (`Essentials.java:989-991`).
2. A term that parses as a UUID is looked up among the online players (`Essentials.java:993-994`).
3. Otherwise `server.getPlayerExact(term)`, and when that finds nobody and offline players are not
   wanted, `server.getPlayer(term)` (`Essentials.java:995-1007`). *Outside the checkout*:
   `getPlayerExact` compares names without case, and `getPlayer(String)` answers the online player
   whose name starts with the term, without case, preferring the shortest name.
4. With no online match, the usermap is asked for the term (`Essentials.java:1009-1013`): the name
   is sanitized to lower case with every character outside `a-z0-9` as `_`
   (`ModernUUIDCache.java:70-88`, `StringUtil.java:37-42`, `safe-usermap-names`) and mapped to the
   UUID of `usermap.bin` (`ModernUUIDCache.java:56`, `ModernUserMap.java:116-131`).
5. A found user who is offline when offline is not wanted is `playerNotFound`
   (`Essentials.java:1016-1018`). A found user the source may see is the answer. An offline lookup
   with the exact name answers even a hidden user (`Essentials.java:1020-1027`).
6. With nobody found yet, `server.matchPlayer(term)` lists the online players whose name contains
   the term, without case, or only the one online player whose name equals the term without case
   when there is one (*outside the checkout*, as the javadoc of `Server.matchPlayer` says,
   `org/bukkit/Server.java:538-549` of the Spigot API 1.20.1 sources). If that list is empty, the
   first online player the source may see whose display name, with formatting stripped and
   lowercased, contains the lowercased term is the answer. This is the only step that matches
   nicknames (`Essentials.java:1029-1040`). If the list is not empty, an exact name wins, then a
   display name that starts with the term (case sensitive), then the first of the list
   (`Essentials.java:1041-1061`).
7. Otherwise `playerNotFound` (`Essentials.java:1063`).

Whether the source may see a player is `canInteractWith`: the console sees everyone, a player sees
themselves and everyone their client can see, and a missing source sees everyone who is not
vanished (`Essentials.java:1067-1090`). Golem has no vanish, so the port can drop this.

### 3.4 Display names

Messages show `User.getDisplayName`, which is the Bukkit display name, or the account name for a
vanished player (`User.java:587-590`). With `change-displayname`, Essentials sets it on join and on
respawn (`User.java:568-584`, `EssentialsPlayerListener.java:184`) to `getNick`
(`User.java:510-566`): the account name, or `nickname-prefix` (`~`) plus the nickname unless the
player holds `essentials.nick.hideprefix`. For an operator `ops-name-color` (`§4`, dark red) comes
in front and `§r` behind. Prefixes and suffixes of the permission plugin are off
(`add-prefix-suffix` false, `Settings.java:1247-1249`, and `SuperpermsHandler.getPrefix` answers
null, `SuperpermsHandler.java:141-143`). Nicknames need `essentials.nick`, which the group
`default` does not hold. The tpa commands compare and key requests by the account name, not the
display name (`Commandtpa.java:27`, `User.java:407`).

### 3.5 The teleport pipeline

Every player has one `AsyncTeleport` (`User.java:65`, `109`), its owner. A teleport has an owner,
whose cooldown, warmup timer, bypass nodes and money count, and a teleportee, who moves.

#### Entry points

| Method | Lines | Warmup and cooldown | Messages of its own | Used by |
| --- | --- | --- | --- | --- |
| `now(Location, cooldown, cause)` | `AsyncTeleport.java:111-118` | Cooldown only when asked, no warmup, no cost | None | console `spawn` (`Commandspawn.java:84`), new player spawn |
| `now(Player, cooldown, cause)` | `AsyncTeleport.java:120-133` | Cooldown only when asked, no warmup, no cost | `teleporting` to the owner on success | admin `tpo` (`Commandtpo.java:28`) |
| `teleport(Location, charge, cause)` | `AsyncTeleport.java:227-230` | Both, owner is teleportee | None | `home`, `home bed`, `spawn-if-no-home` |
| `teleport(Player, charge, cause)` | `AsyncTeleport.java:232-236` | Both | `teleportToPlayer` to the owner, before anything else | `tpa` with tpauto, `tpaccept` of a tpa |
| `teleportPlayer(IUser, Location, charge, cause)` | `AsyncTeleport.java:238-241` | Both, owner's cooldown, teleportee moves | None | `tpaccept` of a tpahere, `spawn` |
| `warp(IUser, name, charge, cause)` | `AsyncTeleport.java:414-440` | Both | `warpingTo` on success | `warp` |
| `back(charge)`, `back(teleporter, charge)` | `AsyncTeleport.java:442-453` | `teleportOther` | `backUsageMsg` first | `back` |
| `back(future)` | `AsyncTeleport.java:455-458` | None: a bare `nowAsync` to the last location, no type, no cost | None | release from jail with `teleport-when-freed: back` (`User.java:790-796`, `Commandtogglejail.java:153-159`) |
| `respawn(charge, cause)` | `AsyncTeleport.java:358-393` | Both | None | `home` with the bed and no home |

#### A teleport with warmup (`AsyncTeleport.teleport`, private)

`AsyncTeleport.java:255-304`, and `teleportOther` at `306-356` the same way with a teleporter who
may be null.

1. The delay is `teleport-delay`, 0 (`AsyncTeleport.java:256`).
2. `TeleportWarmupEvent` fires. A cancelled one answers false (`AsyncTeleport.java:258-264`).
3. With a cost, the owner must afford it (`AsyncTeleport.java:268-272`, `Trade.java:204-228`), and
   a command cost becomes a plain money cost so that the target world's rules do not change it
   (`AsyncTeleport.java:274-278`). All costs are 0.
4. The cooldown is checked without updating it (`AsyncTeleport.java:281-284`).
5. With no delay or `essentials.teleport.timer.bypass` on owner or teleportee, the cooldown is
   stamped, `nowAsync` runs, the cost is charged and the future answers true
   (`AsyncTeleport.java:285-299`). The live server always takes this branch. The charge does not
   look at what `nowAsync` did: a move that `PreTeleportEvent` cancelled (the future answers false)
   or that failed with `passengerTeleportFail`, `unsafeTeleportDestination` or an error of the
   safety search (the future fails) is charged all the same, since `Trade.charge` only checks that
   the owner can pay (`AsyncTeleport.java:160-162`, `167`, `200`, `205`, `290-297`, `342-348`,
   `Trade.java:295-327`). All costs are 0 live. See Q36.
6. Otherwise a running warmup is cancelled, the teleportee gets `dontMoveMessage` with the delay,
   and `AsyncTimedTeleport` starts (`AsyncTeleport.java:301-303`).

`teleportOther` differs: a null teleporter, the console, skips the delay and the cost
(`AsyncTeleport.java:334-347`), and a cancelled warmup event leaves the future open
(`AsyncTeleport.java:311-313`).

#### The move itself (`nowAsync`)

`AsyncTeleport.java:155-225`:

1. A pending warmup of the owner is cancelled without a message (`AsyncTeleport.java:156`).
2. `PreTeleportEvent` fires. A cancelled one answers false (`AsyncTeleport.java:158-163`).
3. A teleportee with passengers fails with `passengerTeleportFail` unless
   `teleport-passenger-dismount`, which ejects them on the server thread
   (`AsyncTeleport.java:165-177`).
4. A teleportee with `essentials.back.onteleport`, everyone, gets their current position as last
   location (`AsyncTeleport.java:179-181`, `User.java:395-398`, `UserData.java:284-290`).
5. With `teleport-safety` on and `force-disable-teleport-safety` off, a target outside the world
   border is clamped to its edge (`AsyncTeleport.java:183-187`, `LocationUtil.java:120-150`).
6. The chunk of the target is loaded and generated if missing (`AsyncTeleport.java:188`,
   `PaperLib.getChunkAtAsync(world, x, z, true, true)`). *Outside the checkout*: without Paper,
   which Arclight is, PaperLib loads it synchronously on the server thread.
7. If the block is unsafe for the teleportee (`LocationUtil.isBlockUnsafeForUser`), there are three
   cases. With `teleport-safety` and `force-disable-teleport-safety` it teleports there anyway.
   With `teleport-safety` alone, the live case, it teleports to `LocationUtil.getSafeDestination`,
   and an exception of the search such as `holeInFloor` fails the future. Without
   `teleport-safety` it fails with `unsafeTeleportDestination` (`AsyncTeleport.java:190-207`).
8. If the block is safe, with `force-disable-teleport-safety` the player is moved there by
   `Player.teleport`, not centered (`AsyncTeleport.java:209-211`). Otherwise, with
   `teleport-to-center`, the target becomes x + 0.5, round(y), z + 0.5 with its yaw and pitch
   (`AsyncTeleport.java:212-218`, `LocationUtil.java:187-193`).
9. The future answers true (`AsyncTeleport.java:220`) right after the call of
   `PaperLib.teleportAsync` or `Player.teleport`, whose own result is dropped
   (`AsyncTeleport.java:191-220`). So `teleportHome`, `warpingTo`, `teleporting`, `teleportAtoB`,
   `requestAcceptedAuto` and the charge follow also when a listener cancels Bukkit's
   `PlayerTeleportEvent` or the move fails (Q31). The `teleported` event of section 8.3 fixes this.

The move is `PaperLib.teleportAsync(player, loc, cause)`, *outside the checkout* Bukkit's
`Player.teleport` on a server without Paper. Bukkit then fires `PlayerTeleportEvent`, and
Essentials' own listener (`EssentialsPlayerListener.java:728-755`) reacts for the causes `PLUGIN`
and `COMMAND` in three ways. With `register-back-in-listener` (off) it would set the last location.
With `teleport-invulnerability` it starts 4 seconds of invulnerability (`User.java:995-1012`). For
holders of `essentials.fly` it keeps the flight state across worlds. While a player is
invulnerable, PvP damage from or to them is cancelled (`EssentialsEntityListener.java:81-83`). A
respawn starts the same 4 seconds (`EssentialsPlayerListener.java:180-189`).

Beside the invulnerability, for `login-attack-delay` (5) seconds after their login the hits of a
player on another player are cancelled unless the player holds `essentials.pvpdelay.exempt`, which
has no default and so goes to operators (`EssentialsEntityListener.java:77-80`,
`plugin.yml:1129-1130`). No other part of the port covers it. It needs the same cancellable attack
event as the invulnerability after a teleport (section 8.3), and the same decision of the owner
(Q27).

A `PlayerTarget`, the target of tpa, reads the target player's position when `nowAsync` runs, not
when the request is made (`PlayerTarget.java:12-19`, `AsyncTeleport.java:183`).

#### A dimension change

Every teleport of this page into another dimension (home, warp, back, spawn, tpa or tpahere to a
player there), and every portal, fires `PlayerChangedWorldEvent`, and Essentials then does three
things (`EssentialsPlayerListener.java:905-943`):

- `world-change-fly-reset: true` (`config.yml:235`): a player in survival or adventure without
  `essentials.fly` gets fall distance 0 and loses the ability to fly.
- `world-change-speed-reset: true` (`config.yml:251`): a player without `essentials.speed` gets
  walk speed 0.2 and fly speed 0.1. A holder is clamped to `max-walk-speed` and `max-fly-speed`
  (`config.yml:664-665`) unless they hold `essentials.speed.bypass`.
- `world-change-preserve-flying: true` (`config.yml:240`): a holder of `essentials.fly` gets their
  flight back after the change (`EssentialsPlayerListener.java:742-753`, `935-943`).

The join flow resets the speed in the same way (`EssentialsPlayerListener.java:531-534`). The
modpack has mods that give flight through the player's abilities, for example Doom Angel Ring
3.0.1 (`modrinth.index.json` of the server checkout). Whether each restores its flight after the
reset is open (section 9). See Q33.

#### Cooldown

`AsyncTeleport.java:50-102`. The owner's `timestamps.lastteleport` (`UserData.java:305-312`) is
compared with now minus `teleport-cooldown`. A stamp in the future is reset to now and passes
(`AsyncTeleport.java:64-68`). A stamp inside the cooldown fails with `timeBeforeTeleport` and the
remaining time (`AsyncTeleport.java:69-76`), unless the owner holds
`essentials.teleport.cooldown.bypass`, and for the types `BACK` and `TPA` also
`essentials.teleport.cooldown.bypass.back` or `.tpa` (`AsyncTeleport.java:85-102`). The two nodes
of the types are `default: true` (`plugin.yml:1561-1567`), so everyone holds them and the parent
alone decides (section 2.2). Every teleport that passes stamps the time and saves the player file
(`AsyncTeleport.java:79-81`), also with a cooldown of 0. The type is a field of the owner's
`AsyncTeleport` that `back` sets to `BACK` (`AsyncTeleport.java:449`) and `tpa` and `tpaccept` set
to `TPA` (`Commandtpa.java:48`, `Commandtpaccept.java:117`, `126`), and nothing sets it back to
`NORMAL`.

#### Warmup timer (`AsyncTimedTeleport`)

Not used on the live server, since `teleport-delay` is 0, but part of the port.

- It records the teleportee's health and position, each coordinate as `round(c * 0.3)`, and whether
  the owner holds `essentials.teleport.timer.move` (`AsyncTimedTeleport.java:15`, `41-56`).
- It runs every 20 ticks on an async thread, first after 20 ticks (`AsyncTimedTeleport.java:58`).
- It cancels without a message when the owner or teleportee is offline
  (`AsyncTimedTeleport.java:76-92`). It cancels with `pendingTeleportCancelled` to owner and
  teleportee when, without the move node, a rounded coordinate changed or the health dropped
  (`AsyncTimedTeleport.java:94-98`, `163-168`). Either way it fires `TeleportWarmupCancelledEvent`
  (`AsyncTimedTeleport.java:156-161`).
- On the server thread it refreshes the health, and once the delay is over stamps the cooldown,
  sends `teleportationCommencing` to the teleportee, checks the cost again, runs `nowAsync` or
  `respawnNow`, and charges when the future completes (`AsyncTimedTeleport.java:100-145`). A
  cooldown that fails at this point only sends `cooldownWithMessage` to both and falls through to
  the teleport, so with a warmup the cooldown is a message, not a stop
  (`AsyncTimedTeleport.java:107-114`, `115-127`). See Q35. The charge runs on every normal
  completion of the future, so also when `PreTeleportEvent` cancelled the move, but not when the
  move failed with an error (`AsyncTimedTeleport.java:128-136`).

#### Safety (`LocationUtil`)

- A player can stand in the hollow materials: every `Material` that Bukkit calls transparent,
  minus barrier and farmland, plus light, and water only with `is-water-safe`
  (`LocationUtil.java:40-70`, `88-94`). The code also means to remove the path, but looks it up by
  the old name `GRASS_PATH` (`LocationUtil.java:36`). `EnumUtil.getMaterial` answers null for a
  name that does not exist (`EnumUtil.java:25-38`, `72-74`), the 1.20.1 name is `DIRT_PATH` and
  the Spigot API 1.20.1 has no `GRASS_PATH` (`org/bukkit/Material.java:1219`), and `remove(null)`
  on the set does nothing (`LocationUtil.java:63`). Whether a dirt path is hollow rests on
  `Material.isTransparent` alone. *Outside the checkout*: in the Spigot API 1.20.1 sources
  `isTransparent` lists neither `DIRT_PATH` nor `FARMLAND` (`org/bukkit/Material.java:6809-6998`),
  so a path is not hollow there. Arclight's build of the API is to verify (section 9).
- `isBlockUnsafeForUser` (`LocationUtil.java:152-164`) answers safe for an online player in the
  same world who is in creative, spectator or god mode and may fly. Otherwise unsafe is
  `isBlockDamaging`, `isBlockAboveAir` or outside the world border.
- `isBlockDamaging` (`LocationUtil.java:170-184`) answers true when the block below is cactus,
  campfire, fire, magma block, soul campfire, soul fire, sweet berry bush, wither rose
  (`LocationUtil.java:29-30`), lava (`31-32`) or a bed, when the feet block is a nether portal, or
  when the feet or head block is not hollow. The feet block is `floor(y)`, so a position on a slab,
  soul sand, farmland, a path (by the Spigot API, see the first point) or a chest is unsafe.
- `isBlockAboveAir` (`LocationUtil.java:116-118`): y above the max height, or the block below is
  hollow.
- `getSafeDestination(user, loc)` (`LocationUtil.java:203-216`) skips the search for creative,
  spectator and god mode unless `force-safe-teleport-location`, switching on flight when more than
  two unsafe blocks are below (`LocationUtil.java:283-301`), and answers the rounded target.
- `getSafeDestination(loc)` (`LocationUtil.java:218-281`) works in steps. No world is
  `destinationNotSet`. The target is clamped into the border. While above air, y drops by one, but
  below y = 0 the search gives up and returns to the original y (`LocationUtil.java:238-244`). If
  still unsafe, x and z step by one (`245-248`). Then come the 343 offsets of a cube of radius 3,
  nearest first (`LocationUtil.java:72-83`), with y held between the minimum height and the logical
  height, or the max height for targets above the logical height (`LocationUtil.java:225-227`,
  `249-261`). Then y climbs to the top and x steps by one (`262-268`). Then y falls, and at the
  bottom x steps by one and y jumps to the highest block, until x is 48 past the target, which
  throws `holeInFloor` (`LocationUtil.java:269-279`). The answer is the block center with the
  target's yaw and pitch (`LocationUtil.java:280`).

#### Respawn path

`respawn` (`AsyncTeleport.java:358-393`) runs warmup, cost and cooldown like a teleport and then
`respawnNow` (`AsyncTeleport.java:395-412`): the bed spawn of the player,
`PaperLib.getBedSpawnLocationAsync(player, true)`. If there is none, it fires a synthetic
`PlayerRespawnEvent` with the spawn of the player's current world and teleports to whatever that
event answers. *Outside the checkout*: Bukkit's bed spawn is the player's respawn point as vanilla
resolves it for a living player, so a bed, a charged respawn anchor whose charge is not used, or a
forced point of `/spawnpoint`, and none when the block is gone or blocked.

#### Messages of the pipeline

| Key | Line | Text |
| --- | --- | --- |
| `dontMoveMessage` | 305 | `<primary>Teleportation will commence in<secondary> {0}<primary>. Don''t move.` |
| `timeBeforeTeleport` | 1329 | `<dark_red>Time before next teleport\:<secondary> {0}<dark_red>.` |
| `cooldownWithMessage` | 176 | `<dark_red>Cooldown\: {0}` |
| `teleportationCommencing` | 1286 | `<primary>Teleportation commencing...` |
| `passengerTeleportFail` | 917 | `<dark_red>You cannot be teleported while carrying passengers.` |
| `destinationNotSet` | 234 | `Destination not set\!` |
| `holeInFloor` | 508 | `<dark_red>Hole in floor\!` |

### 3.6 The teleport request queue

- Every player holds a `LinkedHashMap` from the requester's account name to a `TpaRequest`
  (`User.java:73`) with the requester's name and UUID, whether it is a tpahere, a location and the
  time (`IUser.java:307-350`). It is not saved, so a restart drops every request. A logout keeps
  it: on quit the `User` object gets an `OfflinePlayerStub` as base and stays in the user cache,
  since `cleanup` only drops the online entry, and the rejoin uses it again (`User.java:364-373`,
  `UserData.java:75-78`, `ModernUserMap.java:153-171`, `233-235`,
  `EssentialsPlayerListener.java:341`). The requests are gone once the cache drops the object:
  after `max-user-cache-value-expiry` seconds without access (600, not in `config.yml`), at
  `max-user-cache-count`, or when the garbage collector takes the soft value
  (`ModernUserMap.java:37-41`, `Settings.java:1687-1696`).
- `requestTeleport(requester, here)` (`User.java:406-421`) reuses the requester's entry, sets the
  time to now, the kind, and the location: for a tpahere the requester's position, for a tpa the
  target's own position, which nobody reads. It removes the entry, and if the queue still holds
  `tpa-max-requests` (5) entries it removes the last key, which is the newest other request, then
  adds the entry at the end.
- `getOutstandingTpaRequest(name, inform)` (`User.java:437-452`) answers the entry of that
  requester while it is younger than `tpa-accept-cancellation`, else removes it and, if asked,
  sends `requestTimedOutFrom` to the target.
- `getNextTpaRequest(inform, ignoreExpirations, excludeHere)` (`User.java:459-489`) walks the
  queue newest first, removes expired entries with `requestTimedOutFrom` if asked, skips tpahere
  entries if asked, and answers the newest valid one.
- Expiry is lazy. Nobody is told when a request expires unless the target runs one of the commands
  that look.

| Key | Line | Text |
| --- | --- | --- |
| `requestTimedOutFrom` | 1104 | `<dark_red>Teleport request from <secondary>{0} <dark_red>has timed out.` |

### 3.7 The player store

#### `userdata/<uuid>.yml`

A file per player in `plugins/Essentials/userdata/` (`UserData.java:44-49`), mapped from
`UserConfigHolder` by the object mapper of Configurate, whose naming turns a field `lastAccountName`
into the key `last-account-name` (`ModernUUIDCache.java:43`). The keys of this part:

| YAML key | Type | Default | Field | Read and written by |
| --- | --- | --- | --- | --- |
| `homes.<name>` | location | absent | `UserConfigHolder.java:36-48` | `UserData.java:138-217` |
| `lastlocation` | location | absent | `UserConfigHolder.java:80-91` | `UserData.java:279-290` |
| `logoutlocation` | location | absent | `UserConfigHolder.java:93-104` | `UserData.java:292-303`, set on quit `EssentialsPlayerListener.java:320` |
| `teleportenabled` | boolean | `true` | `UserConfigHolder.java:130-138` | `UserData.java:398-405`, toggled by `/tptoggle` (section 4.7) |
| `teleportauto` | boolean | `false` | `UserConfigHolder.java:140-148` | `UserData.java:407-414`, toggled by `/tpauto` |
| `ignore` | list of UUIDs | absent | `UserConfigHolder.java:150-162` | `UserData.java:429-460`, by `/ignore` |
| `timestamps.lastteleport` | milliseconds | `0` | `UserConfigHolder.java:377` | `UserData.java:305-312` |
| `timestamps.logout`, `timestamps.login` | milliseconds | `0` | `UserConfigHolder.java:427`, `437` | not in this part |
| `nickname` | string | absent | `UserConfigHolder.java:50` | display name, section 3.4 |
| `last-account-name` | string | absent | `UserConfigHolder.java:265` | the usermap |

A new map of homes is a `HashMap` (`UserConfigHolder.java:39-44`), so the order of homes made in a
session is hash order.

#### The location format

`LocationTypeSerializer.java:16-46` writes a location as a map:

```yaml
world: 5d3f...   # the UUID of the Bukkit world
world-name: world
x: 308.5
y: 64.0
z: 392.5
yaw: 90.0
pitch: 0.0
```

`LazyLocation.location()` (`LazyLocation.java:59-88`) finds the world by UUID, then by `world` as
a name, then by `world-name`, and answers null when none is loaded. Every caller treats that null
as a missing location.

#### `warps/<file>.yml`

`Warps.java`. One file per warp in `plugins/Essentials/warps/` (`Warps.java:27-33`), named
`sanitizeFileName(name)`, lower case with every character outside `a-z0-9-` as `_`
(`StringUtil.java:24-26`). The file holds the location keys at its root plus `name`, the name as
typed, and `lastowner`, the UUID of who set it (`Warps.java:87-92`). Loading reads every `.yml`
with a `name` and a `world` (`Warps.java:122-142`), and logs `loadWarpError` for a warp file that
fails to load (`Warps.java:137`). Names compare without case (`Warps.java:158-181`). A new name
whose file already exists, because another name sanitizes the same, is `similarWarpExist`
(`Warps.java:78-82`). Deleting removes the file (`Warps.java:110-119`).

| Key | Line | Text |
| --- | --- | --- |
| `loadWarpError` | 719 | `<dark_red>Failed to load warp {0}.` (log only) |

#### `spawn.yml`

`SpawnStorage.java`. `plugins/Essentials/spawn.yml` (`SpawnStorage.java:23`) holds
`spawns.<group>` as a location per lowercased group (`SpawnStorage.java:37-44`). The path is split
on dots (`EssentialsConfiguration.java:354-360`).

#### Saving

`save()` hands a copy of the tree to one writer thread shared by every file
(`EssentialsConfiguration.java:54`, `457-490`). `blockingSave()` waits for it (`463-470`). A load
while writes are pending is skipped (`EssentialsConfiguration.java:362-366`).

#### Who writes the last location

| When | Condition | Where |
| --- | --- | --- |
| Before every teleport of Essentials, also one that then fails | `essentials.back.onteleport`, everyone | `AsyncTeleport.java:179-181` |
| On death, the death position | `essentials.back.ondeath` and `back` not disabled | `EssentialsEntityListener.java:159-177` |
| Teleports of others (vanilla `/tp`, mods) | Only with `register-back-in-listener`, off | `EssentialsPlayerListener.java:735-737` |

Two readers use the stored location: `/back` (section 5.1) and the release from jail with
`teleport-when-freed: back` (`config.yml:749`, `Settings.java:2010-2024`), which runs
`AsyncTeleport.back(future)`, a bare `nowAsync` to the last location without type, message or cost,
and falls back to `respawn` when it fails (`AsyncTeleport.java:455-458`, `User.java:790-796`,
`Commandtogglejail.java:153-159`). Jail is an admin command of the [social page](social.md), but
its release depends on this store.

## 4 Teleport requests

### 4.1 tpa

**Names.** `tpa`, with the aliases `call`, `ecall`, `etpa`, `tpask`, `etpask` and the usage
`/<command> <player>` (`plugin.yml:518-521`, `messages.properties:1363-1366`). Nodes:
`essentials.tpa` (live), the target needs `essentials.tpaccept` (live), `essentials.tpacancel` for
the hint (live), `essentials.worlds.<target world>` (rule off), `essentials.vanish.interact`,
`essentials.chat.ignoreexempt`, the timer and cooldown bypass nodes with type `TPA`, and
`essentials.back.onteleport` (everyone).

**Behaviour** (`Commandtpa.java:21-79`):

1. Console: `onlyPlayers`. No argument: usage (`Commandtpa.java:22-24`).
2. The target is `getPlayer(server, user, args, 0)`, online only (`Commandtpa.java:26`).
3. The target is the requester by account name: usage (`Commandtpa.java:27-29`).
4. The target lacks `essentials.tpaccept`: `teleportNoAcceptPermission` (`Commandtpa.java:30-32`).
5. The target has `teleportenabled` false: `teleportDisabled` (`Commandtpa.java:33-35`).
6. Different worlds with `world-teleport-permissions` and no `essentials.worlds.<target world>`:
   `noPerm` (`Commandtpa.java:36-38`).
7. A valid tpa of this requester is already queued at the target: `requestSentAlready`
   (`Commandtpa.java:41-43`). An expired one is removed silently on the way
   (`User.java:432-452`). The check is per kind: `hasOutstandingTpaRequest(name, here)` answers
   true only for a request of the same kind (`User.java:432-435`, called at `Commandtpa.java:41`
   and `Commandtpahere.java:37`), and `requestTeleport` keeps one entry per requester
   (`User.java:407-409`). A pending tpahere of the same requester is therefore not refused: the tpa
   turns it into a tpa, sends the target the three or four lines of step 9 again and moves the
   entry to the end of the queue. See S8.
8. The target has `teleportauto` and does not ignore the requester: the requester's `AsyncTeleport`
   gets type `TPA` and teleports the requester to the target's live position with the cost `tpa`
   (`Commandtpa.java:45-50`), sending `teleportToPlayer` first (`AsyncTeleport.java:233-236`). On
   success the target gets `requestAcceptedAuto` and the requester `requestAcceptedFromAuto`
   (`Commandtpa.java:51-56`). The command ends without `requestSent` (`Commandtpa.java:57`).
9. Else, when the target does not ignore the requester (`UserData.java:443-445`),
   `TPARequestEvent` fires, and a cancelled one is `teleportRequestCancelled`
   (`Commandtpa.java:60-65`). The request is queued (`Commandtpa.java:66`), and the target gets
   `teleportRequest`, `typeTpaccept`, `typeTpdeny` and, with a timeout,
   `teleportRequestTimeoutInfo` (`Commandtpa.java:67-72`).
10. The requester gets `requestSent`, also when the target ignores them, so an ignore stays secret,
    and `typeTpacancel` with `essentials.tpacancel` (`Commandtpa.java:75-78`).
11. The dispatcher charges `tpa` after the run (`EssentialsCommand.java:168-173`), 0.

**Messages.**

| Key | Line | Text | To |
| --- | --- | --- | --- |
| `teleportNoAcceptPermission` | 1300 | `<secondary>{0} <dark_red>does not have permission to accept teleport requests.` | requester, as error |
| `teleportDisabled` | 1294 | `<secondary>{0} <dark_red>has teleportation disabled.` | requester, as error |
| `noPerm` | 885 | `<dark_red>You do not have the <secondary>{0}<dark_red> permission.` | requester, as error |
| `requestSentAlready` | 1102 | `<dark_red>You have already sent {0}<dark_red> a teleport request.` | requester, as error |
| `teleportRequestCancelled` | 1303 | `<primary>Your teleport request to <secondary>{0}<primary> was cancelled.` | requester, as error |
| `teleportRequest` | 1301 | `<secondary>{0}<primary> has requested to teleport to you.` | target |
| `typeTpaccept` | 1457 | `<primary>To teleport, type <secondary>/tpaccept<primary>.` | target |
| `typeTpdeny` | 1458 | `<primary>To deny this request, type <secondary>/tpdeny<primary>.` | target |
| `teleportRequestTimeoutInfo` | 1305 | `<primary>This request will timeout after<secondary> {0} seconds<primary>.` | target, `{0}` = 120 |
| `requestSent` | 1101 | `<primary>Request sent to<secondary> {0}<primary>.` | requester |
| `typeTpacancel` | 1456 | `<primary>To cancel this request, type <secondary>/tpacancel<primary>.` | requester |
| `teleportToPlayer` | 1307 | `<primary>Teleporting to <secondary>{0}<primary>.` | requester, tpauto |
| `requestAcceptedAuto` | 1095 | `<primary>Automatically accepted a teleport request from {0}.` | target, tpauto |
| `requestAcceptedFromAuto` | 1097 | `<secondary>{0} <primary>accepted your teleport request automatically.` | requester, tpauto |

Every `{0}` is a display name. The teleport messages of section 3.5 apply to the tpauto case.

**Config.** `tpa-accept-cancellation`, `tpa-max-requests`, `world-teleport-permissions`, and the
teleport keys of section 2.1.

**Bridge.** The online players with UUID, name and display name, the dimension of both players,
`message`, and for tpauto a teleport to a player's position at the moment of the move.

**Quirks.** S1, S2, S8, Q1, Q4, Q20, Q21 of section 7.

### 4.2 tpahere

**Names.** `tpahere`, with the alias `etpahere` and the usage `/<command> <player>`
(`plugin.yml:530-533`, `messages.properties:1385-1388`). Nodes: `essentials.tpahere` (live),
`essentials.tpaccept` for the target (live), `essentials.tpahere` or `essentials.tpaall` for the
requester on accept (`Commandtpaccept.java:89`), and `essentials.worlds.<requester world>` (off).

**Behaviour** (`Commandtpahere.java:17-57`): as tpa steps 1 to 7, with three differences.

1. The world rule checks the requester's own world (`Commandtpahere.java:32-34`).
2. There is no tpauto branch, so a tpahere always waits for an answer.
3. The queued request has `here` true and the requester's position at the time of sending
   (`Commandtpahere.java:47`, `User.java:410`). The target gets `teleportHereRequest` instead of
   `teleportRequest` (`Commandtpahere.java:48`), and the requester always gets `typeTpacancel`
   (`Commandtpahere.java:56`).

The check of tpa step 7 looks here for a pending tpahere (`Commandtpahere.java:37`), so a pending
tpa of the same requester is not refused and turns into a tpahere in the same way. See S8.

**Messages.** Those of tpa, and:

| Key | Line | Text |
| --- | --- | --- |
| `teleportHereRequest` | 1295 | `<secondary>{0}<primary> has requested that you teleport to them.` |

**Bridge.** As tpa, plus the requester's position (dimension, x, y, z, yaw, pitch) at sending.

**Quirks.** S1, S2, S3, S8, Q4, Q19.

### 4.3 tpaccept

**Names.** `tpaccept`, with the aliases `etpaccept`, `tpyes`, `etpyes` and the usage
`/<command> [player|*]` (`plugin.yml:526-529`), in messages `/<command> [otherplayer]` with the
three forms (`messages.properties:1377-1384`). Nodes: `essentials.tpaccept` (live), the
requester's `essentials.tpa`, `essentials.tpahere` or `essentials.tpaall`, and the requester's
bypass nodes with type `TPA`.

**Behaviour** (`Commandtpaccept.java:24-130`):

1. Console: `onlyPlayers`. `*` or `all` (any case) accepts all (`Commandtpaccept.java:27-31`).
2. No valid pending request: `noPendingRequest`. The check purges expired requests and tells the
   target `requestTimedOutFrom`. For all, tpahere requests do not count
   (`Commandtpaccept.java:33-35`).
3. All: each tpa request, newest first, is handled. A failure shows its error and the loop goes on.
   Each request is removed, and then `requestAcceptedAll` comes with the count
   (`Commandtpaccept.java:51-65`). tpahere requests stay.
4. With a name: `requestAccepted`, then the request of `getPlayer(server, user, args, 0)`, online
   only (`Commandtpaccept.java:42-43`).
5. Without a name: `requestAccepted`, then the newest valid request (`Commandtpaccept.java:45-46`).
6. Handling (`Commandtpaccept.java:78-130`): no request is `noPendingRequest`. An offline requester
   is removed and `noPendingRequest`. A tpahere whose requester lacks `tpahere` and `tpaall`, or a
   tpa whose requester lacks `tpa`, or the world rule, is `noPendingRequest`. Then
   `TeleportRequestResponseEvent` fires, and a cancelled one ends silently. The requester gets
   `requestAcceptedFrom`. Errors of the teleport go to the requester and the accepter gets
   `pendingTeleportCancelled` (`Commandtpaccept.java:109-113`). Only the offline case removes the
   request: one refused for a missing node or the world rule, and one whose event was cancelled,
   stays in the queue (`Commandtpaccept.java:84-95`, `99-104`). A bare `/tpaccept` keeps answering
   that newest request with `noPendingRequest`, which hides the older valid ones, until it expires
   (`User.java:459-489`). `/tpaccept *` removes a tpa request in its `finally`
   (`Commandtpaccept.java:54-62`). See Q37.
7. tpahere: the requester's `AsyncTeleport` with type `TPA` moves the accepter to the stored
   location with the cost `tpaccept` billed to the requester, and on success the requester gets
   `teleporting` (`Commandtpaccept.java:114-123`).
8. tpa: the requester's `AsyncTeleport` with type `TPA` moves the requester to the accepter's live
   position with the cost `tpaccept`, sending `teleportToPlayer` to the requester
   (`Commandtpaccept.java:125-127`).
9. The request is removed after the teleport is started (`Commandtpaccept.java:129`). No dispatcher
   charge (`Commandtpaccept.java:40`, `48`).

**Messages.**

| Key | Line | Text | To |
| --- | --- | --- | --- |
| `noPendingRequest` | 884 | `<dark_red>You do not have a pending request.` | accepter, as error |
| `requestAccepted` | 1093 | `<primary>Teleport request accepted.` | accepter |
| `requestAcceptedAll` | 1094 | `<primary>Accepted <secondary>{0} <primary>pending teleport request(s).` | accepter |
| `requestAcceptedFrom` | 1096 | `<secondary>{0} <primary>accepted your teleport request.` | requester |
| `pendingTeleportCancelled` | 936 | `<dark_red>Pending teleportation request cancelled.` | accepter on error |
| `teleporting` | 1297 | `<primary>Teleporting...` | requester of a tpahere |
| `teleportToPlayer` | 1307 | `<primary>Teleporting to <secondary>{0}<primary>.` | requester of a tpa |
| `requestTimedOutFrom` | 1104 | see section 3.6 | accepter |

**Bridge.** Positions of both players, a teleport to a stored location and to a live player
position, `message` to both, the quit event to drop requests of players who left.

**Quirks.** Q2, Q3, Q5, Q19, Q21, Q22, Q37.

### 4.4 tpdeny

**Names.** `tpdeny`, with the aliases `etpdeny`, `tpno`, `etpno` and the usage
`/<command> [player|*]` (`plugin.yml:546-549`), in messages `/<command>` with three forms
(`messages.properties:1397-1404`). Node `essentials.tpdeny` (live).

**Behaviour** (`Commandtpdeny.java:19-89`):

1. Console: `onlyPlayers`. `*` or `all` denies all (`Commandtpdeny.java:21-26`).
2. No valid pending request, tpahere included, without telling about expired ones:
   `noPendingRequest` (`Commandtpdeny.java:28-30`).
3. All: each request newest first. `TeleportRequestResponseEvent` fires, and a cancelled one is
   skipped without removal (`Commandtpdeny.java:64-69`). An online requester gets
   `requestDeniedFrom`. The request is removed and counted, and then `requestDeniedAll` follows
   (`Commandtpdeny.java:61-79`).
4. With a name: the request of `getPlayer(server, user, args, 0)`. Without: the newest
   (`Commandtpdeny.java:33-41`).
5. No request, or an offline or unknown requester: `noPendingRequest`, and the request stays
   (`Commandtpdeny.java:43-50`).
6. The event fires. Then the target gets `requestDenied`, the requester `requestDeniedFrom`, and
   the request is removed (`Commandtpdeny.java:52-58`). The dispatcher charges `tpdeny`, 0.

**Messages.**

| Key | Line | Text |
| --- | --- | --- |
| `requestDenied` | 1098 | `<primary>Teleport request denied.` |
| `requestDeniedFrom` | 1100 | `<secondary>{0} <primary>denied your teleport request.` |
| `requestDeniedAll` | 1099 | `<primary>Denied <secondary>{0} <primary>pending teleport request(s).` |
| `noPendingRequest` | 884 | as above |

**Quirks.** Q5, Q20.

### 4.5 tpacancel

**Names.** `tpacancel`, with the alias `etpacancel` and the usage `/<command> [player]`
(`plugin.yml:542-545`, `messages.properties:1371-1376`). Node `essentials.tpacancel` (live), whose
description in `plugin.yml:1331-1332` wrongly says `/tpaccept`.

**Behaviour** (`Commandtpacancel.java:25-46`):

1. Console: `onlyPlayers`.
2. Without a name: from the queue of every other online player, the request keyed by the
   requester's name is removed, valid or expired. More than none is `teleportRequestAllCancelled`,
   none is `noPendingRequest` (`Commandtpacancel.java:27-39`).
3. With a name: `getPlayer(server, user, args, 0)`, the request at that player is removed, and only
   if there was one the requester gets `teleportRequestSpecificCancelled` with the account name
   (`Commandtpacancel.java:40-44`). Nothing is said when there was none.
4. The target of a cancelled request is never told. The dispatcher charges `tpacancel`, 0.

**Messages.**

| Key | Line | Text |
| --- | --- | --- |
| `teleportRequestAllCancelled` | 1302 | `<primary>All outstanding teleport requests cancelled.` (the count is passed and not shown) |
| `teleportRequestSpecificCancelled` | 1304 | `<primary>Outstanding teleport request with<secondary> {0}<primary> cancelled.` |
| `noPendingRequest` | 884 | as above |

**Quirks.** S1 (silent cancel), Q6.

### 4.6 tpauto

**Names.** `tpauto`, with the alias `etpauto` and the usage `/<command> [player]`
(`plugin.yml:538-541`, `messages.properties:1393-1396`). Nodes `essentials.tpauto` (live),
`essentials.tpauto.others` (`Commandtpauto.java:9`, not live).

**Behaviour** (`Commandtpauto.java`, `EssentialsToggleCommand.java`):

1. Player with one argument: `on`, `1` (any case) or a word starting with `ena` (case sensitive)
   turns it on, `off`, `0` or `dis...` turns it off (`EssentialsToggleCommand.java:35-42`). Another
   word toggles the players it names with the others node and the own state without
   (`EssentialsToggleCommand.java:20-27`).
2. Two arguments with the others node: the named players, the second argument as value
   (`EssentialsToggleCommand.java:28-29`). Anything else toggles the own state (`30-32`).
3. Others: the name must have two characters after trimming, else `playerNotFound`. The players
   of `server.matchPlayer` are toggled, which is the one online player whose name equals the text
   when there is one, else every online player whose name contains it (section 3.3 step 6), hidden
   ones skipped for a source without `vanish.interact` (`EssentialsToggleCommand.java:44-68`).
4. Console: others only (`Commandtpauto.java:13-15`).
5. Toggle (`Commandtpauto.java:23-37`): `teleportauto` is set and saved. The player gets
   `autoTeleportEnabled` or `autoTeleportDisabled`, and `teleportationDisabledWarning` when turning
   it on while `teleportenabled` is false. A sender who is not the player gets
   `autoTeleportEnabledFor` or `autoTeleportDisabledFor`. The dispatcher charges `tpauto`, 0.

**Messages.**

| Key | Line | Text |
| --- | --- | --- |
| `autoTeleportEnabled` | 30 | `<primary>You are now automatically approving teleport requests.` |
| `autoTeleportDisabled` | 28 | `<primary>You are no longer automatically approving teleport requests.` |
| `teleportationDisabledWarning` | 1289 | `<primary>You must enable teleportation before other players can teleport to you.` |
| `autoTeleportEnabledFor` | 31 | `<secondary>{0}<primary> is now automatically approving teleport requests.` |
| `autoTeleportDisabledFor` | 29 | `<secondary>{0}<primary> is no longer automatically approving teleport requests.` |

**Storage.** `teleportauto` of the player file.

**Quirks.** S2 (auto accept of a mismatched name), Q23.

### 4.7 tptoggle

**Names.** `tptoggle`, with the alias `etptoggle` and the usage `/<command> [player] [on|off]`
(`plugin.yml:570-573`, `messages.properties:1442-1444`). Nodes `essentials.tptoggle` and
`essentials.tptoggle.others` (`Commandtptoggle.java:9`), neither live. It is the command that
writes `teleportenabled`, which sections 3.7, 4.1 step 5, 4.2 and 4.6 step 5 read.

**Behaviour** (`Commandtptoggle.java:8-35`): an `EssentialsToggleCommand` like `tpauto`, with the
others node `essentials.tptoggle.others`, parsing as section 4.6 steps 1 to 4. It sets and saves
the flag and sends `teleportationEnabled` or `teleportationDisabled`, and to another sender
`teleportationEnabledFor` or `teleportationDisabledFor`. The flag also guards the admin commands
`tp`, `tphere` and `tpaall` (`Commandtp.java:30`, `81`, `99-102`, `Commandtphere.java:20`,
`Commandtpaall.java:36`). The port needs the command among the admin commands or a migration rule
for the flag.

**Usage text.** The usage line `tptoggleCommandUsage1` (1443) has no
`tptoggleCommandUsage1Description`, and no locale file has one. `messages.properties` holds
`tptoggleCommandUsageDescription` (1444) instead. `EssentialsCommand` registers each usage line
with the key `<line key>Description` (`EssentialsCommand.java:51-63`), and every print of the
usage translates that key, which then throws `MissingResourceException` (`I18n.java:156-168`). The
dispatcher prints the usage on `NotEnoughArgumentsException` (`Essentials.java:832-843`), which no
path of `tptoggle` throws (`Commandtptoggle.java:12-20`, `EssentialsToggleCommand.java:20-68`).
`/help /tptoggle` prints it once the command is loaded (`Commandhelp.java:44`, `55-57`), which the
first `/tptoggle` or its tab completion since the start does, also by a player without the node
(`Essentials.java:627-635`, `686`, `797-811`). The player then reads the header lines and then
`errorWithMessage` with the Java text of the exception (`Essentials.java:855-856`, `891-897`).
`essentials.help` is in the group `default` (plugins.md lines 64 to 69), so this is live. The port
gives every usage line its description. See Q38.

**Messages.**

| Key | Line | Text |
| --- | --- | --- |
| `teleportationEnabled` | 1290 | `<primary>Teleportation <secondary>enabled<primary>.` |
| `teleportationDisabled` | 1287 | `<primary>Teleportation <secondary>disabled<primary>.` |
| `teleportationEnabledFor` | 1291 | `<primary>Teleportation <secondary>enabled <primary>for <secondary>{0}<primary>.` |
| `teleportationDisabledFor` | 1288 | `<primary>Teleportation <secondary>disabled <primary>for <secondary>{0}<primary>.` |

**Storage.** `teleportenabled` of the player file.

**Quirks.** Q38.

## 5 Back, homes and warps

### 5.1 back

**Names.** `back`, with the aliases `eback`, `return`, `ereturn` and the usage `/<command> [player]`
(`plugin.yml:23-26`, `messages.properties:33-38`). Nodes: `essentials.back` (live),
`essentials.back.others` (not live), `essentials.back.into.<world>` of the requester (everyone,
registered true), `essentials.worlds.<world>` (off), `essentials.back.ondeath` (live),
`essentials.back.onteleport` (everyone), the bypass nodes with type `BACK`.

**Behaviour** (`Commandback.java:17-75`, `AsyncTeleport.java:442-453`):

1. Player with an argument and `back.others`: the target is
   `getPlayer(server, args, 0, true, false)`, online, hidden included. The sender gets `backOther`
   with the account name before the teleport (`Commandback.java:20-23`, `37-41`). Without the node
   the argument is ignored.
2. Console: an argument is needed, else usage (`Commandback.java:29-35`).
3. No last location, or its world is not loaded: `noLocationFound` (`Commandback.java:44-46`).
4. For a player source: the world rule (off) (`Commandback.java:54-56`), and the requester must
   hold `essentials.back.into.<world of the last location>`, else `noPerm`
   (`Commandback.java:58-60`).
5. Console: `back(null, null)`, so no warmup and no cost (`Commandback.java:63-64`). Others: the
   requester affords `back` and owns the warmup (`Commandback.java:65-68`). Self: the same with the
   player as owner (`Commandback.java:69-72`).
6. `AsyncTeleport.back` sets the type `BACK`, sends `backUsageMsg` to the moved player before any
   check, and runs `teleportOther` to the last location (`AsyncTeleport.java:447-453`). `nowAsync`
   stores the current position as the new last location, so a second `/back` returns
   (`AsyncTeleport.java:179-181`).

**Death** (`EssentialsEntityListener.java:159-177`, priority HIGH, ignoring cancelled events): for a
player who is not an NPC, `send-info-after-death` (off) would send `infoAfterDeath`. With
`essentials.back.ondeath` and `back` not disabled, the death position becomes the last location and
the player gets `backAfterDeath`. With `death-messages` (`true`, `config.yml:558`,
`Settings.java:1377-1379`) the handler keeps the vanilla death message, and false would clear it
(`EssentialsEntityListener.java:174-176`).

**Messages.**

| Key | Line | Text |
| --- | --- | --- |
| `backOther` | 39 | `<primary>Returned<secondary> {0}<primary> to previous location.` |
| `noLocationFound` | 872 | `<dark_red>No valid location found.` |
| `noPerm` | 885 | as above, `{0}` = `essentials.back.into.<world>` |
| `backUsageMsg` | 46 | `<primary>Returning to previous location.` (world and position are passed and not shown) |
| `backAfterDeath` | 32 | `<primary>Use the<secondary> /back<primary> command to return to your death point.` |
| `infoAfterDeath` | 540 | `<primary>You died in <yellow>{0} <primary>at <yellow>{1}, {2}, {3}<primary>.` (off) |

**Bridge.** The death event with the dimension and position of the death, the position of the
player before every teleport, and the list of dimensions for the `back.into` nodes.

**Storage.** `lastlocation`.

**Quirks.** Q7, Q8, Q15, Q21, S4.

### 5.2 home

**Names.** `home`, with the aliases `ehome`, `homes`, `ehomes` and the usage
`/<command> [player:][name]` (`plugin.yml:210-213`, `messages.properties:509-514`). Nodes:
`essentials.home` (live), `essentials.home.bed` (live), `essentials.home.others` (not live),
`essentials.sethome.multiple.unlimited` for the limit shown, `essentials.worlds.<world>` with
`world-home-permissions` (off).

**Behaviour** (`Commandhome.java:26-152`):

1. Console: `onlyPlayers`. The cost is `home` (`Commandhome.java:28`).
2. The argument is split on `:`. Without a colon, or without `home.others`, the first part is the
   home name. With both, the first part is a player found by `getPlayer(nameParts, 0, true, true)`,
   offline included, and the second part, if any, the home name (`Commandhome.java:32-42`).
3. Name `bed`, any case (`Commandhome.java:44-70`): without `home.bed` it is `noAccessCommand` as
   error. An offline owner is `bedOffline`. Otherwise the owner's bed spawn counts (section 3.5,
   respawn path). With one, `UserTeleportHomeEvent` of type `BED` fires, a teleport with the cost
   runs, and on success the player gets `teleportHome` with `bed`. Without one, it is `bedMissing`
   as error.
4. Any other name, lowercased, goes to `goHome` (`Commandhome.java:71`, `131-152`): an empty name or
   a name with no location throws `NotEnoughArgumentsException`. A numeric name picks the n-th home
   of the list (`UserData.java:138-146`). Then the world rule (off), `UserTeleportHomeEvent` of type
   `HOME`, the teleport, and on success `teleportHome` with the name as typed and lowercased.
5. `NotEnoughArgumentsException`, so no name, an unknown name or a home in an unloaded world, leads
   to the list branch, which first gets the owner's bed spawn, or none for an offline owner
   (`Commandhome.java:72-117`):
   1. No homes and the owner is the player: with `spawn-if-no-home`, a bed and `home.bed` run
      `AsyncTeleport.respawn`, the bed spawn. Else a teleport goes to the spawn of the player's
      current world (`Commandhome.java:77-88`), each after `UserTeleportHomeEvent` of type `BED`
      or `SPAWN`. Without `spawn-if-no-home`: `noHomeSetPlayer` (`Commandhome.java:89-91`).
   2. No homes of another player, or no home in a loaded world: `noHomeSetPlayer`
      (`Commandhome.java:92-93`).
   3. Exactly one home and the owner is the player: a teleport to it, whatever name was typed
      (`Commandhome.java:94-99`).
   4. Otherwise the list: the names, plus `bed` or `bedNull` for holders of `home.bed`, joined with
      `, ` as `homes` (`Commandhome.java:100-110`). The count and the limit (`*` for unlimited, `?`
      for offline) are passed and not shown (`Commandhome.java:121-129`).

   A home in an unloaded world therefore never leads to another place and never to an empty error:
   a player whose homes all lie in unloaded worlds reads `noHomeSetPlayer`, a bare `/home`
   included, since the spawn fallback of sub-step 1 needs no home at all, and a player with
   another home in a loaded world gets the list. Sub-step 3 runs only after `hasValidHomes` proved
   the single home loaded, so its `goHome` cannot throw there (`Commandhome.java:92-99`,
   `UserData.java:154-161`).

**Messages.**

| Key | Line | Text |
| --- | --- | --- |
| `teleportHome` | 1296 | `<primary>Teleporting to <secondary>{0}<primary>.` |
| `bedOffline` | 78 | `<dark_red>Cannot teleport to the beds of offline users.` |
| `bedMissing` | 76 | `<dark_red>Your bed is either unset, missing or blocked.` |
| `noHomeSetPlayer` | 866 | `<primary>Player has not set a home.` |
| `homes` | 515 | `<primary>Homes\:<reset> {0}` |
| `bed` | 75 | `<i>bed<reset>` |
| `bedNull` | 77 | `<st>bed<reset>` |
| `noAccessCommand` | 857 | as above |
| `noPerm` | 885 | as above |

**Bridge.** The respawn point of a player, validated as vanilla does for a living player without
using an anchor charge, the spawn of a dimension, and teleports across dimensions.

**Storage.** `homes`.

**Quirks.** Q9, Q10, Q11, Q12, Q13, Q16, Q17, S7.

### 5.3 sethome

**Names.** `sethome`, with the aliases `esethome`, `createhome`, `ecreatehome` and the usage
`/<command> [[player:]name]` (`plugin.yml:426-429`, `messages.properties:1155-1160`). Nodes:
`essentials.sethome` (live), `essentials.sethome.multiple` (live),
`essentials.sethome.multiple.<set>` and `.unlimited` (not live), `essentials.sethome.others` (not
live).

**Behaviour** (`Commandsethome.java:23-101`):

1. Console: `onlyPlayers`. The default name is `home` (`Commandsethome.java:26`).
2. An argument with a colon is split into owner and name (`Commandsethome.java:30-33`). One part is
   the name, lowercased. Two parts, with or without a colon, make the second the name and, with
   `sethome.others`, the first the owner, found by `getPlayer(term, true, true)`
   (`Commandsethome.java:35-45`). Without the node `/sethome steve:base` and `/sethome a b` set the
   player's own home `base` or `b`.
3. The limit (`Commandsethome.java:89-101`): without `multiple.unlimited` on the setter, the limit
   is the setter's: 1, or `sethome-multiple.default` (3) with `sethome.multiple`, raised by every
   set `<s>` of `sethome-multiple` whose node `essentials.sethome.multiple.<s>` the setter holds
   (`Settings.java:202-223`). At the limit, only an existing name, compared as typed, may be
   overwritten, else `maxHomes`. A limit of 1 renames the home to `home`
   (`Commandsethome.java:47-49`).
4. `bed` or an integer name: `invalidHomeName` (`Commandsethome.java:50-52`).
5. The location is the setter's position. An unsafe block for the owner is
   `unsafeTeleportDestination` (`Commandsethome.java:54-57`, section 3.5, safety).
6. With `confirm-home-overwrite` (off) a first try only warns (`Commandsethome.java:59-64`).
7. `HomeModifyEvent` fires (`Commandsethome.java:66-81`). Then `setHome` stores the name through
   `safeString` (lower case, every character outside `a-z0-9` as `_`) and saves
   (`UserData.java:181-186`), and the player gets `homeSet` (`Commandsethome.java:84`). The
   dispatcher charges `sethome`, 0.

**Messages.**

| Key | Line | Text |
| --- | --- | --- |
| `homeSet` | 518 | `<primary>Home set to current location.` (world, position and name are passed and not shown) |
| `maxHomes` | 760 | `<dark_red>You cannot set more than<secondary> {0} <dark_red>homes.` |
| `invalidHomeName` | 552 | `<dark_red>Invalid home name\!` |
| `unsafeTeleportDestination` | 1490 | `<dark_red>The teleport destination is unsafe.` |
| `homeConfirmation` | 516 | `<primary>You already have a home named <secondary>{0}<primary>\!\nTo overwrite your existing home, type the command again.` (off) |

**Bridge.** The player's position and the blocks at feet, head and below for the safety rule, or a
safety query of the bridge mod.

**Storage.** `homes.<safe name>`.

**Quirks.** Q11, Q12, Q16, Q32 (the compass, section 5.8).

### 5.4 delhome

**Names.** `delhome`, with the aliases `edelhome`, `remhome`, `eremhome`, `rmhome`, `ermhome` and
the usage `/<command> [player:]<name>` (`plugin.yml:106-109`, `messages.properties:207-212`).
Nodes `essentials.delhome` (live), `essentials.delhome.others` (not live).

**Behaviour** (`Commanddelhome.java:39-79`):

1. No argument: usage (`Commanddelhome.java:41-43`).
2. The argument is split on `:` (`Commanddelhome.java:49-55`). Two parts and either the console or
   `delhome.others`: the owner is `getPlayer(parts, 0, true, true)`, offline included, and the name
   the second part lowercased (`Commanddelhome.java:57-59`). The console with one part: usage
   (`Commanddelhome.java:60-61`). Else the player's own home named by the first part, so without
   `delhome.others` `/delhome steve:base` and `/delhome steve base` delete the own home `steve`,
   where `/sethome steve:base` sets the own home `base` (`Commanddelhome.java:50-55`, `62-64`,
   `Commandsethome.java:35-45`).
3. `bed`: `invalidHomeName` (`Commanddelhome.java:66-68`). `*`: every home of the owner is deleted,
   with no confirmation (`Commanddelhome.java:69-74`). Else that one home
   (`Commanddelhome.java:75-77`).
4. Each delete fires `HomeModifyEvent`, then `delHome`, which takes a numeric name as the n-th home
   and tries the name and its `safeString` form (`UserData.java:188-199`). It answers `deleteHome`
   or, if missing, `invalidHome` with the name (`Commanddelhome.java:21-37`). The dispatcher
   charges `delhome`, 0.

**Messages.**

| Key | Line | Text |
| --- | --- | --- |
| `deleteHome` | 201 | `<primary>Home<secondary> {0} <primary>has been removed.` |
| `invalidHome` | 551 | `<dark_red>Home<secondary> {0} <dark_red>doesn''t exist\!` |
| `invalidHomeName` | 552 | as above |

**Quirks.** Q11, Q13, Q14.

### 5.5 warp

**Names.** `warp`, with the aliases `ewarp`, `warps`, `ewarps` and the usage
`/<command> <pagenumber|warp> [player]` (`plugin.yml:594-597`, `messages.properties:1548-1553`).
Nodes: `essentials.warp` (live), `essentials.warp.list` (live), `essentials.warp.others` and
`essentials.warp.otherplayers` (not live), `essentials.warps.<warp>` (only with
`per-warp-permission`, off).

**Behaviour** (`Commandwarp.java:28-109`):

1. Player with no argument or a first argument of digits only: without `warp.list`
   `warpListPermission`, else the list (`Commandwarp.java:30-36`).
2. Two arguments and `warp.others` or `warp.otherplayers`: the target is
   `getPlayer(server, user, args, 1)` (`Commandwarp.java:39-43`). Else the player warps
   themselves, and a second argument is ignored (`Commandwarp.java:44-45`).
3. `warpUser` (`Commandwarp.java:89-99`): the cost is the command cost
   `warp-<name lowercased, _ as ->` plus `warp`, computed for the moved player, and the sender
   must afford it. With `per-warp-permission` the sender needs `essentials.warps.<name>`, else
   `warpUsePermission`. Then the sender's `AsyncTeleport.warp` runs. The error future comes from
   the source of the target, not of the sender (`Commandwarp.java:89-98`), so a failure of an
   operator's `/warp spawn Alex` is shown to Alex. See Q34.
4. `AsyncTeleport.warp` (`AsyncTeleport.java:414-440`): `UserWarpEvent` fires. The warp's location
   is looked up by name without case, and `warpNotExist` answers when the warp or its world is
   missing (`Warps.java:56-67`). A teleport with warmup on the sender follows, and on success
   `warpingTo` goes to the moved player and to the sender if different.
5. The list (`Commandwarp.java:60-87`, `101-109`) holds the warps the sender may use, sorted
   without case (`Warps.java:46-53`). None is `noWarpsDefined`. A page holds 20, and a page above
   the last is the last. More than 20 warps show `warpsCount` and `warpList`, fewer `warps`.
6. Console (`Commandwarp.java:48-57`): fewer than two arguments or an integer first is the list,
   without the `warp.list` check. Else the target `getPlayer(args, 1, true, false)` is warped with
   no cost, the warmup on the target.

**Messages.**

| Key | Line | Text |
| --- | --- | --- |
| `warpListPermission` | 1562 | `<dark_red>You do not have permission to list warps.` |
| `noWarpsDefined` | 901 | `<primary>No warps defined.` |
| `warpsCount` | 1566 | `<primary>There are<secondary> {0} <primary>warps. Showing page <secondary>{1} <primary>of <secondary>{2}<primary>.` |
| `warpList` | 1561 | `{0}` |
| `warps` | 1565 | `<primary>Warps\:<reset> {0}` |
| `warpUsePermission` | 1572 | `<dark_red>You do not have permission to use that warp.` |
| `warpingTo` | 1560 | `<primary>Warping to<secondary> {0}<primary>.` (world and position passed, not shown) |
| `warpNotExist` | 1563 | `<dark_red>That warp does not exist.` |

**Storage.** `warps/*.yml`.

**Quirks.** Q18, Q34, S7.

### 5.6 setwarp

**Names.** `setwarp`, with the aliases `createwarp`, `ecreatewarp`, `esetwarp` and the usage
`/<command> <warp>` (`plugin.yml:438-441`, `messages.properties:1175-1178`). Nodes
`essentials.setwarp`, `essentials.warp.overwrite.<safeString(name)>` (`plugin.yml:1388-1391`),
neither live.

**Behaviour** (`Commandsetwarp.java:18-57`):

1. Console: `onlyPlayers`. No argument: usage (`Commandsetwarp.java:20-22`).
2. An integer, an empty name or a Windows device name (`con`, `prn`, `aux`, `nul`, `com0` to
   `com9`, `lpt0` to `lpt9`) is `invalidWarpName` (`Commandsetwarp.java:24-30`,
   `StringUtil.java:21`, `32-34`). Nothing else is checked.
3. A new warp, or an existing one with the overwrite node, fires `WarpModifyEvent` and is stored at
   the setter's position, with no safety check (`Commandsetwarp.java:39-52`, `Warps.java:75-93`).
   An existing one without the node is `warpOverwrite` (`Commandsetwarp.java:53-55`). Then
   `warpSet` with the name as typed (`Commandsetwarp.java:56`). An existing warp whose world is not
   loaded counts as new: `getWarp` throws `WarpNotFoundException` when its location does not
   resolve, the command ignores that, and the create branch calls `setWarp`, which finds the stored
   entry by name without case and overwrites its location without any check of
   `essentials.warp.overwrite.<name>` (`Commandsetwarp.java:35-45`, `Warps.java:56-67`, `77-88`).
   `setWarp` also writes `name` as typed, so a spelling that differs only in case replaces the
   stored one. See S9.

**Messages.**

| Key | Line | Text |
| --- | --- | --- |
| `invalidWarpName` | 562 | `<dark_red>Invalid warp name\!` |
| `warpOverwrite` | 1564 | `<dark_red>You cannot overwrite that warp.` |
| `warpSet` | 1571 | `<primary>Warp<secondary> {0} <primary>set.` |
| `similarWarpExist` | 1219 | `<dark_red>A warp with a similar name already exists.` |

**Quirks.** Q18, S7, S9.

### 5.7 delwarp

**Names.** `delwarp`, with the aliases `edelwarp`, `remwarp`, `eremwarp`, `rmwarp`, `ermwarp` and
the usage `/<command> <warp>` (`plugin.yml:118-121`, `messages.properties:221-224`). Node
`essentials.delwarp`, not live.

**Behaviour** (`Commanddelwarp.java:19-44`): player or console. No argument: usage. A known warp,
also one in an unloaded world, fires `WarpModifyEvent`, loses its file and answers `deleteWarp`
with the name as typed. An unknown one is `warpNotExist`. A file that cannot be deleted is
`warpDeleteError` (`Warps.java:110-119`).

**Messages.**

| Key | Line | Text |
| --- | --- | --- |
| `deleteWarp` | 204 | `<primary>Warp<secondary> {0} <primary>has been removed.` |
| `warpNotExist` | 1563 | as above |
| `warpDeleteError` | 1554 | `<dark_red>Problem deleting the warp file.` |

### 5.8 The compass

On every join, respawn and dimension change, `updateCompass` points the compass of the player at a
home in their current world, else at their first home, else at their bed spawn
(`EssentialsPlayerListener.java:183`, `433`, `618-633`, `951`, `UserData.java:163-175`). No node is
needed, since `compass-towards-home-perm: false` (`config.yml:831`) skips the check of
`essentials.home.compass` (`plugin.yml:888-890`, default false). No other part of the port covers
it, and it is the one effect of `/sethome` that a player sees without a command.

*Outside the checkout*: Bukkit's `setCompassTarget` sends the player the spawn position packet, so
a vanilla compass points at the home instead of the world spawn. The plan does not name it. Either
the bridge gets an action that sends that packet to one player (section 8.2), or Golem drops the
behaviour. See Q32.

**Bridge.** The `compass` action of section 8.2, only if the owner keeps the behaviour.

**Quirks.** Q32.

## 6 EssentialsXSpawn

### 6.1 spawn

**Names.** `spawn`, with the alias `espawn` and the usage `/<command> [player]`
(`spawn/plugin.yml:15-18`). `setspawn` has the alias `esetspawn`, not `espawn`
(`spawn/plugin.yml:11-14`). No usage keys exist in `messages.properties`, so a usage error prints
the description and usage of `spawn/plugin.yml` (section 3.1, step 8). Nodes: `essentials.spawn`
(live, `EssentialsSpawn.java:76`), `essentials.spawn.others` (not live).

**Behaviour** (`Commandspawn.java:24-94`):

1. Player: the cost `spawn` must be affordable (`Commandspawn.java:26-27`). With an argument and
   `spawn.others`, the target is `getPlayer(server, user, args, 0)` and on success gets
   `teleportAtoB` with the sender's display name and `spawn` unless it is the sender
   (`Commandspawn.java:28-38`). Else the target is the sender (`Commandspawn.java:39-41`).
2. Console: an argument is needed. The target is `getPlayer(args, 0, true, false)`, and on success
   it gets `teleportAtoB` with `Console` (`Commandspawn.java:47-59`, `Console.java:15-17`,
   `messages.properties:175`).
3. `respawn` (`Commandspawn.java:69-94`) takes the spawn of the target's group, `default` for
   everyone (section 2.2). None ends silently. `UserTeleportSpawnEvent` fires. The console runs
   `now(spawn)` without cooldown, warmup or cost. A player runs the sender's
   `teleportPlayer(target, spawn, cost)`, and on success the sender gets `teleporting`.
4. `SpawnStorage.getSpawn(group)` (`SpawnStorage.java:46-71`): the lowercased group's spawn, else
   `default`, else the spawn location of the first world whose environment is `NORMAL`, else of the
   first world. Until someone runs `/setspawn`, that is the Overworld's world spawn (plugins.md line
   93 gives 308 64 392).

**Messages.**

| Key | Line | Text |
| --- | --- | --- |
| `teleportAtoB` | 1292 | `<secondary>{0}<primary> teleported you to <secondary>{1}<primary>.` |
| `teleporting` | 1297 | `<primary>Teleporting...` |

**Bridge.** The world spawn of the Overworld, and the dimension type of every dimension, for "the
first normal world".

**Storage.** `spawn.yml`.

**Quirks.** Q24, Q25.

### 6.2 setspawn

**Names.** `setspawn`, with the alias `esetspawn` and the usage `/<command> <group>`
(`spawn/plugin.yml:11-14`), although the group is optional. Node `essentials.setspawn`, not live.

**Behaviour** (`Commandsetspawn.java:17-21`): player only. The group is every argument joined with
spaces, or `default`. The setter's position is stored for the lowercased group in memory and in
`spawn.yml`, without a safety check and without changing the world spawn of vanilla
(`SpawnStorage.java:37-44`). Then the setter gets `spawnSet` with the group as typed. The
dispatcher charges `setspawn`, 0.

| Key | Line | Text |
| --- | --- | --- |
| `spawnSet` | 1261 | `<primary>Spawn location set for group<secondary> {0}<primary>.` |

**Quirks.** Q25, Q26, S7.

### 6.3 The two listeners, off on the live server

`EssentialsSpawn.onEnable` registers each listener only when its priority is not `none`
(`EssentialsSpawn.java:43-53`, `Settings.java:1457-1489`). With both off, which is the live state
by plugins.md, Golem adds neither, as the plan says. What they would do:

- **Respawn** (`EssentialsSpawnPlayerListener.java:37-77`): skipped for a jailed player. With
  `respawn-at-home` (off): the respawn point, unless it is an anchor and `respawn-at-anchor` is off,
  then the first home in the death world, then the first home. Then a random respawn location if
  one is configured (none). Then the group's spawn, which replaces vanilla's respawn point, so beds
  and anchors would be ignored.
- **Join** (`EssentialsSpawnPlayerListener.java:79-168`): an old player in a `spawn-on-join` group
  (none) goes to the spawn without safety (`nowUnsafe`). A new player goes to a random spawn
  location or to the spawn `newbies` (falling back to `default` and the world spawn) one tick later,
  and two ticks later the server gets `newbies.announce-format` and the player the kit `tools`. The
  listener logs `teleportNewPlayerError` (`EssentialsSpawnPlayerListener.java:162`).

| Key | Line | Text |
| --- | --- | --- |
| `teleportNewPlayerError` | 1299 | `<dark_red>Failed to teleport new player\!` (log only) |

With both off, a new player starts and a dead player respawns as vanilla decides, at the world
spawn of vanilla, not at the point of `/setspawn` (Q25).

## 7 Quirks and security issues

Each entry gets its own issue in `skytekx/golem` with the label in the first column, as the plan
requires. "Live" says whether a player of the group `default` can trigger it on the live server
today. A security entry marked live is filed as a private security advisory while EssentialsX
still runs, as the plan says.

### 7.1 Security

| ID | Label | Live | Today | Evidence | Golem |
| --- | --- | --- | --- | --- | --- |
| S1 | security | Yes | Held back in a private security advisory while EssentialsX runs on the live server. | Held back in a private security advisory while EssentialsX runs on the live server. | A rate limit per requester (requests per minute) and per target, a cooldown after a cancel or deny for the same pair, and one line per request |
| S2 | security | Yes | Held back in a private security advisory while EssentialsX runs on the live server. | Held back in a private security advisory while EssentialsX runs on the live server. | Exact name or a unique prefix of an online player, an error that lists the candidates when ambiguous, never substring or nickname matching for teleports, and the resolved name in every confirmation |
| S3 | security | Yes | A tpahere moves the accepter to where the requester stood when sending, which can be far from where the requester is at accept time and where they can prepare a box or a drop that the safety check calls safe. The request text says "teleport to them" and shows no place. | `User.java:410`, `Commandtpaccept.java:114-123`, `messages.properties:1295`. The safety check looks only at feet, head and the block below, `LocationUtil.java:170-184` | Accept moves to the requester's position at accept time, after the safety search, and the request names the requester's dimension |
| S4 | security | Yes | The safety search is weak for a modpack: the hazards below are a fixed list of vanilla blocks, mod blocks count by `Material.isTransparent` of Bukkit, the descent stops at y below 0 instead of the minimum height of the dimension, and when nothing is safe within radius 3 the search walks up to 48 blocks along x. A `/back` to a death in the void or in lava depends on it. | `LocationUtil.java:26-70`, `175-183`, `238-244`, `262-279`, `EssentialsEntityListener.java:170-172` | A safety search in the bridge mod on the server thread with vanilla's own collision and fluid and damage checks (`isFree`, `getBlockState().isSuffocating`, fluids, damage tags), the dimension's real height range, a bounded search box, and a clear refusal when nothing is found |
| S5 | security | No | A negative command cost pays the player: costs are parsed without a sign check, `charge` calls `takeMoney` with the negative value, which adds money. | `Settings.java:417-447`. `Trade.java:320-327` for the charge of the dispatcher (`EssentialsCommand.java:172`) and `Trade.java:299-308` for a teleport, which first turns the command cost into a money cost (`AsyncTeleport.java:274-278`). `User.java:333-345` | Refuse negative costs when the config loads |
| S6 | security | No | Costs are charged after the teleport, and a failed charge after a warmup only shows an error, so a player who spends the money during the warmup teleports free. The immediate path charges after it starts `nowAsync`, and a move that is cancelled or fails is charged as well (Q36). | `AsyncTeleport.java:290-297`, `342-348`, `AsyncTimedTeleport.java:119-136`, `Trade.java:295-327` | Charge, or reserve, before the move and refund when the move fails |
| S7 | security | No | Text from names is turned into formatting: placeholders convert `§` codes to colors, and the list of homes is inserted as MiniMessage without escaping, safe only because `setHome` sanitizes names. Warp and spawn group names keep any character. Players cannot type `§` today (*outside the checkout*: vanilla disconnects for illegal characters in a command), but the console, command blocks and migrated data can. | `I18n.java:186-191`, `Commandhome.java:109`, `UserData.java:181-186`, `Commandsetwarp.java:24-30`, `Commandsetspawn.java:18` | Every name and every player text is a literal text component, never parsed. Names of homes, warps and groups follow one strict charset that the store enforces |
| S8 | security | Yes | Held back in a private security advisory while EssentialsX runs on the live server. | Held back in a private security advisory while EssentialsX runs on the live server. | One pending request per pair whatever its kind. A new kind replaces the old only after an explicit cancel. The accept names the kind and the direction. The rate limit of S1 counts both commands |
| S9 | security | No | `/setwarp <name>` overwrites an existing warp whose world is not loaded, location and spelling, without `essentials.warp.overwrite.<name>`, since the lookup of the old warp fails and the create branch runs. | `Commandsetwarp.java:35-45`, `Warps.java:56-67`, `77-88` | The overwrite node guards every stored warp, loaded or not. A warp in a missing dimension is an error that names it |

### 7.2 Quirks

| ID | Label | Live | Today | Evidence | Golem |
| --- | --- | --- | --- | --- | --- |
| Q1 | quirk | Yes | At `tpa-max-requests` the newest other pending request is dropped, against the config's own words "old requests will instantly time out". | `User.java:413-417`, `config.yml:759-761` | Drop the oldest request and tell its requester |
| Q2 | quirk | Yes | `/tpaccept` prints "Teleport request accepted." before it checks the request, so the player can read it right before "Error: You do not have a pending request." | `Commandtpaccept.java:42-46`, `78-95` | Confirm only after the checks pass |
| Q3 | quirk | Yes | For a tpahere, "Teleporting..." goes to the requester, who does not move, and the accepter, who moves, hears nothing after "Teleport request accepted." | `Commandtpaccept.java:118-123` | The moved player gets the teleport message |
| Q4 | quirk | Yes | `/tpa` or `/tpahere` to oneself prints the usage instead of saying why. | `Commandtpa.java:27-29`, `Commandtpahere.java:23-25` | A message that one cannot send a request to oneself |
| Q5 | quirk | No | `/tpdeny *` loops forever on the server thread when a listener cancels `TeleportRequestResponseEvent`, since the request is not removed. A single `/tpdeny` also keeps the request of a requester who went offline, which `/tpaccept` removes and `/tpdeny *` removes without telling anyone. | `Commandtpdeny.java:47-50`, `61-69`, `71-76`, `Commandtpaccept.java:84-87` | Every denial removes the request, and no loop runs over a list that does not shrink |
| Q6 | quirk | Yes | `/tpacancel <player>` says nothing when there was no request, and `/tpacancel` counts expired requests as cancelled. | `Commandtpacancel.java:27-44` | Say "no request" and count only valid ones |
| Q7 | quirk | Yes | The last location is written before the teleport is known to work, so a teleport that fails in the safety search or later overwrites the way back. | `AsyncTeleport.java:179-181` before `190-207` | Write the last location from the position before the move, after the move succeeded |
| Q8 | quirk | Yes | `backUsageMsg` goes out before the checks of the teleport, so a cooldown or safety failure follows "Returning to previous location.". | `AsyncTeleport.java:451-452` | The message after success |
| Q9 | quirk | Yes | `/home <unknown name>` teleports to the only home, or to the spawn when there is none, instead of saying the home does not exist. A home whose world is gone gives "Player has not set a home." when no home of the player is in a loaded world, a bare `/home` included, or the list when another one is, and never names the broken home. | `Commandhome.java:71`, `92-110`, `131-138`, `UserData.java:154-161` | An unknown or broken home is an error that names it. Only a bare `/home` falls back |
| Q10 | quirk | Yes | `/home` with no home goes to the spawn of the dimension the player is in, not to the `/spawn` point, and when the bed is gone the respawn path fires a synthetic respawn event to the current world's spawn. In the Nether, the End or the Skylands the world spawn of Bukkit is the Overworld's coordinates in that dimension (*outside the checkout*, vanilla's derived level data, to verify). The plan says "the bed and then the spawn". | `Commandhome.java:80`, `86`, `AsyncTeleport.java:395-411` | Bed first, then the `/spawn` point, as the plan says. The owner confirms the order |
| Q11 | quirk | Yes | Home names are stored through `safeString` but looked up as typed: `/sethome my-base` stores `my_base`, `/home my-base` does not find it, and at the limit `/sethome my-base` is refused although it would only overwrite. `homeSet` does not show the stored name. | `UserData.java:148-152`, `181-186`, `Commandsethome.java:36`, `59`, `93`, `messages.properties:518` | One normalization for set, get, delete and the limit, and the stored name in the reply |
| Q12 | quirk | Yes | `/sethome` is refused as unsafe on a slab, soul sand, farmland, a path, a chest or a bed, because the feet block is `floor(y)` and must be hollow (a path only because the Spigot API does not call it transparent, while the removal the code intends asks for the old name `GRASS_PATH`, section 3.5). The same rule moves a teleport to a player standing on such a block to a nearby spot, possibly behind a wall. | `Commandsethome.java:55-57`, `LocationUtil.java:36`, `63`, `170-184`, `AsyncTeleport.java:190-198` | Judge the space the player's box needs from the exact y, with vanilla's collision shapes |
| Q13 | quirk | Yes | A numeric home name picks the n-th home of an order nobody sees, which for homes set in the session is hash order: `/delhome 1` deletes some home, and `/home 2` goes to some home. | `UserData.java:138-146`, `188-192`, `UserConfigHolder.java:39-44` | No numeric addressing, or numbers shown in the list in a stable order |
| Q14 | quirk | Yes | `/delhome *` deletes every home at once without confirmation. | `Commanddelhome.java:69-74` | Ask for confirmation |
| Q15 | quirk | Yes | `/back` after a death into the void lands on the first safe spot that the search meets climbing from the bottom of the world in the death column, which can be a cave or whatever island lies above the place of the fall. It fails with "Error: Hole in floor!" only when that column and the next 48 columns along x hold no safe spot, as over open void in the End or the Skylands. After a death in lava it lands within 3 blocks of the lava. | `EssentialsEntityListener.java:170-172`, `LocationUtil.java:238-279` with the climb at `262-268` and `holeInFloor` at `269-279` | A death location keeps its dimension and position. The return uses the safety search of S4 and says plainly when no safe spot is near |
| Q16 | quirk | Yes | The home limit of `/sethome player:name` is the setter's, while `/home player:` shows the owner's. With a limit of 1 any name becomes `home` silently. | `Commandsethome.java:47-49`, `90-98`, `Commandhome.java:128` | The owner's limit, and an error instead of a silent rename |
| Q17 | quirk | Yes | A bare `/home` with several homes lists them even when one is named `home`. | `Commandhome.java:30`, `71`, `100-110` | Go to the home named `home` when it exists, else list |
| Q18 | quirk | No | `/warp 0` throws an index error shown as "Error: fromIndex = -20". A warp named with digits beyond the integer range, such as `99999999999`, can be set but not used, since `/warp` takes any digit string as a page. | `Commandwarp.java:30`, `67-79`, `Commandsetwarp.java:24`, `NumberUtil.java:107-114` | Pages from 1, and warp names that cannot be pages |
| Q19 | quirk | Yes | `/tpaccept` bills the cost `tpaccept` to the requester and runs the warmup and cooldown of the requester even when the accepter moves. | `Commandtpaccept.java:106`, `116-123`, `AsyncTeleport.java:238-241` | Cost, cooldown and warmup of the player who moves, decided per teleport |
| Q20 | quirk | Yes | Requests are keyed by account name and live only in memory: a restart drops them, they survive a logout of the target as long as the user cache keeps the target's `User` object (section 3.6), and expiry is lazy, so the requester never learns that a request timed out. | `User.java:73`, `406-421`, `437-489`, `ModernUserMap.java:37-41` | Requests keyed by UUID, dropped on quit of either side, and an expiry on the tick that tells the requester |
| Q21 | quirk | No | The cooldown bypass type is a field that `back` and the tpa commands set and nothing resets, so later teleports use the wrong bypass rule. | `AsyncTeleport.java:31`, `85-102`, `449`, `Commandtpa.java:48`, `Commandtpaccept.java:117`, `126` | The type travels with each teleport |
| Q22 | quirk | No | The world rule checks the wrong player or world: tpaccept of a tpa checks the accepter for the requester's world, tpahere checks the requester's own world. | `Commandtpaccept.java:89`, `93`, `Commandtpahere.java:32` | Check the player who moves for the target dimension |
| Q23 | quirk | No | `/tpauto <name>` with the others node toggles every online player whose name contains the text, when the text is not the exact name of an online player. An exact name toggles that one player (*outside the checkout*, `Server.matchPlayer`). | `EssentialsToggleCommand.java:51-64` | Exactly one player, as in S2 |
| Q24 | quirk | Yes | `/spawn` ends silently when the spawn's world is not loaded. | `Commandspawn.java:70-73`, `LazyLocation.java:59-82` | An error that names the missing dimension |
| Q25 | quirk | Yes | `/setspawn` does not move vanilla's world spawn, and with both listeners off, new players and respawns without a bed use vanilla's world spawn, so `/spawn` and the respawn point drift apart after the first `/setspawn`. | `Commandsetspawn.java:19`, `SpawnStorage.java:37-44`, `EssentialsSpawn.java:43-53` | The owner decides: `setspawn` in the Overworld also sets the world spawn, or the two stay apart and the docs say so |
| Q26 | quirk | No | `/setspawn` takes any text as group: spaces stay, and a dot nests the YAML path, so `spawns.a.b` cannot be read back after a reload and the spawn is lost. | `Commandsetspawn.java:18`, `EssentialsConfiguration.java:354-360`, `EssentialsConfiguration.java:139-151` | A group is one node segment of `golem-permissions`, checked |
| Q27 | quirk | Yes | No warmup and no cooldown let a player leave any fight at once with `/home`, `/spawn`, `/back` or a tpauto friend, and the 4 seconds after a teleport block PvP damage both ways. Whether PvP matters on this server is open. | `config.yml:112`, `116`, `120`, `EssentialsEntityListener.java:81-83` | The owner decides on a combat tag that blocks or delays command teleports for some seconds after damage |
| Q28 | quirk | No | The warmup timer reads player position and health on an async thread, every second, and detects movement in buckets of 3.33 blocks, so moving 3 blocks can pass and a tiny step can cancel. A delay of 0.5 seconds waits a full second. | `AsyncTimedTeleport.java:15`, `48-50`, `58`, `81-98` | A warmup on the server tick with a distance threshold in blocks |
| Q29 | quirk | No | A teleport loads, and if missing generates, the target chunk synchronously on the server thread on Arclight, which can stall a tick for a far warp or home. | `AsyncTeleport.java:188` (*outside the checkout*: PaperLib without Paper) | Let vanilla's ticket system load the chunk for the move, and do the safety search after it is loaded |
| Q30 | quirk | Yes | Messages drop the data they are given: `teleporting`, `backUsageMsg`, `homeSet` and `warpingTo` get the world and position, `homes` the count and limit, and the player sees none of it. Numbers that do show are grouped, 1234 as `1,234`. | `AsyncTeleport.java:130`, `451`, `Commandsethome.java:84`, `Commandhome.java:109`, `I18n.java:196-209` | Show the dimension and the block position where a text promises a place, and numbers without grouping |
| Q31 | quirk | Yes | The success message and the charge follow the call of the move, not its result, so a teleport that a listener cancels or that fails still reads "Teleporting to home." | `AsyncTeleport.java:191-220` | Messages and charges after the `teleported` event of section 8.3 |
| Q32 | quirk | Yes | The compass of every player with a home stops pointing at the world spawn, without a node or a word, and points at the first home even when that lies in another dimension. | `EssentialsPlayerListener.java:618-633`, `UserData.java:163-175`, `config.yml:831` | The owner decides: drop it, or a per-player toggle with the target named |
| Q33 | quirk | Yes | Every dimension change takes away flight and resets walk and fly speed for players without `essentials.fly` and `essentials.speed`, which undoes what mods grant through the abilities. | `EssentialsPlayerListener.java:531-534`, `905-933`, `config.yml:235`, `251` | Not ported. Golem never writes the abilities of a player on a dimension change |
| Q34 | quirk | No | The errors of `/warp <warp> <player>` go to the moved player, not to the sender. | `Commandwarp.java:89-98` | Errors go to the sender |
| Q35 | quirk | No | With `teleport-delay` above 0, a cooldown that has not run out at the end of the warmup only prints "Cooldown: ..." to both players, and the teleport goes ahead. | `AsyncTimedTeleport.java:107-127` | The cooldown is checked before the warmup and at its end, and a failure stops the teleport |
| Q36 | quirk | No | A teleport that a listener cancels or that fails (passengers, an unsafe target without safety, no safe spot) is charged all the same. After a warmup the cancelled one is charged and the failed one is not. | `AsyncTeleport.java:160-162`, `167`, `200`, `205`, `290-297`, `342-348`, `AsyncTimedTeleport.java:128-136`, `Trade.java:295-327` | Charge only after the `teleported` event of section 8.3 reports success, as S6 and Q31 ask |
| Q37 | quirk | No | A request that `/tpaccept` refuses for a missing node of the requester or the world rule, or whose response event a listener cancels, stays queued: each bare `/tpaccept` reads "Teleport request accepted." and "Error: You do not have a pending request." again, and older valid requests cannot be accepted without a name until it expires. Not live, since every player of the group holds `tpa` and `tpahere`, the world rule is off and no listener cancels. | `Commandtpaccept.java:45-46`, `84-104`, `User.java:459-489` | A refused request is removed and its requester told why |
| Q38 | quirk | Yes | `/help /tptoggle` prints the help header and then "Error: " with the Java text of a missing resource, because the usage line of `tptoggle` has no description key. Any player can trigger it once `tptoggle` has been typed or tab completed since the start. | `messages.properties:1441-1444`, `EssentialsCommand.java:51-63`, `Commandhelp.java:55-57`, `Essentials.java:855-856`, `891-897` | Every usage line ships with its description, and a missing text never reaches a player as a Java error |

## 8 What the port needs from the game

The bridge has the events `playerJoined`, `playerLeft`, `chat`, `death` with the id of the damage
type and the place of the death, `respawn` with the dimension and `tick`, the questions of the game
and the actions `command`, `message`, `broadcast`, `teleport` and `listName`, see
[Bridge](../../bridge.md#the-host-class). The teleport side needs more, and
[EssentialsX bridge needs](bridge-needs.md) holds each need of this section against what the
bridge offers.

### 8.1 Queries

| Query | Answer | For |
| --- | --- | --- |
| Position of a player | Dimension id, x, y, z, yaw, pitch | back before every move, tpahere, sethome, setwarp, setspawn, warmup movement |
| Online players | UUID, account name, display name | matching (section 3.3), messages |
| Respawn point of a player | Dimension, position and angle as vanilla resolves it for a living player (bed, charged anchor without using a charge, forced point), or none | `home bed`, spawn-if-no-home |
| World spawn | Position of the Overworld's shared spawn, and of a given dimension | `spawn` without `/setspawn`, Q10 |
| Dimensions | Ids and whether a dimension is the Overworld | the first normal world of `SpawnStorage.java:63-71`, the `back.into` nodes, unloaded dimensions of stored locations |
| Game mode and may fly | Creative, spectator, flight allowed | the safety exemption of `LocationUtil.java:152-155` |
| Health | Current health | warmup cancel on damage, if a warmup is configured |
| Passengers and vehicle | Has passengers, rides something | `AsyncTeleport.java:165-177` |
| Safe location | The nearest safe position to a target, or none, run in Java on the server thread with the chunk loaded | the safety search, S4, Q12 |

Rust may not touch game state from another thread
([The rules of the server thread](../../bridge.md#the-rules-of-the-server-thread)), so a
query is either a call from Rust into the JVM made during a native call on the server thread, or a
state the mod pushes with the events. Which one is a decision of the bridge.

### 8.2 Actions

| Action | Fields | Behaviour the port needs |
| --- | --- | --- |
| `teleport` | A request id, player UUID, dimension, x, y, z, yaw, pitch, flags `safe`, `center`, `eject` | Runs on the server thread: dismounts the player from a vehicle and ejects passengers, loads the chunk through a ticket, runs the safety search when `safe`, centers x and z when `center`, moves across dimensions with `ServerPlayer.teleportTo`, and reports the result as an event |
| `message` with style | Player UUID and a JSON text component | The colors of section 1.2 without `§` codes and without parsing player text, S7 |
| `compass`, only if the owner keeps the compass (Q32) | Player UUID, dimension, x, y, z | Sends the spawn position packet to that one player, so a vanilla compass points at the given home (section 5.8) |

`message` takes plain text ([Actions](../../bridge.md#actions)), which forces `§` codes for colors.
A component form keeps names literal.

### 8.3 Events

| Event | Fields | For |
| --- | --- | --- |
| `teleported` (new) | Request id, player, success or the reason, the position before and after | the last location (Q7), and the success messages and the charge after the result of the move instead of after its call (section 3.5 step 9, Q31) |
| `death` with position (exists) | Dimension, x, y, z, yaw, pitch of the death | `back.ondeath` |
| `respawn` (exists) | Dimension, plus the position | the 4 seconds of invulnerability, if kept |
| `playerLeft` (exists) | Plus the position | drop requests and warmups (Q20), the `logoutlocation` |
| `tick` (exists) | | warmup timers, request expiry |
| `attack` (new, cancellable) | Attacker and victim UUIDs, damage | the invulnerability after a teleport (`EssentialsEntityListener.java:81-83`), the PvP delay after a login (`EssentialsEntityListener.java:77-80`) and a warmup cancel on damage, only if the owner keeps them (Q27) |

### 8.4 Storage

The [store](../../store.md) of the plugins holds, keyed by UUID, the homes of a player (a name and a
location with a dimension id), the last location that `/back` leads to, the logout location and the
ignore list, and it holds a table of the warps with name and location and a table of the spawns per
group. `teleportenabled`, `teleportauto`, the last teleport time and the last owner of a warp have
no column there, so the port adds them to the schema or drops them in the migration. Requests and
warmups stay in memory.

The migration of the plan reads `userdata/*.yml`, `warps/*.yml` and `spawn.yml` with the keys of
section 3.7. A stored location names a Bukkit world by UUID and name (`LazyLocation.java:12-29`), so
the migration needs the map from each Bukkit world of Arclight to its dimension id, read from the
live server (section 9). Home names are already `safeString` names. Warp names keep their case and
need the charset of S7 or a rename list.

### 8.5 Permissions

`golem-permissions` answers undefined for a node nobody names, so the port defines the defaults
`essentials.back.onteleport` true, `essentials.back.into.<dimension>` true,
`essentials.teleport.cooldown.bypass.tpa` true and `essentials.teleport.cooldown.bypass.back` true
itself (section 2.2), unless it drops the split of the cooldown bypass per type. A dimension id such
as `minecraft:the_nether` holds a `:`, which a node segment does not allow
([The configuration](../permissions.md#the-configuration)), so nodes that name a dimension need a
mapping, for example `essentials.back.into.minecraft.the_nether`. The same holds for
`essentials.worlds.<world>` and `essentials.warps.<warp>` with upper case warp names.

### 8.6 Commands

The bridge registers each command with one greedy string argument
([Commands](../../bridge.md#commands)), so the
tab completion of names, homes and warps that EssentialsX offers (`Commandhome.java:154-185`,
`Commandwarp.java:111-132`, `Commandtpaccept.java:67-76`) is gone unless the bridge grows
suggestions. The answer of `command()` is synchronous, so every message that depends on the move
comes later through `message` after the `teleported` event.

## 9 Open points to check on the live server

- The values of `respawn-listener-priority` and `spawn-join-listener-priority` in the live
  `plugins/Essentials/config.yml`, which the default file sets to `high` (section 1.1).
- The Bukkit world names and UUIDs of Arclight for the Overworld, the Nether, the End and the
  Skylands, and their environments, for the migration and for "the first normal world".
- The spawn location that Bukkit reports for the Nether, the End and the Skylands (Q10).
- How Arclight's `Material.isTransparent` judges the blocks of the mods that players stand in or
  next to, and whether it leaves out `DIRT_PATH` and `FARMLAND` as the Spigot API 1.20.1 does
  (section 3.5, S4, Q12).
- Whether PvP is on (Q27).
- Whether the mods that give flight through the player's abilities, such as Doom Angel Ring 3.0.1,
  restore it after the reset of a dimension change (section 3.5, Q33).
- Whether `plugins/Essentials/warps/` and `spawn.yml` exist on the Mac mini. The build machine has
  neither (plugins.md line 37), and plugins.md lines 96 to 99 record no use of `/spawn` up to
  2026-11-01 01:20.
- The behaviour of LuckPerms for nodes it does not name (section 2.2), of Bukkit's `getPlayer` and
  `matchPlayer`, with the rule that an exact name makes `matchPlayer` answer that one player
  (section 3.3, Q23), of Bukkit's bed spawn (section 3.5), of Bukkit's `setCompassTarget`
  (section 5.8) and of PaperLib without Paper (Q29), all outside the checkout.

## Sources

The page rests on the sources below, as read on 2026-11-06.

| Source | What the page takes from it |
| --- | --- |
| The checkout of EssentialsX 2.22.0 at commit `100f575293afff1c2bc8e1468ce1cffc153319fc` | Every `file:line` citation of sections 1 to 9 (section 1) |
| `spigot-api-1.20.1-R0.1-20230921.163938-66-sources.jar`, the Spigot API 1.20.1 sources | `org/bukkit/Material.java:1219` (`DIRT_PATH`) and `6809-6998` (`isTransparent`), and `org/bukkit/Server.java:538-549` (the javadoc of `matchPlayer`), for sections 3.3 and 3.5 |
| [`okf/modpack/plugins.md`](https://github.com/skytekx/skytekx3-1201-server/blob/3fa5ae681d12233fe81003fbfe5a2d3c529747fc/okf/modpack/plugins.md) of `skytekx/skytekx3-1201-server` at commit `3fa5ae68` | The live config (line 37), the group `default` (lines 64 to 69), the listener priorities (lines 89 to 91), the world spawn (line 93) and the use of `/spawn` (lines 96 to 99) |
| `modrinth.index.json` of the server checkout | Doom Angel Ring 3.0.1 among the mods that give flight (section 3.5) |
| [Plan](../../plan.md), [Permissions](../permissions.md) and [Bridge](../../bridge.md) of Golem at commit `f630fb1` | The plan, the rules of `golem-permissions` and the bridge |

The page cites no line of the Arclight jar of the live server or of forge-1.20.1-47.4.18. What
Arclight, CraftBukkit, PaperLib, LuckPerms and vanilla do beyond the sources above is marked
*outside the checkout* and listed in section 9.

Every command, alias and usage line of sections 4 to 6 matches `plugin.yml` and
`spawn/plugin.yml`. The citations below are checked against the checkout and hold.

| Citation | What it holds |
| --- | --- |
| `build.gradle:6` | The version 2.22.0 |
| `Settings.java:1501-1517` | `teleport-invulnerability` read in seconds and kept in milliseconds |
| `config.yml:749`, `757`, `761`, `808`, `835` | `teleport-when-freed: back`, `tpa-accept-cancellation: 120`, `tpa-max-requests: 5`, `world-home-permissions: false`, `spawn-if-no-home: true` |
| `User.java:406-421` | The requester's entry is reused, and at the limit the last key, the newest other request, is removed |
| `Commandtpaccept.java:89`, `93` | The tpahere and tpa checks of the requester's nodes and of the world rule |
| `LocationUtil.java:238-244` | The descent gives up below y = 0 and returns to the original y |
| `Commandwarp.java:30-36` | No argument or a digit string lists, `warpListPermission` without `warp.list` |
| `SpawnStorage.java:46-71` | Group spawn, then `default`, then the first world of environment `NORMAL` |
| `EssentialsEntityListener.java:159-177` | The death position becomes the last location for holders of `back.ondeath`, and the same handler reads `death-messages` (section 5.1) |
| `messages.properties:1301`, `1295`, `1104`, `872`, `516`, `1490`, `760` | Texts of `teleportRequest`, `teleportHereRequest`, `requestTimedOutFrom`, `noLocationFound`, `homeConfirmation`, `unsafeTeleportDestination`, `maxHomes` |
| `Commanddelhome.java:69-74` | `*` deletes every home without confirmation |
| `PermissionsDefaults.java:17-33` | `essentials.back.into.<world>` registered with default true per world |
| `UserConfigHolder.java:377` | `timestamps.lastteleport`, default 0 |
| `plugin.yml:1331-1332` | The description of `essentials.tpacancel` names `/tpaccept` |
| `plugin.yml:1561-1567` | `essentials.teleport.cooldown.bypass.tpa` and `.back` with `default: true` |
| `spawn/plugin.yml:11-18` | `setspawn` and `spawn` with their aliases and usage |

## Rulings and issues

This table rules on every entry of section 7, against the checkout of EssentialsX 2.22.0 and the
sources of [Sources](#sources). Each entry that holds as a quirk has its issue in `skytekx/golem`.
The entries that a player can use on the live server today stay in private security advisories
until EssentialsX no longer runs there, so their text above is held back too. The kind is the
ruling, which can differ from the label in the column Label of section 7.

| ID | Ruling | Issue |
| --- | --- | --- |
| S1 | Security, live | Private advisory |
| S2 | Security, live | Private advisory |
| S3 | Quirk | [#33](https://github.com/skytekx/golem/issues/33) |
| S4 | Quirk | [#28](https://github.com/skytekx/golem/issues/28) |
| S5 | Quirk | [#34](https://github.com/skytekx/golem/issues/34) |
| S6 | Refuted. A player who spends the money during the warmup is not teleported, because `AsyncTimedTeleport.java:119-121` checks the cost again right before the move. | None |
| S7 | Quirk | [#35](https://github.com/skytekx/golem/issues/35) |
| S8 | Security, live | Private advisory |
| S9 | Security | [#36](https://github.com/skytekx/golem/issues/36) |
| Q1 | Quirk | [#37](https://github.com/skytekx/golem/issues/37) |
| Q2 | Quirk | [#38](https://github.com/skytekx/golem/issues/38) |
| Q3 | Quirk | [#39](https://github.com/skytekx/golem/issues/39) |
| Q4 | Quirk | [#40](https://github.com/skytekx/golem/issues/40) |
| Q5 | Quirk | [#41](https://github.com/skytekx/golem/issues/41) |
| Q6 | Quirk | [#42](https://github.com/skytekx/golem/issues/42) |
| Q7 | Quirk | [#43](https://github.com/skytekx/golem/issues/43) |
| Q8 | Quirk | [#44](https://github.com/skytekx/golem/issues/44) |
| Q9 | Quirk | [#29](https://github.com/skytekx/golem/issues/29) |
| Q10 | Quirk | [#45](https://github.com/skytekx/golem/issues/45) |
| Q11 | Quirk | [#46](https://github.com/skytekx/golem/issues/46) |
| Q12 | Quirk | [#47](https://github.com/skytekx/golem/issues/47) |
| Q13 | Quirk | [#48](https://github.com/skytekx/golem/issues/48) |
| Q14 | Quirk | [#49](https://github.com/skytekx/golem/issues/49) |
| Q15 | Quirk | [#50](https://github.com/skytekx/golem/issues/50) |
| Q16 | Quirk | [#51](https://github.com/skytekx/golem/issues/51) |
| Q17 | Quirk | [#52](https://github.com/skytekx/golem/issues/52) |
| Q18 | Quirk | [#53](https://github.com/skytekx/golem/issues/53) |
| Q19 | Quirk | [#54](https://github.com/skytekx/golem/issues/54) |
| Q20 | Quirk | [#55](https://github.com/skytekx/golem/issues/55) |
| Q21 | Quirk | [#30](https://github.com/skytekx/golem/issues/30) |
| Q22 | Quirk | [#56](https://github.com/skytekx/golem/issues/56) |
| Q23 | Quirk | [#57](https://github.com/skytekx/golem/issues/57) |
| Q24 | Quirk | [#58](https://github.com/skytekx/golem/issues/58) |
| Q25 | Quirk | [#59](https://github.com/skytekx/golem/issues/59) |
| Q26 | Quirk | [#60](https://github.com/skytekx/golem/issues/60) |
| Q27 | Quirk | [#61](https://github.com/skytekx/golem/issues/61) |
| Q28 | Quirk | [#62](https://github.com/skytekx/golem/issues/62) |
| Q29 | Quirk | [#63](https://github.com/skytekx/golem/issues/63) |
| Q30 | Quirk | [#64](https://github.com/skytekx/golem/issues/64) |
| Q31 | Quirk | [#65](https://github.com/skytekx/golem/issues/65) |
| Q32 | Quirk | [#66](https://github.com/skytekx/golem/issues/66) |
| Q33 | Quirk | [#67](https://github.com/skytekx/golem/issues/67) |
| Q34 | Quirk | [#68](https://github.com/skytekx/golem/issues/68) |
| Q35 | Quirk | [#69](https://github.com/skytekx/golem/issues/69) |
| Q36 | Merged into Q31, which has the same cause and fix. | None |
| Q37 | Quirk | [#70](https://github.com/skytekx/golem/issues/70) |
| Q38 | Quirk | [#32](https://github.com/skytekx/golem/issues/32) |
