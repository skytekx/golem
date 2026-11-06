// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The subcommands of golem-store: store runs a script of calls on a store and prints the answer
 * of every step as it goes, store-describe prints the schema of a file, and store-bench times
 * the calls of the server thread on a filled store.
 */

//! The primitives of golem-store.
//!
//! `store <script.json>` runs the steps of a script in order and prints one line of JSON per
//! step as soon as it ran, `{"step", "op", "ok": <answer>}` or `{"step", "op", "error": {"kind",
//! "message"}}`, so a notebook that kills the process sees every step before the kill. The kind
//! of an error is the variant of `StoreError`, `refused`, `failed`, `closed`, `open` or
//! `database`, or `argument` for a step the devtool cannot read.
//!
//! ```text
//! {"steps": [
//!   {"op": "open", "path": "store.db"},
//!   {"op": "join", "uuid": "...", "name": "Alex", "at": 1000},
//!   {"op": "transfer", "source": "...", "target": "...", "amount": 250, "at": 2000, "note": "pay"},
//!   {"op": "close"}
//! ]}
//! ```
//!
//! A step is a call of `Store` by its name with its arguments by name, a location being
//! `{"dimension", "x", "y", "z", "yaw", "pitch"}`, where a number may also be a string such as
//! `"NaN"`. The devtool adds `reopen`, which closes the store and opens the same file, `repeat`,
//! which runs its `steps` `count` times with `{i}` in a string replaced by the round from 1 and
//! prints one line for all rounds, `sleep` with `ms`, and `wait`, which prints its line and then
//! waits until its standard input ends, for a notebook that kills the process there.
//!
//! When the standard output is gone, such as a pipe whose reader closed, a subcommand stops at
//! once, a script closes its store first, and the exit code is 1 with the reason on the standard
//! error.

use std::{
    collections::BTreeMap,
    fs,
    hint::black_box,
    io::{
        Read as _,
        Write as _, //
    },
    path::PathBuf,
    time::{
        Duration,
        Instant, //
    },
};

use golem_store::{
    Check,
    Description,
    Location,
    Mail,
    PermissionGroup,
    PermissionSet,
    Player,
    PlayerPermissions,
    Receipt,
    Stats,
    Store,
    StoreError,
    Transfer,
    Uuid,
    Warp, //
};
use serde_json::{
    Map,
    Value,
    json, //
};

use crate::Subcommand;

pub const SCRIPT: Subcommand = (
    "store",
    "<script.json> the answers of a store to a script of calls",
    script,
);
pub const DESCRIBE: Subcommand = (
    "store-describe",
    "<file> the schema of a store file",
    describe,
);
pub const BENCH: Subcommand = (
    "store-bench",
    "--path <file> [--players <n>] [--repeat <n>] the cost of the calls of the server thread",
    bench,
);

/// Reports a failure on the standard error and answers the exit code. A standard error that is
/// gone loses the line, but never panics.
fn fail(name: &str, message: &str) -> i32 {
    let _ = writeln!(std::io::stderr().lock(), "golem-devtool: {name}: {message}");
    1
}

/// Writes one line of JSON on the standard output, or answers why it could not, such as a pipe
/// whose reader is gone. Rust ignores `SIGPIPE`, so a closed pipe is an error here and not a
/// signal, and `println!` would panic on it.
fn emit(line: &Value) -> std::io::Result<()> {
    let mut output = std::io::stdout().lock();
    writeln!(output, "{line}")?;
    output.flush()
}

/// The failure of a closed standard output, after which a subcommand stops.
fn output_closed(name: &str, error: &std::io::Error) -> i32 {
    fail(name, &format!("the standard output is closed: {error}"))
}

/// A failure of one step: the kind and the message.
struct StepError {
    kind: &'static str,
    message: String,
}

impl From<StoreError> for StepError {
    fn from(error: StoreError) -> Self {
        let kind = match &error {
            StoreError::Refused(_) => "refused",
            StoreError::Failed(_) => "failed",
            StoreError::Closed => "closed",
            StoreError::Open { .. } => "open",
            StoreError::Database(_) => "database",
        };
        Self {
            kind,
            message: error.to_string(),
        }
    }
}

fn argument(message: String) -> StepError {
    StepError {
        kind: "argument",
        message,
    }
}

type Answer = Result<Value, StepError>;

/// The arguments of one step.
struct Arguments<'a>(&'a Map<String, Value>);

