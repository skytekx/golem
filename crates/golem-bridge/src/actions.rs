// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The actions of golem-bridge: what Rust asks the server thread to do, the queue the mod drains
 * at the end of every server tick, and the wire form in which drain() hands each action to Java.
 *
 * A number of an action travels as the shortest decimal that reads back as the same number,
 * which Display of Rust writes and Double.parseDouble of Java reads, and an infinity as Java
 * spells it, Infinity or -Infinity, since Display writes inf, which Java does not read.
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
            AtomicU64,
            Ordering, //
        }, //
    }, //
};

use crate::Location;

/// The character between the fields of an action on the wire, U+001F, the unit separator.
pub const SEPARATOR: char = '\u{1f}';

/// What Rust asks the server thread to do.
#[derive(Clone, Debug, PartialEq)]
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
    /// Send a styled text to one player as a system message.
    StyledMessage {
        /// The UUID of the player.
        uuid: String,
        /// The JSON of a [`Text`](crate::Text), which `Component.Serializer` of the game reads.
        json: String,
    },
    /// Send a styled text to every player but the hidden ones, and to the console.
    StyledBroadcast {
        /// The UUIDs of the players who do not get it, which never hold a comma.
        hidden: Vec<String>,
        /// The JSON of a [`Text`](crate::Text), which `Component.Serializer` of the game reads.
        json: String,
    },
    /// Teleport a player to a location, also into another dimension, and report how it ended
    /// with the events teleported and teleportFailed, which carry the id.
    Teleport {
        /// The id of the teleport, which [`Actions::teleport`] hands out once per process.
        id: u64,
        /// The UUID of the player.
        uuid: String,
        /// Where the player goes.
        location: Location,
        /// The radius of the search for a safe place near the location, which the mod runs
        /// once it loaded the chunks of the search, or `None` for the place as it is.
        safe: Option<u32>,
    },
    /// Set or clear the name of a player in the player list.
    ListName {
        /// The UUID of the player.
        uuid: String,
        /// The name the player list shows, or `None` for the name the client shows by itself.
        name: Option<String>,
    },
}

/// Whether a field with the separator in it was already logged, so that a player who keeps
/// sending it fills no log.
static SEPARATOR_LOGGED: AtomicBool = AtomicBool::new(false);

/// The id of the next teleport of the process.
static NEXT_TELEPORT: AtomicU64 = AtomicU64::new(1);

/// The largest radius of a search for a safe place, in blocks, which the mod holds every search
/// to.
pub const MAX_SAFE_RADIUS: u32 = 8;

/// Answers a new id for a teleport, one higher than the one before, from 1 on, so no two
/// teleports of a process share an id.
fn teleport_id() -> u64 {
    NEXT_TELEPORT.fetch_add(1, Ordering::Relaxed)
}

impl Action {
    /// Answers the kind of the action, the first field of its wire form.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Command(_) => "command",
            Self::Message { .. } => "message",
            Self::Broadcast(_) => "broadcast",
            Self::StyledMessage { .. } => "styledMessage",
            Self::StyledBroadcast { .. } => "styledBroadcast",
            Self::Teleport { .. } => "teleport",
            Self::ListName { .. } => "listName",
        }
    }

    /// Answers the fields of the wire form after the kind.
    fn fields(&self) -> Vec<String> {
        match self {
            Self::Command(command) => vec![command.clone()],
            Self::Message { uuid, text } => vec![uuid.clone(), text.clone()],
            Self::Broadcast(text) => vec![text.clone()],
            Self::StyledMessage { uuid, json } => vec![uuid.clone(), json.clone()],
            Self::StyledBroadcast { hidden, json } => vec![hidden.join(","), json.clone()],
            Self::Teleport {
                id,
                uuid,
                location,
                safe,
            } => vec![
                id.to_string(),
                uuid.clone(),
                location.dimension.clone(),
                number(location.x),
                number(location.y),
                number(location.z),
                angle(location.yaw),
                angle(location.pitch),
                safe.map(|radius| radius.min(MAX_SAFE_RADIUS).to_string())
                    .unwrap_or_default(),
            ],
            Self::ListName { uuid, name } => vec![uuid.clone(), name.clone().unwrap_or_default()],
        }
    }

    /// Encodes the action in the wire form of okf/bridge.md.
    ///
    /// The fields are joined by [`SEPARATOR`], and the first of them is the kind. No field may
    /// hold the separator, so a field that holds it has it replaced by a space, and the first
    /// such replacement of the process is logged. A list name of `None` and an empty one are
    /// both the empty field, which clears the name. The radius of a safe teleport is held to
    /// [`MAX_SAFE_RADIUS`], and a teleport without a search has the empty field. The hidden
    /// players of a styled broadcast are one field, their UUIDs joined by commas.
    pub fn encode(&self) -> String {
        let mut wire = String::from(self.kind());
        for field in self.fields() {
            wire.push(SEPARATOR);
            if field.contains(SEPARATOR) {
                log_separator(self.kind());
                wire.extend(field.chars().map(|c| if c == SEPARATOR { ' ' } else { c }));
            } else {
                wire.push_str(&field);
            }
        }
        wire
    }

    /// Decodes the wire form of an action, as the mod reads it.
    ///
    /// Answers `None` for an unknown kind, a wrong number of fields or a number that does not
    /// read, which the mod skips. An empty list name and an empty radius read as `None`.
    pub fn decode(wire: &str) -> Option<Self> {
        let fields: Vec<&str> = wire.split(SEPARATOR).collect();
        match fields.as_slice() {
            ["command", command] => Some(Self::Command((*command).to_owned())),
            ["message", uuid, text] => Some(Self::Message {
                uuid: (*uuid).to_owned(),
                text: (*text).to_owned(),
            }),
            ["broadcast", text] => Some(Self::Broadcast((*text).to_owned())),
            ["styledMessage", uuid, json] => Some(Self::StyledMessage {
                uuid: (*uuid).to_owned(),
                json: (*json).to_owned(),
            }),
            ["styledBroadcast", hidden, json] => Some(Self::StyledBroadcast {
                hidden: hidden
                    .split(',')
                    .filter(|uuid| !uuid.is_empty())
                    .map(str::to_owned)
                    .collect(),
                json: (*json).to_owned(),
            }),
            ["teleport", id, uuid, dimension, x, y, z, yaw, pitch, safe] => Some(Self::Teleport {
                id: id.parse().ok()?,
                uuid: (*uuid).to_owned(),
                location: Location {
                    dimension: (*dimension).to_owned(),
                    x: x.parse().ok()?,
                    y: y.parse().ok()?,
                    z: z.parse().ok()?,
                    yaw: yaw.parse().ok()?,
                    pitch: pitch.parse().ok()?,
                },
                safe: if safe.is_empty() {
                    None
                } else {
                    Some(safe.parse().ok()?)
                },
            }),
            ["listName", uuid, name] => Some(Self::ListName {
                uuid: (*uuid).to_owned(),
                name: (!name.is_empty()).then(|| (*name).to_owned()),
            }),
            _ => None,
        }
    }
}

