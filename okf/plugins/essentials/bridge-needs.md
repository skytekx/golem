---
# SPDX-License-Identifier: GPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
# SPDX-FileCopyrightText: 2011 Essentials Team
# SPDX-FileCopyrightText: EssentialsX contributors
type: Spec
title: EssentialsX bridge needs
description: One table of every query, action, event and storage need of the port of EssentialsX 2.22.0 and EssentialsXSpawn 2.22.0, with the commands that need each, what the bridge contract of Golem offers for it and what is missing.
tags: [plugins, essentials, essentialsx, spawn, bridge, port, gaps]
timestamp: 2026-11-06
---

# EssentialsX bridge needs

This page collects what `golem-essentials` and `golem-essentials-spawn` need from the game through
the bridge, from the two parts of the spec, [teleport.md](teleport.md) and [social.md](social.md),
and the checkout of EssentialsX 2.22.0 at commit `100f575293afff1c2bc8e1468ce1cffc153319fc`. It is
part of the port spec and keeps the license of EssentialsX, the GNU GPL 3.0 or later, like the two
parts.

Each need is held against the contract of [Bridge](../../bridge.md) of Golem at commit `f630fb1`,
with the plugin runtime of [Plugins](../index.md) and the core plugin of [Core plugin](../core.md)
beside it. The contract offers this.

| Kind | What [Bridge](../../bridge.md) offers |
| --- | --- |
| Events | `playerJoined(uuid, name)`, `playerLeft(uuid, name)`, `chat(uuid, name, message)`, which can only cancel and runs on a worker thread, `death(uuid, name, cause, dimension, x, y, z, yaw, pitch)`, `respawn(uuid, name, dimension)` and `tick(nanos)` |
| Commands | `command(name, uuid, senderName, level, arguments)` for every name of `commandNames()`, with the permission level 0 to 4 of the source, which is a player who runs the command themself, the console or another source such as a command block, a command that another source runs as a player through `execute as` included. The mod registers each name as a Brigadier literal with one greedy string argument and no requirement |
| Queries | The interface `Game`, which Rust asks while a native of the server thread runs: `players()`, `location(uuid)`, `respawn(uuid)`, `spawn(dimension)`, `mainHand(uuid)`, `heights(dimension)` and `safeLocation(dimension, x, y, z, radius)` |
| Actions | `command`, `message` and `broadcast` with plain text, `teleport` with the UUID, the dimension, x, y, z, yaw and pitch, run as the command `tp` runs it, and `listName` with a UUID and a name or the empty string |
| Permissions | The handler `golem:permissions` of the Forge `PermissionAPI`, which asks the native `permission(uuid, node)`, and one warning of the mod when Forge takes another handler |

The column Bridge of each table below names what of this the need uses, or says Missing. The column
Gap names what the need still lacks. "Decide" marks a need that only exists if Golem keeps a
behaviour that the teleport part or the social part recommends to drop or puts to the owner. "Not
needed" marks a need that the port drops, with the reason.