impl Arguments<'_> {
    fn value(&self, key: &str) -> Result<&Value, StepError> {
        self.0
            .get(key)
            .ok_or_else(|| argument(format!("the step needs {key:?}")))
    }

    fn text(&self, key: &str) -> Result<&str, StepError> {
        self.value(key)?
            .as_str()
            .ok_or_else(|| argument(format!("{key:?} is not a string")))
    }

    fn optional_text(&self, key: &str) -> Result<Option<&str>, StepError> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(_) => self.text(key).map(Some),
        }
    }

    fn integer(&self, key: &str) -> Result<i64, StepError> {
        self.value(key)?
            .as_i64()
            .ok_or_else(|| argument(format!("{key:?} is not an integer")))
    }

    /// An integer that may be missing or null, which answers `None`. Any other value that is no
    /// integer is refused, so a typo never stands in for a default.
    fn optional_integer(&self, key: &str) -> Result<Option<i64>, StepError> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(_) => self.integer(key).map(Some),
        }
    }

    fn count(&self, key: &str) -> Result<usize, StepError> {
        let value = self.integer(key)?;
        usize::try_from(value).map_err(|_| argument(format!("{key:?} is below zero")))
    }

    fn flag(&self, key: &str) -> Result<bool, StepError> {
        self.value(key)?
            .as_bool()
            .ok_or_else(|| argument(format!("{key:?} is not true or false")))
    }

    fn uuid(&self, key: &str) -> Result<Uuid, StepError> {
        let text = self.text(key)?;
        Uuid::parse(text).ok_or_else(|| argument(format!("{key:?} {text:?} is not a UUID")))
    }

    fn optional_uuid(&self, key: &str) -> Result<Option<Uuid>, StepError> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(_) => self.uuid(key).map(Some),
        }
    }

    fn strings(&self, key: &str) -> Result<Vec<String>, StepError> {
        match self.0.get(key) {
            None => Ok(Vec::new()),
            Some(Value::Array(items)) => items
                .iter()
                .map(|item| {
                    item.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| argument(format!("{key:?} holds a value that is no string")))
                })
                .collect(),
            Some(_) => Err(argument(format!("{key:?} is not a list"))),
        }
    }

    fn location(&self, key: &str) -> Result<Location, StepError> {
        read_location(self.value(key)?).map_err(|why| argument(format!("{key:?} {why}")))
    }

    fn optional_location(&self, key: &str) -> Result<Option<Location>, StepError> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(_) => self.location(key).map(Some),
        }
    }
}

/// A number of a location, which may be a string such as `NaN` or `inf`.
fn number(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::String(text) => text.parse().ok(),
        other => other.as_f64(),
    }
}

fn read_location(value: &Value) -> Result<Location, String> {
    let object = value.as_object().ok_or("is not a location")?;
    let dimension = object
        .get("dimension")
        .and_then(Value::as_str)
        .ok_or("has no dimension")?;
    let axis = |name: &str| number(object.get(name)).ok_or(format!("has no number {name}"));
    #[allow(clippy::cast_possible_truncation)]
    let angle = |name: &str| axis(name).map(|value| value as f32);
    Ok(Location {
        dimension: dimension.to_owned(),
        x: axis("x")?,
        y: axis("y")?,
        z: axis("z")?,
        yaw: angle("yaw")?,
        pitch: angle("pitch")?,
    })
}

fn location_json(location: &Location) -> Value {
    json!({
        "dimension": location.dimension,
        "x": location.x,
        "y": location.y,
        "z": location.z,
        "yaw": location.yaw,
        "pitch": location.pitch,
    })
}

fn player_json(player: &Player) -> Value {
    json!({
        "uuid": player.uuid.to_string(),
        "name": player.name,
        "first_join": player.first_join,
        "last_join": player.last_join,
        "last_seen": player.last_seen,
        "logout": player.logout.as_ref().map(location_json),
        "back": player.back.as_ref().map(location_json),
        "afk": player.afk,
        "nickname": player.nickname,
    })
}

fn warp_json(warp: &Warp) -> Value {
    json!({
        "name": warp.name,
        "location": location_json(&warp.location),
        "permission": warp.permission,
    })
}

fn mail_json(mail: &Mail) -> Value {
    json!({
        "id": mail.id,
        "sender": mail.sender.map(|sender| sender.to_string()),
        "sender_name": mail.sender_name,
        "receiver": mail.receiver.to_string(),
        "text": mail.text,
        "sent_at": mail.sent_at,
        "read": mail.read,
    })
}

fn transfer_json(transfer: &Transfer) -> Value {
    json!({
        "id": transfer.id,
        "at": transfer.at,
        "source": transfer.source.map(|source| source.to_string()),
        "target": transfer.target.map(|target| target.to_string()),
        "amount": transfer.amount,
        "note": transfer.note,
    })
}

fn receipt_json(receipt: Receipt) -> Value {
    json!({
        "transfer": receipt.transfer,
        "source_balance": receipt.source_balance,
        "target_balance": receipt.target_balance,
    })
}

fn group_json(group: &PermissionGroup) -> Value {
    json!({
        "name": group.name,
        "weight": group.weight,
        "parents": group.parents,
        "nodes": group.nodes,
    })
}

fn player_permissions_json(player: &PlayerPermissions) -> Value {
    json!({
        "uuid": player.uuid.to_string(),
        "groups": player.groups,
        "nodes": player.nodes,
    })
}

