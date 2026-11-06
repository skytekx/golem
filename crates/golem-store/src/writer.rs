// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The writer of the store: a thread of its own that owns the connection to the file and applies
 * the changes of the server thread in the order the store took them.
 *
 * Every call of the store that changes something sends one operation, so one call is always
 * applied whole or not at all. The writer takes every operation that waits, up to MAX_BATCH, and
 * applies them in one transaction, so a burst of changes costs one sync of the log instead of
 * one per change. A flush, a read and the stop wait in the same queue, so each one sees every
 * change sent before it.
 *
 * A failed transaction is rolled back and its operations are applied again one by one, each in
 * a transaction of its own, until the one that fails. The file then holds every change before
 * that one, the writer drops every change after it and the store refuses new ones, so the file
 * is always a prefix of the changes in their order and never has a gap. A restart of Golem reads
 * the file again.
 */

use std::{
    sync::{
        Arc, Mutex,
        atomic::{
            AtomicBool,
            AtomicU64,
            Ordering, //
        },
        mpsc::{
            Receiver,
            SyncSender,
            TryRecvError, //
        },
    },
    time::Instant,
};

use rusqlite::{
    Connection,
    Transaction,
    params, //
};

use crate::{
    model::{
        Location,
        Mail,
        PermissionGroup,
        Player,
        PlayerPermissions,
        StoreError,
        Transfer,
        Uuid, //
    },
    schema::describe_error,
};

/// The most operations the writer applies in one transaction.
pub const MAX_BATCH: usize = 4096;

/// One change of the store, which the writer applies as one unit.
#[derive(Debug)]
pub enum Op {
    PutPlayer(Box<Player>),
    PutHome {
        player: Uuid,
        name: String,
        location: Location,
    },
    RemoveHome {
        player: Uuid,
        name: String,
    },
    PutWarp {
        name: String,
        location: Location,
        permission: Option<String>,
    },
    RemoveWarp(String),
    PutSpawn {
        group: String,
        location: Location,
    },
    RemoveSpawn(String),
    PutMail(Box<Mail>),
    MarkMailRead(Uuid),
    RemoveMail(i64),
    ClearMail(Uuid),
    PutIgnore {
        player: Uuid,
        ignored: Uuid,
    },
    RemoveIgnore {
        player: Uuid,
        ignored: Uuid,
    },
    CreateAccount {
        account: Uuid,
        deposit: Option<Box<Transfer>>,
    },
    Transfer(Box<Transfer>),
    PutGroup(Box<PermissionGroup>),
    RemoveGroup(String),
    PutPlayerPermissions(Box<PlayerPermissions>),
    RemovePlayerPermissions(Uuid),
    ReplacePermissions {
        groups: Vec<PermissionGroup>,
        players: Vec<PlayerPermissions>,
    },
    PutSetting {
        key: String,
        value: String,
    },
    RemoveSetting(String),
}

/// A read that runs on the connection of the writer, after every change sent before it.
pub type Read = Box<dyn FnOnce(&Connection) + Send>;

/// What the store sends the writer.
pub enum Message {
    Write(Op),
    /// Answers once every change before it is in the file.
    Flush(SyncSender<Result<(), StoreError>>),
    Read(Read),
    /// Applies every change before it, checkpoints the log into the file, closes the file and
    /// ends the thread, then answers.
    Stop(SyncSender<Result<(), StoreError>>),
}

/// What the writer and the store share: the counts and the failure.
#[derive(Debug, Default)]
pub struct Shared {
    /// The changes sent to the writer.
    pub queued: AtomicU64,
    /// The changes in the file.
    pub written: AtomicU64,
    /// The changes the writer dropped after a failure.
    pub lost: AtomicU64,
    /// The transactions the writer committed.
    pub batches: AtomicU64,
    /// The most changes in one transaction.
    pub largest_batch: AtomicU64,
    /// The time the writer spent applying and committing, in nanoseconds.
    pub busy_nanos: AtomicU64,
    /// Set once the writer failed, so the store refuses changes without taking its lock.
    pub failed: AtomicBool,
    /// Why the writer failed.
    pub failure: Mutex<Option<String>>,
}

