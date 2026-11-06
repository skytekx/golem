// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The launch of a JVM from the arguments of the java launcher, after main.c, args.c, java.c and
 * wildcard.c in src/java.base/share/native/libjli of JDK 17.
 *
 * The launcher works in three steps, and this file takes them in the same order. It expands the
 * @files on the command line up to the main class. It drops -client and -server, which choose a
 * JVM in jvm.cfg. Then it reads the options up to the main class: a launcher option with a value
 * after a space becomes the form with = that JNI_CreateJavaVM takes, a class path becomes the
 * property java.class.path with its wildcards expanded, an old form gets its new name, and every
 * other option passes as it is. What only the launcher can do, such as printing its help, or
 * what Golem does not do, such as starting a jar, is refused with an error that names it.
 *
 * The environment variables of the launcher, JDK_JAVA_OPTIONS and CLASSPATH, do not apply: the
 * launch is the arguments alone. The ones the JVM reads itself, such as JAVA_TOOL_OPTIONS, apply
 * inside JNI_CreateJavaVM as they do under the launcher.
 */

use std::path::Path;

use crate::{
    ArgError,
    LaunchSpec,
    argfile, //
};

/// The separator of a class path or a module path on Linux and macOS, where Golem runs.
pub(crate) const PATH_SEPARATOR: char = ':';

/// The option that names the module path, in the form that `JNI_CreateJavaVM` takes.
const MODULE_PATH: &str = "--module-path=";

/// The largest argument file the launcher reads, `MAX_ARGF_SIZE` of args.c.
const MAX_ARGFILE_SIZE: u64 = 0x7fff_ffff;

/// The options of the module system whose value may follow after a space, `IsModuleOption` of
/// java.c.
fn is_module_option(name: &str) -> bool {
    matches!(
        name,
        "--module-path"
            | "-p"
            | "--upgrade-module-path"
            | "--add-modules"
            | "--enable-native-access"
            | "--limit-modules"
            | "--add-exports"
            | "--add-opens"
            | "--add-reads"
            | "--patch-module"
    )
}

/// The options of the module system in the form with =, `IsLongFormModuleOption` of java.c, which
/// leaves out --add-opens= as the launcher does.
fn is_long_form_module_option(name: &str) -> bool {
    [
        "--module-path=",
        "--upgrade-module-path=",
        "--add-modules=",
        "--enable-native-access=",
        "--limit-modules=",
        "--add-exports=",
        "--add-reads=",
        "--patch-module=",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
}

/// The class path options, `IsClassPathOption` of java.c.
fn is_class_path_option(name: &str) -> bool {
    matches!(name, "-classpath" | "-cp" | "--class-path")
}

/// The launcher options whose value may follow after a space, `IsLauncherOption` of java.c.
fn is_launcher_option(name: &str) -> bool {
    is_class_path_option(name)
        || matches!(
            name,
            "--module" | "-m" | "--describe-module" | "-d" | "--source"
        )
}

/// The options whose value may follow after a space, `IsWhiteSpaceOption` of java.c.
fn is_white_space_option(name: &str) -> bool {
    is_module_option(name) || is_launcher_option(name)
}

/// The argument of the launcher after its @files are expanded, the work of `JLI_PreprocessArg`
/// and `checkArg` in args.c.
struct Expansion {
    arguments: Vec<String>,
    /// The main class has been seen, so no later argument is expanded.
    main_seen: bool,
    /// The last option takes a value after a space, so the next argument is that value and not
    /// the main class.
    expecting_value: bool,
    /// --disable-@files has been seen.
    stopped: bool,
}

impl Expansion {
    fn new() -> Self {
        Self {
            arguments: Vec::new(),
            main_seen: false,
            expecting_value: false,
            stopped: false,
        }
    }

    /// Takes one argument as `checkArg` does, which finds the main class and --disable-@files.
    fn check(&mut self, argument: String) {
        if argument.starts_with('-') {
            self.expecting_value = false;
            if is_white_space_option(&argument) {
                // The value of -jar, --module and -m is the main entry, so no value is
                // expected and the next argument ends the expansion.
                self.expecting_value = !matches!(argument.as_str(), "-jar" | "--module" | "-m");
            } else if argument == "--disable-@files" {
                self.stopped = true;
            } else if argument.starts_with("--module=") {
                self.main_seen = true;
            }
        } else {
            if !self.expecting_value {
                self.main_seen = true;
            }
            self.expecting_value = false;
        }
        self.arguments.push(argument);
    }

    /// Takes one argument of the command line, as `JLI_PreprocessArg` does.
    fn take(&mut self, argument: &str, base: &Path) -> Result<(), ArgError> {
        if self.main_seen {
            self.arguments.push(argument.to_owned());
            return Ok(());
        }
        let Some(rest) = argument.strip_prefix('@') else {
            self.check(argument.to_owned());
            return Ok(());
        };
        if self.stopped || rest.is_empty() {
            self.check(argument.to_owned());
        } else if rest.starts_with('@') {
            self.check(rest.to_owned());
        } else {
            for token in read(rest, base)? {
                self.check(token);
            }
        }
        Ok(())
    }
}

/// The tokens of the argument file `name`, relative to `base`.
fn read(name: &str, base: &Path) -> Result<Vec<String>, ArgError> {
    let path = base.join(name);
    let size = std::fs::metadata(&path)
        .map_err(|error| ArgError::new(format!("could not open `{name}': {error}")))?
        .len();
    if size > MAX_ARGFILE_SIZE {
        return Err(ArgError::new(format!(
            "{name}: an argument file must not be larger than {MAX_ARGFILE_SIZE} bytes"
        )));
    }
    let bytes = std::fs::read(&path)
        .map_err(|error| ArgError::new(format!("could not open `{name}': {error}")))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| ArgError::new(format!("{name}: the argument file is not UTF-8")))?;
    argfile::tokens(&text).map_err(|error| ArgError::new(format!("{name}: {}", error.message)))
}

