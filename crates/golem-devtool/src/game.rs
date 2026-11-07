// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The game of the devtool: a game that answers from a script, in place of the one that the mod
 * connects, the test plugin ask that asks it, and the subcommand game-wire, which reads answers
 * in the wire form of the mod as the natives read them.
 */

//! The scripted game, the test plugin `ask` and the subcommand `game-wire`.
//!
//! The `game` of a script of the subcommand plugins names what the game answers:
//!
//! ```text
//! "game": {
//!   "players": [{"uuid": "...", "name": "Alex", "displayName": "Alex", "listName": null,
//!                "ping": 12, "gameMode": "survival", "level": 0, "idleMs": 1500,
//!                "location": {"dimension": "minecraft:overworld", "x": 0.5, "y": 64, "z": 0.5},
//!                "respawn": {"dimension": "...", "x": 1, "y": 64, "z": 2}, "mainHand": null}],
//!   "dimensions": [{"id": "minecraft:overworld", "spawn": {"x": 0, "y": 64, "z": 0, "angle": 0},
//!                   "minY": -64, "maxY": 320, "safe": {"x": 0.5, "y": 65, "z": 0.5}}]
//! }
//! ```
//!
//! A player without `respawn` has none and one without `mainHand` holds nothing, and
//! `"players": null` makes the game answer none for the players, as a game that cannot answer
//! does. A dimension
//! with `safe` answers that place to every search for a safe place, one with `"safe":
//! "unloaded"` answers that a chunk the search must read is not loaded, and one without answers
//! that there is none. A script without `game` asks a game that answers nothing.

use golem_bridge::{
    BlockLocation,
    Event,
    Game,
    GameMode,
    Heights,
    Item,
    Location,
    OnlinePlayer,
    SafePlace,
    Sender,
    Spawn,
    Verdict,
    wire, //
};
use golem_plugin::{
    CommandSpec,
    Context,
    Plugin, //
};
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

pub const GAME_WIRE: Subcommand = (
    "game-wire",
    "<answers.json> the answers of the game in the wire form, as the natives read them",
    game_wire,
);

/// A location as JSON, its fields beside the others of an action, a step or a player.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationSpec {
    dimension: String,
    x: f64,
    y: f64,
    z: f64,
    #[serde(default)]
    yaw: f32,
    #[serde(default)]
    pitch: f32,
}

impl From<LocationSpec> for Location {
    fn from(spec: LocationSpec) -> Self {
        Self {
            dimension: spec.dimension,
            x: spec.x,
            y: spec.y,
            z: spec.z,
            yaw: spec.yaw,
            pitch: spec.pitch,
        }
    }
}

/// Answers a location as JSON, in the form of [`LocationSpec`].
pub fn location_json(location: &Location) -> Value {
    json!({
        "dimension": location.dimension,
        "x": location.x,
        "y": location.y,
        "z": location.z,
        "yaw": location.yaw,
        "pitch": location.pitch,
    })
}

/// What the scripted game answers.
#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameSpec {
    #[serde(default = "no_players")]
    players: Option<Vec<PlayerSpec>>,
    #[serde(default)]
    dimensions: Vec<DimensionSpec>,
}

/// A player of the scripted game.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlayerSpec {
    uuid: String,
    name: String,
    display_name: Option<String>,
    list_name: Option<String>,
    #[serde(default)]
    ping: u32,
    #[serde(default = "survival")]
    game_mode: String,
    #[serde(default)]
    level: u8,
    #[serde(default)]
    idle_ms: u64,
    location: LocationSpec,
    respawn: Option<BlockSpec>,
    main_hand: Option<ItemSpec>,
}

/// The players of a script that names none: nobody is online.
#[allow(clippy::unnecessary_wraps)]
fn no_players() -> Option<Vec<PlayerSpec>> {
    Some(Vec::new())
}

/// The default game mode of a scripted player.
fn survival() -> String {
    "survival".to_owned()
}

/// A block of the scripted game.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BlockSpec {
    dimension: String,
    x: i32,
    y: i32,
    z: i32,
}

/// An item of the scripted game.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ItemSpec {
    id: String,
    count: u32,
}

/// A dimension of the scripted game.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DimensionSpec {
    id: String,
    spawn: SpawnSpec,
    min_y: i32,
    max_y: i32,
    safe: Option<SafeSpec>,
}

/// What every search for a safe place in a scripted dimension answers.
#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum SafeSpec {
    /// The place.
    Place(PlaceSpec),
    /// The word unloaded, for a chunk that is not loaded.
    Word(Unloaded),
}

/// The word unloaded of a [`SafeSpec`].
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Unloaded {
    Unloaded,
}

/// The spawn of a scripted dimension.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SpawnSpec {
    x: i32,
    y: i32,
    z: i32,
    #[serde(default)]
    angle: f32,
}

/// A place without a rotation, which the search for a safe place answers.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlaceSpec {
    x: f64,
    y: f64,
    z: f64,
}

