// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The records of the store as the plugins see them, the rules every value must meet before the
 * store takes it, and the errors and refusals of a call.
 *
 * The rules here are those of the store itself: a name is never empty and holds no control
 * character, a location is finite, an amount of money is whole cents. What a plugin allows on top,
 * such as the number of homes of a group or the characters of a nickname, is the business of the
 * plugin.
 */

use std::{
    fmt,
    path::PathBuf, //
};

/// The longest name the store takes, in characters: a home, a warp, a group, a setting key, the
/// name or the nickname of a player.
pub const MAX_NAME_CHARS: usize = 64;

/// The longest text the store takes, in bytes: a mail, the note of a transfer, the value of a
/// setting, a dimension or a permission node.
pub const MAX_TEXT_BYTES: usize = 65536;

/// The longest permission node or group of the permissions, and the longest dimension, in bytes.
pub const MAX_NODE_BYTES: usize = 256;

/// The UUID of a player, kept as its 128 bits and written in lower case with its four dashes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Uuid(pub u128);

impl Uuid {
    /// Reads a UUID of 32 hexadecimal digits in either case with dashes after the 8th, 12th, 16th
    /// and 20th, the form Minecraft writes, or answers `None`.
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() != 36 {
            return None;
        }
        let mut value = 0_u128;
        for (index, &byte) in bytes.iter().enumerate() {
            if matches!(index, 8 | 13 | 18 | 23) {
                if byte != b'-' {
                    return None;
                }
                continue;
            }
            let digit = char::from(byte).to_digit(16)?;
            value = (value << 4) | u128::from(digit);
        }
        Some(Self(value))
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex = format!("{:032x}", self.0);
        write!(
            formatter,
            "{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32]
        )
    }
}

/// A place in a world: the dimension, such as `minecraft:overworld`, the position and where the
/// player looks.
#[derive(Clone, Debug, PartialEq)]
pub struct Location {
    /// The dimension, such as `minecraft:the_nether`.
    pub dimension: String,
    /// The position east of the origin.
    pub x: f64,
    /// The height.
    pub y: f64,
    /// The position south of the origin.
    pub z: f64,
    /// The rotation around the vertical axis, in degrees.
    pub yaw: f32,
    /// The rotation up and down, in degrees.
    pub pitch: f32,
}

/// What the store knows of a player.
///
/// Every time is in milliseconds since 1970-01-01 UTC, as the caller gives it, so the migration
/// tool can carry the times of `EssentialsX` over and a notebook can name its own clock.
#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    /// The UUID, which never changes.
    pub uuid: Uuid,
    /// The name the player had when they joined last.
    pub name: String,
    /// When the player joined the first time.
    pub first_join: i64,
    /// When the player joined the last time.
    pub last_join: i64,
    /// When the player was last seen: their last join, their last quit or a later moment.
    pub last_seen: i64,
    /// Where the player stood when they quit last.
    pub logout: Option<Location>,
    /// The place `/back` returns the player to.
    pub back: Option<Location>,
    /// Whether the player is away from the keyboard.
    pub afk: bool,
    /// The nickname, if the player has one.
    pub nickname: Option<String>,
}

/// A warp: a named place, with the permission node a player needs to use it, if any.
#[derive(Clone, Debug, PartialEq)]
pub struct Warp {
    /// The name.
    pub name: String,
    /// The place.
    pub location: Location,
    /// The node a player needs for the warp, or `None` when every player who may warp may use it.
    pub permission: Option<String>,
}

/// A mail of one player or of the console to a player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mail {
    /// The number of the mail, from 1, unique in the store.
    pub id: i64,
    /// The player who sent the mail, or `None` for the console.
    pub sender: Option<Uuid>,
    /// The name the sender had when they sent it, `Console` for the console.
    pub sender_name: String,
    /// The player the mail is for.
    pub receiver: Uuid,
    /// The text.
    pub text: String,
    /// When it was sent, in milliseconds since 1970-01-01 UTC.
    pub sent_at: i64,
    /// Whether the receiver has read it.
    pub read: bool,
}

/// One entry of the log of the accounts: an amount that left one account and reached another.
///
/// A deposit has no source, which is money the server creates, and a withdrawal has no target,
/// which is money the server destroys. Every change of a balance is such an entry, so the balance
/// of an account is always the sum of what reached it less the sum of what left it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transfer {
    /// The number of the entry, from 1, in the order the store took them.
    pub id: i64,
    /// When it happened, in milliseconds since 1970-01-01 UTC.
    pub at: i64,
    /// The account the money left, `None` for a deposit.
    pub source: Option<Uuid>,
    /// The account the money reached, `None` for a withdrawal.
    pub target: Option<Uuid>,
    /// The amount in cents, always above zero.
    pub amount: i64,
    /// Why, such as `pay` or `eco give`.
    pub note: String,
}

