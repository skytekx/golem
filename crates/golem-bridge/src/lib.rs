// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * golem-bridge: the bridge between the Golem bridge mod in the JVM and the plugins in Rust.
 *
 * The bridge mod is a Forge mod. It listens on the Forge event bus and reports what happens to
 * the native methods of the host class com.skytekx.golem.host.Host, which lives in the jar
 * golem-host.jar on the module path of the JVM. The host registers those native methods before
 * it calls the main class, so no library is loaded from Java. Every native method runs on the
 * server thread, so a handler may answer a cancellable event before the game goes on.
 *
 * Rust never touches the game from another thread. It queues actions, and the mod drains the
 * queue at the end of every server tick and runs them on the server thread. While a native of the
 * server thread runs, a handler may also ask the game, through the object of the interface Game
 * that the mod connects and the JNIEnv of that native.
 *
 * okf/bridge.md is the contract between this crate and the Java of bridge/, with the signature
 * of every native method and the wire form of every action.
 */

//! The bridge between the Golem bridge mod in the JVM and the plugins in Rust.
//!
//! | Module | Holds |
//! | --- | --- |
//! | `actions` | [`Action`], [`Actions`], the queue, and the wire form of an action |
//! | `game` | [`Game`] and [`NoGame`], the places the game answers, and [`wire`], their wire form |
//! | `jni_game` | The game through the JNI, with the object the mod connected |
//! | `ticks` | [`TickWindow`] and [`TickStats`], the statistics of the last 100 ticks |
//! | [`host`] | The global state and one plain function per native method |
//! | `natives` | The functions the JVM calls, thin wrappers over [`host`], and their registration |
//!
//! The crate holds no test, which is the rule of CONTRIBUTING.md section 5. The notebook
//! `tests/notebooks/04_plugins.ipynb` drives the functions of [`host`] through golem-devtool.

mod actions;
mod game;
pub mod host;
mod jni_game;
mod natives;
mod ticks;

pub use crate::{
    actions::{
        Action,
        Actions,
        SEPARATOR, //
    },
    game::{
        BlockLocation,
        Game,
        GameMode,
        Heights,
        Item,
        Location,
        NoGame,
        OnlinePlayer,
        SafePlace,
        Spawn,
        wire, //
    },
    ticks::{
        MAX_TPS,
        TickStats,
        TickWindow,
        WINDOW, //
    },
};

/// The binary name of the host class, whose native methods the bridge registers.
pub const HOST_CLASS: &str = "com/skytekx/golem/host/Host";

/// A player as the mod reports them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    /// The UUID in its usual form with dashes.
    pub uuid: String,
    /// The name of the profile.
    pub name: String,
}

/// The permission level of the console and of RCON, the highest level of the game.
pub const MAX_LEVEL: u8 = 4;

/// Who runs a command of a plugin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sender {
    /// The server console or RCON: a source without an entity at the level [`MAX_LEVEL`].
    Console,
    /// A player who runs the command themself.
    ///
    /// A plugin judges them by the lower of `level` and the rights of the player, since the
    /// game may run a command of a player at another level, such as a function of an
    /// advancement at the level 2.
    Player {
        /// The player.
        player: Player,
        /// The permission level of the source, from 0 to [`MAX_LEVEL`], which is the operator
        /// level of the player unless the game ran the command at another one.
        level: u8,
    },
    /// Every other source: a command block, a function, or a command that another source runs
    /// as an entity through `execute as`, a player included, named after that entity at the
    /// level of the source, so it never borrows the rights of a player.
    Other {
        /// The name of the source as the game gives it, such as `@` for a command block.
        name: String,
        /// The permission level of the source, from 0 to [`MAX_LEVEL`].
        level: u8,
    },
}

/// What the mod reports, each on the server thread.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The server is starting, before the worlds load.
    ServerStarting {
        /// The version of Forge, such as 47.4.18.
        forge_version: String,
        /// The version of Minecraft, such as 1.20.1.
        minecraft_version: String,
    },
    /// The server has started and reached Done.
    ServerStarted,
    /// The server is stopping.
    ServerStopping,
    /// The server has stopped, after the worlds were saved.
    ServerStopped,
    /// A server tick ended.
    Tick {
        /// How long the tick took, in nanoseconds.
        nanos: u64,
    },
    /// A player joined.
    PlayerJoined(Player),
    /// A player left.
    PlayerLeft(Player),
    /// A player sent a chat message, which a handler may cancel.
    Chat {
        /// Who wrote it.
        player: Player,
        /// The raw text.
        message: String,
    },
    /// A player died.
    Death {
        /// Who died.
        player: Player,
        /// The id of the damage type, such as minecraft:fall.
        cause: String,
        /// Where they died.
        location: Location,
    },
    /// A player respawned.
    Respawn {
        /// Who respawned.
        player: Player,
        /// The dimension of the respawn, such as minecraft:overworld.
        dimension: String,
    },
}

