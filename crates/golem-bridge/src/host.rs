// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The host of golem-bridge: the global state behind the native methods of the host class, and
 * one plain Rust function per native method that does its whole work without a JVM.
 *
 * The native methods convert what the JVM hands them and call the functions of this file, so
 * golem-devtool and the notebooks drive the same path that the mod drives on the server thread.
 */

//! The native methods of the host class as plain Rust functions.
//!
//! Each function of this module is named after its native method of okf/bridge.md and takes
//! the arguments of the JVM after their conversion, with `None` for a null string. It never
//! panics: a panic of the handler is caught, logged and answered with the pass value, which is
//! nothing, `false` or an empty list. A null where the contract names a string is a fault of
//! the mod, which is logged and answered with the pass value as well, with two exceptions that
//! the contract names: a command of the console has neither a sender UUID nor a sender name,
//! and a null `arguments` reads as the empty string.
//!
//! The handler sits behind a lock. The server thread takes it for most natives, but Forge 47
//! calls `chat()` on a worker thread and `commandNames()` on the main thread at startup, and
//! mods call `permission()` on any thread. The server thread waits for the lock as long as the
//! call of another thread runs, which never waits for the server thread, since the game answers
//! none off the server thread. A call of another thread waits at most [`OFF_THREAD_WAIT`] once
//! the server thread is known, and then answers the pass value with a line on the log at powers
//! of ten: a question of the server thread may wait for a worker of the server, such as the one
//! that runs the chat message, and a wait without end would keep both waiting. A call from
//! inside the handler itself, on the thread that holds the lock, answers the pass value at once,
//! since it would wait for itself forever, and the log names the first such call of each
//! native.
//!
//! Every function that calls the handler takes the game that the native may ask, and hands the
//! handler that game behind [`ServerThread`], which answers none off the server thread. The
//! server thread is the thread of the first `serverStarting()`, which Forge calls there.

use std::{
    any::Any,
    cell::Cell,
    collections::HashSet,
    panic::{
        self,
        AssertUnwindSafe, //
    },
    sync::{
        Mutex,
        MutexGuard,
        OnceLock,
        PoisonError,
        TryLockError,
        atomic::{
            AtomicBool,
            Ordering, //
        }, //
    },
    thread::{
        self,
        ThreadId, //
    },
    time::{
        Duration,
        Instant, //
    }, //
};

use crate::{
    Action,
    Actions,
    BlockLocation,
    Event,
    Game,
    Handler,
    Heights,
    Item,
    Location,
    MAX_LEVEL,
    NoGame,
    OnlinePlayer,
    Player,
    SafePlace,
    Sender,
    Spawn,
    Throttle,
    TickStats,
    TickWindow,
    Verdict,
    log, //
};

/// The version of Golem that `version()` answers.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How long a call of another thread than the server thread waits for the handler before it
/// answers the pass value.
pub const OFF_THREAD_WAIT: Duration = Duration::from_secs(1);

/// The place of the handler that [`install`] fills.
type Slot = Option<Box<dyn Handler>>;

/// What the native methods share: the handler, the queue of actions and the ticks.
#[derive(Default)]
struct Host {
    handler: Mutex<Slot>,
    actions: Actions,
    ticks: Mutex<TickWindow>,
}

/// The state of the process, made on its first use.
static HOST: OnceLock<Host> = OnceLock::new();

/// True once the mod has called `version()`, which it does while Forge constructs it.
static CONNECTED: AtomicBool = AtomicBool::new(false);

/// True from the first `tick()` until `serverStopped()`, while the mod drains the queue of actions
/// at the end of every server tick.
static TICKING: AtomicBool = AtomicBool::new(false);

/// The thread of the first `serverStarting()`, the server thread, on which the game may be asked.
static SERVER_THREAD: OnceLock<ThreadId> = OnceLock::new();

/// Whether a question to the game off the server thread was already logged.
static OFF_THREAD_LOGGED: AtomicBool = AtomicBool::new(false);

/// The natives whose call from inside the handler was already logged.
static REENTRY_LOGGED: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);

/// The calls of other threads that waited for the handler in vain, by their native.
static WAITED: Throttle = Throttle::new();

thread_local! {
    /// True while the handler runs on this thread, which tells a call from inside the handler
    /// apart from a call of another thread that waits for the lock.
    static IN_HANDLER: Cell<bool> = const { Cell::new(false) };
}

