// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The subcommands of golem-bridge and golem-plugin: plugins feeds a script of native calls to
 * the plugin runtime with the core plugin and test plugins, as the mod would, and prints what
 * the natives answered and queued. actions-encode prints the wire form of actions, tick-stats
 * the statistics of a window of ticks at given instants, and natives the table of native
 * methods that the host registers.
 */

//! The primitives of golem-bridge and golem-plugin.
//!
//! `plugins <script.json>` runs every step of a script through the functions of
//! `golem_bridge::host`, which the native methods call after converting their arguments, so the
//! path is the one of the server thread without the JVM. The script names a handler, the
//! plugins of the runtime and the steps:
//!
//! ```text
//! {
//!   "handler": "runtime",            runtime (the default), panicking or none
//!   "budgetMs": 20,                  the budget of a call, by default 1 ms
//!   "source": {"version": "0.1.0", "repository": "...", "commit": "..."},   or "build"
//!   "plugins": [{"kind": "core"}, {"kind": "echo", "name": "echo-a", "commands": ["a"]}],
//!   "game": {"players": [], "dimensions": []},   the scripted game of the module game
//!   "steps": [{"op": "chat", "uuid": "...", "name": "Alex", "message": "hello"}]
//! }
//! ```
//!
//! Every native step hands the natives the scripted game, or a game that answers nothing when
//! the script names none, and the natives hand it to the plugins behind the guard of the server
//! thread, as they do the game of the mod.
//!
//! A step is a native method by its Java name with its Java parameters, where a missing parameter
//! is null: `version`, `serverStarting`, `serverStarted`, `serverStopping`, `serverStopped`, `tick`
//! (with `nanos`, and `repeat` and `sleepMs` of the devtool), `playerJoined`, `playerLeft`, `chat`,
//! `death`, `respawn` (with `endConquered`, false when left out), `commandNames`, `command` (with
//! `level`, 4 when left out), `permission` and `drain`. A `death` step takes the `dimension`, `x`,
//! `y`, `z`, `yaw` and `pitch` of the place, each 0 when left out, and no dimension stands for
//! null. The devtool adds `stats`, `budgets`, `push` (an action straight into the queue), `install`
//! (a fresh handler of the same script), `sleep` and `together`, which runs its `steps` each on a
//! thread of its own, the next one `staggerMs` after the one before, as Forge calls `chat` on a
//! worker thread beside the server thread. A step of `together` is a native method, `stats` or
//! `sleep`.
//!
//! The test plugins are these. `echo` broadcasts the name of every event but a tick, and the
//! location of a death, unless it is `quiet`, and answers its `commands` with a line, a message
//! to a player sender and a console command. `cancel` cancels a chat message that holds its
//! `word` and answers cancel to every other event, which the runtime must ignore. `panic` panics
//! on the events named `on`, on a permission when `on` is `permission`, and in its `commands`.
//! `slow` sleeps `sleepMs` on the first `times` events named `on`. `reenter` calls the native
//! `commandNames` from inside its handler on the events named `on` and broadcasts how many names
//! it got. `grant` grants the nodes of its `grants` and denies those of its `denies`, and with
//! `operators` answers every other node as the live server answers a node that nothing names,
//! true for a UUID of `operators` and false for every other player. `mover`
//! answers its `commands` with the helpers of the context that teleport a player to its place
//! `to`, to a place that is no number and to one beyond the bounds of the world, and name them in
//! the player list. `ask` asks the game, as the module game describes.

use std::{
    fs,
    thread,
    time::{
        Duration,
        Instant, //
    }, //
};

use golem_bridge::{
    Action,
    Actions,
    Event,
    Game,
    Handler,
    Location,
    NoGame,
    Sender,
    TickStats,
    TickWindow,
    Verdict,
    host, //
};
use golem_plugin::{
    CommandSpec,
    Context,
    CorePlugin,
    DEFAULT_BUDGET,
    Plugin,
    Runtime,
    Source, //
};
use serde::{
    Deserialize,
    Serialize, //
};
use serde_json::{
    Value,
    json, //
};

use crate::{
    Subcommand,
    game::{
        Ask,
        GameSpec,
        LocationSpec,
        ScriptedGame,
        location_json, //
    },
};