fn stats_json(stats: &Stats) -> Value {
    json!({
        "queued": stats.queued,
        "written": stats.written,
        "lost": stats.lost,
        "pending": stats.pending,
        "batches": stats.batches,
        "largest_batch": stats.largest_batch,
        "busy_nanos": stats.busy_nanos,
        "failure": stats.failure,
        "closed": stats.closed,
    })
}

fn check_json(check: &Check) -> Value {
    let mismatches: Vec<Value> = check
        .ledger_mismatches
        .iter()
        .map(|(uuid, balance, sum)| json!({"account": uuid.to_string(), "balance": balance, "log": sum}))
        .collect();
    json!({
        "passed": check.passed(),
        "integrity": check.integrity,
        "foreign_key_violations": check.foreign_key_violations,
        "ledger_mismatches": mismatches,
        "cache_differences": check.cache_differences,
        "rows": check.rows,
    })
}

fn description_json(description: &Description) -> Value {
    let tables: Vec<Value> = description
        .tables
        .iter()
        .map(|table| {
            let columns: Vec<Value> = table
                .columns
                .iter()
                .map(|column| {
                    json!({
                        "name": column.name,
                        "type": column.kind,
                        "not_null": column.not_null,
                        "primary_key": column.primary_key,
                    })
                })
                .collect();
            let keys: Vec<Value> = table
                .foreign_keys
                .iter()
                .map(|(from, table, to)| json!({"from": from, "table": table, "to": to}))
                .collect();
            json!({
                "name": table.name,
                "strict": table.strict,
                "rows": table.rows,
                "columns": columns,
                "foreign_keys": keys,
            })
        })
        .collect();
    json!({
        "application_id": description.application_id,
        "version": description.version,
        "journal_mode": description.journal_mode,
        "tables": tables,
        "indexes": description.indexes,
    })
}

/// What a script works on: the store and the file it was opened from.
#[derive(Default)]
struct Session {
    store: Option<Store>,
    path: Option<PathBuf>,
}

impl Session {
    fn store(&self) -> Result<&Store, StepError> {
        self.store
            .as_ref()
            .ok_or_else(|| argument("no store is open, the script needs an open step".to_owned()))
    }

    fn open(&mut self, path: PathBuf) -> Answer {
        if let Some(store) = self.store.take() {
            store.close()?;
        }
        let store = Store::open(&path)?;
        let answer = json!({
            "players": store.player_count(),
            "accounts": store.account_count(),
            "warps": store.warps().len(),
            "settings": store.settings().len(),
        });
        self.store = Some(store);
        self.path = Some(path);
        Ok(answer)
    }

    /// Closes the store and keeps it, so a later step meets a closed store.
    fn close(&mut self) -> Answer {
        let store = self.store()?;
        store.close()?;
        Ok(stats_json(&store.stats()))
    }
}

