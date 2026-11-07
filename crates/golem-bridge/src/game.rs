// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The game of golem-bridge: what a handler may ask the game while a native of the server thread
 * runs, the places of the world it answers with, and the wire form of the answers of the mod.
 *
 * The trait Game is the question. The natives answer it through the JNI with the object that
 * the mod connected, NoGame answers nothing, and golem-devtool answers from a script. The host
 * hands the handler every game behind a guard that answers nothing off the server thread, so
 * Rust never reads the game from another thread.
 */

use std::fmt;

/// A place in the world with a rotation, as the game keeps it for a player.
///
/// The coordinates are those of the feet. The rotation is in degrees: `yaw` around the vertical
/// axis, where 0 faces south and 90 west, and `pitch` up and down, where -90 looks straight up.
#[derive(Clone, Debug, PartialEq)]
pub struct Location {
    /// The id of the dimension, such as minecraft:overworld.
    pub dimension: String,
    /// The x coordinate.
    pub x: f64,
    /// The y coordinate, the height.
    pub y: f64,
    /// The z coordinate.
    pub z: f64,
    /// The rotation around the vertical axis, in degrees.
    pub yaw: f32,
    /// The rotation up and down, in degrees.
    pub pitch: f32,
}

impl Location {
    /// Answers whether every coordinate and angle is a finite number, which the game needs.
    pub fn is_finite(&self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.z.is_finite()
            && self.yaw.is_finite()
            && self.pitch.is_finite()
    }

    /// Answers whether the block of the place lies within the bounds of the world that the
    /// command tp takes, `Level.isInSpawnableBounds`: x and z from -30000000 to below 30000000,
    /// and y from -20000000 to below 20000000.
    pub fn is_in_world(&self) -> bool {
        let horizontal = -30_000_000.0..30_000_000.0;
        horizontal.contains(&self.x.floor())
            && horizontal.contains(&self.z.floor())
            && (-20_000_000.0..20_000_000.0).contains(&self.y.floor())
    }
}

impl fmt::Display for Location {
    /// Writes the dimension, the coordinates with two decimals and the rotation with one, as
    /// the log and the answers of a command show a location.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {:.2} {:.2} {:.2}, yaw {:.1}, pitch {:.1}",
            self.dimension, self.x, self.y, self.z, self.yaw, self.pitch
        )
    }
}

/// A block in the world, such as a bed or the spawn of a dimension.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockLocation {
    /// The id of the dimension, such as minecraft:overworld.
    pub dimension: String,
    /// The x coordinate of the block.
    pub x: i32,
    /// The y coordinate of the block.
    pub y: i32,
    /// The z coordinate of the block.
    pub z: i32,
}

impl fmt::Display for BlockLocation {
    /// Writes the dimension and the coordinates.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {} {} {}",
            self.dimension, self.x, self.y, self.z
        )
    }
}

/// The shared spawn of a dimension.
#[derive(Clone, Debug, PartialEq)]
pub struct Spawn {
    /// The block of the spawn.
    pub block: BlockLocation,
    /// The angle a player faces there, in degrees.
    pub angle: f32,
}

/// The game mode of a player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameMode {
    /// Survival.
    Survival,
    /// Creative.
    Creative,
    /// Adventure.
    Adventure,
    /// Spectator.
    Spectator,
}

impl GameMode {
    /// Answers the name of the game mode as the game writes it, such as survival.
    pub fn name(self) -> &'static str {
        match self {
            Self::Survival => "survival",
            Self::Creative => "creative",
            Self::Adventure => "adventure",
            Self::Spectator => "spectator",
        }
    }

    /// Answers the game mode of its name as the game writes it.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "survival" => Some(Self::Survival),
            "creative" => Some(Self::Creative),
            "adventure" => Some(Self::Adventure),
            "spectator" => Some(Self::Spectator),
            _ => None,
        }
    }
}

/// A player online, as the game answers them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnlinePlayer {
    /// The UUID in its usual form with dashes.
    pub uuid: String,
    /// The name of the profile.
    pub name: String,
    /// The display name, which mods may change, as plain text.
    pub display_name: String,
    /// The name in the player list as plain text, or `None` while the client shows the name of
    /// the profile.
    pub list_name: Option<String>,
    /// The ping, in milliseconds.
    pub ping_ms: u32,
    /// The game mode.
    pub game_mode: GameMode,
    /// The permission level, the operator level, from 0 to 4.
    pub level: u8,
    /// The milliseconds since the last action of the player, for an automatic away mark.
    pub idle_ms: u64,
}

/// An item and how many of it a stack holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// The registry id, such as minecraft:diamond.
    pub id: String,
    /// The count of the stack.
    pub count: u32,
}