pub const PLUGINS: Subcommand = (
    "plugins",
    "<script.json> the answers of the plugin runtime",
    plugins,
);
pub const ACTIONS_ENCODE: Subcommand = (
    "actions-encode",
    "<actions.json> the wire form of actions",
    actions_encode,
);
pub const TICK_STATS: Subcommand = (
    "tick-stats",
    "<ticks.json> the statistics of ticks at given instants",
    tick_stats,
);
pub const NATIVES: Subcommand = ("natives", "the native methods of the host class", natives);

/// The commit of the source of a script that names none.
const TEST_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// The repository of the source of a script that names none.
const TEST_REPOSITORY: &str = "https://github.com/skytekx/golem";

/// A script of the subcommand plugins.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Script {
    #[serde(default)]
    handler: HandlerKind,
    budget_ms: Option<f64>,
    source: Option<SourceSpec>,
    #[serde(default)]
    plugins: Vec<PluginSpec>,
    game: Option<GameSpec>,
    steps: Vec<Step>,
}

/// The handler that a script installs.
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
enum HandlerKind {
    /// The plugin runtime with the plugins of the script.
    #[default]
    Runtime,
    /// A handler that panics in every method.
    Panicking,
    /// No handler at all.
    None,
}

/// The source of the core plugin.
#[derive(Deserialize)]
#[serde(untagged)]
enum SourceSpec {
    /// The word build, for the source of this build.
    Build(String),
    /// A source given field by field.
    Given {
        version: String,
        repository: String,
        commit: String,
    },
}

/// A plugin of the runtime of a script.
#[derive(Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum PluginSpec {
    /// The core plugin with the source of the script.
    Core,
    /// A plugin that broadcasts the events and answers its commands.
    Echo {
        name: String,
        #[serde(default)]
        commands: Vec<String>,
        #[serde(default)]
        quiet: bool,
    },
    /// A plugin that cancels the chat messages that hold a word.
    Cancel { name: String, word: String },
    /// A plugin that panics on the events of one name and in its commands.
    Panic {
        name: String,
        on: String,
        #[serde(default)]
        commands: Vec<String>,
    },
    /// A plugin that sleeps on the first events of one name.
    Slow {
        name: String,
        on: String,
        sleep_ms: u64,
        #[serde(default = "one")]
        times: u32,
    },
    /// A plugin that calls a native from inside the handler on the events of one name.
    Reenter { name: String, on: String },
    /// A plugin that grants and denies nodes of the `PermissionAPI` of Forge, and answers every
    /// other node when it knows the operators.
    Grant {
        name: String,
        #[serde(default)]
        grants: Vec<String>,
        #[serde(default)]
        denies: Vec<String>,
        #[serde(default)]
        operators: Option<Vec<String>>,
    },
    /// A plugin that asks the game on its commands and on the events whose names `on` lists.
    Ask {
        name: String,
        #[serde(default)]
        commands: Vec<String>,
        #[serde(default)]
        on: Vec<String>,
    },
    /// A plugin that teleports players and names them in the player list on its commands.
    Mover {
        name: String,
        commands: Vec<String>,
        to: LocationSpec,
    },
}

/// The default of a count of the script.
fn one() -> u32 {
    1
}

/// The default level of a command of the script, the level of the console.
fn max_level() -> i32 {
    i32::from(golem_bridge::MAX_LEVEL)
}

/// A step of a script: a native method with its parameters, or a step of the devtool.
#[derive(Deserialize)]
#[serde(
    tag = "op",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Step {
    Version,
    ServerStarting {
        forge_version: Option<String>,
        minecraft_version: Option<String>,
    },
    ServerStarted,
    ServerStopping,
    ServerStopped,
    Tick {
        nanos: i64,
        #[serde(default = "one")]
        repeat: u32,
        #[serde(default)]
        sleep_ms: u64,
    },
    PlayerJoined {
        uuid: Option<String>,
        name: Option<String>,
    },
    PlayerLeft {
        uuid: Option<String>,
        name: Option<String>,
    },
    Chat {
        uuid: Option<String>,
        name: Option<String>,
        message: Option<String>,
    },
    Death {
        uuid: Option<String>,
        name: Option<String>,
        cause: Option<String>,
        dimension: Option<String>,
        #[serde(default)]
        x: f64,
        #[serde(default)]
        y: f64,
        #[serde(default)]
        z: f64,
        #[serde(default)]
        yaw: f32,
        #[serde(default)]
        pitch: f32,
    },
    Respawn {
        uuid: Option<String>,
        name: Option<String>,
        dimension: Option<String>,
        #[serde(default)]
        end_conquered: bool,
    },
    CommandNames,
    Command {
        name: Option<String>,
        sender_uuid: Option<String>,
        sender_name: Option<String>,
        #[serde(default = "max_level")]
        level: i32,
        arguments: Option<String>,
    },
    Permission {
        uuid: Option<String>,
        node: Option<String>,
    },
    Drain,
    Stats,
    Budgets,
    Push {
        action: ActionSpec,
    },
    Install,
    Sleep {
        ms: u64,
    },
    Together {
        #[serde(default)]
        stagger_ms: u64,
        steps: Vec<Step>,
    },
}

