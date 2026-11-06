// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The schema of the store, the migrations from one version to the next, and the opening of a
 * file with the settings of SQLite that the durability of the store rests on.
 *
 * A store is a SQLite file whose application id is APPLICATION_ID and whose user version is the
 * version of its schema. Opening a file runs every migration from its version to the latest in
 * one transaction each, so a crash during a migration leaves the file at the version before it.
 * A file of another application, of a newer Golem or that holds tables without being a store is
 * refused, never changed.
 */

use std::path::Path;

use rusqlite::{
    Connection,
    ErrorCode,
    OpenFlags, //
};

use crate::model::StoreError;

/// The application id of a store, the bytes `Golm`, which `file` and `SQLite` tools show.
pub const APPLICATION_ID: i32 = 0x476f_6c6d;

/// The version of the schema this build writes.
pub const SCHEMA_VERSION: i32 = 1;

/// The schema of version 1.
///
/// Every table is STRICT, so a value of the wrong type is an error instead of being stored as
/// it comes. A UUID is the text Minecraft writes, in lower case. A time is milliseconds since
/// 1970 UTC. Money is whole cents. A location is the six columns of a dimension, a position and
/// a rotation, and a location that may be missing has all six or none.
const SCHEMA_1: &str = "
CREATE TABLE players (
    uuid TEXT PRIMARY KEY NOT NULL CHECK (length(uuid) = 36 AND uuid = lower(uuid)),
    name TEXT NOT NULL,
    first_join INTEGER NOT NULL,
    last_join INTEGER NOT NULL,
    last_seen INTEGER NOT NULL,
    logout_dimension TEXT,
    logout_x REAL,
    logout_y REAL,
    logout_z REAL,
    logout_yaw REAL,
    logout_pitch REAL,
    back_dimension TEXT,
    back_x REAL,
    back_y REAL,
    back_z REAL,
    back_yaw REAL,
    back_pitch REAL,
    afk INTEGER NOT NULL DEFAULT 0 CHECK (afk IN (0, 1)),
    nickname TEXT,
    CHECK ((logout_dimension IS NULL) = (logout_x IS NULL)
        AND (logout_x IS NULL) = (logout_y IS NULL)
        AND (logout_y IS NULL) = (logout_z IS NULL)
        AND (logout_z IS NULL) = (logout_yaw IS NULL)
        AND (logout_yaw IS NULL) = (logout_pitch IS NULL)),
    CHECK ((back_dimension IS NULL) = (back_x IS NULL)
        AND (back_x IS NULL) = (back_y IS NULL)
        AND (back_y IS NULL) = (back_z IS NULL)
        AND (back_z IS NULL) = (back_yaw IS NULL)
        AND (back_yaw IS NULL) = (back_pitch IS NULL))
) STRICT;

CREATE TABLE homes (
    player TEXT NOT NULL REFERENCES players (uuid),
    name TEXT NOT NULL,
    dimension TEXT NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    z REAL NOT NULL,
    yaw REAL NOT NULL,
    pitch REAL NOT NULL,
    PRIMARY KEY (player, name)
) STRICT;

CREATE TABLE warps (
    name TEXT PRIMARY KEY NOT NULL,
    dimension TEXT NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    z REAL NOT NULL,
    yaw REAL NOT NULL,
    pitch REAL NOT NULL,
    permission TEXT
) STRICT;

CREATE TABLE spawns (
    group_name TEXT PRIMARY KEY NOT NULL,
    dimension TEXT NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    z REAL NOT NULL,
    yaw REAL NOT NULL,
    pitch REAL NOT NULL
) STRICT;

CREATE TABLE mail (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    sender TEXT REFERENCES players (uuid),
    sender_name TEXT NOT NULL,
    receiver TEXT NOT NULL REFERENCES players (uuid),
    text TEXT NOT NULL,
    sent_at INTEGER NOT NULL,
    read INTEGER NOT NULL DEFAULT 0 CHECK (read IN (0, 1))
) STRICT;

CREATE INDEX mail_by_receiver ON mail (receiver, id);

CREATE TABLE ignores (
    player TEXT NOT NULL REFERENCES players (uuid),
    ignored TEXT NOT NULL REFERENCES players (uuid),
    PRIMARY KEY (player, ignored),
    CHECK (player <> ignored)
) STRICT;