/// The game that answers from a script.
pub struct ScriptedGame {
    spec: GameSpec,
}

impl ScriptedGame {
    /// The game that answers what `spec` names.
    pub fn new(spec: GameSpec) -> Self {
        Self { spec }
    }

    fn of(&self, uuid: &str) -> Option<&PlayerSpec> {
        self.spec
            .players
            .as_ref()?
            .iter()
            .find(|player| player.uuid == uuid)
    }

    fn dimension(&self, id: &str) -> Option<&DimensionSpec> {
        self.spec
            .dimensions
            .iter()
            .find(|dimension| dimension.id == id)
    }
}

impl Game for ScriptedGame {
    fn players(&self) -> Option<Vec<OnlinePlayer>> {
        self.spec
            .players
            .as_ref()?
            .iter()
            .map(|player| {
                Some(OnlinePlayer {
                    uuid: player.uuid.clone(),
                    name: player.name.clone(),
                    display_name: player
                        .display_name
                        .clone()
                        .unwrap_or_else(|| player.name.clone()),
                    list_name: player.list_name.clone(),
                    ping_ms: player.ping,
                    game_mode: GameMode::from_name(&player.game_mode)?,
                    level: player.level,
                    idle_ms: player.idle_ms,
                })
            })
            .collect()
    }

    fn location(&self, uuid: &str) -> Option<Location> {
        Some(self.of(uuid)?.location.clone().into())
    }

    fn respawn(&self, uuid: &str) -> Option<Option<BlockLocation>> {
        Some(self.of(uuid)?.respawn.clone().map(|block| BlockLocation {
            dimension: block.dimension,
            x: block.x,
            y: block.y,
            z: block.z,
        }))
    }

    fn spawn(&self, dimension: &str) -> Option<Spawn> {
        let spawn = &self.dimension(dimension)?.spawn;
        Some(Spawn {
            block: BlockLocation {
                dimension: dimension.to_owned(),
                x: spawn.x,
                y: spawn.y,
                z: spawn.z,
            },
            angle: spawn.angle,
        })
    }

    fn main_hand(&self, uuid: &str) -> Option<Option<Item>> {
        Some(self.of(uuid)?.main_hand.clone().map(|item| Item {
            id: item.id,
            count: item.count,
        }))
    }

    fn heights(&self, dimension: &str) -> Option<Heights> {
        let dimension = self.dimension(dimension)?;
        Some(Heights {
            min: dimension.min_y,
            max: dimension.max_y,
        })
    }

    fn safe_location(&self, target: &Location, _radius: u32) -> Option<SafePlace> {
        let dimension = self.dimension(&target.dimension)?;
        Some(match &dimension.safe {
            None => SafePlace::Nowhere,
            Some(SafeSpec::Word(Unloaded::Unloaded)) => SafePlace::Unloaded,
            Some(SafeSpec::Place(place)) => SafePlace::Found(Location {
                x: place.x,
                y: place.y,
                z: place.z,
                ..target.clone()
            }),
        })
    }
}

/// Answers a player as JSON, in the form of the players of [`GameSpec`].
fn player_json(player: &OnlinePlayer) -> Value {
    json!({
        "uuid": player.uuid,
        "name": player.name,
        "displayName": player.display_name,
        "listName": player.list_name,
        "ping": player.ping_ms,
        "gameMode": player.game_mode.name(),
        "level": player.level,
        "idleMs": player.idle_ms,
    })
}

/// Answers a block as JSON.
fn block_json(block: &BlockLocation) -> Value {
    json!({"dimension": block.dimension, "x": block.x, "y": block.y, "z": block.z})
}

/// Answers a spawn as JSON.
fn spawn_json(spawn: &Spawn) -> Value {
    let mut value = block_json(&spawn.block);
    value["angle"] = json!(spawn.angle);
    value
}

/// Answers an item as JSON.
fn item_json(item: &Item) -> Value {
    json!({"id": item.id, "count": item.count})
}

/// Answers heights as JSON.
fn heights_json(heights: Heights) -> Value {
    json!({"min": heights.min, "max": heights.max})
}

/// Answers a safe place as JSON: null when the game cannot answer or the answer does not read,
/// `{"none": true}` when no place is safe, `{"unloaded": true}` when a chunk is not loaded, and
/// the place otherwise.
fn safe_json(answer: Option<SafePlace>) -> Value {
    match answer {
        None => Value::Null,
        Some(SafePlace::Nowhere) => json!({"none": true}),
        Some(SafePlace::Unloaded) => json!({"unloaded": true}),
        Some(SafePlace::Found(place)) => location_json(&place),
    }
}