/// Answers a coordinate in the wire form: the shortest decimal that reads back as the same
/// number, which Display writes, or an infinity as Java spells it, since Display writes inf. NaN
/// is spelt alike in both.
fn number(value: f64) -> String {
    if value.is_infinite() {
        infinity(value.is_sign_negative())
    } else {
        value.to_string()
    }
}

/// Answers an angle in the wire form, as [`number`] answers a coordinate.
fn angle(value: f32) -> String {
    if value.is_infinite() {
        infinity(value.is_sign_negative())
    } else {
        value.to_string()
    }
}

/// Answers an infinity as `Double.toString` of Java writes it.
fn infinity(negative: bool) -> String {
    if negative { "-Infinity" } else { "Infinity" }.to_owned()
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
    queue: Arc<Mutex<Queue>>,
}

/// What the lock of the queue holds: the actions that wait, and whether the server stops.
#[derive(Debug, Default)]
struct Queue {
    actions: VecDeque<Action>,
    /// True from the stop of the server, which `serverStopping()` tells, or `serverStopped()`
    /// when the game did not announce the stop, to the next `serverStarting()`, while no tick
    /// drains the queue, so a teleport queued then would never report its end.
    stopped: bool,
}

impl Actions {
    /// An empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues one action behind the others, as it is. A teleport enters the queue through
    /// [`Actions::teleport`] alone, the one call that gives it its id and refuses it while the
    /// server stops. This call checks neither: it takes an [`Action::Teleport`] with whatever id
    /// it carries, also while the server stops, and the rule of one report for every id does
    /// not hold for such a teleport.
    pub fn push(&self, action: Action) {
        self.lock().actions.push_back(action);
    }

    /// Queues a teleport of the player with `uuid` behind the other actions and answers its id,
    /// or `None` from the stop of the server to the next `serverStarting()`, while no tick would
    /// run it.
    ///
    /// The id and the place in the queue are taken under one lock with the stop, so every id
    /// that this answers reports its end, on whatever thread the call runs and on every way that
    /// the game ends its server while the JVM still runs the mod. A JVM that is halted or killed
    /// runs nothing, so no report comes then, and no teleport outlives the process.
    pub fn teleport(&self, uuid: &str, location: &Location, safe: Option<u32>) -> Option<u64> {
        let mut queue = self.lock();
        if queue.stopped {
            return None;
        }
        let id = teleport_id();
        queue.actions.push_back(Action::Teleport {
            id,
            uuid: uuid.to_owned(),
            location: location.clone(),
            safe,
        });
        Some(id)
    }

    /// Takes every queued action, oldest first.
    pub fn drain(&self) -> Vec<Action> {
        self.lock().actions.drain(..).collect()
    }

    /// Puts drained actions back in front of the queue, in their order, for a drain whose
    /// answer never reached the mod.
    pub(crate) fn requeue(&self, actions: Vec<Action>) {
        let mut queue = self.lock();
        for action in actions.into_iter().rev() {
            queue.actions.push_front(action);
        }
    }

    /// Takes the teleports out of the queue as the server stops, oldest first, each as its id
    /// and its UUID, and leaves the other actions as they wait. [`Actions::teleport`] queues
    /// none from here to [`Actions::start`].
    pub(crate) fn stop(&self) -> Vec<(u64, String)> {
        let mut queue = self.lock();
        queue.stopped = true;
        let mut teleports = Vec::new();
        queue.actions.retain(|action| match action {
            Action::Teleport { id, uuid, .. } => {
                teleports.push((*id, uuid.clone()));
                false
            }
            _ => true,
        });
        teleports
    }

    /// Lets the queue take teleports again, as a server starts.
    pub(crate) fn start(&self) {
        self.lock().stopped = false;
    }

    /// Answers how many actions wait.
    pub fn len(&self) -> usize {
        self.lock().actions.len()
    }

    /// Answers whether no action waits.
    pub fn is_empty(&self) -> bool {
        self.lock().actions.is_empty()
    }

    /// Answers the queue, also when a thread panicked while it held the lock.
    fn lock(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
