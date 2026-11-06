---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Spec
title: Store
description: The store of Golem in the crate golem-store, one SQLite file beside the world with the players, homes, warps, spawns, mail, ignore lists, accounts, permissions and settings of the plugins, its schema, its threads, its durability, its API and its cost on the server thread.
tags: [store, sqlite, plugins, essentials, permissions, economy, durability]
timestamp: 2026-11-06
---

# Store

The store holds the data of the plugins in one SQLite file beside the world: the players, their
homes, the warps, the spawns, the mail, the ignore lists, the accounts with the log of every
transfer, the permissions and a table of settings. It lives in the crate `golem-store`, which is
Golem's own code under the AGPL 3 only. The ports of EssentialsX 2.22.0 and EssentialsXSpawn,
under the GPL 3 or later, and of LuckPerms 5.5.71, under the MIT license, use it and keep the
licenses of their upstreams, which combine with it under section 13 of the GPL 3 and the AGPL 3,
see [Licensing](../Documentation/licensing.md). The migration tool of the
[plan](plan.md#data-migration) writes it from the data of EssentialsX and LuckPerms.

SQLite is the amalgamation 3.53.2 that `libsqlite3-sys` 0.38.2 bundles, through `rusqlite` 0.40.2,
both pinned exactly in the `Cargo.toml` of the workspace, since `rusqlite` asks for any
`libsqlite3-sys` 0.38. So every host runs the same SQLite whatever its system has, and the build
compiles it with the C compiler of the host. `golem_store::sqlite_version()` answers the version the
store runs. `rusqlite` and `libsqlite3-sys` are MIT and SQLite is in the public domain, so
`cargo deny check` passes them against `deny.toml` with no new license.

The notebook `tests/notebooks/09_store.ipynb` holds the store to this page through
`golem-devtool`.

## The design

The plugins call the store on the server thread, where a call has a budget of 1 ms, as
[Plugins](plugins/index.md#budgets) says. A sync of a file waits for the disk, and on macOS with
`F_FULLFSYNC` for the drive itself, so no call of the server thread may wait for one.

```text
server thread                       golem-store thread
  read  ──► cache                    (owns the connection)
  change ─► check, cache, queue ───► batch of up to 4096 changes, one transaction, commit
                                       ▼
                                     golem-store.db, golem-store.db-wal
```

- The store loads the whole file into a cache when it opens: every row except the log of the
  accounts. A read answers from the cache, with a hash or tree lookup and a copy of the answer.
- A change checks its rules against the cache. A change it refuses changes nothing. Otherwise it
  changes the cache and puts one operation on the queue of the writer, both under the lock of the
  cache, so the writer sees the operations in the order the cache took them. A read after a
  change sees it at once.
- The thread `golem-store` owns the only connection to the file. It takes every operation that
  waits, up to 4096, applies them in one transaction and commits it, so a burst of changes costs
  one sync. A flush, a read through the writer and the stop wait in the same queue, so each sees
  every change before it.
- Each call of the store sends at most one operation, and an operation is applied whole or not at
  all. A transfer moves the money from one account, to the other and into the log in one
  operation, and so in one transaction.

The cache costs memory instead of time. A store of 5000 players with three homes, two mails and
an account each is a file of 7.6 MB, and a process with it open held 40 MB resident, see
[The cost on the server thread](#the-cost-on-the-server-thread). The server has far fewer
players.

## The threads

| Call | Thread | Waits for the disk |
| --- | --- | --- |
| Every read, such as `player`, `home`, `balance` or `permissions` | Any, the server thread first | No |
| Every change, such as `join`, `set_home`, `transfer` or `set_setting` | Any, the server thread first | No |
| `flush`, `close`, `check`, `ledger` | Not the server thread | Yes |
| `open` | The start of the program, before the server thread runs | Yes |

`Store` is a handle that clones, so every plugin holds one, and it is `Send` and `Sync`. The lock
of the cache is held for the length of one call and never while the writer works, so a call of
the server thread never waits for the writer.

`check` copies the cache under that lock, so that it compares the file with the cache as the call
found it, and every other call waits for that copy. Every table of the cache sits behind an `Arc`,
the players and the accounts in 64 parts, so the copy takes about 140 pointers instead of the
rows, and its time does not grow with the players. In the run of `09_store` on 2026-11-06 a check
of the store of 5000 players held the lock 28 microseconds in the median in the debug build. A
change after the copy copies the part of the players or of the accounts, or the table, that it
changes, once while the check still holds the copy.

## Durability

| Setting | Value | Why |
| --- | --- | --- |
| `journal_mode` | `WAL` | A commit appends to the log beside the file, and a crash at any moment leaves the last commit or the one before it |
| `synchronous` | `FULL` | Every commit syncs the log, so a commit also survives the loss of power |
| `fullfsync` | `ON` | The sync reaches the disk itself on macOS, where a plain `fsync` stops at the cache of the drive. Other systems ignore it |
| `locking_mode` | `EXCLUSIVE` | The writer holds the lock of the file from the open to the close, so no other process reads or writes it |
| `foreign_keys` | `ON` | A home, a mail, an ignore, an account and an entry of the log always name a row that exists |
| `trusted_schema` | `OFF` | The schema runs no function of the application |
| `busy_timeout` | 0 | A file that another process holds is refused at once |

What this gives:

- A change is in the file and synced once `flush` or `close` answers. Between a change and its
  commit lies the lag of the writer, which is one commit for a change now and then and longer
  for a burst of thousands.
- A crash of the process or a kill at any moment, which `09_store` holds to, and a loss of power,
  which the sync of every commit covers as SQLite documents for WAL with `FULL`, leave the file
  with every change up to some point and none after it, in the order the store took them. At worst
  it misses the last changes before the crash, never one in the middle. The next open replays the
  log.
- No second process can open the file while the store is open, a second Golem as little as the
  migration tool or the `sqlite3` shell, so the cache stays the truth of the file. The open of the
  second one answers `another process holds the file, so it is open in a second Golem or a tool`.
  A second `Store::open` of the file in the same process answers `the store is open in this
  process already`, since the store keeps the canonical paths of its open files.
- When a transaction fails, because the disk is full or SQLite refuses a row, the writer rolls it
  back and applies its operations again one by one, each in a transaction of its own, until the one
  that fails. The file then holds every change before it. The writer drops every change after it,
  logs `[golem] store: the writer failed and the store takes no more changes: <reason>` and every
  later change, `flush` and `close` answer `the store takes no more changes, since the writer
  failed: <reason>`. Reads still answer from the cache, which may hold changes that the file
  lacks, so the server is restarted, which reads the file again.
- `close` applies every change, checkpoints the log into the file with `wal_checkpoint(TRUNCATE)`,
  closes it and ends the writer, and answers whether every change reached the file. A change after
  it answers `the store is closed`. A second close, also one on another thread while the first
  still writes, such as the callback of the exit hook beside the main path of `golem run`, waits
  until the writer ended and answers as the first, so a failure stays a failure. A store whose last
  handle is dropped without a close closes the same way and logs that it was not closed.

## The schema

The file has the application id `0x476f6c6d`, the bytes `Golm`, and the schema version 1 in its
user version. Every table is `STRICT`, so a value of the wrong type is an error instead of being
stored as it comes. A UUID is the text Minecraft writes, in lower case with its four dashes. A
time is milliseconds since 1970-01-01 UTC, as the caller gives it, so the migration tool carries
the times of EssentialsX over. Money is whole cents. A location is the six columns of a
dimension, a position and a rotation, and a location that may be missing has all six or none.
The column Key is the place of the column in the primary key.

### players

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `uuid` | TEXT | no | 1 | | The UUID |
| `name` | TEXT | no | | | The name of the last join |
| `first_join` | INTEGER | no | | | The first join |
| `last_join` | INTEGER | no | | | The last join |
| `last_seen` | INTEGER | no | | | The last join, the last quit or a later moment |
| `logout_dimension` | TEXT | yes | | | Where the player quit last |
| `logout_x` | REAL | yes | | | |
| `logout_y` | REAL | yes | | | |
| `logout_z` | REAL | yes | | | |
| `logout_yaw` | REAL | yes | | | |
| `logout_pitch` | REAL | yes | | | |
| `back_dimension` | TEXT | yes | | | Where `/back` leads |
| `back_x` | REAL | yes | | | |
| `back_y` | REAL | yes | | | |
| `back_z` | REAL | yes | | | |
| `back_yaw` | REAL | yes | | | |
| `back_pitch` | REAL | yes | | | |
| `afk` | INTEGER | no | | | 1 while the player is away from the keyboard, else 0 |
| `nickname` | TEXT | yes | | | The nickname |

### homes

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `player` | TEXT | no | 1 | `players.uuid` | The owner |
| `name` | TEXT | no | 2 | | The name of the home |
| `dimension` | TEXT | no | | | The location |
| `x` | REAL | no | | | |
| `y` | REAL | no | | | |
| `z` | REAL | no | | | |
| `yaw` | REAL | no | | | |
| `pitch` | REAL | no | | | |

### warps

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `name` | TEXT | no | 1 | | The name of the warp |
| `dimension` | TEXT | no | | | The location |
| `x` | REAL | no | | | |
| `y` | REAL | no | | | |
| `z` | REAL | no | | | |
| `yaw` | REAL | no | | | |
| `pitch` | REAL | no | | | |
| `permission` | TEXT | yes | | | The node a player needs for the warp, or none |

### spawns

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `group_name` | TEXT | no | 1 | | The group, `default` for the one spawn of the server |
| `dimension` | TEXT | no | | | The location |
| `x` | REAL | no | | | |
| `y` | REAL | no | | | |
| `z` | REAL | no | | | |
| `yaw` | REAL | no | | | |
| `pitch` | REAL | no | | | |

### mail

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `id` | INTEGER | no | 1 | | The number of the mail, from 1, never given twice |
| `sender` | TEXT | yes | | `players.uuid` | The sender, none for the console |
| `sender_name` | TEXT | no | | | The name of the sender when they sent it |
| `receiver` | TEXT | no | | `players.uuid` | The receiver |
| `text` | TEXT | no | | | The text |
| `sent_at` | INTEGER | no | | | When it was sent |
| `read` | INTEGER | no | | | 1 once the receiver read it, else 0 |

The index `mail_by_receiver` on `receiver` and `id` finds the mail of a player. The table
counts with `AUTOINCREMENT`, whose counter in `sqlite_sequence` keeps the largest number also after
its mail is removed, so a number never comes back, also after a restart.

### ignores

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `player` | TEXT | no | 1 | `players.uuid` | The player who ignores |
| `ignored` | TEXT | no | 2 | `players.uuid` | The player they ignore, never themselves |

### accounts

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `player` | TEXT | no | 1 | `players.uuid` | The owner |
| `balance` | INTEGER | no | | | The balance in cents, never below zero |

### transfers

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `id` | INTEGER | no | 1 | | The number of the entry, from 1, in the order of the store |
| `at` | INTEGER | no | | | When it happened |
| `source` | TEXT | yes | | `accounts.player` | The account the money left, none for a deposit |
| `target` | TEXT | yes | | `accounts.player` | The account the money reached, none for a withdrawal |
| `amount` | INTEGER | no | | | The amount in cents, above zero |
| `note` | TEXT | no | | | Why, such as `pay` or `eco give` |

An entry has a source, a target or both, and never the same account twice. The indexes
`transfers_by_source` and `transfers_by_target` find the entries of an account. Every change of a
balance is an entry, the starting balance of an account included, so the balance of an account is
always what reached it less what left it, and `check` holds every account to that.

### permission_groups

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `name` | TEXT | no | 1 | | The name of the group |
| `weight` | INTEGER | no | | | The weight, a 32 bit integer |

### permission_group_parents

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `group_name` | TEXT | no | 1 | `permission_groups.name` | The group |
| `position` | INTEGER | no | | | The place of the parent in the list of the group |
| `parent` | TEXT | no | 2 | `permission_groups.name` | A group it inherits from |

### permission_group_nodes

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `group_name` | TEXT | no | 1 | `permission_groups.name` | The group |
| `position` | INTEGER | no | | | The place of the node in the list of the group |
| `node` | TEXT | no | 2 | | A node, a deny with its leading `-` |

### permission_player_groups

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `player` | TEXT | no | 1 | | The player, who need not be in `players` |
| `position` | INTEGER | no | | | The place of the group in the list of the player |
| `group_name` | TEXT | no | 2 | `permission_groups.name` | A group of the player besides `default` |

### permission_player_nodes

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `player` | TEXT | no | 1 | | The player |
| `position` | INTEGER | no | | | The place of the node in the list of the player |
| `node` | TEXT | no | 2 | | A node of the player |

### settings

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `key` | TEXT | no | 1 | | The key |
| `value` | TEXT | no | | | The value |

## Versions and migrations

The migrations are a list in `schema.rs`: the entry at index n takes a store of version n to
n + 1, and the open runs every entry from the version of the file to the latest, each in one
transaction with the new user version, so a crash during a migration leaves the file at the
version before it. A new version appends its entry and never changes one before it, since stores
of every older version exist. Today the list holds one entry, from version 0, a new or empty
file, to version 1.

| The file | The open |
| --- | --- |
| Missing, or empty | Creates the schema of version 1 |
| A store of version 1 | Opens it |
| A store of a newer version | `the store has the schema version 2, newer than the version 1 this Golem knows` |
| A store of a version below 0 | `the store has the schema version -1, which no Golem writes` |
| A SQLite file of another application | `the file is a SQLite database of another application, with the application id 0x7` |
| A SQLite file with tables but no application id | `the file is a SQLite database with 1 tables and indexes but no store of Golem` |
| No SQLite file | `the file is not a SQLite database` |
| Held by another process | `another process holds the file, so it is open in a second Golem or a tool` |
| Open in this process, under any path to it | `the store is open in this process already` |

Each answer stands behind `cannot open the store <path>: `, and a refused file is never changed.
The open reads what a file is through a connection that only reads, and opens the file for the
writer only once it accepted it, since a connection that may write checkpoints the log of a
database in the journal mode WAL into the file when it closes and deletes the log. So a refused
file and its log `-wal` stay as they were, byte for byte, also the log that a crash of another
program left behind. The connection that reads may leave the index of the log, a file `-shm`,
beside them, as every reader of SQLite does.

## The API

Every call takes the UUID of a player as `Uuid`, which `Uuid::parse` reads from the text
Minecraft writes in either case. Times are milliseconds since 1970-01-01 UTC and money is cents
as `i64`.

### Players

| Call | What it does |
| --- | --- |
| `join(uuid, name, at)` | Creates the player on the first join, else sets the name, the last join and the last seen time, and answers `Joined` with `first` and the `previous_name` when the name changed |
| `quit(uuid, logout, at)` | Sets the logout location and the last seen time |
| `set_back(uuid, location)` | Sets or clears the place of `/back` |
| `set_afk(uuid, afk)`, `set_nickname(uuid, nickname)`, `set_last_seen(uuid, at)` | Set the field |
| `put_player(player)` | Puts a whole record in place, for the migration tool, keeping homes, mail and account |
| `player(uuid)` | The record |
| `player_named(name)` | The player who joined last with the name, in any case |
| `players_nicknamed(nickname)` | The players with the nickname, in any case, sorted by name |
| `player_count()` | The number of players |

### Places

| Call | What it does |
| --- | --- |
| `set_home(uuid, name, location)` | Sets a home and answers whether it replaced one |
| `remove_home(uuid, name)` | Removes a home and answers whether there was one |
| `home(uuid, name)`, `homes(uuid)`, `home_count(uuid)` | The home, the homes sorted by name, their number |
| `set_warp(name, location, permission)` | Sets a warp with the node it needs, if any |
| `remove_warp(name)`, `warp(name)`, `warps()` | As for homes |
| `set_spawn(group, location)` | Sets the spawn of a group, `DEFAULT_GROUP` for the one spawn |
| `remove_spawn(group)`, `spawn(group)`, `spawns()` | As for homes |

A name of a home, a warp or a setting is compared byte for byte. A port that wants names without
case folds them before it calls the store. The spawn of the group `default` is the one spawn of
the server, which holds for every player whose groups have none, as in EssentialsXSpawn.

### Mail and ignore lists

| Call | What it does |
| --- | --- |
| `send_mail(sender, sender_name, receiver, text, at)` | Sends a mail, from the console when `sender` is none, and answers its number |
| `mail(receiver)`, `unread_mail(receiver)` | The mail oldest first, the number not read |
| `mark_mail_read(receiver)` | Marks every mail read and answers how many were not |
| `remove_mail(receiver, id)`, `clear_mail(receiver)` | Remove one mail or all |
| `ignore(player, other)`, `unignore(player, other)` | Change the list and answer whether it changed |
| `ignores(player, other)`, `ignore_list(player)` | The answer and the list |

### Accounts

| Call | What it does |
| --- | --- |
| `create_account(uuid, starting, at, note)` | Opens the account, a starting balance above zero as a deposit of the log |
| `deposit(uuid, amount, at, note)` | Adds money the server creates, as `eco give` |
| `withdraw(uuid, amount, at, note)` | Takes money the server destroys, as `eco take` |
| `transfer(source, target, amount, at, note)` | Moves money between two accounts in one transaction, as `pay` |
| `set_balance(uuid, balance, at, note)` | Sets the balance through a deposit or withdrawal of the difference, as `eco set` |
| `balance(uuid)`, `top_balances(limit)`, `account_count()` | The balance, the largest first, the number |
| `ledger(account, limit)` | The newest entries of the log of one account or all, read through the writer |

Each change answers a `Receipt` with the number of the entry and the balances after it.

### Permissions

| Call | What it does |
| --- | --- |
| `permissions()` | The whole `PermissionSet`, whose `to_toml` is the text `golem-permissions` loads |
| `permissions_generation()` | A count from 0 at the open that moves with every change of the permissions |
| `permissions_with_generation()` | The generation and the set of that generation, read under one lock, the call a resolver reloads through |
| `replace_permissions(set)` | Puts a whole set in place, such as `PermissionSet::from_toml` of the seed |
| `put_permission_group(group)`, `remove_permission_group(name)` | Change one group |
| `put_player_permissions(player)`, `remove_player_permissions(uuid)` | Change one player |
| `permission_group(name)`, `player_permissions(uuid)` | One group, one player |

The store hands the permissions to `golem-permissions` as the TOML of
[config/permissions.toml](../config/permissions.toml), which `Permissions::from_toml` reads into
the resolver, and takes the seed file the same way, so neither crate names the other and each
keeps its license. `PermissionSet::from_toml` takes a player key only where `golem-permissions`
takes it, as a UUID in lower case with its four dashes, and refuses any other key with the message
of `golem-permissions`, `players.<key>: the key is not a UUID in lower case with its four dashes`
and an example. The store keeps a set as it is given and checks only what keeps its rows whole:
every group that a list names exists, no list names an entry twice, and no group that another
group or a player names is removed. `golem-permissions` judges the rest, the syntax of a node, the
group `default` and the cycles of inheritance, so a port loads a changed set into a resolver
before it writes the change, and loads the set again when the generation moved.

A resolver keeps the generation of the set it loaded, looks at `permissions_generation()` to learn
whether the permissions changed, and reloads through `permissions_with_generation()`, which answers
the set and its generation under one lock. `permissions()` and then `permissions_generation()` take
the lock twice, so a change of another thread between them, such as the worker thread of the chat
beside the server thread, pairs the set before the change with the generation after it. A resolver
that keeps that pair never reloads the change, and a node that was revoked or denied stays granted
until the next change.

### Settings and the store itself

| Call | What it does |
| --- | --- |
| `set_setting(key, value)`, `remove_setting(key)`, `setting(key)`, `settings()` | A table of text by key |
| `Store::open(path)` | Opens or creates the file, migrates it, loads the cache, starts the writer |
| `flush()` | Waits until every change before it is in the file and synced |
| `close()` | Writes, checkpoints and closes the file and ends the writer, and every later close answers as the first |
| `stats()` | The changes queued, written, lost and pending, the batches and the time of the writer |
| `MAX_BATCH` | 4096, the most changes in one transaction of the writer |
| `check()` | The integrity of the file, its foreign keys, every balance against the log and the file against the cache as the call found it, which a change of another thread during the check does not disturb |
| `describe(path)` | The schema of a file that no process holds, as SQLite reports it. As every reader of SQLite it may leave the files `-wal` and `-shm` beside the store |

## Refusals

A refused change answers `StoreError::Refused` with a `Refusal` and changes nothing. A change of
an account checks its note first, then its amount, then whether source and target are the same
account, then whether both exist, the source first, then whether the source holds the amount and
the target can take it. Money is an `i64` of cents, so an amount with a fraction cannot be asked
for at all.

| Refusal | Message |
| --- | --- |
| `NotAboveZero` | `the amount of 0 cents is not above zero` |
| `Negative` | `the balance of -1 cents is below zero`, for a starting balance or `set_balance` |
| `SameAccount` | `a transfer from the account of <uuid> to itself moves nothing` |
| `UnknownAccount` | `the player <uuid> has no account` |
| `Overdraw` | `the account of <uuid> holds 100 cents, less than the 150 cents to take` |
| `Overflow` | `the account of <uuid> holds 1 cents, and 9223372036854775807 more would pass the largest balance of 9223372036854775807 cents` |
| `AccountExists` | `the player <uuid> has an account already` |
| `UnknownPlayer` | `the player <uuid> is not in the store` |
| `IgnoreSelf` | `the player <uuid> cannot ignore themselves` |
| `UnknownGroup` | `the group "moderator" is not in the permissions` |
| `GroupInUse` | `the group "default" cannot be removed, since the group admin names it` |
| `Twice` | `the nodes of the group default name "essentials.home" twice` |
| `Invalid` | `the name of a home is empty`, `the name of a home has 65 characters, more than 64`, `the location of a home has the x NaN, which is not a finite number` |

The rules of a value: a name of a player, a home, a warp, a setting key, a nickname or the sender of
a mail has 1 to 64 characters and no control character. A node, a group or a dimension has 1 to 256
bytes and no white space or control character. A text of a mail, a note or a setting value has at
most 65536 bytes and no NUL, and a mail is not empty. A location is finite, and the store takes a
coordinate or an angle of -0.0 as 0.0, since SQLite writes a REAL without a fraction as an integer
and the file gives the zero back without its sign. A read then answers the same before a restart
and after it.

## The place in the program golem

The store is not wired into `golem run` yet. The ports wire it in this way.

1. `golem run` opens `golem-store.db` in the server directory, beside `world/`, with
   `Store::open` after it has resolved the server directory and before `install_plugins`, so a
   store that does not open stops the start with `golem: cannot open the store ...` before the JVM
   exists.
2. It seeds the permissions with `replace_permissions(PermissionSet::from_toml(...))` of
   `config/permissions.toml` when the store has no group yet.
3. It hands a clone of the `Store` to each port it adds to the runtime.
4. A plugin of the program calls `flush` on `ServerStopped`, the last event of the server thread,
   where a wait costs no tick, so every change of the ports is in the file before the JVM ends.
5. It calls `close` after `golem_jvm::run` returns, before `report`. A JVM that ends through
   `System.exit`, as the stop of a server without the bridge does on a signal, never returns from
   `golem_jvm::run`, so `golem-jvm` needs a callback in its exit hook that runs `close` there. The
   hook logs alone today, see [JVM host](jvm-host.md#the-hooks).

The file holds player data, so the repository of the server ignores `golem-store.db`, its log
`golem-store.db-wal` and the index of the log `golem-store.db-shm`, which the open leaves beside the
file when it reads what the file is. That fixes the quirk of the plan that the player data of
Essentials sits in a tracked folder.

## The cost on the server thread

`golem-devtool store-bench` fills a new store with 5000 players, each with three homes, two mails,
an account and an ignore, with 50 warps, three spawns, 20 settings and eleven groups of 20 nodes,
opens it again and times 2000 calls of each kind one by one. The table gives the mean and the 99th
percentile of a call in microseconds, of the debug build in the run of `09_store` and of a release
build in a run of `golem-devtool store-bench --players 5000 --repeat 2000`, both on 2026-11-06 in
the cloud container of the session with an Intel Xeon at 2.10 GHz and 4 cores that another agent
shared.

| Call | Kind | Debug mean | Debug p99 | Release mean | Release p99 |
| --- | --- | --- | --- | --- | --- |
| `player` | read | 0.62 | 1.06 | 0.29 | 0.58 |
| `player_named` | read | 1.25 | 2.22 | 0.37 | 0.66 |
| `home` | read | 0.67 | 1.29 | 0.35 | 0.69 |
| `homes` | read | 1.15 | 2.15 | 0.38 | 0.73 |
| `warp` | read | 0.63 | 0.99 | 0.13 | 0.32 |
| `spawn` | read | 0.16 | 0.23 | 0.07 | 0.06 |
| `balance` | read | 0.34 | 0.74 | 0.17 | 0.57 |
| `top_balances(10)` | read | 0.78 | 1.89 | 0.06 | 0.06 |
| `unread_mail` | read | 0.36 | 0.60 | 0.12 | 0.24 |
| `mail` | read | 1.24 | 12.68 | 0.40 | 0.77 |
| `ignores` | read | 0.74 | 2.00 | 0.48 | 0.86 |
| `setting` | read | 0.27 | 0.45 | 0.08 | 0.12 |
| `player_permissions` | read | 0.34 | 0.82 | 0.07 | 0.41 |
| `permissions` | read | 50.79 | 85.89 | 30.84 | 63.89 |
| `join` | change | 4.85 | 22.13 | 1.76 | 5.81 |
| `set_back` | change | 4.45 | 14.45 | 1.88 | 11.24 |
| `set_afk` | change | 3.43 | 11.12 | 1.43 | 4.91 |
| `set_home` | change | 2.20 | 5.56 | 0.66 | 2.68 |
| `send_mail` | change | 1.86 | 5.75 | 0.85 | 2.99 |
| `mark_mail_read` | change | 0.85 | 3.48 | 0.42 | 2.31 |
| `transfer` | change | 3.30 | 8.60 | 0.76 | 3.51 |
| `deposit` | change | 3.17 | 11.09 | 0.70 | 3.31 |
| `ignore` | change | 0.93 | 1.48 | 0.39 | 0.77 |
| `set_setting` | change | 0.94 | 3.27 | 0.25 | 2.01 |

Every call stays three orders of magnitude inside the budget of 1 ms, and a change costs about as
much as a read, since it waits for nothing. The whole permission set is the one call that copies
much, and a resolver loads it once and again only when the generation moved. The longest single
call was 1.48 ms in the debug run and 0.29 ms in the release run, both a copy of the permission
set, which is the scheduler of a shared machine and not the store, since the 99th percentile of
the same call is 86 and 64 microseconds.

The writer applied the 18000 changes of the timed calls in 952 ms of the debug build and 280 ms of
the release build on its own thread, while they cost the server thread 52 ms in the debug build.
The open of the store of 5000 players took 127 ms in the debug build and 49 ms in the release
build, and the release process held 40 MB resident with the store open, which bounds the cache
from above. The file was 7.6 MB.

## Tests

The notebook `tests/notebooks/09_store.ipynb` drives the store through the subcommands
`golem-devtool store`, which runs a JSON script of calls and prints the answer of every step as a
line as soon as it ran, `store-describe`, which prints the schema of a file, and `store-bench`.
It checks the schema against the tables of this page, every call and every refusal, a check while
another thread changes the store, a reopen after a close, a store dropped without a close, a
reopen after a kill between changes, a reader of the answers that goes away, a writer that fails
at a limit of the file size, the files the open refuses and the cost above.

Related: [Plugins](plugins/index.md), [Permissions](plugins/permissions.md),
[Plan](plan.md#data-migration), [Index](index.md).
