// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The subcommands of golem-game and golem-model: contract call makes the calls of the contract
 * with the bridge mod against a scripted contract, and api coordinate calls the coordinate
 * functions over one, each step on the thread and under the frames that the script names.
 */

//! The primitives of golem-game and golem-model.
//!
//! `contract call <script.json>` and `api coordinate <script.json>` read the same script: the
//! entities of a scripted contract, those of a second one for the steps that open a frame
//! inside another, and the steps:
//!
//! ```text
//! {
//!   "entities": [{"uuid": "...", "id": 7, "level": 0, "player": true, "allowFlight": true,
//!                 "fault": null}],
//!   "inner": [],
//!   "steps": [{"op": "playerAllowFlight", "on": "server", "uuid": "...", "id": 7, "level": 0}]
//! }
//! ```
//!
//! An entity is no player and may not fly unless the script says so, its runtime id and its
//! level are 0 when left out, and one with a `fault` throws that text in every call that reads
//! it. Two entities of one contract that carry one UUID are refused.
//!
//! A step asks where its `on` names. No line of the bridge mod opens a frame yet, so every
//! frame here is one that golem-devtool opens:
//!
//! - `server`, the default: on the main thread, inside a frame over `entities`.
//! - `idle`: on the main thread, with no frame open.
//! - `thread`: on a thread of its own, which opens no frame, while a frame over `entities` is
//!   open on the main thread.
//! - `ending`: on a thread of its own while it ends, in the destructor of a thread local that
//!   the thread made before its frames, so that the frames of the thread are gone already.
//! - `nested`: in a frame over `entities`, then in a frame over `inner` that is opened inside
//!   the first, then in the first again once the second is closed. It answers
//!   `{"before": ..., "inside": ..., "after": ...}`.
//! - `stale`: with a token that was taken in a frame over `entities` which is closed since,
//!   first with no frame open and then while a frame over `inner` is open in its place. It
//!   answers `{"closed": ..., "replaced": ...}`.
//! - `unordered`: once a frame over `entities` was dropped while a frame over `inner`, opened
//!   inside it, was still kept. It asks with the token that the thread answers then and with
//!   the token that was taken in the second frame, and answers `{"fresh": ..., "kept": ...}`.
//!
//! The steps of `contract call` are the calls of `bridge/contract/` by their names,
//! `entityFind` with a `uuid` and `playerAllowFlight` with a `uuid`, an `id` and a `level`. A
//! step answers `{"value": ...}`, `{"miss": true}` for a call that missed, `{"fault": "..."}`
//! for one that failed and `{"token": false}` where no call is made: on a thread that holds no
//! server token, and with a token whose frame is closed.
//!
//! The steps of `api coordinate` are the functions of `golem_model::coordinate` by their
//! paths, `entity::find` with a `uuid`, `player::allow_flight` with a `uuid`, an `id` and a
//! `level`, and `server::bukkit_version`, which takes no token and so answers the same
//! wherever it is asked. A step answers `{"value": ...}`, or `{"error": "<code>", "message":
//! "..."}` with the code of the error.
//!
//! Both print the answers of the steps, `calls`, the count of the calls that the contract over
//! `entities` answered, and `innerCalls`, that of the contract over `inner`.

use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc,
    thread, //
};

use golem_game::{
    EntityRef,
    Frame,
    JniFault,
    Server,
    Uuid,
    contract::{
        entities,
        players, //
    },
    scripted::{
        Scripted,
        ScriptedEntity, //
    }, //
};
use golem_model::coordinate;
use serde::Deserialize;
use serde_json::{
    Value,
    json, //
};

use crate::{
    Subcommand,
    bridge::{
        finish,
        read, //
    },
};

pub const CONTRACT: Subcommand = (
    "contract",
    "call <script.json> the calls of the contract against a scripted contract",
    contract,
);

pub const API: Subcommand = (
    "api",
    "coordinate <script.json> the coordinate functions over a scripted contract",
    api,
);

