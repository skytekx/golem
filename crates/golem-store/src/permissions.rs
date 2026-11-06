// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The permissions of the store as one set, and its form as the TOML of config/permissions.toml.
 *
 * golem-permissions reads that TOML into its resolver, so the store hands its set to the
 * resolver as text and takes the seed file the same way, without either crate naming the other.
 * The store keeps the set as it is given and checks only what keeps its rows whole: every name
 * and node is one token of check_token, no list names an entry twice, and every group a list
 * names exists. golem-permissions judges the rest, the syntax of a node, the group default and
 * the cycles of inheritance, when it loads the text, so a port loads a changed set into a
 * resolver before it writes the change.
 */

use std::collections::{
    BTreeMap,
    BTreeSet, //
};

use serde::{
    Deserialize,
    Serialize, //
};

use crate::model::{
    PermissionGroup,
    PlayerPermissions,
    Refusal,
    Uuid,
    check_token, //
};

/// The groups and the players of the permissions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PermissionSet {
    /// The groups, sorted by name.
    pub groups: Vec<PermissionGroup>,
    /// The players with groups or nodes of their own, sorted by UUID.
    pub players: Vec<PlayerPermissions>,
}

/// The file as TOML holds it, the format of golem-permissions.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    groups: BTreeMap<String, GroupFile>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    players: BTreeMap<String, PlayerFile>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct GroupFile {
    #[serde(default, skip_serializing_if = "is_zero")]
    weight: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    inherits: Vec<String>,
    #[serde(default)]
    nodes: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PlayerFile {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    groups: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    nodes: Vec<String>,
}

/// The refusal of a player key that is no UUID, in the words of golem-permissions.
const NOT_A_UUID: &str = "the key is not a UUID in lower case with its four dashes, such as 00000000-0000-4000-8000-000000000001";

/// A key as TOML writes it in a path, bare when it can be and quoted otherwise, as
/// golem-permissions names it in a message.
fn toml_key(text: &str) -> String {
    let bare = !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
    if bare {
        text.to_owned()
    } else {
        format!("{text:?}")
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero(weight: &i32) -> bool {
    *weight == 0
}

impl PermissionSet {
    /// Reads a set from the TOML of `config/permissions.toml`. It checks the shape of the file
    /// and that every player is a UUID in lower case with its four dashes, the key that
    /// golem-permissions takes, with the message golem-permissions gives, and leaves the rest to
    /// golem-permissions.
    pub fn from_toml(text: &str) -> Result<Self, String> {
        let file: File = toml::from_str(text).map_err(|error| error.message().to_owned())?;
        let groups = file
            .groups
            .into_iter()
            .map(|(name, group)| PermissionGroup {
                name,
                weight: group.weight,
                parents: group.inherits,
                nodes: group.nodes,
            })
            .collect();
        let mut players = Vec::with_capacity(file.players.len());
        for (key, player) in file.players {
            // Uuid::parse takes either case, as the calls of the store do, but a key of the file
            // in upper case is refused by golem-permissions, and two spellings of one player
            // would become two entries of the set.
            let uuid = Uuid::parse(&key)
                .filter(|_| !key.bytes().any(|byte| byte.is_ascii_uppercase()))
                .ok_or_else(|| format!("players.{}: {NOT_A_UUID}", toml_key(&key)))?;
            players.push(PlayerPermissions {
                uuid,
                groups: player.groups,
                nodes: player.nodes,
            });
        }
        players.sort_by_key(|player| player.uuid);
        Ok(Self { groups, players })
    }

    /// Writes the set as the TOML of `config/permissions.toml`, which
    /// `golem_permissions::Permissions::from_toml` reads.
    pub fn to_toml(&self) -> String {
        let file = File {
            groups: self
                .groups
                .iter()
                .map(|group| {
                    let entry = GroupFile {
                        weight: group.weight,
                        inherits: group.parents.clone(),
                        nodes: group.nodes.clone(),
                    };
                    (group.name.clone(), entry)
                })
                .collect(),
            players: self
                .players
                .iter()
                .map(|player| {
                    let entry = PlayerFile {
                        groups: player.groups.clone(),
                        nodes: player.nodes.clone(),
                    };
                    (player.uuid.to_string(), entry)
                })
                .collect(),
        };
        // A map of strings to tables of strings, numbers and lists always serializes.
        toml::to_string_pretty(&file).unwrap_or_default()
    }

    /// Checks the set as the store takes it: names and nodes of [`check_token`], no entry twice
    /// in a list, and every group a group or a player names in the set.
    pub(crate) fn check(&self) -> Result<(), Refusal> {
        let mut names = BTreeSet::new();
        for group in &self.groups {
            check_group(group)?;
            if !names.insert(group.name.as_str()) {
                return Err(Refusal::Twice {
                    what: "the groups of the permissions".to_owned(),
                    entry: group.name.clone(),
                });
            }
        }
        for group in &self.groups {
            if let Some(parent) = group
                .parents
                .iter()
                .find(|parent| !names.contains(parent.as_str()))
            {
                return Err(Refusal::UnknownGroup(parent.clone()));
            }
        }
        let mut players = BTreeSet::new();
        for player in &self.players {
            check_player(player)?;
            if !players.insert(player.uuid) {
                return Err(Refusal::Twice {
                    what: "the players of the permissions".to_owned(),
                    entry: player.uuid.to_string(),
                });
            }
            if let Some(group) = player
                .groups
                .iter()
                .find(|group| !names.contains(group.as_str()))
            {
                return Err(Refusal::UnknownGroup(group.clone()));
            }
        }
        Ok(())
    }
}

/// Checks a group on its own: its name, its parents and its nodes, each once.
pub(crate) fn check_group(group: &PermissionGroup) -> Result<(), Refusal> {
    check_token("the name of a group", &group.name)?;
    check_list(
        "a parent of a group",
        &format!("the parents of the group {}", group.name),
        &group.parents,
    )?;
    check_list(
        "a node of a group",
        &format!("the nodes of the group {}", group.name),
        &group.nodes,
    )
}

/// Checks a player of the permissions on their own: their groups and nodes, each once.
pub(crate) fn check_player(player: &PlayerPermissions) -> Result<(), Refusal> {
    check_list(
        "a group of a player",
        &format!("the groups of the player {}", player.uuid),
        &player.groups,
    )?;
    check_list(
        "a node of a player",
        &format!("the nodes of the player {}", player.uuid),
        &player.nodes,
    )
}

fn check_list(what: &'static str, list_name: &str, list: &[String]) -> Result<(), Refusal> {
    let mut seen = BTreeSet::new();
    for entry in list {
        check_token(what, entry)?;
        if !seen.insert(entry.as_str()) {
            return Err(Refusal::Twice {
                what: list_name.to_owned(),
                entry: entry.clone(),
            });
        }
    }
    Ok(())
}
