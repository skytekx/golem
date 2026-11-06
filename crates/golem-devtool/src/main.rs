// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * golem-devtool: the door of the notebooks into the crates of Golem.
 *
 * Every subcommand is one primitive of one crate. It reads its input from its arguments or from
 * a file, prints one JSON document on standard output and exits with 0, or prints the error on
 * standard error and exits with 1, so a notebook checks one primitive at a time. Each crate has
 * its module here, and the table below is the only place that names them all.
 */

//! The primitives of the Golem crates for the notebooks.

mod bridge;
mod game;
mod jvm;
mod permissions;

/// A subcommand: its name, a line of help and the function that runs it on the arguments after
/// its name.
type Subcommand = (&'static str, &'static str, fn(&[String]) -> i32);

const SUBCOMMANDS: &[Subcommand] = &[
    jvm::ARGFILE,
    jvm::LAUNCH_SPEC,
    jvm::JVM_RUN,
    permissions::CHECK,
    permissions::GROUPS,
    bridge::PLUGINS,
    bridge::ACTIONS_ENCODE,
    bridge::TICK_STATS,
    bridge::NATIVES,
    game::GAME_WIRE,
];

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some(name) = arguments.first() else {
        usage();
        std::process::exit(1);
    };
    let Some((_, _, run)) = SUBCOMMANDS.iter().find(|(known, _, _)| known == name) else {
        eprintln!("golem-devtool: no subcommand {name}");
        usage();
        std::process::exit(1);
    };
    std::process::exit(run(&arguments[1..]));
}

fn usage() {
    eprintln!("usage: golem-devtool <subcommand> [arguments]");
    for (name, help, _) in SUBCOMMANDS {
        eprintln!("  {name:<14} {help}");
    }
}