/// Marks the handler as running on this thread until it is dropped, also when the handler
/// panics and the panic unwinds through the call.
struct Inside;

impl Inside {
    fn enter() -> Self {
        IN_HANDLER.set(true);
        Self
    }
}

impl Drop for Inside {
    fn drop(&mut self) {
        IN_HANDLER.set(false);
    }
}

fn host() -> &'static Host {
    HOST.get_or_init(Host::default)
}

/// Installs the handler that the native methods call, and answers the queue they drain.
///
/// A second call replaces the handler and keeps the queue, with the actions in it. A call from
/// inside the handler is logged and keeps the running handler, since the handler holds the lock
/// this function would wait for.
pub fn install(handler: Box<dyn Handler>) -> Actions {
    let host = host();
    if IN_HANDLER.get() {
        log(format_args!(
            "install was called from inside the handler, so the running handler stays"
        ));
    } else {
        *host.handler.lock().unwrap_or_else(PoisonError::into_inner) = Some(handler);
    }
    host.actions.clone()
}

/// Answers the queue of actions that `drain()` empties.
pub fn actions() -> Actions {
    host().actions.clone()
}

/// Answers the tick statistics so far.
pub fn stats() -> TickStats {
    host()
        .ticks
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .stats()
}

/// Runs `body` and answers `pass` when it panics, after a line on the log that names `native`.
///
/// Every function of this module runs its work through it, and so do the native methods
/// around their conversions, so no panic of Rust unwinds into the JVM.
pub fn guard<T>(native: &str, pass: T, body: impl FnOnce() -> T) -> T {
    match panic::catch_unwind(AssertUnwindSafe(body)) {
        Ok(value) => value,
        Err(payload) => {
            log(format_args!(
                "the native {native} panicked with \"{}\", so it answers the pass value",
                panic_message(payload.as_ref())
            ));
            pass
        }
    }
}

/// Answers the message of a panic, from the payload that `catch_unwind` caught.
pub fn panic_message(payload: &(dyn Any + Send)) -> &str {
    if let Some(message) = payload.downcast_ref::<&str>() {
        message
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message
    } else {
        "a payload that is no string"
    }
}

/// The game behind the guard of the server thread: every question answers none on another
/// thread, or before the server started, and the first such question is logged.
///
/// Rust never reads the game from another thread. A handler that is called beside the server
/// thread, such as on a chat message, gets this guard and must do without the game.
pub struct ServerThread<'a>(pub &'a dyn Game);

impl ServerThread<'_> {
    /// Answers whether the game may be asked on this thread, and logs the first refusal.
    fn allowed(question: &str) -> bool {
        if SERVER_THREAD.get() == Some(&thread::current().id()) {
            return true;
        }
        if !OFF_THREAD_LOGGED.swap(true, Ordering::Relaxed) {
            log(format_args!(
                "a plugin asked the game for {question} off the server thread, so the game answers none, and later questions like it are not logged"
            ));
        }
        false
    }
}

impl Game for ServerThread<'_> {
    fn players(&self) -> Option<Vec<OnlinePlayer>> {
        Self::allowed("players").then(|| self.0.players()).flatten()
    }

    fn location(&self, uuid: &str) -> Option<Location> {
        Self::allowed("location")
            .then(|| self.0.location(uuid))
            .flatten()
    }

    fn respawn(&self, uuid: &str) -> Option<Option<BlockLocation>> {
        Self::allowed("respawn")
            .then(|| self.0.respawn(uuid))
            .flatten()
    }

    fn spawn(&self, dimension: &str) -> Option<Spawn> {
        Self::allowed("spawn")
            .then(|| self.0.spawn(dimension))
            .flatten()
    }

    fn main_hand(&self, uuid: &str) -> Option<Option<Item>> {
        Self::allowed("mainHand")
            .then(|| self.0.main_hand(uuid))
            .flatten()
    }

    fn heights(&self, dimension: &str) -> Option<Heights> {
        Self::allowed("heights")
            .then(|| self.0.heights(dimension))
            .flatten()
    }

    fn safe_location(&self, target: &Location, radius: u32) -> Option<SafePlace> {
        Self::allowed("safeLocation")
            .then(|| self.0.safe_location(target, radius))
            .flatten()
    }
}

/// Answers the version of Golem, the answer of `version()`, and marks the bridge connected.
pub fn version() -> &'static str {
    CONNECTED.store(true, Ordering::Release);
    VERSION
}