/// An action as JSON.
#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum ActionSpec {
    Command {
        command: String,
    },
    Message {
        uuid: String,
        text: String,
    },
    Broadcast {
        text: String,
    },
    Teleport {
        uuid: String,
        #[serde(flatten)]
        location: LocationSpec,
    },
    ListName {
        uuid: String,
        name: Option<String>,
    },
}

impl From<ActionSpec> for Action {
    fn from(spec: ActionSpec) -> Self {
        match spec {
            ActionSpec::Command { command } => Self::Command(command),
            ActionSpec::Message { uuid, text } => Self::Message { uuid, text },
            ActionSpec::Broadcast { text } => Self::Broadcast(text),
            ActionSpec::Teleport { uuid, location } => Self::Teleport {
                uuid,
                location: location.into(),
            },
            ActionSpec::ListName { uuid, name } => Self::ListName { uuid, name },
        }
    }
}

/// Answers an action as JSON, in the form of [`ActionSpec`].
fn action_json(action: &Action) -> Value {
    match action {
        Action::Command(command) => json!({"kind": "command", "command": command}),
        Action::Message { uuid, text } => json!({"kind": "message", "uuid": uuid, "text": text}),
        Action::Broadcast(text) => json!({"kind": "broadcast", "text": text}),
        Action::Teleport { uuid, location } => {
            let mut value = location_json(location);
            value["kind"] = json!("teleport");
            value["uuid"] = json!(uuid);
            value
        }
        Action::ListName { uuid, name } => json!({"kind": "listName", "uuid": uuid, "name": name}),
    }
}

/// Reads the JSON file that the only argument names.
pub fn read<T: for<'de> Deserialize<'de>>(name: &str, arguments: &[String]) -> Result<T, String> {
    let [path] = arguments else {
        return Err(format!("{name} takes one file"));
    };
    let text = fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{path}: {error}"))
}

/// Prints `result` as the answer of a subcommand and answers its exit status.
pub fn finish(name: &str, result: Result<Value, String>) -> i32 {
    match result {
        Ok(document) => {
            println!("{document:#}");
            0
        }
        Err(error) => {
            eprintln!("golem-devtool: {name}: {error}");
            1
        }
    }
}

fn plugins(arguments: &[String]) -> i32 {
    finish("plugins", read("plugins", arguments).and_then(run_script))
}

/// Runs a script and answers the answer of every step.
fn run_script(script: Script) -> Result<Value, String> {
    let budget = match script.budget_ms {
        Some(ms) if ms.is_finite() && ms >= 0.0 => Duration::from_secs_f64(ms / 1000.0),
        Some(ms) => return Err(format!("the budget {ms} ms is no duration")),
        None => DEFAULT_BUDGET,
    };
    let source = match script.source {
        None => Source {
            version: host::VERSION.to_owned(),
            repository: TEST_REPOSITORY.to_owned(),
            commit: TEST_COMMIT.to_owned(),
        },
        Some(SourceSpec::Build(word)) if word == "build" => Source::of_build(),
        Some(SourceSpec::Build(word)) => return Err(format!("the source {word} is unknown")),
        Some(SourceSpec::Given {
            version,
            repository,
            commit,
        }) => Source {
            version,
            repository,
            commit,
        },
    };
    let mut run = Run {
        kind: script.handler,
        budget,
        source,
        plugins: script.plugins,
        budgets: None,
        actions: host::actions(),
        game: script.game.map(ScriptedGame::new),
    };
    run.install();
    let answers: Vec<Value> = script
        .steps
        .into_iter()
        .enumerate()
        .map(|(index, step)| {
            let mut answer = run.step(step);
            answer["step"] = json!(index);
            answer
        })
        .collect();
    Ok(json!({
        "handler": run.kind_name(),
        "budgetMs": budget.as_secs_f64() * 1000.0,
        "steps": answers,
    }))
}