impl Event {
    /// Answers the name of the native method that reports the event, such as chat.
    pub fn name(&self) -> &'static str {
        match self {
            Self::ServerStarting { .. } => "serverStarting",
            Self::ServerStarted => "serverStarted",
            Self::ServerStopping => "serverStopping",
            Self::ServerStopped => "serverStopped",
            Self::Tick { .. } => "tick",
            Self::PlayerJoined(_) => "playerJoined",
            Self::PlayerLeft(_) => "playerLeft",
            Self::Chat { .. } => "chat",
            Self::Death { .. } => "death",
            Self::Respawn { .. } => "respawn",
        }
    }

    /// Answers whether the game drops the event when a handler answers [`Verdict::Cancel`].
    ///
    /// Chat is the one event of the contract whose native method answers a verdict.
    pub fn cancellable(&self) -> bool {
        matches!(self, Self::Chat { .. })
    }
}

/// What a handler answers to an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The game goes on as it would.
    Pass,
    /// The game drops the event, where the event can be cancelled.
    Cancel,
}

/// What answers the events and the commands of the mod.
///
/// The plugin runtime of golem-plugin is the handler of a running Golem. Every method runs on
/// the server thread but those the contract names, so it must return quickly. Each method gets
/// the game behind the guard of the server thread, which answers none on any other thread.
pub trait Handler: Send {
    /// Answers an event.
    fn event(&mut self, event: &Event, actions: &Actions, game: &dyn Game) -> Verdict;

    /// The names of the commands the mod registers, each without its slash.
    fn command_names(&self) -> Vec<String>;

    /// Runs a command and answers the lines the sender sees.
    fn command(
        &mut self,
        name: &str,
        sender: &Sender,
        arguments: &str,
        actions: &Actions,
        game: &dyn Game,
    ) -> Vec<String>;

    /// Answers whether the player with `uuid` holds `node` of the `PermissionAPI` of Forge, or
    /// `None` to leave it to the default resolver of the node. The default answers `None`.
    ///
    /// Mods ask Forge on any thread, so this may run beside the server thread.
    fn permission(
        &mut self,
        uuid: &str,
        node: &str,
        actions: &Actions,
        game: &dyn Game,
    ) -> Option<bool> {
        let _ = (uuid, node, actions, game);
        None
    }
}

/// Installs the handler that the native methods call, and answers the queue they drain.
///
/// The native methods are plain functions, so the handler lives in a global of this crate. A
/// second call replaces the handler and keeps the queue. See [`host::install`].
pub fn install(handler: Box<dyn Handler>) -> Actions {
    host::install(handler)
}

/// Registers the native methods of the host class on the JVM of `env`.
///
/// The host calls this on its own thread after the JVM exists and before the main class runs.
/// `FindClass` looks the host class up, which on a thread of the invocation API goes through the
/// system class loader. On an error the pending exception of the JVM is printed and cleared, so
/// the caller may go on with the JVM.
pub fn register_natives(env: &mut jni::JNIEnv<'_>) -> jni::errors::Result<()> {
    natives::register(env)
}

/// Answers the name and the JNI descriptor of every native method that [`register_natives`]
/// registers, in the order of okf/bridge.md.
pub fn native_signatures() -> Vec<(&'static str, &'static str)> {
    natives::signatures()
}

/// The tick statistics so far, over the ticks that the native method `tick()` recorded.
pub fn stats() -> TickStats {
    host::stats()
}

/// Answers whether the bridge mod has reached the natives, so that it runs the queued actions.
/// A server without the mod in `mods/`, or before Forge constructs it, never runs them.
pub fn connected() -> bool {
    host::connected()
}

/// Writes one line to the log of the server, the standard error, behind the prefix `[golem]`.
///
/// It never panics, so a native method may log on its way out of a panic. A line that the
/// standard error does not take is lost.
pub fn log(message: std::fmt::Arguments<'_>) {
    use std::io::Write as _;

    let _ = writeln!(std::io::stderr().lock(), "[golem] {message}");
}

/// Counts the faults of each kind, so that a fault that repeats, such as on every tick, is
/// logged the first time, the tenth, the hundredth and at every further power of ten, as the
/// guard of the mod logs.
pub(crate) struct Throttle(std::sync::Mutex<Option<std::collections::HashMap<&'static str, u64>>>);

impl Throttle {
    /// A throttle that has counted nothing.
    pub(crate) const fn new() -> Self {
        Self(std::sync::Mutex::new(None))
    }

    /// Counts one more fault of `kind`, and answers the count when a line on the log is due.
    pub(crate) fn count(&self, kind: &'static str) -> Option<u64> {
        let mut counts = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let count = counts
            .get_or_insert_with(std::collections::HashMap::new)
            .entry(kind)
            .or_insert(0);
        *count += 1;
        let mut rest = *count;
        while rest.is_multiple_of(10) {
            rest /= 10;
        }
        (rest == 1).then_some(*count)
    }
}