/// What a change of the accounts answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    /// The entry of the log, or `None` when a new balance equals the old one and nothing moved.
    pub transfer: Option<i64>,
    /// The balance of the source after the change, if there is a source.
    pub source_balance: Option<i64>,
    /// The balance of the target after the change, if there is a target.
    pub target_balance: Option<i64>,
}

/// A group of the permissions, as `golem-permissions` reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionGroup {
    /// The name, such as `default`.
    pub name: String,
    /// The weight, a higher weight first.
    pub weight: i32,
    /// The groups whose nodes the group takes on, in the order given.
    pub parents: Vec<String>,
    /// The nodes, a deny with its leading `-`, in the order given.
    pub nodes: Vec<String>,
}

/// The groups and nodes of one player of the permissions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerPermissions {
    /// The player.
    pub uuid: Uuid,
    /// The groups of the player besides the group `default`, in the order given.
    pub groups: Vec<String>,
    /// The nodes of the player, in the order given.
    pub nodes: Vec<String>,
}

/// A failure of an open, a read or a write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreError {
    /// The call breaks a rule of the store, and nothing changed.
    Refused(Refusal),
    /// The writer failed to write an earlier change, so the store takes no more changes. The
    /// file holds every change before the one that failed.
    Failed(String),
    /// The store is closed.
    Closed,
    /// The file is no store this Golem can open, or another process holds it.
    Open {
        /// The file.
        path: PathBuf,
        /// Why.
        reason: String,
    },
    /// `SQLite` answered an error on a read or a check.
    Database(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(refusal) => write!(formatter, "{refusal}"),
            Self::Failed(reason) => write!(
                formatter,
                "the store takes no more changes, since the writer failed: {reason}"
            ),
            Self::Closed => formatter.write_str("the store is closed"),
            Self::Open { path, reason } => {
                write!(
                    formatter,
                    "cannot open the store {}: {reason}",
                    path.display()
                )
            }
            Self::Database(reason) => write!(formatter, "SQLite failed: {reason}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<Refusal> for StoreError {
    fn from(refusal: Refusal) -> Self {
        Self::Refused(refusal)
    }
}

/// Why the store refuses a call. A refused call changes nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The player has never joined and was never put into the store.
    UnknownPlayer(Uuid),
    /// The player has no account.
    UnknownAccount(Uuid),
    /// The player has an account already.
    AccountExists(Uuid),
    /// An amount to move is zero or below.
    NotAboveZero(i64),
    /// A balance to set is below zero.
    Negative(i64),
    /// The source holds less than the amount.
    Overdraw {
        /// The account.
        account: Uuid,
        /// What it holds, in cents.
        balance: i64,
        /// What should leave it, in cents.
        amount: i64,
    },
    /// The target would hold more than the largest balance, `i64::MAX` cents.
    Overflow {
        /// The account.
        account: Uuid,
        /// What it holds, in cents.
        balance: i64,
        /// What should reach it, in cents.
        amount: i64,
    },
    /// A transfer names the same account as source and target.
    SameAccount(Uuid),
    /// A player would ignore themselves.
    IgnoreSelf(Uuid),
    /// A name, a text or a location breaks a rule of the store.
    Invalid {
        /// What it is, such as `the name of a home`.
        what: &'static str,
        /// Why it is refused.
        why: String,
    },
    /// A group of the permissions is not in the store or in the set.
    UnknownGroup(String),
    /// A group of the permissions that another group or a player names cannot be removed.
    GroupInUse {
        /// The group.
        group: String,
        /// Who names it, such as `the group admin`.
        by: String,
    },
    /// A list names an entry twice.
    Twice {
        /// What the list is, such as `the nodes of the group default`.
        what: String,
        /// The entry.
        entry: String,
    },
}

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPlayer(uuid) => write!(formatter, "the player {uuid} is not in the store"),
            Self::UnknownAccount(uuid) => write!(formatter, "the player {uuid} has no account"),
            Self::AccountExists(uuid) => {
                write!(formatter, "the player {uuid} has an account already")
            }
            Self::NotAboveZero(amount) => {
                write!(formatter, "the amount of {amount} cents is not above zero")
            }
            Self::Negative(balance) => {
                write!(formatter, "the balance of {balance} cents is below zero")
            }
            Self::Overdraw {
                account,
                balance,
                amount,
            } => write!(
                formatter,
                "the account of {account} holds {balance} cents, less than the {amount} cents to take"
            ),
            Self::Overflow {
                account,
                balance,
                amount,
            } => write!(
                formatter,
                "the account of {account} holds {balance} cents, and {amount} more would pass the largest balance of {} cents",
                i64::MAX
            ),
            Self::SameAccount(uuid) => write!(
                formatter,
                "a transfer from the account of {uuid} to itself moves nothing"
            ),
            Self::IgnoreSelf(uuid) => {
                write!(formatter, "the player {uuid} cannot ignore themselves")
            }
            Self::Invalid { what, why } => write!(formatter, "{what} {why}"),
            Self::UnknownGroup(group) => {
                write!(formatter, "the group {group:?} is not in the permissions")
            }
            Self::GroupInUse { group, by } => write!(
                formatter,
                "the group {group:?} cannot be removed, since {by} names it"
            ),
            Self::Twice { what, entry } => write!(formatter, "{what} name {entry:?} twice"),
        }
    }
}

