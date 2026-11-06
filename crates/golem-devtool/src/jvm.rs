// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The subcommands of golem-jvm: argfile prints the tokens of argument files, launch-spec the
 * launch that the arguments of the java launcher or a Forge server describe, and jvm-run runs
 * such a launch in the JVM of a Java home and prints how it ended.
 */

use std::path::{
    Path,
    PathBuf, //
};

use golem_jvm::{
    JavaHome,
    LaunchSpec, //
};
use serde_json::json;

use crate::Subcommand;

pub const ARGFILE: Subcommand = ("argfile", "<file>... the tokens of argument files", argfile);
pub const LAUNCH_SPEC: Subcommand = (
    "launch-spec",
    "--server <dir> | --base <dir> -- <argument>... the launch spec",
    launch_spec,
);
pub const JVM_RUN: Subcommand = (
    "jvm-run",
    "--java-home <dir> --server <dir> | --base <dir> -- <argument>... how a launch ends",
    jvm_run,
);

/// What launch-spec and jvm-run are asked for.
#[derive(Default)]
struct Request {
    /// `--java-home`, for jvm-run.
    java_home: Option<PathBuf>,
    /// `--server`, a Forge server to launch as its run.sh does.
    server: Option<PathBuf>,
    /// `--base`, the directory of the @files of the launcher arguments.
    base: Option<PathBuf>,
    /// Every `--add-module-path`, a jar to put on the module path after the launch is read.
    jars: Vec<PathBuf>,
    /// The arguments of the java launcher after `--`.
    launcher: Vec<String>,
}

/// Reads the arguments of launch-spec and jvm-run.
fn parse(arguments: &[String]) -> Result<Request, String> {
    let mut request = Request::default();
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        if argument == "--" {
            request.launcher = rest.cloned().collect();
            break;
        }
        let mut value = || {
            rest.next()
                .map(PathBuf::from)
                .ok_or_else(|| format!("{argument} needs a value"))
        };
        match argument.as_str() {
            "--java-home" => request.java_home = Some(value()?),
            "--server" => request.server = Some(value()?),
            "--base" => request.base = Some(value()?),
            "--add-module-path" => request.jars.push(value()?),
            _ => return Err(format!("unknown argument {argument}")),
        }
    }
    if request.server.is_some() == request.base.is_some() {
        return Err("name either --server or --base".to_owned());
    }
    if request.server.is_some() && !request.launcher.is_empty() {
        return Err("--server takes no launcher arguments".to_owned());
    }
    Ok(request)
}

/// The launch of a request, read relative to `directory`.
fn spec(request: &Request, directory: &Path) -> Result<LaunchSpec, golem_jvm::ArgError> {
    let mut spec = if request.server.is_some() {
        LaunchSpec::forge_server(directory)?
    } else {
        LaunchSpec::from_launcher_args(&request.launcher, directory)?
    };
    for jar in &request.jars {
        spec.add_module_path(jar);
    }
    Ok(spec)
}

fn argfile(arguments: &[String]) -> i32 {
    if arguments.is_empty() {
        eprintln!("golem-devtool: argfile needs a file");
        return 1;
    }
    for file in arguments {
        let text = match std::fs::read_to_string(file) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("golem-devtool: argfile: {file}: {error}");
                return 1;
            }
        };
        match golem_jvm::read_argfile(&text) {
            Ok(tokens) => println!("{}", json!({ "file": file, "tokens": tokens })),
            Err(error) => {
                eprintln!("golem-devtool: argfile: {file}: {error}");
                return 1;
            }
        }
    }
    0
}

fn launch_spec(arguments: &[String]) -> i32 {
    let request = match parse(arguments) {
        Ok(request) if request.java_home.is_none() => request,
        Ok(_) => {
            eprintln!("golem-devtool: launch-spec takes no --java-home");
            return 1;
        }
        Err(error) => {
            eprintln!("golem-devtool: launch-spec: {error}");
            return 1;
        }
    };
    let directory = request.server.as_ref().or(request.base.as_ref());
    match spec(&request, directory.map_or(Path::new("."), PathBuf::as_path)) {
        Ok(spec) => {
            println!(
                "{}",
                json!({
                    "options": spec.options,
                    "main_class": spec.main_class,
                    "arguments": spec.arguments,
                })
            );
            0
        }
        Err(error) => {
            eprintln!("golem-devtool: launch-spec: {error}");
            1
        }
    }
}

/// Makes `path` absolute, so it stays valid when the working directory changes.
fn absolute(path: &Path) -> Result<PathBuf, String> {
    std::path::absolute(path).map_err(|error| format!("{}: {error}", path.display()))
}

/// Runs the launch of a request in the directory of its server or its base, as java runs in
/// the directory it is started in.
fn run_request(mut request: Request) -> Result<serde_json::Value, String> {
    let java_home = request
        .java_home
        .as_deref()
        .ok_or("jvm-run needs --java-home")
        .and_then(|home| absolute(home).map_err(|_| "the Java home is not readable"))?;
    request.jars = request
        .jars
        .iter()
        .map(|jar| absolute(jar))
        .collect::<Result<_, _>>()?;
    let directory = request.server.as_ref().or(request.base.as_ref());
    if let Some(directory) = directory {
        std::env::set_current_dir(directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
    }
    let spec = spec(&request, Path::new(".")).map_err(|error| error.to_string())?;
    let outcome = golem_jvm::run(&JavaHome(java_home), &spec, |_| Ok(()))
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "main": if outcome.main.is_ok() { "returned" } else { "threw" },
        "exception": outcome.main.err(),
        "destroy_code": outcome.destroy_code,
    }))
}

fn jvm_run(arguments: &[String]) -> i32 {
    let outcome = parse(arguments).and_then(run_request);
    match outcome {
        Ok(outcome) => {
            println!("{outcome}");
            0
        }
        Err(error) => {
            eprintln!("golem-devtool: jvm-run: {error}");
            1
        }
    }
}
