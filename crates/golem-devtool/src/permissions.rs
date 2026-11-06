// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The subcommands of golem-permissions: permissions prints the decisions of a configuration for
 * a player and nodes, and permissions-groups prints the groups of a player in the order a check
 * searches them, each with its nodes.
 */

use std::{
    hint::black_box,
    time::Instant, //
};

use golem_permissions::Permissions;
use serde_json::json;

use crate::Subcommand;

pub const CHECK: Subcommand = (
    "permissions",
    "--config <file> --player <uuid> --node <node>... [--repeat <n>] the decisions",
    check,
);
pub const GROUPS: Subcommand = (
    "permissions-groups",
    "--config <file> --player <uuid> the groups of a player with their nodes",
    groups,
);

/// The arguments both subcommands read.
#[derive(Default)]
struct Options {
    config: Option<String>,
    player: Option<String>,
    nodes: Vec<String>,
    repeat: Option<u32>,
}

/// Prints `{"decisions": [{"node", "value", "reason"}]}`, one decision per `--node` in the order
/// given. With `--repeat`, it also runs every check that many times through the form without a
/// reason and prints the mean time of one check.
fn check(arguments: &[String]) -> i32 {
    let name = CHECK.0;
    let (options, permissions) = match prepare(name, arguments, true) {
        Ok(prepared) => prepared,
        Err(message) => return fail(name, &message),
    };
    let player = options.player.as_deref().unwrap_or_default();
    if options.nodes.is_empty() {
        return fail(name, "give at least one --node");
    }
    let decisions: Vec<_> = options
        .nodes
        .iter()
        .map(|node| {
            let decision = permissions.check(player, node);
            json!({"node": node, "value": decision.value, "reason": decision.reason})
        })
        .collect();
    let mut output = json!({ "decisions": decisions });
    if let Some(repeat) = options.repeat {
        let started = Instant::now();
        for _ in 0..repeat {
            for node in &options.nodes {
                black_box(permissions.value(black_box(player), black_box(node)));
            }
        }
        let nodes = u32::try_from(options.nodes.len()).unwrap_or(u32::MAX);
        let checks = f64::from(repeat) * f64::from(nodes);
        let nanos = started.elapsed().as_secs_f64() * 1e9 / checks.max(1.0);
        output["repeat"] = json!(repeat);
        output["nanos_per_check"] = json!(nanos);
    }
    println!("{output}");
    0
}

/// Prints `{"player", "groups": [{"group", "weight", "depth", "rank", "nodes"}]}`.
fn groups(arguments: &[String]) -> i32 {
    let name = GROUPS.0;
    let (options, permissions) = match prepare(name, arguments, false) {
        Ok(prepared) => prepared,
        Err(message) => return fail(name, &message),
    };
    let player = options.player.as_deref().unwrap_or_default();
    let Some(memberships) = permissions.groups_of(player) else {
        return fail(name, &format!("the player {player:?} is not a UUID"));
    };
    let groups: Vec<_> = memberships
        .iter()
        .map(|membership| {
            json!({
                "group": membership.group,
                "weight": membership.weight,
                "depth": membership.depth,
                "rank": membership.rank,
                "nodes": permissions.group_nodes(&membership.group).unwrap_or_default(),
            })
        })
        .collect();
    println!("{}", json!({"player": player, "groups": groups}));
    0
}

/// Reads the arguments and loads the configuration they name.
fn prepare(
    name: &str,
    arguments: &[String],
    takes_nodes: bool,
) -> Result<(Options, Permissions), String> {
    let options = parse(arguments, takes_nodes)?;
    let Some(config) = options.config.as_deref() else {
        return Err(format!("{name} needs --config <file>"));
    };
    if options.player.is_none() {
        return Err(format!("{name} needs --player <uuid>"));
    }
    let text = std::fs::read_to_string(config)
        .map_err(|error| format!("cannot read {config}: {error}"))?;
    let permissions =
        Permissions::from_toml(&text).map_err(|error| format!("{config}: {error}"))?;
    Ok((options, permissions))
}

fn parse(arguments: &[String], takes_nodes: bool) -> Result<Options, String> {
    let mut options = Options::default();
    let mut rest = arguments.iter();
    while let Some(flag) = rest.next() {
        let mut value = || {
            rest.next()
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "--config" => options.config = Some(value()?),
            "--player" => options.player = Some(value()?),
            "--node" if takes_nodes => options.nodes.push(value()?),
            "--repeat" if takes_nodes => {
                let text = value()?;
                let repeat = text
                    .parse()
                    .map_err(|_| format!("--repeat takes a count, not {text:?}"))?;
                options.repeat = Some(repeat);
            }
            _ => return Err(format!("unknown argument {flag:?}")),
        }
    }
    Ok(options)
}

fn fail(name: &str, message: &str) -> i32 {
    eprintln!("golem-devtool: {name}: {message}");
    1
}
