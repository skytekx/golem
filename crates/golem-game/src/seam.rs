// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The seam of golem-game: what answers the calls of the contract. The functions of the module
 * contract are the calls, and each hands its arguments to the one method of this trait that
 * carries its name. The scripted contract of the feature testing is the one thing that
 * implements the trait today.
 */

use std::{
    error,
    fmt, //
};

use crate::{
    EntityRef,
    Uuid, //
};

/// A call of the contract that gave no answer: the game threw, its answer did not read, or the
/// call was refused and never made, since its server token was of a frame that is closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JniFault {
    message: String,
    refused: bool,
}

impl JniFault {
    /// The fault that `message` describes, such as the class and the message of what the
    /// game threw.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            refused: false,
        }
    }

    /// The fault of a call that was refused and never made, since its server token was of a
    /// frame that is closed.
    pub(crate) fn refused() -> Self {
        Self {
            message: "The server token of the call is of a frame that is closed.".to_owned(),
            refused: true,
        }
    }

    /// Answers what the fault describes.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Answers whether the call was refused and never made, since its server token was of a
    /// frame that is closed.
    pub const fn is_refusal(&self) -> bool {
        self.refused
    }
}

impl fmt::Display for JniFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl error::Error for JniFault {}

/// What the call `entityFind` answers for an entity it found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntityFound {
    /// The runtime id of the entity.
    pub id: i32,
    /// The index of the level that the entity is on.
    pub level: u16,
    /// Whether the entity is a player.
    pub player: bool,
}

/// What answers the calls of the contract, one method per call of `bridge/contract/`.
///
/// A method takes the arguments of its call and answers what the call answers: `Ok(None)` for
/// a miss, when the game holds no object of that name or the object is of another kind, and a
/// [`JniFault`] when the call failed.
pub trait Contract {
    /// The call `entityFind` of `bridge/contract/entities.toml`.
    fn entity_find(&self, uuid: Uuid) -> Result<Option<EntityFound>, JniFault>;

    /// The call `playerAllowFlight` of `bridge/contract/players.toml`.
    fn player_allow_flight(&self, player: EntityRef) -> Result<Option<bool>, JniFault>;
}
