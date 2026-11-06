// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The handle of the store that the plugins share.
 *
 * Every read answers from the cache. Every change checks its rules against the cache, changes
 * the cache and sends one operation to the writer while it holds the lock of the cache, so the
 * operations reach the writer in the order the cache took them. Nothing on this side waits for
 * the file, except flush, close, check and ledger, which say so and are not for the server
 * thread.
 */

use std::{
    cmp::Reverse,
    collections::BTreeMap,
    path::{
        Path,
        PathBuf, //
    },
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::Ordering,
        mpsc::{
            self,
            Sender, //
        },
    },
    thread::JoinHandle,
};

use rusqlite::Connection;

use crate::{
    cache::{
        Cache,
        fold, //
    },
    model::{
        Location,
        Mail,
        PermissionGroup,
        Player,
        PlayerPermissions,
        Receipt,
        Refusal,
        StoreError,
        Transfer,
        Uuid,
        Warp,
        check_location,
        check_name,
        check_text,
        check_token, //
    },
    permissions::{
        PermissionSet,
        check_group,
        check_player, //
    },
    schema,
    writer::{
        self,
        Message,
        Op,
        Shared, //
    },
};

/// The answer of [`Store::join`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Joined {
    /// Whether this is the first join of the player.
    pub first: bool,
    /// The name the player had before, when it changed.
    pub previous_name: Option<String>,
}

/// The counts of the writer, for the status of the server and the notebooks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stats {
    /// The changes the store sent to the writer.
    pub queued: u64,
    /// The changes in the file.
    pub written: u64,
    /// The changes the writer dropped after a failure.
    pub lost: u64,
    /// The changes on their way to the file.
    pub pending: u64,
    /// The transactions the writer committed.
    pub batches: u64,
    /// The most changes in one transaction.
    pub largest_batch: u64,
    /// The time the writer spent applying and committing, in nanoseconds.
    pub busy_nanos: u64,
    /// Why the writer failed, if it did.
    pub failure: Option<String>,
    /// Whether the store is closed.
    pub closed: bool,
}

/// What [`Store::check`] found in the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    /// The lines of `PRAGMA integrity_check`, which is `ok` alone for a sound file.
    pub integrity: Vec<String>,
    /// The rows that name a row of another table that does not exist.
    pub foreign_key_violations: i64,
    /// The accounts whose balance is not the sum of the log, each with its balance and that sum.
    pub ledger_mismatches: Vec<(Uuid, i64, i64)>,
    /// The parts in which the file and the cache differ, which is none once every change is in
    /// the file.
    pub cache_differences: Vec<String>,
    /// The rows of every table.
    pub rows: BTreeMap<String, i64>,
}

impl Check {
    /// Whether the file is sound, its accounts agree with their log and the cache agrees with it.
    pub fn passed(&self) -> bool {
        self.integrity == ["ok"]
            && self.foreign_key_violations == 0
            && self.ledger_mismatches.is_empty()
            && self.cache_differences.is_empty()
    }
}

/// The group whose spawn is the one spawn of the server, which holds for every player whose
/// groups have no spawn of their own, as in `EssentialsXSpawn`.
pub const DEFAULT_GROUP: &str = "default";

// Every plugin holds a handle, and the plugins run on the server thread and on the worker
// thread of the chat, so the handle must cross threads. This line fails to build if it cannot.
const _: fn() = || {
    fn shared<T: Send + Sync>() {}
    shared::<Store>();
};

/// The handle of an open store. A clone is another handle on the same store, so every plugin
/// holds one.
#[derive(Clone)]
pub struct Store {
    inner: Arc<Inner>,
}

struct Inner {
    path: PathBuf,
    state: Mutex<State>,
    shared: Arc<Shared>,
    writer: Mutex<Option<JoinHandle<()>>>,
}

struct State {
    cache: Cache,
    /// The queue of the writer, or `None` once the store is closed.
    sender: Option<Sender<Message>>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        let open = self.state.get_mut().map_or_else(
            |poisoned| poisoned.into_inner().sender.is_some(),
            |state| state.sender.is_some(),
        );
        if open {
            crate::log(format_args!(
                "the store {} was not closed, so it closes as its last handle goes",
                self.path.display()
            ));
            if let Err(error) = shutdown(self) {
                crate::log(format_args!("{error}"));
            }
        }
    }
}

/// Sends the stop to the writer and waits until the file is closed.
fn shutdown(inner: &Inner) -> Result<(), StoreError> {
    let sender = {
        let mut state = inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(sender) = state.sender.take() else {
            return Ok(());
        };
        sender
    };
    let (reply, answer) = mpsc::sync_channel(1);
    let sent = sender.send(Message::Stop(reply));
    drop(sender);
    let outcome = match sent {
        Ok(()) => answer.recv().unwrap_or_else(|_| {
            Err(StoreError::Failed(
                "the writer ended before it closed the file".to_owned(),
            ))
        }),
        Err(_) => Err(StoreError::Failed(
            "the writer ended before the stop".to_owned(),
        )),
    };
    let handle = inner
        .writer
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    if let Some(handle) = handle
        && handle.join().is_err()
    {
        return Err(StoreError::Failed("the writer panicked".to_owned()));
    }
    outcome
}

