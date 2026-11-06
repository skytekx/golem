// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The configuration of golem-permissions: the TOML file, its strict validation and the tables it
 * becomes, so that a check finds everything it needs built at load time.
 */

use std::{
    collections::{
        BTreeMap,
        HashMap,
        HashSet,
        VecDeque, //
    },
    sync::Arc, //
};

use serde::Deserialize;

use crate::{
    ConfigError,
    DEFAULT_GROUP,
    Permissions,
    node,
    resolve::{
        Chain,
        Member, //
    },
    table::Table, //
};

/// The file as TOML holds it. Every table refuses a key it does not know.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    groups: BTreeMap<String, GroupFile>,
    #[serde(default)]
    players: BTreeMap<String, PlayerFile>,
}

/// What a player key that is not a UUID gets told.
const NOT_A_UUID: &str = "the key is not a UUID in lower case with its four dashes, such as 00000000-0000-4000-8000-000000000001";

/// A table `[groups.<name>]`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupFile {
    #[serde(default)]
    weight: i32,
    #[serde(default)]
    inherits: Vec<String>,
    #[serde(default)]
    nodes: Vec<String>,
}

/// A table `[players.<uuid>]`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayerFile {
    #[serde(default)]
    groups: Vec<String>,
    #[serde(default)]
    nodes: Vec<String>,
}

/// A group as the checks read it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Group {
    pub name: String,
    pub weight: i32,
    pub inherits: Vec<usize>,
    /// The nodes as the file writes them, for the API.
    pub nodes: Vec<String>,
    pub table: Table,
}

/// A player that the file names, as the checks read them.
#[derive(Clone, Debug, Default)]
pub(crate) struct Player {
    pub table: Table,
    pub chain: Arc<Chain>,
}

/// Reads and validates a configuration.
pub(crate) fn load(text: &str) -> Result<Permissions, ConfigError> {
    let file: File = toml::from_str(text).map_err(|error| syntax_error(text, &error))?;
    let names = group_names(&file.groups)?;
    let Some(&default) = names.get(DEFAULT_GROUP) else {
        return Err(error(
            &format!("groups.{DEFAULT_GROUP}"),
            "the group is missing, and every player is in it",
        ));
    };
    let mut groups = Vec::with_capacity(file.groups.len());
    for (name, group) in &file.groups {
        groups.push(Group {
            name: name.clone(),
            weight: group.weight,
            inherits: group_list(
                &names,
                &group.inherits,
                &format!("groups.{}.inherits", key(name)),
            )?,
            nodes: group.nodes.clone(),
            table: node_table(&group.nodes, &format!("groups.{}.nodes", key(name)))?,
        });
    }
    refuse_cycles(&groups)?;

    let mut chains = HashMap::new();
    let everyone = chain(&groups, &mut chains, vec![default]);
    let mut players = HashMap::with_capacity(file.players.len());
    for (uuid, player) in &file.players {
        let at = format!("players.{}", key(uuid));
        let id = node::parse_uuid(uuid)
            .filter(|_| !uuid.bytes().any(|byte| byte.is_ascii_uppercase()))
            .ok_or_else(|| error(&at, NOT_A_UUID))?;
        let mut direct = group_list(&names, &player.groups, &format!("{at}.groups"))?;
        direct.push(default);
        players.insert(
            id,
            Player {
                table: node_table(&player.nodes, &format!("{at}.nodes"))?,
                chain: chain(&groups, &mut chains, direct),
            },
        );
    }
    Ok(Permissions {
        groups,
        names,
        players,
        everyone,
    })
}

fn error(at: &str, message: &str) -> ConfigError {
    ConfigError {
        message: format!("{at}: {message}"),
    }
}

/// A key as TOML writes it in a path: bare when it can be, quoted otherwise.
fn key(text: &str) -> String {
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

/// A failure of the TOML reader on one line, with the line and the column where it stopped.
fn syntax_error(text: &str, failure: &toml::de::Error) -> ConfigError {
    let message = failure
        .message()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    let Some(span) = failure.span() else {
        return ConfigError { message };
    };
    let before = text.get(..span.start).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |part| part.chars().count())
        + 1;
    ConfigError {
        message: format!("line {line}, column {column}: {message}"),
    }
}

/// The index of every group by name, after checking the names.
fn group_names(
    groups: &BTreeMap<String, GroupFile>,
) -> Result<HashMap<String, usize>, ConfigError> {
    let mut names = HashMap::with_capacity(groups.len());
    for (index, name) in groups.keys().enumerate() {
        node::check_name(name).map_err(|why| {
            error(
                &format!("groups.{}", key(name)),
                &format!("the group name {name:?} {why}"),
            )
        })?;
        names.insert(name.clone(), index);
    }
    Ok(names)
}

