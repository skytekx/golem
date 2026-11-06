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
//! the contract names: a command of the console has no sender, and a null `arguments` reads as
//! the empty string.
//!
//! The handler sits behind a lock. The server thread takes it for most natives, but Forge 47
//! calls `chat()` on a worker thread and `commandNames()` on the main thread at startup, so a
//! call that finds the lock taken by another thread waits for it, which takes no longer than one
//! call of the handler. A call from inside the handler itself, on the thread that holds the
//! lock, answers the pass value at once, since it would wait for itself forever.

use std::{
    any::Any,
    cell::Cell,
    panic::{
        self,
        AssertUnwindSafe, //
    },
    sync::{
        Mutex,
        OnceLock,
        PoisonError, //
    },
    time::Instant, //
};

use crate::{
    Action,
    Actions,
    Event,
    Handler,
    Player,
    Sender,
    TickStats,
    TickWindow,
    Verdict,
    log, //
};

/// The version of Golem that `version()` answers.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the native methods share: the handler, the queue of actions and the ticks.
#[derive(Default)]
struct Host {
    handler: Mutex<Option<Box<dyn Handler>>>,
    actions: Actions,
    ticks: Mutex<TickWindow>,
}

/// The state of the process, made on its first use.
static HOST: OnceLock<Host> = OnceLock::new();

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

/// Answers the version of Golem, the answer of `version()`.
pub fn version() -> &'static str {
    VERSION
}

/// Reports `ServerStartingEvent` with the versions of Forge and Minecraft.
pub fn server_starting(forge_version: Option<&str>, minecraft_version: Option<&str>) {
    const NATIVE: &str = "serverStarting";
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
        );
    });
}

/// Reports `ServerStartedEvent`.
pub fn server_started() {
    guard("serverStarted", (), || {
        dispatch("serverStarted", &Event::ServerStarted);
    });
}

/// Reports `ServerStoppingEvent`.
pub fn server_stopping() {
    guard("serverStopping", (), || {
        dispatch("serverStopping", &Event::ServerStopping);
    });
}

/// Reports `ServerStoppedEvent`.
pub fn server_stopped() {
    guard("serverStopped", (), || {
        dispatch("serverStopped", &Event::ServerStopped);
    });
}

/// Records the end of a server tick that took `nanos`, and reports it.
///
/// The tick enters the statistics before the handler sees it, so a plugin that reads them on
/// the tick reads that tick too. A negative duration, which the mod never sends, counts as 0.
pub fn tick(nanos: i64) {
    guard("tick", (), || {
        let nanos = u64::try_from(nanos).unwrap_or(0);
        host()
            .ticks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .record(nanos, Instant::now());
        dispatch("tick", &Event::Tick { nanos });
    });
}

/// Reports `PlayerLoggedInEvent`.
pub fn player_joined(uuid: Option<&str>, name: Option<&str>) {
    const NATIVE: &str = "playerJoined";
    guard(NATIVE, (), || {
        if let Some(player) = player(NATIVE, uuid, name) {
            dispatch(NATIVE, &Event::PlayerJoined(player));
        }
    });
}

/// Reports `PlayerLoggedOutEvent`.
pub fn player_left(uuid: Option<&str>, name: Option<&str>) {
    const NATIVE: &str = "playerLeft";
    guard(NATIVE, (), || {
        if let Some(player) = player(NATIVE, uuid, name) {
            dispatch(NATIVE, &Event::PlayerLeft(player));
        }
    });
}

/// Reports `ServerChatEvent` and answers true when the handler cancels it.
pub fn chat(uuid: Option<&str>, name: Option<&str>, message: Option<&str>) -> bool {
    const NATIVE: &str = "chat";
    guard(NATIVE, false, || {
        let (Some(player), Some(message)) = (
            player(NATIVE, uuid, name),
            required(NATIVE, "message", message),
        ) else {
            return false;
        };
        dispatch(NATIVE, &Event::Chat { player, message }) == Verdict::Cancel
    })
}

/// Reports the death of a player with the id of its damage type.
pub fn death(uuid: Option<&str>, name: Option<&str>, cause: Option<&str>) {
    const NATIVE: &str = "death";
    guard(NATIVE, (), || {
        let (Some(player), Some(cause)) =
            (player(NATIVE, uuid, name), required(NATIVE, "cause", cause))
        else {
            return;
        };
        dispatch(NATIVE, &Event::Death { player, cause });
    });
}

/// Reports `PlayerRespawnEvent` with the id of the dimension.
pub fn respawn(uuid: Option<&str>, name: Option<&str>, dimension: Option<&str>) {
    const NATIVE: &str = "respawn";
    guard(NATIVE, (), || {
        let (Some(player), Some(dimension)) = (
            player(NATIVE, uuid, name),
            required(NATIVE, "dimension", dimension),
        ) else {
            return;
        };
        dispatch(NATIVE, &Event::Respawn { player, dimension });
    });
}

/// Answers the names of the commands the mod registers.
pub fn command_names() -> Vec<String> {
    const NATIVE: &str = "commandNames";
    guard(NATIVE, Vec::new(), || {
        with_handler(NATIVE, Vec::new(), |handler, _| handler.command_names())
    })
}

/// Runs a command and answers the lines its sender sees.
///
/// A sender with neither a UUID nor a name is the console. A sender with only one of them is a
/// fault of the mod, and the command is refused rather than run as the console, which may do
/// more than any player.
pub fn command(
    name: Option<&str>,
    sender_uuid: Option<&str>,
    sender_name: Option<&str>,
    arguments: Option<&str>,
) -> Vec<String> {
    const NATIVE: &str = "command";
    guard(NATIVE, Vec::new(), || {
        let Some(name) = required(NATIVE, "name", name) else {
            return Vec::new();
        };
        let sender = match (sender_uuid, sender_name) {
            (None, None) => Sender::Console,
            (Some(uuid), Some(player)) => Sender::Player(Player {
                uuid: uuid.to_owned(),
                name: player.to_owned(),
            }),
            _ => {
                log(format_args!(
                    "the native command got a sender with only one of senderUuid and senderName, so it refuses the command {name}"
                ));
                return Vec::new();
            }
        };
        let arguments = arguments.unwrap_or_default();
        with_handler(NATIVE, Vec::new(), |handler, actions| {
            handler.command(&name, &sender, arguments, actions)
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
fn dispatch(native: &str, event: &Event) -> Verdict {
    with_handler(native, Verdict::Pass, |handler, actions| {
        handler.event(event, actions)
    })
}

/// Calls the handler with the queue, or answers `pass` without a handler or from inside it.
///
/// A call of another thread waits until the handler is free, so a chat message that arrives
/// on a worker thread while a tick runs on the server thread still reaches the plugins.
fn with_handler<T>(native: &str, pass: T, call: impl FnOnce(&mut dyn Handler, &Actions) -> T) -> T {
    if IN_HANDLER.get() {
        log(format_args!(
            "the native {native} was called from inside the handler, so it answers the pass value"
        ));
        return pass;
    }
    let host = host();
    let mut slot = host.handler.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(handler) = slot.as_deref_mut() else {
        return pass;
    };
    let _inside = Inside::enter();
    call(handler, &host.actions)
}

/// Answers the string of a field that the contract names, or logs its absence.
fn required(native: &str, field: &str, value: Option<&str>) -> Option<String> {
    if value.is_none() {
        log(format_args!(
            "the native {native} got null for {field}, so it answers the pass value"
        ));
    }
    value.map(str::to_owned)
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