/// Checks a name of the store: 1 to [`MAX_NAME_CHARS`] characters, none a control character.
pub(crate) fn check_name(what: &'static str, name: &str) -> Result<(), Refusal> {
    let invalid = |why: String| Err(Refusal::Invalid { what, why });
    if name.is_empty() {
        return invalid("is empty".to_owned());
    }
    let count = name.chars().count();
    if count > MAX_NAME_CHARS {
        return invalid(format!(
            "has {count} characters, more than {MAX_NAME_CHARS}"
        ));
    }
    if let Some(control) = name.chars().find(|character| character.is_control()) {
        return invalid(format!("holds the control character {control:?}"));
    }
    Ok(())
}

/// Checks a text of the store: at most [`MAX_TEXT_BYTES`] bytes and no NUL, which `SQLite` would
/// cut the text at in some of its functions.
pub(crate) fn check_text(what: &'static str, text: &str) -> Result<(), Refusal> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(Refusal::Invalid {
            what,
            why: format!("has {} bytes, more than {MAX_TEXT_BYTES}", text.len()),
        });
    }
    if text.contains('\0') {
        return Err(Refusal::Invalid {
            what,
            why: "holds a NUL character".to_owned(),
        });
    }
    Ok(())
}

/// Checks a node or a group of the permissions, or a dimension: 1 to [`MAX_NODE_BYTES`] bytes,
/// no white space and no control character. `golem-permissions` judges the rest of the syntax
/// when it loads the set.
pub(crate) fn check_token(what: &'static str, token: &str) -> Result<(), Refusal> {
    match token_problem(token) {
        None => Ok(()),
        Some(why) if token.is_empty() => Err(Refusal::Invalid { what, why }),
        Some(why) => Err(Refusal::Invalid {
            what,
            why: format!("{token:?} {why}"),
        }),
    }
}

/// What is wrong with a token of [`check_token`], if anything.
fn token_problem(token: &str) -> Option<String> {
    if token.is_empty() {
        return Some("is empty".to_owned());
    }
    if token.len() > MAX_NODE_BYTES {
        return Some(format!(
            "has {} bytes, more than {MAX_NODE_BYTES}",
            token.len()
        ));
    }
    token
        .chars()
        .find(|character| character.is_whitespace() || character.is_control())
        .map(|bad| format!("holds the character {bad:?}"))
}

/// Checks a location: a dimension of [`check_token`] and finite numbers.
pub(crate) fn check_location(what: &'static str, location: &Location) -> Result<(), Refusal> {
    if let Some(problem) = token_problem(&location.dimension) {
        return Err(Refusal::Invalid {
            what,
            why: format!(
                "names the dimension {:?}, which {problem}",
                location.dimension
            ),
        });
    }
    let numbers = [
        ("x", location.x),
        ("y", location.y),
        ("z", location.z),
        ("yaw", f64::from(location.yaw)),
        ("pitch", f64::from(location.pitch)),
    ];
    if let Some((axis, value)) = numbers.iter().find(|(_, value)| !value.is_finite()) {
        return Err(Refusal::Invalid {
            what,
            why: format!("has the {axis} {value}, which is not a finite number"),
        });
    }
    Ok(())
}
