// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The ids of golem-game: the small values that name an object of the game in Rust and across
 * the JNI, as okf/api.md section 8.1 lists them.
 */

use std::{
    fmt,
    hash::{
        Hash,
        Hasher, //
    },
};

/// The lengths of the five groups of a UUID in its usual form, in hexadecimal digits.
const GROUPS: [usize; 5] = [8, 4, 4, 4, 12];

/// A UUID, the id of an entity or of a profile, as the two halves that cross the JNI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Uuid {
    most: u64,
    least: u64,
}

impl Uuid {
    /// The UUID of its two halves, the most significant 64 bits and the least significant.
    pub const fn from_halves(most: u64, least: u64) -> Self {
        Self { most, least }
    }

    /// Answers the most significant 64 bits.
    pub const fn most(self) -> u64 {
        self.most
    }

    /// Answers the least significant 64 bits.
    pub const fn least(self) -> u64 {
        self.least
    }

    /// Reads a UUID in its usual form, five groups of hexadecimal digits joined by dashes, in
    /// any case, or answers `None` for any other text.
    pub fn parse(text: &str) -> Option<Self> {
        let mut value: u128 = 0;
        let mut groups = text.split('-');
        for length in GROUPS {
            let group = groups.next()?;
            if group.len() != length {
                return None;
            }
            for digit in group.chars() {
                value = (value << 4) | u128::from(digit.to_digit(16)?);
            }
        }
        if groups.next().is_some() {
            return None;
        }
        let most = u64::try_from(value >> 64).ok()?;
        let least = u64::try_from(value & u128::from(u64::MAX)).ok()?;
        Some(Self { most, least })
    }
}

impl fmt::Display for Uuid {
    /// Writes the UUID in its usual form, in lower case with dashes.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            self.most >> 32,
            (self.most >> 16) & 0xffff,
            self.most & 0xffff,
            self.least >> 48,
            self.least & 0xffff_ffff_ffff
        )
    }
}

/// The name of an entity, a player included.
///
/// The UUID is the name. The runtime id and the level are hints for the lookup, since an
/// entity that changes its dimension gets a new runtime id and keeps its UUID, as okf/api.md
/// section 8.1 says. So two names are equal, and hash alike, when their UUIDs are equal.
#[derive(Clone, Copy, Debug)]
pub struct EntityRef {
    id: i32,
    level: u16,
    uuid: Uuid,
}

impl PartialEq for EntityRef {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl Eq for EntityRef {}

impl Hash for EntityRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

impl EntityRef {
    /// The name of the entity with the runtime id `id` on the level `level` and the UUID
    /// `uuid`.
    pub const fn new(id: i32, level: u16, uuid: Uuid) -> Self {
        Self { id, level, uuid }
    }

    /// Answers the runtime id of the entity, which the game gives it for one run.
    pub const fn id(self) -> i32 {
        self.id
    }

    /// Answers the index of the level the entity was last seen on.
    pub const fn level(self) -> u16 {
        self.level
    }

    /// Answers the UUID of the entity, the same across restarts and dimensions.
    pub const fn uuid(self) -> Uuid {
        self.uuid
    }
}