/// Replaces `{i}` in every string of a step with the round.
fn with_round(step: &Value, round: usize) -> Value {
    match step {
        Value::String(text) => Value::String(text.replace("{i}", &round.to_string())),
        Value::Array(items) => {
            Value::Array(items.iter().map(|item| with_round(item, round)).collect())
        }
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), with_round(value, round)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Runs one step and answers what it printed.
fn run_step(session: &mut Session, step: &Value) -> Answer {
    let fields = step
        .as_object()
        .ok_or_else(|| argument("a step is not an object".to_owned()))?;
    let op = fields
        .get("op")
        .and_then(Value::as_str)
        .ok_or_else(|| argument("a step has no op".to_owned()))?;
    let arguments = Arguments(fields);
    match op {
        "open" => session.open(PathBuf::from(arguments.text("path")?)),
        "close" => session.close(),
        "reopen" => {
            let path = session
                .path
                .clone()
                .ok_or_else(|| argument("no store was opened before".to_owned()))?;
            session.open(path)
        }
        "repeat" => repeat(session, &arguments),
        "sleep" => {
            let millis = u64::try_from(arguments.integer("ms")?).unwrap_or(0);
            std::thread::sleep(Duration::from_millis(millis));
            Ok(Value::Null)
        }
        "describe" => Ok(description_json(&golem_store::describe(
            arguments.text("path")?.as_ref(),
        )?)),
        _ => {
            let store = session.store()?;
            lifecycle(store, op)
                .or_else(|| players(store, op, &arguments))
                .or_else(|| places(store, op, &arguments))
                .or_else(|| messages(store, op, &arguments))
                .or_else(|| accounts(store, op, &arguments))
                .or_else(|| permissions(store, op, &arguments))
                .or_else(|| settings(store, op, &arguments))
                .unwrap_or_else(|| Err(argument(format!("there is no op {op:?}"))))
        }
    }
}

/// Runs the steps of a repeat `count` times and answers the count, the refusals and the time of
/// a call.
fn repeat(session: &mut Session, arguments: &Arguments<'_>) -> Answer {
    let count = arguments.count("count")?;
    let steps = arguments
        .value("steps")?
        .as_array()
        .ok_or_else(|| argument("\"steps\" is not a list".to_owned()))?
        .clone();
    let mut refused = 0_u64;
    let mut calls = 0_u64;
    let started = Instant::now();
    for round in 1..=count {
        for step in &steps {
            calls += 1;
            match run_step(session, &with_round(step, round)) {
                Ok(_) => {}
                Err(error) if error.kind == "refused" => refused += 1,
                Err(error) => {
                    return Err(StepError {
                        kind: error.kind,
                        message: format!("round {round}: {}", error.message),
                    });
                }
            }
        }
    }
    let nanos = started.elapsed().as_nanos() / u128::from(calls.max(1));
    Ok(json!({"rounds": count, "calls": calls, "refused": refused, "nanos_per_call": nanos}))
}

fn lifecycle(store: &Store, op: &str) -> Option<Answer> {
    let answer = match op {
        "flush" => store.flush().map(|()| Value::Null),
        "stats" => Ok(stats_json(&store.stats())),
        "check" => store.check().map(|check| check_json(&check)),
        _ => return None,
    };
    Some(answer.map_err(StepError::from))
}

fn players(store: &Store, op: &str, arguments: &Arguments<'_>) -> Option<Answer> {
    let run = || -> Answer {
        Ok(match op {
            "join" => {
                let joined = store.join(
                    arguments.uuid("uuid")?,
                    arguments.text("name")?,
                    arguments.integer("at")?,
                )?;
                json!({"first": joined.first, "previous_name": joined.previous_name})
            }
            "quit" => {
                store.quit(
                    arguments.uuid("uuid")?,
                    arguments.location("location")?,
                    arguments.integer("at")?,
                )?;
                Value::Null
            }
            "set_back" => {
                store.set_back(
                    arguments.uuid("uuid")?,
                    arguments.optional_location("location")?,
                )?;
                Value::Null
            }
            "set_afk" => {
                store.set_afk(arguments.uuid("uuid")?, arguments.flag("afk")?)?;
                Value::Null
            }
            "set_nickname" => {
                store.set_nickname(
                    arguments.uuid("uuid")?,
                    arguments.optional_text("nickname")?,
                )?;
                Value::Null
            }
            "set_last_seen" => {
                store.set_last_seen(arguments.uuid("uuid")?, arguments.integer("at")?)?;
                Value::Null
            }
            "put_player" => {
                store.put_player(Player {
                    uuid: arguments.uuid("uuid")?,
                    name: arguments.text("name")?.to_owned(),
                    first_join: arguments.integer("first_join")?,
                    last_join: arguments.integer("last_join")?,
                    last_seen: arguments.integer("last_seen")?,
                    logout: arguments.optional_location("logout")?,
                    back: arguments.optional_location("back")?,
                    afk: arguments.flag("afk")?,
                    nickname: arguments.optional_text("nickname")?.map(str::to_owned),
                })?;
                Value::Null
            }
            "player" => store
                .player(arguments.uuid("uuid")?)
                .map_or(Value::Null, |player| player_json(&player)),
            "player_named" => store
                .player_named(arguments.text("name")?)
                .map_or(Value::Null, |player| player_json(&player)),
            "players_nicknamed" => Value::Array(
                store
                    .players_nicknamed(arguments.text("nickname")?)
                    .iter()
                    .map(player_json)
                    .collect(),
            ),
            "player_count" => json!(store.player_count()),
            _ => return Err(argument(String::new())),
        })
    };
    known(run())
}

/// Turns the answer of a group of ops into `None` when the group does not know the op, which
/// it signals with an argument error without a message.
fn known(answer: Answer) -> Option<Answer> {
    match answer {
        Err(StepError {
            kind: "argument",
            message,
        }) if message.is_empty() => None,
        other => Some(other),
    }
}

fn places(store: &Store, op: &str, arguments: &Arguments<'_>) -> Option<Answer> {
    let run = || -> Answer {
        Ok(match op {
            "set_home" => json!(store.set_home(
                arguments.uuid("uuid")?,
                arguments.text("name")?,
                arguments.location("location")?,
            )?),
            "remove_home" => {
                json!(store.remove_home(arguments.uuid("uuid")?, arguments.text("name")?)?)
            }
            "home" => store
                .home(arguments.uuid("uuid")?, arguments.text("name")?)
                .map_or(Value::Null, |location| location_json(&location)),
            "homes" => {
                let homes: BTreeMap<String, Value> = store
                    .homes(arguments.uuid("uuid")?)
                    .iter()
                    .map(|(name, location)| (name.clone(), location_json(location)))
                    .collect();
                json!(homes)
            }
            "home_count" => json!(store.home_count(arguments.uuid("uuid")?)),
            "set_warp" => json!(store.set_warp(
                arguments.text("name")?,
                arguments.location("location")?,
                arguments.optional_text("permission")?,
            )?),
            "remove_warp" => json!(store.remove_warp(arguments.text("name")?)?),
            "warp" => store
                .warp(arguments.text("name")?)
                .map_or(Value::Null, |warp| warp_json(&warp)),
            "warps" => Value::Array(store.warps().iter().map(warp_json).collect()),
            "set_spawn" => {
                json!(store.set_spawn(arguments.text("group")?, arguments.location("location")?)?)
            }
            "remove_spawn" => json!(store.remove_spawn(arguments.text("group")?)?),
            "spawn" => store
                .spawn(arguments.text("group")?)
                .map_or(Value::Null, |location| location_json(&location)),
            "spawns" => {
                let spawns: BTreeMap<String, Value> = store
                    .spawns()
                    .iter()
                    .map(|(group, location)| (group.clone(), location_json(location)))
                    .collect();
                json!(spawns)
            }
            _ => return Err(argument(String::new())),
        })
    };
    known(run())
}

fn messages(store: &Store, op: &str, arguments: &Arguments<'_>) -> Option<Answer> {
    let run = || -> Answer {
        Ok(match op {
            "send_mail" => json!(store.send_mail(
                arguments.optional_uuid("sender")?,
                arguments.text("sender_name")?,
                arguments.uuid("receiver")?,
                arguments.text("text")?,
                arguments.integer("at")?,
            )?),
            "mail" => Value::Array(
                store
                    .mail(arguments.uuid("receiver")?)
                    .iter()
                    .map(mail_json)
                    .collect(),
            ),
            "unread_mail" => json!(store.unread_mail(arguments.uuid("receiver")?)),
            "mark_mail_read" => json!(store.mark_mail_read(arguments.uuid("receiver")?)?),
            "remove_mail" => {
                json!(store.remove_mail(arguments.uuid("receiver")?, arguments.integer("id")?)?)
            }
            "clear_mail" => json!(store.clear_mail(arguments.uuid("receiver")?)?),
            "ignore" => json!(store.ignore(arguments.uuid("player")?, arguments.uuid("other")?)?),
            "unignore" => {
                json!(store.unignore(arguments.uuid("player")?, arguments.uuid("other")?)?)
            }
            "ignores" => json!(store.ignores(arguments.uuid("player")?, arguments.uuid("other")?)),
            "ignore_list" => {
                let list: Vec<String> = store
                    .ignore_list(arguments.uuid("player")?)
                    .iter()
                    .map(Uuid::to_string)
                    .collect();
                json!(list)
            }
            _ => return Err(argument(String::new())),
        })
    };
    known(run())
}

fn accounts(store: &Store, op: &str, arguments: &Arguments<'_>) -> Option<Answer> {
    let note = || {
        arguments
            .optional_text("note")
            .map(Option::unwrap_or_default)
    };
    let run = || -> Answer {
        Ok(match op {
            "create_account" => receipt_json(store.create_account(
                arguments.uuid("uuid")?,
                arguments.integer("balance")?,
                arguments.integer("at")?,
                note()?,
            )?),
            "deposit" => receipt_json(store.deposit(
                arguments.uuid("uuid")?,
                arguments.integer("amount")?,
                arguments.integer("at")?,
                note()?,
            )?),
            "withdraw" => receipt_json(store.withdraw(
                arguments.uuid("uuid")?,
                arguments.integer("amount")?,
                arguments.integer("at")?,
                note()?,
            )?),
            "transfer" => receipt_json(store.transfer(
                arguments.uuid("source")?,
                arguments.uuid("target")?,
                arguments.integer("amount")?,
                arguments.integer("at")?,
                note()?,
            )?),
            "set_balance" => receipt_json(store.set_balance(
                arguments.uuid("uuid")?,
                arguments.integer("balance")?,
                arguments.integer("at")?,
                note()?,
            )?),
            "balance" => json!(store.balance(arguments.uuid("uuid")?)),
            "top_balances" => {
                let top: Vec<Value> = store
                    .top_balances(arguments.count("limit")?)
                    .iter()
                    .map(|(uuid, balance)| json!([uuid.to_string(), balance]))
                    .collect();
                json!(top)
            }
            "account_count" => json!(store.account_count()),
            "ledger" => {
                let entries = store.ledger(
                    arguments.optional_uuid("account")?,
                    arguments.count("limit")?,
                )?;
                Value::Array(entries.iter().map(transfer_json).collect())
            }
            _ => return Err(argument(String::new())),
        })
    };
    known(run())
}

fn permissions(store: &Store, op: &str, arguments: &Arguments<'_>) -> Option<Answer> {
    let run = || -> Answer {
        Ok(match op {
            "replace_permissions" => {
                let text = match arguments.optional_text("toml_file")? {
                    Some(path) => fs::read_to_string(path)
                        .map_err(|error| argument(format!("cannot read {path}: {error}")))?,
                    None => arguments.text("toml")?.to_owned(),
                };
                let set = PermissionSet::from_toml(&text).map_err(argument)?;
                store.replace_permissions(set)?;
                Value::Null
            }
            "permissions" => {
                let set = store.permissions();
                json!({
                    "groups": set.groups.iter().map(group_json).collect::<Vec<_>>(),
                    "players": set.players.iter().map(player_permissions_json).collect::<Vec<_>>(),
                    "toml": set.to_toml(),
                    "generation": store.permissions_generation(),
                })
            }
            "permission_group" => store
                .permission_group(arguments.text("name")?)
                .map_or(Value::Null, |group| group_json(&group)),
            "player_permissions" => store
                .player_permissions(arguments.uuid("uuid")?)
                .map_or(Value::Null, |player| player_permissions_json(&player)),
            "put_permission_group" => {
                let weight = i32::try_from(arguments.optional_integer("weight")?.unwrap_or(0))
                    .map_err(|_| argument("\"weight\" does not fit 32 bits".to_owned()))?;
                json!(store.put_permission_group(PermissionGroup {
                    name: arguments.text("name")?.to_owned(),
                    weight,
                    parents: arguments.strings("parents")?,
                    nodes: arguments.strings("nodes")?,
                })?)
            }
            "remove_permission_group" => {
                json!(store.remove_permission_group(arguments.text("name")?)?)
            }
            "put_player_permissions" => {
                store.put_player_permissions(PlayerPermissions {
                    uuid: arguments.uuid("uuid")?,
                    groups: arguments.strings("groups")?,
                    nodes: arguments.strings("nodes")?,
                })?;
                Value::Null
            }
            "remove_player_permissions" => {
                json!(store.remove_player_permissions(arguments.uuid("uuid")?)?)
            }
            _ => return Err(argument(String::new())),
        })
    };
    known(run())
}

fn settings(store: &Store, op: &str, arguments: &Arguments<'_>) -> Option<Answer> {
    let run = || -> Answer {
        Ok(match op {
            "set_setting" => {
                store.set_setting(arguments.text("key")?, arguments.text("value")?)?;
                Value::Null
            }
            "remove_setting" => json!(store.remove_setting(arguments.text("key")?)?),
            "setting" => json!(store.setting(arguments.text("key")?)),
            "settings" => {
                let settings: BTreeMap<String, String> = store.settings().into_iter().collect();
                json!(settings)
            }
            _ => return Err(argument(String::new())),
        })
    };
    known(run())
}

/// Prints the answer of every step of a script as one line, as soon as the step ran.
fn script(arguments: &[String]) -> i32 {
    let name = SCRIPT.0;
    let [path] = arguments else {
        return fail(name, "give the script, a JSON file");
    };
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return fail(name, &format!("cannot read {path}: {error}")),
    };
    let script: Value = match serde_json::from_str(&text) {
        Ok(script) => script,
        Err(error) => return fail(name, &format!("{path}: {error}")),
    };
    let Some(steps) = script.get("steps").and_then(Value::as_array) else {
        return fail(name, &format!("{path}: the script has no list \"steps\""));
    };
    let mut session = Session::default();
    let mut printed = Ok(());
    for (index, step) in steps.iter().enumerate() {
        let op = step.get("op").and_then(Value::as_str).unwrap_or_default();
        if op == "wait" {
            printed = emit(&json!({"step": index, "op": op, "ok": "waiting"}));
            if printed.is_err() {
                break;
            }
            let mut rest = Vec::new();
            let _ = std::io::stdin().read_to_end(&mut rest);
            continue;
        }
        let line = match run_step(&mut session, step) {
            Ok(value) => json!({"step": index, "op": op, "ok": value}),
            Err(error) => json!({
                "step": index,
                "op": op,
                "error": {"kind": error.kind, "message": error.message},
            }),
        };
        printed = emit(&line);
        if printed.is_err() {
            break;
        }
    }
    // A store the script left open closes here, so its last changes reach the file, also when
    // the reader of the answers went away and the script stopped early.
    let closed = session.store.take().map_or(Ok(()), |store| store.close());
    if let Err(error) = printed {
        return output_closed(name, &error);
    }
    if let Err(error) = closed {
        return fail(name, &error.to_string());
    }
    0
}

