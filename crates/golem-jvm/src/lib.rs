// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * golem-jvm: the JVM host of Golem.
 *
 * A Forge server starts with java @user_jvm_args.txt @libraries/.../unix_args.txt, and the java
 * launcher turns those files into options of the JVM, a main class and its arguments. This
 * crate does the same work inside the Golem process: it reads the argument files as the launcher
 * reads them, turns the launcher options into the form that JNI_CreateJavaVM takes, loads libjvm
 * of the Java home, creates the JVM on a thread of its own, registers the native methods of the
 * bridge, calls the main class and waits in DestroyJavaVM until the last thread of the server
 * ends. On macOS the JVM must not run on the main thread, so it never does.
 *
 * Golem passes -Xrs, so the JVM leaves SIGINT, SIGTERM and SIGHUP to the host, which stops the
 * server through the server thread and never with a signal.
 *
 * okf/jvm-host.md describes the rules of the arguments, the options the host adds and the stop
 * path as they are built here.
 */

//! The JVM host of Golem.

mod argfile;
// The invocation API of the JNI is C, so the module that calls it needs unsafe code.
#[allow(unsafe_code)]
mod invocation;
mod launch;

use std::path::{
    Path,
    PathBuf, //
};

/// A failure to read the launch arguments.
#[derive(Debug)]
pub struct ArgError {
    /// What is wrong, with the file and the argument where there is one.
    pub message: String,
}

impl ArgError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ArgError {}

/// A failure to start the JVM.
#[derive(Debug)]
pub struct JvmError {
    /// What is wrong.
    pub message: String,
}

impl JvmError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for JvmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for JvmError {}

/// The tokens of an argument file of the java launcher, as the launcher reads them.
///
/// White space separates the tokens. Single and double quotes group, also in the middle of a
/// token, and a quote that the line leaves open ends with the line. Inside quotes a backslash
/// escapes the next character, with `\n`, `\r`, `\t` and `\f` for the control characters, and a
/// backslash at the end of a line joins the next line without its indentation. A `#` outside
/// quotes starts a comment to the end of the line. An `@` inside an argument file is no
/// reference to another file, the token stays as it is.
///
/// # Errors
///
/// Returns an [`ArgError`] when the text holds a NUL byte, which no argument can carry.
pub fn read_argfile(text: &str) -> Result<Vec<String>, ArgError> {
    argfile::tokens(text)
}

/// The launch of a JVM: its options in the form of `JNI_CreateJavaVM`, the main class and the
/// arguments of the main class.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LaunchSpec {
    /// The options, such as `-Xmx4G`, `--add-opens=java.base/java.lang=ALL-UNNAMED` or
    /// `-Djava.class.path=a.jar`.
    pub options: Vec<String>,
    /// The main class in its binary name, such as `cpw.mods.bootstraplauncher.BootstrapLauncher`.
    pub main_class: String,
    /// The arguments of the main method.
    pub arguments: Vec<String>,
}

impl LaunchSpec {
    /// The launch that the arguments of the java launcher describe, with every @file expanded
    /// relative to `base`.
    ///
    /// The @files expand up to the main class, an argument `@@name` stands for `@name`, and
    /// `--disable-@files` stops the expansion. The options that take a value after a space,
    /// such as `-p`, `--add-opens` or `-cp`, become the form with `=` or the property
    /// java.class.path, with the wildcards of a class path expanded relative to `base`. The old
    /// forms of options get their new names, `-client` and `-server` are dropped, and a launch
    /// without a class path gets the class path `.`, all as the launcher does. Everything after
    /// the main class is an argument of main.
    ///
    /// # Errors
    ///
    /// Returns an [`ArgError`] for an @file that does not read, an option that misses its
    /// value, a launch without a main class, and the options that only the launcher can carry
    /// out or that Golem does not do, such as `-jar`, `-m`, `--source`, `-version`, `--help` or
    /// `--dry-run`.
    pub fn from_launcher_args(arguments: &[String], base: &Path) -> Result<Self, ArgError> {
        launch::from_launcher_args(arguments, base)
    }

    /// The launch of the Forge server installed in `server`, as its run.sh starts it.
    ///
    /// The launch is `@user_jvm_args.txt`, when that file exists, and then the one
    /// `libraries/net/minecraftforge/forge/*/unix_args.txt`, relative to `server`.
    ///
    /// # Errors
    ///
    /// Returns an [`ArgError`] when `server` holds no Forge server or several, and every error
    /// of [`LaunchSpec::from_launcher_args`].
    pub fn forge_server(server: &Path) -> Result<Self, ArgError> {
        launch::forge_server(server)
    }

    /// Puts `jar` on the module path, so its classes load in the boot layer.
    ///
    /// The jar joins the end of the last `--module-path`, the one the JVM takes, or becomes the
    /// module path when there is none.
    pub fn add_module_path(&mut self, jar: &Path) {
        launch::add_module_path(self, jar);
    }
}

/// A Java home, the directory that holds bin/java and the library libjvm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaHome(pub PathBuf);

impl JavaHome {
    /// The path of libjvm in the server variant.
    pub fn libjvm(&self) -> PathBuf {
        let name = if cfg!(target_os = "macos") {
            "libjvm.dylib"
        } else {
            "libjvm.so"
        };
        self.0.join("lib").join("server").join(name)
    }
}

/// How the JVM ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The main method returned, or the exception it threw, as the JVM describes it.
    pub main: Result<(), String>,
    /// The result of `DestroyJavaVM`, 0 when every thread of the server ended.
    pub destroy_code: i32,
}

/// Runs `spec` in a new JVM of `java_home` and waits until it ends.
///
/// The JVM runs on a thread of its own. `register` runs on that thread after the JVM exists and
/// before the main class, to register native methods. A call to `System.exit` in the JVM ends the
/// process after the exit hook of the host has logged it, so this function does not return then.
///
/// The thread is named golem-jvm and has a stack of 8 MiB, or the size of `-Xss` when that is
/// larger. The options of `spec` reach `JNI_CreateJavaVM` in their order, followed by
/// `-Dsun.java.command` and `-Dsun.java.launcher=SUN_STANDARD`, which the launcher sets the same
/// way, and the hooks `exit` and `abort`. The main class loads through the system class loader,
/// with the checks and the messages of the launcher, and a main class that does not load ends
/// the process with status 1, as it does under the launcher. A process holds at most one JVM,
/// so this function runs once.
///
/// # Errors
///
/// Returns a [`JvmError`] when libjvm does not load, `JNI_CreateJavaVM` fails, an option holds a
/// NUL byte, or `register` fails or panics.
pub fn run<F>(java_home: &JavaHome, spec: &LaunchSpec, register: F) -> Result<Outcome, JvmError>
where
    F: FnOnce(&mut jni::JNIEnv<'_>) -> jni::errors::Result<()> + Send + 'static,
{
    invocation::run(java_home, spec, register)
}

/// Answers whether the signal of number `signal` was ignored when the process started, as
/// `nohup` leaves `SIGHUP`.
///
/// The JVM installs no handler of its own for a shutdown signal that is ignored, so a server that
/// the java launcher starts under nohup keeps running on a hangup. The host asks this before it
/// watches the shutdown signals itself, so that it keeps the same rule.
pub fn signal_ignored(signal: i32) -> bool {
    invocation::signal_ignored(signal)
}