/// The state of a script between its steps.
struct Run {
    kind: HandlerKind,
    budget: Duration,
    source: Source,
    plugins: Vec<PluginSpec>,
    budgets: Option<golem_plugin::Budgets>,
    actions: Actions,
    /// The game of the script, or none for a game that answers nothing.
    game: Option<ScriptedGame>,
}

impl Run {
    /// Answers the game of the script, which a native step hands the natives.
    fn game(&self) -> &(dyn Game + Sync) {
        match &self.game {
            Some(game) => game,
            None => &NoGame,
        }
    }

    /// Answers the name of the handler, as the script names it.
    fn kind_name(&self) -> &'static str {
        match self.kind {
            HandlerKind::Runtime => "runtime",
            HandlerKind::Panicking => "panicking",
            HandlerKind::None => "none",
        }
    }

    /// Installs a fresh handler of the script, as the program golem does once.
    fn install(&mut self) {
        match self.kind {
            HandlerKind::Runtime => {
                let mut runtime = Runtime::new(self.budget);
                for spec in &self.plugins {
                    runtime.add(build_plugin(spec, &self.source));
                }
                self.budgets = Some(runtime.budgets_handle());
                self.actions = golem_bridge::install(Box::new(runtime));
            }
            HandlerKind::Panicking => {
                self.actions = golem_bridge::install(Box::new(Panicking));
            }
            HandlerKind::None => {}
        }
    }

    /// Runs one step and answers what its native answered.
    fn step(&mut self, step: Step) -> Value {
        if let Some(answer) = native(&step, self.game()) {
            return answer;
        }
        match step {
            Step::Budgets => answer("budgets", self.budgets_json()),
            Step::Push { action } => {
                self.actions.push(action.into());
                json!({"op": "push", "answer": null, "queued": self.actions.len()})
            }
            Step::Install => {
                self.install();
                json!({"op": "install", "answer": null, "queued": self.actions.len()})
            }
            Step::Together { stagger_ms, steps } => together(stagger_ms, &steps, self.game()),
            _ => unreachable!("native() answers every other step"),
        }
    }

    /// Answers the budgets as counts, which are the same on every run, and whether the longest
    /// call went over the budget.
    fn budgets_json(&self) -> Value {
        let Some(budgets) = &self.budgets else {
            return Value::Null;
        };
        let limit = u64::try_from(budgets.limit().as_nanos()).unwrap_or(u64::MAX);
        let rows: Vec<Value> = budgets
            .snapshot()
            .into_iter()
            .map(|(name, budget)| {
                json!({
                    "plugin": name,
                    "calls": budget.calls,
                    "overBudget": budget.over_budget,
                    "panics": budget.panics,
                    "longestOverBudget": budget.max_nanos > limit,
                })
            })
            .collect();
        json!(rows)
    }
}

