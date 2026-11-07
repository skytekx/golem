---
# SPDX-License-Identifier: GPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
# SPDX-FileCopyrightText: 2011 Essentials Team
# SPDX-FileCopyrightText: EssentialsX contributors
type: Spec
title: EssentialsX social, info, status, economy and admin commands
description: The part of the port of EssentialsX 2.22.0 to golem-essentials that covers messages, mail, ignore, the info commands, AFK, suicide, the economy and the admin commands eco, gamemode, kick and ban, as the Java code does them with the default config, with their messages, config keys, bridge needs and the quirks Golem fixes.
tags: [essentials, port, spec, messages, mail, afk, economy, gamemode]
timestamp: 2026-11-06
---

# EssentialsX social, info, status, economy and admin commands

This page is one part of the spec for `golem-essentials`, the port of EssentialsX 2.22.0 (commit
`100f575293afff1c2bc8e1468ce1cffc153319fc`). It covers every command of the live group `default`
that the teleport page, which holds the teleport, home, warp and spawn commands, does not cover,
the admin commands `eco`, `gamemode` (with `gm` and its forms), `kick` and `ban`, the economy core
and the chat side. No page covers signs, since no Essentials sign works with the live config, see
[The sign node](#the-sign-node). The port keeps the license of EssentialsX, GPL 3 or later, so this
page quotes the English texts of `messages.properties` and the defaults of `config.yml` as they
are.

## How to read this page

Paths are relative to the checkout of EssentialsX 2.22.0, the tag
[2.22.0](https://github.com/EssentialsX/Essentials/tree/2.22.0), unless the table names another
place. The prefixes keep them short. A page of Golem is a link to the section that holds the fact.

| Prefix | Path |
| --- | --- |
| `E/` | `Essentials/src/main/java/com/earth2me/essentials/` |
| `R/` | `Essentials/src/main/resources/` |
| `plugins.md` | [`okf/modpack/plugins.md`](https://github.com/skytekx/skytekx3-1201-server/blob/3fa5ae681d12233fe81003fbfe5a2d3c529747fc/okf/modpack/plugins.md) of the server repository `skytekx/skytekx3-1201-server` at commit `3fa5ae68` |
| `server.properties.example` | [`server.properties.example`](https://github.com/skytekx/skytekx3-1201-server/blob/3fa5ae681d12233fe81003fbfe5a2d3c529747fc/server.properties.example) of the same commit |
| `bridge/mod/...` | The tree of Golem at commit `f630fb1` |

`E/X.java:10-20` means lines 10 to 20 of that file. `msg:N` is line N of `R/messages.properties`,
`cfg:N` line N of `R/config.yml` and `yml:N` line N of `R/plugin.yml`. Facts about CraftBukkit,
Brigadier or vanilla that the checkout does not hold say so where they stand.

Message texts are quoted raw from `R/messages.properties`. In them `\:` and `\!` are escapes of
the properties format for `:` and `!`, `''` is the escape of `MessageFormat` for `'`, `{0}` is the
first argument, and the tags are MiniMessage. `<primary>` is `#ffaa00` and `<secondary>` is
`#ff5555` (`cfg:479-481`, `E/Settings.java:2222-2237`).

The live server runs the `config.yml` that 2.22.0 wrote on 2026-06-28. Its line numbers match the
default file: `use-bukkit-permissions` at 354 and `player-commands` at 269 in both
(`plugins.md:55-57`, `cfg:269`, `cfg:354`). The only live change that page records is the two
listener priorities of the spawn module, which set `none` where the default is `high`
(`plugins.md:89`, `cfg:1273`, `cfg:1279`). No key of this part is among them, so the defaults below
are the live settings. Where the code default differs from `config.yml`, the value of
`config.yml` is the live one and this page names both.

## The live permission picture

### The group default

The group `default` of LuckPerms holds 39 nodes (`plugins.md:64-69`). Of them, 22 belong to this
part, 16 to the teleport page, and the last one, `essentials.signs.use.*`, to no command, see
[The sign node](#the-sign-node). Operators hold every node whose `plugin.yml` entry has no
`default` or `default: op`, since `essentials.*` is `default: op` (`yml:624-628`) and Bukkit gives
an undeclared or undefaulted node to operators. LuckPerms keeps that with its default settings
(Bukkit and LuckPerms, outside the checkout).

| Command | Node it checks | In the group `default` |
| --- | --- | --- |
| `msg`, `r` | `essentials.msg` (`r` checks `msg`, `E/User.java:133-135`) | Yes |
| `mail` | `essentials.mail`, and `essentials.mail.send` for `send` | Both |
| `ignore` | `essentials.ignore` | Yes |
| `me` | `essentials.me` | Yes |
| `helpop` | `essentials.helpop` | Yes |
| `list` | `essentials.list` | Yes |
| `seen` | `essentials.seen` | Yes |
| `realname` | `essentials.realname` | Yes |
| `motd` | `essentials.motd` | Yes |
| `rules` | `essentials.rules` | Yes |
| `help` | `essentials.help` | Yes |
| `ping` | `essentials.ping` | Yes |
| `getpos` | `essentials.getpos` | Yes |
| `compass` | `essentials.compass` | Yes |
| `depth` | `essentials.depth` | Yes |
| `afk` | `essentials.afk` | Yes |
| `suicide` | `essentials.suicide` | Yes |
| `balance` | `essentials.balance` | Yes |
| `balancetop` | `essentials.balancetop` | Yes |
| `pay` | `essentials.pay` | Yes |
| `worth` | `essentials.worth` | Yes |
| `eco` | `essentials.eco` | No, operators |
| `gamemode` and its aliases | `essentials.gamemode` | No, operators |
| `kick` | `essentials.kick` | No, operators |
| `ban` | `essentials.ban` | No, operators |
| `payconfirmtoggle` | `essentials.payconfirmtoggle` | No, operators |

Every sub node this page names, such as `essentials.msg.color` or `essentials.balance.others`, is
not in the group `default`, so only operators hold it.

### Nodes nobody holds

These nodes are `default: false` in `plugin.yml` and not in the group, so neither players nor
operators hold them on the live server.

| Node | Line | Effect of nobody holding it |
| --- | --- | --- |
| `essentials.afk.auto` | `yml:635-637` | Automatic AFK never fires, see [AFK](#afk) |
| `essentials.chat.ignoreexempt` | `yml:706-708` | Everybody can be ignored |
| `essentials.kick.exempt` | `yml:979-981` | Nobody is exempt from `kick` and the AFK kick |
| `essentials.ban.exempt` | `yml:671-673` | Nobody is exempt from `ban` |
| `essentials.balancetop.exclude` | `yml:664-666` | Every account shows in `balancetop` |
| `essentials.exempt` | `yml:1568-1575` | The parent of the exempt nodes above |

### The handler without Vault

Vault is not installed (`plugins.md:16-22` lists EssentialsX, EssentialsXSpawn and LuckPerms).
Essentials tries `LuckPermsHandler`, `ModernVaultHandler`, `GenericVaultHandler` and
`SuperpermsHandler` in that order (`E/perm/PermissionsHandler.java:175-199`). The first three
need the plugin Vault (`E/perm/impl/AbstractVaultHandler.java:112-119`,
`E/perm/impl/LuckPermsHandler.java:117`), so `SuperpermsHandler` wins, because it finds the plugin
LuckPerms by name (`E/perm/impl/SuperpermsHandler.java:164-181`), and Essentials logs at boot that
it found LuckPerms without Vault (`E/perm/PermissionsHandler.java:223-230`). That has three effects
on this part.

- A group lookup answers null, which becomes `default` for every player
  (`E/perm/impl/SuperpermsHandler.java:36-38`, `E/perm/PermissionsHandler.java:39-48`). `/list`
  therefore shows one group `default`, and the per group files `motd_<group>.txt` can only be
  `motd_default.txt`.
- Prefixes and suffixes are empty (`E/perm/impl/SuperpermsHandler.java:141-148`,
  `E/perm/PermissionsHandler.java:133-153`).
- Wildcards such as `essentials.signs.use.*` are emulated by walking up the node
  (`E/perm/impl/SuperpermsHandler.java:95-118`).

Golem has one permission system, `golem-permissions`, so the port asks it for every node this
page names and keeps the default values of the table above.

### The sign node

The 39th node of the group, `essentials.signs.use.*` (`plugins.md:64-69`), belongs to no command.
With the default config no Essentials sign works at all. Every entry of `enabledSigns` is a comment
(`cfg:380-405`), so `signsEnabled` stays false (`E/Settings.java:998-1021`), and every sign
listener returns at once (`E/signs/SignBlockListener.java:35`, `79`, `120`, `139`, `163`,
`E/signs/SignPlayerListener.java:29`, `E/signs/SignEntityListener.java:22`, `43`), although
Essentials registers them (`E/Essentials.java:502-509`). The node has no effect on the live
server, and an Essentials sign in the world is dead text today.

The sign audit of the plan (the row Signs of [EssentialsX 2.22.0](../../plan.md#essentialsx-2220))
therefore only has to confirm that `enabledSigns` is still empty on the Mac mini. If it is, the port
needs no sign, and `sign-use-per-second: 4` (`cfg:410`), `allow-old-id-signs: false` (`cfg:415`) and
`unprotected-sign-names` (`cfg:423`) need no port either. A port of signs would check
`essentials.signs.<type>.use` or `essentials.signs.use.<type>`, and `.create` and `.break` the same
way (`E/signs/EssentialsSign.java:104`, `204`, `225`).

## Shared machinery

These steps run for every command of this part. The command sections only name what differs.

### Dispatch of a command

`Essentials.onCommandEssentials` (`E/Essentials.java:728-866`) runs every Essentials command.

1. If another Bukkit plugin registers the same label and the label is not in
   `overridden-commands`, Essentials hands the command to that plugin
   (`E/Essentials.java:730-751`, `E/AlternativeCommandsHandler.java:32-67`). Only Bukkit plugin
   commands count, and LuckPerms shares no name with this part, so this never happens live. A
   label that starts with `e` is never handed over unless it equals the command's name, so `/emsg`
   or `/eban` always run Essentials (`E/Essentials.java:730`).
2. The console is logged as `<name> issued server command: /<label> <args>`, and a command block
   as `CommandBlock at <x>,<y>,<z> issued server command: /<label> <args>`
   (`E/Essentials.java:764-772`, `log-console-commands: true` at `cfg:781`,
   `log-command-block-commands: true` at `cfg:777`).
3. A player who holds `essentials.mail` gets the mail notice before any command but `mail`, as
   long as `mail` is not in `disabled-commands`, see [Mail notice](#mail-notice)
   (`E/Essentials.java:777-779`).
4. A label in `disabled-commands` (empty, `cfg:182-184`) first runs the known command of that
   label when it belongs to the server or to a plugin other than Essentials, and only otherwise
   answers `commandDisabled` (`E/Essentials.java:788-795`).
5. The node `essentials.<name>` is checked, with `essentials.msg` for `r`
   (`E/Essentials.java:807-811`, `E/User.java:133-135`). Without it the player reads
   `noAccessCommand` and the log gets `deniedAccessCommand`.
6. A jailed player is stopped unless they hold `essentials.jail.allow.<name>`. They read
   `playerJailedFor` with their name and the time left when the jail has a timeout, else
   `jailMessage` (`E/Essentials.java:813-820`).
7. For a player the command cost is checked before the command and charged after it
   (`E/commands/EssentialsCommand.java:167-173`). The cost of a command is the entry of its name in
   `command-costs`, which is empty (`cfg:855-859`, `E/Settings.java:449-456`,
   `E/Trade.java:204-223`, `E/Trade.java:368-384`), so no command of this part costs anything.
8. `NotEnoughArgumentsException` prints the usage. With `verbose-command-usages: true` (`cfg:188`)
   it prints `commandHelpLine1`, `commandHelpLine2` with the description, `commandHelpLine3` and one
   `commandHelpLineUsage` per usage key `<name>CommandUsage<n>`
   (`E/Essentials.java:832-854`). The usage keys are read at start up, optional `[...]` parts
   coloured `commandArgumentOptional`, required `<...>` parts `commandArgumentRequired`, and the
   `|` between choices `commandArgumentOr` (`E/commands/EssentialsCommand.java:44`,
   `E/commands/EssentialsCommand.java:51-75`). A command without usage keys prints the
   `description` and `usage` of `plugin.yml` instead (`E/Essentials.java:844-847`). A message of
   the exception is printed after the usage (`E/Essentials.java:848-850`).
9. Any other exception prints `errorWithMessage` with its text (`E/Essentials.java:891-901`).

Messages of the dispatch:

- `noAccessCommand` (`msg:857`): `<dark_red>You do not have access to that command.`
- `deniedAccessCommand` (`msg:225`, log only):
  `<secondary>{0} <dark_red>was denied access to command.`
- `commandDisabled` (`msg:150`): `<secondary>The command<primary> {0}<secondary> is disabled.`
- `playerJailedFor` (`msg:943`):
  `<primary>Player<secondary> {0} <primary>jailed for<secondary> {1}<primary>.`
- `jailMessage` (`msg:631`): `<dark_red>You do the crime, you do the time.`
- `errorWithMessage` (`msg:347`): `<secondary>Error\:<dark_red> {0}`
- `onlyPlayers` (`msg:909`): `<dark_red>Only in-game players can use <secondary>{0}<dark_red>.`
- `playerNotFound` (`msg:950`): `<dark_red>Player not found.`
- `noPerm` (`msg:885`): `<dark_red>You do not have the <secondary>{0}<dark_red> permission.`
- `commandHelpLine1` (`msg:153`): `<primary>Command Help\: <white>/{0}`
- `commandHelpLine2` (`msg:154`): `<primary>Description\: <white>{0}`
- `commandHelpLine3` (`msg:155`): `<primary>Usage(s);`
- `commandHelpLineUsage` (`msg:157`): `{0} <primary>- {1}`
- `commandArgumentRequired` (`msg:148`): `<yellow>`, `commandArgumentOptional` (`msg:146`):
  `<gray>`, `commandArgumentOr` (`msg:147`): `<secondary>`

A command that only `run(Server, User, ...)` implements answers `onlyPlayers` to the console,
because the base class throws it (`E/commands/EssentialsCommand.java:184-186`).

### What every player command does first

The listener of `PlayerCommandPreprocessEvent` runs at `MONITOR` for every command a player types,
Essentials or not (`E/EssentialsPlayerListener.java:777-903`).

1. Commands in `socialspy-commands` (`cfg:194-216`) are shown to players with social spy on as
   `socialSpyCmdFormat`, but `msg` and `r`, which `SimpleMessageRecipient` handles
   (`E/EssentialsPlayerListener.java:805-823`). Social spy needs `essentials.socialspy`, which only
   operators hold.
2. A muted player is stopped for commands in `mute-commands`, which are `f` and `kittycannon`
   (`cfg:257-260`, `E/EssentialsPlayerListener.java:826-844`).
3. The activity of the player is updated, which ends AFK, except for `afk` itself, which does not
   update, and `vanish`, which does not broadcast (`E/EssentialsPlayerListener.java:846-863`).
4. `command-cooldowns` is empty (`cfg:716-719`), so no cooldown applies and `commandCooldown` is
   never sent (`E/EssentialsPlayerListener.java:865-902`,
   `E/EssentialsPlayerListener.java:884-885`).

- `socialSpyCmdFormat` (`msg:1241`): `<gray>{0}\: {1}`
- `commandCooldown` (`msg:149`): `<secondary>You cannot type that command for {0}.`

### How a player name is matched

Most commands go through `Essentials.matchUser` (`E/Essentials.java:985-1064`), with a source user,
a flag `getHidden` and a flag `getOffline`.

1. `@p` and `@s` from a player answer the player themselves (`E/Essentials.java:989-991`).
2. A UUID answers the online player with it (`E/Essentials.java:993-994`).
3. Else an exact online name is looked up, case insensitive. When `getOffline` is false, a prefix
   match follows: CraftBukkit's `Server.getPlayer(String)` takes the online player whose name
   starts with the term with the smallest difference in length (`E/Essentials.java:996-1007`,
   with the prefix rule of CraftBukkit, outside the checkout).
4. If nothing is found, the name is looked up in the user map, which holds every player that
   ever joined (`E/Essentials.java:1012`, `E/Essentials.java:980-982`). With
   `safe-usermap-names: true` (`cfg:773`) the key is the name in lower case with every character
   outside `a-z0-9` replaced by `_` (`E/userstorage/ModernUUIDCache.java:70-88`,
   `E/utils/StringUtil.java:37-41`).
5. A found user who is offline fails when `getOffline` is false. A found user who is hidden from
   the source fails, unless `getHidden` is set or the search is offline and the term equals the
   name exactly (`E/Essentials.java:1015-1027`).
6. Else CraftBukkit's `Server.matchPlayer` lists the online players whose name contains the term,
   or only the one whose name equals it (`E/Essentials.java:1029`, with the rule of CraftBukkit,
   outside the checkout). If the list is empty, the first online player whose display name, without
   formatting codes and in lower case, contains the term wins (`E/Essentials.java:1031-1040`).
   Else the first one whose name equals the term, then the first whose display name starts with
   the term (case sensitive), then simply the first of the list (`E/Essentials.java:1041-1062`).
7. Hidden means vanished. `canInteractWith` reads no node: the console sees everyone, a player
   sees themselves and everyone their client can see (`Player.canSee`), a missing source sees
   everyone who is not vanished, and an offline user counts as hidden from a player
   (`E/Essentials.java:1066-1090`, `E/User.java:726-731`). The node `essentials.vanish.interact`
   enters elsewhere: as the `getHidden` flag that the online lookup of a player passes to
   `matchUser` (`E/commands/EssentialsCommand.java:126-141`, `E/User.java:1014-1016`), and as the
   `skipHidden` flag of the loop commands and `realname`
   (`E/commands/EssentialsLoopCommand.java:51`, `91`, `E/commands/Commandrealname.java:23`). The
   table below holds for the commands of this part. A port that turns `canInteractWith` into a
   node check differs for console lookups and for players hidden by other means.

Nicknames only enter through the display name in step 6 and in `realname`. `nick` is not in the
group `default`, and the build machine holds one player file (`plugins.md:37`), so no player is
known to have a nickname.

The commands that loop over players (`msg`, `pay`, `eco`, `gamemode`) use
`EssentialsLoopCommand` (`E/commands/EssentialsLoopCommand.java:22-136`): `@s` and `@p` are the
sender, `*` and `**` are all online players (and `**` all known players for the offline loop) when
the command allows wildcards, a UUID is taken as is, and anything else goes to `matchUser`.

| Command | Lookup | Online only | Finds vanished |
| --- | --- | --- | --- |
| `msg`, `getpos <player>`, `afk <player>`, `gamemode ... <player>` | `getPlayer(server, sender, term)` | Yes | With `essentials.vanish.interact` |
| `mail send`, `pay`, `eco`, `balance <player>` (console), `ban` | `getPlayer(server, term, true, true)` | No | Yes |
| `ignore`, `balance <player>` (player) | `getPlayer(server, args, 0, false, true)` | No | Only by exact name |
| `kick`, `getpos` and `afk` from the console | `getPlayer(server, args, 0, true, false)` | Yes | Yes |
| `seen` | Exact user map name first, then the lookup of `msg` (`E/commands/Commandseen.java:40-46`, `73`) | No | Shown as offline |
| `realname` | Substring of the display name of online players | Yes | With `essentials.vanish.interact` |

### How texts reach the player

- Every text is a key of `messages.properties`, formatted with `MessageFormat` and parsed as
  MiniMessage (`E/User.java:1127-1146`, `E/I18n.java:171-194`). Arguments are escaped for
  MiniMessage first, and legacy `§` codes in them become MiniMessage tags
  (`E/I18n.java:186-191`, `E/adventure/SpigotAdventureFacet.java:112-119`,
  `E/adventure/SpigotAdventureFacet.java:136-139`). Only arguments wrapped in
  `AdventureUtil.parsed`, such as currency strings, are parsed as markup
  (`E/adventure/AdventureUtil.java:42-61`). A key whose text is empty is not sent
  (`E/User.java:1129-1131`).
- `MessageFormat` formats a number argument itself with the default locale of the JVM, with
  grouping (`E/I18n.java:196-209`, `java.text.MessageFormat`, outside the checkout). An X of 1234
  reads `1,234` in English.
- `locale` is not set (`cfg:467`) and `per-player-locale: false` (`cfg:474`), so the language is
  the default locale of the JVM (`E/I18n.java:50-53`, `E/I18n.java:211-225`). Which locale the Mac
  mini's JVM has is not recorded. This page gives the English texts.
- The display name is the name, prefixed with the colour of `ops-name-color: '4'` (`cfg:30`), dark
  red, for operators and followed by `§r` (`E/User.java:510-566`, `E/Settings.java:551-575`). It is
  set on join, respawn, world change and before most commands that show it
  (`E/User.java:568-584`, `change-displayname: true` at `cfg:63`). A vanished player shows their
  plain name, since `hide-displayname-in-vanish: true` (`cfg:60`, `E/User.java:586-590`). The code
  default of that key is false (`E/Settings.java:1600-1602`). `nickname-prefix: '~'` (`cfg:34`)
  only matters for players with a nickname. Without EssentialsChat and with `add-prefix-suffix`
  unset, no prefix or suffix is added (`cfg:69-73`, `E/Settings.java:1247-1263`).
  `change-playerlist` is unset and defaults to false (`cfg:75-77`, `E/Settings.java:1228-1235`), so
  the tab list keeps the vanilla name.

### Formatting codes in player text

`FormatUtil.formatMessage(user, permBase, text)` (`E/utils/FormatUtil.java:74-83`) is used for
`msg`, `r`, `mail` and `me`.

1. The supported codes come from `<base>.color` (the 16 colours), `<base>.format` (bold,
   strikethrough, underline, italic, reset), `<base>.magic` and the explicitly set nodes
   `<base>.<colour name>` (`E/utils/FormatUtil.java:17-19`, `E/utils/FormatUtil.java:246-276`).
2. `§` codes outside the supported set are removed, and `&` codes inside it become `§` codes
   (`E/utils/FormatUtil.java:229-244`, `E/utils/FormatUtil.java:93-132`). `&` codes outside it stay
   as literal text.
3. Without `<base>.url`, a dot becomes a space where at least two characters out of letters,
   digits, `_`, `-` and `.` (the class `[\w-_\.]`, after an optional `http://` or `https://`) come
   before it and at least two ASCII letters follow it, and the replacement repeats until nothing
   matches: `play.example.net` becomes `play example net`, `file.txt` becomes `file txt` and
   `x.y.zz` becomes `x.y zz`, while `a.b`, `a.com`, `1.5`, `1.20.1` and `v2.x` stay
   (`E/utils/FormatUtil.java:79-81`, `E/utils/FormatUtil.java:297-306`, the pattern at
   `providers/BaseProviders/src/main/java/net/ess3/provider/AbstractChatEvent.java:10`). The
   examples come from a run of the same pattern and loop on Java 21 on 2026-11-06.

No player of the group `default` holds a colour, format or url node, so their text is plain. The
console gets `FormatUtil.replaceFormat`, which turns every `&` code into a `§` code
(`E/utils/FormatUtil.java:86-91`).

### Broadcasts and the ignore list

- `broadcastTl(sender, exclude, key, args)` skips everything when the sender is vanished, skips
  every player who ignores the sender, and skips players the predicate excludes
  (`E/Essentials.java:1209-1233`). Without a sender nobody is skipped for ignoring
  (`E/Essentials.java:1189-1192`).
- A player ignores another if their list holds the other's UUID and the other lacks
  `essentials.chat.ignoreexempt` (`E/UserData.java:443-445`, `E/User.java:1095-1098`).

### Mail notice

`notifyOfMail` sends `youHaveNewMail` with the number of unread, not expired mails, at most once
per `notify-player-of-mail-cooldown: 60` seconds (`cfg:166`, `E/User.java:1274-1283`,
`E/UserData.java:369-381`). The code default of the key is 0 (`E/Settings.java:1960-1962`).

- `youHaveNewMail` (`msg:1634`):
  `<primary>You have<secondary> {0} <primary>messages\! Type <secondary>/mail read<primary> to view your mail.`

### Durations

`DateUtil.formatDateDiff` writes the distance to a time as at most the three largest of years,
months, days, hours, minutes and seconds, for example `2 days 3 hours 15 minutes`, or `now`
(`E/utils/DateUtil.java:137-174`, the units at `msg:195-196`, `msg:519-520`, `msg:772-773`,
`msg:782-783`, `msg:1119-1120` and `msg:1631-1632`, and `now` at `msg:900`).
`DateUtil.parseDateDiff` reads `1y2mo3w4d5h6m7s` style input and caps it at ten years ahead
(`E/utils/DateUtil.java:25-113`).

### Mute

Mute is set by the admin command `mute`, which is not part of this page. The commands `msg`, `r`,
`mail send`, `mail sendtemp`, `me` and `afk <message>` refuse a muted player, and chat is cancelled
for them (`E/EssentialsPlayerListener.java:191-213`). `helpop` does not check mute, see Q9.

- `voiceSilenced` (`msg:1543`): `<primary>Your voice has been silenced\!`
- `voiceSilencedTime` (`msg:1544`): `<primary>Your voice has been silenced for {0}\!`
- `voiceSilencedReason` (`msg:1545`):
  `<primary>Your voice has been silenced\! Reason\: <secondary>{0}`
- `voiceSilencedReasonTime` (`msg:1546`):
  `<primary>Your voice has been silenced for {0}\! Reason\: <secondary>{1}`
- `mutedUserSpeaks` (`msg:818`, log only): `{0} tried to speak, but is muted\: {1}`

## Messages

### msg

**Names.** Command `msg` with the aliases `w`, `m`, `t`, `pm`, `emsg`, `epm`, `tell`, `etell`,
`whisper` and `ewhisper`, usage `/<command> <to> <message>` (`yml:310-313`, `msg:792-795`).

| Node | Checked at | Live |
| --- | --- | --- |
| `essentials.msg` | `E/Essentials.java:807` | Group `default` |
| `essentials.msg.multiple`, for the targets `*` and `**` | `E/commands/Commandmsg.java:27` | Operators |
| `essentials.msg.color`, `.format`, `.magic`, `.rgb`, `.url`, `.<colour name>` | `E/utils/FormatUtil.java:79`, `E/utils/FormatUtil.java:236-276` | Operators |
| `essentials.msgtoggle.bypass`, to reach a player with messages off | `E/messaging/SimpleMessageRecipient.java:163` | Operators |
| `essentials.chat.spy.exempt`, to stay out of social spy | `E/messaging/SimpleMessageRecipient.java:122-123` | Operators |
| `essentials.vanish.interact`, to message vanished players | `E/User.java:1014-1016` | Operators |

**Behaviour** (`E/commands/Commandmsg.java:21-56`,
`E/messaging/SimpleMessageRecipient.java:86-189`).

1. Fewer than two arguments print the usage. The message is every argument from the second on,
   joined with single spaces (`E/commands/EssentialsCommand.java:82-91`).
2. A muted player is refused with the mute message.
3. The message of a player goes through `formatMessage(user, "essentials.msg", ...)`, the one of
   the console through `replaceFormat`.
4. A target `Console` or the translated console name (`consoleName`, `msg:175`) sends the message
   to the console (`E/commands/Commandmsg.java:43-47`, `E/Console.java:15-19`).
5. Else the target is matched online, see the table above. `@s` messages the sender themselves, and
   `*` or `**` with `essentials.msg.multiple` message every online player the sender can see.
6. `PrivateMessagePreSendEvent` fires and can cancel or change the message
   (`E/messaging/SimpleMessageRecipient.java:87-93`).
7. The recipient decides (`E/messaging/SimpleMessageRecipient.java:154-189`). Offline is
   `UNREACHABLE`. Messages off (`msgtoggle`) without the bypass node of the sender is
   `MESSAGES_IGNORED`. A recipient who ignores the sender is `SENDER_IGNORED`. Both refusals apply
   only to a player as sender, and the console always gets through `msgtoggle` and the ignore list
   (`E/messaging/SimpleMessageRecipient.java:163`, `169`). Otherwise the recipient reads
   `msgFormat` with the sender's display name, `me` and the message.
8. The sender then reads (`E/messaging/SimpleMessageRecipient.java:95-141`)
   `recentlyForeverAlone` for `UNREACHABLE`, `msgIgnore` for `MESSAGES_IGNORED`, and nothing at all
   for `SENDER_IGNORED`. For an AFK recipient they read `userAFKWithMessage` or `userAFK` and then
   the echo, and otherwise the echo `msgFormat` with `me`, the recipient's display name and the
   message.
9. With `socialspy-messages: true` (`cfg:224`) every online player with social spy on, other than
   the two, reads `socialSpyPrefix` (or `socialSpyMutedPrefix` for a muted sender, since
   `socialspy-listen-muted-players: true` at `cfg:220`) and `socialSpyMsgFormat`, with display
   names (`socialspy-uses-displaynames: true`, `cfg:228`), unless either side holds
   `essentials.chat.spy.exempt` (`E/messaging/SimpleMessageRecipient.java:116-139`). Both sides
   must be players, so a message to or from the console is never spied
   (`E/messaging/SimpleMessageRecipient.java:118-123`).
10. On success the sender's reply target becomes the recipient
    (`E/messaging/SimpleMessageRecipient.java:142-145`). The recipient's reply target becomes the
    sender if `last-message-reply-recipient` is false for the recipient, or if it is true
    (`cfg:681`, the live value, while the code default is false, `E/Settings.java:1698-1701`) and
    their reply target is unset, offline, or they received no message in the last
    `last-message-reply-recipient-timeout: 180` seconds (`cfg:687`)
    (`E/messaging/SimpleMessageRecipient.java:176-187`). A player can flip the flag with
    `rtoggle`, which only operators hold, and it is stored per player
    (`E/UserData.java:734-741`).
11. `PrivateMessageSentEvent` fires (`E/messaging/SimpleMessageRecipient.java:147-148`).

Nothing is stored but the reply target, a weak reference, and the time of the last message
received, `lastMessageMs`, which drives the 180-second rule of step 10. Both live in memory only
(`E/messaging/SimpleMessageRecipient.java:40-41`, `181`, `187`,
`E/messaging/SimpleMessageRecipient.java:201-214`). A port that keeps only the reply target cannot
reproduce that rule. The state of `msgtoggle` is not stored either. It lives in memory
(`E/User.java:83`, `1019-1026`) and is lost on a restart. The console can message players, and
players can message the console.

**Messages.**

- `msgFormat` (`msg:800`):
  `<primary>[<secondary>{0}<primary> -> <secondary>{1}<primary>] <reset>{2}`
- `meSender` (`msg:768`): `me`, and `meRecipient` (`msg:769`): `me`
- `recentlyForeverAlone` (`msg:1049`): `<dark_red>{0} recently went offline.`
- `msgIgnore` (`msg:801`): `<secondary>{0} <dark_red>has messages disabled.`
- `userAFK` (`msg:1497`): `<gray>{0} <dark_purple>is currently AFK and may not respond.`
- `userAFKWithMessage` (`msg:1498`):
  `<gray>{0} <dark_purple>is currently AFK and may not respond\: {1}`
- `socialSpyPrefix` (`msg:1248`): `<white>[<primary>SS<white>] <reset>`
- `socialSpyMutedPrefix` (`msg:1243`): `<white>[<primary>SS<white>] <gray>(muted) <reset>`
- `socialSpyMsgFormat` (`msg:1242`):
  `<primary>[<secondary>{0}<gray> -> <secondary>{1}<primary>] <gray>{2}`
- `consoleName` (`msg:175`): `Console`
- `msgCommandUsage1Description` (`msg:795`):
  `Privately sends the given message to the specified player`

**Config.** `socialspy-messages: true` (`cfg:224`), `socialspy-listen-muted-players: true`
(`cfg:220`), `socialspy-uses-displaynames: true` (`cfg:228`), `last-message-reply-recipient: true`
(`cfg:681`), `last-message-reply-recipient-timeout: 180` (`cfg:687`).

**Bridge.** The online players with UUID, name and display name, a styled message to one player,
and the console as a recipient, which the log can take.

**Quirks.** Q1, Q2, Q3, Q5, Q10.

### r

**Names.** Command `r` with the aliases `er`, `reply` and `ereply`, usage `/<command> <message>`
(`yml:382-385`, `msg:1038-1041`). It checks `essentials.msg` (`E/User.java:134`), which the group
`default` holds.

**Behaviour** (`E/commands/Commandr.java:18-49`).

1. No argument prints the usage. A muted player is refused. The message is formatted as for `msg`.
2. With no reply target, or with a target the sender cannot see while
   `last-message-reply-vanished: false` (`cfg:692`, the live value, while the code default is
   true, `E/Settings.java:1703-1706`), the reply target is cleared and the sender reads
   `foreverAlone`.
3. Else the message goes to the target exactly as in steps 6 to 11 of `msg`. The target may be
   the console.

- `foreverAlone` (`msg:426`): `<dark_red>You have nobody to whom you can reply.`
- `rCommandUsage1Description` (`msg:1041`):
  `Replies to the last player to message you with the given text`

**Config.** `last-message-reply-vanished: false` (`cfg:692`).

**Quirks.** Q5.

### mail

**Names.** Command `mail` with the aliases `email`, `eemail`, `memo` and `ememo`, usage
`/<command> [read|clear|clear [number]|clear <player> [number]|send [to] [message]|sendtemp [to] [expire time] [message]|sendall [message]]`
(`yml:294-297`, `msg:728-745`).

| Node | Checked at | Live |
| --- | --- | --- |
| `essentials.mail` | `E/Essentials.java:807` | Group `default` |
| `essentials.mail.send` | `E/commands/Commandmail.java:67` | Group `default` |
| `essentials.mail.sendtemp` | `E/commands/Commandmail.java:108` | Operators |
| `essentials.mail.sendall` | `E/commands/Commandmail.java:151` | Operators |
| `essentials.mail.sendtempall` (not declared in `plugin.yml`) | `E/commands/Commandmail.java:161` | Operators |
| `essentials.mail.clear.others` | `E/commands/Commandmail.java:176` | Operators |
| `essentials.mail.clearall` | `E/commands/Commandmail.java:206` | Operators |
| `essentials.mail.color` and the other format nodes of `mail` | `E/commands/Commandmail.java:86` | Operators |

**Behaviour for a player** (`E/commands/Commandmail.java:33-216`).

- `mail read [page]`:
  1. With no mail, `noMail`.
  2. Every expired mail is removed, and every other mail is turned into a line and marked read,
     all of them, not only the page shown (`E/commands/Commandmail.java:41-52`).
  3. The lines show in pages of nine, with the header `infoPages` titled `Mail: read` and
     `readNextPage` when more pages follow (`E/textreader/SimpleTextPager.java:26-68`). A page out
     of range reads `infoUnknownChapter`.
  4. Then `mailClear`, and the list is saved (`E/commands/Commandmail.java:62-63`). When every
     stored mail has expired, the player reads `noMail` and the command ends before the save, so
     the expired mails stay in the file (`E/commands/Commandmail.java:54-57`), since
     `getMailMessages` returns a copy (`E/UserData.java:389-391`).
  5. A mail line is `mailMessage` for an old plain mail, else `mailFormatNew` or
     `mailFormatNewRead`, with `Timed` appended for a mail that expires, and the arguments are the
     time sent as `yyyy/MM/dd HH:mm` in the server's time zone, the sender name and the text
     (`E/MailServiceImpl.java:18`, `E/MailServiceImpl.java:63-78`).
- `mail send <player> <message>`:
  1. Needs `essentials.mail.send` and refuses a muted sender.
  2. The target is any known player, online or not, vanished or not, by the lookup of `pay`.
     Not found reads `playerNeverOnServer`.
  3. The text is the arguments from the third on, with `§` codes removed, characters outside the
     printable Unicode ranges removed, and then `formatMessage(user, "essentials.mail", ...)`
     (`E/commands/Commandmail.java:86`, `E/utils/StringUtil.java:16`,
     `E/utils/StringUtil.java:45-47`). More than 1000 characters read `mailTooLong`.
  4. If the target does not ignore the sender, a server wide counter is checked: it resets when a
     minute has passed since its start and allows `mails-per-minute: 1000` mails (`cfg:668`) for
     the whole server, not per player (`E/commands/Commandmail.java:25-26`,
     `E/commands/Commandmail.java:91-99`). Past it, `mailDelay`. Then `UserMailEvent` fires and the
     mail is put first in the target's list (`E/MailServiceImpl.java:30-49`).
  5. If the target ignores the sender, the mail is dropped silently.
  6. In both cases the sender reads `mailSentTo` and their text
     (`E/commands/Commandmail.java:103-104`).
- `mail sendtemp <player> <expire time> <message>` is `send` with an expiry from `parseDateDiff`
  and `mailSentToExpire` (`E/commands/Commandmail.java:107-149`). An expiry that `parseDateDiff`
  cannot read is `illegalDate`, shown through `errorWithMessage`, here and in `sendtempall`
  (`E/utils/DateUtil.java:74-76`).
- `mail sendall <message>` and `mail sendtempall <expire time> <message>` load every known player
  on a worker thread and mail each, and read `mailSent` (`E/commands/Commandmail.java:150-169`,
  `E/commands/Commandmail.java:293-313`). They skip every check of `send`, since the task
  `SendAll` mails every user directly: no mute check, no limit of 1000 characters, no per-minute
  counter, and no ignore list, so a player who ignores the sender gets the mail all the same.
- `mail clear [number]` clears all mail, or the mail with that number counted over the stored list,
  expired mails included (`E/commands/Commandmail.java:170-204`). A number above the count reads
  `mailClearIndex`, an empty list `noMail` (`noMailOther` for another player), success
  `mailCleared`. A second argument that is no positive number needs `essentials.mail.clear.others`
  and names a player.
- `mail clearall` clears every player's mail on a worker thread and reads `mailClearedAll`
  (`E/commands/Commandmail.java:205-214`, `E/commands/Commandmail.java:387-397`).
- Anything else prints the usage.

**Behaviour for the console** (`E/commands/Commandmail.java:219-291`). `read` answers
`onlyPlayers`. `clear <player> [number]`, `clearall`, `send`, `sendtemp`, `sendall` and
`sendtempall` work as above, but without the length check, the ignore check and the counter, and
with `replaceFormat` on the text. Every send variant of the console answers `mailSent`, never
`mailSentTo` or `mailSentToExpire`, and does not echo the text
(`E/commands/Commandmail.java:255-257`, `266-267`, `270-271`, `275-276`, `286-287`).
`mail <player> <message>` without `send` also sends. Console mail names `Console` as the sender.

**On join**, a player with `essentials.mail` reads `noNewMail` when they have no unread mail, since
`notify-no-new-mail: true` (`cfg:162`), and else the [mail notice](#mail-notice)
(`E/EssentialsPlayerListener.java:493-501`).

**Stores.** The key `mail` of the player file, a list of entries with `legacy`, `read`, `message`,
and for new mails `sender-name`, `sender-uuid` (the text `null` for the console), `timestamp` and
`expire` in milliseconds, `expire` 0 for never (`E/config/holders/UserConfigHolder.java:116-128`,
`E/config/serializers/MailMessageSerializer.java:14-44`,
`Essentials/src/main/java/net/essentialsx/api/v2/services/mail/MailMessage.java:92-97`). The list
has no size limit.

**Messages.**

- `noMail` (`msg:873`): `<primary>You do not have any mail.`
- `noMailOther` (`msg:874`): `<secondary>{0} <primary>does not have any mail.`
- `mailClear` (`msg:724`): `<primary>To clear your mail, type<secondary> /mail clear<primary>.`
- `mailCleared` (`msg:725`): `<primary>Mail cleared\!`
- `mailClearedAll` (`msg:726`): `<primary>Mail cleared for all players\!`
- `mailClearIndex` (`msg:727`): `<dark_red>You must specify a number between 1-{0}.`
- `mailDelay` (`msg:746`): `Too many mails have been sent within the last minute. Maximum\: {0}`
- `mailFormatNew` (`msg:747`):
  `<primary>[<reset>{0}<primary>] <primary>[<reset>{1}<primary>] <reset>{2}`
- `mailFormatNewTimed` (`msg:748`):
  `<primary>[<yellow>⚠<primary>] <primary>[<reset>{0}<primary>] <primary>[<reset>{1}<primary>] <reset>{2}`
- `mailFormatNewRead` (`msg:749`):
  `<primary>[<reset>{0}<primary>] <primary>[<reset>{1}<primary>] <gray><i>{2}`
- `mailFormatNewReadTimed` (`msg:750`):
  `<primary>[<yellow>⚠<primary>] <primary>[<reset>{0}<primary>] <primary>[<reset>{1}<primary>] <gray><i>{2}`
- `mailMessage` (`msg:752`): `{0}`
- `mailSent` (`msg:753`): `<primary>Mail sent\!`
- `mailSentTo` (`msg:754`): `<secondary>{0}<primary> has been sent the following mail\:`
- `mailSentToExpire` (`msg:755`):
  `<secondary>{0}<primary> has been sent the following mail which will expire in <secondary>{1}<primary>\:`
- `mailTooLong` (`msg:756`):
  `<dark_red>Mail message too long. Try to keep it below 1000 characters.`
- `playerNeverOnServer` (`msg:949`):
  `<dark_red>Player<secondary> {0} <dark_red>was never on this server.`
- `illegalDate` (`msg:539`): `Illegal date format.`
- `noNewMail` (`msg:882`): `<primary>You have no new mail.`
- `infoPages` (`msg:545`):
  `<yellow> ---- <primary>{2} <yellow>--<primary> Page <secondary>{0}<primary>/<secondary>{1} <yellow>----`
- `readNextPage` (`msg:1043`): `<primary>Type<secondary> /{0} {1} <primary>to read the next page.`
- `infoUnknownChapter` (`msg:546`): `<dark_red>Unknown chapter.`

**Config.** `mails-per-minute: 1000` (`cfg:668`), `notify-no-new-mail: true` (`cfg:162`),
`notify-player-of-mail-cooldown: 60` (`cfg:166`).

**Bridge.** Join events, a styled message, and storage. Nothing else of the game.

**Quirks.** Q1, Q3, Q5, Q6, Q7, Q8.

### ignore

**Names.** Command `ignore` with the aliases `eignore`, `unignore`, `eunignore`, `delignore`,
`edelignore`, `remignore`, `eremignore`, `rmignore` and `ermignore`, usage `/<command> <player>`
(`yml:218-221`, `msg:531-534`). Every alias toggles, so `unignore` does not only remove. It checks
`essentials.ignore` (group `default`). Players only.

**Behaviour** (`E/commands/Commandignore.java:16-43`).

1. Without an argument the player reads `ignoredList` with the names of their ignored players,
   joined by spaces, or `noIgnored`.
2. The target is any known player by the lookup of `ignore` in the table above.
3. A target with `essentials.chat.ignoreexempt` reads `ignoreExempt`, even if they are already on
   the list.
4. A target on the list is removed: `unignorePlayer`.
5. The player themselves: `ignoreYourself`.
6. Else the target is added: `ignorePlayer`.

**Effect.** An ignored player's chat does not reach the ignoring player
(`E/EssentialsPlayerListener.java:214-228`), nor their `msg`, `r` and `mail`, nor the AFK and
`suicide` broadcasts about them (`E/Essentials.java:1215-1218`), nor their `me` when the chat radius
is above 0 (`E/commands/Commandme.java:62-63`). With radius 0, the default, `me` reaches the
ignoring player, see Q4.
`pay` ignores the list, since `pay-excludes-ignore-list: false` (`cfg:894`).

**Stores.** The key `ignore` of the player file, a list of UUIDs
(`E/config/holders/UserConfigHolder.java:150-162`, `E/UserData.java:451-461`).

- `ignoredList` (`msg:535`): `<primary>Ignored\:<reset> {0}`
- `noIgnored` (`msg:867`): `<primary>You are not ignoring anyone.`
- `ignoreExempt` (`msg:536`): `<dark_red>You may not ignore that player.`
- `ignorePlayer` (`msg:537`): `<primary>You ignore player<secondary> {0} <primary>from now on.`
- `unignorePlayer` (`msg:1470`):
  `<primary>You are not ignoring player<secondary> {0} <primary>anymore.`
- `ignoreYourself` (`msg:538`): `<primary>Ignoring yourself won''t solve your problems.`

**Quirks.** Q1, Q4, Q31.

### me

**Names.** Command `me` with the aliases `action`, `eaction`, `describe`, `edescribe` and `eme`,
usage `/<command> <description>` (`yml:298-301`, `msg:764-767`).

| Node | Checked at | Live |
| --- | --- | --- |
| `essentials.me` | `E/Essentials.java:807` | Group `default` |
| `essentials.chat.color`, `.format`, `.magic`, `.rgb`, `.url` | `E/commands/Commandme.java:40` | Operators |
| `essentials.chat.spy`, for local `me` out of range | `E/commands/Commandme.java:71` | Operators |

**Behaviour** (`E/commands/Commandme.java:26-111`).

1. A muted player is refused, then no argument prints the usage.
2. The text goes through `formatMessage(user, "essentials.chat", ...)`.
3. `chat.radius` is 0 (`cfg:970`, `E/Settings.java:225-232`), so `action` goes to every online
   player through `broadcastTl` without a sender, which applies no ignore list and no vanish check,
   and `UserActionEvent` fires (`E/commands/Commandme.java:43-48`, `E/Essentials.java:1189-1192`).
4. With a radius above 0 the text would go to players in the same world within the radius who do
   not ignore the sender, plus spies, and `localNoOne` would follow when nobody heard it
   (`E/commands/Commandme.java:49-98`). `localNoOne` is empty (`msg:721`), so it would show
   nothing.
5. The console broadcasts `action` with `@` as the name and `replaceFormat` on the text
   (`E/commands/Commandme.java:102-111`).

`/me` does not reach the console. `broadcastTl` loops over the online players only
(`E/Essentials.java:1215-1232`), so the line of `action` is not in the server log, apart from the
command itself. A port that logs it changes that.

- `action` (`msg:2`): `<dark_purple>* {0} <dark_purple>{1}`

**Config.** `chat.radius: 0` (`cfg:970`).

**Quirks.** Q2, Q4, Q5.

### helpop

**Names.** Command `helpop` with the aliases `ac`, `eac`, `amsg`, `eamsg` and `ehelpop`, usage
`/<command> <message>` (`yml:206-209`, `msg:501-504`). It checks `essentials.helpop` (group
`default`). `essentials.helpop.receive` (`yml:876`, operators) receives.

**Behaviour** (`E/commands/Commandhelpop.java:25-63`).

1. No argument prints the usage. The text loses its `§` codes and is not otherwise formatted.
2. The log gets `helpOp` as legacy text.
3. Every online player with `essentials.helpop.receive` is a recipient.
   `HelpopMessageSendEvent` fires and may change the recipients.
4. The sender reads `helpOp` too when they are not a recipient, and each recipient reads `helpOp`.
5. The console sends as `Console`.

There is no mute check, no ignore check and no limit.

- `helpOp` (`msg:499`): `<dark_red>[HelpOp]<reset> <primary>{0}\:<reset> {1}`

**Quirks.** Q5, Q9.

## Info

### list

**Names.** Command `list` with the aliases `elist`, `online`, `eonline`, `playerlist`,
`eplayerlist`, `plist`, `eplist`, `who` and `ewho`, usage `/<command> [group]` (`yml:286-289`,
`msg:712-715`). It checks `essentials.list` (group `default`). `essentials.list.hidden` or
`essentials.vanish.interact` shows vanished players (operators, `E/commands/Commandlist.java:24`).

**Behaviour** (`E/commands/Commandlist.java:19-41`, `E/PlayerList.java:25-221`).

1. The summary is `listAmount` with the visible count and `server.getMaxPlayers()`, or
   `listAmountHidden` with the count of hidden players the sender may see in the middle
   (`E/PlayerList.java:53-76`).
2. The online players are grouped by their group in lower case, which is always `default` on the
   live server (`E/PlayerList.java:79-90`, [the handler](#the-handler-without-vault)).
3. Without an argument, the groups of the `list` section are shown first. The section is
   `Admins: owner admin` (`cfg:445-454`), which merges the groups `owner` and `admin`. No player is
   in them, so nothing shows. The remaining groups follow, sorted, each as `listGroupTag` with the
   group name, which is `default` in lower case, and the players (`E/PlayerList.java:137-221`, the
   group name at `E/PlayerList.java:209`).
4. With a group argument, only that group shows, together with the groups its entry of the
   `list` section merges, or `groupDoesNotExist`. Its tag writes the argument with the first
   letter in title case, so `/list default` reads `Default: ...`, unlike the lower case `default: `
   of the grouped list (`E/PlayerList.java:117-129`).
5. Each player is shown as `listAfkTag` when AFK, `listHiddenTag` when vanished, then their display
   name, separated by `, ` and sorted by display name without codes, case insensitive
   (`E/PlayerList.java:25-50`, `E/User.java:1161-1164`). Every name is followed by `<white>`,
   which ends the operator colour before the separator (`E/PlayerList.java:47`).
   `real-names-on-list: false` (`cfg:457`) adds no real names.
6. The console sees everybody.

- `listAmount` (`msg:710`):
  `<primary>There are <secondary>{0}<primary> out of maximum <secondary>{1}<primary> players online.`
- `listAmountHidden` (`msg:711`):
  `<primary>There are <secondary>{0}<primary>/<secondary>{1}<primary> out of maximum <secondary>{2}<primary> players online.`
- `listGroupTag` (`msg:716`): `<primary>{0}<reset>\: ` (with a trailing space)
- `listAfkTag` (`msg:709`): `<gray>[AFK]<reset>`
- `listHiddenTag` (`msg:717`): `<gray>[HIDDEN]<reset>`
- `listRealName` (`msg:718`): `({0})`
- `groupDoesNotExist` (`msg:471`): `<dark_red>There''s no one online in this group\!`
- `groupNumber` (`msg:472`): `<secondary>{0}<white> online, for the full list\:<secondary> /{1} {2}`
- `connectedPlayers` (`msg:173`): `<primary>Connected players<reset>`, never shown, see Q26

So a live `/list` with Alex and an operator Steve reads `There are 2 out of maximum <n> players
online.` and `default: Alex, Steve` with Steve in dark red and the comma white; `/list default`
reads `Default: Alex, Steve`.

**Config.** `list` (`cfg:445-454`), `real-names-on-list: false` (`cfg:457`).

**Bridge.** Online players with UUID, name, op flag and AFK flag, and the maximum player count.

**Quirks.** Q2, Q26, Q27.

### seen

**Names.** Command `seen` with the aliases `eseen`, `ealts` and `alts`, usage
`/<command> <playername>` (`yml:418-421`, `msg:1122-1125`). It checks `essentials.seen` (group
`default`). Operators also hold `essentials.seen.banreason`, `.ip`, `.location`, `.uuid`,
`.firstlogin`, `.whitelist`, `.ipsearch`, `.alts` (the label must contain `alts`) and
`essentials.geoip.show` (`E/commands/Commandseen.java:33-37`, `yml:1157-1182`).

**Behaviour** (`E/commands/Commandseen.java:29-230`).

1. No argument prints the usage.
2. A UUID or an exact user map name finds the player (`E/commands/Commandseen.java:40-46`).
3. If nobody is found, an IP with `.ipsearch` lists accounts from it, a banned IP reads
   `isIpBanned`, and a banned name reads `whoisBanned`. Else a worker thread looks the name up again
   and then by the online lookup of `msg`, or reads `playerNeverOnServer`
   (`E/commands/Commandseen.java:48-86`).
4. With a label that contains `alts` (`alts`, `ealts`) and `essentials.seen.alts`, the found
   player is not reported. The sender reads `runningPlayerMatch` with the found player's display
   name, and a worker thread loads every known player file and lists the accounts whose last login
   address equals theirs, as `matchingIPAddress` and `matchingAccounts`, or `noMatchingPlayers`
   (`E/commands/Commandseen.java:37`, `93-94`, `203-230`). The IP and ban checks of step 3 are
   skipped with that label, so `/alts <address>` does not search by address
   (`E/commands/Commandseen.java:49`). Without the node, `/alts` is a plain `seen`.
5. An online player the sender can see gets `seenOnline` with the time since their login, then
   `seenAccounts` if they had other names, `whoisUuid` with `.uuid`, `whoisFirstLogin` with
   `.firstlogin`, `whoisAFK` when AFK, `whoisJail` when jailed, `whoisMuted` or `whoisMutedReason`
   when muted, `whoisGeoLocation` when known and allowed, and `whoisIPAddress` with `.ip`
   (`E/commands/Commandseen.java:102-141`). The first login is no field of Essentials: it is
   Bukkit's `OfflinePlayer.getFirstPlayed()` (*outside the checkout*: CraftBukkit keeps it in the
   player's `.dat` as `bukkit.firstPlayed`), written with `DateFormat.LONG` for date and time in
   the plugin's locale (`E/commands/Commandseen.java:116-118`, `E/utils/DateUtil.java:176-180`).
6. Anyone else, vanished players included, gets `seenOffline` with the time since their logout,
   then `seenAccounts` only if they had more than one other name, and `whoisUuid` with `.uuid`. A
   player without a logout time reads `userUnknown` instead of these three
   (`E/commands/Commandseen.java:145-157`). Then come `whoisWhitelist` with `.whitelist`
   (`E/commands/Commandseen.java:159-161`), `whoisBanned` and `whoisTempBanned` when banned, the
   mute lines, the geo line, `whoisIPAddress` with `.ip` and a known address, and `whoisLocation`
   of their logout with `.location` (`E/commands/Commandseen.java:143-201`). An offline player gets
   no first login line. The ban reason needs `.banreason`. Without it the ban line reads `true`.

The mute, jail, AFK and ban lines show to every holder of `essentials.seen`, see Q23.

- `seenOnline` (`msg:1127`):
  `<primary>Player<secondary> {0} <primary>has been <green>online<primary> since <secondary>{1}<primary>.`
- `seenOffline` (`msg:1126`):
  `<primary>Player<secondary> {0} <primary>has been <dark_red>offline<primary> since <secondary>{1}<primary>.`
- `seenAccounts` (`msg:1121`): `<primary>Player has also been known as\:<secondary> {0}`
- `userUnknown` (`msg:1514`):
  `<dark_red>Warning\: The user ''<secondary>{0}<dark_red>'' has never joined this server.`
- `whoisUuid` (`msg:1607`): `<primary> - UUID\:<reset> {0}`
- `whoisFirstLogin` (`msg:1608`): `<primary> - First login:<reset> {0}`
- `whoisAFK` (`msg:1581`): `<primary> - AFK\:<reset> {0}`
- `whoisJail` (`msg:1597`): `<primary> - Jail\:<reset> {0}`
- `whoisMuted` (`msg:1600`): `<primary> - Muted\:<reset> {0}`
- `whoisMutedReason` (`msg:1601`): `<primary> - Muted\:<reset> {0} <primary>Reason\: <secondary>{1}`
- `whoisGeoLocation` (`msg:1592`): `<primary> - Location\:<reset> {0}`
- `whoisIPAddress` (`msg:1596`): `<primary> - IP Address\:<reset> {0}`
- `whoisWhitelist` (`msg:1609`): `<primary> - Whitelist\:<reset> {0}`
- `whoisBanned` (`msg:1583`): `<primary> - Banned\:<reset> {0}`
- `whoisTempBanned` (`msg:1605`): `<primary> - Ban expires\:<reset> {0}`
- `whoisLocation` (`msg:1598`): `<primary> - Location\:<reset> ({0}, {1}, {2}, {3})`
- `isIpBanned` (`msg:576`): `<primary>IP <secondary>{0} <primary>is banned.`
- `runningPlayerMatch` (`msg:1118`):
  `<primary>Running search for players matching ''<secondary>{0}<primary>'' (this could take a little while).`
- `matchingIPAddress` (`msg:758`):
  `<primary>The following players previously logged in from that IP address\:`
- `matchingAccounts` (`msg:759`): `{0}`
- `noMatchingPlayers` (`msg:875`): `<primary>No matching players found.`
- `true` (`msg:1455`): `<green>true<reset>`, `false` (`msg:391`): `<dark_red>false<reset>`,
  `now` (`msg:900`): `now`

**Reads.** `timestamps.login`, `timestamps.logout`, `past-usernames`, `last-account-name`,
`ip-address`, `logoutlocation`, `muted`, `mute-reason`, `timestamps.mute`, `jailed` and
`geolocation` (see [Storage](#storage)), and Bukkit's first played time of an online player for
`.firstlogin`.

**Bridge.** Login and logout times and the logout position from join and quit events, the ban
entry of a name with reason and expiry, the whitelist flag, the address at login, for operators,
and the time of a player's first join, for operators.

**Quirks.** Q1, Q23, Q24, Q32.

### realname

**Names.** Command `realname` with the alias `erealname`, usage `/<command> <nickname>`
(`yml:390-393`, `msg:1045-1048`). It checks `essentials.realname` (group `default`).

**Behaviour** (`E/commands/Commandrealname.java:16-38`). No argument prints the usage. Every online
player whose display name, without codes and in lower case, contains the argument in lower case
is listed with `realName`, vanished players skipped for senders without
`essentials.vanish.interact`. No match reads `playerNotFound`. It refreshes the display name of
every online player on the way (`E/commands/Commandrealname.java:29`).

- `realName` (`msg:1044`): `<white>{0}<reset><primary> is <white>{1}`

### motd and the motd on join

**Names.** Command `motd` with the alias `emotd`, usage `/<command> [chapter] [page]`
(`yml:306-309`, `msg:789-790`). It checks `essentials.motd` (group `default`). It has no usage
keys, so a usage error would print the lines of `plugin.yml`.

**The text file** (`E/textreader/TextInput.java:35-109`).

1. For a player, `motd_<name>.txt`, then `motd_<group>.txt` (live always `motd_default.txt`), then
   `motd.txt` in the data folder, the first that exists. Names are sanitised for file names.
2. A missing `motd.txt` is copied from the jar, and the call fails with
   `File motd.txt does not exist. Creating one for you.` (`E/textreader/TextInput.java:97-107`).
3. Lines are read as UTF-8, and `&` codes become `§` codes (`E/textreader/TextInput.java:84`). A
   line that starts with `#` and is longer than one character starts a chapter. Its comma
   separated titles become bookmarks (`E/textreader/TextInput.java:77-83`).
4. Files are cached by name and reread when their time changes
   (`E/textreader/TextInput.java:48-66`).

The live folder holds the sample `motd.txt` (`plugins.md:37`), which is `R/motd.txt`:

```text
&6Welcome, {PLAYER}&6!
&6Type &c/help&6 for a list of commands.
&6Type &c/list&6 to see who else is online.
&6Players online:&c {ONLINE} &6- World time:&c {WORLDTIME12}
```

**Keywords** (`E/textreader/KeywordReplacer.java:34-402`). Each `{NAME}` or `{NAME:sub}` with a
known name is replaced. The sample uses three of them.

| Keyword | Value | Code |
| --- | --- | --- |
| `{PLAYER}`, `{DISPLAYNAME}` | Display name | `E/textreader/KeywordReplacer.java:194-199` |
| `{ONLINE}` | Online players minus vanished ones | `E/textreader/KeywordReplacer.java:251-259` |
| `{WORLDTIME12}` | Day time of the player's world as `h:mm aa` in English, tick 0 being 6:00 AM | `E/textreader/KeywordReplacer.java:327-331`, `E/utils/DescParseTickFormat.java:35`, `E/utils/DescParseTickFormat.java:210-253` |

The others are `USERNAME`, `NICKNAME`, `PREFIX`, `SUFFIX`, `GROUP`, `BALANCE`, `MAILS`, `PLAYTIME`,
`WORLD`, `WORLDNAME`, `UNIQUE`, `WORLDS`, `PLAYERLIST`, `TIME`, `DATE`, `WORLDTIME24`, `WORLDDATE`,
`COORDS`, `TPS`, `UPTIME`, `IP`, `ADDRESS`, `PLUGINS` and `VERSION`
(`E/textreader/KeywordReplacer.java:34-62`). An unknown name stays as typed
(`E/textreader/KeywordReplacer.java:398-399`). The port needs only the three of the live file, but
the migration must refuse a live file that uses others it does not support.

**/motd** (`E/commands/Commandmotd.java:15-22`, `E/textreader/TextPager.java:23-141`).

1. The display name is refreshed. With no argument or a number, the part before the first chapter
   shows in pages of nine: `infoPages` with `Motd` as title, each line prefixed `§r`, and
   `readNextPage` when more follow. A page past the end reads `infoUnknownChapter`.
2. A file that starts with a chapter line lists the chapters after `infoChapter` instead.
3. A chapter name shows that chapter with `infoChapterPages` and pages of nine. An unknown one
   reads `infoUnknownChapter`.
4. The console sees the same, with empty player keywords.

**On join** (`E/EssentialsPlayerListener.java:409-616`). The join work runs on a worker thread and
then on a later tick of the server thread (`E/EssentialsPlayerListener.java:409-416`,
`E/EssentialsPlayerListener.java:583-616`). With `delay-motd: 0` (`cfg:738`) the MOTD shows at
once in that tick (`E/EssentialsPlayerListener.java:483-491`): if `motd` is not disabled, the file
has lines and the player holds `essentials.motd`, up to 20 lines before the first chapter show
without header (`E/EssentialsPlayerListener.java:557-581`, `E/textreader/TextPager.java:61-89`).
The mail line follows (`E/EssentialsPlayerListener.java:493-501`). With the live file a new player
Alex reads:

```text
Welcome, Alex!
Type /help for a list of commands.
Type /list to see who else is online.
Players online: 1 - World time: 6:00 AM
```

in gold with the commands and values in red, at day time 0, and then `You have no new mail.`

The join flow also does things outside this part, which the port must place somewhere:
`last-account-name` and `past-usernames` (`E/EssentialsPlayerListener.java:421-424`,
`E/UserData.java:614-622`), `timestamps.login` and `ip-address` (`E/UserData.java:553-559`), the
compass target (`E/EssentialsPlayerListener.java:620-633`), and a reset of the walk and fly speed to
0.2 and 0.1 for players without `essentials.speed` (`E/EssentialsPlayerListener.java:531-534`),
which may undo speeds that mods set. It also switches social spy, fly mode and god mode off for
players who have them on without `essentials.socialspy`, `essentials.fly` or `essentials.god`, with
a log line each (`E/EssentialsPlayerListener.java:536-549`), and clears the pending confirmation
of `clearinventory` and every pending pay confirmation (`E/EssentialsPlayerListener.java:551-552`),
so a confirmation of [pay](#pay) does not survive a relog.

- `infoPages` (`msg:545`): see [mail](#mail)
- `infoChapter` (`msg:541`): `<primary>Select chapter\:`
- `infoChapterPages` (`msg:542`):
  `<yellow> ---- <primary>{0} <yellow>--<primary> Page <secondary>{1}<primary> of <secondary>{2} <yellow>----`
- `infoUnknownChapter` (`msg:546`): `<dark_red>Unknown chapter.`
- `readNextPage` (`msg:1043`): see [mail](#mail)

**Config.** `delay-motd: 0` (`cfg:738`).

**Bridge.** The join event, the day time of the player's dimension and the online count. Nothing
is stored.

### rules

**Names.** Command `rules` with the alias `erules`, usage `/<command> [chapter] [page]`
(`yml:414-417`, `msg:1116-1117`). It checks `essentials.rules` (group `default`).

**Behaviour** (`E/commands/Commandrules.java:15-22`). The same as `/motd` with the files
`rules_<name>.txt`, `rules_<group>.txt` and `rules.txt`, the title `Rules`, and no join part. The
default `R/rules.txt` is:

```text
[1] Be respectful
[2] Be ethical
[3] Use common sense
```

The live folder has no `rules.txt` in the file list of `plugins.md:37`. The first `/rules` then
copies the default and shows `Error: File rules.txt does not exist. Creating one for you.`, through
`errorWithMessage` (`E/textreader/TextInput.java:97-107`, `E/Essentials.java:895-897`), and the next
`/rules` shows the three lines. See Q25.

### help

**Names.** Command `help` with the alias `ehelp`, usage `/<command> [search term] [page]`
(`yml:202-205`, `msg:493-494`). It checks `essentials.help` (group `default`). Players only. The
console reads `helpConsole`.

**Behaviour** (`E/commands/Commandhelp.java:30-89`, `E/textreader/HelpInput.java:25-127`).

1. A `help.txt` would be shown like the MOTD. There is none live (`plugins.md:37`).
2. `/help /<command>` shows the help of that command: `commandHelpLine1`, `commandHelpLine2`,
   `commandHelpLine4` with the aliases, `commandHelpLine3` and the usage lines
   (`E/commands/Commandhelp.java:38-65`). The command is looked up in the server's map of known
   commands, which also holds the vanilla commands (*outside the checkout*, CraftBukkit), so
   `/help /gamerule` answers for vanilla `gamerule` although the list of step 3 never shows vanilla.
   `/help /gamemode` is no example, since `gamemode` is an Essentials label as well (`yml:170`),
   and which command owns the plain label depends on the registration order of CraftBukkit. A
   command that is not Essentials, or an Essentials command that nobody has run or tab completed
   since the start, prints the `description` and `usage` it carries instead of the usage lines
   (`E/commands/Commandhelp.java:40-61`, the command map of Essentials at
   `E/commands/Commandhelp.java:44` and the fallback at `59-61`). A usage line without its
   description key throws, which `/help /tptoggle` shows as `errorWithMessage` with the Java text
   of the exception (Q38 of the teleport page).
3. Else a list is built of the commands of every Bukkit plugin, never of vanilla or mod commands
   (`E/textreader/HelpInput.java:34-41`). An Essentials command shows when it is not disabled and
   the player holds `essentials.<name>`. With `non-ess-in-help: true` (`cfg:946`) and
   `hide-permissionless-help: true` (`cfg:952`) a command of another plugin shows when the player
   holds `essentials.help.<plugin>`, `essentials.help.<plugin>.<command>` or one of its own
   permissions. A command without permission stays hidden (`E/textreader/HelpInput.java:69-104`).
4. A search term of more than 26 characters is cut to 25. It keeps commands whose plugin, name or
   description contains it (`E/commands/Commandhelp.java:70-75`,
   `E/textreader/HelpInput.java:62-67`). A term equal to a plugin name lists that plugin.
5. Without a search term the text starts with one `helpPlugin` line per plugin that has a
   command to show, all before the first command line. With a term these lines are left out
   (`E/textreader/HelpInput.java:109-116`, `126`).
6. The result is paged by nine as for the MOTD, with the title `Help` or `Help: <term>`.

- `helpConsole` (`msg:495`): `To view help from the console, type ''?''.`
- `helpMatching` (`msg:498`): `<primary>Commands matching "<secondary>{0}<primary>"\:`
- `helpFrom` (`msg:496`): `<primary>Commands from {0}\:`
- `helpLine` (`msg:497`): `<primary>/{0}<reset>\: {1}`
- `helpPlugin` (`msg:500`): `<dark_red>{0}<reset>\: Plugin Help\: /help {1}`
- `commandHelpLine4` (`msg:156`): `<primary>Aliases(s)\: <white>{0}`
- `commandHelpFailedForPlugin` (`msg:152`, log only): `Error getting help for plugin\: {0}`

**Config.** `non-ess-in-help: true` (`cfg:946`), `hide-permissionless-help: true` (`cfg:952`).

**Golem.** The list becomes the commands of the Golem plugins the player may use, with their
descriptions. See Q2 and Q34.

### ping

**Names.** Command `ping` with the aliases `echo`, `eecho`, `eping`, `pong` and `epong`, usage
`/<command>` (`yml:350-353`, `msg:937-938`). It checks `essentials.ping` (group `default`).

**Behaviour** (`E/commands/Commandping.java:14-20`). No argument reads `pong`. Any argument is
echoed back to the sender alone, with every `&` code turned into a `§` code. It measures nothing.

- `pong` (`msg:963`): `Pong\!`

**Quirks.** Q10 (the echo lets a player colour text for themselves, and it reaches nobody else).

### getpos

**Names.** Command `getpos` with the aliases `coords`, `egetpos`, `position`, `eposition`,
`whereami`, `ewhereami`, `getlocation`, `egetlocation`, `getloc` and `egetloc`, usage
`/<command> [player]` (`yml:178-181`, `msg:443-446`). It checks `essentials.getpos` (group
`default`), and `essentials.getpos.others` (operators) for another player.

**Behaviour** (`E/commands/Commandgetpos.java:17-45`).

1. Without the others node, or without an argument, the sender's own position: `currentWorld` with
   the Bukkit world name, `posX`, `posY` and `posZ` with block coordinates, `posYaw` with
   `(yaw + 360) % 360`, `posPitch`.
2. With the node and an argument, the position of that online player and `distance` from the
   sender when both are in the same world.
3. The console needs a player argument.

Numbers go through `MessageFormat`, so `posX` of 1234 reads `X: 1,234`, and yaw and pitch show up
to three decimals. See Q11.

- `currentWorld` (`msg:192`): `<primary>Current World\:<secondary> {0}`
- `posX` (`msg:974`): `<primary>X\: {0} (+East <-> -West)`
- `posY` (`msg:975`): `<primary>Y\: {0} (+Up <-> -Down)`
- `posZ` (`msg:977`): `<primary>Z\: {0} (+South <-> -North)`
- `posYaw` (`msg:976`): `<primary>Yaw\: {0} (Rotation)`
- `posPitch` (`msg:964`): `<primary>Pitch\: {0} (Head angle)`
- `distance` (`msg:304`): `<primary>Distance\: {0}`

**Bridge.** Position, rotation and dimension of a player. The world name of Bukkit, such as
`world` or `world_nether`, is not the dimension id. Golem shows the dimension id, such as
`minecraft:overworld`.

### compass

**Names.** Command `compass` with the aliases `ecompass`, `direction` and `edirection`, usage
`/<command>` (`yml:95-98`, `msg:161-162`). It checks `essentials.compass` (group `default`).
Players only.

**Behaviour** (`E/commands/Commandcompass.java:12-35`). The bearing is
`(int) (yaw + 180 + 360) % 360`, so north is 0. Below 23 is `north`, below 68 `northEast`, below
113 `east`, below 158 `southEast`, below 203 `south`, below 248 `southWest`, below 293 `west`,
below 338 `northWest`, and the rest `north`. It sends `compassBearing`.

- `compassBearing` (`msg:160`): `<primary>Bearing\: {0} ({1} degrees).`
- `north` (`msg:863`): `N`, `northEast` (`msg:862`): `NE`, `east` (`msg:315`): `E`,
  `southEast` (`msg:1220`): `SE`, `south` (`msg:1221`): `S`, `southWest` (`msg:1222`): `SW`,
  `west` (`msg:1580`): `W`, `northWest` (`msg:864`): `NW`

**Bridge.** The yaw of a player, in the convention of the game, where 0 faces south.

### depth

**Names.** Command `depth` with the aliases `edepth`, `height` and `eheight`, usage `/<command>`
in `plugin.yml` (`yml:122-125`) and `/depth` in `depthCommandUsage` (`msg:232-233`). It checks
`essentials.depth` (group `default`). Players only.

**Behaviour** (`E/commands/Commanddepth.java:12-21`). The block Y minus the sea level of the
player's world: above 0 `depthAboveSea`, below 0 `depthBelowSea` with the absolute value, else
`depth`.

- `depthAboveSea` (`msg:230`): `<primary>You are<secondary> {0} <primary>block(s) above sea level.`
- `depthBelowSea` (`msg:231`): `<primary>You are<secondary> {0} <primary>block(s) below sea level.`
- `depth` (`msg:229`): `<primary>You are at sea level.`

**Bridge.** The block Y of a player and the sea level of their dimension, which comes from the noise
settings of each dimension, the Skylands datapack dimension included.

## Status

### afk

**Names.** Command `afk` with the aliases `eafk`, `away` and `eaway`, usage
`/<command> [player/message...]` (`yml:11-14`, `msg:6-11`).

| Node | Checked at | Live |
| --- | --- | --- |
| `essentials.afk` | `E/Essentials.java:807` | Group `default` |
| `essentials.afk.message` | `E/commands/Commandafk.java:64` | Operators |
| `essentials.afk.others` | `E/commands/Commandafk.java:20` | Operators |
| `essentials.afk.auto`, `default: false` | `E/User.java:906` | Nobody |
| `essentials.afk.kickexempt` | `E/User.java:880` | Operators |
| `essentials.kick.exempt`, `default: false` | `E/User.java:879` | Nobody |
| `essentials.sleepingignored`, on every join and every AFK change | `E/EssentialsPlayerListener.java:448-449`, `E/User.java:690` | Operators |

**The command** (`E/commands/Commandafk.java:19-104`).

1. With `essentials.afk.others` and an argument, the first argument is tried as an online player.
   If it matches, the rest is the message, else all arguments are the message for the sender.
   Without the node, all arguments are the message.
2. A message from a player needs them unmuted and holding `essentials.afk.message`, else
   `noPermToAFKMessage`. So a player of the group `default` can only type `/afk` alone.
3. The status is toggled with cause `COMMAND`, and `AfkStatusChangeEvent` can cancel it
   (`E/User.java:683-701`).
4. Going AFK stores the position and the time, sets the message, and marks the player as ignored
   for sleeping, since `sleep-ignores-afk-players: true` (`cfg:540`, `E/User.java:690-693`). The
   others read `userIsAway` (or `userIsAwayWithMessage`), the player `userIsAwaySelf` (or
   `userIsAwaySelfWithMessage`), unless the player is vanished.
5. Leaving AFK clears position, message and time, and the same messages with `NotAway` follow.
6. The broadcast goes through `broadcastTl` with the player as sender, so players who ignore them
   and the player themselves are left out, and only with `broadcast-afk-message: true` (`cfg:555`)
   (`E/commands/Commandafk.java:96-102`).
7. From the console, `afk <player> [message]`, with no message checks.

**Activity** ends AFK. `updateActivity` sets the player not AFK, and when it broadcasts, sends the
`NotAway` pair as above. It always stores the time of the activity (`E/User.java:835-847`). These
count as activity:

| Source | Code | Broadcast |
| --- | --- | --- |
| Moving more than 3 blocks away from the AFK position, or into another world, checked on every block change, with `cancel-afk-on-move: true` (`cfg:529`) | `E/EssentialsPlayerListener.java:234-286` | Yes |
| Any command but `afk`, with `cancel-afk-on-interact: true` (`cfg:524`) | `E/EssentialsPlayerListener.java:846-863` | Yes, but not for `vanish` |
| Any interaction but the physical ones, such as stepping on a pressure plate | `E/EssentialsPlayerListener.java:966-1013` | Yes |
| Attacking an entity or shooting a player, and shooting a bow while AFK | `E/EssentialsEntityListener.java:63`, `E/EssentialsEntityListener.java:71`, `E/EssentialsEntityListener.java:256-264` | Yes |
| Fishing, with `cancel-afk-on-fish: true` (`cfg:535`) | `E/EssentialsPlayerListener.java:1193-1201` | Yes |
| Chat, with `cancel-afk-on-chat: true` (`cfg:532`), run on the server thread | `E/EssentialsPlayerListener.java:230`, `E/User.java:861-866` | Yes |
| Join and quit | `E/EssentialsPlayerListener.java:600`, `E/EssentialsPlayerListener.java:335` | No |

`freeze-afk-players: false` (`cfg:517`) leaves AFK players movable and vulnerable, and
`disable-item-pickup-while-afk: false` (`cfg:521`) lets them pick items up.

**Automatic AFK and the kick** (`E/User.java:868-916`). A timer runs 1000 ticks after start and
then every 50 ticks, for every online player (`E/Essentials.java:432-433`,
`E/EssentialsTimer.java:30-68`). After a grace of 10 seconds since the last activity:

1. `auto-afk-timeout: -1` (`cfg:495`, read with a fallback to the old key `auto-afk-kick`,
   `E/Settings.java:1288-1291`) disables the kick. With a positive value a player without
   `essentials.kick.exempt` and `essentials.afk.kickexempt` would be kicked with
   `autoAfkKickReason` and holders of `essentials.kick.notify` would read `playerKicked`, or the
   commands of `afk-timeout-commands` (empty, `cfg:509-511`) would run as the console.
2. `auto-afk: 300` (`cfg:489`) sets a player AFK after 300 seconds without activity, with cause
   `ACTIVITY` and the same broadcast, but only if they hold `essentials.afk.auto`. Nobody holds it
   live, see [Nodes nobody holds](#nodes-nobody-holds), so automatic AFK never happens on the live
   server. See Q21.

**The list name.** `afk-list-name: "none"` (`cfg:550`) leaves the tab list alone
(`E/Settings.java:821-822`, `E/User.java:703-713`). With a value it would set the tab list name to
it, with `{PLAYER}` and `{USERNAME}` replaced. The plan asks for a mark in the player list for AFK
players (the row Status of [EssentialsX 2.22.0](../../plan.md#essentialsx-2220)). That is a change
of the live behaviour, which the bridge action `listName` serves
([Actions](../../bridge.md#actions)). It needs the owner's confirmation as a change, as Q21 asks for
automatic AFK.

**Sleep.** The node `essentials.sleepingignored` acts on every join too, not only on an AFK
change. The join flow sets every holder as ignored for the sleep count
(`E/EssentialsPlayerListener.java:448-449`), and an AFK change keeps holders ignored
(`E/User.java:690`). The node has no default (`yml:1243-1244`), so it goes to every operator, and
the code leaves every operator out of the sleep count from their join on, AFK or not. *Outside the
checkout*: Arclight 1.0.6 never reads the flag that `setSleepingIgnored` writes, so on the live
server operators and AFK players count for sleeping, and the port keeps that (Q37, see
[Rulings and issues](#rulings-and-issues)). Vanilla 1.20.1 has no such flag per player, Arclight
adds the one of CraftBukkit, and Forge 47 has no event that drops a player from the count of
`SleepStatus`, which is still to verify. A port of the flag would need a mixin in the bridge, or
Golem drops the behaviour as a decision of the owner.

**Elsewhere.** `/list` marks AFK players (`E/PlayerList.java:34-36`), `msg` warns the sender
(`E/messaging/SimpleMessageRecipient.java:105-112`), and `seen` shows the flag.

**Stores.** The key `afk` of the player file, saved on every change (`E/UserData.java:574-581`).
Position, time and message live in memory (`E/User.java:88-90`).

- `userIsAway` (`msg:1503`): `<gray>* {0} <gray>is now AFK.`
- `userIsAwayWithMessage` (`msg:1504`): `<gray>* {0} <gray>is now AFK.`
- `userIsAwaySelf` (`msg:1506`): `<gray>You are now AFK.`
- `userIsAwaySelfWithMessage` (`msg:1507`): `<gray>You are now AFK.`
- `userIsNotAway` (`msg:1505`): `<gray>* {0} <gray>is no longer AFK.`
- `userIsNotAwaySelf` (`msg:1508`): `<gray>You are no longer AFK.`
- `noPermToAFKMessage` (`msg:887`): `<dark_red>You don''t have permission to set an AFK message.`
- `autoAfkKickReason` (`msg:27`): `You have been kicked for idling more than {0} minutes.`
- `playerKicked` (`msg:944`):
  `<primary>Player<secondary> {0} <primary>kicked<secondary> {1}<primary> for<secondary> {2}<primary>.`

**Config.** `auto-afk: 300` (`cfg:489`), `auto-afk-timeout: -1` (`cfg:495`),
`afk-timeout-commands` empty (`cfg:509`), `freeze-afk-players: false` (`cfg:517`),
`disable-item-pickup-while-afk: false` (`cfg:521`), `cancel-afk-on-interact: true` (`cfg:524`),
`cancel-afk-on-move: true` (`cfg:529`), `cancel-afk-on-chat: true` (`cfg:532`),
`cancel-afk-on-fish: true` (`cfg:535`), `sleep-ignores-afk-players: true` (`cfg:540`),
`afk-list-name: "none"` (`cfg:550`), `broadcast-afk-message: true` (`cfg:555`).

**Bridge.** The position of every online player, sampled at least once a second, or a move event.
Events for interaction, attack, bow shots, fishing and every command a player runs. Ticks. An
action that excludes a player from the sleep count, at an AFK change and at join, only if the
owner wants the behaviour of the code rather than the live one (Q37). A kick action for the
timeout. The tab list name only if `afk-list-name` is set or the owner confirms the mark of the
plan.

**Quirks.** Q5, Q21, Q22, Q37.

### suicide

**Names.** Command `suicide` with the alias `esuicide`, usage `/<command>` (`yml:486-489`,
`msg:1277-1278`). It checks `essentials.suicide` (group `default`). Players only.

**Behaviour** (`E/commands/Commandsuicide.java:17-26`).

1. An `EntityDamageEvent` with cause `SUICIDE` and damage `Float.MAX_VALUE` is fired and set as the
   last damage cause. Whether a listener cancels it is not checked.
2. The health is set to 0. CraftBukkit then calls `die` with the generic damage source, so the
   vanilla death message reads `<name> died` (`death.attack.generic`, and CraftBukkit's
   `setHealth`, outside the checkout).
3. The player reads `suicideMessage`, and every player who does not ignore them, themselves
   included, reads `suicideSuccess`, unless the player is vanished.

There is no cooldown.

**After the death.** The death handler of Essentials stores the death position as the `/back`
location of every holder of `essentials.back.ondeath`, which the whole group `default` holds, and
sends `backAfterDeath`, unless `back` is in `disabled-commands`
(`E/EssentialsEntityListener.java:159-173`). The respawn moves the player to their bed or the
world spawn. So `/suicide` and then `/back` make a two-way teleport between any place and the
respawn point, with no warmup and no cooldown (`teleport-cooldown: 0` at `cfg:112`,
`teleport-delay: 0` at `cfg:116`), and the return runs the safety search of the teleport page. The
price is the death drop, which the modpack's Gravestone mod (`gravestone-forge-1.20.1-1.0.35.jar`
in `modrinth.index.json` of the server checkout) keeps in a grave at the death position, so the
player gets back to it with `/back`. See Q35. With `death-messages: true` (`cfg:558`,
`E/Settings.java:1377-1379`) the vanilla death message stays
(`E/EssentialsEntityListener.java:174-176`).

- `suicideMessage` (`msg:1279`): `<primary>Goodbye cruel world...`
- `suicideSuccess` (`msg:1280`): `<primary>Player <secondary>{0} <primary>took their own life.`
- `backAfterDeath` (`msg:32`):
  `<primary>Use the<secondary> /back<primary> command to return to your death point.`

**Bridge.** An action that kills a player with a damage type the port chooses, so that Forge fires
`LivingDeathEvent` and mods see a normal death. The `command` action with `kill <uuid>` would
work, but uses `genericKill` (`<name> was killed`) and shows `[Server: Killed <name>]` to operators
while `broadcast-console-to-ops=true` (`server.properties.example:5`).

**Quirks.** Q5, Q30, Q35.

## Economy

### The economy core

Essentials keeps the money itself: without Vault no other economy layer is selected
(`E/economy/EconomyLayers.java:26-61`), so `getMoney` and `setMoney` use the player file
(`E/User.java:630-663`).

| Rule | Value | Code |
| --- | --- | --- |
| Starting balance | `starting-balance: 0` (`cfg:851`) | `E/Settings.java:326-328`, `E/UserData.java:92-114` |
| Maximum | `max-money: 10000000000000` (`cfg:872`) | `E/Settings.java:1161-1168` |
| Minimum | `min-money: -10000` (`cfg:877`), and a positive value is negated | `E/Settings.java:1170-1181` |
| Clamping on load | A stored value above the maximum or below the minimum is clamped | `E/UserData.java:105-113` |
| Clamping on set | Above the maximum throws `MaxMoneyException`, or clamps when asked not to throw. Below the minimum clamps silently | `E/UserData.java:120-136` |
| Affordability | Balance minus cost at least 0, or at least the minimum with `essentials.eco.loan` (operators) | `E/User.java:353-362` |
| Command costs | None, `command-costs` is empty (`cfg:855-859`) | `E/Settings.java:449-456` |
| Symbol | `currency-symbol: '$'` (`cfg:864`). Only one character that is no digit, else `$` | `E/Settings.java:1066-1080` |
| Symbol position | `currency-symbol-suffix: false` (`cfg:868`) | `E/Settings.java:1083-1085` |
| Format | `#,##0.00` with US symbols and rounding `FLOOR` (`currency-format` and `currency-symbol-format-locale` are commented out, `cfg:915`, `cfg:926`) | `E/Settings.java:1877-1895` |
| Display | A trailing `.00` is cut, and the sign goes before the symbol | `E/utils/NumberUtil.java:68-101` |
| Logging | `economy-log-enabled: false` (`cfg:880`) | `E/Settings.java:1189` |
| Slow economy warning | 25 ms, no key in `config.yml` | `E/Settings.java:1561`, `E/User.java:615-623` |

So 0 shows as `$0`, 1234 as `$1,234`, 1234.5 as `$1,234.50`, 0.999 as `$0.99`, -5 as `-$5`, and
-0.001 as `-$0.01`, since `FLOOR` rounds towards minus infinity.

- `currency` (`msg:191`): `{0}{1}`, the symbol and the amount
- `maxMoney` (`msg:761`):
  `<dark_red>This transaction would exceed the balance limit for this account.`
- `notEnoughMoney` (`msg:897`): `<dark_red>You do not have sufficient funds.`

**Amount input** (`E/utils/NumberUtil.java:150-194`). The text minus everything outside `0-9`, `.`
and `,` is parsed with `NumberFormat` for `Locale.US`, which treats `,` as grouping and stops at
the first character it cannot use. What is removed must be empty or one of `k`, `m`, `b`, `t` in
any case, which multiply by a thousand, million, billion or trillion. Anything else fails. The
parse yields a `long` or a `double`, and the `double` is turned into a decimal through its text.
So `1,5` is 15, `1.000.000` is 1, `5k` is 5000, `-5` and `10%` fail, and
`0.1234567890123456789` becomes `0.12345678901234568`. See Q12 and Q13.

**Stored as** the key `money` of the player file, written as the text of the decimal
(`E/config/holders/UserConfigHolder.java:26-34`,
`E/config/serializers/BigDecimalTypeSerializer.java:21-53`). Every player file gets the key on load
(`E/UserData.java:111-113`).

**Events.** `UserBalanceUpdateEvent` fires before every change and may alter the new balance
(`E/User.java:651-655`). `PreTransactionEvent` and `TransactionEvent` fire around `pay`
(`E/User.java:304-317`). No plugin of the live server listens to them. The port may offer the same
through the GraphQL API (the row Economy of [EssentialsX 2.22.0](../../plan.md#essentialsx-2220)).

### The worth table

`worth.yml` (`R/worth.yml`) maps item names to prices under `worth`. The live file is the sample
(`plugins.md:37`).

1. The name of an item is its Bukkit material name in lower case without underscores, so
   `DIAMOND_PICKAXE` is `diamondpickaxe` (`E/Worth.java:35`).
2. A price is looked up as `worth.<name>.0` when the section has exactly one key, then
   `worth.<name>.*`, then `worth.<name>` (`E/Worth.java:43-59`). The data value step only runs
   before the flattening (`E/Worth.java:37-41`). A missing or negative price means the item cannot
   be sold, and 0 sells it for nothing (`R/worth.yml:3`).
3. The sample has 166 names written for Minecraft before 1.13. Compared with the item and block
   keys of the language file of 1.20.1, 108 of them are names of 1.20.1, and 58 are not and can
   never match: `log`, `wool`, `leaves`, `sapling`, `wood`, `rawfish`, `storageminecart`,
   `goldchestplate`, `mushroomsoup`, `stationarywater`, `workbench`, `woodsword`,
   `poweredminecart`, `fence`, `stonespade`, `claybrick`, `goldrecord`, `redrose`, `grilledpork`,
   `wooddoor`, `goldhelmet`, `redstonetorchon`, `ironspade`, `woodaxe`, `seeds`, `woodplate`,
   `diamondspade`, `irondoorblock`, `goldhoe`, `goldsword`, `boat`, `stoneplate`, `wallsign`,
   `sulphur`, `rails`, `woodhoe`, `goldleggings`, `yellowflower`, `woodstairs`, `bed`, `watch`,
   `doublestep`, `woodpickaxe`, `cookedfish`, `glowingredstoneore`, `woodspade`, `diode`, `soil`,
   `stationarylava`, `goldpickaxe`, `goldboots`, `goldaxe`, `goldspade`, `sugarcaneblock`,
   `greenrecord`, `inksack`, `step` and `sign` (the language file still has a legacy key
   `item.minecraft.sign`, but 1.20.1 has no item `sign`). Mod items have no price. See Q20.

Golem keys the table by the registry id of the item, migrates the names that still exist, and
reports the ones it cannot place, as the [migration of the plan](../../plan.md#data-migration) asks.

### balance

**Names.** Command `balance` with the aliases `bal`, `ebal`, `ebalance`, `money` and `emoney`,
usage `/<command> [player]` (`yml:31-34`, `msg:48-53`). It checks `essentials.balance` (group
`default`), and `essentials.balance.others` (operators) for another player.

**Behaviour** (`E/commands/Commandbalance.java:18-37`).

1. With exactly one argument and the others node, the balance of that known player, online or not,
   as `balanceOther`, with their name when they are vanished and their display name otherwise
   (`E/commands/Commandbalance.java:30`).
2. Else, with fewer than two arguments, the sender's own balance as `balance`. So `/balance Alex`
   without the node shows the sender's own balance. See Q19.
3. Two or more arguments print the usage.
4. The console needs one argument and finds anybody (`E/commands/Commandbalance.java:23`).

- `balance` (`msg:47`): `<green>Balance\:<secondary> {0}`
- `balanceOther` (`msg:54`): `<green>Balance of {0}<green>\:<secondary> {1}`

### balancetop

**Names.** Command `balancetop` with the aliases `ebalancetop`, `baltop` and `ebaltop`, usage
`/<command> [page]` (`yml:35-38`, `msg:57-60`). It checks `essentials.balancetop` (group
`default`), and `essentials.balancetop.force` (operators) rebuilds at once with `force`.

**Behaviour** (`E/commands/Commandbalancetop.java:38-153`, `E/BalanceTopImpl.java:28-77`).

1. A page number is read from the first argument, 0 by default, which shows as page 1.
2. A list younger than two minutes is shown from the cache.
3. Else, with more than 50 known players, `orderBalances` is sent, and a worker thread loads every
   player file, skips NPC accounts (`npcs-in-balance-ranking: false`, `cfg:725`) and players with
   `essentials.balancetop.exclude` (live nobody, and offline players use the stored flag
   `baltop-exempt`), sums the total, and sorts by balance, highest first, without limit
   (`baltop-entry-limit: -1`, `cfg:910`).
4. The text is `serverTotal`, then one `balanceTopLine` per player with rank, name (the display name
   of an online player, the plain name of a vanished one, the last account name of an offline one,
   `E/BalanceTopImpl.java:42-49`) and balance. `show-zero-baltop: true`
   (`cfg:899`) and `baltop-requirements` of 0 balance and 0 seconds of play time
   (`cfg:903-905`) keep everybody. The rank counts players that a filter hides
   (`E/commands/Commandbalancetop.java:139-146`). The play time is read for every ranked player at
   every rebuild, also with the live 0: the statistic `PLAY_ONE_MINUTE` of the online player, or of
   `Bukkit.getOfflinePlayer(uuid)` for an offline one, which loads that player's statistics file
   (*outside the checkout*, CraftBukkit), in ticks divided by 20
   (`E/commands/Commandbalancetop.java:124-137`).
5. It shows as `balanceTop` with the time of the cache in the short date and time format of the
   JVM locale, then the page of nine lines with `infoPages` titled `Balancetop`.

- `balanceTop` (`msg:55`): `<primary>Top balances ({0})`
- `orderBalances` (`msg:913`):
  `<primary>Ordering balances of<secondary> {0} <primary>users, please wait...`
- `serverTotal` (`msg:1143`): `<primary>Server Total\:<secondary> {0}`
- `balanceTopLine` (`msg:56`): `{0}. {1}, {2}`

**Stores.** `baltop-exempt` caches the exclude node of a player
(`E/User.java:1315-1322`, `E/UserData.java:743-750`).

**Golem.** One query over the accounts of the store, ordered by balance, instead of a cache built
from files. `baltop-requirements.minimum-playtime` needs a play time per player from the bridge.
Without one the port documents the key as unsupported. See Q32.

### pay

**Names.** Command `pay` with the alias `epay`, usage `/<command> <player> <amount>`
(`yml:338-341`, `msg:918-921`). Players only.

| Node | Checked at | Live |
| --- | --- | --- |
| `essentials.pay` | `E/Essentials.java:807` | Group `default` |
| `essentials.pay.offline`, to pay offline or vanished players and to use `**` | `E/commands/Commandpay.java:56-66` | Operators |
| `essentials.pay.multiple`, for `*` and `**` | `E/commands/Commandpay.java:61` | Operators |
| `essentials.eco.loan`, to go below 0 down to `min-money` | `E/User.java:358` | Operators |

**Behaviour** (`E/commands/Commandpay.java:27-102`, `E/User.java:298-321`).

1. Fewer than two arguments print the usage. An amount that contains `-` reads
   `payMustBePositive`. An amount without any digit or dot prints the usage.
2. The amount is parsed as in [Amount input](#the-economy-core). Below
   `minimum-pay-amount: 0.001` (`cfg:891`) reads `minimumPayAmount` with the minimum written
   exactly.
3. The target goes through the offline loop: `@s` is the sender, a UUID is taken, `*` and `**`
   need `essentials.pay.multiple`, and anything else is the lookup of `pay` in the table above,
   which takes offline players and partial online names.
4. A target that is offline or hidden from the sender, without `essentials.pay.offline`, reads
   `payOffline`.
5. A target with `accepting-pay` false (set by `paytoggle`, operators only) reads
   `notAcceptingPay`. The ignore list is not checked (`pay-excludes-ignore-list: false`,
   `cfg:894`).
6. Confirmation is off: `default-enabled-confirm-commands` is empty (`cfg:742-744`) and no player
   of the group can turn it on, see [payconfirmtoggle](#payconfirmtoggle). With it on, the first
   `/pay` only stores the amount and sends `confirmPayment`, and the same amount again pays
   (`E/commands/Commandpay.java:72-82`, `E/commands/Commandpay.java:98-101`). Pending
   confirmations live in memory, in a `WeakHashMap`, and every join clears them (`E/User.java:68`,
   `E/EssentialsPlayerListener.java:552`).
7. `payUser` refuses 0 or less with `payMustBePositive`, and an amount the sender cannot afford
   with `notEnoughMoney`. `PreTransactionEvent` may cancel or change the amount. The sender's
   balance is lowered, then the target's raised. The sender reads `moneySentTo` and the target
   `moneyRecievedFrom`, and `TransactionEvent` fires.
8. If the target would pass the maximum, the sender reads `maxMoney` and gets the amount back
   (`E/commands/Commandpay.java:86-91`).
9. Paying oneself is not refused.

- `payMustBePositive` (`msg:926`): `<dark_red>Amount to pay must be positive.`
- `minimumPayAmount` (`msg:771`): `<secondary>The minimum amount you can pay is {0}.`
- `payOffline` (`msg:927`): `<dark_red>You cannot pay offline users.`
- `notAcceptingPay` (`msg:892`): `<dark_red>{0} <dark_red>is not accepting payment.`
- `confirmPayment` (`msg:172`):
  `<gray>To <b>CONFIRM</b><gray> payment of <primary>{0}<gray>, please repeat command\: <primary>{1}`
- `moneySentTo` (`msg:781`): `<green>{0} has been sent to {1}.`
- `moneyRecievedFrom` (`msg:780`): `<green>{0}<primary> has been received from<green> {1}<primary>.`
- `notEnoughMoney` (`msg:897`) and `maxMoney` (`msg:761`): see [The economy core](#the-economy-core)

**Stores.** `money` of both players. `accepting-pay` and `confirm-pay` are read
(`E/UserData.java:707-723`).

**Quirks.** Q1, Q12, Q13, Q15, Q17, Q18.

### payconfirmtoggle

**Names.** Command `payconfirmtoggle` with the aliases `epayconfirmtoggle`, `payconfirmoff`,
`epayconfirmoff`, `payconfirmon`, `epayconfirmon`, `payconfirm` and `epayconfirm`, usage
`/<command>` (`yml:346-349`, `msg:930-931`). It checks `essentials.payconfirmtoggle`, which only
operators hold.

**Behaviour** (`E/commands/Commandpayconfirmtoggle.java:13-23`). A label that contains
`payconfirmon` turns confirmation on, one that contains `payconfirmoff` turns it off, any other
toggles. The choice is stored as `confirm-pay`, pending confirmations are cleared, and the player
reads `payConfirmToggleOn` or `payConfirmToggleOff`.

- `payConfirmToggleOn` (`msg:923`): `<primary>You will now be prompted to confirm payments.`
- `payConfirmToggleOff` (`msg:922`): `<primary>You will no longer be prompted to confirm payments.`

### worth

**Names.** Command `worth` with the aliases `eprice`, `price` and `eworth`, usage
`/<command> <<itemname>|<id>|hand|inventory|blocks> [-][amount]` (`yml:618-621`,
`msg:1619-1628`). It checks `essentials.worth` (group `default`).

**Behaviour for a player** (`E/commands/Commandworth.java:24-56`,
`E/items/AbstractItemDb.java:153-183`, `E/Worth.java:78-132`).

1. The items are the held item for no argument or `hand` (main hand, else off hand), every stack of
   the inventory with the armour slots and the off hand for `inventory`, `invent` or `all`, every
   block stack of the same for `blocks`, or the item of that name from the item database
   `items.json` (`E/items/AbstractItemDb.java:153-183`, the inventory with the armour slots at
   `E/items/AbstractItemDb.java:161` and `168`). Air reads `itemSellAir`.
2. For each distinct item, the amount is the second argument (a leading `-` means all but that
   many, a trailing `s` means stacks), or every such item in the inventory without the four armour
   slots (`E/Worth.java:104`). The off hand counts (`E/Worth.java:103-116`,
   `E/craftbukkit/Inventories.java:232-244`, `453-455`). So worn armour shows in `/worth all` as a
   line with 0 items and a total of `$0`, adds nothing, and still counts as an item for the total
   line (`E/commands/Commandworth.java:29-47`, `86-90`). `/worth diamondchestplate` with the
   chestplate worn reads 0 items the same way. `worth` always asks as a bulk request, so an amount
   above what the player has is priced as given (`E/commands/Commandworth.java:78`,
   `E/Worth.java:121-128`). The price comes from the [worth table](#the-worth-table). No price
   reads `itemCannotBeSold`, which aborts a single item and is skipped in a bulk list.
3. Each item reads `worth`, or `worthMeta` when it has a damage value, with the item name, the
   total, the amount and the price each.
4. With more than one item priced, `totalSellableAll`, or `totalSellableBlocks` for `blocks`.

**From the console**, `worth <item> [amount]` with amount 1 by default
(`E/commands/Commandworth.java:59-64`, `E/commands/Commandworth.java:66-77`).

- `worth` (`msg:1618`):
  `<green>Stack of {0} worth <secondary>{1}<green> ({2} item(s) at {3} each)`
- `worthMeta` (`msg:1629`):
  `<green>Stack of {0} with metadata of {1} worth <secondary>{2}<green> ({3} item(s) at {4} each)`
- `totalSellableAll` (`msg:1353`):
  `<green>The total worth of all sellable items and blocks is <secondary>{1}<green>.`
- `totalSellableBlocks` (`msg:1354`):
  `<green>The total worth of all sellable blocks is <secondary>{1}<green>.`
- `itemCannotBeSold` (`msg:578`): `<dark_red>That item cannot be sold to the server.`
- `itemSellAir` (`msg:617`): `You really tried to sell Air? Put an item in your hand.`
- `itemMustBeStacked` (`msg:601`), only for an item with `trade-in-stacks.<item>: true`, a key
  `config.yml` does not have, so never live (`E/Worth.java:96-101`, `E/Settings.java:1096-1098`):
  `<dark_red>Item must be traded in stacks. A quantity of 2s would be two stacks, etc.`
- `itemNotEnough1` (`msg:612`), only for `sell`, since `worth` asks for the bulk amount
  (`E/Worth.java:121-129`): `<dark_red>You do not have enough of that item to sell.`
- `itemNotEnough2` (`msg:613`), only for `sell`:
  `<primary>If you meant to sell all of your items of that type, use<secondary> /sell itemname<primary>.`
- `itemNotEnough3` (`msg:614`), only for `sell`:
  `<secondary>/sell itemname -1<primary> will sell all but one item, etc.`

**Bridge.** The held items and the inventory of a player, as registry id, count, damage and
whether it is a block. An alias table for item names, or registry ids alone.

**Quirks.** Q20.

### eco

**Names.** Command `eco` with the aliases `eeco`, `economy` and `eeconomy`, usage
`/<command> <give|take|set|reset> <player> <amount>` (`yml:130-133`, `msg:316-325`). It checks
`essentials.eco`, which only operators hold. The console may use it.

**Behaviour** (`E/commands/Commandeco.java:27-82`).

1. Fewer than two arguments print the usage. The first is `give`, `take`, `set` or `reset` in any
   case. `reset` uses `starting-balance`. The others parse the third argument as in
   [Amount input](#the-economy-core). Any failure prints the usage.
2. A third argument that ends with `%` is meant as a percentage of each balance, but the parse of
   such an amount always fails, so the usage follows. See Q16.
3. The target goes through the offline loop with wildcards allowed: `*` is every online player,
   `**` every known player, loaded from their files, and anything else the lookup of `pay`.
4. `give` adds the amount: the target reads `addedToAccount` and the sender
   `addedToOthersAccount` with the new balance (`E/User.java:282-291`). A balance past the maximum
   throws `maxMoney`, which ends the loop where it is. An amount of 0 does nothing and says
   nothing.
5. `take` checks that the result stays at or above `min-money` (-10000), whatever the target's
   permissions, and else shows `minimumBalanceError`. Then the target reads `takenFromAccount` and
   the sender `takenFromOthersAccount` (`E/User.java:333-346`).
6. `set` and `reset` clamp the amount between the minimum and the maximum, set it, and send
   `setBal` to the target and `setBalOthers` to the sender. The clamp at the minimum never acts for
   `set`: the parse cannot yield a negative amount, since `-5` leaves `-` as an unknown suffix and
   prints the usage, and `min-money` is never above 0, because a positive value is negated
   (`E/utils/NumberUtil.java:155-179`, `E/commands/Commandeco.java:44-47`,
   `E/Settings.java:1170-1176`). For `reset` it would act only with a `starting-balance` below
   `min-money`. Live they are 0 and -10000.

- `addedToAccount` (`msg:3`): `<yellow>{0}<green> has been added to your account.`
- `addedToOthersAccount` (`msg:4`):
  `<yellow>{0}<green> added to<yellow> {1}<green> account. New balance\:<yellow> {2}`
- `takenFromAccount` (`msg:1282`): `<yellow>{0}<green> has been taken from your account.`
- `takenFromOthersAccount` (`msg:1283`):
  `<yellow>{0}<green> taken from<yellow> {1}<green> account. New balance\:<yellow> {2}`
- `minimumBalanceError` (`msg:770`): `<dark_red>The minimum balance a user can have is {0}.`
- `setBal` (`msg:1152`): `<green>Your balance was set to {0}.`
- `setBalOthers` (`msg:1153`): `<green>You set {0}<green>''s balance to {1}.`

**Quirks.** Q1, Q14, Q15, Q16, Q17.

## Admin

### gamemode, gm and the direct forms

**Names.** Command `gamemode` with the aliases `adventure`, `eadventure`, `adventuremode`,
`eadventuremode`, `creative`, `ecreative`, `eecreative`, `creativemode`, `ecreativemode`,
`egamemode`, `gm`, `egm`, `gma`, `egma`, `gmc`, `egmc`, `gms`, `egms`, `gmt`, `egmt`, `survival`,
`esurvival`, `survivalmode`, `esurvivalmode`, `gmsp`, `sp`, `egmsp`, `spec` and `spectator`, usage
`/<command> <survival|creative|adventure|spectator> [player]` (`yml:170-173`, `msg:432-435`).

| Node | Checked at | Live |
| --- | --- | --- |
| `essentials.gamemode` | `E/Essentials.java:807` | Operators |
| `essentials.gamemode.all` or `essentials.gamemode.<mode>` | `E/commands/Commandgamemode.java:80-83` | Operators |
| `essentials.gamemode.others` | `E/commands/Commandgamemode.java:38`, `E/commands/Commandgamemode.java:45` | Operators |
| `essentials.gamemode.*`, `default: op`, with the three above as children | `yml:835-843` | Operators |

**The mode of a word** (`E/commands/Commandgamemode.java:85-100`). The word is put in lower case
and tested in this order (`E/commands/Commandgamemode.java:88-96`). Creative is `gmc`, `egmc`, `1`,
`c` or any word that contains `creat`. Survival is `gms`, `egms`, `0`, `s` or any word that
contains `survi`. Adventure is `gma`, `egma`, `2`, `a` or any word that contains `advent`.
Spectator is `gmsp`, `egmsp`, `3`, `sp` or any word that contains `spec`. The toggle is `gmt`,
`egmt`, `t` or any word that contains `toggle` or `cycle`. Anything else is no mode. Only `creat`,
`survi`, `advent`, `spec`, `toggle` and `cycle` are tested as parts of the word. `1`, `c`, `0`,
`s`, `2`, `a`, `3`, `sp` and `t` must be the whole word, in any case. Creative is tested first, so
a word that holds both `creat` and `spec` is creative. The toggle goes from survival to creative,
from creative to adventure, and from anything else to survival
(`E/commands/Commandgamemode.java:53-55`). The toggle works only for the sender: for another player
`matchGameMode` answers null and `setUserGamemode` throws `NotEnoughArgumentsException` with
`gameModeInvalid`, so `/gmt Alex`, `/gm toggle Alex`, `/gm t Alex` and the console's `gmt Alex`
print the usage and then `You need to specify a valid player/mode.`, and nothing changes
(`E/commands/Commandgamemode.java:26-28`, `39`, `46`, `66-69`).

**Behaviour for a player** (`E/commands/Commandgamemode.java:34-64`).

1. No argument: the mode of the label, so `/gmc` is creative and `/gm` or `/gamemode` alone print
   the usage.
2. Two arguments with a second one longer than two characters and the others node: the mode of the
   first argument for the online players the second names, with `*` for all. A second argument of
   one or two characters is ignored, and the first argument sets the sender's own mode.
3. One argument: its mode for the sender, or, if it is no mode and the sender holds the others
   node, the mode of the label for the player it names.
4. Without the node of the mode the sender reads `cantGamemode` with the localised mode, else their
   mode is set and they read `gameMode`.
5. For another player the sender's nodes are checked, `cantGamemode` names the mode in upper
   case, and only the sender reads `gameMode` with the target's display name. The target is not
   told (`E/commands/Commandgamemode.java:66-78`).
6. The console needs a target: `gamemode <mode> <player>` or `<form> <player>`. Three or more
   arguments do nothing and say nothing (`E/commands/Commandgamemode.java:22-31`).

Setting the mode goes through Bukkit's `setGameMode`, which fires `PlayerGameModeChangeEvent`.
With `gamemode-change-preserve-flying: false` (`cfg:245`) Essentials does nothing more
(`E/EssentialsPlayerListener.java:1203-1222`).

- `gameMode` (`msg:430`):
  `<primary>Set game mode<secondary> {0} <primary>for <secondary>{1}<primary>.`
- `cantGamemode` (`msg:125`): `<dark_red>You do not have permission to change to gamemode {0}`
- `gameModeInvalid` (`msg:431`): `<dark_red>You need to specify a valid player/mode.`, after the
  usage, when the toggle is asked for another player
- `survival` (`msg:1281`): `survival`, `creative` (`msg:190`): `creative`, `adventure` (`msg:5`):
  `adventure`, `spectator` (`msg:1262`): `spectator`

**Golem.** The plan makes these thin wrappers over vanilla `gamemode` (the row Admin of
[EssentialsX 2.22.0](../../plan.md#essentialsx-2220)). Vanilla then answers with its own texts,
`Set own game mode to %s` or `Set %s's game mode to %s` to the sender,
`Your game mode has been updated to %s` to the target, and `[<source>: ...]` to operators
(`commands.gamemode.success.self`, `commands.gamemode.success.other`, `gameMode.changed`,
`chat.type.admin` in the language file of 1.20.1). That fixes Q29, but changes the texts operators
know. The port must decide which texts it keeps. The word rules above, the toggle and every alias
stay, so the commands operators type keep working.

**Bridge.** The current game mode of a player, for the toggle, and an action that sets a game
mode, or the `command` action with `gamemode <mode> <name>`.

**Quirks.** Q1, Q2, Q29.

### kick

**Names.** Command `kick` with the alias `ekick`, usage `/<command> <player> [reason]`
(`yml:258-261`, `msg:650-653`). It checks `essentials.kick`. `essentials.kick.exempt` protects a
target (nobody), and `essentials.kick.notify` receives the notice (operators).

**Behaviour** (`E/commands/Commandkick.java:23-59`).

1. No argument prints the usage. The target is an online player by the lookup of `kick`, which
   takes a prefix or a substring of a name.
2. A player sender cannot kick a target hidden from them without `essentials.vanish.interact`
   (`playerNotFound`), nor a target with `essentials.kick.exempt` (`kickExempt`). The console can.
3. The reason is the rest, or `kickDefault`. `\n` and `|` become line breaks and `&` codes become
   `§` codes.
4. `UserKickEvent` may cancel or change the reason, and then the target is kicked with it.
5. The log and every holder of `essentials.kick.notify` get `playerKicked` with the sender's
   display name (or `Console`), the target's name and the reason.

- `kickDefault` (`msg:654`): `Kicked from server.`
- `kickExempt` (`msg:656`): `<dark_red>You cannot kick that person.`
- `playerKicked` (`msg:944`): see [afk](#afk)

**Bridge.** A kick action with a styled reason, or the `command` action with vanilla `kick`, which
writes its own `Kicked %s: %s` and tells operators.

**Quirks.** Q1, Q2, Q33.

### ban

**Names.** Command `ban` with the alias `eban`, usage `/<command> <player> [reason]`
(`yml:39-42`, `msg:61-64`). It checks `essentials.ban`. A player sender needs
`essentials.ban.offline` (operators) to ban offline players, `essentials.ban.exempt` (nobody)
protects a target, and `essentials.ban.notify` (operators) receives the notice.

**Behaviour** (`E/commands/Commandban.java:24-65`).

1. No argument prints the usage. The target is any known player by the lookup of `ban`. If none
   matches, the typed text itself becomes the name to ban (`E/commands/Commandban.java:30-35`).
2. An offline target needs `essentials.ban.offline` from a player sender (`banExemptOffline`). An
   online target with `essentials.ban.exempt` cannot be banned by a player (`banExempt`).
3. The reason is the rest with `\n` and `|` as line breaks and `&` codes as `§` codes, or
   `defaultBanReason`.
4. The name goes into Bukkit's name ban list with the reason, no expiry, and as source the
   sender's display name, or `Console` (`E/commands/Commandban.java:44`,
   `E/commands/Commandban.java:53`).
5. The target is kicked with `banFormat`, the log gets `playerBanned`, the sender reads
   `userUnknown` if the name never joined, and every holder of `essentials.ban.notify` reads
   `playerBanned`.

- `defaultBanReason` (`msg:197`): `The Ban Hammer has spoken\!`
- `banFormat` (`msg:67`): `<secondary>You have been banned\:\n<reset>{0}`
- `banExempt` (`msg:65`): `<dark_red>You cannot ban that player.`
- `banExemptOffline` (`msg:66`): `<dark_red>You may not ban offline players.`
- `playerBanned` (`msg:941`):
  `<primary>Player<secondary> {0} <primary>banned<secondary> {1} <primary>for\: <secondary>{2}<primary>.`
- `userUnknown` (`msg:1514`): see [seen](#seen)

**At login** a banned name or address reads the texts of [Login](#login).

**Bridge.** A ban action that writes the vanilla name ban list with reason and source, and the
ban entry as a query for `seen`. The `command` action with vanilla `ban` loses the source and
needs the name to resolve to a profile.

**Quirks.** Q1, Q2, Q28.

### Commands this part depends on

This page reads the state and the nodes of these admin commands, and its fields come from them.
Each needs a decision: port among the admin commands after the usage audit, or drop with its
fields in the migration.

| Command | Aliases | Usage | Line | State it writes |
| --- | --- | --- | --- | --- |
| `msgtoggle` | `emsgtoggle` | `/<command> [player] [on\|off]` | `yml:314-317` | In memory only, read by `msg` and `r` |
| `rtoggle` | `ertoggle`, `replytoggle`, `ereplytoggle` | `/<command> [player] [on\|off]` | `yml:386-389` | `last-message-reply-recipient` |
| `paytoggle` | `epaytoggle`, `payoff`, `epayoff`, `payon`, `epayon` | `/<command> [player]` | `yml:342-345` | `accepting-pay` |
| `socialspy` | `esocialspy` | `/<command> [player] [on\|off]` | `yml:462-465` | `socialspy` |
| `mute` | `emute`, `silence`, `esilence`, `unmute`, `eunmute` | `/<command> <player> [datediff] [reason]` | `yml:318-321` | `muted`, `mute-reason`, `timestamps.mute` |
| `nick` | `enick`, `nickname`, `enickname` | `/<command> [player] <nickname\|off>` | `yml:326-329` | `nickname` |
| `vanish` | `v`, `ev`, `evanish` | `/<command> [player] [on\|off]` | `yml:590-593` | In memory, and every "hidden" rule of this page rests on it |
| `togglejail` | `jail`, `ejail`, `tjail`, `etjail`, `etogglejail`, `unjail`, `eunjail` | `/<command> <player> <jailname> [datediff]` | `yml:506-509` | `jail`, `jailed`, jail timestamps |

Without `vanish` and `mute` in Golem, every hidden and mute rule of this page falls away, as the
teleport page says for vanish in its section 3.3.

## Listeners without a command

### Login

On a server below 1.21.8 Essentials registers `LoginListenerPre1_21`
(`E/EssentialsPlayerListener.java:171-176`), which is the one Arclight 1.20.1 runs.

- A banned name reads `banJoin` or, with an expiry, `tempbanJoin` instead of the vanilla text, and
  a banned address reads `banIpJoin` or `tempbanIpJoin` (`E/EssentialsPlayerListener.java:636-662`).
  This belongs to [ban](#ban).
- On a full server a holder of `essentials.joinfullserver` (no default, so operators,
  `yml:973-974`) is let in. Everybody else reads `serverFull`, since
  `use-custom-server-full-message: true` (`cfg:610`, `E/EssentialsPlayerListener.java:663-676`).
- The whitelist text `whitelistKick` and `essentials.whitelist.bypass` live only in the listener
  for Paper 1.21.8 and later (`E/EssentialsPlayerListener.java:716-724`), so they are not live, and
  `use-custom-whitelist-message: true` (`cfg:614`) does nothing on the live server.

| Key | Line | Text |
| --- | --- | --- |
| `banJoin` | `msg:70` | `You are banned from this server. Reason\: {0}` |
| `tempbanJoin` | `msg:1312` | `You are banned from this server for {0}. Reason\: {1}` |
| `banIpJoin` | `msg:68` | `Your IP address is banned from this server. Reason\: {0}` |
| `tempbanIpJoin` | `msg:69` | `Your IP address is temporarily banned from this server for {0}. Reason\: {1}` |
| `serverFull` | `msg:1140` | `Server is full\!` |
| `whitelistKick` | `msg:1141` | `Whitelist enabled\!` (not live) |

The bridge has no login event. Keeping these texts needs one that can allow a login and set the
reason of a refusal. Dropping them keeps the vanilla texts and the vanilla rule that an operator
does not pass a full server unless their entry in `ops.json` says so (vanilla, outside the
checkout).

### Join and quit messages

`allow-silent-join-quit: false` (`cfg:576`), `custom-join-message: "none"` (`cfg:589`),
`custom-quit-message: "none"` (`cfg:590`), `custom-new-username-message: "none"` (`cfg:606`) and
`hide-join-quit-messages-above: -1` (`cfg:619`) leave the vanilla join and quit messages untouched
(`E/EssentialsPlayerListener.java:297-310`, `452-479`). The port needs nothing while they stay at
their defaults. Supporting them needs a bridge that can replace or hide the vanilla message.

### Other listeners

No page ports these, and the plan does not name them.

- `login-attack-delay: 5` (`cfg:659`): for 5 seconds after their login the hits of a player on
  another player are cancelled unless the player holds `essentials.pvpdelay.exempt` (operators)
  (`E/EssentialsEntityListener.java:77-80`). It needs a cancellable attack event.
- `milk-bucket-easter-egg: true` (`cfg:694-695`, `E/Settings.java:1724-1726`): a player who hits an
  adult `Ageable` with a milk bucket in the main hand turns it into a baby, gets an empty bucket
  and the hit is cancelled (`E/EssentialsEntityListener.java:52-61`). No node is checked. See Q36.
- `death-messages: true` (`cfg:558`, `E/Settings.java:1377-1379`): the vanilla death message of
  `/suicide` and every other death stays (`E/EssentialsEntityListener.java:174-176`).
- On join, holders of `essentials.updatecheck` (operators) get the version notice of the update
  check (`E/EssentialsPlayerListener.java:503-509`), which the plan does not port
  ([Telemetry](../../plan.md#telemetry)).

## The chat side

EssentialsX formats no chat in the live configuration.

- The formats of the `chat` section (`cfg:963-1042`) belong to EssentialsChat, which the section
  itself says must be installed (`cfg:960-961`) and which the server does not run
  (`plugins.md:16-22`). The core reads only `chat.radius`, for `me` (`E/Settings.java:225-232`), and
  the shout settings, which only EssentialsChat uses (`E/Settings.java:258-270`).
- The core listener of `AsyncPlayerChatEvent` runs at `LOWEST`
  (`E/EssentialsPlayerListener.java:191-232`). It cancels the message of a muted player and tells
  them, removes from the recipients every player who ignores the sender, ends the sender's AFK on
  the server thread, and refreshes the sender's display name. It does not touch the format.
- The line therefore keeps the server's own format. CraftBukkit's default is `<%1$s> %2$s` with the
  display name in the first slot (CraftBukkit, outside the checkout). Since Essentials gives
  operators the display name `§4<name>§r`, operators may show in dark red there. How Arclight
  renders it is not verified.
- No prefix or suffix joins the name, since EssentialsChat is absent and Vault too
  (`E/Settings.java:1247-1263`, `E/perm/impl/SuperpermsHandler.java:141-148`).

**Ignore in chat.** Essentials removes the ignoring players from the recipient set, and logs that
it cannot when another plugin makes the set unchangeable
(`E/EssentialsPlayerListener.java:214-228`). The bridge's `chat` call can only cancel a message
([The host class](../../bridge.md#the-host-class)), so `golem-essentials` has two ways.

1. The verdict of `chat` names the players who must not get the message, and the mod delivers the
   signed message to every other player itself. This keeps chat as it is for the clients and needs
   a change to the bridge.
2. The plugin cancels the message and sends `<name> message` as a system message to every other
   player. This needs no change to the bridge, but the message loses its signature and arrives up
   to a tick later. `enforce-secure-profile=false` (`server.properties.example:13`) means the
   server does not require signatures today.

The first way is the one this page recommends, see Q31.

**Formatting codes in chat.** Golem passes the raw text of a player through and never turns `&`
codes into formatting. The vanilla server refuses `§` in chat and commands (vanilla, outside the
checkout), so a player cannot fake formatting there.

## Storage

Essentials keeps one YAML file per player, `userdata/<uuid>.yml`, written through Configurate with
names in lower case with dashes (`E/UserData.java:44-53`,
`E/config/EssentialsConfiguration.java:55-66`, and the dashes show in
`E/EssentialsUpgrade.java:264-276`). These keys belong to this part.

| Key | Type | Written by | Holder |
| --- | --- | --- | --- |
| `money` | Decimal as text | Every balance change, and on load | `E/config/holders/UserConfigHolder.java:26-34` |
| `mail` | List of mail entries, see [mail](#mail) | `mail`, `mail read`, `mail clear` | `E/config/holders/UserConfigHolder.java:116-128` |
| `ignore` | List of UUIDs | `ignore` | `E/config/holders/UserConfigHolder.java:150-162` |
| `afk` | Boolean | Every AFK change | `E/config/holders/UserConfigHolder.java:224-232` |
| `accepting-pay` | Boolean, default true | `paytoggle` | `E/config/holders/UserConfigHolder.java:295-303` |
| `confirm-pay` | Boolean or absent | `payconfirmtoggle` | `E/config/holders/UserConfigHolder.java:305-313` |
| `last-message-reply-recipient` | Boolean or absent | `rtoggle` | `E/config/holders/UserConfigHolder.java:325-333` |
| `baltop-exempt` | Boolean | `balancetop` | `E/config/holders/UserConfigHolder.java:335-343` |
| `socialspy` | Boolean | `socialspy` | `E/config/holders/UserConfigHolder.java:245-253` |
| `nickname` | Text or absent | `nick` | `E/config/holders/UserConfigHolder.java:50-58` |
| `last-account-name` | Text | Join | `E/config/holders/UserConfigHolder.java:265-273`, `E/UserData.java:614-622` |
| `past-usernames` | List of texts, newest first | Join, on a name change | `E/config/holders/UserConfigHolder.java:355-367` |
| `ip-address` | Text | Join | `E/config/holders/UserConfigHolder.java:214-222`, `E/UserData.java:553-559` |
| `timestamps.login`, `timestamps.logout` | Milliseconds | Join, quit, and the timer for crashes | `E/config/holders/UserConfigHolder.java:427-445`, `E/EssentialsTimer.java:90-96` |
| `logoutlocation` | `world` (UUID), `world-name`, `x`, `y`, `z`, `yaw`, `pitch` | Quit | `E/config/holders/UserConfigHolder.java:93-104`, `E/config/serializers/LocationTypeSerializer.java:33-46` |
| `muted`, `mute-reason`, `timestamps.mute` | Read by `seen` and the mute checks | `mute` | `E/config/holders/UserConfigHolder.java:184-202`, `E/config/holders/UserConfigHolder.java:397-405` |
| `npc` | Boolean | Economy API | `E/config/holders/UserConfigHolder.java:255-263` |

The user map (`usermap.bin`, `uuids.bin`, `plugins.md:37`) maps sanitised names to UUIDs
(`E/userstorage/ModernUUIDCache.java:70-104`). The Golem store needs the same index for offline
lookups, as one table of players with UUID, last name and past names.

**Keys that no page maps.** The migration of the plan fails on any entry it cannot place
([Data migration](../../plan.md#data-migration)), so it needs a target or an explicit drop for every
key of `UserConfigHolder`. Neither this page nor the teleport page maps these:

| Key | Holder line | What it is |
| --- | --- | --- |
| `unlimited` | `E/config/holders/UserConfigHolder.java:61` | Materials of `/unlimited` |
| `powertools`, `powertoolsenabled` | `:71`, `:285` | Commands bound to items, `/powertool` |
| `jail`, `jailed`, `timestamps.jail`, `timestamps.onlinejail` | `:106`, `:204`, `:407`, `:417` | Jail, and this page only reads `jailed` in `seen` |
| `godmode` | `:164` | `/god` |
| `flymode` | `:174` | Fly state that the join restores for holders of `essentials.fly` |
| `geolocation` | `:235` | EssentialsGeoIP, not installed, and this page only reads it in `seen` |
| `npc-name` | `:275` | Economy API accounts |
| `confirm-clear` | `:315` | `/clearinventoryconfirmtoggle` |
| `shouting` | `:345` | EssentialsChat, not installed |
| `timestamps.lastheal` | `:387` | `/heal` cooldown |
| `timestamps.kits` | `:448` | Kit delays, and the plan ports no kits |
| `timestamps.command-cooldowns` | `:463` | `command-cooldowns`, empty |

The state of `msgtoggle` is not in the file at all. It lives in memory (`E/User.java:83`,
`1019-1026`) and is lost on a restart.

**Migration.** The [migration tool of the plan](../../plan.md#data-migration) reads these keys. It
turns `money` into the whole cents of the Golem accounts by the rules of
[Balances](../../store.md#balances): a balance with a fraction of a cent rounds half to even and the
report names the account, and a balance below zero is refused with its debt, see issue
[#106](https://github.com/skytekx/golem/issues/106). It keeps mails with their read and expiry
flags, keeps ignore lists, and drops `afk`, which is runtime state.

## What the port needs from the bridge

The bridge today has the events of the start and the stop of the server, join, quit, chat (cancel
only), death, respawn and tick, the command call, the permission handler of Forge, the questions of
the game that the column Today of [Queries](#queries) names, and the actions `command`, `message`,
`broadcast`, `teleport` and `listName`, where `message` and `broadcast` send plain literal text
([The host class](../../bridge.md#the-host-class), [The game](../../bridge.md#the-game),
[Permissions](../../bridge.md#permissions), [Actions](../../bridge.md#actions), and
`bridge/mod/src/main/java/com/skytekx/golem/bridge/Actions.java:66-74`, `93` and `100` in the Golem
tree). This part needs more.

### Queries

A command handler runs on the server thread, so the game can answer these while it runs.

| Query | Used by | Today |
| --- | --- | --- |
| Online players with UUID, name and the operator flag | Name matching, `list`, the operator colour, `golem-permissions` | `players()`, with the operator level of the profile ([The game](../../bridge.md#the-game)) |
| The maximum player count | `list` | None |
| Position, rotation and dimension of a player | `getpos`, `compass`, `depth`, AFK on move, logout position | `location(uuid)`, for an online player ([The game](../../bridge.md#the-game)) |
| Sea level and day time of a dimension | `depth`, `{WORLDTIME12}` | None, since `heights(dimension)` answers the lowest y of a block and the y above the highest alone ([The game](../../bridge.md#the-game)) |
| Game mode of a player | `gmt` | `players()` ([The game](../../bridge.md#the-game)) |
| Held items and inventory, as registry id, count, damage, block flag | `worth` | `mainHand(uuid)`, with the registry id and the count of the main hand alone ([The game](../../bridge.md#the-game)) |
| Play time of a player, online or offline | `balancetop` with `baltop-requirements.minimum-playtime`, 0 live | None |
| The name ban entry with reason, source and expiry, and the whitelist flag | `seen`, `ban` | None |
| The remote address at login | `seen` for operators, if the port keeps it | None |

### Actions

| Action | Used by | Today |
| --- | --- | --- |
| A styled message to one player, with colours and plain text for player content | Every command | `message` sends a plain literal, and colours need a component action or legacy codes |
| Kick with a reason | `kick`, the AFK timeout | `command kick ...` |
| Ban a name with reason and source | `ban` | `command ban ...` loses the source |
| Set the game mode | `gamemode` | `command gamemode ...` |
| Kill a player with a chosen damage type | `suicide` | `command kill ...` gives `was killed` and an operator notice |
| Mark a player as ignored for the sleep count | AFK and join, only if the owner wants the behaviour of the code rather than the live one, see Q37 | None, and Forge 47 has no event for it, so it needs a mixin |
| Deliver a chat message to all but some players | Ignore in chat | None, see [The chat side](#the-chat-side) |
| Tab list name | `afk-list-name`, off by default, and the mark for AFK players that the plan asks for (the row Status of [EssentialsX 2.22.0](../../plan.md#essentialsx-2220)) | `listName` ([Actions](../../bridge.md#actions)) |

### Events

| Event | Used by | Today |
| --- | --- | --- |
| Login, which can allow a login or set the reason of a refusal | The texts of [Login](#login) | None |
| Join, with the first join flag | MOTD, mail notice, `seen`, user index | `playerJoined(uuid, name)` |
| Join and quit messages that can be replaced or hidden | The keys of [Join and quit messages](#join-and-quit-messages), only away from their defaults | None |
| Quit, with the position | `seen` | `playerLeft(uuid, name)` without the position |
| Chat with recipients | Mute, ignore, AFK | `chat`, cancel only, on a worker thread |
| Every command a player runs, vanilla and mod commands too | AFK activity | Only the commands of the plugins |
| Move by a block, or a position poll | AFK on move | A poll with `location(uuid)`, asked on `tick` ([The host class](../../bridge.md#the-host-class), [The game](../../bridge.md#the-game)) |
| Interact, attack, bow shot, fishing, with a cancellable attack | AFK activity and `login-attack-delay` | No event and no cancellable attack. The time since the last action in `players()`, which the game resets when the player moves, chats, uses an item or clicks, shows activity without one ([The game](../../bridge.md#the-game)) |
| Tick | The AFK timer every 50 ticks | `tick` |

### The command tree

- Vanilla has the commands `msg`, `tell`, `w`, `me`, `list`, `kick`, `ban`, `gamemode` and `help`.
  The bridge registers a plugin name with `dispatcher.register`
  (`bridge/mod/src/main/java/com/skytekx/golem/bridge/PluginCommands.java:47-61` in the Golem tree),
  and Brigadier merges a literal of the same name into the existing node: the new command replaces
  the executable of the literal alone, while the vanilla children stay and are tried first, literal
  children before arguments (Brigadier 1.1.8 `CommandNode.addChild` and
  `CommandNode.getRelevantNodes`, outside the checkout). So `/gamemode creative Alex` and
  `/msg Alex hi` would still run vanilla. The bridge must remove the vanilla node before it
  registers the plugin's, for these nine names. See Q2.
- Essentials hides from each player the commands they may not use when the client gets its command
  list (`E/EssentialsPlayerListener.java:1270-1308`). The bridge registers every plugin command
  without a requirement, since a command of a plugin needs no permission level in Brigadier
  ([Commands](../../bridge.md#commands),
  `bridge/mod/src/main/java/com/skytekx/golem/bridge/PluginCommands.java:71-80` in the Golem tree),
  so every player would see every command in completion. The bridge needs a requirement that asks
  `golem-permissions`, and must resend the command tree when permissions change.

### Storage

The [store](../../store.md) of the plugins holds the players with their last name, the times of
their joins and the place of their last quit, the accounts, the mails and the ignore lists, all
keyed by UUID. The past names, the expiry of a mail and the per player toggles of the table above
have no column there, so the port adds them to the schema or drops them in the migration.

## Teleports

No command of this part teleports a player itself, so teleport safety, `teleport-delay`,
`teleport-cooldown` and unloaded destinations do not apply to its commands. One path leads into the
teleport page: `/suicide` sets the `/back` location of every holder of `essentials.back.ondeath`,
and `/back` then returns the player to the death position with the safety search of the teleport
page, see [suicide](#suicide) and Q35.

## Quirks and security

Each line is an issue for `skytekx/golem` with the label `quirk` or `security`, as
[Quirks and security](../../plan.md#quirks-and-security) of the plan asks. Security issues that work
on the live server go to a private advisory first.

| No. | Kind | Old behaviour, with evidence | Golem |
| --- | --- | --- | --- |
| Q1 | security | Held back in a private security advisory while EssentialsX runs on the live server. | Exact name, case insensitive, or UUID. For online targets a unique prefix may stand, and an ambiguous one lists the candidates and does nothing. Money, mail, ignore lists, bans and kicks never take a partial name |
| Q2 | quirk | Nine names are also vanilla commands, and the bridge merges them into the vanilla nodes, so vanilla keeps most of them (`bridge/mod/src/main/java/com/skytekx/golem/bridge/PluginCommands.java:55`, and Brigadier `CommandNode.addChild`) | The bridge replaces the vanilla node for every name a plugin owns |
| Q3 | quirk | A `msg` to a player who ignores the sender shows the sender nothing, not even their echo, which tells them they are ignored, while `mail` pretends success (`E/messaging/SimpleMessageRecipient.java:102-103`, `E/commands/Commandmail.java:91-104`) | Both show the sender the normal echo and deliver nothing |
| Q4 | quirk | With `chat.radius: 0`, `me` is broadcast without a sender, so ignore lists and vanish do not apply (`E/commands/Commandme.java:44-46`, `E/Essentials.java:1189-1192`, `cfg:970`) | `me` skips players who ignore the sender |
| Q5 | security | `msg`, `r`, `me`, `helpop`, `afk` and `suicide` have no rate limit, and `afk` and `suicide` broadcast to every player each time (`E/commands/Commandafk.java:96-99`, `E/commands/Commandsuicide.java:25`, `E/commands/Commandhelpop.java:43-62`) | A limit per sender and command, and AFK and suicide broadcasts at most once per interval per player |
| Q6 | security | Held back in a private security advisory while EssentialsX runs on the live server. | Limits per sender and per recipient, and a cap on stored mails per player |
| Q7 | quirk | `mail read` marks every mail read, also those on pages the player did not see (`E/commands/Commandmail.java:42-52`, `E/commands/Commandmail.java:59-63`) | Only the shown mails are marked read |
| Q8 | quirk | `mail clear <n>` counts expired mails that `mail read` no longer shows, so the number can hit the wrong mail (`E/commands/Commandmail.java:186-198`, `E/commands/Commandmail.java:45-47`) | Expired mails are dropped before any numbering |
| Q9 | quirk | `helpop` does not check mute, so a muted player still reaches all operators (`E/commands/Commandhelpop.java:25-63`) | Mute applies to `helpop`, or the port records the choice to keep it open |
| Q10 | quirk | Without `.url`, a dot in `msg`, `mail` and `me` with two characters before it and two letters after it becomes a space, so `file.txt` reads `file txt` while `1.5` and `a.b` stay ([Formatting codes](#formatting-codes-in-player-text), `E/utils/FormatUtil.java:79-81`, `E/utils/FormatUtil.java:297-306`). `ping <text>` echoes `&` codes as colours to the sender (`E/commands/Commandping.java:18`) | Links are sent as plain text without click action instead of being rewritten, and `ping` echoes plain text |
| Q11 | quirk | Numbers go through `MessageFormat` with the JVM locale, so `getpos` shows `X: 1,234`, which cannot be pasted into a command and changes with the locale (`E/I18n.java:186-193`, `E/commands/Commandgetpos.java:37-41`, `msg:974`) | Coordinates as plain integers, and money with the configured format only |
| Q12 | security | Amounts are parsed with US grouping, so `1,5` pays 15 and `1.000.000` pays 1, and `k`, `m`, `b`, `t` multiply silently (`E/utils/NumberUtil.java:150-190`) | Only plain decimals with at most the currency scale, while `,` and suffixes are refused with a clear error |
| Q13 | quirk | `minimum-pay-amount: 0.001` allows tenths of a cent while every display floors to cents, so `/pay Alex 0.005` reads `$0 has been sent to Alex`, and long fractions pass through a double (`cfg:891`, `E/Settings.java:1889-1890`, `E/utils/NumberUtil.java:68-74`, `E/utils/NumberUtil.java:183-184`) | Integer amounts in the smallest unit, a minimum of one unit, and the display rounds the way it stores |
| Q14 | quirk | `eco take` can push a player without `essentials.eco.loan` below 0, down to -10000, though the config says negative balances need that node. `eco set` cannot, since its amount is never negative (`E/commands/Commandeco.java:62`, `E/utils/NumberUtil.java:155-179`, `cfg:874-877`) | Accounts without the loan node stay at 0 or above, and `eco take` past it fails with a message |
| Q15 | quirk | `pay` lowers the payer before it raises the payee and repairs that after `maxMoney`, ignoring a failed repair. `eco give *` stops at the first account past the maximum after paying the ones before (`E/User.java:312-313`, `E/commands/Commandpay.java:86-91`, `E/commands/Commandeco.java:57-59`) | Every transfer and every bulk change is one store transaction that checks all limits first |
| Q16 | quirk | `eco` percentages never work: the parse rejects the `%` that marks them (`E/commands/Commandeco.java:37-47`, `E/utils/NumberUtil.java:158-180`) | Either a working percentage or no percentage, decided in the port |
| Q17 | quirk | `eco give` and `eco take` of 0 do nothing and say nothing. `pay` to oneself is allowed and shows both messages (`E/User.java:283-285`, `E/User.java:334-336`, `E/commands/Commandpay.java:61-97`) | 0 and self payments are refused with a message |
| Q18 | quirk | Pay confirmation compares amounts with their scale, so `10` and `10.0` differ, and no player of the group can turn confirmation on (`E/commands/Commandpay.java:72`, `cfg:742-744`) | Amounts compare by value, and whether players get confirmation is a config choice |
| Q19 | quirk | `/balance <player>` without `essentials.balance.others` shows the sender's own balance as if it were the answer (`E/commands/Commandbalance.java:29-34`) | It answers that the node is missing |
| Q20 | quirk | 58 of the 166 names of the sample `worth.yml` are names from before 1.13 and never match, so logs, wool, golden and wooden tools cannot be priced, and mod items never can (`E/Worth.java:35`, `R/worth.yml:6-186`) | The table is keyed by registry id and the migration reports every name it cannot map |
| Q21 | quirk | Automatic AFK needs `essentials.afk.auto`, which is `default: false` and not in the group, so it never fires live although `auto-afk: 300` and `player-commands` suggest it does (`yml:635-637`, `E/User.java:906`, `cfg:271`, `cfg:489`), while the row Status of [EssentialsX 2.22.0](../../plan.md#essentialsx-2220) asks for automatic AFK | The port keeps it off by default, or the owner decides to turn it on as a known change |
| Q22 | quirk | `userIsAwayWithMessage` has no `{1}`, so the AFK message never shows in the broadcast (`msg:1504`, `E/commands/Commandafk.java:98`) | The message shows, for players allowed to set one |
| Q23 | security | Held back in a private security advisory while EssentialsX runs on the live server. | Mute reasons, jail and ban details only for staff nodes |
| Q24 | quirk | `seen` lists past names of an online player when there is one, of an offline player only when there are two (`E/commands/Commandseen.java:108`, `E/commands/Commandseen.java:148`) | The same rule for both |
| Q25 | quirk | The first `/rules` on a server without `rules.txt` writes the file and shows an error, and the live folder has none (`E/textreader/TextInput.java:97-107`, `plugins.md:37`) | The port ships the text and never shows a file error to a player |
| Q26 | quirk | Without Vault every group is `default`, so the `Admins` merge of `list` never applies, and the `connectedPlayers` branch is dead because the handler name never equals `ConfigPermissions` (`E/perm/PermissionsHandler.java:39-48`, `E/perm/PermissionsHandler.java:238-240`, `E/PlayerList.java:211-213`) | `list` groups by the primary group of `golem-permissions` |
| Q27 | quirk | `ops-name-color: '4'` paints operators dark red in every message and in the ban source (`cfg:30`, `E/User.java:528-540`, `E/commands/Commandban.java:44`) | A choice of the owner, and the colour never goes into stored data |
| Q28 | security | `ban` with an unknown name bans the typed text as a name, records the display name with colour codes as source, and checks `ban.exempt` only for online targets (`E/commands/Commandban.java:30-44`, `E/commands/Commandban.java:53`) | Bans need a known player or an explicit flag, and the source is the plain name or UUID |
| Q29 | security | `gamemode` on another player tells only the sender, reports nothing to other operators, takes words that only contain a mode, and ignores a target of up to two characters, changing the sender's own mode instead. The toggle forms fail for another player with the usage and an invalid mode error (`E/commands/Commandgamemode.java:38`, `E/commands/Commandgamemode.java:66-78`, `E/commands/Commandgamemode.java:85-100`) | Vanilla `gamemode` with its notices, and a target that is given is never ignored |
| Q30 | quirk | `suicide` ignores a cancelled damage event, so protections cannot stop it, and each use prints a death message and a broadcast (`E/commands/Commandsuicide.java:20-25`) | The kill goes through the vanilla damage path that mods can cancel, with a limit per player |
| Q31 | quirk | Chat ignore needs a changeable recipient set and fails with a log line otherwise, while Golem's chat call can only cancel (`E/EssentialsPlayerListener.java:214-228`, [The host class](../../bridge.md#the-host-class)) | A recipient filter in the bridge, see [The chat side](#the-chat-side) |
| Q32 | quirk | `seen`, `balancetop`, `mail sendall`, `mail clearall` and `eco **` load every player file, on worker threads that also send messages (`E/commands/Commandseen.java:64-86`, `E/BalanceTopImpl.java:28-67`, `E/commands/Commandmail.java:293-313`, `E/commands/Commandmail.java:387-397`) | Queries of the store, with messages sent through actions on the server thread |
| Q33 | quirk | A kick or ban reason cannot hold `\|`, which becomes a line break (`E/commands/Commandkick.java:42`, `E/commands/Commandban.java:48`) | Only `\n` breaks a line |
| Q34 | quirk | `help` lists only Bukkit plugin commands, none of vanilla or the mods (`E/textreader/HelpInput.java:34-41`) | `help` lists the Golem commands the player may use, and the client's command tree hides the others |
| Q35 | quirk | `/suicide` and then `/back` make a free two-way teleport between any place and the respawn point, with no warmup and no cooldown, and with a gravestone mod the items wait at the death position (`E/commands/Commandsuicide.java:17-26`, `E/EssentialsEntityListener.java:169-172`, `cfg:112`, `cfg:116`) | The owner decides whether a death by `/suicide` sets the back location |
| Q36 | security | Any player can turn an adult animal or villager of another player into a baby with a milk bucket, which no protection node checks, and a baby villager cannot trade (`E/EssentialsEntityListener.java:52-61`, `cfg:694-695`, and for the trade rule vanilla, outside the checkout) | Not ported |
| Q37 | quirk | Operators never count for sleeping, from their join on, because `essentials.sleepingignored` has no default and goes to them (`E/EssentialsPlayerListener.java:448-449`, `yml:1243-1244`) | Not ported, or a node that an operator sets on purpose |

### Classes the plan names

- Formatting codes that fake system messages: player text is escaped for MiniMessage
  (`E/I18n.java:186-191`) and plain for the group `default`, so no default player can fake a
  system line today. The risks left are Q10 and Q27. Golem must keep player text out of every
  markup parser.
- Spam through `msg` and `mail`: Q5 and Q6.
- Money that goes negative or rounds wrong: Q12 to Q17.
- Teleports into unsafe or unloaded places: no command of this part teleports, but `/suicide` and
  then `/back` lead into the safety search of the teleport page, see Q35.

## Open points

- The JVM locale of the Mac mini decides the language and the number format of every message
  (`E/I18n.java:50-53`). The boot line `Using locale ...` (`E/I18n.java:218`) of the live log
  answers it.
- Whether Arclight shows the operator colour in chat lines is not verified.
- Whether `rules.txt` exists on the Mac mini, which the first `/rules` creates.
- Whether Forge 47 has an event that drops a player from the count of `SleepStatus`, see
  [afk](#afk) and Q37.
- The usage audit of [Before the port](../../plan.md#before-the-port) decides whether the economy
  and `eco` are used at all, an [open decision](../../plan.md#open-decisions) of the plan.

Related: [Plan](../../plan.md), [Bridge](../../bridge.md), [Store](../../store.md),
[EssentialsX teleport port](teleport.md), [EssentialsX bridge needs](bridge-needs.md) and the page
[Plugins](https://github.com/skytekx/skytekx3-1201-server/blob/3fa5ae681d12233fe81003fbfe5a2d3c529747fc/okf/modpack/plugins.md) of the wiki of the server.

## Sources

The page is checked against the checkout of EssentialsX 2.22.0 at commit
`100f575293afff1c2bc8e1468ce1cffc153319fc` on 2026-11-06, and against `plugins.md` of
[How to read this page](#how-to-read-this-page) and the pages [Plan](../../plan.md) and
[Bridge](../../bridge.md) and the bridge mod of Golem at commit `f630fb1` on 2026-11-07. Every
command, alias and usage line of the page matches `plugin.yml`, and the message keys that the
commands of this part send are quoted with their texts. A fact outside the checkout says so where it
stands and rests on one of these sources.

- The Spigot API 1.20.1 sources, `spigot-api-1.20.1-R0.1-20230921.163938-66-sources.jar`, for the
  Bukkit interfaces such as `Server.getPlayer` and `Server.matchPlayer`
  (`org/bukkit/Server.java:518-549`).
- The Arclight jar of the live server, `arclight-forge-1.20.1-1.0.6-6de9fec-safeiter2.jar`, which
  `arclight.jar` of the server repository at commit `3fa5ae68` points to. It holds
  CraftBukkit as Arclight 1.0.6 runs it, with the recipient set of chat (Q31) and the sleep flag
  (Q37), and the Minecraft 1.20.1 server classes, with `detectRateSpam` (Q5).
- Forge 1.20.1-47.4.18 (`forge-1.20.1-47.4.18-universal.jar`), the Forge of the live server, for
  the events of Forge 47.
- Brigadier 1.1.8 (`brigadier-1.1.8.jar`), for `CommandNode.addChild` and
  `CommandNode.getRelevantNodes`.
- The language file `en_us.json` of Minecraft 1.20.1, for the vanilla texts of `gamemode`, `kick`,
  `kill` and the death messages, and for the item and block keys that the names of `worth.yml` are
  compared with.
- `modrinth.index.json` of the server checkout, for the Gravestone mod
  `gravestone-forge-1.20.1-1.0.35.jar`.
- Java 21, for `java.text.MessageFormat` and the run of the URL pattern of
  [Formatting codes in player text](#formatting-codes-in-player-text).

These citations are read against the checkout and hold.

| Citation | Claim of the page | Verdict |
| --- | --- | --- |
| `E/commands/Commandmail.java:25-26`, `91-99` | One static counter for the whole server, reset a minute after its start | Holds |
| `E/User.java:1274-1283` | `youHaveNewMail` at most once per `notify-player-of-mail-cooldown` | Holds |
| `cfg:166`, `cfg:681`, `cfg:687`, `cfg:692` | 60, true, 180, false | Hold |
| `E/commands/Commandgamemode.java:85-100` | The words of a mode | Holds |
| `E/utils/NumberUtil.java:150-194` | US parse, the suffixes `k`, `m`, `b`, `t`, a double through its text | Holds |
| `msg:1503-1508` | The AFK texts, `userIsAwayWithMessage` without `{1}` | Holds |
| `E/commands/Commandseen.java:120-133` | AFK, jail and mute lines for every holder of `essentials.seen` | Holds |
| `E/commands/Commandbalance.java:18-37` | `/balance <name>` without the others node shows the own balance | Holds |
| `E/Essentials.java:432-433` | The timer starts after 1000 ticks and runs every 50 | Holds |
| `E/commands/Commandme.java:43-48` | With radius 0, `broadcastTl` without a sender | Holds |
| `E/Settings.java:1877-1895` | `#,##0.00`, US symbols, rounding `FLOOR` | Holds |
| `E/commands/Commandignore.java:16-43` | The toggle and its order of checks | Holds |
| `E/commands/Commandhelpop.java:25-63` | No mute check, the sender reads their own line | Holds |
| `E/commands/Commandgetpos.java:17-45` | Block coordinates, yaw as `(yaw + 360) % 360` | Holds |
| `E/commands/Commandkick.java:41-42` | `\n` and `\|` become line breaks | Holds |
| `E/utils/NumberUtil.java:155-179`, `E/commands/Commandeco.java:44-47`, `E/Settings.java:1170-1176` | `eco set` never goes below 0, and a positive `min-money` is negated | Holds |
| `E/commands/Commandgamemode.java:22-31`, `26-28`, `39`, `46`, `53-55`, `66-69` | The toggle fails for another player, and the console with three arguments does nothing | Holds |
| `E/commands/Commandseen.java:116-118`, `145-157`, `159-161`, `E/utils/DateUtil.java:176-180` | The first login, UUID and whitelist lines of `seen` | Holds |
| `E/commands/Commandseen.java:37`, `49`, `93-94`, `203-230` | The label `alts` | Holds |
| `E/messaging/SimpleMessageRecipient.java:40`, `118-123`, `163`, `169`, `181`, `187` | The console, the refusals, social spy and `lastMessageMs` of `msg` | Holds |
| `E/Essentials.java:1080-1090`, `E/User.java:726-731`, `E/commands/EssentialsCommand.java:126-141`, `E/commands/EssentialsLoopCommand.java:51`, `91`, `E/commands/Commandrealname.java:23` | `canInteractWith` asks `Player.canSee`, and where `essentials.vanish.interact` enters | Holds |
| `E/commands/Commandmail.java:54-57`, `63`, `150-169`, `255-287`, `293-313`, `E/UserData.java:389-391`, `E/utils/DateUtil.java:74-76`, `msg:539` | `mail read` with only expired mails, the send variants of the console, `sendall` without the checks of `send`, and `illegalDate` | Holds |
| `E/items/AbstractItemDb.java:161`, `168`, `E/Worth.java:104`, `103-116`, `121-128`, `E/craftbukkit/Inventories.java:232-244`, `453-455`, `E/commands/Commandworth.java:29-47`, `78`, `86-90`, `E/Settings.java:1096-1098` | Worn armour in `/worth all` reads 0 items, and `itemMustBeStacked` needs a key that `config.yml` lacks | Holds |
| `E/commands/Commandlist.java:24`, `E/PlayerList.java:47`, `117-129`, `209` | Hidden players, the tag of `/list <group>` and the `<white>` after each name | Holds |
| `E/Essentials.java:730`, `765-770`, `777`, `788-795`, `813-820`, `msg:943`, `msg:631` | Labels that start with `e`, the log line, the mail notice, `disabled-commands` and jail in the dispatch | Holds |
| `E/textreader/HelpInput.java:109-116`, `126`, `E/commands/Commandhelp.java:40-61`, `44`, `59-61`, `yml:170` | The `helpPlugin` lines, `/help /<command>` and its fallback to `plugin.yml` | Holds |
| `E/commands/Commandbalancetop.java:124-143` | The play time is read for every ranked player at every rebuild | Holds |
| `E/commands/Commandignore.java:30`, `E/commands/Commandbalance.java:23`, `30`, `E/commands/Commandseen.java:73`, `E/Essentials.java:1029-1062` | The lookups of `ignore`, `balance <player>` and `seen` | Holds |
| `E/EssentialsPlayerListener.java:536-552`, `E/User.java:68` | The resets of the join, and the pay confirmations as a `WeakHashMap` in memory | Holds |
| `providers/BaseProviders/src/main/java/net/ess3/provider/AbstractChatEvent.java:10`, `E/utils/FormatUtil.java:297-306` | The URL rule and its class `[\w-_\.]` | Holds |
| [Plan](../../plan.md): [Quirks and security](../../plan.md#quirks-and-security), the rows Status, Economy, Signs and Admin of [EssentialsX 2.22.0](../../plan.md#essentialsx-2220), [Telemetry](../../plan.md#telemetry), [Data migration](../../plan.md#data-migration), [Before the port](../../plan.md#before-the-port) and [Open decisions](../../plan.md#open-decisions) | The rows and sections of the plan that this page names | Hold |
| [Bridge](../../bridge.md): [The host class](../../bridge.md#the-host-class), [Commands](../../bridge.md#commands), [The game](../../bridge.md#the-game), [Permissions](../../bridge.md#permissions) and [Actions](../../bridge.md#actions) | The events, the `chat` call that can only cancel, plugin commands without a permission level in Brigadier, the questions of the game, the permission handler and the five actions of the bridge | Hold |
| `bridge/mod/src/main/java/com/skytekx/golem/bridge/PluginCommands.java:47-61`, `55`, `71-80`, `Actions.java:66-74`, `93`, `100` | `dispatcher.register` of a literal without a requirement, the five kinds of action, and `message` and `broadcast` as `Component.literal` | Hold |

## Rulings and issues

This table rules on every entry of [Quirks and security](#quirks-and-security), against the
checkout of EssentialsX 2.22.0 and the sources of [Sources](#sources). Each entry that holds as a
quirk has its issue in `skytekx/golem`. The entries that a player can use on the live server today
stay in private security advisories until EssentialsX no longer runs there, so their text above
is held back too. The kind is the ruling, which can differ from the label in the column Kind of
that table.

| ID | Ruling | Issue |
| --- | --- | --- |
| Q1 | Security, live | Private advisory |
| Q2 | Not live, since EssentialsX owns these names through the Bukkit command map today. It is a need of the bridge instead: the bridge must remove the vanilla node before it registers a plugin command, see the [bridge needs](bridge-needs.md). | None |
| Q3 | Quirk | [#71](https://github.com/skytekx/golem/issues/71) |
| Q4 | Quirk | [#72](https://github.com/skytekx/golem/issues/72) |
| Q5 | Not a security issue. Vanilla 1.20.1 throttles every chat command in `detectRateSpam`, about one a second for a player. | None |
| Q6 | Security, live | Private advisory |
| Q7 | Quirk | [#73](https://github.com/skytekx/golem/issues/73) |
| Q8 | Quirk | [#74](https://github.com/skytekx/golem/issues/74) |
| Q9 | Quirk | [#75](https://github.com/skytekx/golem/issues/75) |
| Q10 | Quirk | [#76](https://github.com/skytekx/golem/issues/76) |
| Q11 | Quirk | [#77](https://github.com/skytekx/golem/issues/77) |
| Q12 | Quirk | [#78](https://github.com/skytekx/golem/issues/78) |
| Q13 | Quirk | [#79](https://github.com/skytekx/golem/issues/79) |
| Q14 | Quirk | [#80](https://github.com/skytekx/golem/issues/80) |
| Q15 | Quirk | [#81](https://github.com/skytekx/golem/issues/81) |
| Q16 | Quirk | [#82](https://github.com/skytekx/golem/issues/82) |
| Q17 | Quirk | [#83](https://github.com/skytekx/golem/issues/83) |
| Q18 | Quirk | [#84](https://github.com/skytekx/golem/issues/84) |
| Q19 | Quirk | [#85](https://github.com/skytekx/golem/issues/85) |
| Q20 | Quirk | [#86](https://github.com/skytekx/golem/issues/86) |
| Q21 | Quirk | [#87](https://github.com/skytekx/golem/issues/87) |
| Q22 | Quirk | [#88](https://github.com/skytekx/golem/issues/88) |
| Q23 | Security, live | Private advisory |
| Q24 | Quirk | [#89](https://github.com/skytekx/golem/issues/89) |
| Q25 | Quirk | [#90](https://github.com/skytekx/golem/issues/90) |
| Q26 | Quirk | [#91](https://github.com/skytekx/golem/issues/91) |
| Q27 | Quirk | [#31](https://github.com/skytekx/golem/issues/31) |
| Q28 | Quirk | [#92](https://github.com/skytekx/golem/issues/92) |
| Q29 | Quirk | [#93](https://github.com/skytekx/golem/issues/93) |
| Q30 | Quirk | [#94](https://github.com/skytekx/golem/issues/94) |
| Q31 | Not live. Arclight 1.0.6 hands EssentialsX a recipient set it can change, so chat ignore works today. The bridge must offer the same filter of recipients, see the [bridge needs](bridge-needs.md). | None |
| Q32 | A design note, not a quirk. | None |
| Q33 | Quirk | [#95](https://github.com/skytekx/golem/issues/95) |
| Q34 | Quirk | [#96](https://github.com/skytekx/golem/issues/96) |
| Q35 | Quirk | [#97](https://github.com/skytekx/golem/issues/97) |
| Q36 | Quirk | [#98](https://github.com/skytekx/golem/issues/98) |
| Q37 | Not live. Arclight 1.0.6 never reads the flag that `setSleepingIgnored` writes, so operators and AFK players count for sleeping today, and the port keeps that. | None |