impl Shared {
    /// The failure, if the writer failed.
    pub fn failure(&self) -> Option<String> {
        if !self.failed.load(Ordering::Acquire) {
            return None;
        }
        self.failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn fail(&self, reason: String) {
        crate::log(format_args!(
            "the writer failed and the store takes no more changes: {reason}"
        ));
        let mut failure = self
            .failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if failure.is_none() {
            *failure = Some(reason);
        }
        self.failed.store(true, Ordering::Release);
    }

    fn outcome(&self) -> Result<(), StoreError> {
        self.failure()
            .map_or(Ok(()), |reason| Err(StoreError::Failed(reason)))
    }
}

/// The loop of the writer thread, until a stop or until every sender is gone.
pub fn run(mut connection: Connection, messages: &Receiver<Message>, shared: &Arc<Shared>) {
    let mut batch = Vec::new();
    loop {
        let Ok(first) = messages.recv() else {
            // Every handle of the store is gone without a stop, which the store never does, so
            // the file is closed as a stop would.
            let _ = finish(connection, shared);
            return;
        };
        let mut control = None;
        match first {
            Message::Write(op) => batch.push(op),
            other => control = Some(other),
        }
        while control.is_none() && batch.len() < MAX_BATCH {
            match messages.try_recv() {
                Ok(Message::Write(op)) => batch.push(op),
                Ok(other) => control = Some(other),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        if !batch.is_empty() {
            apply(&mut connection, &mut batch, shared);
        }
        match control {
            None => {}
            Some(Message::Write(_)) => unreachable!("a write never ends a batch"),
            Some(Message::Flush(reply)) => {
                let _ = reply.send(shared.outcome());
            }
            Some(Message::Read(read)) => read(&connection),
            Some(Message::Stop(reply)) => {
                let outcome = finish(connection, shared);
                let _ = reply.send(outcome);
                return;
            }
        }
    }
}

/// Checkpoints the log into the file and closes it.
fn finish(connection: Connection, shared: &Shared) -> Result<(), StoreError> {
    let checkpoint = connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    let closed = connection.close().map_err(|(_, error)| error);
    let outcome = shared.outcome();
    if let Err(error) = checkpoint.and(closed) {
        let reason = describe_error(&error);
        crate::log(format_args!(
            "the store could not close its file cleanly: {reason}"
        ));
        // The log holds every commit, so the next open replays what the checkpoint missed.
        return outcome.and(Err(StoreError::Database(reason)));
    }
    outcome
}

/// Applies a batch in one transaction, or as much of it as `SQLite` takes in order.
fn apply(connection: &mut Connection, batch: &mut Vec<Op>, shared: &Shared) {
    let count = batch.len() as u64;
    if shared.failed.load(Ordering::Acquire) {
        shared.lost.fetch_add(count, Ordering::Relaxed);
        batch.clear();
        return;
    }
    let started = Instant::now();
    if transaction(connection, batch).is_ok() {
        shared.written.fetch_add(count, Ordering::Relaxed);
        shared.batches.fetch_add(1, Ordering::Relaxed);
        shared.largest_batch.fetch_max(count, Ordering::Relaxed);
    } else {
        // The transaction is rolled back. Each operation then gets its own, so the file keeps
        // every change before the first one SQLite refuses.
        let mut done = 0_u64;
        for op in batch.iter() {
            if let Err(error) = transaction(connection, std::slice::from_ref(op)) {
                shared.fail(format!("{} on {}", describe_error(&error), describe(op)));
                break;
            }
            done += 1;
            shared.batches.fetch_add(1, Ordering::Relaxed);
        }
        shared.written.fetch_add(done, Ordering::Relaxed);
        shared.lost.fetch_add(count - done, Ordering::Relaxed);
    }
    let nanos = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
    shared.busy_nanos.fetch_add(nanos, Ordering::Relaxed);
    batch.clear();
}

/// Applies operations in one transaction and commits it, or rolls it back on the first error.
fn transaction(connection: &mut Connection, ops: &[Op]) -> rusqlite::Result<()> {
    let transaction =
        connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    for op in ops {
        write(&transaction, op)?;
    }
    transaction.commit()
}

/// The operation in a few words, for the log of a failure.
fn describe(op: &Op) -> String {
    let text = format!("{op:?}");
    let name = text.split([' ', '(', '{']).next().unwrap_or("an operation");
    format!("the change {name}")
}

/// A refusal of the file where the cache allowed a change, which means that the cache and the
/// file disagree.
fn mismatch(what: &str) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
        Some(format!("the file disagrees with the cache: {what}")),
    )
}

/// The place of an entry in its list, as `SQLite` stores it.
fn position(index: usize) -> i64 {
    i64::try_from(index).unwrap_or(i64::MAX)
}

/// The six values of a location.
fn location_values(location: &Location) -> (String, f64, f64, f64, f64, f64) {
    (
        location.dimension.clone(),
        location.x,
        location.y,
        location.z,
        f64::from(location.yaw),
        f64::from(location.pitch),
    )
}

/// The six values of a location that may be missing.
#[allow(clippy::type_complexity)]
fn optional_location_values(
    location: Option<&Location>,
) -> (
    Option<String>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
) {
    match location {
        Some(location) => {
            let (dimension, x, y, z, yaw, pitch) = location_values(location);
            (
                Some(dimension),
                Some(x),
                Some(y),
                Some(z),
                Some(yaw),
                Some(pitch),
            )
        }
        None => (None, None, None, None, None, None),
    }
}

/// Runs one statement of an operation.
fn run_sql(
    transaction: &Transaction<'_>,
    sql: &str,
    values: impl rusqlite::Params,
) -> rusqlite::Result<()> {
    transaction.execute(sql, values).map(drop)
}

/// Applies one operation inside a transaction.
fn write(transaction: &Transaction<'_>, op: &Op) -> rusqlite::Result<()> {
    match op {
        Op::PutPlayer(player) => put_player(transaction, player),
        Op::PutHome {
            player,
            name,
            location,
        } => put_home(transaction, *player, name, location),
        Op::RemoveHome { player, name } => run_sql(
            transaction,
            "DELETE FROM homes WHERE player = ?1 AND name = ?2",
            params![player.to_string(), name],
        ),
        Op::PutWarp {
            name,
            location,
            permission,
        } => put_warp(transaction, name, location, permission.as_deref()),
        Op::RemoveWarp(name) => run_sql(transaction, "DELETE FROM warps WHERE name = ?1", [name]),
        Op::PutSpawn { group, location } => put_spawn(transaction, group, location),
        Op::RemoveSpawn(group) => run_sql(
            transaction,
            "DELETE FROM spawns WHERE group_name = ?1",
            [group],
        ),
        Op::PutMail(mail) => put_mail(transaction, mail),
        Op::MarkMailRead(receiver) => run_sql(
            transaction,
            "UPDATE mail SET read = 1 WHERE receiver = ?1 AND read = 0",
            [receiver.to_string()],
        ),
        Op::RemoveMail(id) => run_sql(transaction, "DELETE FROM mail WHERE id = ?1", [id]),
        Op::ClearMail(receiver) => run_sql(
            transaction,
            "DELETE FROM mail WHERE receiver = ?1",
            [receiver.to_string()],
        ),
        Op::PutIgnore { player, ignored } => run_sql(
            transaction,
            "INSERT INTO ignores (player, ignored) VALUES (?1, ?2) ON CONFLICT DO NOTHING",
            params![player.to_string(), ignored.to_string()],
        ),
        Op::RemoveIgnore { player, ignored } => run_sql(
            transaction,
            "DELETE FROM ignores WHERE player = ?1 AND ignored = ?2",
            params![player.to_string(), ignored.to_string()],
        ),
        Op::CreateAccount { account, deposit } => {
            run_sql(
                transaction,
                "INSERT INTO accounts (player, balance) VALUES (?1, 0)",
                [account.to_string()],
            )?;
            deposit
                .as_ref()
                .map_or(Ok(()), |deposit| transfer(transaction, deposit))
        }
        Op::Transfer(entry) => transfer(transaction, entry),
        Op::PutGroup(group) => put_group(transaction, group, true),
        Op::RemoveGroup(name) => run_sql(
            transaction,
            "DELETE FROM permission_groups WHERE name = ?1",
            [name],
        ),
        Op::PutPlayerPermissions(player) => put_player_permissions(transaction, player),
        Op::RemovePlayerPermissions(uuid) => remove_player_permissions(transaction, *uuid),
        Op::ReplacePermissions { groups, players } => {
            replace_permissions(transaction, groups, players)
        }
        Op::PutSetting { key, value } => run_sql(
            transaction,
            "INSERT INTO settings (key, value) VALUES (?1, ?2) \
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![key, value],
        ),
        Op::RemoveSetting(key) => {
            run_sql(transaction, "DELETE FROM settings WHERE key = ?1", [key])
        }
    }
}

fn put_home(
    transaction: &Transaction<'_>,
    player: Uuid,
    name: &str,
    location: &Location,
) -> rusqlite::Result<()> {
    let (dimension, x, y, z, yaw, pitch) = location_values(location);
    run_sql(
        transaction,
        "INSERT INTO homes (player, name, dimension, x, y, z, yaw, pitch) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
         ON CONFLICT (player, name) DO UPDATE SET dimension = excluded.dimension, \
         x = excluded.x, y = excluded.y, z = excluded.z, yaw = excluded.yaw, \
         pitch = excluded.pitch",
        params![player.to_string(), name, dimension, x, y, z, yaw, pitch],
    )
}

fn put_warp(
    transaction: &Transaction<'_>,
    name: &str,
    location: &Location,
    permission: Option<&str>,
) -> rusqlite::Result<()> {
    let (dimension, x, y, z, yaw, pitch) = location_values(location);
    run_sql(
        transaction,
        "INSERT INTO warps (name, dimension, x, y, z, yaw, pitch, permission) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
         ON CONFLICT (name) DO UPDATE SET dimension = excluded.dimension, \
         x = excluded.x, y = excluded.y, z = excluded.z, yaw = excluded.yaw, \
         pitch = excluded.pitch, permission = excluded.permission",
        params![name, dimension, x, y, z, yaw, pitch, permission],
    )
}

fn put_spawn(
    transaction: &Transaction<'_>,
    group: &str,
    location: &Location,
) -> rusqlite::Result<()> {
    let (dimension, x, y, z, yaw, pitch) = location_values(location);
    run_sql(
        transaction,
        "INSERT INTO spawns (group_name, dimension, x, y, z, yaw, pitch) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
         ON CONFLICT (group_name) DO UPDATE SET dimension = excluded.dimension, \
         x = excluded.x, y = excluded.y, z = excluded.z, yaw = excluded.yaw, \
         pitch = excluded.pitch",
        params![group, dimension, x, y, z, yaw, pitch],
    )
}

fn put_mail(transaction: &Transaction<'_>, mail: &Mail) -> rusqlite::Result<()> {
    run_sql(
        transaction,
        "INSERT INTO mail (id, sender, sender_name, receiver, text, sent_at, read) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            mail.id,
            mail.sender.map(|sender| sender.to_string()),
            mail.sender_name,
            mail.receiver.to_string(),
            mail.text,
            mail.sent_at,
            mail.read
        ],
    )
}