/// Runs a step that is a native method, `stats` or `sleep`, with the game of the script, and
/// answers what it answered, or None for a step of the devtool that needs the state of the
/// script.
fn native(step: &Step, game: &(dyn Game + Sync)) -> Option<Value> {
    Some(match step {
        Step::Version => answer("version", host::version()),
        Step::ServerStarting {
            forge_version,
            minecraft_version,
        } => done("serverStarting", || {
            host::server_starting(forge_version.as_deref(), minecraft_version.as_deref(), game);
        }),
        Step::ServerStarted => done("serverStarted", || host::server_started(game)),
        Step::ServerStopping => done("serverStopping", || host::server_stopping(game)),
        Step::ServerStopped => done("serverStopped", || host::server_stopped(game)),
        Step::Tick {
            nanos,
            repeat,
            sleep_ms,
        } => done("tick", || ticks(*nanos, *repeat, *sleep_ms, game)),
        Step::PlayerJoined { uuid, name } => done("playerJoined", || {
            host::player_joined(uuid.as_deref(), name.as_deref(), game);
        }),
        Step::PlayerLeft { uuid, name } => done("playerLeft", || {
            host::player_left(uuid.as_deref(), name.as_deref(), game);
        }),
        Step::Chat {
            uuid,
            name,
            message,
        } => answer(
            "chat",
            host::chat(uuid.as_deref(), name.as_deref(), message.as_deref(), game),
        ),
        Step::Death {
            uuid,
            name,
            cause,
            dimension,
            x,
            y,
            z,
            yaw,
            pitch,
        } => done("death", || {
            let location = dimension.clone().map(|dimension| Location {
                dimension,
                x: *x,
                y: *y,
                z: *z,
                yaw: *yaw,
                pitch: *pitch,
            });
            host::death(
                uuid.as_deref(),
                name.as_deref(),
                cause.as_deref(),
                location,
                game,
            );
        }),
        Step::Respawn {
            uuid,
            name,
            dimension,
            end_conquered,
        } => done("respawn", || {
            host::respawn(
                uuid.as_deref(),
                name.as_deref(),
                dimension.as_deref(),
                *end_conquered,
                game,
            );
        }),
        Step::CommandNames => answer("commandNames", host::command_names()),
        Step::Command {
            name,
            sender_uuid,
            sender_name,
            level,
            arguments,
        } => answer(
            "command",
            host::command(
                name.as_deref(),
                sender_uuid.as_deref(),
                sender_name.as_deref(),
                *level,
                arguments.as_deref(),
                game,
            ),
        ),
        Step::Permission { uuid, node } => answer(
            "permission",
            host::permission(uuid.as_deref(), node.as_deref(), game),
        ),
        Step::Drain => drain(),
        Step::Stats => answer("stats", stats_json(golem_bridge::stats())),
        Step::Sleep { ms } => done("sleep", || thread::sleep(Duration::from_millis(*ms))),
        Step::Budgets | Step::Push { .. } | Step::Install | Step::Together { .. } => return None,
    })
}

/// Runs `steps` each on a thread of its own, the next one `stagger_ms` after the one before,
/// and answers their answers in the order of the steps, each with the milliseconds from the
/// start of the first to its own end.
fn together(stagger_ms: u64, steps: &[Step], game: &(dyn Game + Sync)) -> Value {
    const REFUSED: &str = "together runs native methods, stats and sleep alone";
    let start = Instant::now();
    let answers: Vec<Value> = thread::scope(|scope| {
        let threads: Vec<_> = steps
            .iter()
            .zip(0u32..)
            .map(|(step, index)| {
                let delay = Duration::from_millis(stagger_ms) * index;
                thread::Builder::new()
                    .name(format!("devtool-{index}"))
                    .spawn_scoped(scope, move || {
                        thread::sleep(delay);
                        let mut answer = native(step, game).unwrap_or_else(
                            || json!({"op": "refused", "answer": null, "error": REFUSED}),
                        );
                        answer["endMs"] = json!(start.elapsed().as_secs_f64() * 1000.0);
                        answer
                    })
            })
            .collect();
        threads
            .into_iter()
            .map(|thread| match thread {
                Ok(thread) => thread.join().unwrap_or(Value::Null),
                Err(error) => json!({"op": "refused", "answer": null, "error": error.to_string()}),
            })
            .collect()
    });
    json!({"op": "together", "answer": answers})
}

/// Answers the step `op` with the answer of its native, null for nothing.
fn answer(op: &str, value: impl Serialize) -> Value {
    json!({"op": op, "answer": value})
}

/// Runs a native that answers nothing, and answers the step `op` with null.
fn done(op: &str, native: impl FnOnce()) -> Value {
    native();
    json!({"op": op, "answer": null})
}

/// Feeds `repeat` ticks of `nanos` to the native tick, each after a sleep of `sleep_ms`.
fn ticks(nanos: i64, repeat: u32, sleep_ms: u64, game: &dyn Game) {
    for _ in 0..repeat {
        thread::sleep(Duration::from_millis(sleep_ms));
        host::tick(nanos, game);
    }
}

/// Drains the queue through the native drain, and answers the wire form and the decoded
/// action of every line.
fn drain() -> Value {
    let wire = host::drain();
    let decoded: Vec<Value> = wire
        .iter()
        .map(|line| {
            Action::decode(line)
                .as_ref()
                .map_or(Value::Null, action_json)
        })
        .collect();
    json!({"op": "drain", "answer": wire, "decoded": decoded})
}

