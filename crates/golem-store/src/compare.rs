// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The reads that run on the connection of the writer: the check of a file against its own rules
 * and against the cache, and the log of the accounts, which the cache does not hold.
 */

use std::{
    collections::{
        BTreeMap,
        HashMap, //
    },
    sync::Arc,
    time::Instant,
};

use rusqlite::{
    Connection,
    params, //
};

use crate::{
    cache::{
        Cache,
        Entry,
        Sharded, //
    },
    model::{
        Location,
        PermissionGroup,
        PlayerPermissions,
        StoreError,
        Transfer,
        Uuid,
        Warp, //
    },
    schema::describe_error,
    store::Check,
};

/// The rows of a cache in a form that compares, without the indexes, which follow from them.
///
/// Every table of the cache sits behind an `Arc`, the players and the accounts in 64 parts, so a
/// snapshot takes about 140 pointers and copies no row. The check takes it under the lock of the
/// cache, where every call of the server thread waits for it, and its cost does not grow with the
/// rows.
#[derive(Clone, Debug)]
pub struct Snapshot {
    players: Sharded<Arc<Entry>>,
    warps: Arc<BTreeMap<String, Warp>>,
    spawns: Arc<BTreeMap<String, Location>>,
    accounts: Sharded<i64>,
    groups: Arc<BTreeMap<String, Arc<PermissionGroup>>>,
    permission_players: Arc<BTreeMap<Uuid, Arc<PlayerPermissions>>>,
    settings: Arc<BTreeMap<String, String>>,
    next_mail: i64,
    next_transfer: i64,
    /// How long the copy took, in nanoseconds.
    copy_nanos: u64,
}

/// Copies the rows of the cache.
pub fn snapshot(cache: &Cache) -> Snapshot {
    let started = Instant::now();
    let mut snapshot = Snapshot {
        players: cache.players.clone(),
        warps: cache.warps.clone(),
        spawns: cache.spawns.clone(),
        accounts: cache.accounts.clone(),
        groups: cache.groups.clone(),
        permission_players: cache.permission_players.clone(),
        settings: cache.settings.clone(),
        next_mail: cache.next_mail,
        next_transfer: cache.next_transfer,
        copy_nanos: 0,
    };
    snapshot.copy_nanos = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
    snapshot
}

/// The rows of a cache that the check loaded from the file, which it owns, so nothing is copied.
fn into_snapshot(cache: Cache) -> Snapshot {
    Snapshot {
        players: cache.players,
        warps: cache.warps,
        spawns: cache.spawns,
        accounts: cache.accounts,
        groups: cache.groups,
        permission_players: cache.permission_players,
        settings: cache.settings,
        next_mail: cache.next_mail,
        next_transfer: cache.next_transfer,
        copy_nanos: 0,
    }
}

/// Whether every player of two snapshots has the same part.
fn same<T: PartialEq + ?Sized>(
    file: &Snapshot,
    cache: &Snapshot,
    pick: impl Fn(&Entry) -> &T,
) -> bool {
    file.players.len() == cache.players.len()
        && cache.players.iter().all(|(uuid, entry)| {
            file.players
                .get(uuid)
                .is_some_and(|other| pick(other) == pick(entry))
        })
}

/// The parts in which two snapshots differ, by name.
fn differences(file: &Snapshot, cache: &Snapshot) -> Vec<String> {
    let parts = [
        ("players", same(file, cache, |entry| &entry.record)),
        ("homes", same(file, cache, |entry| &entry.homes)),
        ("ignore lists", same(file, cache, |entry| &entry.ignores)),
        ("mail", same(file, cache, |entry| entry.mail.as_slice())),
        ("warps", file.warps == cache.warps),
        ("spawns", file.spawns == cache.spawns),
        ("accounts", file.accounts == cache.accounts),
        ("permission groups", file.groups == cache.groups),
        (
            "permission players",
            file.permission_players == cache.permission_players,
        ),
        ("settings", file.settings == cache.settings),
        ("the numbers of the mail", file.next_mail == cache.next_mail),
        ("the log", file.next_transfer == cache.next_transfer),
    ];
    parts
        .iter()
        .filter(|(_, same)| !same)
        .map(|(name, _)| (*name).to_owned())
        .collect()
}

