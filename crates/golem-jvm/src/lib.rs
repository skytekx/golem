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
 */

//! The JVM host of Golem.

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

/// A failure to start the JVM.
#[derive(Debug)]
pub struct JvmError {
    /// What is wrong.
    pub message: String,
}

/// The tokens of an argument file of the java launcher, as the launcher reads them.
pub fn read_argfile(text: &str) -> Result<Vec<String>, ArgError> {
    let _ = text;
    todo!("golem-jvm: read_argfile")
}

/// The launch of a JVM: its options in the form of JNI_CreateJavaVM, the main class and the
/// arguments of the main class.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LaunchSpec {
    /// The options, such as -Xmx4G, --add-opens=java.base/java.lang=ALL-UNNAMED or
    /// -Djava.class.path=a.jar.
    pub options: Vec<String>,
    /// The main class in its binary name, such as cpw.mods.bootstraplauncher.BootstrapLauncher.
    pub main_class: String,
    /// The arguments of the main method.
    pub arguments: Vec<String>,
}

impl LaunchSpec {
    /// The launch that the arguments of the java launcher describe, with every @file expanded
    /// relative to `base`.
    pub fn from_launcher_args(arguments: &[String], base: &Path) -> Result<Self, ArgError> {
        let _ = (arguments, base);
        todo!("golem-jvm: LaunchSpec::from_launcher_args")
    }

    /// The launch of the Forge server installed in `server`, as its run.sh starts it.
    pub fn forge_server(server: &Path) -> Result<Self, ArgError> {
        let _ = server;
        todo!("golem-jvm: LaunchSpec::forge_server")
    }

    /// Puts `jar` on the module path, so its classes load in the boot layer.
    pub fn add_module_path(&mut self, jar: &Path) {
        let _ = jar;
        todo!("golem-jvm: LaunchSpec::add_module_path")
    }
}

/// A Java home, the directory that holds bin/java and the library libjvm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaHome(pub PathBuf);

impl JavaHome {
    /// The path of libjvm in the server variant.
    pub fn libjvm(&self) -> PathBuf {
        todo!("golem-jvm: JavaHome::libjvm")
    }
}

/// How the JVM ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The main method returned, or the exception it threw, as the JVM describes it.
    pub main: Result<(), String>,
    /// The result of DestroyJavaVM, 0 when every thread of the server ended.
    pub destroy_code: i32,
}

/// Runs `spec` in a new JVM of `java_home` and waits until it ends.
///
/// The JVM runs on a thread of its own. `register` runs on that thread after the JVM exists and
/// before the main class, to register native methods. A call to System.exit in the JVM ends the
/// process after the exit hook of the host has logged it, so this function does not return then.
pub fn run<F>(java_home: &JavaHome, spec: &LaunchSpec, register: F) -> Result<Outcome, JvmError>
where
    F: FnOnce(&mut jni::JNIEnv<'_>) -> jni::errors::Result<()> + Send + 'static,
{
    let _ = (java_home, spec, register);
    todo!("golem-jvm: run")
}