/// What the search for a safe place answers.
#[derive(Clone, Debug, PartialEq)]
pub enum SafePlace {
    /// The nearest safe place, with the dimension and the rotation of the target.
    Found(Location),
    /// No place within the radius is safe.
    Nowhere,
    /// The search met a chunk that is not loaded before it found a safe place. The game loads
    /// no chunk for a question, so a plugin asks again once the chunk is loaded.
    Unloaded,
}

/// The heights of the blocks of a dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Heights {
    /// The lowest y of a block, such as -64 in the Overworld.
    pub min: i32,
    /// The y above the highest block, such as 320 in the Overworld.
    pub max: i32,
}

/// What a handler may ask the game.
///
/// Every answer is `None` when the game cannot answer: off the server thread, without a game,
/// for a player who is not online, for a dimension the server does not have, or when the mod
/// failed. An answer that may be empty is an `Option` inside, such as `Some(None)` for a player
/// without a respawn point. A query reads the game at once, so a handler asks only what it needs.
pub trait Game {
    /// Answers every player online.
    fn players(&self) -> Option<Vec<OnlinePlayer>>;

    /// Answers where the player with `uuid` is.
    fn location(&self, uuid: &str) -> Option<Location>;

    /// Answers the respawn point that the player with `uuid` holds, which a bed, a respawn
    /// anchor or `/spawnpoint` set, as Minecraft stores it, or `Some(None)` when they have none.
    /// Minecraft checks the block only when the player respawns, so the point may name a bed
    /// that is gone.
    fn respawn(&self, uuid: &str) -> Option<Option<BlockLocation>>;

    /// Answers the shared spawn of a dimension.
    fn spawn(&self, dimension: &str) -> Option<Spawn>;

    /// Answers the item in the main hand of the player with `uuid`, or `Some(None)` for an empty
    /// hand.
    fn main_hand(&self, uuid: &str) -> Option<Option<Item>>;

    /// Answers the heights of the blocks of a dimension.
    fn heights(&self, dimension: &str) -> Option<Heights>;

    /// Answers the nearest place within `radius` blocks of `target` where a player stands
    /// safely, with the dimension and the rotation of the target, or that there is none. The
    /// radius is held to at most 8. The search reads loaded chunks alone and loads none, so at
    /// the first place in its order whose chunk is not loaded, before it found a safe one, it
    /// answers [`SafePlace::Unloaded`].
    fn safe_location(&self, target: &Location, radius: u32) -> Option<SafePlace>;

    /// Answers the player online whose name or UUID is `who`, in any case, or `Some(None)` when
    /// nobody online has it.
    fn player(&self, who: &str) -> Option<Option<OnlinePlayer>> {
        let players = self.players()?;
        Some(players.into_iter().find(|player| {
            player.name.eq_ignore_ascii_case(who) || player.uuid.eq_ignore_ascii_case(who)
        }))
    }
}

/// A game that answers nothing, for a call without a game, such as a test of a plugin.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoGame;

impl Game for NoGame {
    fn players(&self) -> Option<Vec<OnlinePlayer>> {
        None
    }

    fn location(&self, _uuid: &str) -> Option<Location> {
        None
    }

    fn respawn(&self, _uuid: &str) -> Option<Option<BlockLocation>> {
        None
    }

    fn spawn(&self, _dimension: &str) -> Option<Spawn> {
        None
    }

    fn main_hand(&self, _uuid: &str) -> Option<Option<Item>> {
        None
    }

    fn heights(&self, _dimension: &str) -> Option<Heights> {
        None
    }

    fn safe_location(&self, _target: &Location, _radius: u32) -> Option<SafePlace> {
        None
    }
}

/// The wire form of the answers of the mod, the fields joined by U+001F, as okf/bridge.md
/// specifies them.
///
/// Each function reads the answer of one method of the interface Game of the host jar, which is
/// not null, and answers `None` when it does not read: a wrong number of fields, a number that
/// does not read or is not finite, or a game mode the game does not have. An operator level
/// outside 0 to 4, which the game allows, reads as the nearest of them. The empty answer is
/// the inner `None` of the answers that may be empty, and [`SafePlace::Nowhere`] for a safe
/// place.
pub mod wire {
    use std::sync::atomic::{
        AtomicBool,
        Ordering, //
    };

    use crate::{
        BlockLocation,
        GameMode,
        Heights,
        Item,
        Location,
        OnlinePlayer,
        SEPARATOR,
        SafePlace,
        Spawn, //
    };

    /// The answer of `safeLocation` when the search met a chunk that is not loaded.
    pub const UNLOADED: &str = "unloaded";

    /// Whether an operator level outside 0 to 4 was already logged.
    static LEVEL_LOGGED: AtomicBool = AtomicBool::new(false);