/// Prints the schema of a store file.
fn describe(arguments: &[String]) -> i32 {
    let name = DESCRIBE.0;
    let [path] = arguments else {
        return fail(name, "give the store file");
    };
    match golem_store::describe(path.as_ref()) {
        Ok(description) => match emit(&description_json(&description)) {
            Ok(()) => 0,
            Err(error) => output_closed(name, &error),
        },
        Err(error) => fail(name, &error.to_string()),
    }
}

/// The options of store-bench.
struct BenchOptions {
    path: PathBuf,
    players: usize,
    repeat: usize,
}

fn bench_options(arguments: &[String]) -> Result<BenchOptions, String> {
    let mut options = BenchOptions {
        path: PathBuf::new(),
        players: 1000,
        repeat: 2000,
    };
    let mut rest = arguments.iter();
    while let Some(flag) = rest.next() {
        let value = rest.next().ok_or_else(|| format!("{flag} needs a value"))?;
        let count = || {
            value
                .parse::<usize>()
                .ok()
                .filter(|&count| count > 0)
                .ok_or_else(|| format!("{flag} takes a count above zero, not {value:?}"))
        };
        match flag.as_str() {
            "--path" => options.path = PathBuf::from(value),
            "--players" => options.players = count()?,
            "--repeat" => options.repeat = count()?,
            _ => return Err(format!("unknown argument {flag:?}")),
        }
    }
    if options.path.as_os_str().is_empty() {
        return Err("store-bench needs --path <file>".to_owned());
    }
    if options.path.exists() {
        return Err(format!(
            "{} exists, and store-bench fills a new store",
            options.path.display()
        ));
    }
    Ok(options)
}

