// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The error of golem-model: what a coordinate answers in place of a value, each with the code
 * that a client of the API reads.
 */

use std::{
    error,
    fmt, //
};

use golem_game::{
    JniFault,
    NotOnServerThread, //
};

/// What a coordinate answers in place of a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The game holds no object of the name and of the kind that the coordinate was asked
    /// about: the entity is gone, or it is of another kind, as a zombie that is asked as a
    /// player.
    EntityGone,
    /// The coordinate lives in a level, and the calling thread holds no server token, or the
    /// token of the call was of a frame that is closed.
    NotOnServerThread,
    /// The call of the contract failed: the game threw, or its answer did not read.
    Fault(JniFault),
}

impl Error {
    /// Answers the code of the error, which a client of the API reads.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EntityGone => "ENTITY_GONE",
            Self::NotOnServerThread => "NOT_ON_SERVER_THREAD",
            Self::Fault(_) => "GAME_FAULT",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntityGone => formatter.write_str("The entity is gone or is of another kind."),
            Self::NotOnServerThread => fmt::Display::fmt(&NotOnServerThread, formatter),
            Self::Fault(fault) => write!(formatter, "The game gave no answer: {fault}"),
        }
    }
}

impl error::Error for Error {}

impl From<NotOnServerThread> for Error {
    fn from(_: NotOnServerThread) -> Self {
        Self::NotOnServerThread
    }
}

impl From<JniFault> for Error {
    fn from(fault: JniFault) -> Self {
        if fault.is_refusal() {
            Self::NotOnServerThread
        } else {
            Self::Fault(fault)
        }
    }
}

/// The answer of a coordinate, whose error is [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