/// Checks the file: the integrity of `SQLite`, the foreign keys, every balance against the log,
/// the rows against the snapshot of the cache, and the rows of every table.
pub fn check(connection: &Connection, cache: &Snapshot) -> Result<Check, StoreError> {
    let sqlite = |error: rusqlite::Error| StoreError::Database(describe_error(&error));
    let mut statement = connection
        .prepare("PRAGMA integrity_check")
        .map_err(sqlite)?;
    let integrity = statement
        .query_map([], |row| row.get(0))
        .map_err(sqlite)?
        .collect::<rusqlite::Result<Vec<String>>>()
        .map_err(sqlite)?;
    let foreign_key_violations = connection
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .map_err(sqlite)?;
    let ledger_mismatches = ledger_mismatches(connection).map_err(sqlite)?;
    let file = into_snapshot(Cache::load(connection).map_err(sqlite)?);
    let mut statement = connection
        .prepare(
            "SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
             ORDER BY name",
        )
        .map_err(sqlite)?;
    let tables = statement
        .query_map([], |row| row.get(0))
        .map_err(sqlite)?
        .collect::<rusqlite::Result<Vec<String>>>()
        .map_err(sqlite)?;
    let mut counts = BTreeMap::new();
    for table in tables {
        // The name comes from sqlite_schema, and the quotes keep it one identifier.
        let count: i64 = connection
            .query_row(
                &format!("SELECT count(*) FROM \"{}\"", table.replace('"', "\"\"")),
                [],
                |row| row.get(0),
            )
            .map_err(sqlite)?;
        counts.insert(table, count);
    }
    Ok(Check {
        integrity,
        foreign_key_violations,
        ledger_mismatches,
        cache_differences: differences(&file, cache),
        rows: counts,
        copy_nanos: cache.copy_nanos,
    })
}

/// The accounts whose balance is not what reached them less what left them, each with its
/// balance and that sum.
///
/// The sums run in Rust as `i128`. The `sum` of `SQLite` fails with `integer overflow` once the
/// amounts it adds pass `i64::MAX`, which a valid log reaches: a starting balance of `i64::MAX`
/// cents, a withdrawal of one cent and a deposit of one cent.
fn ledger_mismatches(connection: &Connection) -> rusqlite::Result<Vec<(Uuid, i64, i128)>> {
    let mut sums: HashMap<String, i128> = HashMap::new();
    let mut statement = connection.prepare("SELECT source, target, amount FROM transfers")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let amount = i128::from(row.get::<_, i64>(2)?);
        if let Some(source) = row.get::<_, Option<String>>(0)? {
            *sums.entry(source).or_default() -= amount;
        }
        if let Some(target) = row.get::<_, Option<String>>(1)? {
            *sums.entry(target).or_default() += amount;
        }
    }
    let mut statement =
        connection.prepare("SELECT player, balance FROM accounts ORDER BY player")?;
    let mut rows = statement.query([])?;
    let mut mismatches = Vec::new();
    while let Some(row) = rows.next()? {
        let player: String = row.get(0)?;
        let balance: i64 = row.get(1)?;
        let sum = sums.get(&player).copied().unwrap_or(0);
        if i128::from(balance) != sum {
            let uuid = Uuid::parse(&player).unwrap_or(Uuid(0));
            mismatches.push((uuid, balance, sum));
        }
    }
    Ok(mismatches)
}

/// The last `limit` entries of the log, the newest first, of one account or of all.
pub fn ledger(
    connection: &Connection,
    account: Option<Uuid>,
    limit: i64,
) -> rusqlite::Result<Vec<Transfer>> {
    let entry = |row: &rusqlite::Row<'_>| -> rusqlite::Result<Transfer> {
        let uuid = |column: usize| -> rusqlite::Result<Option<Uuid>> {
            let text: Option<String> = row.get(column)?;
            Ok(text.as_deref().and_then(Uuid::parse))
        };
        Ok(Transfer {
            id: row.get(0)?,
            at: row.get(1)?,
            source: uuid(2)?,
            target: uuid(3)?,
            amount: row.get(4)?,
            note: row.get(5)?,
        })
    };
    if let Some(account) = account {
        let mut statement = connection.prepare(
            "SELECT id, at, source, target, amount, note FROM transfers \
             WHERE source = ?1 OR target = ?1 ORDER BY id DESC LIMIT ?2",
        )?;
        return statement
            .query_map(params![account.to_string(), limit], entry)?
            .collect();
    }
    let mut statement = connection.prepare(
        "SELECT id, at, source, target, amount, note FROM transfers ORDER BY id DESC LIMIT ?1",
    )?;
    statement.query_map([limit], entry)?.collect()
}