/// Answers tick statistics as JSON.
fn stats_json(stats: TickStats) -> Value {
    json!({"ticks": stats.ticks, "mspt": stats.mspt, "tps": stats.tps})
}

/// Builds the plugin of a script.
fn build_plugin(spec: &PluginSpec, source: &Source) -> Box<dyn Plugin> {
    match spec.clone() {
        PluginSpec::Core => Box::new(CorePlugin::new(source.clone())),
        PluginSpec::Echo {
            name,
            commands,
            quiet,
        } => Box::new(Echo {
            name,
            commands,
            quiet,
        }),
        PluginSpec::Cancel { name, word } => Box::new(Cancel { name, word }),
        PluginSpec::Panic { name, on, commands } => Box::new(Panic { name, on, commands }),
        PluginSpec::Slow {
            name,
            on,
            sleep_ms,
            times,
        } => Box::new(Slow {
            name,
            on,
            sleep: Duration::from_millis(sleep_ms),
            left: times,
        }),
        PluginSpec::Reenter { name, on } => Box::new(Reenter { name, on }),
        PluginSpec::Grant {
            name,
            grants,
            denies,
            operators,
        } => Box::new(Grant {
            name,
            grants,
            denies,
            operators,
        }),
        PluginSpec::Ask { name, commands, on } => Box::new(Ask { name, commands, on }),
        PluginSpec::Mover { name, commands, to } => Box::new(Mover {
            name,
            commands,
            to: to.into(),
        }),
    }
}

/// Answers the specs of the names of `commands`, owned by `plugin`.
fn specs(plugin: &str, commands: &[String]) -> Vec<CommandSpec> {
    commands
        .iter()
        .map(|name| CommandSpec {
            name: name.clone(),
            summary: format!("A command of the test plugin {plugin}"),
        })
        .collect()
}

/// A test plugin that broadcasts every event but a tick and answers its commands.
struct Echo {
    name: String,
    commands: Vec<String>,
    quiet: bool,
}

impl Plugin for Echo {
    fn name(&self) -> &str {
        &self.name
    }

    fn commands(&self) -> Vec<CommandSpec> {
        specs(&self.name, &self.commands)
    }

    fn on_event(&mut self, event: &Event, context: &mut Context<'_>) -> Verdict {
        if self.quiet {
            return Verdict::Pass;
        }
        match event {
            Event::Tick { .. } => {}
            Event::Death { location, .. } => {
                context.broadcast(&format!("{} death at {location}", self.name));
            }
            Event::Respawn {
                end_conquered: true,
                ..
            } => context.broadcast(&format!("{} respawn from the End", self.name)),
            _ => context.broadcast(&format!("{} {}", self.name, event.name())),
        }
        Verdict::Pass
    }

    fn on_command(
        &mut self,
        name: &str,
        sender: &Sender,
        arguments: &str,
        context: &mut Context<'_>,
    ) -> Vec<String> {
        let who = match sender {
            Sender::Console => "the console".to_owned(),
            Sender::Player { player, .. } => {
                context.send_message(
                    &player.uuid,
                    &format!("{} heard {}", self.name, player.name),
                );
                player.name.clone()
            }
            Sender::Other { name, level } => format!("{name} at level {level}"),
        };
        context.run_command(&format!("/say {} ran {name}", self.name));
        vec![format!(
            "{} ran {name} for {who} with [{arguments}]",
            self.name
        )]
    }
}

/// A test plugin that cancels a chat message that holds its word, and answers cancel to every
/// other event, which the runtime ignores.
struct Cancel {
    name: String,
    word: String,
}

impl Plugin for Cancel {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_event(&mut self, event: &Event, _context: &mut Context<'_>) -> Verdict {
        match event {
            Event::Chat { message, .. } if !message.contains(&self.word) => Verdict::Pass,
            _ => Verdict::Cancel,
        }
    }
}

/// A test plugin that panics on the events of one name and in its commands.
struct Panic {
    name: String,
    on: String,
    commands: Vec<String>,
}

impl Plugin for Panic {
    fn name(&self) -> &str {
        &self.name
    }

    fn commands(&self) -> Vec<CommandSpec> {
        specs(&self.name, &self.commands)
    }

    fn on_event(&mut self, event: &Event, _context: &mut Context<'_>) -> Verdict {
        assert!(
            event.name() != self.on,
            "the test plugin {} panics on {}",
            self.name,
            self.on
        );
        Verdict::Pass
    }