/// The UUID of the player number `index` of the bench.
fn bench_uuid(index: usize) -> Uuid {
    Uuid(0x0be4_c400_0000_4000_8000_0000_0000_0000 | index as u128)
}

fn bench_location(index: usize) -> Location {
    #[allow(clippy::cast_precision_loss)]
    let offset = index as f64;
    Location {
        dimension: "minecraft:overworld".to_owned(),
        x: offset,
        y: 64.0,
        z: -offset,
        yaw: 90.0,
        pitch: 0.0,
    }
}

/// The times of the calls of one kind, in nanoseconds.
fn summary(mut nanos: Vec<u64>) -> Value {
    nanos.sort_unstable();
    let count = nanos.len().max(1);
    let total: u64 = nanos.iter().sum();
    let at = |share: usize| {
        nanos
            .get((count * share / 100).min(count - 1))
            .copied()
            .unwrap_or(0)
    };
    #[allow(clippy::cast_precision_loss)]
    let micros = |value: u64| value as f64 / 1000.0;
    json!({
        "calls": nanos.len(),
        "mean_us": micros(total / count as u64),
        "median_us": micros(at(50)),
        "p99_us": micros(at(99)),
        "max_us": micros(nanos.last().copied().unwrap_or(0)),
    })
}

/// Fills a store with `players` players as a server after years would hold them.
fn fill(store: &Store, players: usize) -> Result<(), StoreError> {
    let at = 1_700_000_000_000;
    for index in 0..players {
        let uuid = bench_uuid(index);
        store.join(uuid, &format!("player{index}"), at)?;
        for home in ["base", "farm", "mine"] {
            store.set_home(uuid, home, bench_location(index))?;
        }
        store.create_account(uuid, 100_000, at, "starting balance")?;
        if index > 0 {
            let sender = bench_uuid(index - 1);
            store.send_mail(Some(sender), "player", uuid, "hello there", at)?;
            store.send_mail(None, "Console", uuid, "welcome to the server", at)?;
            store.ignore(uuid, sender)?;
        }
        if index % 10 == 0 {
            store.set_nickname(uuid, Some(&format!("nick{index}")))?;
        }
    }
    for index in 0..50 {
        let permission = format!("essentials.warps.warp{index}");
        store.set_warp(
            &format!("warp{index}"),
            bench_location(index),
            Some(&permission),
        )?;
    }
    for group in ["default", "admin", "builder"] {
        store.set_spawn(group, bench_location(0))?;
    }
    for index in 0..20 {
        store.set_setting(&format!("setting{index}"), "value")?;
    }
    let mut groups = Vec::new();
    for index in 0..10 {
        groups.push(PermissionGroup {
            name: format!("group{index}"),
            weight: index,
            parents: if index == 0 {
                Vec::new()
            } else {
                vec![format!("group{}", index - 1)]
            },
            nodes: (0..20)
                .map(|node| format!("essentials.node{node}"))
                .collect(),
        });
    }
    groups.push(PermissionGroup {
        name: "default".to_owned(),
        weight: 0,
        parents: Vec::new(),
        nodes: vec!["essentials.home".to_owned()],
    });
    let members = (0..players.min(100))
        .map(|index| PlayerPermissions {
            uuid: bench_uuid(index),
            groups: vec!["group9".to_owned()],
            nodes: vec!["-essentials.suicide".to_owned()],
        })
        .collect();
    store.replace_permissions(PermissionSet {
        groups,
        players: members,
    })
}