/// Puts a whole set in place of the permissions.
fn replace_permissions(
    transaction: &Transaction<'_>,
    groups: &[PermissionGroup],
    players: &[PlayerPermissions],
) -> rusqlite::Result<()> {
    transaction.execute_batch(
        "DELETE FROM permission_player_nodes; DELETE FROM permission_player_groups; \
         DELETE FROM permission_group_nodes; DELETE FROM permission_group_parents; \
         DELETE FROM permission_groups;",
    )?;
    // Every group exists before any group names it as a parent.
    for group in groups {
        run_sql(
            transaction,
            "INSERT INTO permission_groups (name, weight) VALUES (?1, ?2)",
            params![group.name, group.weight],
        )?;
    }
    for group in groups {
        put_group(transaction, group, false)?;
    }
    for player in players {
        put_player_permissions(transaction, player)?;
    }
    Ok(())
}

fn put_player(transaction: &Transaction<'_>, player: &Player) -> rusqlite::Result<()> {
    let (logout_dimension, logout_x, logout_y, logout_z, logout_yaw, logout_pitch) =
        optional_location_values(player.logout.as_ref());
    let (back_dimension, back_x, back_y, back_z, back_yaw, back_pitch) =
        optional_location_values(player.back.as_ref());
    transaction.execute(
        "INSERT INTO players (uuid, name, first_join, last_join, last_seen, \
         logout_dimension, logout_x, logout_y, logout_z, logout_yaw, logout_pitch, \
         back_dimension, back_x, back_y, back_z, back_yaw, back_pitch, afk, nickname) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, \
         ?18, ?19) \
         ON CONFLICT (uuid) DO UPDATE SET name = excluded.name, \
         first_join = excluded.first_join, last_join = excluded.last_join, \
         last_seen = excluded.last_seen, logout_dimension = excluded.logout_dimension, \
         logout_x = excluded.logout_x, logout_y = excluded.logout_y, \
         logout_z = excluded.logout_z, logout_yaw = excluded.logout_yaw, \
         logout_pitch = excluded.logout_pitch, back_dimension = excluded.back_dimension, \
         back_x = excluded.back_x, back_y = excluded.back_y, back_z = excluded.back_z, \
         back_yaw = excluded.back_yaw, back_pitch = excluded.back_pitch, \
         afk = excluded.afk, nickname = excluded.nickname",
        params![
            player.uuid.to_string(),
            player.name,
            player.first_join,
            player.last_join,
            player.last_seen,
            logout_dimension,
            logout_x,
            logout_y,
            logout_z,
            logout_yaw,
            logout_pitch,
            back_dimension,
            back_x,
            back_y,
            back_z,
            back_yaw,
            back_pitch,
            player.afk,
            player.nickname
        ],
    )?;
    Ok(())
}