/// Answers an answer that may be empty as JSON: null when the game cannot answer or the answer
/// does not read, the object `{"none": true}` when it answers none, and the value otherwise.
// The type is the one of the answers of the trait Game that may be empty.
#[allow(clippy::option_option)]
fn maybe<T>(answer: Option<Option<T>>, json: impl Fn(&T) -> Value) -> Value {
    match answer {
        None => Value::Null,
        Some(None) => json!({"none": true}),
        Some(Some(value)) => json(&value),
    }
}

/// A test plugin that asks the game on its commands and on the events named `on`.
///
/// A command answers one line, the JSON of the answer: `players`, `player <who>`,
/// `location <uuid>`, `respawn <uuid>`, `spawn <dimension>`, `hand <uuid>`,
/// `heights <dimension>` and `safe <dimension> <x> <y> <z> <radius>`. On an event named `on` it
/// asks for the players and broadcasts how many it got, or that the game did not answer.
pub struct Ask {
    pub name: String,
    pub commands: Vec<String>,
    pub on: Option<String>,
}

impl Ask {
    /// Answers the JSON of one question to the game.
    fn ask(game: &dyn Game, arguments: &str) -> Value {
        let words: Vec<&str> = arguments.split_whitespace().collect();
        match words.as_slice() {
            ["players"] => game.players().map_or(Value::Null, |players| {
                json!(players.iter().map(player_json).collect::<Vec<_>>())
            }),
            ["player", who] => maybe(game.player(who), player_json),
            ["location", uuid] => game
                .location(uuid)
                .as_ref()
                .map_or(Value::Null, location_json),
            ["respawn", uuid] => maybe(game.respawn(uuid), block_json),
            ["spawn", dimension] => game
                .spawn(dimension)
                .as_ref()
                .map_or(Value::Null, spawn_json),
            ["hand", uuid] => maybe(game.main_hand(uuid), item_json),
            ["heights", dimension] => game.heights(dimension).map_or(Value::Null, heights_json),
            ["safe", dimension, x, y, z, radius] => {
                let (Ok(x), Ok(y), Ok(z), Ok(radius)) =
                    (x.parse(), y.parse(), z.parse(), radius.parse())
                else {
                    return json!({"error": "safe takes a dimension, x, y, z and a radius"});
                };
                let target = Location {
                    dimension: (*dimension).to_owned(),
                    x,
                    y,
                    z,
                    yaw: 0.0,
                    pitch: 0.0,
                };
                safe_json(game.safe_location(&target, radius))
            }
            _ => json!({"error": format!("no question {arguments}")}),
        }
    }
}

impl Plugin for Ask {
    fn name(&self) -> &str {
        &self.name
    }

    fn commands(&self) -> Vec<CommandSpec> {
        self.commands
            .iter()
            .map(|name| CommandSpec {
                name: name.clone(),
                summary: format!("A command of the test plugin {}", self.name),
            })
            .collect()
    }

    fn on_event(&mut self, event: &Event, context: &mut Context<'_>) -> Verdict {
        if self.on.as_deref() == Some(event.name()) {
            let line = match context.game().players() {
                Some(players) => format!("{} saw {} players", self.name, players.len()),
                None => format!("{} got no answer", self.name),
            };
            context.broadcast(&line);
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
        vec![Self::ask(context.game(), arguments).to_string()]
    }
}

/// An answer of the mod to read: the question, the answer in the wire form, and for a spawn
/// the dimension and for a safe place the target.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireEntry {
    question: String,
    answer: String,
    dimension: Option<String>,
    target: Option<LocationSpec>,
}

fn game_wire(arguments: &[String]) -> i32 {
    finish(
        "game-wire",
        read::<Vec<WireEntry>>("game-wire", arguments).and_then(|entries| {
            let rows: Result<Vec<Value>, String> = entries.into_iter().map(read_wire).collect();
            rows.map(|rows| json!(rows))
        }),
    )
}

/// Reads one answer as the natives read it, and answers its JSON, null for an answer that does
/// not read.
fn read_wire(entry: WireEntry) -> Result<Value, String> {
    let answer = entry.answer.as_str();
    let decoded = match entry.question.as_str() {
        "players" => wire::player(answer)
            .as_ref()
            .map_or(Value::Null, player_json),
        "location" => wire::location(answer)
            .as_ref()
            .map_or(Value::Null, location_json),
        "respawn" => maybe(wire::respawn(answer), block_json),
        "spawn" => {
            let dimension = entry.dimension.as_deref().unwrap_or("minecraft:overworld");
            wire::spawn(dimension, answer)
                .as_ref()
                .map_or(Value::Null, spawn_json)
        }
        "mainHand" => maybe(wire::main_hand(answer), item_json),
        "heights" => wire::heights(answer).map_or(Value::Null, heights_json),
        "safeLocation" => {
            let target: Location = entry
                .target
                .ok_or("a safeLocation entry needs a target")?
                .into();
            safe_json(wire::safe_location(&target, answer))
        }
        other => return Err(format!("no question {other}")),
    };
    Ok(json!({"question": entry.question, "answer": entry.answer, "decoded": decoded}))
}