    fn on_command(
        &mut self,
        name: &str,
        _sender: &Sender,
        _arguments: &str,
        _context: &mut Context<'_>,
    ) -> Vec<String> {
        panic!("the test plugin {} panics in the command {name}", self.name);
    }

    fn on_permission(
        &mut self,
        _uuid: &str,
        node: &str,
        _context: &mut Context<'_>,
    ) -> Option<bool> {
        assert!(
            self.on != "permission",
            "the test plugin {} panics on the permission {node}",
            self.name
        );
        None
    }
}

/// A test plugin that grants the nodes of `grants`, denies those of `denies` and leaves every
/// other node to the next plugin, or with `operators` answers it as the live server answers a
/// node that nothing names: true for an operator and false for every other player.
struct Grant {
    name: String,
    grants: Vec<String>,
    denies: Vec<String>,
    operators: Option<Vec<String>>,
}

impl Plugin for Grant {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_permission(
        &mut self,
        uuid: &str,
        node: &str,
        _context: &mut Context<'_>,
    ) -> Option<bool> {
        if self.grants.iter().any(|granted| granted == node) {
            Some(true)
        } else if self.denies.iter().any(|denied| denied == node) {
            Some(false)
        } else {
            let operators = self.operators.as_ref()?;
            Some(
                operators
                    .iter()
                    .any(|operator| operator.eq_ignore_ascii_case(uuid)),
            )
        }
    }
}

/// A test plugin that sleeps on the first events of one name.
struct Slow {
    name: String,
    on: String,
    sleep: Duration,
    left: u32,
}

impl Plugin for Slow {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_event(&mut self, event: &Event, _context: &mut Context<'_>) -> Verdict {
        if self.left > 0 && event.name() == self.on {
            self.left -= 1;
            thread::sleep(self.sleep);
        }
        Verdict::Pass
    }
}

/// A test plugin that calls the native `commandNames` from inside the handler, which must
/// answer the pass value at once instead of waiting for the lock it runs under.
struct Reenter {
    name: String,
    on: String,
}

impl Plugin for Reenter {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_event(&mut self, event: &Event, context: &mut Context<'_>) -> Verdict {
        if event.name() == self.on {
            let names = host::command_names();
            context.broadcast(&format!("{} got {} names", self.name, names.len()));
        }
        Verdict::Pass
    }
}

/// A test plugin that answers its commands with the helpers of the context for the actions on
/// players: `teleport <uuid>` to its place, `nowhere <uuid>` to a place that is no number,
/// `far <uuid>` to x 30000001, beyond the bounds of the world, `name <uuid> <text>` and
/// `clear <uuid>`.
struct Mover {
    name: String,
    commands: Vec<String>,
    to: Location,
}

impl Plugin for Mover {
    fn name(&self) -> &str {
        &self.name
    }

    fn commands(&self) -> Vec<CommandSpec> {
        specs(&self.name, &self.commands)
    }

    fn on_command(
        &mut self,
        _name: &str,
        _sender: &Sender,
        arguments: &str,
        context: &mut Context<'_>,
    ) -> Vec<String> {
        let mut words = arguments.splitn(3, ' ');
        let (verb, uuid, rest) = (
            words.next().unwrap_or_default(),
            words.next().unwrap_or_default(),
            words.next().unwrap_or_default(),
        );
        let answer = match verb {
            "teleport" => format!("queued {}", context.teleport(uuid, &self.to)),
            "nowhere" => {
                let nowhere = Location {
                    x: f64::NAN,
                    ..self.to.clone()
                };
                format!("queued {}", context.teleport(uuid, &nowhere))
            }
            "far" => {
                let far = Location {
                    x: 30_000_001.0,
                    ..self.to.clone()
                };
                format!("queued {}", context.teleport(uuid, &far))
            }
            "name" => {
                context.set_list_name(uuid, rest);
                "named".to_owned()
            }
            "clear" => {
                context.clear_list_name(uuid);
                "cleared".to_owned()
            }
            other => format!("{} knows no {other}", self.name),
        };
        vec![answer]
    }
}

/// A handler that panics in every method, in place of the runtime, so a script can show that
/// a native answers its pass value whatever the handler does.
struct Panicking;

