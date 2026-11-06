// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The reads that run on the connection of the writer: the check of a file against its own rules
 * and against the cache, and the log of the accounts, which the cache does not hold.
 */

use std::collections::BTreeMap;

use rusqlite::{
    Connection,
    params, //
};

use crate::{
    cache::Cache,
    model::{
        Location,
        Mail,
        PermissionGroup,
        Player,
        PlayerPermissions,
        StoreError,
        Transfer,
        Uuid,
        Warp, //
    },
    schema::describe_error,
    store::Check,
};

/// A player with what belongs to them, in a form that compares.
#[derive(Clone, Debug, PartialEq)]
struct PlayerRows {
    record: Player,
    homes: Vec<(String, Location)>,
    ignores: Vec<Uuid>,
    mail: Vec<Mail>,
}

/// The rows of a cache in a form that compares, without the indexes, which follow from them.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    players: BTreeMap<Uuid, PlayerRows>,
    warps: Vec<Warp>,
    spawns: Vec<(String, Location)>,
    accounts: BTreeMap<Uuid, i64>,
    groups: Vec<PermissionGroup>,
    permission_players: Vec<PlayerPermissions>,
    settings: Vec<(String, String)>,
    next_mail: i64,
    next_transfer: i64,
}

/// Copies the rows of the cache.
pub fn snapshot(cache: &Cache) -> Snapshot {
    let players = cache
        .players
        .iter()
        .map(|(&uuid, entry)| {
            let rows = PlayerRows {
                record: entry.record.clone(),
                homes: entry
                    .homes
                    .iter()
                    .map(|(name, location)| (name.clone(), location.clone()))
                    .collect(),
                ignores: entry.ignores.iter().copied().collect(),
                mail: entry.mail.clone(),
            };
            (uuid, rows)
        })
        .collect();
    Snapshot {
        players,
        warps: cache.warps.values().cloned().collect(),
        spawns: cache
            .spawns
            .iter()
            .map(|(group, location)| (group.clone(), location.clone()))
            .collect(),
        accounts: cache
            .accounts
            .iter()
            .map(|(&uuid, &balance)| (uuid, balance))
            .collect(),
        groups: cache.groups.values().cloned().collect(),
        permission_players: cache.permission_players.values().cloned().collect(),
        settings: cache
            .settings
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        next_mail: cache.next_mail,
        next_transfer: cache.next_transfer,
    }
}

/// Whether every player of two snapshots has the same part.
fn same<T: PartialEq + ?Sized>(
    file: &Snapshot,
    cache: &Snapshot,
    pick: impl Fn(&PlayerRows) -> &T,
) -> bool {
    file.players.len() == cache.players.len()
        && file
            .players
            .iter()
            .zip(&cache.players)
            .all(|((one, rows), (other, theirs))| one == other && pick(rows) == pick(theirs))
}

/// The parts in which two snapshots differ, by name.
fn differences(file: &Snapshot, cache: &Snapshot) -> Vec<String> {
    let parts = [
        ("players", same(file, cache, |rows| &rows.record)),
        ("homes", same(file, cache, |rows| rows.homes.as_slice())),
        (
            "ignore lists",
            same(file, cache, |rows| rows.ignores.as_slice()),
        ),
        ("mail", same(file, cache, |rows| rows.mail.as_slice())),
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
    let mut statement = connection
        .prepare(
            "SELECT a.player, a.balance, \
             coalesce((SELECT sum(amount) FROM transfers WHERE target = a.player), 0) \
             - coalesce((SELECT sum(amount) FROM transfers WHERE source = a.player), 0) \
             FROM accounts AS a ORDER BY a.player",
        )
        .map_err(sqlite)?;
    let mut ledger_mismatches = Vec::new();
    let mut rows = statement.query([]).map_err(sqlite)?;
    while let Some(row) = rows.next().map_err(sqlite)? {
        let player: String = row.get(0).map_err(sqlite)?;
        let balance: i64 = row.get(1).map_err(sqlite)?;
        let sum: i64 = row.get(2).map_err(sqlite)?;
        if balance != sum {
            let uuid = Uuid::parse(&player).unwrap_or(Uuid(0));
            ledger_mismatches.push((uuid, balance, sum));
        }
    }
    let file = snapshot(&Cache::load(connection).map_err(sqlite)?);
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
    })
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
