// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
// SPDX-FileCopyrightText: lucko (Luck) <luck@lucko.me>
// SPDX-FileCopyrightText: LuckPerms contributors
/*
 * The resolution of golem-permissions: which source decides whether a player holds a node, in
 * the order of the rules that lib.rs documents, without a string built on the way.
 */

use crate::{
    Permissions,
    node,
    table::Hit, //
};

/// A group of a player in the search order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Member {
    /// The index of the group.
    pub group: usize,
    /// One for a group the player is in directly, the default group included, and one more
    /// for every step of inheritance, along the shortest path.
    pub depth: u32,
    /// The position of the weight and depth of the group in the search order, from zero.
    pub rank: u32,
}

/// The groups of a player in the order a check searches them, built at load time.
#[derive(Clone, Debug, Default)]
pub(crate) struct Chain {
    pub members: Box<[Member]>,
}

/// What decided a check, borrowed from the node it named.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome<'a> {
    /// The player is not a UUID.
    BadPlayer,
    /// The node is not a node a check may name.
    BadNode,
    /// No source names the node.
    Undefined,
    /// The nodes of the player decided.
    Player(Hit<'a>),
    /// A group decided. The rival is a group of the same rank that said the opposite.
    Group {
        group: usize,
        hit: Hit<'a>,
        rival: Option<(usize, Hit<'a>)>,
    },
}

impl Outcome<'_> {
    /// True for a grant, false for a deny, None when nothing decided.
    pub fn value(self) -> Option<bool> {
        match self {
            Self::Player(hit) | Self::Group { hit, .. } => Some(hit.value.granted()),
            Self::BadPlayer | Self::BadNode | Self::Undefined => None,
        }
    }
}

/// Resolves whether the player with `uuid` holds `node`.
pub(crate) fn resolve<'a>(permissions: &Permissions, uuid: &str, node: &'a str) -> Outcome<'a> {
    let Some(id) = node::parse_uuid(uuid) else {
        return Outcome::BadPlayer;
    };
    if node::check_node(node).is_err() {
        return Outcome::BadNode;
    }
    let (own, chain) = match permissions.players.get(&id) {
        Some(player) => (Some(&player.table), &*player.chain),
        None => (None, &*permissions.everyone),
    };
    if let Some(hit) = own.and_then(|table| table.find(node)) {
        return Outcome::Player(hit);
    }
    resolve_groups(permissions, chain, node)
}

/// Searches the groups rank by rank. The first rank in which any group matches decides, and
/// when the groups of that rank disagree, the deny wins.
fn resolve_groups<'a>(permissions: &Permissions, chain: &Chain, node: &'a str) -> Outcome<'a> {
    let members = &chain.members;
    let mut start = 0;
    while start < members.len() {
        let rank = members[start].rank;
        let end = members[start..]
            .iter()
            .position(|member| member.rank != rank)
            .map_or(members.len(), |offset| start + offset);
        let mut decided: Option<(usize, Hit<'a>)> = None;
        let mut rival: Option<(usize, Hit<'a>)> = None;
        for member in &members[start..end] {
            let Some(hit) = permissions.groups[member.group].table.find(node) else {
                continue;
            };
            match decided {
                None => decided = Some((member.group, hit)),
                Some((_, current)) if current.value.granted() && !hit.value.granted() => {
                    rival = decided;
                    decided = Some((member.group, hit));
                }
                Some((_, current))
                    if current.value.granted() != hit.value.granted() && rival.is_none() =>
                {
                    rival = Some((member.group, hit));
                }
                Some(_) => {}
            }
        }
        if let Some((group, hit)) = decided {
            return Outcome::Group { group, hit, rival };
        }
        start = end;
    }
    Outcome::Undefined
}

/// The sentence that names the rule behind an outcome.
pub(crate) fn reason(
    permissions: &Permissions,
    uuid: &str,
    node: &str,
    outcome: Outcome<'_>,
) -> String {
    match outcome {
        Outcome::BadPlayer => format!("The player {uuid:?} is not a UUID"),
        Outcome::BadNode => {
            let why = node::check_node(node).err().unwrap_or_default();
            format!("The node {node:?} {why}")
        }
        Outcome::Undefined => format!("No rule names {node}"),
        Outcome::Player(hit) => format!("The player {}", says(hit)),
        Outcome::Group { group, hit, rival } => {
            let name = &permissions.groups[group].name;
            match rival {
                None => format!("Group {name} {}", says(hit)),
                // The rival is always the grant, since a deny of the rank takes the decision.
                Some((other, against)) => format!(
                    "Group {name} denies {} and group {} of the same rank grants {}, and the deny wins",
                    hit.pattern, permissions.groups[other].name, against.pattern,
                ),
            }
        }
    }
}

/// What a source says, as the end of a sentence that names it.
fn says(hit: Hit<'_>) -> String {
    let pattern = hit.pattern;
    match (hit.value.grant, hit.value.deny) {
        (true, true) => format!("grants and denies {pattern}, and the deny wins"),
        (false, true) => format!("denies {pattern}"),
        _ => format!("grants {pattern}"),
    }
}
