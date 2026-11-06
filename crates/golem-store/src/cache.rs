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
 *
 * A copy of the cache, which the check of the store takes under the lock of the cache, must not
 * stall the calls of the server thread that wait for that lock. The players and the accounts are
 * maps in 64 parts behind an Arc each, the other tables are maps behind an Arc, and a player with
 * their homes, mail and ignore list, a group and the permissions of a player sit behind an Arc as
 * well, so a copy takes pointers instead of rows and costs the same for any number of rows. A
 * change copies a part, a table or an entry first when a copy still shares it, so the copy keeps
 * it as it was.
 */

use std::{
    cmp::Reverse,
    collections::{
        BTreeMap,
        BTreeSet,
        HashMap, //
    },
    sync::Arc,
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

/// The parts of a [`Sharded`] map.
const SHARDS: usize = 64;

/// A map by UUID in [`SHARDS`] parts behind an `Arc` each, so a copy of it copies 64 pointers
/// whatever its size. A change copies the part it changes first while a copy still shares that
/// part, which is about a 64th of the map, so the copy keeps the part as it was.
#[derive(Clone, Debug)]
pub struct Sharded<V> {
    shards: Vec<Arc<HashMap<Uuid, V>>>,
    len: usize,
}

impl<V> Default for Sharded<V> {
    fn default() -> Self {
        Self {
            shards: (0..SHARDS).map(|_| Arc::new(HashMap::new())).collect(),
            len: 0,
        }
    }
}

impl<V: Clone> Sharded<V> {
    /// The part of a UUID. The low bits of the UUID of a player are random in both versions
    /// Minecraft gives, 4 online and 3 offline, so the players spread over the parts.
    fn shard(uuid: &Uuid) -> usize {
        usize::from(uuid.0.to_le_bytes()[0]) % SHARDS
    }

    pub fn get(&self, uuid: &Uuid) -> Option<&V> {
        self.shards[Self::shard(uuid)].get(uuid)
    }

    pub fn contains_key(&self, uuid: &Uuid) -> bool {
        self.shards[Self::shard(uuid)].contains_key(uuid)
    }

    /// The value to change, after its part was copied when a copy of the map shares it.
    pub fn get_mut(&mut self, uuid: &Uuid) -> Option<&mut V> {
        let shard = &mut self.shards[Self::shard(uuid)];
        if !shard.contains_key(uuid) {
            return None;
        }
        Arc::make_mut(shard).get_mut(uuid)
    }

    pub fn insert(&mut self, uuid: Uuid, value: V) -> Option<V> {
        let old = Arc::make_mut(&mut self.shards[Self::shard(&uuid)]).insert(uuid, value);
        if old.is_none() {
            self.len += 1;
        }
        old
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Uuid, &V)> {
        self.shards.iter().flat_map(|shard| shard.iter())
    }
}

impl<V: PartialEq> PartialEq for Sharded<V> {
    fn eq(&self, other: &Self) -> bool {
        // Both maps put a UUID into the same part, so they agree part by part.
        self.len == other.len && self.shards == other.shards
    }
}

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
    /// The players, each changed through [`Cache::player_mut`].
    pub players: Sharded<Arc<Entry>>,
    /// The players by their name in lower case. A name can pass from one player to another, so
    /// a name may lead to several.
    pub names: HashMap<String, Vec<Uuid>>,
    /// The players by their nickname in lower case.
    pub nicknames: HashMap<String, Vec<Uuid>>,
    pub warps: Arc<BTreeMap<String, Warp>>,
    pub spawns: Arc<BTreeMap<String, Location>>,
    pub accounts: Sharded<i64>,
    /// The accounts by balance, the largest first and then by UUID.
    pub ranking: BTreeSet<(Reverse<i64>, Uuid)>,
    pub groups: Arc<BTreeMap<String, Arc<PermissionGroup>>>,
    pub permission_players: Arc<BTreeMap<Uuid, Arc<PlayerPermissions>>>,
    /// Counts the changes of the permissions, so a resolver knows when to load them again.
    pub permissions_generation: u64,
    pub settings: Arc<BTreeMap<String, String>>,
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

    /// The entry of a player to change. An entry that a copy of the cache still shares is copied
    /// first, so the copy keeps it as it was.
    pub fn player_mut(&mut self, uuid: &Uuid) -> Option<&mut Entry> {
        self.players.get_mut(uuid).map(Arc::make_mut)
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
        match self.player_mut(&uuid) {
            Some(entry) => entry.record = record,
            None => {
                self.players.insert(uuid, Arc::new(Entry::new(record)));
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
            if let Some(entry) = cache.player_mut(&player) {
                entry.homes.insert(name, location(row, 2)?);
            }
        }

        let mut statement = connection.prepare("SELECT player, ignored FROM ignores")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let (player, ignored) = (uuid(row, 0)?, uuid(row, 1)?);
            if let Some(entry) = cache.player_mut(&player) {
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
            if let Some(entry) = cache.player_mut(&mail.receiver) {
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
            Arc::make_mut(&mut cache.warps).insert(name, warp);
        }

        let mut statement =
            connection.prepare("SELECT group_name, dimension, x, y, z, yaw, pitch FROM spawns")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            Arc::make_mut(&mut cache.spawns).insert(row.get(0)?, location(row, 1)?);
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
            Arc::make_mut(&mut cache.settings).insert(row.get(0)?, row.get(1)?);
        }
        Ok(cache)
    }
}

fn load_permissions(connection: &Connection, cache: &mut Cache) -> rusqlite::Result<()> {
    let mut groups = BTreeMap::new();
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
        groups.insert(name, group);
    }
    let mut statement = connection.prepare(
        "SELECT group_name, parent FROM permission_group_parents ORDER BY group_name, position",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(0)?;
        if let Some(group) = groups.get_mut(&name) {
            group.parents.push(row.get(1)?);
        }
    }
    let mut statement = connection.prepare(
        "SELECT group_name, node FROM permission_group_nodes ORDER BY group_name, position",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(0)?;
        if let Some(group) = groups.get_mut(&name) {
            group.nodes.push(row.get(1)?);
        }
    }
    let mut players = BTreeMap::new();
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
        let entry = players.entry(player).or_insert_with(|| empty(player));
        entry.groups.push(row.get(1)?);
    }
    let mut statement = connection
        .prepare("SELECT player, node FROM permission_player_nodes ORDER BY player, position")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let player = uuid(row, 0)?;
        let entry = players.entry(player).or_insert_with(|| empty(player));
        entry.nodes.push(row.get(1)?);
    }
    cache.groups = Arc::new(
        groups
            .into_iter()
            .map(|(name, group)| (name, Arc::new(group)))
            .collect(),
    );
    cache.permission_players = Arc::new(
        players
            .into_iter()
            .map(|(uuid, player)| (uuid, Arc::new(player)))
            .collect(),
    );
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