/// Drops -client and -server before the main class, as `CheckJvmType` of java.c does with the
/// names of jvm.cfg, and refuses the choice of another JVM.
fn drop_jvm_type(arguments: Vec<String>) -> Result<Vec<String>, ArgError> {
    let mut kept = Vec::with_capacity(arguments.len());
    let mut rest = arguments.into_iter();
    while let Some(argument) = rest.next() {
        if is_white_space_option(&argument) {
            kept.push(argument);
            kept.extend(rest.next());
            continue;
        }
        if !argument.starts_with('-') {
            kept.push(argument);
            break;
        }
        if argument.starts_with("-XXaltjvm=") || argument.starts_with("-J-XXaltjvm=") {
            return Err(refused(
                &argument,
                "it chooses another JVM than the one of the Java home",
            ));
        }
        // KnownVMIndex of java.c strips a leading -J before it compares with jvm.cfg.
        let name = argument.strip_prefix("-J").unwrap_or(&argument);
        if name != "-server" && name != "-client" {
            kept.push(argument);
        }
    }
    kept.extend(rest);
    Ok(kept)
}

/// An option that Golem refuses, with the reason.
fn refused(option: &str, reason: &str) -> ArgError {
    ArgError::new(format!("{option} is not supported by Golem: {reason}"))
}

/// The launcher options that Golem refuses, with the reason, or None for any other option.
fn refusal(option: &str) -> Option<&'static str> {
    let reason = match option {
        "-jar" => {
            "Golem starts a main class, so put the jar on the class path and name its main class"
        }
        "--module" | "-m" => {
            "Golem starts a main class, so name it and put its module on the module path"
        }
        "--source" => "Golem does not compile a source file",
        "--list-modules"
        | "--describe-module"
        | "-d"
        | "--validate-modules"
        | "--show-resolved-modules" => {
            "it reports on the modules and exits, which is work of the java launcher"
        }
        "-help" | "-h" | "-?" | "--help" | "-X" | "--help-extra" => {
            "it prints the help of the java launcher"
        }
        "-version" | "--version" | "-showversion" | "--show-version" | "-fullversion"
        | "--full-version" => {
            "it prints the version of the java launcher, which java -version of the Java home prints as well"
        }
        "--dry-run" => "Golem has no dry run and always calls the main class",
        "-jre-restrict-search" | "-jre-no-restrict-search" => {
            "Java 9 and later have no other JRE to search"
        }
        _ => {
            return if option.starts_with("--module=") {
                Some("Golem starts a main class, so name it and put its module on the module path")
            } else if option.starts_with("--source=") || option.starts_with("--source ") {
                Some("Golem does not compile a source file")
            } else if option.starts_with("--describe-module=") {
                Some("it reports on the modules and exits, which is work of the java launcher")
            } else if option == "-XshowSettings" || option.starts_with("-XshowSettings:") {
                Some("it prints the settings of the java launcher")
            } else if option.starts_with("-splash:") {
                Some("a server shows no splash screen")
            } else if option.starts_with("-version:") {
                Some("Java 9 and later run no other version of Java")
            } else {
                None
            };
        }
    };
    Some(reason)
}

