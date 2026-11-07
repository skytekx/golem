// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
// SPDX-FileCopyrightText: lucko (Luck) <luck@lucko.me>
// SPDX-FileCopyrightText: LuckPerms contributors
/*
 * golem-permissions: the permissions of Golem, in place of LuckPerms.
 *
 * This crate is the port of LuckPerms 5.5.71 to Golem, and it keeps the MIT license of
 * LuckPerms with the copyright of its holders, as every port of a Bukkit plugin keeps the license
 * of its upstream (ports.toml, Documentation/licensing.md section 6). The rest of Golem is under
 * the AGPL 3 only, which the MIT license combines with.
 *
 * Groups inherit from groups, nodes may end in a wildcard, a node may be denied, and a player
 * may hold nodes and groups of their own. The groups and nodes are a tracked configuration file,
 * so a new host has the same permissions as the old one. One source of truth decides every
 * question, and every decision names the rule that made it.
 *
 * The file node.rs is the syntax of a node, a group name and a UUID. The file table.rs is the
 * table of one source, which finds the most specific match of a node. The file config.rs reads
 * and validates the TOML and builds the tables and the search order of every player at load
 * time. The file resolve.rs is the order of the rules and the sentence each decision carries.
 *
 * okf/plugins/permissions.md describes the configuration and the order of the rules.
 */

//! Groups, nodes and player values of Golem, in place of `LuckPerms`.
//!
//! # The configuration
//!
//! A configuration is TOML with two tables, `groups` and `players`, and every table refuses a
//! key it does not know.
//!
//! ```toml
//! [groups.default]
//! nodes = ["essentials.home", "essentials.signs.use.*"]
//!
//! [groups.admin]
//! weight = 100
//! inherits = ["default"]
//! nodes = ["essentials.*", "golem.*"]
//!
//! [players.00000000-0000-4000-8000-000000000001]
//! groups = ["admin"]
//! nodes = ["-essentials.suicide"]
//! ```
//!
//! - A group has a `weight`, an integer that is 0 when left out, the groups it `inherits` from
//!   and its `nodes`. Its name is one segment of a node. The group [`DEFAULT_GROUP`] must exist,
//!   and every player is in it.
//! - A player is a UUID in lower case with its dashes, with the `groups` they are in besides the
//!   default group and the `nodes` of their own.
//! - A node is segments joined by dots. A segment holds `a` to `z`, `0` to `9`, `_` and `-` and
//!   starts with one of the first three, so `Essentials.Home` and `a..b` are no nodes. A node
//!   that ends in `.*` is a wildcard for every node below its prefix, `*` alone is a wildcard for
//!   every node, and a leading `-` denies what the node names instead of granting it. A false
//!   value is always written with the `-`, there is no table form.
//!
//! Loading refuses bad TOML, an unknown key, a bad group name, a missing default group, a node
//! of bad syntax, a node or a group named twice in one list, an inheritance from a group that is
//! not defined, an inheritance in a cycle, a player key that is not a UUID in lower case and a
//! group of a player that is not defined. The [`ConfigError`] names where, as a TOML path or a
//! line and a column.
//!
//! # The rules
//!
//! [`Permissions::check`] answers whether a player holds a node by these rules, in this order.
//!
//! 1. A check names a player by UUID, in either case, and one node with no wildcard and no `-`.
//!    Anything else is undefined, with a reason that says what is wrong.
//! 2. The nodes of the player come first. When one of them matches, it decides, whatever the
//!    groups say.
//! 3. Then the groups of the player, the default group among them, in ranks: a higher weight
//!    first, and at the same weight a closer group first. A group the player is in directly has
//!    depth 1, and every step of inheritance adds one, along the shortest path. The first rank in
//!    which any group matches decides.
//! 4. Within one source, the player or one group, the most specific match decides: the node
//!    itself, then the wildcard with the longest prefix, then `*`. The order of the nodes in the
//!    file plays no part.
//! 5. A wildcard `a.*` matches every node below `a` and not `a` itself, as in `LuckPerms`.
//! 6. When one source grants and denies the same node, which is an equal specificity, the deny
//!    wins.
//! 7. The groups of one rank do not outrank each other. Each decides by rules 4 to 6, and when
//!    they disagree, the deny wins.
//! 8. When no source matches, the node is undefined and [`Decision::value`] is `None`. A caller
//!    treats undefined as not granted, unless it has a default of its own.
//!
//! `LuckPerms` merges every source into one map and looks up the node itself in all of them
//! before any wildcard, so a wildcard deny of a player loses to a node that a group grants by
//! name. Golem asks the sources one by one, so the player decides first, and a group of a higher
//! rank decides before a lower one.
//!
//! # Cost
//!
//! Loading builds a table per source and the search order of every player, shared by the
//! players with the same groups. A check then reads the UUID, looks the player up and asks each
//! source at most one question per segment of the node until one matches.
//! [`Permissions::value`] allocates nothing, also for a player or a node that a check may not
//! name, which is the form for the server thread.
//! [`Permissions::check`] adds the sentence of the reason.

