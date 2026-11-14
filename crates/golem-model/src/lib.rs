// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * golem-model: the meaning of the coordinates of the API of Golem.
 *
 * A coordinate of the API has one function here that holds its meaning, so that every way into
 * the game runs the same code and two ways in cannot drift apart. A function of a coordinate
 * that lives in a level takes the server token.
 *
 * okf/api.md sections 1, 6 and 7 are the design of this crate. Today golem-devtool is the one
 * caller of these functions: no schema stands above them yet, and the program golem does not
 * link this crate.
 */

//! One function per coordinate of the API of Golem.
//!
//! | Module | Holds |
//! | --- | --- |
//! | [`coordinate`] | The functions, one module per type of the schema |
//! | `error` | [`Error`], what a coordinate answers in place of a value, with its code |
//!
//! The names of the objects and the server token are those of golem-game, which this crate
//! hands on, so a crate above it names golem-model alone.
//!
//! The crate holds no test, which is the rule of CONTRIBUTING.md section 5. The notebook
//! `tests/notebooks/20_api_contract.ipynb` drives the functions through golem-devtool.

#![forbid(unsafe_code)]

pub mod coordinate;
mod error;

pub use golem_game::{
    EntityRef,
    Server,
    Uuid, //
};

pub use crate::error::{
    Error,
    Result, //
};

/// Answers the server token of this thread, or [`Error::NotOnServerThread`] on a thread that
/// holds none.
pub fn server<'t>() -> Result<Server<'t>> {
    Ok(golem_game::server()?)
}