    /// Answers the fields of an answer.
    fn fields(answer: &str) -> Vec<&str> {
        answer.split(SEPARATOR).collect()
    }

    /// Answers a coordinate, which must be finite.
    fn coordinate(field: &str) -> Option<f64> {
        field.parse::<f64>().ok().filter(|value| value.is_finite())
    }

    /// Answers an angle, which must be finite.
    fn angle(field: &str) -> Option<f32> {
        field.parse::<f32>().ok().filter(|value| value.is_finite())
    }

    /// Reads one answer of `players`.
    pub fn player(answer: &str) -> Option<OnlinePlayer> {
        let [
            uuid,
            name,
            display_name,
            ping,
            game_mode,
            level,
            idle,
            list_name,
        ] = fields(answer)[..]
        else {
            return None;
        };
        Some(OnlinePlayer {
            uuid: uuid.to_owned(),
            name: name.to_owned(),
            display_name: display_name.to_owned(),
            list_name: (!list_name.is_empty()).then(|| list_name.to_owned()),
            // A ping below 0, which the game never answers, counts as 0.
            ping_ms: u32::try_from(ping.parse::<i64>().ok()?.max(0)).ok()?,
            game_mode: GameMode::from_name(game_mode)?,
            level: operator_level(name, level)?,
            idle_ms: idle.parse().ok()?,
        })
    }

    /// Reads the operator level of the player `name`, held to 0 to 4.
    ///
    /// The game takes the level of ops.json and of op-permission-level in server.properties
    /// without a bound, so `/op` may give a level of 5. The level reads as the nearest level the
    /// contract knows, and the first such level of the process is logged.
    fn operator_level(name: &str, field: &str) -> Option<u8> {
        let level: i64 = field.parse().ok()?;
        let held = level.clamp(0, i64::from(crate::MAX_LEVEL));
        if held != level && !LEVEL_LOGGED.swap(true, Ordering::Relaxed) {
            crate::log(format_args!(
                "the game answered the operator level {level} for {name}, which lies outside 0 to 4, so Golem reads it as {held}, and later levels like it are not logged"
            ));
        }
        u8::try_from(held).ok()
    }

    /// Reads the answer of `location`.
    pub fn location(answer: &str) -> Option<Location> {
        let [dimension, x, y, z, yaw, pitch] = fields(answer)[..] else {
            return None;
        };
        Some(Location {
            dimension: dimension.to_owned(),
            x: coordinate(x)?,
            y: coordinate(y)?,
            z: coordinate(z)?,
            yaw: angle(yaw)?,
            pitch: angle(pitch)?,
        })
    }

    /// Reads the answer of `respawn`.
    pub fn respawn(answer: &str) -> Option<Option<BlockLocation>> {
        if answer.is_empty() {
            return Some(None);
        }
        let [dimension, x, y, z] = fields(answer)[..] else {
            return None;
        };
        Some(Some(BlockLocation {
            dimension: dimension.to_owned(),
            x: x.parse().ok()?,
            y: y.parse().ok()?,
            z: z.parse().ok()?,
        }))
    }

    /// Reads the answer of `spawn` for `dimension`.
    pub fn spawn(dimension: &str, answer: &str) -> Option<Spawn> {
        let [x, y, z, spawn_angle] = fields(answer)[..] else {
            return None;
        };
        Some(Spawn {
            block: BlockLocation {
                dimension: dimension.to_owned(),
                x: x.parse().ok()?,
                y: y.parse().ok()?,
                z: z.parse().ok()?,
            },
            angle: angle(spawn_angle)?,
        })
    }

    /// Reads the answer of `mainHand`.
    pub fn main_hand(answer: &str) -> Option<Option<Item>> {
        if answer.is_empty() {
            return Some(None);
        }
        let [id, count] = fields(answer)[..] else {
            return None;
        };
        Some(Some(Item {
            id: id.to_owned(),
            count: count.parse().ok()?,
        }))
    }

    /// Reads the answer of `heights`.
    pub fn heights(answer: &str) -> Option<Heights> {
        let [min, max] = fields(answer)[..] else {
            return None;
        };
        Some(Heights {
            min: min.parse().ok()?,
            max: max.parse().ok()?,
        })
    }

    /// Reads the answer of `safeLocation` for `target`, whose dimension and rotation it keeps.
    pub fn safe_location(target: &Location, answer: &str) -> Option<SafePlace> {
        match answer {
            "" => return Some(SafePlace::Nowhere),
            UNLOADED => return Some(SafePlace::Unloaded),
            _ => {}
        }
        let [x, y, z] = fields(answer)[..] else {
            return None;
        };
        Some(SafePlace::Found(Location {
            x: coordinate(x)?,
            y: coordinate(y)?,
            z: coordinate(z)?,
            ..target.clone()
        }))
    }
}