/// What one call of the bench works on: the round, a player and another player.
struct Round {
    index: usize,
    number: usize,
    player: Uuid,
    other: Uuid,
}

/// The time of a call of the bench, a day after the fill.
const BENCH_AT: i64 = 1_700_086_400_000;

/// The calls of the server thread that the bench times, the reads first.
#[allow(clippy::type_complexity)]
const CALLS: &[(&str, fn(&Store, &Round))] = &[
    ("player", |store, round| {
        let _ = black_box(store.player(round.player));
    }),
    ("player_named", |store, round| {
        let _ = black_box(store.player_named(&format!("PLAYER{}", round.number)));
    }),
    ("home", |store, round| {
        let _ = black_box(store.home(round.player, "base"));
    }),
    ("homes", |store, round| {
        let _ = black_box(store.homes(round.player));
    }),
    ("warp", |store, round| {
        let _ = black_box(store.warp(&format!("warp{}", round.index % 50)));
    }),
    ("spawn", |store, _| {
        let _ = black_box(store.spawn("default"));
    }),
    ("balance", |store, round| {
        let _ = black_box(store.balance(round.player));
    }),
    ("top_balances_10", |store, _| {
        let _ = black_box(store.top_balances(10));
    }),
    ("unread_mail", |store, round| {
        let _ = black_box(store.unread_mail(round.player));
    }),
    ("mail", |store, round| {
        let _ = black_box(store.mail(round.player));
    }),
    ("ignores", |store, round| {
        let _ = black_box(store.ignores(round.player, round.other));
    }),
    ("setting", |store, _| {
        let _ = black_box(store.setting("setting7"));
    }),
    ("player_permissions", |store, round| {
        let _ = black_box(store.player_permissions(round.player));
    }),
    ("permissions", |store, _| {
        let _ = black_box(store.permissions());
    }),
    ("join", |store, round| {
        let name = format!("player{}", round.number);
        let _ = black_box(store.join(round.player, &name, BENCH_AT));
    }),
    ("set_back", |store, round| {
        let back = Some(bench_location(round.index));
        let _ = black_box(store.set_back(round.player, back));
    }),
    ("set_afk", |store, round| {
        let _ = black_box(store.set_afk(round.player, round.index % 2 == 0));
    }),
    ("set_home", |store, round| {
        let home = bench_location(round.index);
        let _ = black_box(store.set_home(round.player, "base", home));
    }),
    ("send_mail", |store, round| {
        let sent = store.send_mail(Some(round.other), "player", round.player, "hi", BENCH_AT);
        let _ = black_box(sent);
    }),
    ("mark_mail_read", |store, round| {
        let _ = black_box(store.mark_mail_read(round.player));
    }),
    ("transfer", |store, round| {
        let sent = store.transfer(round.player, round.other, 1, BENCH_AT, "pay");
        let _ = black_box(sent);
    }),
    ("deposit", |store, round| {
        let _ = black_box(store.deposit(round.player, 5, BENCH_AT, "eco give"));
    }),
    ("ignore", |store, round| {
        let _ = black_box(store.ignore(round.other, round.player));
    }),
    ("set_setting", |store, round| {
        let _ = black_box(store.set_setting("counter", &round.index.to_string()));
    }),
];