/// Answers whether the mod has called `version()`, which it does while Forge constructs it.
pub fn connected() -> bool {
    CONNECTED.load(Ordering::Acquire)
}

/// Answers whether the mod drains the queue of actions at the end of every server tick, which it
/// does from the first `tick()` until `serverStopped()`.
pub fn ticking() -> bool {
    TICKING.load(Ordering::Acquire)
}

/// Reports `ServerStartingEvent` with the versions of Forge and Minecraft, and takes the thread
/// of the first call as the server thread.
pub fn server_starting(
    forge_version: Option<&str>,
    minecraft_version: Option<&str>,
    game: &dyn Game,
) {
    const NATIVE: &str = "serverStarting";
    let _ = SERVER_THREAD.set(thread::current().id());
    guard(NATIVE, (), || {
        let (Some(forge_version), Some(minecraft_version)) = (
            required(NATIVE, "forgeVersion", forge_version),
            required(NATIVE, "minecraftVersion", minecraft_version),
        ) else {
            return;
        };
        dispatch(
            NATIVE,
            &Event::ServerStarting {
                forge_version,
                minecraft_version,
            },
            game,
        );
    });
}

/// Reports `ServerStartedEvent`.
pub fn server_started(game: &dyn Game) {
    guard("serverStarted", (), || {
        dispatch("serverStarted", &Event::ServerStarted, game);
    });
}

/// Reports `ServerStoppingEvent`.
pub fn server_stopping(game: &dyn Game) {
    guard("serverStopping", (), || {
        dispatch("serverStopping", &Event::ServerStopping, game);
    });
}

/// Reports `ServerStoppedEvent`, after which no tick drains the queue of actions.
pub fn server_stopped(game: &dyn Game) {
    TICKING.store(false, Ordering::Release);
    guard("serverStopped", (), || {
        dispatch("serverStopped", &Event::ServerStopped, game);
    });
}

/// Records the end of a server tick that took `nanos`, and reports it.
///
/// The tick enters the statistics before the handler sees it, so a plugin that reads them on
/// the tick reads that tick too. A negative duration, which the mod never sends, counts as 0.
pub fn tick(nanos: i64, game: &dyn Game) {
    // The mod drains the queue right after this call, at the end of the same tick.
    TICKING.store(true, Ordering::Release);
    guard("tick", (), || {
        let nanos = u64::try_from(nanos).unwrap_or(0);
        host()
            .ticks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .record(nanos, Instant::now());
        dispatch("tick", &Event::Tick { nanos }, game);
    });
}

/// Reports `PlayerLoggedInEvent`.
pub fn player_joined(uuid: Option<&str>, name: Option<&str>, game: &dyn Game) {
    const NATIVE: &str = "playerJoined";
    guard(NATIVE, (), || {
        if let Some(player) = player(NATIVE, uuid, name) {
            dispatch(NATIVE, &Event::PlayerJoined(player), game);
        }
    });
}

/// Reports `PlayerLoggedOutEvent`.
pub fn player_left(uuid: Option<&str>, name: Option<&str>, game: &dyn Game) {
    const NATIVE: &str = "playerLeft";
    guard(NATIVE, (), || {
        if let Some(player) = player(NATIVE, uuid, name) {
            dispatch(NATIVE, &Event::PlayerLeft(player), game);
        }
    });
}

/// Reports `ServerChatEvent` and answers true when the handler cancels it.
///
/// Forge calls it on a worker thread, so the guard of the game answers none to the handler.
pub fn chat(
    uuid: Option<&str>,
    name: Option<&str>,
    message: Option<&str>,
    game: &dyn Game,
) -> bool {
    const NATIVE: &str = "chat";
    guard(NATIVE, false, || {
        let (Some(player), Some(message)) = (
            player(NATIVE, uuid, name),
            required(NATIVE, "message", message),
        ) else {
            return false;
        };
        dispatch(NATIVE, &Event::Chat { player, message }, game) == Verdict::Cancel
    })
}