/// The indices of the groups a list names, refusing a group that does not exist and a name
/// that the list holds twice.
fn group_list(
    names: &HashMap<String, usize>,
    list: &[String],
    at: &str,
) -> Result<Vec<usize>, ConfigError> {
    let mut indices = Vec::with_capacity(list.len());
    for name in list {
        let Some(&index) = names.get(name) else {
            return Err(error(at, &format!("the group {name:?} is not defined")));
        };
        if indices.contains(&index) {
            return Err(error(at, &format!("the group {name:?} is named twice")));
        }
        indices.push(index);
    }
    Ok(indices)
}

/// The table of a list of nodes, refusing a node of bad syntax and a node the list holds twice.
fn node_table(nodes: &[String], at: &str) -> Result<Table, ConfigError> {
    let mut table = Table::default();
    let mut seen = HashSet::with_capacity(nodes.len());
    for text in nodes {
        let rule =
            node::parse_rule(text).map_err(|why| error(at, &format!("the node {text:?} {why}")))?;
        if !seen.insert(text.as_str()) {
            return Err(error(at, &format!("the node {text:?} is named twice")));
        }
        table.add(rule);
    }
    Ok(table)
}

/// Refuses a group that inherits from itself through any path.
fn refuse_cycles(groups: &[Group]) -> Result<(), ConfigError> {
    // The state of each group in the search: 0 not seen, 1 on the path, 2 done. The search keeps
    // its own stack, so a long chain of inheritance cannot overflow the stack of the thread.
    let mut state = vec![0_u8; groups.len()];
    for start in 0..groups.len() {
        if state[start] != 0 {
            continue;
        }
        // The path from the start, each group with the index of its next parent to visit.
        let mut path = vec![(start, 0)];
        state[start] = 1;
        while let Some(&mut (group, ref mut next)) = path.last_mut() {
            let Some(&parent) = groups[group].inherits.get(*next) else {
                state[group] = 2;
                path.pop();
                continue;
            };
            *next += 1;
            match state[parent] {
                0 => {
                    state[parent] = 1;
                    path.push((parent, 0));
                }
                1 => return Err(cycle_error(groups, &path, parent)),
                _ => {}
            }
        }
    }
    Ok(())
}

/// The error of the cycle that closes when the last group of `path` inherits `parent`, which is
/// on the path.
fn cycle_error(groups: &[Group], path: &[(usize, usize)], parent: usize) -> ConfigError {
    let closing = path.last().map_or(parent, |&(group, _)| group);
    let at = format!("groups.{}.inherits", key(&groups[closing].name));
    if closing == parent {
        return error(&at, "the group inherits from itself");
    }
    let from = path
        .iter()
        .position(|&(group, _)| group == parent)
        .unwrap_or(0);
    let mut cycle: Vec<&str> = path[from..]
        .iter()
        .map(|&(group, _)| groups[group].name.as_str())
        .collect();
    cycle.push(&groups[parent].name);
    error(
        &at,
        &format!("the inheritance {} is a cycle", cycle.join(", ")),
    )
}

/// The search order of a player whose direct groups are `direct`, shared by every player with
/// the same groups.
fn chain(
    groups: &[Group],
    chains: &mut HashMap<Vec<usize>, Arc<Chain>>,
    mut direct: Vec<usize>,
) -> Arc<Chain> {
    direct.sort_unstable();
    direct.dedup();
    chains
        .entry(direct)
        .or_insert_with_key(|direct| Arc::new(build_chain(groups, direct)))
        .clone()
}

/// Finds every group of a player with its depth, the length of the shortest path of
/// inheritance from the player, and sorts them into the search order: higher weight first,
/// then closer first. Groups of the same weight and depth share a rank.
fn build_chain(groups: &[Group], direct: &[usize]) -> Chain {
    let mut depth = vec![u32::MAX; groups.len()];
    let mut queue = VecDeque::new();
    for &group in direct {
        depth[group] = 1;
        queue.push_back(group);
    }
    while let Some(group) = queue.pop_front() {
        for &parent in &groups[group].inherits {
            if depth[parent] == u32::MAX {
                depth[parent] = depth[group] + 1;
                queue.push_back(parent);
            }
        }
    }
    let mut members: Vec<Member> = (0..groups.len())
        .filter(|&group| depth[group] != u32::MAX)
        .map(|group| Member {
            group,
            depth: depth[group],
            rank: 0,
        })
        .collect();
    members.sort_by(|a, b| {
        let (one, other) = (&groups[a.group], &groups[b.group]);
        other
            .weight
            .cmp(&one.weight)
            .then(a.depth.cmp(&b.depth))
            .then(one.name.cmp(&other.name))
    });
    let mut rank = 0;
    for index in 1..members.len() {
        let (previous, member) = (&members[index - 1], &members[index]);
        if groups[previous.group].weight != groups[member.group].weight
            || previous.depth != member.depth
        {
            rank += 1;
        }
        members[index].rank = rank;
    }
    Chain {
        members: members.into(),
    }
}
