// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The subcommand of golem-bridge and golem-plugin: plugins feeds a script of events and commands
 * to the plugin runtime with the core plugin, as the mod would, and prints what the runtime
 * answered and queued.
 */

use crate::Subcommand;

pub const PLUGINS: Subcommand = (
    "plugins",
    "<script.json> the answers of the plugin runtime",
    plugins,
);

fn plugins(arguments: &[String]) -> i32 {
    let _ = arguments;
    eprintln!("golem-devtool: plugins is not built yet");
    1
}