/// The entities of two scripted contracts and the steps to run against them.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Script {
    #[serde(default)]
    entities: Vec<EntitySpec>,
    #[serde(default)]
    inner: Vec<EntitySpec>,
    steps: Vec<Step>,
}

/// An entity of a scripted contract.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EntitySpec {
    uuid: String,
    #[serde(default)]
    id: i32,
    #[serde(default)]
    level: u16,
    #[serde(default)]
    player: bool,
    #[serde(default)]
    allow_flight: bool,
    fault: Option<String>,
}

/// One step: a call or a function by its name, where it asks and the entity it asks about.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Step {
    op: String,
    #[serde(default)]
    on: On,
    uuid: Option<String>,
    #[serde(default)]
    id: i32,
    #[serde(default)]
    level: u16,
}

/// Where a step asks.
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum On {
    /// On the main thread, inside a frame.
    #[default]
    Server,
    /// On the main thread, with no frame open.
    Idle,
    /// On a thread of its own, which opens no frame, while a frame is open on the main thread.
    Thread,
    /// On a thread of its own while it ends, once the frames of that thread are gone.
    Ending,
    /// Before, inside and after a second frame that is opened inside the first.
    Nested,
    /// With a token whose frame is closed.
    Stale,
    /// Once an outer frame was dropped before the frame that was opened inside it.
    Unordered,
}

impl On {
    /// Answers the word of the script.
    fn name(self) -> &'static str {
        match self {
            Self::Server => "server",
            Self::Idle => "idle",
            Self::Thread => "thread",
            Self::Ending => "ending",
            Self::Nested => "nested",
            Self::Stale => "stale",
            Self::Unordered => "unordered",
        }
    }
}

/// What the steps of a script are: the calls of the contract or the coordinate functions.
#[derive(Clone, Copy)]
enum Layer {
    Contract,
    Coordinate,
}

/// What one step asks, with its arguments read.
#[derive(Clone, Copy)]
enum Asked {
    /// The call `entityFind`.
    EntityFind(Uuid),
    /// The call `playerAllowFlight`.
    PlayerAllowFlight(EntityRef),
    /// The function `entity::find`.
    Find(Uuid),
    /// The function `player::allow_flight`.
    AllowFlight(EntityRef),
    /// The function `server::bukkit_version`.
    BukkitVersion,
}

impl Asked {
    /// Reads what `step` asks by the names of `layer`, before anything asks for a token, so
    /// that a name that the layer does not hold is refused wherever the step runs.
    fn of(step: &Step, layer: Layer) -> Result<Self, String> {
        match (layer, step.op.as_str()) {
            (Layer::Contract, "entityFind") => Ok(Self::EntityFind(uuid_of(step.uuid.as_ref())?)),
            (Layer::Contract, "playerAllowFlight") => Ok(Self::PlayerAllowFlight(entity_of(step)?)),
            (Layer::Contract, other) => Err(format!("the contract has no call {other}")),
            (Layer::Coordinate, "entity::find") => Ok(Self::Find(uuid_of(step.uuid.as_ref())?)),
            (Layer::Coordinate, "player::allow_flight") => Ok(Self::AllowFlight(entity_of(step)?)),
            (Layer::Coordinate, "server::bukkit_version") => Ok(Self::BukkitVersion),
            (Layer::Coordinate, other) => {
                Err(format!("golem-model has no coordinate function {other}"))
            }
        }
    }