impl Store {
    /// Opens the store at `path`, creating the file when it does not exist, brings its schema to
    /// the latest version, loads the cache and starts the writer thread `golem-store`.
    ///
    /// The open reads the whole file and is meant for the start of the server, before the
    /// server thread runs. While the store is open, no other process can open the file.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref().to_owned();
        let connection = schema::open(&path)?;
        let cache = Cache::load(&connection).map_err(|error| StoreError::Open {
            path: path.clone(),
            reason: schema::describe_error(&error),
        })?;
        let shared = Arc::new(Shared::default());
        let (sender, messages) = mpsc::channel();
        let writer_shared = Arc::clone(&shared);
        let handle = std::thread::Builder::new()
            .name("golem-store".to_owned())
            .spawn(move || writer::run(connection, &messages, &writer_shared))
            .map_err(|error| StoreError::Open {
                path: path.clone(),
                reason: format!("the thread golem-store did not start: {error}"),
            })?;
        Ok(Self {
            inner: Arc::new(Inner {
                path,
                state: Mutex::new(State {
                    cache,
                    sender: Some(sender),
                }),
                shared,
                writer: Mutex::new(Some(handle)),
            }),
        })
    }

    /// The file of the store.
    pub fn path(&self) -> &Path {
        &self.inner.path
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // A panic of a caller while it held the lock leaves the cache as the last change left
        // it, which every change makes in one step after its checks, so the cache stays usable.
        self.inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Runs a change: `change` checks its rules and changes the cache, or refuses and leaves it
    /// as it is, and its operation goes to the writer under the same lock.
    fn change<T>(
        &self,
        change: impl FnOnce(&mut Cache) -> Result<(T, Option<Op>), Refusal>,
    ) -> Result<T, StoreError> {
        let mut state = self.lock();
        if state.sender.is_none() {
            return Err(StoreError::Closed);
        }
        if let Some(reason) = self.inner.shared.failure() {
            return Err(StoreError::Failed(reason));
        }
        let (value, op) = change(&mut state.cache)?;
        if let Some(op) = op {
            let shared = &self.inner.shared;
            let sent = state
                .sender
                .as_ref()
                .map(|sender| sender.send(Message::Write(op)));
            if !matches!(sent, Some(Ok(()))) {
                let reason = "the writer thread ended".to_owned();
                shared.failed.store(true, Ordering::Release);
                *shared
                    .failure
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(reason.clone());
                return Err(StoreError::Failed(reason));
            }
            shared.queued.fetch_add(1, Ordering::Relaxed);
        }
        Ok(value)
    }

    fn read<T>(&self, read: impl FnOnce(&Cache) -> T) -> T {
        read(&self.lock().cache)
    }

    /// Sends a message that carries a reply to the writer and waits for the reply.
    fn ask<T>(
        &self,
        message: impl FnOnce(mpsc::SyncSender<Result<T, StoreError>>) -> Message,
    ) -> Result<T, StoreError> {
        let (reply, answer) = mpsc::sync_channel(1);
        {
            let state = self.lock();
            let sender = state.sender.as_ref().ok_or(StoreError::Closed)?;
            sender
                .send(message(reply))
                .map_err(|_| StoreError::Failed("the writer thread ended".to_owned()))?;
        }
        answer
            .recv()
            .map_err(|_| StoreError::Failed("the writer ended before it answered".to_owned()))?
    }

    /// Runs a read on the connection of the writer, after every change before it.
    fn query<T: Send + 'static>(
        &self,
        query: impl FnOnce(&Connection) -> Result<T, StoreError> + Send + 'static,
    ) -> Result<T, StoreError> {
        self.ask(|reply| {
            Message::Read(Box::new(move |connection| {
                let _ = reply.send(query(connection));
            }))
        })
    }

    /// Waits until every change before the call is in the file and synced. Not for the server
    /// thread, which never waits for the disk.
    pub fn flush(&self) -> Result<(), StoreError> {
        self.ask(Message::Flush)
    }

    /// Writes every change into the file, checkpoints the log into it, closes it and ends the
    /// writer, then answers whether every change reached the file. A later change answers
    /// [`StoreError::Closed`], a read still answers from the cache, and a second close does
    /// nothing. The program golem calls it before it ends.
    pub fn close(&self) -> Result<(), StoreError> {
        shutdown(&self.inner)
    }

    /// The counts of the writer.
    pub fn stats(&self) -> Stats {
        let shared = &self.inner.shared;
        let queued = shared.queued.load(Ordering::Relaxed);
        let written = shared.written.load(Ordering::Relaxed);
        let lost = shared.lost.load(Ordering::Relaxed);
        Stats {
            queued,
            written,
            lost,
            pending: queued.saturating_sub(written).saturating_sub(lost),
            batches: shared.batches.load(Ordering::Relaxed),
            largest_batch: shared.largest_batch.load(Ordering::Relaxed),
            busy_nanos: shared.busy_nanos.load(Ordering::Relaxed),
            failure: shared.failure(),
            closed: self.lock().sender.is_none(),
        }
    }

    /// Checks the file after every change before the call: the integrity of `SQLite`, the foreign
    /// keys, every balance against the log, and the file against the cache as the call found
    /// it. Not for the server thread, since it reads the whole file.
    pub fn check(&self) -> Result<Check, StoreError> {
        let snapshot = {
            let state = self.lock();
            if state.sender.is_none() {
                return Err(StoreError::Closed);
            }
            crate::compare::snapshot(&state.cache)
        };
        self.query(move |connection| crate::compare::check(connection, &snapshot))
    }

    // The players.

    /// Records that a player joined with `name` at `at`: a new player is created, a known one
    /// gets the name, the last join and the last seen time.
    pub fn join(&self, uuid: Uuid, name: &str, at: i64) -> Result<Joined, StoreError> {
        check_name("the name of a player", name)?;
        self.change(|cache| {
            let (record, joined) = if let Some(entry) = cache.players.get(&uuid) {
                let mut record = entry.record.clone();
                let previous_name = (record.name != name).then(|| record.name.clone());
                name.clone_into(&mut record.name);
                record.last_join = at;
                record.last_seen = at;
                (
                    record,
                    Joined {
                        first: false,
                        previous_name,
                    },
                )
            } else {
                let record = Player {
                    uuid,
                    name: name.to_owned(),
                    first_join: at,
                    last_join: at,
                    last_seen: at,
                    logout: None,
                    back: None,
                    afk: false,
                    nickname: None,
                };
                (
                    record,
                    Joined {
                        first: true,
                        previous_name: None,
                    },
                )
            };
            cache.put_player(record.clone());
            Ok((joined, Some(Op::PutPlayer(Box::new(record)))))
        })
    }

    /// Changes the record of a known player.
    fn edit_player(&self, uuid: Uuid, edit: impl FnOnce(&mut Player)) -> Result<(), StoreError> {
        self.change(|cache| {
            let entry = cache
                .players
                .get(&uuid)
                .ok_or(Refusal::UnknownPlayer(uuid))?;
            let mut record = entry.record.clone();
            edit(&mut record);
            cache.put_player(record.clone());
            Ok(((), Some(Op::PutPlayer(Box::new(record)))))
        })
    }

    /// Records that a player quit at `at`, standing at `logout`.
    pub fn quit(&self, uuid: Uuid, logout: Location, at: i64) -> Result<(), StoreError> {
        check_location("the logout location", &logout)?;
        self.edit_player(uuid, |record| {
            record.logout = Some(logout);
            record.last_seen = at;
        })
    }

    /// Sets the place `/back` returns the player to, or clears it.
    pub fn set_back(&self, uuid: Uuid, back: Option<Location>) -> Result<(), StoreError> {
        if let Some(back) = &back {
            check_location("the back location", back)?;
        }
        self.edit_player(uuid, |record| record.back = back)
    }

    /// Sets whether the player is away from the keyboard.
    pub fn set_afk(&self, uuid: Uuid, afk: bool) -> Result<(), StoreError> {
        self.edit_player(uuid, |record| record.afk = afk)
    }

    /// Sets the nickname of the player, or clears it.
    pub fn set_nickname(&self, uuid: Uuid, nickname: Option<&str>) -> Result<(), StoreError> {
        if let Some(nickname) = nickname {
            check_name("a nickname", nickname)?;
        }
        let nickname = nickname.map(str::to_owned);
        self.edit_player(uuid, |record| record.nickname = nickname)
    }

    /// Sets the time the player was last seen.
    pub fn set_last_seen(&self, uuid: Uuid, at: i64) -> Result<(), StoreError> {
        self.edit_player(uuid, |record| record.last_seen = at)
    }

    /// Puts the whole record of a player in place of the one before, or creates it, keeping
    /// their homes, mail, ignore list and account. This is the call of the migration tool.
    pub fn put_player(&self, record: Player) -> Result<(), StoreError> {
        check_name("the name of a player", &record.name)?;
        if let Some(nickname) = &record.nickname {
            check_name("a nickname", nickname)?;
        }
        if let Some(logout) = &record.logout {
            check_location("the logout location", logout)?;
        }
        if let Some(back) = &record.back {
            check_location("the back location", back)?;
        }
        self.change(|cache| {
            cache.put_player(record.clone());
            Ok(((), Some(Op::PutPlayer(Box::new(record)))))
        })
    }

    /// The record of a player.
    pub fn player(&self, uuid: Uuid) -> Option<Player> {
        self.read(|cache| cache.players.get(&uuid).map(|entry| entry.record.clone()))
    }

    /// The player who joined last with `name`, in any case. A name can pass from one account to
    /// another, and the latest join wins.
    pub fn player_named(&self, name: &str) -> Option<Player> {
        self.read(|cache| {
            let uuids = cache.names.get(&fold(name))?;
            uuids
                .iter()
                .filter_map(|uuid| cache.players.get(uuid))
                .map(|entry| &entry.record)
                .max_by_key(|record| (record.last_join, Reverse(record.uuid)))
                .cloned()
        })
    }

    /// The players whose nickname is `nickname`, in any case, sorted by name and UUID.
    pub fn players_nicknamed(&self, nickname: &str) -> Vec<Player> {
        self.read(|cache| {
            let mut players: Vec<Player> = cache
                .nicknames
                .get(&fold(nickname))
                .into_iter()
                .flatten()
                .filter_map(|uuid| cache.players.get(uuid))
                .map(|entry| entry.record.clone())
                .collect();
            players.sort_by(|a, b| a.name.cmp(&b.name).then(a.uuid.cmp(&b.uuid)));
            players
        })
    }

    /// The number of players in the store.
    pub fn player_count(&self) -> usize {
        self.read(|cache| cache.players.len())
    }

    // The homes.

    /// Sets the home `name` of a player, and answers whether it replaced one.
    pub fn set_home(&self, uuid: Uuid, name: &str, location: Location) -> Result<bool, StoreError> {
        check_name("the name of a home", name)?;
        check_location("the location of a home", &location)?;
        self.change(|cache| {
            let entry = cache
                .players
                .get_mut(&uuid)
                .ok_or(Refusal::UnknownPlayer(uuid))?;
            let replaced = entry
                .homes
                .insert(name.to_owned(), location.clone())
                .is_some();
            let op = Op::PutHome {
                player: uuid,
                name: name.to_owned(),
                location,
            };
            Ok((replaced, Some(op)))
        })
    }

    /// Removes the home `name` of a player, and answers whether there was one.
    pub fn remove_home(&self, uuid: Uuid, name: &str) -> Result<bool, StoreError> {
        self.change(|cache| {
            let entry = cache
                .players
                .get_mut(&uuid)
                .ok_or(Refusal::UnknownPlayer(uuid))?;
            if entry.homes.remove(name).is_none() {
                return Ok((false, None));
            }
            let op = Op::RemoveHome {
                player: uuid,
                name: name.to_owned(),
            };
            Ok((true, Some(op)))
        })
    }

    /// The home `name` of a player.
    pub fn home(&self, uuid: Uuid, name: &str) -> Option<Location> {
        self.read(|cache| cache.players.get(&uuid)?.homes.get(name).cloned())
    }

    /// The homes of a player, sorted by name.
    pub fn homes(&self, uuid: Uuid) -> Vec<(String, Location)> {
        self.read(|cache| {
            cache.players.get(&uuid).map_or_else(Vec::new, |entry| {
                entry
                    .homes
                    .iter()
                    .map(|(name, location)| (name.clone(), location.clone()))
                    .collect()
            })
        })
    }

    /// The number of homes of a player.
    pub fn home_count(&self, uuid: Uuid) -> usize {
        self.read(|cache| {
            cache
                .players
                .get(&uuid)
                .map_or(0, |entry| entry.homes.len())
        })
    }

    // The warps.

    /// Sets the warp `name` with the node a player needs for it, if any, and answers whether it
    /// replaced one.
    pub fn set_warp(
        &self,
        name: &str,
        location: Location,
        permission: Option<&str>,
    ) -> Result<bool, StoreError> {
        check_name("the name of a warp", name)?;
        check_location("the location of a warp", &location)?;
        if let Some(permission) = permission {
            check_token("the permission of a warp", permission)?;
        }
        self.change(|cache| {
            let warp = Warp {
                name: name.to_owned(),
                location: location.clone(),
                permission: permission.map(str::to_owned),
            };
            let replaced = cache.warps.insert(name.to_owned(), warp).is_some();
            let op = Op::PutWarp {
                name: name.to_owned(),
                location,
                permission: permission.map(str::to_owned),
            };
            Ok((replaced, Some(op)))
        })
    }

    /// Removes the warp `name`, and answers whether there was one.
    pub fn remove_warp(&self, name: &str) -> Result<bool, StoreError> {
        self.change(|cache| {
            if cache.warps.remove(name).is_none() {
                return Ok((false, None));
            }
            Ok((true, Some(Op::RemoveWarp(name.to_owned()))))
        })
    }

    /// The warp `name`.
    pub fn warp(&self, name: &str) -> Option<Warp> {
        self.read(|cache| cache.warps.get(name).cloned())
    }

    /// The warps, sorted by name.
    pub fn warps(&self) -> Vec<Warp> {
        self.read(|cache| cache.warps.values().cloned().collect())
    }

    // The spawns.

    /// Sets the spawn of a group, [`DEFAULT_GROUP`] for the one spawn of the server, and answers
    /// whether it replaced one.
    pub fn set_spawn(&self, group: &str, location: Location) -> Result<bool, StoreError> {
        check_token("the group of a spawn", group)?;
        check_location("the location of a spawn", &location)?;
        self.change(|cache| {
            let replaced = cache
                .spawns
                .insert(group.to_owned(), location.clone())
                .is_some();
            let op = Op::PutSpawn {
                group: group.to_owned(),
                location,
            };
            Ok((replaced, Some(op)))
        })
    }

    /// Removes the spawn of a group, and answers whether there was one.
    pub fn remove_spawn(&self, group: &str) -> Result<bool, StoreError> {
        self.change(|cache| {
            if cache.spawns.remove(group).is_none() {
                return Ok((false, None));
            }
            Ok((true, Some(Op::RemoveSpawn(group.to_owned()))))
        })
    }

    /// The spawn of a group.
    pub fn spawn(&self, group: &str) -> Option<Location> {
        self.read(|cache| cache.spawns.get(group).cloned())
    }

    /// The spawns, sorted by group.
    pub fn spawns(&self) -> Vec<(String, Location)> {
        self.read(|cache| {
            cache
                .spawns
                .iter()
                .map(|(group, location)| (group.clone(), location.clone()))
                .collect()
        })
    }

    // The mail.

    /// Sends a mail from a player, or from the console when `sender` is `None`, and answers its
    /// number.
    pub fn send_mail(
        &self,
        sender: Option<Uuid>,
        sender_name: &str,
        receiver: Uuid,
        text: &str,
        at: i64,
    ) -> Result<i64, StoreError> {
        check_name("the name of the sender", sender_name)?;
        check_text("the text of a mail", text)?;
        if text.is_empty() {
            return Err(Refusal::Invalid {
                what: "the text of a mail",
                why: "is empty".to_owned(),
            }
            .into());
        }
        self.change(|cache| {
            if let Some(sender) = sender
                && !cache.players.contains_key(&sender)
            {
                return Err(Refusal::UnknownPlayer(sender));
            }
            let id = cache.next_mail;
            let entry = cache
                .players
                .get_mut(&receiver)
                .ok_or(Refusal::UnknownPlayer(receiver))?;
            let mail = Mail {
                id,
                sender,
                sender_name: sender_name.to_owned(),
                receiver,
                text: text.to_owned(),
                sent_at: at,
                read: false,
            };
            entry.mail.push(mail.clone());
            entry.unread += 1;
            cache.next_mail += 1;
            Ok((id, Some(Op::PutMail(Box::new(mail)))))
        })
    }

    /// The mail of a player, oldest first.
    pub fn mail(&self, receiver: Uuid) -> Vec<Mail> {
        self.read(|cache| {
            cache
                .players
                .get(&receiver)
                .map_or_else(Vec::new, |entry| entry.mail.clone())
        })
    }

    /// The number of mails a player has not read.
    pub fn unread_mail(&self, receiver: Uuid) -> usize {
        self.read(|cache| cache.players.get(&receiver).map_or(0, |entry| entry.unread))
    }

    /// Marks every mail of a player as read, and answers how many were unread.
    pub fn mark_mail_read(&self, receiver: Uuid) -> Result<usize, StoreError> {
        self.change(|cache| {
            let entry = cache
                .players
                .get_mut(&receiver)
                .ok_or(Refusal::UnknownPlayer(receiver))?;
            let unread = entry.unread;
            if unread == 0 {
                return Ok((0, None));
            }
            for mail in &mut entry.mail {
                mail.read = true;
            }
            entry.unread = 0;
            Ok((unread, Some(Op::MarkMailRead(receiver))))
        })
    }

    /// Removes the mail `id` of a player, and answers whether they had it.
    pub fn remove_mail(&self, receiver: Uuid, id: i64) -> Result<bool, StoreError> {
        self.change(|cache| {
            let entry = cache
                .players
                .get_mut(&receiver)
                .ok_or(Refusal::UnknownPlayer(receiver))?;
            let Some(index) = entry.mail.iter().position(|mail| mail.id == id) else {
                return Ok((false, None));
            };
            let mail = entry.mail.remove(index);
            if !mail.read {
                entry.unread -= 1;
            }
            Ok((true, Some(Op::RemoveMail(id))))
        })
    }

    /// Removes every mail of a player, and answers how many there were.
    pub fn clear_mail(&self, receiver: Uuid) -> Result<usize, StoreError> {
        self.change(|cache| {
            let entry = cache
                .players
                .get_mut(&receiver)
                .ok_or(Refusal::UnknownPlayer(receiver))?;
            let count = entry.mail.len();
            if count == 0 {
                return Ok((0, None));
            }
            entry.mail.clear();
            entry.unread = 0;
            Ok((count, Some(Op::ClearMail(receiver))))
        })
    }

    // The ignore lists.

    /// Puts `other` on the ignore list of `player`, and answers whether they were not on it.
    pub fn ignore(&self, player: Uuid, other: Uuid) -> Result<bool, StoreError> {
        if player == other {
            return Err(Refusal::IgnoreSelf(player).into());
        }
        self.change(|cache| {
            if !cache.players.contains_key(&other) {
                return Err(Refusal::UnknownPlayer(other));
            }
            let entry = cache
                .players
                .get_mut(&player)
                .ok_or(Refusal::UnknownPlayer(player))?;
            if !entry.ignores.insert(other) {
                return Ok((false, None));
            }
            let op = Op::PutIgnore {
                player,
                ignored: other,
            };
            Ok((true, Some(op)))
        })
    }

    /// Takes `other` off the ignore list of `player`, and answers whether they were on it.
    pub fn unignore(&self, player: Uuid, other: Uuid) -> Result<bool, StoreError> {
        self.change(|cache| {
            let entry = cache
                .players
                .get_mut(&player)
                .ok_or(Refusal::UnknownPlayer(player))?;
            if !entry.ignores.remove(&other) {
                return Ok((false, None));
            }
            let op = Op::RemoveIgnore {
                player,
                ignored: other,
            };
            Ok((true, Some(op)))
        })
    }

    /// Whether `player` ignores `other`.
    pub fn ignores(&self, player: Uuid, other: Uuid) -> bool {
        self.read(|cache| {
            cache
                .players
                .get(&player)
                .is_some_and(|entry| entry.ignores.contains(&other))
        })
    }

    /// The ignore list of a player, sorted by UUID.
    pub fn ignore_list(&self, player: Uuid) -> Vec<Uuid> {
        self.read(|cache| {
            cache
                .players
                .get(&player)
                .map_or_else(Vec::new, |entry| entry.ignores.iter().copied().collect())
        })
    }

    // The accounts. Every change of an account checks its note first, then its amount or the
    // balance it sets, then the accounts it names, in the order okf/store.md gives the refusals.

    /// Opens the account of a player with `starting` cents, which the log records as a deposit
    /// with `note` when it is above zero.
    pub fn create_account(
        &self,
        uuid: Uuid,
        starting: i64,
        at: i64,
        note: &str,
    ) -> Result<Receipt, StoreError> {
        check_text("the note of a transfer", note)?;
        if starting < 0 {
            return Err(Refusal::Negative(starting).into());
        }
        self.change(|cache| {
            if !cache.players.contains_key(&uuid) {
                return Err(Refusal::UnknownPlayer(uuid));
            }
            if cache.accounts.contains_key(&uuid) {
                return Err(Refusal::AccountExists(uuid));
            }
            let deposit = (starting > 0).then(|| {
                let entry = Transfer {
                    id: cache.next_transfer,
                    at,
                    source: None,
                    target: Some(uuid),
                    amount: starting,
                    note: note.to_owned(),
                };
                cache.next_transfer += 1;
                Box::new(entry)
            });
            cache.set_balance(uuid, starting);
            let receipt = Receipt {
                transfer: deposit.as_ref().map(|entry| entry.id),
                source_balance: None,
                target_balance: Some(starting),
            };
            let op = Op::CreateAccount {
                account: uuid,
                deposit,
            };
            Ok((receipt, Some(op)))
        })
    }

    /// Moves `amount` cents from `source` to `target` in one step, either of which may be `None`
    /// for the server, and logs it.
    fn move_money(
        &self,
        source: Option<Uuid>,
        target: Option<Uuid>,
        amount: i64,
        at: i64,
        note: &str,
    ) -> Result<Receipt, StoreError> {
        check_text("the note of a transfer", note)?;
        self.change(|cache| {
            let (receipt, op) = move_in(cache, source, target, amount, at, note)?;
            Ok((receipt, Some(op)))
        })
    }

    /// Adds `amount` cents to an account, money the server creates, as `eco give` does.
    pub fn deposit(
        &self,
        account: Uuid,
        amount: i64,
        at: i64,
        note: &str,
    ) -> Result<Receipt, StoreError> {
        self.move_money(None, Some(account), amount, at, note)
    }

    /// Takes `amount` cents from an account, money the server destroys, as `eco take` does.
    pub fn withdraw(
        &self,
        account: Uuid,
        amount: i64,
        at: i64,
        note: &str,
    ) -> Result<Receipt, StoreError> {
        self.move_money(Some(account), None, amount, at, note)
    }

    /// Moves `amount` cents from one account to another in one transaction, as `pay` does. It
    /// refuses an amount that is not above zero, the same account twice, an account that does
    /// not exist and a source that holds less than the amount.
    pub fn transfer(
        &self,
        source: Uuid,
        target: Uuid,
        amount: i64,
        at: i64,
        note: &str,
    ) -> Result<Receipt, StoreError> {
        self.move_money(Some(source), Some(target), amount, at, note)
    }

    /// Sets the balance of an account, as `eco set` does, through a deposit or a withdrawal of
    /// the difference, so the log still sums to the balance.
    pub fn set_balance(
        &self,
        account: Uuid,
        balance: i64,
        at: i64,
        note: &str,
    ) -> Result<Receipt, StoreError> {
        check_text("the note of a transfer", note)?;
        if balance < 0 {
            return Err(Refusal::Negative(balance).into());
        }
        self.change(|cache| {
            let current = cache
                .accounts
                .get(&account)
                .copied()
                .ok_or(Refusal::UnknownAccount(account))?;
            // The difference of two balances that are not negative always fits.
            let (receipt, op) = match balance - current {
                0 => {
                    let receipt = Receipt {
                        transfer: None,
                        source_balance: None,
                        target_balance: Some(current),
                    };
                    return Ok((receipt, None));
                }
                more if more > 0 => move_in(cache, None, Some(account), more, at, note)?,
                less => move_in(cache, Some(account), None, -less, at, note)?,
            };
            Ok((receipt, Some(op)))
        })
    }

    /// The balance of a player in cents, or `None` without an account.
    pub fn balance(&self, account: Uuid) -> Option<i64> {
        self.read(|cache| cache.accounts.get(&account).copied())
    }

    /// The `limit` largest balances, the largest first and the same balances by UUID.
    pub fn top_balances(&self, limit: usize) -> Vec<(Uuid, i64)> {
        self.read(|cache| {
            cache
                .ranking
                .iter()
                .take(limit)
                .map(|&(Reverse(balance), uuid)| (uuid, balance))
                .collect()
        })
    }

    /// The number of accounts.
    pub fn account_count(&self) -> usize {
        self.read(|cache| cache.accounts.len())
    }

    /// The last `limit` entries of the log, the newest first, of one account or of all. It
    /// reads the file through the writer after every change before it, so it waits for the
    /// disk and is not for the server thread.
    pub fn ledger(&self, account: Option<Uuid>, limit: usize) -> Result<Vec<Transfer>, StoreError> {
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        self.query(move |connection| {
            crate::compare::ledger(connection, account, limit)
                .map_err(|error| StoreError::Database(schema::describe_error(&error)))
        })
    }

    // The permissions.

    /// The groups and players of the permissions, which
    /// [`PermissionSet::to_toml`] turns into the text golem-permissions loads.
    pub fn permissions(&self) -> PermissionSet {
        self.read(|cache| PermissionSet {
            groups: cache.groups.values().cloned().collect(),
            players: cache.permission_players.values().cloned().collect(),
        })
    }

    /// The count of changes of the permissions since the open, so a resolver loads them again
    /// only when the count moved.
    pub fn permissions_generation(&self) -> u64 {
        self.read(|cache| cache.permissions_generation)
    }

    /// One group of the permissions.
    pub fn permission_group(&self, name: &str) -> Option<PermissionGroup> {
        self.read(|cache| cache.groups.get(name).cloned())
    }

    /// The groups and nodes of one player of the permissions.
    pub fn player_permissions(&self, uuid: Uuid) -> Option<PlayerPermissions> {
        self.read(|cache| cache.permission_players.get(&uuid).cloned())
    }

    /// Puts a whole set in place of the permissions, as the seed of `config/permissions.toml`
    /// or an import of `LuckPerms` does. A player whose lists are both empty is left out.
    pub fn replace_permissions(&self, set: PermissionSet) -> Result<(), StoreError> {
        set.check()?;
        let mut groups = set.groups;
        groups.sort_by(|a, b| a.name.cmp(&b.name));
        let mut players: Vec<PlayerPermissions> = set
            .players
            .into_iter()
            .filter(|player| !player.groups.is_empty() || !player.nodes.is_empty())
            .collect();
        players.sort_by_key(|player| player.uuid);
        self.change(|cache| {
            cache.groups = groups
                .iter()
                .map(|group| (group.name.clone(), group.clone()))
                .collect();
            cache.permission_players = players
                .iter()
                .map(|player| (player.uuid, player.clone()))
                .collect();
            cache.permissions_generation += 1;
            Ok(((), Some(Op::ReplacePermissions { groups, players })))
        })
    }

    /// Puts a group in place of the one of its name, or adds it, and answers whether it
    /// replaced one. Every parent must be a group of the store or the group itself.
    pub fn put_permission_group(&self, group: PermissionGroup) -> Result<bool, StoreError> {
        check_group(&group)?;
        self.change(|cache| {
            if let Some(parent) = group
                .parents
                .iter()
                .find(|parent| **parent != group.name && !cache.groups.contains_key(*parent))
            {
                return Err(Refusal::UnknownGroup(parent.clone()));
            }
            let replaced = cache
                .groups
                .insert(group.name.clone(), group.clone())
                .is_some();
            cache.permissions_generation += 1;
            Ok((replaced, Some(Op::PutGroup(Box::new(group)))))
        })
    }

    /// Removes a group that no other group and no player names, and answers whether it existed.
    pub fn remove_permission_group(&self, name: &str) -> Result<bool, StoreError> {
        self.change(|cache| {
            if !cache.groups.contains_key(name) {
                return Ok((false, None));
            }
            let in_use = |by: String| Refusal::GroupInUse {
                group: name.to_owned(),
                by,
            };
            if let Some(other) = cache
                .groups
                .values()
                .find(|other| other.name != name && other.parents.iter().any(|p| p == name))
            {
                return Err(in_use(format!("the group {}", other.name)));
            }
            if let Some(player) = cache
                .permission_players
                .values()
                .find(|player| player.groups.iter().any(|group| group == name))
            {
                return Err(in_use(format!("the player {}", player.uuid)));
            }
            cache.groups.remove(name);
            cache.permissions_generation += 1;
            Ok((true, Some(Op::RemoveGroup(name.to_owned()))))
        })
    }

    /// Puts the groups and nodes of a player in place of theirs. Both lists empty removes the
    /// player from the permissions.
    pub fn put_player_permissions(&self, player: PlayerPermissions) -> Result<(), StoreError> {
        check_player(&player)?;
        self.change(|cache| {
            if let Some(group) = player
                .groups
                .iter()
                .find(|group| !cache.groups.contains_key(*group))
            {
                return Err(Refusal::UnknownGroup(group.clone()));
            }
            cache.permissions_generation += 1;
            if player.groups.is_empty() && player.nodes.is_empty() {
                cache.permission_players.remove(&player.uuid);
                return Ok(((), Some(Op::RemovePlayerPermissions(player.uuid))));
            }
            cache.permission_players.insert(player.uuid, player.clone());
            Ok(((), Some(Op::PutPlayerPermissions(Box::new(player)))))
        })
    }

    /// Removes a player from the permissions, and answers whether they were in them.
    pub fn remove_player_permissions(&self, uuid: Uuid) -> Result<bool, StoreError> {
        self.change(|cache| {
            if cache.permission_players.remove(&uuid).is_none() {
                return Ok((false, None));
            }
            cache.permissions_generation += 1;
            Ok((true, Some(Op::RemovePlayerPermissions(uuid))))
        })
    }

    // The settings.

    /// Sets a setting.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), StoreError> {
        check_name("the key of a setting", key)?;
        check_text("the value of a setting", value)?;
        self.change(|cache| {
            cache.settings.insert(key.to_owned(), value.to_owned());
            let op = Op::PutSetting {
                key: key.to_owned(),
                value: value.to_owned(),
            };
            Ok(((), Some(op)))
        })
    }

    /// Removes a setting, and answers whether it existed.
    pub fn remove_setting(&self, key: &str) -> Result<bool, StoreError> {
        self.change(|cache| {
            if cache.settings.remove(key).is_none() {
                return Ok((false, None));
            }
            Ok((true, Some(Op::RemoveSetting(key.to_owned()))))
        })
    }

    /// A setting.
    pub fn setting(&self, key: &str) -> Option<String> {
        self.read(|cache| cache.settings.get(key).cloned())
    }

    /// The settings, sorted by key.
    pub fn settings(&self) -> Vec<(String, String)> {
        self.read(|cache| {
            cache
                .settings
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
    }
}

/// Moves `amount` cents from `source` to `target` in the cache, either of which may be `None` for
/// the server, and answers the receipt and the operation of the log. It changes nothing when it
/// refuses.
fn move_in(
    cache: &mut Cache,
    source: Option<Uuid>,
    target: Option<Uuid>,
    amount: i64,
    at: i64,
    note: &str,
) -> Result<(Receipt, Op), Refusal> {
    if amount <= 0 {
        return Err(Refusal::NotAboveZero(amount));
    }
    if let (Some(source), Some(target)) = (source, target)
        && source == target
    {
        return Err(Refusal::SameAccount(source));
    }
    let balance_of = |account: Uuid| {
        cache
            .accounts
            .get(&account)
            .copied()
            .ok_or(Refusal::UnknownAccount(account))
    };
    let source_before = source.map(balance_of).transpose()?;
    let target_before = target.map(balance_of).transpose()?;
    let source_after = match (source, source_before) {
        (Some(account), Some(balance)) if balance < amount => {
            return Err(Refusal::Overdraw {
                account,
                balance,
                amount,
            });
        }
        (_, before) => before.map(|balance| balance - amount),
    };
    let target_after = match (target, target_before) {
        (Some(account), Some(balance)) => {
            Some(balance.checked_add(amount).ok_or(Refusal::Overflow {
                account,
                balance,
                amount,
            })?)
        }
        _ => None,
    };
    if let (Some(account), Some(balance)) = (source, source_after) {
        cache.set_balance(account, balance);
    }
    if let (Some(account), Some(balance)) = (target, target_after) {
        cache.set_balance(account, balance);
    }
    let entry = Transfer {
        id: cache.next_transfer,
        at,
        source,
        target,
        amount,
        note: note.to_owned(),
    };
    cache.next_transfer += 1;
    let receipt = Receipt {
        transfer: Some(entry.id),
        source_balance: source_after,
        target_balance: target_after,
    };
    Ok((receipt, Op::Transfer(Box::new(entry))))
}
