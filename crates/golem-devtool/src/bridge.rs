// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The subcommands of golem-bridge and golem-plugin: plugins feeds a script of native calls to
 * the plugin runtime with the core plugin and test plugins, as the mod would, with a store of
 * the plugins when the script names one, and prints what the natives answered and queued.
 * actions-encode prints the wire form of actions, tick-stats the statistics of a window of ticks
 * at given instants, and natives the table of native methods that the host registers.
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
//!   "permissions": "[groups.default]...",   the TOML of the permissions of the runtime
//!   "store": "golem-store.db",       the file of a store of the plugins, by default none
//!   "steps": [{"op": "chat", "uuid": "...", "name": "Alex", "message": "hello"}]
//! }
//! ```
//!
//! With `store` the devtool opens the store of that file, hands it to the runtime as `golem run`
//! does, and closes it after the last step. The answer names the outcome of that close as
//! `storeClosed`, which is null for a script without a store. With `permissions` alone the
//! runtime checks with those permissions. With both, the devtool seeds a store that holds no
//! group with the permissions, as `golem run` seeds one, and a store that holds a group gives the
//! runtime the resolver over its set. The plugin `permissions` is the plugin `golem-permissions`
//! of the runtime with that resolver. The steps `stored`, which answers the TOML of the set of the
//! store, `generation`, which answers the generation the resolver holds and that of the store,
//! `refresh`, which wakes the reload of the resolver with `Resolver::refresh` and waits up to a
//! second for it, and `behind`, which puts a group into the store behind the resolver, as another
//! writer of the store would, reach them.
//!
//! Every native step hands the natives the scripted game, or a game that answers nothing when
//! the script names none, and the natives hand it to the plugins behind the guard of the server
//! thread, as they do the game of the mod.
//!
//! A step is a native method by its Java name with its Java parameters, where a missing parameter
//! is null: `version`, `serverStarting`, `serverStarted`, `serverStopping`, `serverStopped`, `tick`
//! (with `nanos`, and `repeat` and `sleepMs` of the devtool), `playerJoined`, `playerLeft`, `chat`,
//! `death`, `respawn` (with `endConquered`, false when left out), `teleported`, `teleportFailed`,
//! `commandNames`, `command` (with `level`, 4 when left out), `permission` and `drain`. A `death`
//! step takes the `dimension`, `x`, `y`, `z`, `yaw` and `pitch` of the place, each 0 when left
//! out, and no dimension stands for null. The `from` and `to` of `teleported` are a location,
//! which the step writes in the wire form of the answer of `location`, or a string that the step
//! hands the native as it is. The devtool adds `stats`, `budgets`, `push` (an action straight
//! into the queue), `install` (a fresh handler of the same script), `sleep` and `together`, which
//! runs its `steps` each on a thread of its own, the next one `staggerMs` after the one before, as
//! Forge calls `chat` on a worker thread beside the server thread. A step of `together` is a
//! native method, `stats` or `sleep`.
//!
//! The test plugins are these. `echo` broadcasts the name of every event but a tick, the
//! location of a death, and the id, the UUID and the places of a teleport or the reason it failed,
//! unless it is `quiet`, and answers its `commands` with a line, a message
//! to a player sender and a console command. `cancel` cancels a chat message that holds its
//! `word` and answers cancel to every other event, which the runtime must ignore. `panic` panics
//! on the events named `on`, on a permission when `on` is `permission`, and in its `commands`.
//! `slow` sleeps `sleepMs` on the first `times` events named `on`. `reenter` calls the native
//! `commandNames` from inside its handler on the events named `on` and broadcasts how many names
//! it got. `grant` grants the nodes of its `grants` and denies those of its `denies`, and with
//! `operators` answers every other node as the live server answers a node that nothing names,
//! true for a UUID of `operators` and false for every other player. `checker` answers each node
//! of the arguments of its `commands` with the check of the context for the sender. `mover`
//! answers its `commands` with the helpers of the context that teleport a player to its place
//! `to`, to a place that is no number and to one beyond the bounds of the world, and name them in
//! the player list. `ask` asks the game, as the module game describes. `keeper` reaches the store
//! through the context of its call: its `commands` set and answer a setting, answer the counts of
//! the writer, hold the writer still, name a setting that the plugin sets behind a held writer when
//! it sees `ServerStopped`, answer whether a context that no runtime made holds a store, and set a
//! setting through the store that the resolver of the permissions hands out.

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
    PermissionsPlugin,
    Plugin,
    Resolver,
    Runtime,
    Source, //
};
use golem_store::{
    PermissionGroup,
    PermissionSet,
    Store, //
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
    permissions: Option<String>,
    store: Option<String>,
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
    /// A plugin that reads and changes the store through the context on its commands.
    Keeper { name: String, commands: Vec<String> },
    /// A plugin that answers its commands with the check of each node of the arguments for the
    /// sender, through the context.
    Checker { name: String, commands: Vec<String> },
    /// The plugin of the permissions of the runtime, the port of `LuckPerms`.
    Permissions,
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
    Teleported {
        id: i64,
        uuid: Option<String>,
        from: Option<PlaceArgument>,
        to: Option<PlaceArgument>,
    },
    TeleportFailed {
        id: i64,
        uuid: Option<String>,
        reason: Option<String>,
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
    Stored,
    Generation,
    Refresh,
    Behind {
        group: String,
        #[serde(default)]
        weight: i32,
        #[serde(default)]
        inherits: Vec<String>,
        #[serde(default)]
        nodes: Vec<String>,
    },
}

/// A place that a step hands a native in the wire form of the answer of `location`: a string
/// as it is, which may not read, or a location, which the step writes in that form.
#[derive(Deserialize)]
#[serde(untagged)]
enum PlaceArgument {
    Wire(String),
    Place(LocationSpec),
}

impl PlaceArgument {
    /// Answers the place in the wire form of `location`.
    fn wire(&self) -> String {
        match self {
            Self::Wire(wire) => wire.clone(),
            Self::Place(place) => {
                let place = Location::from(place.clone());
                [
                    place.dimension,
                    place.x.to_string(),
                    place.y.to_string(),
                    place.z.to_string(),
                    place.yaw.to_string(),
                    place.pitch.to_string(),
                ]
                .join(&golem_bridge::SEPARATOR.to_string())
            }
        }
    }
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
        #[serde(default)]
        id: u64,
        uuid: String,
        #[serde(flatten)]
        location: LocationSpec,
        #[serde(default)]
        safe: Option<u32>,
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
            ActionSpec::Teleport {
                id,
                uuid,
                location,
                safe,
            } => Self::Teleport {
                id,
                uuid,
                location: location.into(),
                safe,
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
        Action::Teleport {
            id,
            uuid,
            location,
            safe,
        } => {
            let mut value = location_json(location);
            value["kind"] = json!("teleport");
            value["id"] = json!(id);
            value["uuid"] = json!(uuid);
            value["safe"] = json!(safe);
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
    let (store, resolver) = permissions_of(script.store.as_deref(), script.permissions.as_deref())?;
    let mut run = Run {
        kind: script.handler,
        budget,
        source,
        plugins: script.plugins,
        budgets: None,
        actions: host::actions(),
        game: script.game.map(ScriptedGame::new),
        store,
        resolver,
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
    // The store closes after the last step, as golem run closes it once the JVM ended.
    let closed = run
        .store
        .take()
        .map(|store| store.close().map_err(|error| error.to_string()));
    Ok(json!({
        "handler": run.kind_name(),
        "budgetMs": budget.as_secs_f64() * 1000.0,
        "steps": answers,
        "storeClosed": closed,
    }))
}

/// The store and the resolver of a script, from the file of its store and its TOML of the
/// permissions. A store that holds no group is seeded with the permissions, as `golem run` seeds
/// one, and the resolver stands over the set of a store that holds a group, or over the
/// permissions alone in a script without a store.
fn permissions_of(
    store: Option<&str>,
    permissions: Option<&str>,
) -> Result<(Option<Store>, Option<Resolver>), String> {
    let Some(path) = store else {
        let Some(text) = permissions else {
            return Ok((None, None));
        };
        let permissions =
            golem_permissions::Permissions::from_toml(text).map_err(|error| error.to_string())?;
        return Ok((None, Some(Resolver::from_permissions(permissions))));
    };
    let store = Store::open(path).map_err(|error| error.to_string())?;
    if let Some(text) = permissions
        && store.permissions().groups.is_empty()
    {
        golem_permissions::Permissions::from_toml(text).map_err(|error| error.to_string())?;
        let set = PermissionSet::from_toml(text)?;
        store
            .replace_permissions(set)
            .map_err(|error| error.to_string())?;
    }
    if store.permissions().groups.is_empty() {
        return Ok((Some(store), None));
    }
    let resolver = Resolver::from_store(&store)?;
    Ok((Some(store), Some(resolver)))
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
    /// The store of the script, which the runtime hands its plugins.
    store: Option<Store>,
    /// The resolver of the permissions of the script.
    resolver: Option<Resolver>,
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
                if let Some(store) = &self.store {
                    runtime.set_store(store.clone());
                }
                if let Some(resolver) = &self.resolver {
                    runtime.set_permissions(resolver.clone());
                }
                for spec in &self.plugins {
                    match (spec, &self.resolver) {
                        (PluginSpec::Permissions, Some(resolver)) => {
                            runtime.add(Box::new(PermissionsPlugin::new(resolver.clone())));
                        }
                        (PluginSpec::Permissions, None) => runtime.add(Box::new(
                            PermissionsPlugin::new(Resolver::from_permissions(
                                golem_permissions::Permissions::default(),
                            )),
                        )),
                        _ => runtime.add(build_plugin(spec, &self.source)),
                    }
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
            Step::Stored => answer(
                "stored",
                self.store
                    .as_ref()
                    .map(|store| store.permissions().to_toml()),
            ),
            Step::Generation => answer(
                "generation",
                json!({
                    "resolver": self.resolver.as_ref().map(Resolver::generation),
                    "store": self.store.as_ref().map(Store::permissions_generation),
                }),
            ),
            Step::Refresh => {
                let Some(resolver) = &self.resolver else {
                    return answer("refresh", Value::Null);
                };
                resolver.refresh();
                let target = self.store.as_ref().map(Store::permissions_generation);
                let deadline = Instant::now() + Duration::from_secs(1);
                while target.is_some_and(|target| resolver.generation() != target)
                    && Instant::now() < deadline
                {
                    thread::sleep(Duration::from_millis(5));
                }
                answer("refresh", json!(resolver.generation()))
            }
            Step::Behind {
                group,
                weight,
                inherits,
                nodes,
            } => {
                let Some(store) = &self.store else {
                    return answer("behind", Value::Null);
                };
                let changed = store.put_permission_group(PermissionGroup {
                    name: group,
                    weight,
                    parents: inherits,
                    nodes,
                });
                answer(
                    "behind",
                    changed
                        .map_or_else(|error| json!(error.to_string()), |replaced| json!(replaced)),
                )
            }
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
        Step::Death { .. } | Step::Teleported { .. } | Step::TeleportFailed { .. } => {
            return report(step, game);
        }
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
        Step::Budgets
        | Step::Push { .. }
        | Step::Install
        | Step::Together { .. }
        | Step::Stored
        | Step::Generation
        | Step::Refresh
        | Step::Behind { .. } => return None,
    })
}

/// The place of a death step: its dimension, its coordinates and its rotation.
fn place(dimension: String, [x, y, z]: [f64; 3], [yaw, pitch]: [f32; 2]) -> Location {
    Location {
        dimension,
        x,
        y,
        z,
        yaw,
        pitch,
    }
}

/// Runs a step that reports what became of a player at a place: a death, or the end of a
/// teleport, `teleported` or `teleportFailed`.
fn report(step: &Step, game: &(dyn Game + Sync)) -> Option<Value> {
    Some(match step {
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
            let location = dimension
                .clone()
                .map(|dimension| place(dimension, [*x, *y, *z], [*yaw, *pitch]));
            host::death(
                uuid.as_deref(),
                name.as_deref(),
                cause.as_deref(),
                location,
                game,
            );
        }),
        Step::Teleported { id, uuid, from, to } => done("teleported", || {
            let from = from.as_ref().map(PlaceArgument::wire);
            let to = to.as_ref().map(PlaceArgument::wire);
            host::teleported(*id, uuid.as_deref(), from.as_deref(), to.as_deref(), game);
        }),
        Step::TeleportFailed { id, uuid, reason } => done("teleportFailed", || {
            host::teleport_failed(*id, uuid.as_deref(), reason.as_deref(), game);
        }),
        _ => return None,
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
        PluginSpec::Keeper { name, commands } => Box::new(Keeper {
            name,
            commands,
            stopped: None,
        }),
        PluginSpec::Checker { name, commands } => Box::new(Checker { name, commands }),
        // Run::install adds the plugin of the permissions with the resolver of the script.
        PluginSpec::Permissions => Box::new(PermissionsPlugin::new(Resolver::from_permissions(
            golem_permissions::Permissions::default(),
        ))),
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
            Event::Teleported { id, uuid, from, to } => context.broadcast(&format!(
                "{} teleported {id} {uuid} from {from} to {to}",
                self.name
            )),
            Event::TeleportFailed { id, uuid, reason } => context.broadcast(&format!(
                "{} teleportFailed {id} {uuid} {}",
                self.name,
                reason.name()
            )),
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
/// players: `teleport <uuid>` to its place, `safe <uuid> <radius>` to the nearest safe place
/// within the radius of it, which answers the id, `nowhere <uuid>` to a place that is no number,
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
            "safe" => match rest.parse() {
                Ok(radius) => match context.queue_teleport(uuid, &self.to, Some(radius)) {
                    Some(id) => format!("queued the teleport {id}"),
                    None => "refused".to_owned(),
                },
                Err(_) => "safe takes a radius".to_owned(),
            },
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

/// A test plugin that reaches the store through the context of its call. Its commands take
/// `set <key> <value>`, which sets a setting, `get <key>`, which answers it, `stats`, which
/// answers the counts of the writer, `pause <ms>`, which holds the writer still through the hook
/// of the feature testing of golem-store, `stopped <ms> <key>`, after which the plugin holds the
/// writer still for that long and sets the setting once it sees `ServerStopped`, `outside`,
/// which answers whether a context that no runtime made holds a store, `room <n>`, which
/// answers whether the store takes that many changes now, `fill <n>`, which sets the
/// setting `fill` that many times and answers how many of them the store took and why it took
/// no more, and `resolver <key> <value>`, which sets a setting through the store that the
/// resolver of the permissions hands out.
struct Keeper {
    name: String,
    commands: Vec<String>,
    /// What the plugin does once it sees `ServerStopped`: how long it holds the writer still, and
    /// the setting that it then sets.
    stopped: Option<(Duration, String)>,
}

impl Plugin for Keeper {
    fn name(&self) -> &str {
        &self.name
    }

    fn commands(&self) -> Vec<CommandSpec> {
        specs(&self.name, &self.commands)
    }

    fn on_event(&mut self, event: &Event, context: &mut Context<'_>) -> Verdict {
        if let (Event::ServerStopped, Some((held, key)), Some(store)) =
            (event, &self.stopped, context.store())
        {
            // A change on the last event of the server thread, behind a writer that stands still,
            // which only a flush behind the plugins brings into the file.
            let _ = store.testing_pause_writer(*held);
            let _ = store.set_setting(key, "stopped");
        }
        Verdict::Pass
    }

    fn on_command(
        &mut self,
        _name: &str,
        _sender: &Sender,
        arguments: &str,
        context: &mut Context<'_>,
    ) -> Vec<String> {
        let mut words = arguments.splitn(3, ' ');
        let (verb, key, value) = (
            words.next().unwrap_or_default(),
            words.next().unwrap_or_default(),
            words.next().unwrap_or_default(),
        );
        if verb == "outside" {
            let outside = Context::new(context.actions());
            return vec![format!("store {}", outside.store().is_some())];
        }
        if verb == "stopped" {
            let Ok(ms) = key.parse::<u64>() else {
                return vec![format!("{key} is no number of milliseconds")];
            };
            self.stopped = Some((Duration::from_millis(ms), value.to_owned()));
            return vec![format!("armed {value} behind {ms} ms")];
        }
        let Some(store) = context.store() else {
            return vec!["no store".to_owned()];
        };
        let answer = match verb {
            "set" => match store.set_setting(key, value) {
                Ok(()) => format!("set {key}"),
                Err(error) => format!("refused {key}: {error}"),
            },
            "get" => match store.setting(key) {
                Some(held) => format!("{key} is {held}"),
                None => format!("{key} is not set"),
            },
            "stats" => {
                let stats = store.stats();
                let (queued, written, pending) = (stats.queued, stats.written, stats.pending);
                format!("queued {queued}, written {written}, pending {pending}")
            }
            "room" => match key.parse::<usize>() {
                Ok(changes) => format!("room for {changes}: {}", store.room(changes)),
                Err(_) => format!("{key} is no number of changes"),
            },
            "fill" => match key.parse::<u64>() {
                Ok(changes) => fill(store, changes),
                Err(_) => format!("{key} is no number of changes"),
            },
            "resolver" => match context.permissions().and_then(Resolver::store) {
                Some(handle) => match handle.set_setting(key, value) {
                    Ok(()) => format!("set {key}"),
                    Err(error) => format!("refused {key}: {error}"),
                },
                None => "the resolver hands out no store".to_owned(),
            },
            "pause" => match key.parse::<u64>() {
                Ok(ms) => match store.testing_pause_writer(Duration::from_millis(ms)) {
                    Ok(()) => format!("paused {ms} ms"),
                    Err(error) => format!("not paused: {error}"),
                },
                Err(_) => format!("{key} is no number of milliseconds"),
            },
            other => format!("{} knows no {other}", self.name),
        };
        vec![answer]
    }
}

/// Sets the setting `fill` of a store `changes` times, as a command that changes the store in a
/// loop would, and answers how many changes the store took and the first refusal.
fn fill(store: &Store, changes: u64) -> String {
    let mut took = 0_u64;
    let mut refusal = None;
    for change in 0..changes {
        match store.set_setting("fill", &change.to_string()) {
            Ok(()) => took += 1,
            Err(error) => {
                refusal.get_or_insert(error);
            }
        }
    }
    match refusal {
        Some(error) => format!("took {took} of {changes}: {error}"),
        None => format!("took {took} of {changes}"),
    }
}

/// A test plugin that answers each node of the arguments of its commands with the check of the
/// context for the sender, as `<node>: granted <bool>, set <bool>, has <bool>, <reason>`, where
/// `has` is the answer of the check without the reason.
struct Checker {
    name: String,
    commands: Vec<String>,
}

impl Plugin for Checker {
    fn name(&self) -> &str {
        &self.name
    }

    fn commands(&self) -> Vec<CommandSpec> {
        specs(&self.name, &self.commands)
    }

    fn on_command(
        &mut self,
        _name: &str,
        sender: &Sender,
        arguments: &str,
        context: &mut Context<'_>,
    ) -> Vec<String> {
        arguments
            .split_whitespace()
            .map(|node| {
                let answer = context.check_permission(sender, node);
                format!(
                    "{node}: granted {}, set {}, has {}, {}",
                    answer.granted,
                    answer.set,
                    context.has_permission(sender, node),
                    answer.reason
                )
            })
            .collect()
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
            id: 1,
            uuid: infinite.uuid,
            location: Location {
                dimension: infinite.dimension,
                x: f64::INFINITY,
                y: f64::NEG_INFINITY,
                z: 0.0,
                yaw: f32::INFINITY,
                pitch: f32::NAN,
            },
            safe: None,
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
