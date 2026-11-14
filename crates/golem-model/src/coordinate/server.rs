// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The coordinates of the type Server.
 */

//! The coordinates of `Server`.

use crate::Result;

/// `Server.bukkitVersion`: the version of the Bukkit API whose equivalent the schema holds.
///
/// The coordinate is safe on any thread, so it takes no server token. It answers a `Result`
/// as every coordinate function does, and it never fails.
pub fn bukkit_version() -> Result<&'static str> {
    Ok("1.20.1-R0.1-SNAPSHOT")
}
