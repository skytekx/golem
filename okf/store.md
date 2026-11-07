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
an account each is a file of 7.9 MB, and a process with it open held 35 MB resident, see
[The cost on the server thread](#the-cost-on-the-server-thread). The server has far fewer
players.

## The threads

| Call | Thread | Waits for the disk |
| --- | --- | --- |
| Every read, such as `player`, `home`, `balance` or `permissions` | Any, the server thread first | No |
| Every change, such as `join`, `set_home`, `transfer` or `set_setting` | Any, the server thread first | No, but it waits for the writer behind `MAX_PENDING` changes |
| `flush`, `close`, `check`, `ledger`, `durability` | Not the server thread | Yes |
| The reload of a resolver of the permissions, see [The cost](#the-cost-on-the-server-thread) | Not the server thread | No |
| `open` | The start of the program, before the server thread runs | Yes |

`Store` is a handle that clones, so every plugin holds one, and it is `Send` and `Sync`. The lock
of the cache is held for the length of one call and never while the writer works, so a call of
the server thread never waits for the writer, with one exception.

At most `MAX_PENDING`, 16384 changes or four batches, wait for the writer. A change that finds that
many pending waits until the writer took its next batch, before it takes the lock of the cache, so
every read and every other thread goes on meanwhile. So a thread that changes the store faster
than the writer commits, such as a plugin in a loop or an import, goes at the pace of the writer,
and the memory of the queue and the lag of the file stay bounded. The first such wait of a store
logs `[golem] store: the writer of <path> is 16384 changes behind, so a change waits until it takes
the next batch`, and `stats()` counts the waits and the most changes that were pending at once,
which stays at `MAX_PENDING` or below while one thread at a time changes the store. On the server
thread such a wait costs a tick, and only a burst of more than 16384 changes meets it.

`check` copies the cache under that lock, so that it compares the file with the cache as the call
found it, and every other call waits for that copy. Every table of the cache sits behind an `Arc`,
the players and the accounts in 64 parts, so the copy takes about 140 pointers instead of the rows,
and its time does not grow with the players. In a run of `golem-devtool store-bench` of the debug
build on 2026-11-07 a check of the store of 5000 players held the lock 14 microseconds in the
median. A change after the copy copies the part of the players or of the accounts, or the table,
that it changes, once while the check still holds the copy.

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
  for a burst of thousands, at most the 16384 changes of `MAX_PENDING`.
- A crash of the process or a kill at any moment, and a loss of power, which the sync of every
  commit covers as SQLite documents for WAL with `FULL`, leave the file with every change up to
  some point and none after it, in the order the store took them. At worst it misses the last
  changes before the crash, never one in the middle. The next open replays the log. `09_store`
  kills the process once in the middle of a burst of changes, which holds the store to a crash of
  the process. A kill keeps the cache of the system, so it cannot show a loss of power, and the
  notebook holds the settings of the table above instead, as `durability()` reads them from the
  connection of the writer, since each holds per connection.
- No second process can open the file while the store is open, a second Golem as little as the
  migration tool or the `sqlite3` shell, so the cache stays the truth of the file. The open of the
  second one answers `another process holds the file, so it is open in a second Golem or a tool`.
  A second `Store::open` of the file in the same process answers `the store is open in this
  process already`, since the store keeps the canonical paths of its open files.
- When a transaction fails, because the disk is full or SQLite refuses a row, the writer rolls it
  back and applies its operations again one by one, each in a transaction of its own, until the one
  that fails. The file then holds every change before it. The writer drops every change after it,
  logs `[golem] store: the writer failed and the store takes no more changes: <reason>`, and from
  then on every change, `flush` and `close` answer `the store takes no more changes, since the
  writer failed: <reason>`. The store learns of the failure only once the writer has tried the
  failed batch change by change, so every change it took after the failing one until then
  answered `Ok`, changed the cache and is lost with it, and `stats()` counts it among the lost
  changes with the rest of the batch. Reads still answer from the cache, which then holds changes
  that the file lacks, so the server is restarted, which reads the file again. A writer thread
  that panics fails the store the same way, with the reason `the writer thread panicked`. No
  writer is left then, so a call that reads through it, such as `check`, `ledger` or
  `durability`, answers that reason as well, and so does every close.
- `close` applies every change, checkpoints the log into the file with `wal_checkpoint(TRUNCATE)`,
  closes it and ends the writer, and answers whether every change reached the file. A change after
  it answers `the store is closed`. A second close, also one on another thread while the first
  still writes, such as the callback of the exit hook beside the main path of `golem run`, waits
  until the writer ended and answers as the first, so a failure stays a failure. A store whose last
  handle is dropped without a close closes the same way and logs that it was not closed.

## The schema

The file has the application id `0x476f6c6d`, the bytes `Golm`, and the schema version 1 in its user
version. Every table is `STRICT`, so a value of the wrong type is an error instead of being stored
as it comes. A UUID is the text Minecraft writes, in lower case with its four dashes. A time is
milliseconds since 1970-01-01 UTC, as the caller gives it, so the migration tool carries the times
of EssentialsX over. Money is whole cents. A location is the six columns of a dimension, a position
and a rotation, and a location that may be missing has all six or none. The column Key is the place
of the column in the primary key. A reference that ends in `deleted with it` deletes the row when
the row it names is deleted, the `ON DELETE CASCADE` of SQLite, and every other reference refuses
the delete of a row that another row names. The rules that the tables name for a value, such as
never below zero, above zero, never themselves, all six or none and a 32 bit integer, are each a
`CHECK` of the table, so the file refuses a row that breaks one also when a program writes it past
the store.

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
| `group_name` | TEXT | no | 1 | `permission_groups.name`, deleted with it | The group |
| `position` | INTEGER | no | | | The place of the parent in the list of the group |
| `parent` | TEXT | no | 2 | `permission_groups.name` | A group it inherits from |

The pair of `group_name` and `position` is unique, so two parents never share a place.

### permission_group_nodes

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `group_name` | TEXT | no | 1 | `permission_groups.name`, deleted with it | The group |
| `position` | INTEGER | no | | | The place of the node in the list of the group |
| `node` | TEXT | no | 2 | | A node, a deny with its leading `-` |

The pair of `group_name` and `position` is unique, so two nodes never share a place.

### permission_player_groups

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `player` | TEXT | no | 1 | | The player, who need not be in `players` |
| `position` | INTEGER | no | | | The place of the group in the list of the player |
| `group_name` | TEXT | no | 2 | `permission_groups.name` | A group of the player besides `default` |

The pair of `player` and `position` is unique, so two groups never share a place.

### permission_player_nodes

| Column | Type | Null | Key | References | What it is |
| --- | --- | --- | --- | --- | --- |
| `player` | TEXT | no | 1 | | The player |
| `position` | INTEGER | no | | | The place of the node in the list of the player |
| `node` | TEXT | no | 2 | | A node of the player |

The pair of `player` and `position` is unique, so two nodes never share a place.

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

A name of a home, a warp or a setting and the group of a spawn are compared byte for byte. The store
takes any name of a home, a warp or a setting of 1 to 64 characters without a control character,
and the group of a spawn as it takes a group of the permissions, with 1 to 256 bytes and no white
space or control character, see [Refusals](#refusals). A port folds a name as its plugin does
before it calls the store, see [Names](#names). The spawn of the group `default` is the one spawn
of the server, which holds for every player whose groups have none, as in EssentialsXSpawn.

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
the set and its generation under one lock, on a thread other than the server thread, since a reload
costs about a whole budget of a call, see [The cost on the server
thread](#the-cost-on-the-server-thread). `permissions()` and then `permissions_generation()` take
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
| `stats()` | The changes queued, written, lost and pending, the batches, the time of the writer, the changes that waited for room and the most changes that were pending at once. A change is pending once the store took it and until the writer wrote or dropped it |
| `MAX_BATCH` | 4096, the most changes in one transaction of the writer |
| `MAX_PENDING` | 16384, the most changes that wait for the writer before a change waits for room |
| `durability()` | The settings of the table of [Durability](#durability) as the connection of the writer reports them, read through the writer |
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
a mail has 1 to 64 characters and no control character. A node, a group of the permissions, the
group of a spawn, the permission of a warp or a dimension has 1 to 256 bytes and no white space or
control character. A text of a mail, a note or a setting value has at
most 65536 bytes and no NUL, and a mail is not empty. A location is finite, and the store takes a
coordinate or an angle of -0.0 as 0.0, since SQLite writes a REAL without a fraction as an integer
and the file gives the zero back without its sign. A read then answers the same before a restart
and after it.

## Data from EssentialsX

The migration tool of the [plan](plan.md#data-migration) writes the store from the data of
EssentialsX 2.22.0, which holds values that the store does not take as they are. The rules below say
what the tool does with them, and its report names every value it changed.

### Balances

EssentialsX keeps a balance as a decimal number. In the defaults of its `config.yml`, `/pay` takes
amounts down to the `minimum-pay-amount` of 0.001, and a player with `essentials.eco.loan`, which
the `essentials.*` of the group admin of the seed includes, may go down to the `min-money` of
-10000. The store keeps whole cents of zero or more, so the tool rounds a balance with a fraction of
a cent half to even to whole cents, opens the account with the rounded balance as its starting
balance, so the log sums to it, and names in its report every account it rounded with the balance
before and after. A balance below zero has no place in the store, so the tool refuses it, as it
refuses every entry it cannot place, and names the player and the debt, which an operator settles in
EssentialsX, such as with `eco set`, before the migration runs again. The port of the economy takes
every amount in whole cents, as the store does, so a payment below one cent is refused there.

### Names

EssentialsX folds the names of homes, warps and spawn groups, while the store compares them byte for
byte, so each port folds a name as its plugin does before it calls the store, and the migration tool
carries the folded names of EssentialsX over as they are. A lookup through a port then finds what
EssentialsX found.

| Name | EssentialsX | The port |
| --- | --- | --- |
| A home | `StringUtil.safeString`: lower case, every character but `a` to `z` and `0` to `9` becomes `_`, on a set, a removal and a rename | The same, so `My Home`, `my home` and `my_home` are the one home `my_home` |
| A warp | Kept as it was set, stored under its file name, looked up in any case | Keeps the name as it was set and finds a warp among `warps()` in any case, for `/warp` and before `set_warp`, so two warps never differ in case alone |
| The group of a spawn | Lower case with the rules of English, and `default` for a group without a spawn | The same, so `/spawn` of a player of the group `Admin` finds the spawn set for `admin` |

The folding of a port is also what keeps white space, a zero width character or a character that
turns the direction of text out of the name of a home, as in EssentialsX, since the store refuses
only control characters.

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

`golem-devtool store-bench` fills a new store with 5000 players, each with three homes and an
account, every player but the first with two mails and an ignore of the player before them and every
tenth with a nickname, with 50 warps, three spawns, 20 settings, ten groups of 20 nodes beside the
group `default` with one node, and the first 100 players in a group and with a node of their own,
opens it again and times 2000 calls of each kind one by one, with a flush after each kind. The table
gives the mean and the 99th percentile of a call in microseconds, of a debug build and of a release
build, each in a run of `golem-devtool store-bench --players 5000 --repeat 2000` on 2026-11-07 in
the cloud container of the session with an Intel Xeon at 2.10 GHz and 4 cores that other agents
shared.

| Call | Kind | Debug mean | Debug p99 | Release mean | Release p99 |
| --- | --- | --- | --- | --- | --- |
| `player` | read | 0.55 | 0.93 | 0.22 | 0.59 |
| `player_named` | read | 1.30 | 2.71 | 0.35 | 0.74 |
| `home` | read | 0.65 | 1.33 | 0.28 | 0.59 |
| `homes` | read | 1.10 | 1.66 | 0.34 | 0.65 |
| `warp` | read | 0.38 | 0.63 | 0.12 | 0.23 |
| `spawn` | read | 0.16 | 0.20 | 0.06 | 0.08 |
| `balance` | read | 0.36 | 0.73 | 0.12 | 0.42 |
| `top_balances(10)` | read | 0.75 | 1.13 | 0.06 | 0.11 |
| `unread_mail` | read | 0.36 | 0.67 | 0.10 | 0.20 |
| `mail` | read | 0.62 | 1.07 | 0.21 | 0.56 |
| `ignores` | read | 0.48 | 0.78 | 0.20 | 0.48 |
| `setting` | read | 0.23 | 0.29 | 0.06 | 0.08 |
| `player_permissions` | read | 0.28 | 0.58 | 0.06 | 0.29 |
| `permissions` | read | 49.77 | 79.82 | 27.02 | 41.51 |
| `permissions_with_generation`, `to_toml` and `Permissions::from_toml` | reload | 7479.14 | 9074.06 | 867.06 | 5009.63 |
| `join` | change | 4.50 | 21.14 | 1.15 | 4.05 |
| `set_back` | change | 6.75 | 21.06 | 1.52 | 9.41 |
| `set_afk` | change | 8.60 | 16.13 | 1.22 | 3.10 |
| `set_home` | change | 23.22 | 9.48 | 0.70 | 1.83 |
| `send_mail` | change | 2.10 | 4.34 | 0.91 | 8.73 |
| `mark_mail_read` | change | 2.57 | 2.73 | 0.49 | 1.12 |
| `transfer` | change | 7.32 | 12.79 | 0.78 | 2.19 |
| `deposit` | change | 3.31 | 7.27 | 0.53 | 1.45 |
| `ignore` | change | 1.62 | 2.72 | 0.60 | 1.41 |
| `set_setting` | change | 1.16 | 2.69 | 0.43 | 1.55 |

Every read and every change of the table, the calls of the server thread, stays inside the budget of
1 ms in its mean and in its 99th percentile in both builds. The reload of the permissions is no call
of the server thread and takes far longer, see below. A read of one record takes at most 1.3
microseconds in the mean of the debug build, and a change 1.2 to 23.2 microseconds, since it checks
its rules, changes the cache and queues one operation, and it waits for nothing below `MAX_PENDING`.
A single call that the scheduler of the shared machine stopped for milliseconds raises a mean, which
the 99th percentile beside it shows. The copy of the whole permission set is the one call that
copies much, 50 microseconds in the mean or 20 times inside the budget. The longest single call was
40.09 ms in the debug run, a call of `set_home`, and 0.18 ms in the release run, a call of
`set_back`, which is the scheduler of a shared machine and not the store, since the 99th percentile
of the same calls is 9.5 and 9.4 microseconds.

A resolver loads the set again whenever the generation moved, and every change of the permissions of
any player moves it. A reload is `permissions_with_generation`, the `to_toml` of the set and
`Permissions::from_toml` of `golem-permissions`, and the bench times 100 reloads of its set of
eleven groups and 100 players with entries of their own: 7.5 ms in the mean in the debug build and
0.87 ms in the release build, which is about the whole budget of a call or more, and the time grows
with the players that have entries of their own. So a port reloads on a thread of its own, such as a
worker that builds the new resolver and then swaps it in, and the server thread keeps checking with
the resolver it holds until then. `09_store` holds the mean reload of the bench store in the debug
build below 50 ms.

The writer applied the 20000 changes of the timed calls in 876 ms of the debug build and 399 ms of
the release build on its own thread, while they cost the server thread 122 ms in the debug build.
The open of the store of 5000 players took 130 ms in the debug build and 49 ms in the release build,
and the release process held 35 MB resident with the store open, which bounds the cache from above.
The file was 7.9 MB.

## Tests

The notebook `tests/notebooks/09_store.ipynb` drives the store through the subcommands
`golem-devtool store`, which runs a JSON script of calls and prints the answer of every step as a
line as soon as it ran, `store-describe`, which prints the schema of a file, and `store-bench`. It
checks the schema against the tables of this page, every call and every refusal, a check while
another thread changes the store, a reopen after a close, a store dropped without a close, a reopen
after a kill between changes, a reader of the answers that goes away, a writer that fails at a limit
of the file size, a batch that fails part way at a limit of the pages of the file, a row that the
file refuses behind the cache, a writer thread that panics, the files the open refuses and the cost
above. The steps `pause_writer`, `limit_pages`, `execute_behind` and `panic_writer` of
`golem-devtool store` reach the writer through the hooks of the feature `testing` of `golem-store`,
which only `golem-devtool` turns on and which the program `golem` never calls.

Related: [Plugins](plugins/index.md), [Permissions](plugins/permissions.md),
[Plan](plan.md#data-migration), [Index](index.md).
