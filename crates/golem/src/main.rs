// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * golem: the Rust hybrid server of SkyTekx.
 *
 * golem run starts the JVM inside this process, boots the Forge server of a directory unchanged
 * and runs the plugins around it. A signal never stops the server: the first SIGINT, SIGTERM or
 * SIGHUP queues save-all flush and stop on the server thread, and golem waits until the JVM has
 * returned. golem source names the repository and the commit of the running binary, as section
 * 13 of the AGPL asks.
 *
 * The arguments are read by hand, since four options do not need a crate. okf/jvm-host.md
 * describes the launch, the stop path and where the host jar is found.
 */

//! The program golem.

use std::{
    io::Write,
    path::{
        Path,
        PathBuf, //
    },
    process::ExitCode,
    sync::OnceLock,
    time::Duration,
};

use golem_bridge::{
    Action,
    Actions, //
};
use golem_jvm::{
    JavaHome,
    JvmError,
    LaunchSpec,
    Outcome, //
};
use golem_plugin::{
    CorePlugin,
    DEFAULT_BUDGET,
    Runtime,
    Source, //
};

/// The version of the crates.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The repository of the source.
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// The commit the binary is built from, which build.rs names.
const COMMIT: &str = env!("GOLEM_COMMIT");

/// The jar of the host class, which the bridge mod calls.
const HOST_JAR: &str = "golem-host.jar";

const USAGE: &str = "usage: golem <command>

  run [--server <dir>] [--java-home <dir>] [--host-jar <file>] [--budget-ms <n>] [-- <arg>...]
              runs the Forge server of <dir>, by default the current directory, in the JVM
              of the Java home, by default $JAVA_HOME, with the arguments after -- passed on
              to its main class
  source      names the version, the repository and the commit of this binary
  version     prints the version
  help        prints this help";

/// Logs a line of the host on standard error.
fn log(line: std::fmt::Arguments<'_>) {
    let _ = writeln!(std::io::stderr(), "[golem] {line}");
}

/// The options of golem run.
struct RunOptions {
    server: PathBuf,
    java_home: Option<PathBuf>,
    host_jar: Option<PathBuf>,
    budget: Duration,
    /// The arguments after `--`, which follow the arguments of `unix_args.txt`, as the
    /// arguments of `run.sh` do.
    arguments: Vec<String>,
}

/// Reads the options of golem run, each as `--name value` or `--name=value`.
fn parse_run(arguments: &[String]) -> Result<RunOptions, String> {
    let mut options = RunOptions {
        server: PathBuf::from("."),
        java_home: None,
        host_jar: None,
        budget: DEFAULT_BUDGET,
        arguments: Vec::new(),
    };
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        if argument == "--" {
            options.arguments = rest.cloned().collect();
            break;
        }
        let (name, inline) = match argument.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value.to_owned())),
            _ => (argument.as_str(), None),
        };
        let value = inline
            .or_else(|| rest.next().cloned())
            .ok_or_else(|| format!("{name} needs a value"));
        match name {
            "--server" => options.server = PathBuf::from(value?),
            "--java-home" => options.java_home = Some(PathBuf::from(value?)),
            "--host-jar" => options.host_jar = Some(PathBuf::from(value?)),
            "--budget-ms" => {
                let value = value?;
                let millis = value.parse().map_err(|_| {
                    format!("--budget-ms takes a whole number of milliseconds, not {value}")
                })?;
                options.budget = Duration::from_millis(millis);
            }
            _ => return Err(format!("run has no option {argument}")),
        }
    }
    Ok(options)
}

