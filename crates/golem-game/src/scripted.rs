// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The scripted contract of golem-game: a contract that answers from a list of entities, in
 * place of the game, so that a notebook judges the calls and the coordinate functions above
 * them without a JVM. It stands behind the feature testing.
 */

//! A contract that answers from a script.
//!
//! The scripted contract holds the entities that a script names. It is the one thing that
//! implements the seam today, so what a call answers here is the rule of the script, which the
//! bridge mod has to meet once it answers: an entity is found by its UUID alone, whatever
//! runtime id and level its name carries, a call misses for a UUID that no entity carries and
//! for a call that asks a player of an entity that is none, and a call faults for an entity
//! whose script names a fault. The contract counts the calls it answered, so a notebook holds
//! a coordinate to the calls it may make.

use std::{
    cell::Cell,
    collections::HashSet, //
};

use crate::{
    Contract,
    EntityFound,
    EntityRef,
    JniFault,
    Uuid, //
};

/// An entity of a scripted contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptedEntity {
    /// The name of the entity.
    pub entity: EntityRef,
    /// Whether the entity is a player.
    pub player: bool,
    /// Whether the player may fly, which a call reads of a player alone.
    pub allow_flight: bool,
    /// What every call that reads the entity throws, or `None` for an entity that answers.
    pub fault: Option<String>,
}

/// A contract that answers from the entities of a script.
#[derive(Debug, Default)]
pub struct Scripted {
    entities: Vec<ScriptedEntity>,
    calls: Cell<u64>,
}

impl Scripted {
    /// The contract that answers for `entities`, or the UUID that two of them carry, since two
    /// names with one UUID are one entity.
    pub fn new(entities: Vec<ScriptedEntity>) -> Result<Self, Uuid> {
        let mut names = HashSet::new();
        match entities
            .iter()
            .find(|scripted| !names.insert(scripted.entity))
        {
            Some(twice) => Err(twice.entity.uuid()),
            None => Ok(Self {
                entities,
                calls: Cell::new(0),
            }),
        }
    }

    /// Answers how many calls the contract answered so far.
    pub fn calls(&self) -> u64 {
        self.calls.get()
    }

    /// Counts one call, and answers the entity with the UUID `uuid`, a fault for an entity
    /// whose script names one, or `None` for a UUID that no entity carries.
    fn read(&self, uuid: Uuid) -> Result<Option<&ScriptedEntity>, JniFault> {
        self.calls.set(self.calls.get() + 1);
        let found = self
            .entities
            .iter()
            .find(|scripted| scripted.entity.uuid() == uuid);
        match found.and_then(|scripted| scripted.fault.as_deref()) {
            Some(fault) => Err(JniFault::new(fault)),
            None => Ok(found),
        }
    }
}

impl Contract for Scripted {
    fn entity_find(&self, uuid: Uuid) -> Result<Option<EntityFound>, JniFault> {
        Ok(self.read(uuid)?.map(|scripted| EntityFound {
            id: scripted.entity.id(),
            level: scripted.entity.level(),
            player: scripted.player,
        }))
    }

    fn player_allow_flight(&self, player: EntityRef) -> Result<Option<bool>, JniFault> {
        Ok(self
            .read(player.uuid())?
            .filter(|scripted| scripted.player)
            .map(|scripted| scripted.allow_flight))
    }
}
