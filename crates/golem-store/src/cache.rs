// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The cache of the store: every row of the file except the log of the accounts, in maps that
 * answer the calls of the server thread without SQLite.
 *
 * The cache is loaded once when the store opens and changes only through the calls of the store,
 * in the same order as the writer applies them to the file, so it is always the file plus the
 * changes still on their way. The indexes by name, by nickname and by balance keep the lookups
 * that a command makes by something other than the UUID at the cost of a map lookup.
 */

use std::{
    cmp::Reverse,
    collections::{
        BTreeMap,
        BTreeSet,
        HashMap, //
    },
};

use rusqlite::{
    Connection,
    Row, //
};

use crate::model::{
    Location,
    Mail,
    PermissionGroup,
    Player,
    PlayerPermissions,
    Uuid,
    Warp, //
};

/// A player with what belongs to them alone.
#[derive(Clone, Debug)]
pub struct Entry {
    pub record: Player,
    pub homes: BTreeMap<String, Location>,
    pub ignores: BTreeSet<Uuid>,
    /// The mail of the player, oldest first.
    pub mail: Vec<Mail>,
    pub unread: usize,
}

impl Entry {
    pub fn new(record: Player) -> Self {
        Self {
            record,
            homes: BTreeMap::new(),
            ignores: BTreeSet::new(),
            mail: Vec::new(),
            unread: 0,
        }
    }
}

/// Every row of the store but the log of the accounts.
#[derive(Debug, Default)]
pub struct Cache {
    pub players: HashMap<Uuid, Entry>,
    /// The players by their name in lower case. A name can pass from one player to another, so
    /// a name may lead to several.
    pub names: HashMap<String, Vec<Uuid>>,
    /// The players by their nickname in lower case.
    pub nicknames: HashMap<String, Vec<Uuid>>,
    pub warps: BTreeMap<String, Warp>,
    pub spawns: BTreeMap<String, Location>,
    pub accounts: HashMap<Uuid, i64>,
    /// The accounts by balance, the largest first and then by UUID.
    pub ranking: BTreeSet<(Reverse<i64>, Uuid)>,
    pub groups: BTreeMap<String, PermissionGroup>,
    pub permission_players: BTreeMap<Uuid, PlayerPermissions>,
    /// Counts the changes of the permissions, so a resolver knows when to load them again.
    pub permissions_generation: u64,
    pub settings: BTreeMap<String, String>,
    /// The number the next mail takes.
    pub next_mail: i64,
    /// The number the next entry of the log of the accounts takes.
    pub next_transfer: i64,
}

/// The key of a name in the indexes.
pub fn fold(name: &str) -> String {
    name.to_lowercase()
}

impl Cache {
    /// Puts a player into an index by a key, at the end.
    fn index(index: &mut HashMap<String, Vec<Uuid>>, key: &str, uuid: Uuid) {
        let players = index.entry(fold(key)).or_default();
        if !players.contains(&uuid) {
            players.push(uuid);
        }
    }

    /// Takes a player out of an index by a key.
    fn unindex(index: &mut HashMap<String, Vec<Uuid>>, key: &str, uuid: Uuid) {
        let folded = fold(key);
        if let Some(players) = index.get_mut(&folded) {
            players.retain(|&other| other != uuid);
            if players.is_empty() {
                index.remove(&folded);
            }
        }
    }

    /// Puts the record of a player in place of the one before, keeping the indexes right.
    pub fn put_player(&mut self, record: Player) {
        let uuid = record.uuid;
        if let Some(entry) = self.players.get(&uuid) {
            let (name, nickname) = (entry.record.name.clone(), entry.record.nickname.clone());
            Self::unindex(&mut self.names, &name, uuid);
            if let Some(nickname) = nickname {
                Self::unindex(&mut self.nicknames, &nickname, uuid);
            }
        }
        Self::index(&mut self.names, &record.name, uuid);
        if let Some(nickname) = &record.nickname {
            Self::index(&mut self.nicknames, nickname, uuid);
        }
        match self.players.get_mut(&uuid) {
            Some(entry) => entry.record = record,
            None => {
                self.players.insert(uuid, Entry::new(record));
            }
        }
    }

    /// Sets the balance of an account, keeping the ranking right.
    pub fn set_balance(&mut self, account: Uuid, balance: i64) {
        if let Some(old) = self.accounts.insert(account, balance) {
            self.ranking.remove(&(Reverse(old), account));
        }
        self.ranking.insert((Reverse(balance), account));
    }

    /// Loads every row but the log from the file.
    pub fn load(connection: &Connection) -> rusqlite::Result<Self> {
        let mut cache = Self::default();
        let mut statement = connection.prepare(
            "SELECT uuid, name, first_join, last_join, last_seen, \
             logout_dimension, logout_x, logout_y, logout_z, logout_yaw, logout_pitch, \
             back_dimension, back_x, back_y, back_z, back_yaw, back_pitch, afk, nickname \
             FROM players",
        )?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let record = Player {
                uuid: uuid(row, 0)?,
                name: row.get(1)?,
                first_join: row.get(2)?,
                last_join: row.get(3)?,
                last_seen: row.get(4)?,
                logout: optional_location(row, 5)?,
                back: optional_location(row, 11)?,
                afk: row.get(17)?,
                nickname: row.get(18)?,
            };
            cache.put_player(record);
        }