mod config;
mod node;
mod resolve;
mod table;

use std::{
    collections::HashMap,
    sync::Arc, //
};

use crate::{
    config::{
        Group,
        Player, //
    },
    resolve::Chain, //
};

/// The name of the group that every player is in.
pub const DEFAULT_GROUP: &str = "default";

/// A failure to read the configuration.
#[derive(Debug)]
pub struct ConfigError {
    /// What is wrong, with the group, the player or the node where there is one.
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ConfigError {}

/// The answer to whether a player holds a node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    /// True when granted, false when denied, None when no rule names the node.
    pub value: Option<bool>,
    /// The rule that decided, such as group default grants essentials.home.*.
    pub reason: String,
}

impl Decision {
    /// Answers whether the node is granted. Undefined is not.
    pub fn granted(&self) -> bool {
        self.value == Some(true)
    }
}

/// A group of a player, in the order a check searches them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Membership {
    /// The name of the group.
    pub group: String,
    /// The weight of the group.
    pub weight: i32,
    /// 1 for a group the player is in directly, one more for every step of inheritance.
    pub depth: u32,
    /// The rank of the group in the search, from 0. Groups of one rank share it.
    pub rank: u32,
}

/// The permissions of one configuration.
#[derive(Clone, Debug, Default)]
pub struct Permissions {
    /// The groups, sorted by name.
    pub(crate) groups: Vec<Group>,
    /// The index of every group by name.
    pub(crate) names: HashMap<String, usize>,
    /// The players that the configuration names, by UUID.
    pub(crate) players: HashMap<u128, Player>,
    /// The search order of a player that the configuration does not name.
    pub(crate) everyone: Arc<Chain>,
}

impl Permissions {
    /// Reads a configuration in TOML.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] that names where the configuration breaks a rule of the
    /// format, see the documentation of the crate.
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        config::load(text)
    }

    /// Whether the player with `uuid` holds `node`, with the rule that decided.
    pub fn check(&self, uuid: &str, node: &str) -> Decision {
        let outcome = resolve::resolve(self, uuid, node);
        Decision {
            value: outcome.value(),
            reason: resolve::reason(self, uuid, node, outcome),
        }
    }

    /// Whether the player with `uuid` holds `node`, as [`Decision::value`] of
    /// [`Permissions::check`], without building the reason.
    pub fn value(&self, uuid: &str, node: &str) -> Option<bool> {
        resolve::resolve(self, uuid, node).value()
    }

    /// The groups of the player with `uuid` in the order a check searches them, or `None` when
    /// `uuid` is no UUID. A player that the configuration does not name is in the default group
    /// and what it inherits.
    pub fn groups_of(&self, uuid: &str) -> Option<Vec<Membership>> {
        let id = node::parse_uuid(uuid)?;
        let chain = self
            .players
            .get(&id)
            .map_or(&self.everyone, |player| &player.chain);
        Some(
            chain
                .members
                .iter()
                .map(|member| {
                    let group = &self.groups[member.group];
                    Membership {
                        group: group.name.clone(),
                        weight: group.weight,
                        depth: member.depth,
                        rank: member.rank,
                    }
                })
                .collect(),
        )
    }

    /// The nodes of the group `name` as the configuration writes them, a deny with its `-`, or
    /// `None` when no such group exists.
    pub fn group_nodes(&self, name: &str) -> Option<&[String]> {
        let &index = self.names.get(name)?;
        Some(&self.groups[index].nodes)
    }

    /// The names of the groups, sorted.
    pub fn group_names(&self) -> impl Iterator<Item = &str> {
        self.groups.iter().map(|group| group.name.as_str())
    }
}
