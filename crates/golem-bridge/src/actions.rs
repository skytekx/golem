// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The actions of golem-bridge: what Rust asks the server thread to do, the queue the mod drains
 * at the end of every server tick, and the wire form in which drain() hands each action to Java.
 */

use std::{
    collections::VecDeque,
    sync::{
        Arc,
        Mutex,
        MutexGuard,
        PoisonError,
        atomic::{
            AtomicBool,
            Ordering, //
        }, //
    }, //
};

/// The character between the fields of an action on the wire, U+001F, the unit separator.
pub const SEPARATOR: char = '\u{1f}';

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

/// Whether a field with the separator in it was already logged, so that a player who keeps
/// sending it fills no log.
static SEPARATOR_LOGGED: AtomicBool = AtomicBool::new(false);

impl Action {
    /// Answers the kind of the action, the first field of its wire form.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Command(_) => "command",
            Self::Message { .. } => "message",
            Self::Broadcast(_) => "broadcast",
        }
    }

    /// Encodes the action in the wire form of okf/bridge.md.
    ///
    /// The fields are joined by [`SEPARATOR`], and the first of them is the kind. No field may
    /// hold the separator, so a field that holds it has it replaced by a space, and the first
    /// such replacement of the process is logged.
    pub fn encode(&self) -> String {
        let fields: &[&str] = match self {
            Self::Command(command) => &[command],
            Self::Message { uuid, text } => &[uuid, text],
            Self::Broadcast(text) => &[text],
        };
        let mut wire = String::from(self.kind());
        for field in fields {
            wire.push(SEPARATOR);
            if field.contains(SEPARATOR) {
                log_separator(self.kind());
                wire.extend(field.chars().map(|c| if c == SEPARATOR { ' ' } else { c }));
            } else {
                wire.push_str(field);
            }
        }
        wire
    }

    /// Decodes the wire form of an action, as the mod reads it.
    ///
    /// Answers `None` for an unknown kind or a wrong number of fields, which the mod skips.
    pub fn decode(wire: &str) -> Option<Self> {
        let fields: Vec<&str> = wire.split(SEPARATOR).collect();
        match fields.as_slice() {
            ["command", command] => Some(Self::Command((*command).to_owned())),
            ["message", uuid, text] => Some(Self::Message {
                uuid: (*uuid).to_owned(),
                text: (*text).to_owned(),
            }),
            ["broadcast", text] => Some(Self::Broadcast((*text).to_owned())),
            _ => None,
        }
    }
}

/// Logs the first field of the process that held the separator.
fn log_separator(kind: &str) {
    if !SEPARATOR_LOGGED.swap(true, Ordering::Relaxed) {
        crate::log(format_args!(
            "a field of a {kind} action holds U+001F, which the wire form reserves, so it is sent as a space, and later fields like it are not logged again"
        ));
    }
}

/// The queue of actions that the mod drains at the end of every server tick.
///
/// A clone shares the queue, so a signal handler, a plugin and the API push into the same one.
/// The queue keeps working after a thread panicked while it held the lock, since a push or a
/// drain leaves the queue whole at every point.
#[derive(Clone, Debug, Default)]
pub struct Actions {
    queue: Arc<Mutex<VecDeque<Action>>>,
}

impl Actions {
    /// An empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues one action behind the others.
    pub fn push(&self, action: Action) {
        self.lock().push_back(action);
    }

    /// Takes every queued action, oldest first.
    pub fn drain(&self) -> Vec<Action> {
        self.lock().drain(..).collect()
    }

    /// Puts drained actions back in front of the queue, in their order, for a drain whose
    /// answer never reached the mod.
    pub(crate) fn requeue(&self, actions: Vec<Action>) {
        let mut queue = self.lock();
        for action in actions.into_iter().rev() {
            queue.push_front(action);
        }
    }

    /// Answers how many actions wait.
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// Answers whether no action waits.
    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    /// Answers the queue, also when a thread panicked while it held the lock.
    fn lock(&self) -> MutexGuard<'_, VecDeque<Action>> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