        let mut statement = connection.prepare(
            "SELECT player, name, dimension, x, y, z, yaw, pitch FROM homes ORDER BY player, name",
        )?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let player = uuid(row, 0)?;
            let name: String = row.get(1)?;
            if let Some(entry) = cache.players.get_mut(&player) {
                entry.homes.insert(name, location(row, 2)?);
            }
        }

        let mut statement = connection.prepare("SELECT player, ignored FROM ignores")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let (player, ignored) = (uuid(row, 0)?, uuid(row, 1)?);
            if let Some(entry) = cache.players.get_mut(&player) {
                entry.ignores.insert(ignored);
            }
        }

        let mut statement = connection.prepare(
            "SELECT id, sender, sender_name, receiver, text, sent_at, read FROM mail ORDER BY id",
        )?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let mail = Mail {
                id: row.get(0)?,
                sender: optional_uuid(row, 1)?,
                sender_name: row.get(2)?,
                receiver: uuid(row, 3)?,
                text: row.get(4)?,
                sent_at: row.get(5)?,
                read: row.get(6)?,
            };
            if let Some(entry) = cache.players.get_mut(&mail.receiver) {
                if !mail.read {
                    entry.unread += 1;
                }
                entry.mail.push(mail);
            }
        }
        cache.next_mail = next_id(connection, "mail")?;
        cache.next_transfer = next_id(connection, "transfers")?;

        let mut statement = connection.prepare(
            "SELECT name, dimension, x, y, z, yaw, pitch, permission FROM warps ORDER BY name",
        )?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            let warp = Warp {
                name: name.clone(),
                location: location(row, 1)?,
                permission: row.get(7)?,
            };
            cache.warps.insert(name, warp);
        }

        let mut statement =
            connection.prepare("SELECT group_name, dimension, x, y, z, yaw, pitch FROM spawns")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            cache.spawns.insert(row.get(0)?, location(row, 1)?);
        }

        let mut statement = connection.prepare("SELECT player, balance FROM accounts")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            cache.set_balance(uuid(row, 0)?, row.get(1)?);
        }

        load_permissions(connection, &mut cache)?;

        let mut statement = connection.prepare("SELECT key, value FROM settings")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            cache.settings.insert(row.get(0)?, row.get(1)?);
        }
        Ok(cache)
    }
}

fn load_permissions(connection: &Connection, cache: &mut Cache) -> rusqlite::Result<()> {
    let mut statement = connection.prepare("SELECT name, weight FROM permission_groups")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(0)?;
        let group = PermissionGroup {
            name: name.clone(),
            weight: row.get(1)?,
            parents: Vec::new(),
            nodes: Vec::new(),
        };
        cache.groups.insert(name, group);
    }
    let mut statement = connection.prepare(
        "SELECT group_name, parent FROM permission_group_parents ORDER BY group_name, position",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(0)?;
        if let Some(group) = cache.groups.get_mut(&name) {
            group.parents.push(row.get(1)?);
        }
    }
    let mut statement = connection.prepare(
        "SELECT group_name, node FROM permission_group_nodes ORDER BY group_name, position",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(0)?;
        if let Some(group) = cache.groups.get_mut(&name) {
            group.nodes.push(row.get(1)?);
        }
    }
    let empty = |uuid| PlayerPermissions {
        uuid,
        groups: Vec::new(),
        nodes: Vec::new(),
    };
    let mut statement = connection.prepare(
        "SELECT player, group_name FROM permission_player_groups ORDER BY player, position",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let player = uuid(row, 0)?;
        let entry = cache
            .permission_players
            .entry(player)
            .or_insert_with(|| empty(player));
        entry.groups.push(row.get(1)?);
    }
    let mut statement = connection
        .prepare("SELECT player, node FROM permission_player_nodes ORDER BY player, position")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let player = uuid(row, 0)?;
        let entry = cache
            .permission_players
            .entry(player)
            .or_insert_with(|| empty(player));
        entry.nodes.push(row.get(1)?);
    }
    Ok(())
}

/// The number after the largest that a table of numbered rows ever held. The table counts with
/// `AUTOINCREMENT`, whose counter in `sqlite_sequence` keeps the largest number also after its
/// row is deleted, so a number never comes back after a restart.
fn next_id(connection: &Connection, table: &str) -> rusqlite::Result<i64> {
    let largest: i64 = connection.query_row(
        &format!(
            "SELECT max(coalesce((SELECT max(id) FROM {table}), 0), \
             coalesce((SELECT seq FROM sqlite_sequence WHERE name = '{table}'), 0))"
        ),
        [],
        |row| row.get(0),
    )?;
    Ok(largest.saturating_add(1))
}

/// A column that the schema holds to the text of a UUID.
fn uuid(row: &Row<'_>, column: usize) -> rusqlite::Result<Uuid> {
    let text: String = row.get(column)?;
    Uuid::parse(&text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Text,
            format!("{text:?} is not a UUID").into(),
        )
    })
}

fn optional_uuid(row: &Row<'_>, column: usize) -> rusqlite::Result<Option<Uuid>> {
    let text: Option<String> = row.get(column)?;
    text.map(|_| uuid(row, column)).transpose()
}

/// The six columns of a location from `first` on.
fn location(row: &Row<'_>, first: usize) -> rusqlite::Result<Location> {
    Ok(Location {
        dimension: row.get(first)?,
        x: row.get(first + 1)?,
        y: row.get(first + 2)?,
        z: row.get(first + 3)?,
        yaw: row.get(first + 4)?,
        pitch: row.get(first + 5)?,
    })
}

/// The six columns of a location that may be missing, which the schema holds to all or none.
fn optional_location(row: &Row<'_>, first: usize) -> rusqlite::Result<Option<Location>> {
    let dimension: Option<String> = row.get(first)?;
    dimension.map(|_| location(row, first)).transpose()
}