    /// Asks with the server token `token`, or with the one that the calling thread answers
    /// when `token` is `None`, and answers the answer of the step.
    fn ask(self, token: Option<&Server<'_>>) -> Value {
        match self {
            Self::EntityFind(uuid) => call(token, |server| {
                Ok(entities::entity_find(server, uuid)?.map(
                    |found| json!({"id": found.id, "level": found.level, "player": found.player}),
                ))
            }),
            Self::PlayerAllowFlight(player) => call(token, |server| {
                Ok(players::player_allow_flight(server, player)?.map(|may_fly| json!(may_fly)))
            }),
            Self::Find(uuid) => function(token, |server| {
                Ok(
                    coordinate::entity::find(server, uuid)?.map_or(Value::Null, |found| {
                        json!({
                            "id": found.entity.id(),
                            "level": found.entity.level(),
                            "uuid": found.entity.uuid().to_string(),
                            "kind": match found.kind {
                                coordinate::entity::Kind::Player => "Player",
                                coordinate::entity::Kind::EntityObject => "EntityObject",
                            },
                        })
                    }),
                )
            }),
            Self::AllowFlight(player) => function(token, |server| {
                Ok(json!(coordinate::player::allow_flight(server, player)?))
            }),
            Self::BukkitVersion => answered(coordinate::server::bukkit_version().map(Value::from)),
        }
    }
}

fn contract(arguments: &[String]) -> i32 {
    script("contract", "call", Layer::Contract, arguments)
}

fn api(arguments: &[String]) -> i32 {
    script("api", "coordinate", Layer::Coordinate, arguments)
}

/// Runs the script of the subcommand `family verb`, whose steps `layer` reads.
fn script(family: &str, verb: &str, layer: Layer, arguments: &[String]) -> i32 {
    let name = format!("{family} {verb}");
    match arguments {
        [given, rest @ ..] if given == verb => finish(
            &name,
            read::<Script>(&name, rest).and_then(|script| run(&script, layer)),
        ),
        [given, ..] => finish(family, Err(format!("{family} has no verb {given}"))),
        [] => finish(
            family,
            Err(format!("{family} takes the verb {verb} and one file")),
        ),
    }
}

/// Runs the steps of `script` as `layer` reads them, and answers the answer of each with the
/// counts of the calls that the two scripted contracts answered.
fn run(script: &Script, layer: Layer) -> Result<Value, String> {
    let outer = scripted_of(&script.entities)?;
    let inner = scripted_of(&script.inner)?;
    let mut answers = Vec::with_capacity(script.steps.len());
    for step in &script.steps {
        let asked = Asked::of(step, layer)?;
        let answered = match step.on {
            On::Server => {
                let _frame = Frame::open(outer.clone());
                asked.ask(None)
            }
            On::Idle => asked.ask(None),
            On::Thread => {
                let _frame = Frame::open(outer.clone());
                on_thread(asked)?
            }
            On::Ending => at_end(asked)?,
            On::Nested => {
                let _first = Frame::open(outer.clone());
                let before = asked.ask(None);
                let inside = {
                    let _second = Frame::open(inner.clone());
                    asked.ask(None)
                };
                json!({"before": before, "inside": inside, "after": asked.ask(None)})
            }
            On::Stale => {
                let frame = Frame::open(outer.clone());
                let token = golem_game::server().map_err(|refusal| refusal.to_string())?;
                drop(frame);
                let closed = asked.ask(Some(&token));
                let _next = Frame::open(inner.clone());
                json!({"closed": closed, "replaced": asked.ask(Some(&token))})
            }
            On::Unordered => {
                let first = Frame::open(outer.clone());
                let second = Frame::open(inner.clone());
                let token = golem_game::server().map_err(|refusal| refusal.to_string())?;
                drop(first);
                let answered = json!({"fresh": asked.ask(None), "kept": asked.ask(Some(&token))});
                drop(second);
                answered
            }
        };
        answers.push(json!({"op": step.op, "on": step.on.name(), "answer": answered}));
    }
    Ok(json!({"steps": answers, "calls": outer.calls(), "innerCalls": inner.calls()}))
}

