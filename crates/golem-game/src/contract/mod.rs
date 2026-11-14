// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The calls of the contract with the bridge mod, one module per file of bridge/contract/ and
 * one function per call, each of which takes the server token.
 */

//! The calls of the contract, by their area.
//!
//! A function answers what its call answers: `Ok(None)` for a miss, when the game holds no
//! object of the name or the object is of another kind, and a fault for a call that failed or
//! that was refused, since its server token was of a frame that is closed.

pub mod entities;
pub mod players;
