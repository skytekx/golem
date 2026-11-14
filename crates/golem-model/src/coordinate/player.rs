// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The coordinates of the type Player.
 */

//! The coordinates of `Player`.

use golem_game::contract::players;

use crate::{
    EntityRef,
    Error,
    Result,
    Server, //
};

/// `Player.allowFlight`: whether the player may fly, as in creative mode.
pub fn allow_flight(server: &Server<'_>, player: EntityRef) -> Result<bool> {
    players::player_allow_flight(server, player)?.ok_or(Error::EntityGone)
}
