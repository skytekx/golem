// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The subcommand of golem-permissions: permissions-check prints the decision of a configuration
 * for a player and a node.
 */

use crate::Subcommand;

pub const CHECK: Subcommand = (
    "permissions",
    "--config <file> --player <uuid> --node <node>... the decisions",
    check,
);

fn check(arguments: &[String]) -> i32 {
    let _ = arguments;
    eprintln!("golem-devtool: permissions is not built yet");
    1
}
