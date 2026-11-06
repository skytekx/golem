// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The subcommands of golem-jvm: argfile prints the tokens of argument files, launch-spec the
 * launch that the arguments of the java launcher or a Forge server describe.
 */

use crate::Subcommand;

pub const ARGFILE: Subcommand = ("argfile", "<file>... the tokens of argument files", argfile);
pub const LAUNCH_SPEC: Subcommand = (
    "launch-spec",
    "--server <dir> | --base <dir> <argument>... the launch spec",
    launch_spec,
);

fn argfile(arguments: &[String]) -> i32 {
    let _ = arguments;
    eprintln!("golem-devtool: argfile is not built yet");
    1
}

fn launch_spec(arguments: &[String]) -> i32 {
    let _ = arguments;
    eprintln!("golem-devtool: launch-spec is not built yet");
    1
}