CREATE TABLE accounts (
    player TEXT PRIMARY KEY NOT NULL REFERENCES players (uuid),
    balance INTEGER NOT NULL CHECK (balance >= 0)
) STRICT;

CREATE TABLE transfers (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    at INTEGER NOT NULL,
    source TEXT REFERENCES accounts (player),
    target TEXT REFERENCES accounts (player),
    amount INTEGER NOT NULL CHECK (amount > 0),
    note TEXT NOT NULL,
    CHECK (source IS NOT NULL OR target IS NOT NULL),
    CHECK (source IS NULL OR target IS NULL OR source <> target)
) STRICT;

CREATE INDEX transfers_by_source ON transfers (source, id);
CREATE INDEX transfers_by_target ON transfers (target, id);

CREATE TABLE permission_groups (
    name TEXT PRIMARY KEY NOT NULL,
    weight INTEGER NOT NULL DEFAULT 0 CHECK (weight BETWEEN -2147483648 AND 2147483647)
) STRICT;

CREATE TABLE permission_group_parents (
    group_name TEXT NOT NULL REFERENCES permission_groups (name) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    parent TEXT NOT NULL REFERENCES permission_groups (name),
    PRIMARY KEY (group_name, parent),
    UNIQUE (group_name, position)
) STRICT;

CREATE TABLE permission_group_nodes (
    group_name TEXT NOT NULL REFERENCES permission_groups (name) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    node TEXT NOT NULL,
    PRIMARY KEY (group_name, node),
    UNIQUE (group_name, position)
) STRICT;

CREATE TABLE permission_player_groups (
    player TEXT NOT NULL CHECK (length(player) = 36 AND player = lower(player)),
    position INTEGER NOT NULL,
    group_name TEXT NOT NULL REFERENCES permission_groups (name),
    PRIMARY KEY (player, group_name),
    UNIQUE (player, position)
) STRICT;

CREATE TABLE permission_player_nodes (
    player TEXT NOT NULL CHECK (length(player) = 36 AND player = lower(player)),
    position INTEGER NOT NULL,
    node TEXT NOT NULL,
    PRIMARY KEY (player, node),
    UNIQUE (player, position)
) STRICT;

CREATE TABLE settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
) STRICT;
";

/// The migrations in order: the entry at index `n` takes a store of version `n` to `n + 1`. A
/// new version appends its migration and raises [`SCHEMA_VERSION`], and never changes an entry
/// before it, since stores of every older version exist.
const MIGRATIONS: &[&str] = &[SCHEMA_1];

/// Opens the file at `path` for the writer of the store, creating it when it does not exist,
/// and brings its schema to [`SCHEMA_VERSION`].
///
/// The open first reads what the file is through a connection that only reads, and refuses a
/// file that is no store of this Golem before it opens one that may write. A connection that may
/// write checkpoints the log of a database in the journal mode WAL into the file when it closes
/// and deletes the log, also on a file it refused, while one that only reads never does, so a
/// refused file and its log `-wal` stay as they were. `SQLite` may leave the index of that log,
/// a file `-shm`, beside them. The connection of the writer holds the file in the exclusive
/// locking mode of `SQLite`, so no other process, a second Golem or the migration tool, can read
/// or write it while the store is open. The cache of the store is then the truth of the file,
/// which a second writer would break.
pub fn open(path: &Path) -> Result<Connection, StoreError> {
    let refuse = |reason: String| StoreError::Open {
        path: path.to_owned(),
        reason,
    };
    let sqlite = |error: rusqlite::Error| refuse(describe_error(&error));
    let identity = |reason| match reason {
        Identity::Refused(reason) => refuse(reason),
        Identity::Sqlite(error) => sqlite(error),
    };
    // A busy timeout of zero answers a file that another process holds at once.
    if path.exists() {
        let reader = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(sqlite)?;
        reader
            .busy_timeout(std::time::Duration::ZERO)
            .map_err(sqlite)?;
        identify(&reader).map_err(identity)?;
        reader.close().map_err(|(_, error)| sqlite(error))?;
    }
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let connection = Connection::open_with_flags(path, flags).map_err(sqlite)?;
    // The exclusive locking mode, set before the first read, holds every lock the connection
    // takes until it closes, and in the journal mode WAL it keeps the index of the log in the
    // memory of the process instead of a shared file.
    connection
        .busy_timeout(std::time::Duration::ZERO)
        .map_err(sqlite)?;
    connection
        .pragma_update(None, "locking_mode", "EXCLUSIVE")
        .map_err(sqlite)?;
    // The file was accepted above, and this reads its version again on the connection of the
    // writer.
    let version = identify(&connection).map_err(identity)?;
    configure(&connection).map_err(sqlite)?;
    migrate(&connection, version).map_err(refuse)?;
    Ok(connection)
}

