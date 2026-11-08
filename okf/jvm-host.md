---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Spec
title: JVM host
description: The JVM host of Golem as it is built, how golem-jvm reads the launch of a Forge server as the java launcher of JDK 17 reads it, the options Golem adds, the thread, the hooks, the stop path, the errors and where golem run finds the host jar.
tags: [jvm, jni, launcher, forge, host, signals]
timestamp: 2026-11-08
---

# JVM host

The JVM host runs the Forge server inside the Golem process. The crate `golem-jvm` reads the
argument files of the server as the java launcher reads them, turns the options of the launcher
into the form that `JNI_CreateJavaVM` takes, loads `libjvm` of a Java home, creates the JVM on a
thread of its own and calls the main class until the server ends. The program `golem` wraps it in
`golem run`, with the signals, the plugins and the host jar of the [bridge](bridge.md).

The reference for every rule on this page is the launcher of JDK 17, the files `main.c`, `args.c`,
`java.c` and `wildcard.c` of `src/java.base/share/native/libjli`. The notebook `03_launch_args`
holds the host to the launcher of Temurin 17.0.21 case by case: 19 argument files, 18 launches, 15
launches that fail, 35 refused command lines and the real `user_jvm_args.txt` and `unix_args.txt` of
Forge 47.4.18, each read by `golem-devtool` and run by a probe program under java and in the
embedded JVM, on 2026-11-07 in the cloud container of the session. The verdict of the notebook
checks that this page states its numbers.

## golem run

```text
golem run [--server <dir>] [--java-home <dir>] [--host-jar <file>] [--budget-ms <n>] [-- <arg>...]
```

