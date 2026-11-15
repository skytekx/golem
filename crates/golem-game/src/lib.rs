// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * golem-game: the game as Rust names it.
 *
 * Rust never holds a Java object of the game. Every object has a small value as its name, and a
 * call of the contract with the bridge mod takes that name and answers values. The contract is
 * the list of those calls under bridge/contract/, and this crate holds one function per call,
 * the names of the objects and the server token, without which no call is made.
 *
 * okf/api.md sections 6 to 8 are the design of this crate. What stands today is its safe form:
 * a trait answers the calls, the scripted contract of the feature testing is the one thing that
 * implements it, and golem-devtool alone opens a frame. No line of the bridge mod answers a call
 * yet, and the program golem does not link this crate.
 */

//! The ids of the game, the server token and the calls of the contract with the bridge mod.
//!
//! | Module | Holds |
//! | --- | --- |
//! | `ids` | [`Uuid`] and [`EntityRef`], the names of the objects of the game |
//! | `token` | [`Frame`], [`Server`] and [`server`], the server token of the thread |
//! | `seam` | [`Contract`], what answers the calls, with [`EntityFound`] and [`JniFault`] |
//! | [`contract`] | One function per call of `bridge/contract/`, by its area |
//! | `scripted` | A contract that answers from a script, behind the feature `testing` |
//!
//! A call of the contract takes the server token. [`server`] answers the token of the innermost
//! [`Frame`] that is open on the calling thread, so a thread that holds no frame gets no token
//! and makes no call. A call is answered in the frame of its token alone, so the token of an
//! outer frame stays good while a frame is open inside it, and a call with a token whose frame
//! is closed is refused. In the design a frame stands for one native call of the bridge mod on
//! the thread of a level. Today golem-devtool alone opens a frame, over the scripted contract,
//! and no call reaches a JVM.
//!
//! The crate holds no test, which is the rule of CONTRIBUTING.md section 5. The notebook
//! `tests/notebooks/20_api_contract.ipynb` drives the calls through golem-devtool.

pub mod contract;
mod ids;
#[cfg(feature = "testing")]
pub mod scripted;
mod seam;
mod token;

pub use crate::{
    ids::{
        EntityRef,
        Uuid, //
    },
    seam::{
        Contract,
        EntityFound,
        JniFault, //
    },
    token::{
        Frame,
        NotOnServerThread,
        Server,
        server, //
    },
};
