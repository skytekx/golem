// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The calls of bridge/contract/players.toml.
 */

//! The calls of the contract that read and change players.

use crate::{
    EntityRef,
    JniFault,
    Server, //
};

/// `playerAllowFlight`: whether the player may fly, as in creative mode.
pub fn player_allow_flight(
    server: &Server<'_>,
    player: EntityRef,
) -> Result<Option<bool>, JniFault> {
    server.contract()?.player_allow_flight(player)
}
