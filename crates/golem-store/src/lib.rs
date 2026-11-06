// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * golem-store: the store of Golem, the data of the plugins in one SQLite file beside the world.
 *
 * The plugins call the store on the server thread, where a call has a budget of 1 ms, so no
 * call there touches the disk. The store loads the whole file into a cache when it opens, every
 * read answers from the cache, and every change goes into the cache and onto the queue of a
 * writer thread that owns the file and commits the changes in their order. The file is in the
 * journal mode WAL with full syncs, so a crash at any moment leaves it with every change up to
 * some point and none after it, at worst without the last changes before the crash.
 *
 * The store is Golem's own code under the AGPL 3 only. The ports of EssentialsX, EssentialsXSpawn
 * and LuckPerms use it and keep the licenses of their upstreams, which combine with it under
 * section 13 of the GPL 3 and the AGPL 3.
 *
 * The file model.rs holds the records and the rules of a value, schema.rs the schema and its
 * migrations, cache.rs the cache, writer.rs the writer thread, store.rs the handle the plugins
 * share, permissions.rs the permissions as the TOML of golem-permissions, and compare.rs the
 * reads of the writer, the check of a file and the log of the accounts.
 */

//! The store of Golem: players, homes, warps, spawns, mail, ignore lists, accounts with their
//! log, the permissions and settings, in one `SQLite` file.
//!
//! # Use
//!
//! ```no_run
//! use golem_store::{Location, Store, Uuid};
//!
//! let store = Store::open("golem-store.db")?;
//! let steve = Uuid::parse("00000000-0000-4000-8000-000000000001").unwrap();
//! store.join(steve, "Steve", 1_700_000_000_000)?;
//! let home = Location {
//!     dimension: "minecraft:overworld".to_owned(),
//!     x: 10.5,
//!     y: 64.0,
//!     z: -3.5,
//!     yaw: 90.0,
//!     pitch: 0.0,
//! };
//! store.set_home(steve, "base", home)?;
//! store.create_account(steve, 10_000, 1_700_000_000_000, "starting balance")?;
//! let balance = store.balance(steve);
//! store.close()?;
//! # Ok::<(), golem_store::StoreError>(())
//! ```
//!
//! # Threads
//!
//! | Call | Where | Waits for the disk |
//! | --- | --- | --- |
//! | Every read, such as [`Store::player`] and [`Store::balance`] | Any thread, the server thread first | No |
//! | Every change, such as [`Store::set_home`] and [`Store::transfer`] | Any thread, the server thread first | No |
//! | [`Store::flush`], [`Store::close`], [`Store::check`], [`Store::ledger`] | Not the server thread | Yes |
//! | [`Store::open`] | The start of the program, before the server thread runs | Yes |
//!
//! A change is checked against the cache and refused with a [`Refusal`] before anything changes,
//! or it changes the cache and reaches the file later, in the order the store took it. The file
//! holds every change once [`Store::flush`] or [`Store::close`] answers. okf/store.md describes
//! the schema, the threads, the durability and the API, and the notebook
//! `tests/notebooks/09_store.ipynb` holds the store to it through golem-devtool. The crate holds
//! no test, which is the rule of CONTRIBUTING.md section 5.

mod cache;
mod compare;
mod model;
mod permissions;
mod schema;
mod store;
mod writer;

pub use crate::{
    model::{
        Location,
        MAX_NAME_CHARS,
        MAX_NODE_BYTES,
        MAX_TEXT_BYTES,
        Mail,
        PermissionGroup,
        Player,
        PlayerPermissions,
        Receipt,
        Refusal,
        StoreError,
        Transfer,
        Uuid,
        Warp, //
    },
    permissions::PermissionSet,
    schema::{
        APPLICATION_ID,
        ColumnDescription,
        Description,
        SCHEMA_VERSION,
        TableDescription,
        describe, //
    },
    store::{
        Check,
        DEFAULT_GROUP,
        Joined,
        Stats,
        Store, //
    },
    writer::MAX_BATCH,
};

/// Writes one line to the log of the server, the standard error, behind the prefix
/// `[golem] store:`. A line that the standard error does not take is lost.
pub(crate) fn log(message: std::fmt::Arguments<'_>) {
    use std::io::Write as _;

    let _ = writeln!(std::io::stderr().lock(), "[golem] store: {message}");
}