/// Why [`identify`] refuses a file.
enum Identity {
    Refused(String),
    Sqlite(rusqlite::Error),
}

impl From<rusqlite::Error> for Identity {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

/// Reads what the file is, without changing it, and answers the version of its schema: 0 for a
/// new or empty file, the version of a store this Golem can open, or a refusal.
fn identify(connection: &Connection) -> Result<i32, Identity> {
    let application: i32 =
        connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
    let version: i32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if application != APPLICATION_ID {
        if application != 0 {
            return Err(Identity::Refused(format!(
                "the file is a SQLite database of another application, with the application id {application:#x}"
            )));
        }
        let objects: i64 =
            connection.query_row("SELECT count(*) FROM sqlite_schema", [], |row| row.get(0))?;
        if objects != 0 || version != 0 {
            return Err(Identity::Refused(format!(
                "the file is a SQLite database with {objects} tables and indexes but no store of Golem"
            )));
        }
    }
    let latest = SCHEMA_VERSION;
    if version > latest {
        return Err(Identity::Refused(format!(
            "the store has the schema version {version}, newer than the version {latest} this Golem knows"
        )));
    }
    if version < 0 {
        return Err(Identity::Refused(format!(
            "the store has the schema version {version}, which no Golem writes"
        )));
    }
    Ok(version)
}

/// The settings of the connection that the durability of the store rests on.
///
/// - The journal mode WAL appends every commit to the log beside the file, so a commit is one
///   write at the end of a file and a crash at any moment leaves the last commit or the one
///   before it.
/// - The synchronous mode FULL syncs the log on every commit, so a commit also survives the loss
///   of power. The writer commits on its own thread, so the server thread never waits for it.
/// - `fullfsync` makes that sync reach the disk itself on macOS, where a plain fsync stops at the
///   cache of the drive. Other systems ignore it.
/// - Foreign keys are on, so a home, a mail or an account always names a player of the store.
/// - A schema that is not trusted runs no function of the application.
fn configure(connection: &Connection) -> rusqlite::Result<()> {
    let mode: String =
        connection.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "the journal mode is {mode}, not WAL"
        )));
    }
    connection.pragma_update(None, "synchronous", "FULL")?;
    connection.pragma_update(None, "fullfsync", "ON")?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "trusted_schema", "OFF")?;
    // A write takes the exclusive lock at once, so a second process is refused at its open and
    // not at its first write.
    connection.execute_batch("BEGIN IMMEDIATE; COMMIT;")
}

/// Brings the schema of the file from `version` to [`SCHEMA_VERSION`], one migration per
/// transaction, so a crash during a migration leaves the file at the version before it.
fn migrate(connection: &Connection, version: i32) -> Result<(), String> {
    let first = usize::try_from(version).unwrap_or(0);
    for (from, migration) in MIGRATIONS.iter().enumerate().skip(first) {
        let to = i32::try_from(from + 1).unwrap_or(i32::MAX);
        let script = format!(
            "BEGIN IMMEDIATE;\n{migration}\nPRAGMA application_id = {APPLICATION_ID};\nPRAGMA user_version = {to};\nCOMMIT;"
        );
        if let Err(error) = connection.execute_batch(&script) {
            let _ = connection.execute_batch("ROLLBACK;");
            return Err(format!(
                "the migration from the schema version {from} to {to} failed: {}",
                describe_error(&error)
            ));
        }
    }
    Ok(())
}

/// A sentence for an error of `SQLite`, with the cases an operator meets in their own words.
pub fn describe_error(error: &rusqlite::Error) -> String {
    match error.sqlite_error_code() {
        Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => {
            "another process holds the file, so it is open in a second Golem or a tool".to_owned()
        }
        Some(ErrorCode::NotADatabase) => "the file is not a SQLite database".to_owned(),
        Some(ErrorCode::DatabaseCorrupt) => format!("the file is damaged: {error}"),
        Some(ErrorCode::CannotOpen) => format!("the file cannot be opened: {error}"),
        Some(ErrorCode::DiskFull) => format!("the disk or the file is full: {error}"),
        _ => error.to_string(),
    }
}