/// The value of an environment variable, unless it is unset or empty.
fn variable(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// `path` made absolute with its links resolved, so it stays valid in the server directory.
fn resolve(path: &Path, what: &str) -> Result<PathBuf, String> {
    std::fs::canonicalize(path).map_err(|error| format!("{what} {}: {error}", path.display()))
}

/// The Java home of --java-home, else of `JAVA_HOME`.
fn java_home(given: Option<PathBuf>) -> Result<JavaHome, String> {
    let home = given
        .or_else(|| variable("JAVA_HOME"))
        .ok_or("no Java home: pass --java-home or set JAVA_HOME")?;
    let home = JavaHome(resolve(&home, "the Java home")?);
    if !home.libjvm().is_file() {
        return Err(format!(
            "the Java home {} holds no {}",
            home.0.display(),
            home.libjvm().display()
        ));
    }
    Ok(home)
}

/// The host jar of --host-jar, else of `GOLEM_HOST_JAR`, else the one beside the program, else
/// the one in the server directory.
fn host_jar(given: Option<PathBuf>, server: &Path) -> Result<PathBuf, String> {
    let jar = if let Some(jar) = given.or_else(|| variable("GOLEM_HOST_JAR")) {
        resolve(&jar, "the host jar")?
    } else {
        let beside = std::env::current_exe()
            .ok()
            .and_then(|program| program.parent().map(|directory| directory.join(HOST_JAR)));
        beside
            .into_iter()
            .chain([server.join(HOST_JAR)])
            .find(|candidate| candidate.is_file())
            .ok_or_else(|| {
                format!(
                    "no {HOST_JAR}: pass --host-jar, set GOLEM_HOST_JAR, or put it beside the program or into {}",
                    server.display()
                )
            })
            .and_then(|jar| resolve(&jar, "the host jar"))?
    };
    // The module path is an option string of the JVM, which carries text alone.
    if jar.to_str().is_none() {
        return Err(format!(
            "the path of the host jar {} is not UTF-8",
            jar.display()
        ));
    }
    Ok(jar)
}

/// The source of this binary, which section 13 of the AGPL asks the server to offer.
fn source() -> Source {
    Source {
        version: VERSION.to_owned(),
        repository: REPOSITORY.to_owned(),
        commit: COMMIT.to_owned(),
    }
}

/// Installs the plugin runtime with the core plugin as the handler of the bridge, and answers
/// the queue of actions the mod drains on the server thread.
fn install_plugins(budget: Duration) -> Actions {
    let mut runtime = Runtime::new(budget);
    runtime.add(Box::new(CorePlugin::new(source())));
    golem_bridge::install(Box::new(runtime))
}

/// The JVM once it exists, for the stop of a server that the bridge does not reach.
static JVM: OnceLock<jni::JavaVM> = OnceLock::new();

/// Registers the natives of the bridge and keeps the JVM, on the thread of the JVM before the
/// main class runs.
fn register(env: &mut jni::JNIEnv<'_>) -> jni::errors::Result<()> {
    let _ = JVM.set(env.get_java_vm()?);
    golem_bridge::register_natives(env)
}

/// Ends the JVM as the java launcher does on a signal, with System.exit and the exit status
/// 128 plus the number of the signal, so the shutdown hook of the server stops it and saves the
/// worlds. Before the JVM exists there is nothing to save, and golem exits at once.
fn exit_through_jvm(status: i32) {
    let Some(vm) = JVM.get() else {
        log(format_args!(
            "the JVM does not run yet, so golem exits with {status}"
        ));
        std::process::exit(status);
    };
    let exited = vm.attach_current_thread_as_daemon().and_then(|mut env| {
        env.call_static_method(
            "java/lang/System",
            "exit",
            "(I)V",
            &[jni::objects::JValue::Int(status)],
        )
        .map(drop)
    });
    // System.exit returns only when it could not run, for example while the JVM ends.
    if let Err(error) = exited {
        log(format_args!("the JVM did not take System.exit: {error}"));
    }
}

/// Answers the dump of every thread of the JVM, from the operation `threadPrint` of the
/// `DiagnosticCommand` bean, which is what `jcmd Thread.print` runs.
fn thread_dump(env: &mut jni::JNIEnv<'_>) -> jni::errors::Result<String> {
    use jni::objects::{
        JObject,
        JString,
        JValue, //
    };

    let server = env
        .call_static_method(
            "java/lang/management/ManagementFactory",
            "getPlatformMBeanServer",
            "()Ljavax/management/MBeanServer;",
            &[],
        )?
        .l()?;
    let name = env.new_string("com.sun.management:type=DiagnosticCommand")?;
    let bean = env.new_object(
        "javax/management/ObjectName",
        "(Ljava/lang/String;)V",
        &[JValue::Object(&name)],
    )?;
    let operation = env.new_string("threadPrint")?;
    let none = env.new_object_array(0, "java/lang/String", JObject::null())?;
    let parameters = env.new_object_array(1, "java/lang/Object", &none)?;
    let kind = env.new_string("[Ljava.lang.String;")?;
    let signature = env.new_object_array(1, "java/lang/String", &kind)?;
    let dump = env
        .call_method(
            &server,
            "invoke",
            "(Ljavax/management/ObjectName;Ljava/lang/String;[Ljava/lang/Object;[Ljava/lang/String;)Ljava/lang/Object;",
            &[
                JValue::Object(&bean),
                JValue::Object(&operation),
                JValue::Object(&parameters),
                JValue::Object(&signature),
            ],
        )?
        .l()?;
    let dump = JString::from(dump);
    let text = env.get_string(&dump)?.into();
    Ok(text)
}

/// Prints the dump of every thread on standard output, as the JVM does on SIGQUIT when it has
/// the signal, which -Xrs takes from it. The default action of SIGQUIT would end the process
/// without saving, and an operator sends it for a dump alone.
fn print_thread_dump() {
    let Some(vm) = JVM.get() else {
        log(format_args!(
            "SIGQUIT: the JVM does not run yet, so there is no thread to dump"
        ));
        return;
    };
    let dump = vm.attach_current_thread_as_daemon().and_then(|mut env| {
        let dump = thread_dump(&mut env);
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_describe();
            let _ = env.exception_clear();
        }
        dump
    });
    match dump {
        Ok(text) => {
            let mut out = std::io::stdout().lock();
            let _ = writeln!(out, "{text}");
            let _ = out.flush();
            log(format_args!("SIGQUIT: printed the dump of every thread"));
        }
        Err(error) => log(format_args!(
            "SIGQUIT: no dump of the threads ({error}), jcmd <pid> Thread.print gives one"
        )),
    }
}