/// Times `repeat` calls of every kind, each call on its own, and answers the summary of each.
fn time_calls(store: &Store, players: usize, repeat: usize) -> BTreeMap<&'static str, Value> {
    let mut summaries = BTreeMap::new();
    for &(name, call) in CALLS {
        let mut nanos = Vec::with_capacity(repeat);
        for index in 0..repeat {
            let round = Round {
                index,
                number: index % players,
                player: bench_uuid(index % players),
                other: bench_uuid((index + 1) % players),
            };
            let started = Instant::now();
            call(store, &round);
            nanos.push(u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX));
        }
        summaries.insert(name, summary(nanos));
    }
    summaries
}

/// The resident memory of the process in KiB, which Linux names in `/proc/self/status`, or
/// `None` elsewhere. It bounds the memory of the cache from above.
fn resident_kib() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    line.split_whitespace().nth(1)?.parse().ok()
}

/// Fills a new store, opens it again, times the calls of the server thread and the writer
/// behind them, and prints the summary of each kind in microseconds.
fn bench(arguments: &[String]) -> i32 {
    let name = BENCH.0;
    let options = match bench_options(arguments) {
        Ok(options) => options,
        Err(message) => return fail(name, &message),
    };
    let run = || -> Result<Value, StoreError> {
        let store = Store::open(&options.path)?;
        let started = Instant::now();
        fill(&store, options.players)?;
        let fill_queued = started.elapsed();
        store.flush()?;
        let fill_written = started.elapsed();
        store.close()?;
        let started = Instant::now();
        let store = Store::open(&options.path)?;
        let open = started.elapsed();
        let resident = resident_kib();
        let before = store.stats();
        let calls = time_calls(&store, options.players, options.repeat);
        let started = Instant::now();
        store.flush()?;
        let flush = started.elapsed();
        let after = store.stats();
        let check = store.check()?;
        store.close()?;
        let file_bytes = fs::metadata(&options.path).map_or(0, |metadata| metadata.len());
        let millis = |duration: Duration| duration.as_secs_f64() * 1000.0;
        Ok(json!({
            "players": options.players,
            "repeat": options.repeat,
            "fill_queued_ms": millis(fill_queued),
            "fill_written_ms": millis(fill_written),
            "open_ms": millis(open),
            "calls": calls,
            "flush_after_calls_ms": millis(flush),
            "writer": {
                "written": after.written - before.written,
                "batches": after.batches - before.batches,
                "largest_batch": after.largest_batch,
                "busy_ms": millis(Duration::from_nanos(after.busy_nanos - before.busy_nanos)),
            },
            "check_passed": check.passed(),
            "file_bytes": file_bytes,
            "resident_kib_after_open": resident,
        }))
    };
    match run() {
        Ok(summary) => match emit(&summary) {
            Ok(()) => 0,
            Err(error) => output_closed(name, &error),
        },
        Err(error) => fail(name, &error.to_string()),
    }
}