/// The schema of a store file as `SQLite` reports it, for the notebooks and for operators.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Description {
    /// The application id of the file.
    pub application_id: i32,
    /// The version of the schema.
    pub version: i32,
    /// The journal mode, `wal` for a store.
    pub journal_mode: String,
    /// The tables, sorted by name.
    pub tables: Vec<TableDescription>,
    /// The indexes that the schema names, sorted by name, without those `SQLite` makes for keys.
    pub indexes: Vec<String>,
}

/// One table of a [`Description`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableDescription {
    /// The name.
    pub name: String,
    /// The columns in their order.
    pub columns: Vec<ColumnDescription>,
    /// The foreign keys: the column, the table and the column it names.
    pub foreign_keys: Vec<(String, String, String)>,
    /// Whether the table is STRICT.
    pub strict: bool,
    /// The number of rows.
    pub rows: i64,
}

/// One column of a [`TableDescription`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnDescription {
    /// The name.
    pub name: String,
    /// The type, such as `TEXT`.
    pub kind: String,
    /// Whether the column refuses NULL.
    pub not_null: bool,
    /// The place of the column in the primary key, from 1, or 0.
    pub primary_key: i32,
}

/// Reads the schema of the store at `path` without changing it. A store that a running Golem
/// holds is refused, since the lock of the writer keeps every other reader out.
pub fn describe(path: &Path) -> Result<Description, StoreError> {
    let refuse = |reason: String| StoreError::Open {
        path: path.to_owned(),
        reason,
    };
    let sqlite = |error: rusqlite::Error| refuse(describe_error(&error));
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(sqlite)?;
    connection
        .busy_timeout(std::time::Duration::ZERO)
        .map_err(sqlite)?;
    let value = |name: &str| -> Result<i32, StoreError> {
        connection
            .pragma_query_value(None, name, |row| row.get(0))
            .map_err(sqlite)
    };
    let application_id = value("application_id")?;
    let version = value("user_version")?;
    let journal_mode: String = connection
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .map_err(sqlite)?;
    let names = |kind: &str| -> Result<Vec<String>, StoreError> {
        let mut statement = connection
            .prepare(
                "SELECT name FROM sqlite_schema WHERE type = ?1 AND sql IS NOT NULL \
                 AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )
            .map_err(sqlite)?;
        let rows = statement
            .query_map([kind], |row| row.get(0))
            .map_err(sqlite)?;
        rows.collect::<rusqlite::Result<_>>().map_err(sqlite)
    };
    let mut tables = Vec::new();
    for name in names("table")? {
        tables.push(describe_table(&connection, &name).map_err(sqlite)?);
    }
    Ok(Description {
        application_id,
        version,
        journal_mode,
        tables,
        indexes: names("index")?,
    })
}

fn describe_table(connection: &Connection, name: &str) -> rusqlite::Result<TableDescription> {
    let mut statement = connection
        .prepare("SELECT name, type, \"notnull\", pk FROM pragma_table_info(?1) ORDER BY cid")?;
    let columns = statement
        .query_map([name], |row| {
            Ok(ColumnDescription {
                name: row.get(0)?,
                kind: row.get(1)?,
                not_null: row.get(2)?,
                primary_key: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut statement = connection.prepare(
        "SELECT \"from\", \"table\", \"to\" FROM pragma_foreign_key_list(?1) ORDER BY id, seq",
    )?;
    let foreign_keys = statement
        .query_map([name], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let strict: bool = connection.query_row(
        "SELECT strict FROM pragma_table_list(?1) WHERE schema = 'main'",
        [name],
        |row| row.get(0),
    )?;
    // The name comes from sqlite_schema, and the quotes keep it one identifier.
    let rows = connection.query_row(
        &format!("SELECT count(*) FROM \"{}\"", name.replace('"', "\"\"")),
        [],
        |row| row.get(0),
    )?;
    Ok(TableDescription {
        name: name.to_owned(),
        columns,
        foreign_keys,
        strict,
        rows,
    })
}