impl Handler for Panicking {
    fn event(&mut self, event: &Event, _actions: &Actions, _game: &dyn Game) -> Verdict {
        panic!("the panicking handler panics on {}", event.name());
    }

    fn command_names(&self) -> Vec<String> {
        panic!("the panicking handler panics in commandNames");
    }

    fn command(
        &mut self,
        name: &str,
        _sender: &Sender,
        _arguments: &str,
        _actions: &Actions,
        _game: &dyn Game,
    ) -> Vec<String> {
        panic!("the panicking handler panics in the command {name}");
    }

    fn permission(
        &mut self,
        _uuid: &str,
        node: &str,
        _actions: &Actions,
        _game: &dyn Game,
    ) -> Option<bool> {
        panic!("the panicking handler panics on the permission {node}");
    }
}

/// An entry of the input of actions-encode: an action to encode, a wire form to decode, or a
/// teleport with numbers that are not finite, which JSON cannot carry.
#[derive(Deserialize)]
#[serde(untagged)]
enum EncodeEntry {
    Wire { wire: String },
    Infinite { infinite: InfiniteSpec },
    Action(ActionSpec),
}

/// A teleport of a player into a dimension whose x is infinity, y minus infinity, z 0, yaw
/// infinity and pitch NaN.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InfiniteSpec {
    uuid: String,
    dimension: String,
}

fn actions_encode(arguments: &[String]) -> i32 {
    finish(
        "actions-encode",
        read::<Vec<EncodeEntry>>("actions-encode", arguments).map(|entries| {
            let rows: Vec<Value> = entries.into_iter().map(encode_entry).collect();
            json!(rows)
        }),
    )
}

/// Answers the wire form, its fields and the decoded action of one entry.
fn encode_entry(entry: EncodeEntry) -> Value {
    let wire = match entry {
        EncodeEntry::Wire { wire } => wire,
        EncodeEntry::Infinite { infinite } => Action::Teleport {
            uuid: infinite.uuid,
            location: Location {
                dimension: infinite.dimension,
                x: f64::INFINITY,
                y: f64::NEG_INFINITY,
                z: 0.0,
                yaw: f32::INFINITY,
                pitch: f32::NAN,
            },
        }
        .encode(),
        EncodeEntry::Action(spec) => Action::from(spec).encode(),
    };
    let fields: Vec<&str> = wire.split(golem_bridge::SEPARATOR).collect();
    let decoded = Action::decode(&wire);
    json!({
        "wire": wire,
        "fields": fields,
        "decoded": decoded.as_ref().map_or(Value::Null, action_json),
        "reencoded": decoded.as_ref().map(Action::encode),
    })
}

/// A tick of the input of tick-stats.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TickSpec {
    nanos: u64,
    at_ms: f64,
}

fn tick_stats(arguments: &[String]) -> i32 {
    finish(
        "tick-stats",
        read::<Vec<TickSpec>>("tick-stats", arguments).and_then(window_stats),
    )
}

/// Feeds the ticks to a fresh window at their instants after one base instant, and answers the
/// statistics before the first and after each of them.
fn window_stats(ticks: Vec<TickSpec>) -> Result<Value, String> {
    let base = Instant::now();
    let mut window = TickWindow::new();
    let empty = stats_json(window.stats());
    let mut after = Vec::with_capacity(ticks.len());
    for tick in ticks {
        if !tick.at_ms.is_finite() || tick.at_ms < 0.0 {
            return Err(format!("the instant {} ms is no offset", tick.at_ms));
        }
        let at = base + Duration::from_secs_f64(tick.at_ms / 1000.0);
        window.record(tick.nanos, at);
        after.push(stats_json(window.stats()));
    }
    Ok(json!({
        "window": golem_bridge::WINDOW,
        "maxTps": golem_bridge::MAX_TPS,
        "empty": empty,
        "after": after,
    }))
}

fn natives(arguments: &[String]) -> i32 {
    let result = if arguments.is_empty() {
        let rows: Vec<Value> = golem_bridge::native_signatures()
            .into_iter()
            .map(|(name, signature)| json!({"name": name, "signature": signature}))
            .collect();
        Ok(json!({
            "hostClass": golem_bridge::HOST_CLASS,
            "version": host::VERSION,
            "natives": rows,
        }))
    } else {
        Err("natives takes no argument".to_owned())
    };
    finish("natives", result)
}