The rule of the server thread
([The rules of the server thread](../../bridge.md#the-rules-of-the-server-thread)) shapes every
query. The game answers only while a native of the server thread runs, that is during `command`,
`tick` and every event native but `chat` and `serverStopped`. From `serverStopped` on every question
answers none, since Minecraft closed every level before it. `chat` runs on a worker thread, where
every question answers none, so mute and ignore need only the plugin's own state there, and the
activity of the chat must be queued for the next tick. `permission` runs on the thread of the mod
that asks Forge, so a plugin answers it from what it holds.

The last column of each table names the Bukkit API that the Java code uses for the need. The plan
wants a Golem equivalent of the whole Bukkit API (the ruling of 2026-11-06 in
[Bukkit plugins](../../plan.md#bukkit-plugins)), so these rows are also the first entries of its
coverage check.

## Queries

| Query | Answer | Commands and features | Java source | Bridge | Gap | Bukkit API |
| --- | --- | --- | --- | --- | --- | --- |
| Online players | UUID, account name, display or list name | Name matching of `tpa`, `tpahere`, `tpaccept`, `tpdeny`, `tpacancel`, `tpauto`, `back`, `home`, `sethome`, `delhome`, `warp`, `spawn`, `msg`, `r`, `mail`, `ignore`, `seen`, `realname`, `getpos`, `afk`, `balance`, `pay`, `eco`, `gamemode`, `kick`, `ban`. Also `list`, `helpop` and the notices, every broadcast, and `{ONLINE}` of the MOTD | `Essentials.java:985-1064`, `PlayerList.java:53-90`, `Essentials.java:1209-1232` | `players()`: the UUID, the name of the profile, the display name, the name in the player list, the ping, the game mode, the operator level and the milliseconds since the last action. `player(who)` of [Plugins](../index.md#the-game) finds one online player by name or UUID in any case | None. The prefix and display name matching of `matchUser` runs in the plugin over `players()`. The operator level serves `ops-name-color` and any op based default that survives the port | `Server.getOnlinePlayers`, `getPlayerExact`, `getPlayer(String)`, `matchPlayer` |
| Position of a player | Dimension id, x, y, z, yaw, pitch | `back` (before every move), `tpahere` (at sending), `sethome`, `setwarp`, `setspawn`, `getpos`, `compass` (yaw), `depth` (block y), AFK on move, the logout location, a warmup | `AsyncTeleport.java:179-181`, `User.java:410`, `Commandsethome.java:54`, `Commandgetpos.java:35-45`, `Commandcompass.java`, `Commanddepth.java:13`, `EssentialsPlayerListener.java:234-286`, `UserData.java:292-303` | `location(uuid)`: the dimension, x, y, z of the feet, yaw and pitch, null for a player who is not online | None | `Entity.getLocation` |
| Respawn point | Dimension, position and angle, or none, as vanilla resolves it for a living player without using an anchor charge | `home bed`, `/home` without homes (`spawn-if-no-home`), `bed` or `bedNull` in the list of homes, the compass fallback | `Commandhome.java:51`, `79-84`, `116`, `AsyncTeleport.java:395-411`, `EssentialsPlayerListener.java:624-629` | `respawn(uuid)`: the dimension and x, y, z of the respawn point that the player holds, which a bed, a respawn anchor or `/spawnpoint` set, as Minecraft stores it, empty for a player without a respawn point | The angle, and the validation that vanilla makes on a respawn, since the contract answers the stored point unchecked (`GameQueries.java:96-106`): a bed that is gone or blocked is none, a charged anchor is kept, a forced point of `/spawnpoint` counts while its place is free | `Player.getBedSpawnLocation`, `PaperLib.getBedSpawnLocationAsync` |
| Shared spawn | The spawn of the Overworld, and of a given dimension | `spawn` before any `/setspawn`, `/home` without homes (the spawn of the current dimension), the respawn path without a bed | `SpawnStorage.java:63-71`, `Commandhome.java:80`, `86`, `AsyncTeleport.java:400-407` | `spawn(dimension)`: x, y, z and the angle of the shared spawn, null for a dimension the server does not have. The Nether and the End answer the spawn of the Overworld, since Minecraft keeps one spawn for the worlds of a server | None, as Golem uses the `/spawn` point as the fallback (teleport part, Q10). A spawn of its own for another dimension does not exist in Minecraft | `World.getSpawnLocation` |
| Dimensions | Ids, whether one is the Overworld, whether it is loaded | The first `NORMAL` world of `SpawnStorage`, the nodes `essentials.back.into.<dimension>`, stored locations in a dimension that is not loaded, the migration map of Bukkit worlds | `SpawnStorage.java:63-71`, `PermissionsDefaults.java:17-33`, `LazyLocation.java:59-88` | Missing. `spawn(dimension)` and `heights(dimension)` answer null for a dimension the server does not have, so a plugin can test an id it knows | A list of the dimensions, small. It could join the shared spawn query | `Server.getWorlds`, `World.getEnvironment` |
| Held item and inventory | Main hand, off hand and every stack: registry id, count, damage, whether it is a block | `worth`, `worth hand`, `worth inventory`, `worth all`, `worth blocks` | `Commandworth.java:24-56`, `Worth.java:78-132` | `mainHand(uuid)`: the registry id of the item and the count, empty for an empty hand | The off hand, every other stack, the damage and the block flag, and the item registry for `worth <name>`, or Golem's own alias table | `Player.getInventory`, `getItemInMainHand`, `getItemInOffHand` |
| Height bounds | Minimum y, maximum y, logical height of a dimension | The safety search, if it runs in Rust, and a check of stored and migrated locations | `LocationUtil.java:222-227` | `heights(dimension)`: the lowest y of a block and the y above the highest | The logical height, which the safety search of EssentialsX takes as its top for a target below it (`LocationUtil.java:226-227`). None if the port uses `safeLocation` | `World.getMinHeight`, `getMaxHeight`, `getLogicalHeight`, through the `WorldInfoProvider` of EssentialsX (`ModernDataWorldInfoProvider.java:11-22`) |
| Safe location | The nearest safe position to a target, or none | `home`, `back`, `warp`, `spawn`, `tpa`, `tpaccept` with `teleport-safety: true`, and the refusal of `/sethome` on an unsafe spot | `LocationUtil.java:152-164`, `203-281`, `AsyncTeleport.java:190-207`, `Commandsethome.java:55-57` | `safeLocation(dimension, x, y, z, radius)`: the nearest safe place within a sphere of at most 8 blocks, with a search of the mod's own that keeps the place inside the world border and reads loaded chunks alone, and answers `unloaded` at the first place of a chunk that is not loaded | A target in a chunk that is not loaded, such as a far home or warp, gets `unloaded` until a player or a ticket of the game loads the chunk, so a teleport there finds no safe place (teleport part, Q29). The search belongs in the `teleport` action as a flag, after the action loaded the chunk, answered by the `teleported` event below, and the query stays for `/sethome`, whose chunk is loaded. The search of the mod gives no answer beyond its sphere, where `LocationUtil.java:238-279` also climbs the column and walks up to 48 columns along x | `World.getBlockAt`, `Material.isTransparent`, `World.getWorldBorder` |
| World border | Center and size of a dimension's border | The clamp of a target into the border, the safety check | `AsyncTeleport.java:183-187`, `LocationUtil.java:120-150` | Inside `safeLocation`, whose place lies inside the border. Missing as a query, and the `teleport` action takes the place as it is | The clamp of a target into the border before the move, as a query of center and size or in the `teleport` action. None for a teleport that asks `safeLocation` first | `World.getWorldBorder` |
| Game mode and flight | Game mode, whether the player may fly | The safety exemption for creative, spectator and flying players, and the toggle of `gmt` | `LocationUtil.java:152-155`, `283-301`, `Commandgamemode.java:53-55` | The game mode in `players()`. Missing: whether the player may fly | Whether the player may fly | `Player.getGameMode`, `getAllowFlight` |
| Sea level and day time | Per dimension | `depth`, and `{WORLDTIME12}` of the live MOTD | `Commanddepth.java:13`, `KeywordReplacer.java:327-331` | Missing | Both | `World.getSeaLevel`, `World.getTime` |
| Maximum players | One number | `list` | `PlayerList.java:66-75` | Missing | The number | `Server.getMaxPlayers` |
| Health | Current health | The cancel of a warmup on damage, only with `teleport-delay` above 0 (live 0) | `AsyncTimedTeleport.java:41-56`, `94-98` | Missing | Not needed for the live config | `Player.getHealth` |
| Permission level of a source | 0 to 4 | Who may run an admin command from a command block, a function or `/execute` | The sources of a command in [The host class](../../bridge.md#the-host-class) | `level` of `command`, and the operator level of each online player in `players()` | None | `CommandSender.isOp`, `Permissible.hasPermission` |
| Ban entry and whitelist | Name ban with reason, source and expiry, and the whitelist flag | `seen` for operators, `ban`, the login texts | `Commandseen.java:143-201`, `Commandban.java:53`, `EssentialsPlayerListener.java:636-662` | Missing | Admin only | `Server.getBanList`, `OfflinePlayer.isWhitelisted` |
| Address at login | Remote address | `seen` with `essentials.seen.ip`, the user's `ip-address` | `UserData.java:553-559`, `Commandseen.java:139-141` | Missing | Operators only. Decide whether Golem stores addresses at all | `Player.getAddress` |

## Actions

| Action | Fields | Commands and features | Java source | Bridge | Gap | Bukkit API |
| --- | --- | --- | --- | --- | --- | --- |
| Styled message | Player UUID and a text component, with player content as literal text | Every command: every key of `messages.properties` is MiniMessage with the colors `#ffaa00` and `#ff5555` | `User.java:1127-1146`, `I18n.java:171-209` | `message` sends one plain text to a player as a system message, and `broadcast` to every player and the console | A component form of `message` and `broadcast`. Plain text needs `§` codes for colors, which the [open items](../../open-items.md#open-in-what-is-built) of Golem warn about | `CommandSender.sendMessage`, Adventure |
| Broadcast with exclusions | Text and the players to leave out, or a list of recipients | `afk` and `suicide` broadcasts (players who ignore the sender are left out), `helpop` to holders of `essentials.helpop.receive`, the notices of `kick` and `ban`, `me` | `Essentials.java:1209-1232`, `Commandhelpop.java:43-62` | `broadcast` goes to every player and the console | None strictly: one `message` per recipient works. A broadcast with exclusions saves the actions | `Server.broadcast`, per player `sendMessage` |
| Teleport | Player UUID, dimension, x, y, z, yaw, pitch, a request id, the flags safe and center | `tpa` with tpauto, `tpaccept`, `back`, `home`, `home bed`, `warp`, `spawn` | `AsyncTeleport.java:155-225`, `LocationUtil.java:187-193` | `teleport` with the UUID, the dimension, x, y, z, yaw and pitch, without request id and flags, run as the command `tp` runs it: it posts the teleport event of Forge, which a mod may cancel, skips a place outside the bounds of the world, wraps the angles, keeps the chunk of the place loaded for a moment with the ticket `post_teleport`, lets a rider dismount, wakes a sleeping player, stops the fall and moves the player into another dimension with `teleportTo` | A request id and the `teleported` event below, so the plugin learns success or failure. The flag safe (see Safe location). Center can be done in Rust. Passengers of the player are not ejected by vanilla's path (outside the checkout), which matters only for `teleport-passenger-dismount` | `Player.teleport`, `PaperLib.teleportAsync`, `Entity.eject` |
| Tab list name | Player UUID, name or empty | The AFK mark that the plan wants (live `afk-list-name: "none"`, so a change), nicknames if ported | `User.java:703-713`, `config.yml:550` | `listName` with the UUID and a name, or the empty string, which clears it. The name wins over the name of another mod and holds until Golem clears it or the player leaves | None for a plain mark. Colors need a component | `Player.setPlayerListName` |
| Console command | The command | Fallback for `kick`, `ban`, `gamemode`, `kill` | [Actions](../../bridge.md#actions) | `command`, run as the console | None | `Server.dispatchCommand` |
| Kick | Player UUID, reason as a component | `kick`, and the AFK kick (off, `auto-afk-timeout: -1`) | `Commandkick.java:52`, `User.java:868-916` | Missing. `command kick ...` works with vanilla's own notice | Only if Golem keeps Essentials' notice to holders of `essentials.kick.notify` | `Player.kickPlayer` |
| Ban | Profile name, reason, source, expiry | `ban` | `Commandban.java:44-57` | Missing. `command ban ...` loses the source and needs a resolvable profile | Admin only | `BanList.addBan` |
| Set game mode | Player UUID, mode | `gamemode` and all its forms | `Commandgamemode.java:62`, `76` | Missing as its own action. `command gamemode <mode> <name>` works and gives vanilla's notices, which the plan accepts | None | `Player.setGameMode` |
| Kill with a damage type | Player UUID, damage type | `suicide` | `Commandsuicide.java:17-26` | Missing. `command kill <uuid>` gives `genericKill` and an operator notice | The action | `EntityDamageEvent`, `Player.setHealth` |
| Sleeping ignored | Player UUID, on or off | AFK players with `sleep-ignores-afk-players: true`, and operators on join through `essentials.sleepingignored` | `User.java:690`, `EssentialsPlayerListener.java:448-449` | Missing | Not needed. Arclight 1.0.6 never reads the flag that `setSleepingIgnored` writes, so operators and AFK players count for sleeping today, and the port keeps that (social part, Q37). Forge 1.20.1-47.4.18 has no event that drops a player from the count of `SleepStatus`: its sleep events are `PlayerSleepInBedEvent`, `PlayerWakeUpEvent`, `SleepingLocationCheckEvent`, `SleepingTimeCheckEvent` and `SleepFinishedTimeEvent`, and none of its patches touches `SleepStatus`, so the behaviour of the Java code would need a mixin | `Player.setSleepingIgnored` |
| Compass target | Player UUID, position | The compass that points at a home on join, respawn and dimension change | `EssentialsPlayerListener.java:618-633` | Missing | Decide (teleport part, Q32). The spawn position packet to one player would do | `Player.setCompassTarget` |
| Chat to a subset | The verdict of `chat` names the players who must not get the message | Ignore in chat | `EssentialsPlayerListener.java:214-228` | Missing. `chat(uuid, name, message)` can only cancel | A filter of the recipients. EssentialsX removes from the recipient set every player who ignores the sender, and Arclight 1.0.6 hands it a set it can change, so chat ignore works today (social part, Q31) | `AsyncPlayerChatEvent.getRecipients` |
| Command tree rules | A requirement per command that asks `golem-permissions`, and a resend of the tree when permissions change | Every command, so that players see only what they may use | `EssentialsPlayerListener.java:1270-1308` | Missing. The mod registers every plugin command without a requirement, and the plugin checks the sender itself | The requirement and the resend. The level reaches the plugin, but Brigadier still shows every command | `PlayerCommandSendEvent`, `Player.updateCommands` |
| Replace vanilla nodes | Remove the vanilla literal before the plugin's is registered | `msg`, `tell`, `w`, `me`, `list`, `kick`, `ban`, `gamemode`, `help`, which Brigadier would otherwise merge so that vanilla keeps its arguments | `PluginCommands.java:47-61` of Golem, social part Q2 | Missing. The mod registers every name with `dispatcher.register` (`PluginCommands.java:55`), and Brigadier 1.1.8 `CommandNode.addChild` merges the plugin literal into the vanilla node of the same name: it keeps the vanilla requirement and children and replaces only the command of the literal alone | The removal of the vanilla literal before the plugin's is registered, for every name that a plugin owns | `commands.yml` aliases, the Bukkit command map |
| Suggestions | Names of homes, warps, players, modes | Tab completion that EssentialsX offers | `Commandhome.java:154-185`, `Commandwarp.java:111-132`, `Commandtpaccept.java:67-76` | Missing. Each command has one greedy string argument `arguments` | The suggestions | `TabCompleter` |
| Login verdict | Allow, or refuse with a reason | The ban texts and `serverFull`, and operators passing a full server | `EssentialsPlayerListener.java:636-676` | Missing | Decide: dropping keeps vanilla's texts and rule | `PlayerLoginEvent` |
| Flight and speed | Player UUID, may fly, walk and fly speed | The resets on join and dimension change | `EssentialsPlayerListener.java:531-534`, `905-933` | Missing | None. The teleport part recommends not porting them (Q33) | `Player.setAllowFlight`, `setWalkSpeed`, `setFlySpeed` |

## Events

| Event | Fields | Commands and features | Java source | Bridge | Gap | Bukkit API |
| --- | --- | --- | --- | --- | --- | --- |
| Join | UUID, name | MOTD, mail notice, `seen` (login time, past names), the index of names, AFK reset, the compass (Decide), sleeping ignored for operators (not needed, social part Q37) | `EssentialsPlayerListener.java:400-616` | `playerJoined(uuid, name)` | The address only if Golem keeps it. The first join is in the Golem store | `PlayerJoinEvent` |
| Quit | UUID, name, and the position | `logoutlocation`, `timestamps.logout`, the drop of teleport requests and warmups (teleport part, Q20), AFK, reply targets | `EssentialsPlayerListener.java:289-342` | `playerLeft(uuid, name)`, without the position | The position, or a rule in the contract that `location(uuid)` answers during `playerLeft` | `PlayerQuitEvent` |
| Death | UUID, name, cause, dimension, x, y, z, yaw, pitch | `back.ondeath` and `backAfterDeath`, and `infoAfterDeath` (off) | `EssentialsEntityListener.java:159-177` | `death(uuid, name, cause, dimension, x, y, z, yaw, pitch)`, where `cause` is the id of the damage type | None | `PlayerDeathEvent` |
| Respawn | UUID, name, dimension | The display name, the compass (Decide), 4 seconds of PvP protection (Decide, teleport part Q27) | `EssentialsPlayerListener.java:180-189` | `respawn(uuid, name, dimension)` | None. The position comes from `location(uuid)` | `PlayerRespawnEvent` |
| Chat | UUID, name, text | Mute, ignore, AFK activity | `EssentialsPlayerListener.java:191-232` | `chat(uuid, name, message)`, cancel only, on a worker thread where every question answers none | The filter of the recipients of Chat to a subset above | `AsyncPlayerChatEvent` |
| Tick | Nanoseconds | Request expiry, warmups (unused live), the AFK timer every 50 ticks, polling positions for AFK on move | `EssentialsTimer.java:30-68`, `Essentials.java:432-433` | `tick(nanos)` | None | `BukkitScheduler` |
| Teleported | Request id, success or reason, position before and after | The last location after a move that worked (teleport part, Q7), the success messages and the charge after it (Q31) | `AsyncTeleport.java:179-220` | Missing | The event, the most needed one of this page | `PlayerTeleportEvent`, the future of `teleportAsync` |
| Dimension change | UUID, from, to | AFK on a change of world, the compass and the resets (both Decide) | `EssentialsPlayerListener.java:234-286`, `905-962` | Missing | None if AFK polls positions and the compass and resets are dropped | `PlayerChangedWorldEvent` |
| Activity | Interact, attack, bow shot, fishing | AFK activity with `cancel-afk-on-interact`, `-on-fish` | `EssentialsPlayerListener.java:966-1013`, `1193-1201`, `EssentialsEntityListener.java:63`, `71`, `256-264` | Missing as events. `players()` answers the milliseconds since the last action, which the game resets when the player moves, chats, uses an item or clicks | The events, needed by the automatic AFK that the plan wants (live it never fires, social part Q21), unless the idle time of `players()` stands in for them | `PlayerInteractEvent`, `EntityDamageByEntityEvent`, `EntityShootBowEvent`, `PlayerFishEvent` |
| Attack, cancellable | Attacker and victim UUIDs | 4 seconds of PvP protection after a teleport and a respawn, 5 seconds after a login (`login-attack-delay`), a warmup cancel on damage | `EssentialsEntityListener.java:77-83` | Missing | Decide (teleport part, Q27, and the PvP delay after a login in section 3.5). A Forge `LivingAttackEvent` listener would do | `EntityDamageByEntityEvent` |
| Every player command | UUID, the command line | AFK activity, social spy of commands (operators), `mute-commands` | `EssentialsPlayerListener.java:777-903` | Missing. Only the commands of the plugins reach Rust, through `command` | The event. Forge's `CommandEvent` gives it | `PlayerCommandPreprocessEvent` |
| Login | Profile, address | The login verdict above | `EssentialsPlayerListener.java:636-676` | Missing | Decide | `PlayerLoginEvent` |

## Storage

All of it lives in the Golem store, keyed by UUID, and nothing needs the game. The migration of the
plan reads the EssentialsX files once.

| Store | Fields | Commands | Source keys |
| --- | --- | --- | --- |
| Players | UUID, last account name, past names, login and logout times, logout location, address if kept | `seen`, every offline lookup (`mail send`, `pay`, `eco`, `balance`, `ban`, `ignore`, `home <player>:`) | `last-account-name`, `past-usernames`, `timestamps.login`, `timestamps.logout`, `logoutlocation`, `ip-address`, and `usermap.bin` |
| Homes | Owner, normalized name, location with dimension id | `home`, `sethome`, `delhome`, the compass | `homes.<name>` |
| Last location | Owner, location | `back` | `lastlocation` |
| Toggles | `teleportenabled`, `teleportauto`, `accepting-pay`, `confirm-pay`, `last-message-reply-recipient`, `socialspy`, `baltop-exempt` | `tpa`, `tpahere`, `tpauto`, `tptoggle`, `pay`, `paytoggle`, `payconfirmtoggle`, `r`, `rtoggle`, `socialspy`, `balancetop` | The same keys |
| Ignore lists | Owner, ignored UUIDs | `ignore`, `msg`, `r`, `mail`, chat, the AFK and suicide broadcasts, `tpa` with tpauto | `ignore` |
| Last teleport | Owner, milliseconds | The teleport cooldown (live 0, still stamped) | `timestamps.lastteleport` |
| Mail | Recipient, sender name and UUID, text, time, expiry, read, legacy | `mail`, the notice on join and before commands | `mail` |
| Accounts | UUID, balance in the smallest unit | `balance`, `balancetop`, `pay`, `eco` | `money` |
| Warps | Name, location, last owner | `warp`, `setwarp`, `delwarp` | `warps/<name>.yml` |
| Spawns | Group, location | `spawn`, `setspawn` | `spawn.yml`, `spawns.<group>` |
| Texts and tables | MOTD, rules, the worth table by registry id | `motd`, `rules`, `worth` | `motd.txt`, `rules.txt`, `worth.yml` |
| Memory only | Teleport requests, warmups, reply targets, AFK state, position and time, pay confirmations, `msgtoggle`, home overwrite confirmations | As today, lost on a restart | `User.java:68-98` |
| Migration only | Bukkit world UUID and name to dimension id, and the explicit drop list of the keys that no page maps (social part, section Storage) | Every stored location | `LazyLocation.java:12-29` |

## Permissions

| Need | Why | Bridge | Gap |
| --- | --- | --- | --- |
| A handler of the Forge `PermissionAPI` | Mods that ask Forge see the groups of `golem-permissions` ([LuckPerms 5.5.71](../../plan.md#luckperms-5571) of the plan). The Essentials port itself does not need it | The handler `golem:permissions`, which asks the native `permission(uuid, node)` for a node of the type Boolean. `forge-server.toml` of the world must name it as `permissionHandler`, and the mod logs one warning when Forge takes another handler, as on a world taken over from the live server | None for the bridge. How the port answers the nodes that LuckPerms decided through Arclight is an open decision of the owner, see [Permissions](../permissions.md#arclight-forwards-every-permission-of-a-mod-to-luckperms) |
| The level of a command source | Command blocks and functions must not pass for the console, and no source may borrow the rights of a player through `execute as` | `level` of `command`, and `Sender::Other` with its name and level for a source that is neither a player who runs the command themself nor the console, a source that runs it as a player included, see [The host class](../../bridge.md#the-host-class) | None |
| Defaults that the port declares | `golem-permissions` knows no plugin default (its rule 8), so the port must give everybody `essentials.back.onteleport`, `essentials.back.into.<dimension>`, `essentials.teleport.cooldown.bypass.tpa` and `.back` (teleport part, section 2.2, `plugin.yml:1561-1567`), unless it drops the split of the cooldown bypass per type, give the group `admin` what `default: op` gives operators today, and nobody the `default: false` nodes | Not a bridge need | Belongs to `golem-permissions` and the seed |
| Dimension ids in nodes | `minecraft:the_nether` holds a `:`, which a node segment does not allow | Not a bridge need | A mapping, teleport part section 8.5 |

## What the comparison shows

[Bridge](../../bridge.md) covers the base of the teleport side: players with their operator level
and idle time, position, respawn point, shared spawn, height bounds, a safe location, the `teleport`
action, the death location, the tab list name, the level of a source and the `PermissionAPI`
handler. These needs are missing, in the order in which the live group `default` meets them:

1. **The result of a teleport.** Without a request id and a `teleported` event the plugin cannot
   write the last location after a move that worked, send the success message or charge only on
   success, which every teleport command needs (teleport part Q7 and Q31). A safe search that runs
   in the action, after the chunk is loaded, also finds a place where `safeLocation` answers
   `unloaded` for a chunk that is not loaded (teleport part, Q29).
2. **Styled messages.** Every text of EssentialsX is colored, and the plain `message` would force
   `§` codes into literals.
3. **The command tree.** Brigadier merges a plugin literal into the vanilla node of the same name
   and keeps the vanilla requirement and children. So without removing the vanilla nodes, `/msg`,
   `/tell`, `/w`, `/me`, `/list` and `/help`, which the group `default` uses, keep running vanilla.
   Without a requirement, every player sees every admin command in completion.
4. **Small queries**: whether a player may fly (the safety exemption), sea level and day time
   (`depth`, the MOTD), the maximum players (`list`), the whole inventory (`worth inventory` and
   `blocks`), the logical height, the angle and the validation of the respawn point, and the list
   of dimensions.
5. **The quit position**, or the rule that `location(uuid)` answers during `playerLeft`.
6. **Chat to a subset** for ignore in chat, since `chat` can only cancel while EssentialsX leaves
   out of the recipients every player who ignores the sender, and **kill with a damage type** for
   `suicide`.
7. **Events for AFK**: every player command, interact, attack, bow and fishing, needed once the
   automatic AFK of the plan is on, unless the idle time of `players()` stands in for them.
8. **Decisions before any bridge work**: the compass target, the PvP protections with a
   cancellable attack event, the login texts, and the flight and speed resets, each of which the
   teleport part or the social part recommends to drop or puts to the owner.
9. **Admin only**: kick and ban with Essentials' notices, the ban entry, the address, suggestions.

Sleeping ignored needs nothing: Arclight 1.0.6 never reads the flag that `setSleepingIgnored`
writes, so operators and AFK players count for sleeping today, and the port keeps that (social
part, Q37). The signs need nothing: every Essentials sign is off in the live config (social part,
The sign node).

## Sources

The page is checked against these sources, the pages of Golem on 2026-11-07 and the others on
2026-11-06.

| Source | What the page takes from it |
| --- | --- |
| The checkout of EssentialsX 2.22.0 at commit `100f575293afff1c2bc8e1468ce1cffc153319fc`, the tag [2.22.0](https://github.com/EssentialsX/Essentials/tree/2.22.0) | Every citation of a file of EssentialsX. A bare file name is the file of that name under `Essentials/src/main/java/com/earth2me/essentials/` or one of its folders, `SpawnStorage.java` is in `EssentialsSpawn/src/main/java/com/earth2me/essentials/spawn/`, `ModernDataWorldInfoProvider.java` in `providers/BaseProviders/src/main/java/net/ess3/provider/providers/`, and `config.yml` and `plugin.yml` in `Essentials/src/main/resources/`. `PluginCommands.java` and `GameQueries.java` are files of Golem, see below |
| [Bridge](../../bridge.md), [Plugins](../index.md) and [Core plugin](../core.md) of Golem at commit `f630fb1` | The columns Bridge and the table of what the contract offers |
| `bridge/mod/src/main/java/com/skytekx/golem/bridge/PluginCommands.java` and `GameQueries.java` of the same commit | The registration of the plugin commands with `dispatcher.register` (lines 47 to 61) and the answer of `respawn` (lines 96 to 106) |
| Brigadier 1.1.8 (`brigadier-1.1.8.jar`) | `CommandNode.addChild`, which sets only the command of an existing child of the same name and adds the new children to it, so the existing child keeps its requirement |
| The Arclight jar of the live server, `arclight-forge-1.20.1-1.0.6-6de9fec-safeiter2.jar` | The recipient set of chat and the sleep flag, as the rulings Q31 and Q37 of the social part read them |
| Forge 1.20.1-47.4.18, the Forge of the live server (`forge-1.20.1-47.4.18-sources.jar` and the 662 patches of `forge-1.20.1-47.4.18-userdev.jar`) | The events `CommandEvent` and `LivingAttackEvent`, and the sleep events, none of which changes the count of `SleepStatus` |
| The Spigot API 1.20.1 sources, `spigot-api-1.20.1-R0.1-20230921.163938-66-sources.jar` | Every type and method of the columns Bukkit API, where a method may sit on a supertype, such as `getHealth` on `Damageable` or `getMinHeight` on `WorldInfo` (`org/bukkit/generator/WorldInfo.java:50`) |
| [teleport.md](teleport.md) and [social.md](social.md) | The commands of each need, the quirk IDs and what each part recommends |
| [Plan](../../plan.md) and [Permissions](../permissions.md) of Golem | The ruling on the whole Bukkit API and the open decision on the nodes of the mods that the tables name |

These citations are read against the checkout and hold.

| File | Lines |
| --- | --- |
| `Essentials.java` | 432-433, 985-1064, 1209-1232 |
| `PlayerList.java` | 53-90, 66-75 |
| `AsyncTeleport.java` | 155-225, 179-181, 179-220, 183-187, 190-207, 395-411, 400-407 |
| `AsyncTimedTeleport.java` | 41-56, 94-98 |
| `User.java` | 68-98, 410, 690, 703-713, 868-916, 1127-1146 |
| `UserData.java` | 292-303, 553-559 |
| `I18n.java` | 171-209 |
| `Worth.java` | 78-132 |
| `EssentialsPlayerListener.java` | 180-189, 191-232, 214-228, 234-286, 289-342, 400-616, 448-449, 531-534, 618-633, 624-629, 636-662, 636-676, 777-903, 905-933, 905-962, 966-1013, 1193-1201, 1270-1308 |
| `EssentialsEntityListener.java` | 63, 71, 77-83, 159-177, 256-264 |
| `EssentialsTimer.java` | 30-68 |
| `Commandban.java` | 44-57, 53 |
| `Commandcompass.java` | The yaw at 13 |
| `Commanddepth.java` | 13 |
| `Commandgamemode.java` | 53-55, 62, 76 |
| `Commandgetpos.java` | 35-45 |
| `Commandhelpop.java` | 43-62 |
| `Commandhome.java` | 51, 79-84, 80, 86, 116, 154-185 |
| `Commandkick.java` | 52 |
| `Commandseen.java` | 139-141, 143-201 |
| `Commandsethome.java` | 54, 55-57 |
| `Commandsuicide.java` | 17-26 |
| `Commandtpaccept.java` | 67-76 |
| `Commandwarp.java` | 111-132 |
| `Commandworth.java` | 24-56 |
| `LocationUtil.java` | 120-150, 152-155, 152-164, 187-193, 203-281, 222-227, 226-227, 238-279, 283-301 |
| `PermissionsDefaults.java` | 17-33 |
| `LazyLocation.java` | 12-29, 59-88 |
| `KeywordReplacer.java` | 327-331 |
| `SpawnStorage.java` | 63-71 |
| `ModernDataWorldInfoProvider.java` | 11-22 |
| `config.yml` | 550 |
| `plugin.yml` | 1561-1567 |
