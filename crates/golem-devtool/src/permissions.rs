// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The subcommands of golem-permissions: permissions prints the decisions of a configuration for
 * a player and nodes, with what LuckPerms answered for them on Bukkit, permissions-groups prints
 * the groups of a player in the order a check searches them, each with its nodes, and
 * permissions-command runs commands of the port of LuckPerms on a configuration, as the runtime
 * runs them, without a store.
 */

use std::{
    hint::black_box,
    time::Instant, //
};

use golem_bridge::{
    Player,
    Sender, //
};
use golem_permissions::{
    Permissions,
    command::{
        self,
        Change,
        Host,
        User, //
    },
};
use golem_plugin::Resolver;
use serde::Deserialize;
use serde_json::{
    Value,
    json, //
};

use crate::{
    Subcommand,
    bridge::{
        finish,
        read, //
    },
};

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
pub const COMMAND: Subcommand = (
    "permissions-command",
    "<script.json> the commands of the port of LuckPerms on a configuration",
    commands,
);

/// The arguments both subcommands read.
#[derive(Default)]
struct Options {
    config: Option<String>,
    player: Option<String>,
    nodes: Vec<String>,
    repeat: Option<u32>,
}

/// Prints `{"decisions": [{"node", "value", "reason", "hasPermission"}]}`, one decision per
/// `--node` in the order given, where `hasPermission` holds what `LuckPerms` answered on Bukkit for
/// a player and for an operator. With `--repeat`, it also runs every check that many times through
/// the form without a reason and prints the mean time of one check.
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
            json!({
                "node": node,
                "value": decision.value,
                "reason": decision.reason,
                "hasPermission": {
                    "player": permissions.has_permission(player, node, false),
                    "operator": permissions.has_permission(player, node, true),
                },
            })
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

/// A script of permissions-command: the configuration, the sender, the players the server knows
/// and the commands, which run one after the other on the configuration that the one before left.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Script {
    config: String,
    sender: SenderSpec,
    #[serde(default)]
    players: Vec<PlayerSpec>,
    commands: Vec<CommandSpec>,
}

/// The sender of the commands: the console, a player or another source at its level.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SenderSpec {
    uuid: Option<String>,
    name: Option<String>,
    level: Option<u8>,
}

/// A player that the server knows, online or not.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlayerSpec {
    uuid: String,
    name: String,
    #[serde(default)]
    online: bool,
}

/// A command by its label, `luckperms` or an alias, and its arguments.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CommandSpec {
    label: String,
    arguments: String,
}

/// The sender, the players and the permissions of one command.
struct ScriptHost<'a> {
    sender: &'a Sender,
    players: &'a [PlayerSpec],
    resolver: &'a Resolver,
}

impl Host for ScriptHost<'_> {
    fn holds(&self, node: &str) -> bool {
        self.resolver.has_permission(self.sender, node)
    }

    fn sender_name(&self) -> String {
        match self.sender {
            Sender::Console => "Console".to_owned(),
            Sender::Player { player, .. } => player.name.clone(),
            Sender::Other { name, .. } => name.clone(),
        }
    }

    fn player_named(&self, name: &str) -> Option<User> {
        self.players
            .iter()
            .find(|player| player.name.eq_ignore_ascii_case(name))
            .map(|player| User {
                uuid: player.uuid.to_ascii_lowercase(),
                name: Some(player.name.clone()),
            })
    }

    fn name_of(&self, uuid: &str) -> Option<String> {
        self.players
            .iter()
            .find(|player| player.uuid.eq_ignore_ascii_case(uuid))
            .map(|player| player.name.clone())
    }

    fn online(&self, uuid: &str) -> bool {
        self.players
            .iter()
            .any(|player| player.online && player.uuid.eq_ignore_ascii_case(uuid))
    }
}

/// Prints `{"outcomes": [{"lines", "changes", "config", "saveError"}]}`, one outcome per command,
/// with the TOML of the configuration after a command that changed it.
fn commands(arguments: &[String]) -> i32 {
    finish(COMMAND.0, read(COMMAND.0, arguments).and_then(run_commands))
}

fn run_commands(script: Script) -> Result<Value, String> {
    let sender = match (script.sender.uuid, script.sender.name) {
        (Some(uuid), Some(name)) => Sender::Player {
            player: Player { uuid, name },
            level: script.sender.level.unwrap_or(0),
        },
        (None, Some(name)) => Sender::Other {
            name,
            level: script.sender.level.unwrap_or(0),
        },
        (None, None) => Sender::Console,
        (Some(_), None) => return Err("a sender with a UUID needs a name".to_owned()),
    };
    let mut permissions =
        Permissions::from_toml(&script.config).map_err(|error| format!("config: {error}"))?;
    let mut outcomes = Vec::new();
    for spec in &script.commands {
        let resolver = Resolver::from_permissions(permissions.clone());
        let host = ScriptHost {
            sender: &sender,
            players: &script.players,
            resolver: &resolver,
        };
        let outcome = command::execute(&permissions, &spec.label, &spec.arguments, &host);
        let changes: Vec<Value> = outcome
            .changes
            .iter()
            .map(|change| match change {
                Change::Group(name) => json!({"group": name}),
                Change::RemovedGroup(name) => json!({"removedGroup": name}),
                Change::Player(uuid) => json!({"player": uuid}),
            })
            .collect();
        let config = outcome
            .permissions
            .as_ref()
            .map(|after| after.config().to_toml());
        outcomes.push(json!({
            "label": spec.label,
            "arguments": spec.arguments,
            "lines": outcome.lines,
            "changes": changes,
            "config": config,
            "saveError": outcome.save_error,
        }));
        if let Some(after) = outcome.permissions {
            permissions = after;
        }
    }
    Ok(json!({ "outcomes": outcomes }))
}
