// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The calls of bridge/contract/entities.toml.
 */

//! The calls of the contract that find and read entities.

use crate::{
    EntityFound,
    JniFault,
    Server,
    Uuid, //
};

/// `entityFind`: the entity with this id, or null when no level holds one.
pub fn entity_find(server: &Server<'_>, uuid: Uuid) -> Result<Option<EntityFound>, JniFault> {
    server.contract()?.entity_find(uuid)
}