/// The new name of an old form of an option, as java.c translates it, or None.
fn translation(option: &str) -> Option<String> {
    let new = match option {
        "-Xdiag" => "-Dsun.java.launcher.diag=true",
        "--show-module-resolution" => "-Djdk.module.showModuleResolution=true",
        "-verbosegc" => "-verbose:gc",
        "-t" => "-Xt",
        "-tm" => "-Xtm",
        "-debug" => "-Xdebug",
        "-noclassgc" => "-Xnoclassgc",
        "-Xfuture" | "-verify" => "-Xverify:all",
        "-verifyremote" => "-Xverify:remote",
        "-noverify" => "-Xverify:none",
        _ => {
            return ["-ss", "-oss", "-ms", "-mx"]
                .iter()
                .any(|prefix| option.starts_with(prefix))
                .then(|| format!("-X{}", &option[1..]));
        }
    };
    Some(new.to_owned())
}

/// The options that Golem drops without passing them on: the ones the launcher no longer
/// supports, and --disable-@files, which only stops the expansion of @files. The launcher of JDK
/// 17 passes --disable-@files on to the JVM, which refuses it as an unrecognized option.
fn is_dropped(option: &str) -> bool {
    matches!(
        option,
        "-checksource" | "-cs" | "-noasyncgc" | "--disable-@files"
    )
}

/// The class path with its wildcards expanded, `JLI_WildcardExpandClasspath` of wildcard.c.
///
/// An entry that is a star alone or ends in /* and names no file stands for the jars of its
/// directory, relative to `base`, in the order the directory lists them. A class path whose
/// wildcards match no jar stays as it is.
fn expand_wildcards(class_path: &str, base: &Path) -> String {
    if !class_path.contains('*') {
        return class_path.to_owned();
    }
    let mut entries = Vec::new();
    let mut expanded = false;
    for entry in class_path.split(PATH_SEPARATOR) {
        match wildcard_jars(entry, base) {
            Some(jars) if !jars.is_empty() => {
                entries.extend(jars);
                expanded = true;
            }
            _ => entries.push(entry.to_owned()),
        }
    }
    if expanded {
        entries.join(&PATH_SEPARATOR.to_string())
    } else {
        class_path.to_owned()
    }
}

/// The jars that a wildcard entry of a class path stands for, or None when it is no wildcard.
fn wildcard_jars(entry: &str, base: &Path) -> Option<Vec<String>> {
    let directory = entry.strip_suffix('*')?;
    if !(directory.is_empty() || directory.ends_with('/')) || base.join(entry).exists() {
        return None;
    }
    let listing = std::fs::read_dir(base.join(if directory.is_empty() { "." } else { directory }));
    let jars = listing
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|file| file.file_name().into_string().ok())
        .filter(|name| is_jar_name(name))
        .map(|name| format!("{directory}{name}"))
        .collect();
    Some(jars)
}

/// Whether a file name ends in .jar or .JAR, `isJarFileName` of wildcard.c, which takes no other
/// case.
fn is_jar_name(name: &str) -> bool {
    name.strip_suffix("jar")
        .or_else(|| name.strip_suffix("JAR"))
        .is_some_and(|stem| stem.ends_with('.'))
        && !name.contains(PATH_SEPARATOR)
}

/// The options of the launcher up to the main class, read as `ParseArguments` of java.c reads
/// them.
struct Parser {
    options: Vec<String>,
    /// A class path option set the mode of the launch to a main class.
    class_mode: bool,
    /// A class path is set, by a class path option or by -Djava.class.path.
    have_class_path: bool,
}

impl Parser {
    /// Reads one option and the value it takes from `rest`, as `GetOpt` and `ParseArguments` do.
    fn take(
        &mut self,
        argument: &str,
        rest: &mut std::iter::Peekable<std::vec::IntoIter<String>>,
        base: &Path,
    ) -> Result<(), ArgError> {
        let next_is_value = rest.peek().is_some_and(|next| !next.starts_with('-'));
        let mut option = argument;
        let mut value = None;
        let mut long_form = false;
        if is_launcher_option(argument) || is_module_option(argument) {
            if next_is_value {
                value = rest.next();
            }
            if argument == "-p" {
                option = "--module-path";
            }
        } else if argument.starts_with("--")
            && let Some((_, after)) = argument.split_once('=')
        {
            value = Some(after.to_owned());
            long_form = ![
                "--describe-module=",
                "--module=",
                "--class-path=",
                "--source=",
            ]
            .iter()
            .any(|prefix| argument.starts_with(prefix));
        }
        let value = value.as_deref();
        let has_value = value.is_some_and(|value| !value.is_empty());

        if let Some(reason) = refusal(argument) {
            return Err(refused(argument, reason));
        }
        if is_class_path_option(argument) || argument.starts_with("--class-path=") {
            let Some(value) = value else {
                return Err(ArgError::new(format!(
                    "{argument} requires class path specification"
                )));
            };
            self.options.push(format!(
                "-Djava.class.path={}",
                expand_wildcards(value, base)
            ));
            self.class_mode = true;
            self.have_class_path = true;
        } else if has_value {
            // Every launcher option that takes a value is handled above, so a value here
            // belongs to an option of the JVM in the form with =, or to a module option.
            if long_form {
                self.options.push(option.to_owned());
            } else {
                self.options
                    .push(format!("{option}={}", value.unwrap_or_default()));
            }
        } else if matches!(argument, "--module-path" | "-p" | "--upgrade-module-path") {
            return Err(ArgError::new(format!(
                "{argument} requires module path specification"
            )));
        } else if is_module_option(argument) || is_long_form_module_option(argument) {
            return Err(ArgError::new(format!(
                "{argument} requires modules to be specified"
            )));
        } else if let Some(new) = translation(argument) {
            self.options.push(new);
        } else if !is_dropped(argument) {
            if argument.starts_with("-Djava.class.path=") {
                self.have_class_path = true;
            }
            self.options.push(argument.to_owned());
        }
        Ok(())
    }
}