/// Builds the scripted contract of the entities of a script.
fn scripted_of(specs: &[EntitySpec]) -> Result<Rc<Scripted>, String> {
    let entities = specs
        .iter()
        .map(|spec| {
            Ok(ScriptedEntity {
                entity: EntityRef::new(spec.id, spec.level, uuid_of(Some(&spec.uuid))?),
                player: spec.player,
                allow_flight: spec.allow_flight,
                fault: spec.fault.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Scripted::new(entities)
        .map(Rc::new)
        .map_err(|uuid| format!("the script names the UUID {uuid} twice"))
}

/// Reads the UUID of a step or of an entity.
fn uuid_of(text: Option<&String>) -> Result<Uuid, String> {
    let text = text.ok_or("the step names no uuid")?;
    Uuid::parse(text).ok_or_else(|| format!("{text} is no UUID"))
}

/// Answers the entity that a step names.
fn entity_of(step: &Step) -> Result<EntityRef, String> {
    Ok(EntityRef::new(
        step.id,
        step.level,
        uuid_of(step.uuid.as_ref())?,
    ))
}

/// Makes one call of the contract with `token`, or with the token of the calling thread, and
/// answers what the call answered. Without a token, and with one whose frame is closed, no
/// call is made.
fn call(
    token: Option<&Server<'_>>,
    make: impl FnOnce(&Server<'_>) -> Result<Option<Value>, JniFault>,
) -> Value {
    let answered = match token {
        Some(server) => make(server),
        None => match golem_game::server() {
            Ok(server) => make(&server),
            Err(_) => return json!({"token": false}),
        },
    };
    match answered {
        Ok(Some(value)) => json!({"value": value}),
        Ok(None) => json!({"miss": true}),
        Err(fault) if fault.is_refusal() => json!({"token": false}),
        Err(fault) => json!({"fault": fault.message()}),
    }
}

/// Calls one coordinate function with `token`, or with the token of the calling thread, and
/// answers what the function answered.
fn function(
    token: Option<&Server<'_>>,
    call: impl FnOnce(&Server<'_>) -> golem_model::Result<Value>,
) -> Value {
    answered(match token {
        Some(server) => call(server),
        None => golem_model::server().and_then(|server| call(&server)),
    })
}

/// Answers the answer of a coordinate function as a step prints it.
fn answered(result: golem_model::Result<Value>) -> Value {
    match result {
        Ok(value) => json!({"value": value}),
        Err(error) => json!({"error": error.code(), "message": error.to_string()}),
    }
}

/// Asks on a thread of its own, which opens no frame, and answers what it answered.
fn on_thread(asked: Asked) -> Result<Value, String> {
    thread::scope(|scope| {
        thread::Builder::new()
            .name("devtool-api".to_owned())
            .spawn_scoped(scope, move || asked.ask(None))
            .map_err(|error| format!("the thread of a step did not start: {error}"))?
            .join()
            .map_err(|_| "the thread of a step panicked".to_owned())
    })
}

/// What a thread asks while it ends, and where its answer goes.
struct AtEnd {
    asked: Asked,
    answer: mpsc::Sender<Value>,
}

impl Drop for AtEnd {
    fn drop(&mut self) {
        // Nobody reads the answer when the step has failed already.
        let _ = self.answer.send(self.asked.ask(None));
    }
}

thread_local! {
    /// What this thread asks while it ends.
    static AT_END: RefCell<Option<AtEnd>> = const { RefCell::new(None) };
}

/// Asks on a thread of its own while that thread ends. The thread makes the thread local that
/// asks first, then opens and closes a frame, so that the frames of the thread are dropped
/// before the question is asked where thread locals are dropped in the reverse order.
fn at_end(asked: Asked) -> Result<Value, String> {
    let (answer, answered) = mpsc::channel();
    thread::Builder::new()
        .name("devtool-api".to_owned())
        .spawn(move || {
            AT_END.with_borrow_mut(|slot| *slot = Some(AtEnd { asked, answer }));
            if let Ok(scripted) = Scripted::new(Vec::new()) {
                drop(Frame::open(Rc::new(scripted)));
            }
        })
        .map_err(|error| format!("the thread of a step did not start: {error}"))?
        .join()
        .map_err(|_| "the thread of a step panicked".to_owned())?;
    answered
        .recv()
        .map_err(|_| "the thread of a step ended without an answer".to_owned())
}
