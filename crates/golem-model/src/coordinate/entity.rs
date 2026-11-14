// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The coordinates of the type Entity, and the lookup of an entity by its id.
 */

//! The coordinates of `Entity`.

use golem_game::contract::entities;

use crate::{
    EntityRef,
    Result,
    Server,
    Uuid, //
};

/// The object type that an entity answers as in the schema of okf/api.md, by the names of its
/// section 8.3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A player.
    Player,
    /// An entity of no more specific type.
    EntityObject,
}

/// An entity that [`find`] found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Found {
    /// The name of the entity.
    pub entity: EntityRef,
    /// The object type that it answers as.
    pub kind: Kind,
}

/// `Query.entity(id:)`: the entity with this id, or null when no level holds one.
pub fn find(server: &Server<'_>, uuid: Uuid) -> Result<Option<Found>> {
    Ok(entities::entity_find(server, uuid)?.map(|found| Found {
        entity: EntityRef::new(found.id, found.level, uuid),
        kind: if found.player {
            Kind::Player
        } else {
            Kind::EntityObject
        },
    }))
}
