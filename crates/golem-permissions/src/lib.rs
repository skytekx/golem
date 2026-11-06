// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * golem-permissions: the permissions of Golem, in place of LuckPerms.
 *
 * Groups inherit from groups, nodes may end in a wildcard, a node may be denied, and a player
 * may hold nodes and groups of their own. The groups and nodes are a tracked configuration file,
 * so a new host has the same permissions as the old one. One source of truth decides every
 * question, and every decision names the rule that made it.
 *
 * okf/plugins/permissions.md describes the configuration and the order of the rules.
 */

//! Groups, nodes and player values of Golem, in place of LuckPerms.

/// A failure to read the configuration.
#[derive(Debug)]
pub struct ConfigError {
    /// What is wrong, with the group, the player or the node where there is one.
    pub message: String,
}

/// The answer to whether a player holds a node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    /// True when granted, false when denied, None when no rule names the node.
    pub value: Option<bool>,
    /// The rule that decided, such as group default grants essentials.home.*.
    pub reason: String,
}

/// The permissions of one configuration.
#[derive(Clone, Debug, Default)]
pub struct Permissions {}

impl Permissions {
    /// Reads a configuration in TOML.
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        let _ = text;
        todo!("golem-permissions: Permissions::from_toml")
    }

    /// Whether the player with `uuid` holds `node`.
    pub fn check(&self, uuid: &str, node: &str) -> Decision {
        let _ = (uuid, node);
        todo!("golem-permissions: Permissions::check")
    }
}