/// Whether the main class names a source file, `IsSourceFile` of java.c, which the launcher would
/// compile. The launcher takes the suffix .java in lower case alone.
fn is_source_file(main_class: &str, base: &Path) -> bool {
    main_class
        .strip_suffix("java")
        .is_some_and(|stem| stem.ends_with('.'))
        && base.join(main_class).exists()
}

/// The launch that the arguments of the java launcher describe.
pub(crate) fn from_launcher_args(
    arguments: &[String],
    base: &Path,
) -> Result<LaunchSpec, ArgError> {
    let mut expansion = Expansion::new();
    for argument in arguments {
        expansion.take(argument, base)?;
    }
    let arguments = drop_jvm_type(expansion.arguments)?;

    let mut parser = Parser {
        options: Vec::new(),
        class_mode: false,
        have_class_path: false,
    };
    let mut rest = arguments.into_iter().peekable();
    while let Some(argument) = rest.next_if(|argument| argument.starts_with('-')) {
        parser.take(&argument, &mut rest, base)?;
    }
    let Some(main_class) = rest.next() else {
        return Err(ArgError::new(
            "the arguments name no main class, so there is nothing to run",
        ));
    };
    if is_source_file(&main_class, base) {
        return Err(refused(&main_class, "Golem does not compile a source file"));
    }
    if !parser.class_mode && !parser.have_class_path {
        parser.options.push("-Djava.class.path=.".to_owned());
    }
    Ok(LaunchSpec {
        options: parser.options,
        main_class,
        arguments: rest.collect(),
    })
}

/// The path of the argument file of the Forge server in `server`, relative to it.
fn forge_args_file(server: &Path) -> Result<String, ArgError> {
    const FORGE: &str = "libraries/net/minecraftforge/forge";
    let none = || {
        ArgError::new(format!(
            "{} holds no Forge server: no {FORGE}/*/unix_args.txt",
            server.display()
        ))
    };
    let listing = match std::fs::read_dir(server.join(FORGE)) {
        Ok(listing) => listing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(none()),
        Err(error) => {
            return Err(ArgError::new(format!(
                "could not read {}: {error}",
                server.join(FORGE).display()
            )));
        }
    };
    let mut found = Vec::new();
    for entry in listing.filter_map(Result::ok) {
        if entry.path().join("unix_args.txt").is_file() {
            let version = entry.file_name().into_string().map_err(|name| {
                ArgError::new(format!("the Forge version {} is not UTF-8", name.display()))
            })?;
            found.push(format!("{FORGE}/{version}/unix_args.txt"));
        }
    }
    found.sort();
    match found.len() {
        0 => Err(none()),
        1 => Ok(found.remove(0)),
        _ => Err(ArgError::new(format!(
            "{} holds several Forge servers, so it is not clear which to run: {}",
            server.display(),
            found.join(", ")
        ))),
    }
}

/// The launch of the Forge server in `server`, as run.sh starts it.
pub(crate) fn forge_server(server: &Path) -> Result<LaunchSpec, ArgError> {
    let args_file = forge_args_file(server)?;
    let mut arguments = Vec::new();
    if server.join("user_jvm_args.txt").is_file() {
        arguments.push("@user_jvm_args.txt".to_owned());
    }
    arguments.push(format!("@{args_file}"));
    from_launcher_args(&arguments, server)
}

/// Appends `jar` to the module path that the options set last, or sets it.
pub(crate) fn add_module_path(spec: &mut LaunchSpec, jar: &Path) {
    let jar = jar.to_string_lossy();
    // The JVM takes the last --module-path, so that is the one to extend.
    match spec
        .options
        .iter_mut()
        .rev()
        .find(|option| option.starts_with(MODULE_PATH))
    {
        Some(option) if option.len() > MODULE_PATH.len() => {
            option.push(PATH_SEPARATOR);
            option.push_str(&jar);
        }
        Some(option) => option.push_str(&jar),
        None => spec.options.push(format!("{MODULE_PATH}{jar}")),
    }
}
