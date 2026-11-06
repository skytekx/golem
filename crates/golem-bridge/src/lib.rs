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
 * queue at the end of every server tick and runs them on the server thread.
 *
 * okf/bridge.md is the contract between this crate and the Java of bridge/, with the signature
 * of every native method and the wire form of every action.
 */

//! The bridge between the Golem bridge mod in the JVM and the plugins in Rust.

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

/// Who runs a command of a plugin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sender {
    /// The server console, which includes RCON.
    Console,
    /// A player in the game.
    Player(Player),
}

/// What the mod reports, each on the server thread.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    },
    /// A player respawned.
    Respawn {
        /// Who respawned.
        player: Player,
        /// The dimension of the respawn, such as minecraft:overworld.
        dimension: String,
    },
}

/// What a handler answers to an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The game goes on as it would.
    Pass,
    /// The game drops the event, where the event can be cancelled.
    Cancel,
}

/// What Rust asks the server thread to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Run a command as the console, without its leading slash.
    Command(String),
    /// Send a system message to one player.
    Message {
        /// The UUID of the player.
        uuid: String,
        /// The plain text.
        text: String,
    },
    /// Send a system message to every player and the console.
    Broadcast(String),
}

/// The queue of actions that the mod drains at the end of every server tick.
///
/// A clone shares the queue, so a signal handler, a plugin and the API push into the same one.
#[derive(Clone, Debug, Default)]
pub struct Actions {
    queue: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<Action>>>,
}

impl Actions {
    /// An empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues one action behind the others.
    pub fn push(&self, action: Action) {
        let _ = action;
        todo!("golem-bridge: Actions::push")
    }

    /// Takes every queued action, oldest first.
    pub fn drain(&self) -> Vec<Action> {
        todo!("golem-bridge: Actions::drain")
    }
}

/// What answers the events and the commands of the mod.
///
/// The plugin runtime of golem-plugin is the handler of a running Golem. Every method runs on
/// the server thread, so it must return quickly.
pub trait Handler: Send {
    /// Answers an event.
    fn event(&mut self, event: &Event, actions: &Actions) -> Verdict;

    /// The names of the commands the mod registers, each without its slash.
    fn command_names(&self) -> Vec<String>;

    /// Runs a command and answers the lines the sender sees.
    fn command(
        &mut self,
        name: &str,
        sender: &Sender,
        arguments: &str,
        actions: &Actions,
    ) -> Vec<String>;
}

/// The tick statistics of the running server.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TickStats {
    /// The ticks since the start.
    pub ticks: u64,
    /// The mean milliseconds per tick over the last 100 ticks.
    pub mspt: f64,
    /// The ticks per second over the last 100 ticks, at most 20.
    pub tps: f64,
}

/// Installs the handler that the native methods call, and answers the queue they drain.
///
/// The native methods are plain functions, so the handler lives in a global of this crate. A
/// second call replaces the handler and keeps the queue.
pub fn install(handler: Box<dyn Handler>) -> Actions {
    let _ = handler;
    todo!("golem-bridge: install")
}

/// Registers the native methods of the host class on the JVM of `env`.
///
/// The host calls this on its own thread after the JVM exists and before the main class runs.
pub fn register_natives(env: &mut jni::JNIEnv<'_>) -> jni::errors::Result<()> {
    let _ = env;
    todo!("golem-bridge: register_natives")
}

/// The tick statistics so far.
pub fn stats() -> TickStats {
    todo!("golem-bridge: stats")
}
