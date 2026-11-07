// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The store of the plugins in golem run: the file golem-store.db in the server directory, beside
 * world/, which golem run opens before it installs the plugins and refuses to start without, the
 * seed of the permissions on a store that holds no group, and the close after the JVM ended,
 * whether golem_jvm::run returned or the JVM ended through System.exit and its exit hook.
 *
 * okf/store.md describes these steps in its section on the place in the program golem.
 */

use std::{
    io::ErrorKind,
    path::Path, //
};

use golem_store::{
    PermissionSet,
    Store,
    StoreError, //
};

use crate::log;

/// The file of the store in the server directory, beside `world/`.
pub const STORE_FILE: &str = "golem-store.db";

/// The seed of the permissions in the server directory.
pub const SEED_FILE: &str = "config/permissions.toml";

/// The seed of the permissions of the Golem repository, for a server without one of its own.
const GOLEM_SEED: &str = include_str!("../../../config/permissions.toml");

/// Opens the store of the server directory, or answers why it does not open, which stops the
/// start before the JVM exists.
pub fn open(server: &Path) -> Result<Store, String> {
    let path = server.join(STORE_FILE);
    Store::open(&path).map_err(|error| match error {
        StoreError::Open { .. } => error.to_string(),
        other => format!("cannot open the store {}: {other}", path.display()),
    })
}

/// Seeds the permissions of a store that holds no group, from `config/permissions.toml` of the
/// server directory, or from the seed of the Golem repository when the server has no such file,
/// and answers the line that says which, or `None` for a store that holds a group. golem run
/// logs the line behind its banner, which stays the first line of golem. A seed that breaks a
/// rule of `golem-permissions` stops the start, since a server without its permissions would
/// deny every player every command, and so does a seed that the store cannot write into its file,
/// since the line would then name a seed that the file does not hold.
pub fn seed(store: &Store, server: &Path) -> Result<Option<String>, String> {
    if !store.permissions().groups.is_empty() {
        return Ok(None);
    }
    let file = server.join(SEED_FILE);
    let (text, from) = match std::fs::read_to_string(&file) {
        Ok(text) => (text, format!("{SEED_FILE} of the server")),
        Err(error) if error.kind() == ErrorKind::NotFound => (
            GOLEM_SEED.to_owned(),
            format!("the seed of the Golem repository, since the server has no {SEED_FILE}"),
        ),
        Err(error) => return Err(format!("cannot read the seed {}: {error}", file.display())),
    };
    golem_permissions::Permissions::from_toml(&text)
        .map_err(|error| format!("the permissions of {from} do not load: {error}"))?;
    let set = PermissionSet::from_toml(&text)
        .map_err(|error| format!("the permissions of {from} do not load: {error}"))?;
    store.replace_permissions(set).map_err(|error| {
        format!(
            "the store {} does not take the permissions of {from}: {error}",
            store.path().display()
        )
    })?;
    // The seed is in the file before its line is logged and before a server runs on it, so a disk
    // that takes no write stops the start here.
    store.flush().map_err(|error| {
        format!(
            "the store {} did not write the permissions of {from}: {error}",
            store.path().display()
        )
    })?;
    Ok(Some(format!(
        "the store {} held no group of the permissions, so golem seeded them from {from}",
        store.path().display()
    )))
}

/// Closes the store and answers whether every change reached the file, with a line on the log.
///
/// The exit hook of the JVM calls this beside the main path of golem run, and every close after
/// the first answers as the first, so the line names the outcome once per caller. It logs to the
/// standard error alone, since the exit hook runs while the threads of Java wait at a safepoint.
pub fn close(store: &Store, why: &str) -> bool {
    match store.close() {
        Ok(()) => {
            log(format_args!(
                "{why}, so golem closed the store {} with every change in the file",
                store.path().display()
            ));
            true
        }
        Err(error) => {
            log(format_args!(
                "{why}, and the store {} did not close cleanly: {error}",
                store.path().display()
            ));
            false
        }
    }
}