/// Reports the death of a player with the id of its damage type and where they died.
///
/// The native passes no location when the mod passed null for the dimension, which is logged
/// as a null of the dimension.
pub fn death(
    uuid: Option<&str>,
    name: Option<&str>,
    cause: Option<&str>,
    location: Option<Location>,
    game: &dyn Game,
) {
    const NATIVE: &str = "death";
    guard(NATIVE, (), || {
        let player = player(NATIVE, uuid, name);
        let cause = required(NATIVE, "cause", cause);
        if location.is_none() {
            log_null(NATIVE, "dimension");
        }
        let (Some(player), Some(cause), Some(location)) = (player, cause, location) else {
            return;
        };
        dispatch(
            NATIVE,
            &Event::Death {
                player,
                cause,
                location,
            },
            game,
        );
    });
}

/// Reports `PlayerRespawnEvent` with the id of the dimension and whether the player left the
/// End through its exit portal, `isEndConquered` of the event.
pub fn respawn(
    uuid: Option<&str>,
    name: Option<&str>,
    dimension: Option<&str>,
    end_conquered: bool,
    game: &dyn Game,
) {
    const NATIVE: &str = "respawn";
    guard(NATIVE, (), || {
        let (Some(player), Some(dimension)) = (
            player(NATIVE, uuid, name),
            required(NATIVE, "dimension", dimension),
        ) else {
            return;
        };
        let event = Event::Respawn {
            player,
            dimension,
            end_conquered,
        };
        dispatch(NATIVE, &event, game);
    });
}

/// Answers the names of the commands the mod registers.
pub fn command_names() -> Vec<String> {
    const NATIVE: &str = "commandNames";
    guard(NATIVE, Vec::new(), || {
        with_handler(NATIVE, Vec::new(), &NoGame, |handler, _, _| {
            handler.command_names()
        })
    })
}

/// Runs a command and answers the lines its sender sees.
///
/// A sender with a UUID and a name is a player at the level of the source, one with neither at the
/// level 4 is the console, and one with a name alone is another source at its level. Every other
/// sender is a fault of the mod, and the command is refused rather than run as the console, which
/// may do more than any other source.
pub fn command(
    name: Option<&str>,
    sender_uuid: Option<&str>,
    sender_name: Option<&str>,
    level: i32,
    arguments: Option<&str>,
    game: &dyn Game,
) -> Vec<String> {
    const NATIVE: &str = "command";
    guard(NATIVE, Vec::new(), || {
        let Some(name) = required(NATIVE, "name", name) else {
            return Vec::new();
        };
        let Some(sender) = sender(sender_uuid, sender_name, level, &name) else {
            return Vec::new();
        };
        let arguments = arguments.unwrap_or_default();
        with_handler(NATIVE, Vec::new(), game, |handler, actions, game| {
            handler.command(&name, &sender, arguments, actions, game)
        })
    })
}

/// Answers the sender of the command `name` from what the mod passed, or logs why there is
/// none.
fn sender(uuid: Option<&str>, name: Option<&str>, level: i32, command: &str) -> Option<Sender> {
    let Some(level) = u8::try_from(level).ok().filter(|level| *level <= MAX_LEVEL) else {
        log(format_args!(
            "the native command got the level {level}, which is no permission level, so it refuses the command {command}"
        ));
        return None;
    };
    match (uuid, name) {
        (Some(uuid), Some(name)) => Some(Sender::Player {
            player: Player {
                uuid: uuid.to_owned(),
                name: name.to_owned(),
            },
            level,
        }),
        (None, None) if level == MAX_LEVEL => Some(Sender::Console),
        (None, Some(name)) => Some(Sender::Other {
            name: name.to_owned(),
            level,
        }),
        (None, None) => {
            log(format_args!(
                "the native command got neither senderUuid nor senderName at the level {level}, which only the console has at 4, so it refuses the command {command}"
            ));
            None
        }
        (Some(_), None) => {
            log(format_args!(
                "the native command got senderUuid without senderName, so it refuses the command {command}"
            ));
            None
        }
    }
}

/// Answers whether the player with `uuid` holds the node of the `PermissionAPI` of Forge, or
/// `None` for the default resolver of the node, which is also the pass value.
pub fn permission(uuid: Option<&str>, node: Option<&str>, game: &dyn Game) -> Option<bool> {
    const NATIVE: &str = "permission";
    guard(NATIVE, None, || {
        let (Some(uuid), Some(node)) = (
            required(NATIVE, "uuid", uuid),
            required(NATIVE, "node", node),
        ) else {
            return None;
        };
        with_handler(NATIVE, None, game, |handler, actions, game| {
            handler.permission(&uuid, &node, actions, game)
        })
    })
}