/// Turns SIGINT, SIGTERM and SIGHUP into the stop of an operator, on a thread of its own.
///
/// When the bridge is connected, the first signal queues save-all flush and stop, which the mod
/// runs on the server thread at the end of the next tick. Without the bridge no tick drains the
/// queue, so the first signal ends the JVM through System.exit, which runs the shutdown hook of
/// the server as a signal does under java. A later signal changes nothing, since the stop is
/// already on its way and a second stop would only race it. SIGQUIT prints the dump of every
/// thread and stops nothing, as under java.
fn watch_signals(actions: Actions) -> Result<(), String> {
    use signal_hook::consts::{
        SIGHUP,
        SIGINT,
        SIGQUIT,
        SIGTERM, //
    };

    // A shutdown signal that the process started with ignored stays ignored, as HotSpot leaves
    // it, so a server under nohup keeps running on a hangup.
    let mut watched = vec![SIGQUIT];
    for signal in [SIGINT, SIGTERM, SIGHUP] {
        if golem_jvm::signal_ignored(signal) {
            let name = signal_hook::low_level::signal_name(signal).unwrap_or("a signal");
            log(format_args!(
                "{name} is ignored, as the process started with it ignored, so it does not stop the server"
            ));
        } else {
            watched.push(signal);
        }
    }
    let mut signals = signal_hook::iterator::Signals::new(watched)
        .map_err(|error| format!("could not watch the signals: {error}"))?;
    std::thread::Builder::new()
        .name("golem-signals".to_owned())
        .spawn(move || {
            let mut queued = false;
            for signal in signals.forever() {
                if signal == SIGQUIT {
                    print_thread_dump();
                    continue;
                }
                let name = signal_hook::low_level::signal_name(signal).unwrap_or("a signal");
                if queued {
                    log(format_args!(
                        "{name}: the stop is already queued, waiting for the server to end"
                    ));
                    continue;
                }
                queued = true;
                if golem_bridge::connected() {
                    actions.push(Action::Command("save-all flush".to_owned()));
                    actions.push(Action::Command("stop".to_owned()));
                    log(format_args!(
                        "{name}: queued save-all flush and stop on the server thread"
                    ));
                } else {
                    log(format_args!(
                        "{name}: the bridge is not connected, so the JVM exits through the shutdown hook of the server"
                    ));
                    exit_through_jvm(128 + signal);
                }
            }
        })
        .map_err(|error| format!("could not start the thread golem-signals: {error}"))?;
    Ok(())
}

/// Logs how the JVM ended and answers the exit status of golem.
fn report(outcome: Result<Outcome, JvmError>) -> ExitCode {
    match outcome {
        Ok(Outcome {
            main: Ok(()),
            destroy_code: 0,
        }) => {
            log(format_args!(
                "the server ended: main returned and DestroyJavaVM answered 0"
            ));
            ExitCode::SUCCESS
        }
        Ok(Outcome {
            main: Ok(()),
            destroy_code,
        }) => {
            log(format_args!(
                "main returned, but DestroyJavaVM answered {destroy_code}"
            ));
            ExitCode::FAILURE
        }
        Ok(Outcome {
            main: Err(exception),
            destroy_code,
        }) => {
            log(format_args!(
                "main threw {exception}, DestroyJavaVM answered {destroy_code}"
            ));
            ExitCode::FAILURE
        }
        Err(error) => {
            log(format_args!("the JVM did not run: {error}"));
            ExitCode::FAILURE
        }
    }
}

/// golem run: starts the Forge server in the JVM and waits until it ends.
fn run(arguments: &[String]) -> Result<ExitCode, String> {
    let options = parse_run(arguments)?;
    let server = resolve(&options.server, "the server directory")?;
    let java_home = java_home(options.java_home)?;
    let host_jar = host_jar(options.host_jar, &server)?;
    // The relative paths of unix_args.txt name files of the server directory, as under run.sh.
    std::env::set_current_dir(&server)
        .map_err(|error| format!("could not change into {}: {error}", server.display()))?;
    let mut spec = LaunchSpec::forge_server(&server).map_err(|error| error.to_string())?;
    spec.options.insert(0, "-Xrs".to_owned());
    spec.add_module_path(&host_jar);
    spec.arguments.extend(options.arguments);

    let actions = install_plugins(options.budget);
    watch_signals(actions)?;
    log(format_args!(
        "Golem {VERSION} at {COMMIT} runs {} of {} in the JVM of {}",
        spec.main_class,
        server.display(),
        java_home.0.display()
    ));
    Ok(report(golem_jvm::run(&java_home, &spec, register)))
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.first().map(String::as_str) {
        Some("run") => run(&arguments[1..]).unwrap_or_else(|error| {
            eprintln!("golem: {error}");
            ExitCode::FAILURE
        }),
        Some("source") => {
            println!("version {VERSION}\nrepository {REPOSITORY}\ncommit {COMMIT}");
            ExitCode::SUCCESS
        }
        Some("version" | "--version" | "-V") => {
            println!("golem {VERSION}");
            ExitCode::SUCCESS
        }
        Some("help" | "--help" | "-h") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("golem: no command {other}\n{USAGE}");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}