/// Moves an amount between two accounts and logs it, in the transaction of the batch.
///
/// The cache refused every transfer the accounts cannot take, so each update must change
/// exactly one row. One that changes none means the file and the cache disagree, and the
/// transaction fails instead of logging money that never moved.
fn transfer(transaction: &Transaction<'_>, entry: &Transfer) -> rusqlite::Result<()> {
    if let Some(source) = entry.source {
        let changed = transaction.execute(
            "UPDATE accounts SET balance = balance - ?2 WHERE player = ?1 AND balance >= ?2",
            params![source.to_string(), entry.amount],
        )?;
        if changed != 1 {
            return Err(mismatch(&format!(
                "the account of {source} does not hold {} cents",
                entry.amount
            )));
        }
    }
    if let Some(target) = entry.target {
        let changed = transaction.execute(
            "UPDATE accounts SET balance = balance + ?2 \
             WHERE player = ?1 AND balance <= 9223372036854775807 - ?2",
            params![target.to_string(), entry.amount],
        )?;
        if changed != 1 {
            return Err(mismatch(&format!(
                "the account of {target} cannot take {} cents",
                entry.amount
            )));
        }
    }
    transaction.execute(
        "INSERT INTO transfers (id, at, source, target, amount, note) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            entry.id,
            entry.at,
            entry.source.map(|source| source.to_string()),
            entry.target.map(|target| target.to_string()),
            entry.amount,
            entry.note
        ],
    )?;
    Ok(())
}