/// Takes every queued action, oldest first, each in its wire form.
pub fn drain() -> Vec<String> {
    guard("drain", Vec::new(), || {
        host().actions.drain().iter().map(Action::encode).collect()
    })
}

/// Puts the actions of a drain whose answer never reached the mod back in front of the queue.
///
/// The native `drain()` calls it when it cannot hand its array to the JVM, so a failed call of
/// the JNI loses no action, such as the stop of the server.
pub(crate) fn requeue(lines: &[String]) {
    let actions: Vec<Action> = lines
        .iter()
        .filter_map(|line| Action::decode(line))
        .collect();
    log(format_args!(
        "the native drain could not answer {} actions, so they wait for the next tick",
        actions.len()
    ));
    host().actions.requeue(actions);
}

/// Hands `event` to the handler and answers its verdict, or a pass without a handler.
fn dispatch(native: &'static str, event: &Event, game: &dyn Game) -> Verdict {
    with_handler(native, Verdict::Pass, game, |handler, actions, game| {
        handler.event(event, actions, game)
    })
}

/// Calls the handler with the queue, or answers `pass` without a handler, from inside it, or
/// when the handler stays taken.
///
/// A call of another thread waits until the handler is free, so a chat message that arrives
/// on a worker thread while a tick runs on the server thread still reaches the plugins, but at
/// most [`OFF_THREAD_WAIT`] once the server thread is known.
fn with_handler<T>(
    native: &'static str,
    pass: T,
    game: &dyn Game,
    call: impl FnOnce(&mut dyn Handler, &Actions, &dyn Game) -> T,
) -> T {
    if IN_HANDLER.get() {
        let mut logged = REENTRY_LOGGED
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if logged.get_or_insert_with(HashSet::new).insert(native) {
            log(format_args!(
                "the native {native} was called from inside the handler, so it answers the pass value, and later calls like it are not logged"
            ));
        }
        return pass;
    }
    let host = host();
    let Some(mut slot) = lock_handler(native, host) else {
        return pass;
    };
    let Some(handler) = slot.as_deref_mut() else {
        return pass;
    };
    let _inside = Inside::enter();
    call(handler, &host.actions, &ServerThread(game))
}

/// Takes the lock of the handler for a call of `native`: at once or after the call that holds it
/// on the server thread and before the server thread is known, and within [`OFF_THREAD_WAIT`]
/// on any other thread, else `None` with a line on the log at powers of ten.
fn lock_handler(native: &'static str, host: &'static Host) -> Option<MutexGuard<'static, Slot>> {
    if SERVER_THREAD
        .get()
        .is_none_or(|server| *server == thread::current().id())
    {
        return Some(host.handler.lock().unwrap_or_else(PoisonError::into_inner));
    }
    let deadline = Instant::now() + OFF_THREAD_WAIT;
    let mut pause = Duration::from_micros(50);
    loop {
        match host.handler.try_lock() {
            Ok(slot) => return Some(slot),
            Err(TryLockError::Poisoned(poisoned)) => return Some(poisoned.into_inner()),
            Err(TryLockError::WouldBlock) => {}
        }
        let now = Instant::now();
        if now >= deadline {
            if let Some(count) = WAITED.count(native) {
                log(format_args!(
                    "the native {native} waited {} ms for the handler, which another call holds, so it answers the pass value, {count} times so far",
                    OFF_THREAD_WAIT.as_millis()
                ));
            }
            return None;
        }
        thread::sleep(pause.min(deadline - now));
        pause = (pause * 2).min(Duration::from_millis(1));
    }
}

/// Answers the string of a field that the contract names, or logs its absence.
fn required(native: &str, field: &str, value: Option<&str>) -> Option<String> {
    if value.is_none() {
        log_null(native, field);
    }
    value.map(str::to_owned)
}

/// Logs that the mod passed null for a field that the contract names.
fn log_null(native: &str, field: &str) {
    log(format_args!(
        "the native {native} got null for {field}, so it answers the pass value"
    ));
}

/// Answers the player of a UUID and a name that the contract names, or logs their absence.
fn player(native: &str, uuid: Option<&str>, name: Option<&str>) -> Option<Player> {
    let uuid = required(native, "uuid", uuid);
    let name = required(native, "name", name);
    Some(Player {
        uuid: uuid?,
        name: name?,
    })
}