| Option | Default | What it is |
| --- | --- | --- |
| `--server` | The current directory | The directory of the Forge server |
| `--java-home` | `$JAVA_HOME` | The Java home whose `libjvm` runs the server |
| `--host-jar` | See [The host jar](#the-host-jar) | The jar of the host class of the bridge |
| `--budget-ms` | 1 | The budget of one call into a plugin, in milliseconds, `DEFAULT_BUDGET` of `golem-plugin` |
| `-- <arg>...` | None | Arguments of main behind those of `unix_args.txt`, such as `nogui` |

Every option also takes the form `--name=value`. The server directory and the Java home may hold
bytes that are not UTF-8, as a path of the system may, but the path of the host jar, the command,
the value of `--budget-ms` and the arguments of main after `--` must be UTF-8, since the JVM takes
the module path and the arguments of main as text, and one that is not is an error. `golem run`
resolves the server directory, the Java home and the host jar to absolute paths, changes into the
server directory, since the paths of `unix_args.txt` are relative to it, and reads the launch as
[The launch](#the-launch) describes. It puts `-Xrs` in front of the options and the host jar on the
module path, installs the plugin runtime with the core plugin as the handler of the bridge, starts
the watch of the signals and runs the JVM. It exits with 0 when main returned and `DestroyJavaVM`
answered 0, and with 1 otherwise. `golem source` prints the version, the repository and the commit
of the binary, which `build.rs` takes from the variable `GOLEM_COMMIT`, else from HEAD of the git
checkout of Golem, else names unknown. The checkout counts only when its work tree is the workspace
and tracks `crates/golem/build.rs`, since a copy of Golem inside the checkout of another project
would name a commit of that project, and while the crates, `Cargo.toml`, `Cargo.lock` or
`rust-toolchain.toml` differ from HEAD, also by a file that git does not track, the commit ends in
`-dirty`, for the tree of HEAD is not the source that runs. `golem version` and `golem help` print
the version and the usage.

## The launch

A Forge server starts with its `run.sh`:

```text
java @user_jvm_args.txt @libraries/net/minecraftforge/forge/1.20.1-47.4.18/unix_args.txt "$@"
```

`LaunchSpec::forge_server` builds the same command line. It finds the
`libraries/net/minecraftforge/forge/*/unix_args.txt` of the server directory and refuses a directory
with none. The installer of Forge puts a new version beside the old one and writes a `run.sh` that
names the new one, so of several it takes the one that a word `@<path>` of `run.sh` names, outside a
comment, and refuses a directory whose `run.sh` names none of them or several, or that has no
`run.sh`. It takes `@user_jvm_args.txt` in front when that file exists, and reads both relative to
the server directory with `LaunchSpec::from_launcher_args`. The launch is the options, the main
class `cpw.mods.bootstraplauncher.BootstrapLauncher` and the arguments of main.

### Argument files

`read_argfile` is the state machine of `args.c`, byte for byte.

- White space, the space, the tab, the form feed and the ends of lines, separates the tokens.
- Single and double quotes group, also in the middle of a token, so `pre"fix mid"post` is one token.
  The other quote inside quotes is a plain character.
- Inside quotes a backslash escapes the next character, with `\n`, `\r`, `\t` and `\f` for the
  control characters. A backslash at the end of a line inside quotes joins the next line without
  its leading white space. Outside quotes a backslash is a plain character.
- A quote that a line leaves open ends with the line, and one that the file leaves open ends with
  the file. The launcher reports no error for it, so Golem reports none either.
- A `#` outside quotes starts a comment to the end of the line, also in the middle of a token. The
  part of the token before the `#` is lost, but a quoted part before it stays and joins the next
  token, so `'x'abc#c` followed by `next` reads as `xnext`.
- The launcher reads a file in blocks of 4096 bytes, and the part of a token that lies before the
  end of a block survives a `#` as a quoted part does. Golem reads in the same blocks.
- Empty quotes make an empty token, except at the very end of a file without a line end, where the
  launcher drops them.
- A NUL byte is refused. The launcher cuts the token at it, which is never what a file means.
- A file that is not UTF-8 is refused, since the launch of Golem is text. The launcher hands the
  bytes on, and the JVM decodes them in the encoding of the platform, so under UTF-8 a byte of
  Latin-1, such as the é of `café`, reaches the server as the replacement character.

### The expansion of @files

- An argument `@name` before the main class stands for the tokens of the file `name`, relative to
  the server directory. A file that does not open is an error.
- `@@name` stands for the argument `@name`, and `@` alone is an argument.
- A token inside an argument file is never expanded, so `@other.txt` in a file is a literal
  argument, which before the main class becomes the main class itself.
- After the main class nothing is expanded, also when the main class came from a file.
- `--disable-@files` stops the expansion of the arguments after it.

The value of an option that takes a value after a space, such as `-cp` or `-p`, is no main class,
so the expansion goes on behind it.

### The options

`-client` and `-server` before the main class are dropped, also as `-J-client` and `-J-server`, as
the launcher does with the names of `jvm.cfg`. Then every argument up to the first one that does not
start with `-` is an option, that first one is the main class, and every argument after it belongs
to main. An option that takes a value takes the next argument unless that one starts with `-`.

| Launcher form | Form of `JNI_CreateJavaVM` |
| --- | --- |
| `-p X`, `--module-path X` | `--module-path=X` |
| `--upgrade-module-path X`, `--add-modules X`, `--limit-modules X` | The same name with `=X` |
| `--add-opens X`, `--add-exports X`, `--add-reads X`, `--patch-module X` | The same name with `=X` |
| `--enable-native-access X` | `--enable-native-access=X` |
| `-cp X`, `-classpath X`, `--class-path X`, `--class-path=X` | `-Djava.class.path=X` |
| `-verbosegc`, `-noclassgc`, `-debug` | `-verbose:gc`, `-Xnoclassgc`, `-Xdebug` |
| `-t`, `-tm` | `-Xt`, `-Xtm`, which the JVM of 17 refuses |
| `-verify`, `-Xfuture`, `-verifyremote`, `-noverify` | `-Xverify:all`, `:all`, `:remote`, `:none` |
| `-ss...`, `-ms...`, `-mx...` | `-Xss...`, `-Xms...`, `-Xmx...` |
| `-oss...` | `-Xoss...`, which the JVM of 17 refuses |
| `-Xdiag` | `-Dsun.java.launcher.diag=true` |
| `--show-module-resolution` | `-Djdk.module.showModuleResolution=true` |
| `-checksource`, `-cs`, `-noasyncgc`, `--disable-@files` | Dropped |
| Every other option, such as `-Xmx4G`, `-D...`, `-XX:...`, `-javaagent:...` | Unchanged |

A class path takes the wildcards of the launcher: an entry `*` or one that ends in `/*` and names no
file stands for the files of its directory that end in `.jar` or `.JAR`, in the order the directory
lists them. A launch without `-cp` and without `-Djava.class.path` gets `-Djava.class.path=.` behind
its options. An option without its value is an error with the message of the launcher, such as
`-cp requires class path specification`.

Golem refuses with a clear error what only the launcher can do or what Golem does not do:

| Refused | Why |
| --- | --- |
| `-jar` | Golem starts a main class, so the jar goes on the class path |
| `-m`, `--module`, `--module=` | Golem starts a main class |
| `--source`, a main class that names a `.java` file | Golem compiles no source |
| `-version`, `--version`, `-showversion`, `--show-version`, `-fullversion`, `--full-version` | The launcher prints them |
| `-help`, `-h`, `-?`, `--help`, `-X`, `--help-extra` | The launcher prints them |
| `--list-modules`, `--describe-module`, `-d`, `--validate-modules`, `--show-resolved-modules` | They report on the modules and exit |
| `-XshowSettings`, `--dry-run`, `-splash:` | The launcher prints them, or a server needs none |
| `-XXaltjvm=`, `-J-XXaltjvm=`, `-version:`, `-jre-restrict-search`, `-jre-no-restrict-search` | They choose another JVM or JRE |
| No main class | There is nothing to run |

The variables `JDK_JAVA_OPTIONS` and `CLASSPATH` belong to the launcher and do not apply: the launch
is the files of the server alone, so the environment of a shell does not change what the server
runs. The variables that the JVM reads itself, `JAVA_TOOL_OPTIONS` and `_JAVA_OPTIONS`, apply inside
`JNI_CreateJavaVM` as they do under the launcher.

## The options Golem adds

| Option | Added by | Why |
| --- | --- | --- |
| `-Xrs` | `golem run`, in front | The JVM leaves `SIGINT`, `SIGTERM` and `SIGHUP` to the host |
| The host jar on `--module-path` | `golem run` | The bridge mod finds the host class in the boot layer |
| `-Djava.class.path=.` | The launch, when it has no class path | The launcher does the same |
| `-Dsun.java.command=<main class> <arguments>` | `run`, behind the options | The launcher sets it, and tools such as jps read it |
| `-Dsun.java.launcher=SUN_STANDARD` | `run`, behind the options | The launcher sets it |
| `exit` and `abort` | `run`, last | The hooks of the host, see [The hooks](#the-hooks) |

`add_module_path` appends the jar to the last `--module-path`, which is the one the JVM takes, or
sets `--module-path=<jar>` when there is none. The jar has no `module-info.class` and names itself
`com.skytekx.golem.host` in its manifest, so with the `--add-modules ALL-MODULE-PATH` of Forge it
becomes an automatic module of the boot layer that exports its package.

`-Dsun.java.command` and `-Dsun.java.launcher` stand behind the options of the launch, as under the
launcher, so they win over the same properties in a launch. HotSpot leaves `-Djava.class.path`,
`-Dsun.java.command`, `-Dsun.java.launcher` and every property that starts with them out of
`getInputArguments`, but it records the hooks, so in the embedded JVM `getInputArguments` ends with
`exit` and `abort`. Minecraft prints only the options that start with `-X` in its crash reports, so
the hooks never show there.

## The thread

`run` loads `lib/server/libjvm.so` of the Java home, or `lib/server/libjvm.dylib` on macOS, as the
launcher does, with `RTLD_NOW | RTLD_GLOBAL`, and never unloads it, since threads of the JVM may
outlive `DestroyJavaVM`. It creates the JVM with `JNI_VERSION_1_8` on a thread named `golem-jvm`,
never on the main thread of the process, which macOS forbids for a JVM. The stack of the thread is
8 MiB, or the size of the last `-Xss` when that is larger. The launcher gives its main thread the
size of `-Xss`, or the default of the JVM, 1 MiB on Linux on x86_64 and 2040 KiB on aarch64. No run
loop of AppKit runs on the main thread, so on macOS the server runs headless:
`-Djava.awt.headless=true` comes from the options in `user_jvm_args.txt`, as the JVM array of
`ServerMac.sh` sets it for Arclight, since golem adds no such option itself.

On that thread `run` does what `JavaMain` of `java.c` does.

1. It calls the function that registers the native methods, which `golem run` makes
   `golem_bridge::register_natives`, under `catch_unwind`, so a panic never crosses into the JVM.
2. It loads the main class through `sun.launcher.LauncherHelper.checkAndLoadMain`, the class the
   launcher uses, so the class resolves through the system class loader with the checks and the
   messages of the launcher. A class that does not load ends the process with status 1 through
   `System.exit`, after the message of the launcher.
3. It turns the arguments into strings of the platform encoding with
   `LauncherHelper.makePlatformString` and calls the static `main(String[])`. In Java the thread is
   named `main`, as under the launcher.
4. After main it detaches the thread and calls `DestroyJavaVM`, which waits until every thread that
   is no daemon has ended and then runs the shutdown hooks. An exception that main threw stays
   pending until the thread detaches, so the uncaught exception handler of the JVM prints it as
   under the launcher, and `run` reports its `toString` in the outcome.

A process holds one JVM, so `run` runs once.

## The hooks

| Hook | When HotSpot calls it | What the host does |
| --- | --- | --- |
| `exit` | `System.exit` or `Runtime.halt`, with the status | Logs `[golem] the JVM exits with status N` on standard error, then HotSpot ends the process with that status |
| `abort` | A fatal error, or an error of the options that HotSpot finds while `JNI_CreateJavaVM` runs, before HotSpot aborts the process | Logs `[golem] the JVM aborts on a fatal error`, or `[golem] the JVM could not start, see its message above` while `JNI_CreateJavaVM` runs |

Both hooks catch every panic, so none unwinds into the JVM. `run` does not return after a hook,
since the process ends.

Before it creates the JVM, the host puts `SIGSEGV` and `SIGBUS` back to their default action. The
standard library of Rust holds both with its handler for stack overflows, and HotSpot keeps a
handler it finds as the next of its chain, so a crash in native code, such as the library of a mod,
would reach that handler and end the process with a bare `SIGSEGV`: no fatal error report, no
`hs_err` file and no abort hook. With the default action behind it, HotSpot reports the crash as
under the launcher, and `03_launch_args` holds a write to the address 0 through `Unsafe` to that.

## The stop path

Golem never stops the server with a signal. `-Xrs` keeps the JVM away from `SIGINT`, `SIGTERM` and
`SIGHUP`, and a thread `golem-signals` of the host waits for them with `signal-hook`. From the first
server tick until the server stopped, the bridge mod drains the queue of actions at the end of every
tick, and the first signal in that time queues the actions `command save-all flush` and
`command stop` and logs `[golem] SIGTERM: queued save-all flush and stop on the server thread`. The
bridge mod runs both commands as the console at the end of the next tick, so the stop is the same as
the one of an operator. A later signal logs that the stop is already queued and changes nothing,
since a second stop would only race the first. A server thread that hangs keeps the queued stop
waiting until `SIGKILL`, as under java, where the shutdown hook of the server waits for the server
thread without a bound.

A shutdown signal that golem started with ignored stays ignored, as HotSpot leaves it, so a server
under `nohup` keeps running on a hangup, and golem logs that at the start.

`SIGQUIT` stops nothing. The JVM prints the dump of every thread on it, but `-Xrs` takes that signal
from the JVM too, and its default action would end the process without saving. So the thread
`golem-signals` takes it as well and prints the dump on standard output itself, from the operation
`threadPrint` of the bean `DiagnosticCommand`, which is what `jcmd <pid> Thread.print` runs, and
logs `[golem] SIGQUIT: printed the dump of every thread`. The thread attaches to the JVM under its
own name, so the dump shows it as `golem-signals` and not as the `Thread-N` that the JNI gives a
thread that attaches without a name. `jcmd` and `jstack` attach as under java, since HotSpot starts
its attach listener at startup when `-Xrs` is set, with the socket `.java_pid<pid>` in `/tmp` on
Linux and in the temporary folder of the user on macOS.

Unlike java, golem cannot bring a removed socket back. A tool that finds the socket gone writes the
trigger `.attach_pid<pid>` into the working directory of the process or into `/tmp` and sends
`SIGQUIT`, on which the signal thread of HotSpot would create the socket again, but `-Xrs` takes
that thread away. Once the socket is removed, by hand or by a cleaner of `/tmp` that removes old
files, no tool attaches until golem restarts. A `SIGQUIT` that finds the trigger while the socket is
gone prints no dump, since `jstack` sends several, and logs
`[golem] SIGQUIT: a tool such as jcmd asks to attach, but the socket /tmp/.java_pid<pid> of the JVM is gone`
and why. On a host whose `systemd-tmpfiles` cleans `/tmp` by age, a line `x /tmp/.java_pid*` in a
file of `/etc/tmpfiles.d` keeps the socket. Giving `SIGQUIT` back to HotSpot needs the host to drop
`-Xrs` and take the shutdown signals through handlers of the JVM instead, which waits for a decision
of the owner.

When the server stops, its threads end, `DestroyJavaVM` returns 0 and `golem run` exits with 0.
This replaces the signal stop of Arclight, whose shutdown thread worked on chunks beside the server
thread, issue [#4](https://github.com/skytekx/golem/issues/4).

No tick drains the queue on a server without the bridge mod in `mods/`, before the first tick of the
server and after it stopped. A signal then logs
`[golem] SIGTERM: <reason>, so the JVM exits through the shutdown hook of the server`, where the
reason is `the bridge is not connected`, `the server does not tick yet` or `the server has stopped`,
and calls `System.exit` with 128 plus the number of the signal on a thread of its own, `golem-exit`,
that it attaches to the JVM. That is what the java launcher does on a signal: the shutdown hook of
the server stops it and saves the worlds, and the exit hook ends the process with the status, 143
for `SIGTERM`. Minecraft adds that hook when it starts the server thread, so a signal while the mods
load ends the JVM at once, and one while the worlds load lets them load and then saves them, both as
under java. After the stop, when a thread of a mod that is no daemon keeps the JVM alive, a signal
ends it as under java, also when an earlier signal queued a stop that no tick ran. `System.exit`
runs the shutdown hooks on the thread that calls it, so while they save the worlds, `golem-signals`
still prints the dump on `SIGQUIT` and logs a later signal as
`[golem] SIGINT: the JVM already exits through the shutdown hook of the server`. A signal before the
JVM exists ends golem at once with that status, since nothing is loaded that could be saved.

## The errors

An `ArgError` names the file and the argument where there is one.

| Error | Message |
| --- | --- |
| An @file that does not open | ``could not open `name': <reason>`` |
| A file larger than 2147483647 bytes, not UTF-8 or with a NUL byte | `name: <reason>` |
| An option without its value | The message of the launcher, such as `-p requires module path specification` |
| A refused option | `<option> is not supported by Golem: <reason>` |
| No main class | `the arguments name no main class, so there is nothing to run` |
| No Forge server, or several of which `run.sh` names none or several | `<dir> holds no Forge server: ...`, `<dir> holds several Forge servers, and run.sh does not name one of them alone ...` |

A `JvmError` says why the JVM did not run: `libjvm` does not load or holds no `JNI_CreateJavaVM`, an
option holds a NUL byte, registering the native methods failed or panicked, the thread could not
start or panicked, or `JNI_CreateJavaVM` answered an error, which HotSpot explains on standard
error before.

| Code | Meaning |
| --- | --- |
| `JNI_ERR` | The JVM could not start, for example with a stack that `-Xss` makes too small or an option it does not know, such as `Unrecognized option: -Xt` |
| `JNI_EDETACHED` | The thread is not attached |
| `JNI_EVERSION` | The JVM does not support JNI 1.8 |
| `JNI_ENOMEM` | The JVM ran out of memory while it started |
| `JNI_EEXIST` | A JVM already runs in the process |
| `JNI_EINVAL` | The JVM refused a flag of `-XX:`, such as `Unrecognized VM option 'NoSuchFlag'` |

`golem run` prints `golem: <error>` and exits with 1 for an error before the JVM starts, such as a
missing Java home, and logs `[golem] the JVM did not run: <error>` for a `JvmError`. HotSpot ends
the process itself inside `JNI_CreateJavaVM` on some errors of the options, such as an initial heap
above the maximum or an `-Xmx` that it cannot reserve, after its message and with the status 1, as
under java. No `JvmError` follows then, and the abort hook logs
`[golem] the JVM could not start, see its message above`, so the line does not read as a crash.

## The host jar

`golem run` takes the first of these that is given or exists.

1. `--host-jar <file>`, which must name a file that exists.
2. The variable `GOLEM_HOST_JAR`, which must name a file that exists.
3. `golem-host.jar` in the directory of the `golem` program.
4. `golem-host.jar` in the server directory.

Without any of them `golem run` fails with the error `no golem-host.jar`, which names the four
ways. A directory given in place of the jar fails with `the host jar <path> is not a file` before
the JVM starts, since the JVM would read a directory on the module path as a directory of modules.
The path must be UTF-8, since the module path is an option string of the JVM.

## Where Golem differs from the launcher

Each difference is on purpose, and `03_launch_args` checks that it is the only one.

| Difference | Why | Issue |
| --- | --- | --- |
| `getInputArguments` ends with `exit` and `abort` | The hooks are options of `JNI_CreateJavaVM`, and HotSpot records every option | None |
| `--disable-@files` works | The launcher of Java 17 passes the option on to the JVM, which refuses it as unrecognized | [#7](https://github.com/skytekx/golem/issues/7) |
| A NUL byte in an argument file is an error | The launcher cuts the token at the byte | [#6](https://github.com/skytekx/golem/issues/6) |
| An argument file that is not UTF-8 is an error | The launcher hands its bytes to the JVM, which turns a byte that is not UTF-8 into the replacement character without an error | None |
| `JDK_JAVA_OPTIONS` and `CLASSPATH` do not apply | The launch is the files of the server alone | [#5](https://github.com/skytekx/golem/issues/5) |
| No warnings for `-checksource`, `-cs`, `-noasyncgc` and `-Xfuture` | A library of the host prints no lines of the launcher, and the options act the same | None |

Related: [Bridge](bridge.md), [Plan](plan.md#architecture), [Index](index.md).