/// Writes a group with its parents and nodes in place of the one before. With `row`, the row of
/// the group itself is written too, else it exists already.
fn put_group(
    transaction: &Transaction<'_>,
    group: &PermissionGroup,
    row: bool,
) -> rusqlite::Result<()> {
    if row {
        transaction.execute(
            "INSERT INTO permission_groups (name, weight) VALUES (?1, ?2) \
             ON CONFLICT (name) DO UPDATE SET weight = excluded.weight",
            params![group.name, group.weight],
        )?;
        transaction.execute(
            "DELETE FROM permission_group_parents WHERE group_name = ?1",
            [&group.name],
        )?;
        transaction.execute(
            "DELETE FROM permission_group_nodes WHERE group_name = ?1",
            [&group.name],
        )?;
    }
    for (index, parent) in group.parents.iter().enumerate() {
        transaction.execute(
            "INSERT INTO permission_group_parents (group_name, position, parent) \
             VALUES (?1, ?2, ?3)",
            params![group.name, position(index), parent],
        )?;
    }
    for (index, node) in group.nodes.iter().enumerate() {
        transaction.execute(
            "INSERT INTO permission_group_nodes (group_name, position, node) VALUES (?1, ?2, ?3)",
            params![group.name, position(index), node],
        )?;
    }
    Ok(())
}

fn remove_player_permissions(transaction: &Transaction<'_>, uuid: Uuid) -> rusqlite::Result<()> {
    let player = uuid.to_string();
    transaction.execute(
        "DELETE FROM permission_player_groups WHERE player = ?1",
        [&player],
    )?;
    transaction.execute(
        "DELETE FROM permission_player_nodes WHERE player = ?1",
        [&player],
    )?;
    Ok(())
}

fn put_player_permissions(
    transaction: &Transaction<'_>,
    player: &PlayerPermissions,
) -> rusqlite::Result<()> {
    remove_player_permissions(transaction, player.uuid)?;
    let uuid = player.uuid.to_string();
    for (index, group) in player.groups.iter().enumerate() {
        transaction.execute(
            "INSERT INTO permission_player_groups (player, position, group_name) \
             VALUES (?1, ?2, ?3)",
            params![uuid, position(index), group],
        )?;
    }
    for (index, node) in player.nodes.iter().enumerate() {
        transaction.execute(
            "INSERT INTO permission_player_nodes (player, position, node) VALUES (?1, ?2, ?3)",
            params![uuid, position(index), node],
        )?;
    }
    Ok(())
}
