// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The permissions in the runtime of golem-plugin: the resolver of golem-permissions over the set
 * of the store, which loads the set again on a thread of its own when the generation of the store
 * moves, and the check of a node for a sender that every plugin reaches through its context.
 *
 * A check of the server thread must stay inside the budget of a call, and a reload of a large set
 * costs about a whole budget, so a check reads the permissions that the resolver holds and never
 * waits for a reload. Resolver::refresh looks at the generation, which costs one lock of the store,
 * so the server thread may call it once per tick, and wakes the thread golem-permissions, which
 * reads the set and its generation under one lock of the store, loads it and swaps it in.
 *
 * okf/plugins/permissions.md defines the rules of a check for each sender.
 */

use std::sync::{
    Arc, Mutex, PoisonError, RwLock,
    atomic::{
        AtomicBool,
        Ordering, //
    },
    mpsc,
};

use golem_bridge::{
    Sender,
    log, //
};
use golem_permissions::Permissions;
use golem_store::Store;

use crate::{
    Context,
    OPERATOR_LEVEL, //
};

/// The permissions that a resolver holds, with the generation of the store they came from.
struct Loaded {
    generation: u64,
    permissions: Arc<Permissions>,
}

struct Shared {
    /// The store whose set the resolver loads, or none for a resolver of fixed permissions.
    store: Option<Store>,
    loaded: RwLock<Loaded>,
    /// The generation whose set did not load, which the resolver does not try again.
    failed: Mutex<Option<u64>>,
    /// Whether a wake of the thread golem-permissions waits to be taken.
    pending: AtomicBool,
    wake: Mutex<Option<mpsc::Sender<()>>>,
}

/// The resolver of the permissions that the runtime hands every plugin, a handle that clones.
#[derive(Clone)]
pub struct Resolver {
    shared: Arc<Shared>,
}

/// The answer to whether a sender holds a node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionAnswer {
    /// Whether the sender holds the node.
    pub granted: bool,
    /// Whether a rule decided it, as `isPermissionSet` of Bukkit answers: a node of the player
    /// or of a group that matches, or a sender who holds every node. A port whose upstream gives
    /// a node the default true grants it when no rule decided.
    pub set: bool,
    /// The rule that decided, as a sentence.
    pub reason: String,
}

/// Loads the TOML of a set of the store into `golem-permissions`.
fn load(store: &Store) -> Result<(u64, Permissions), (u64, String)> {
    let (generation, set) = store.permissions_with_generation();
    Permissions::from_toml(&set.to_toml())
        .map(|permissions| (generation, permissions))
        .map_err(|error| (generation, error.to_string()))
}

impl Resolver {
    /// The resolver of the permissions of `store`, loaded on the calling thread, which is the
    /// start of the program, with the thread `golem-permissions` that loads them again.
    ///
    /// # Errors
    ///
    /// Answers why the set of the store does not load into `golem-permissions`, or why the
    /// thread did not start.
    pub fn from_store(store: &Store) -> Result<Self, String> {
        let (generation, permissions) = load(store).map_err(|(_, error)| {
            format!(
                "the permissions of the store {} do not load: {error}",
                store.path().display()
            )
        })?;
        let resolver = Self::new(Some(store.clone()), generation, permissions);
        let (wake, wakes) = mpsc::channel::<()>();
        let shared = Arc::clone(&resolver.shared);
        std::thread::Builder::new()
            .name("golem-permissions".to_owned())
            .spawn(move || {
                for () in wakes {
                    shared.pending.store(false, Ordering::SeqCst);
                    shared.reload();
                }
            })
            .map_err(|error| format!("the thread golem-permissions did not start: {error}"))?;
        *resolver
            .shared
            .wake
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(wake);
        Ok(resolver)
    }

    /// A resolver of fixed permissions without a store, for a tool or a test.
    pub fn from_permissions(permissions: Permissions) -> Self {
        Self::new(None, 0, permissions)
    }

    fn new(store: Option<Store>, generation: u64, permissions: Permissions) -> Self {
        Self {
            shared: Arc::new(Shared {
                store,
                loaded: RwLock::new(Loaded {
                    generation,
                    permissions: Arc::new(permissions),
                }),
                failed: Mutex::new(None),
                pending: AtomicBool::new(false),
                wake: Mutex::new(None),
            }),
        }
    }

    /// The generation of the store whose set the resolver holds.
    pub fn generation(&self) -> u64 {
        self.shared.read().generation
    }

    /// Wakes the thread `golem-permissions` when the generation of the store moved past the set
    /// the resolver holds. It takes one lock of the store and never waits for a reload, so a
    /// caller on the server thread may call it on every tick.
    pub fn refresh(&self) {
        let Some(store) = &self.shared.store else {
            return;
        };
        let generation = store.permissions_generation();
        if generation <= self.generation() || self.shared.failed() == Some(generation) {
            return;
        }
        if self.shared.pending.swap(true, Ordering::SeqCst) {
            return;
        }
        let wake = self
            .shared
            .wake
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if wake.as_ref().is_none_or(|wake| wake.send(()).is_err()) {
            self.shared.pending.store(false, Ordering::SeqCst);
        }
    }

    /// Whether `sender` holds `node`, by the rules of okf/plugins/permissions.md, without the
    /// sentence of the reason. A check of a player is one call of `Permissions::value`.
    pub fn has_permission(&self, sender: &Sender, node: &str) -> bool {
        holds(Some(&self.shared.read().permissions), sender, node)
    }

    /// Whether `sender` holds `node`, with the rule that decided.
    pub fn check_permission(&self, sender: &Sender, node: &str) -> PermissionAnswer {
        check(Some(&self.shared.read().permissions), sender, node)
    }
}

impl Shared {
    fn read(&self) -> std::sync::RwLockReadGuard<'_, Loaded> {
        self.loaded.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn failed(&self) -> Option<u64> {
        *self.failed.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn install(&self, generation: u64, permissions: Arc<Permissions>) {
        let mut set = Loaded {
            generation,
            permissions,
        };
        {
            let mut loaded = self.loaded.write().unwrap_or_else(PoisonError::into_inner);
            if generation >= loaded.generation {
                std::mem::swap(&mut *loaded, &mut set);
            }
        }
        // The set that the resolver does not hold is freed behind the lock, so no check waits
        // while its tables go.
        drop(set);
    }

    /// Loads the set of the store until the resolver holds its generation, or until a set does
    /// not load, which is logged once and kept from the next tries. A resolver that holds a
    /// later generation than the store has nothing to load.
    fn reload(&self) {
        let Some(store) = &self.store else {
            return;
        };
        loop {
            let generation = store.permissions_generation();
            if generation <= self.read().generation || self.failed() == Some(generation) {
                return;
            }
            match load(store) {
                Ok((generation, permissions)) => self.install(generation, Arc::new(permissions)),
                Err((generation, error)) => {
                    *self.failed.lock().unwrap_or_else(PoisonError::into_inner) = Some(generation);
                    log(format_args!(
                        "the permissions of the store {} at the generation {generation} do not load, so the checks keep the permissions they hold: {error}",
                        store.path().display()
                    ));
                    return;
                }
            }
        }
    }
}

/// The reason of a sender who holds every node or none, and whether they hold it.
fn without_nodes(sender: &Sender) -> Option<(bool, String)> {
    match sender {
        Sender::Console => Some((true, "The console holds every node".to_owned())),
        Sender::Other { name, level } if *level >= OPERATOR_LEVEL => Some((
            true,
            format!(
                "The source {name} runs at the level {level}, and a source that is no player holds every node from the level {OPERATOR_LEVEL}"
            ),
        )),
        Sender::Other { name, level } => Some((
            false,
            format!(
                "The source {name} runs at the level {level}, and a source that is no player holds no node below the level {OPERATOR_LEVEL}"
            ),
        )),
        Sender::Player { .. } => None,
    }
}

/// Whether `sender` holds `node` with the permissions `permissions`, or with none.
pub(crate) fn holds(permissions: Option<&Permissions>, sender: &Sender, node: &str) -> bool {
    match sender {
        Sender::Console => true,
        Sender::Other { level, .. } => *level >= OPERATOR_LEVEL,
        Sender::Player { player, .. } => {
            permissions.and_then(|permissions| permissions.value(&player.uuid, node)) == Some(true)
        }
    }
}

/// Whether `sender` holds `node` with the permissions `permissions`, or with none, and why.
pub(crate) fn check(
    permissions: Option<&Permissions>,
    sender: &Sender,
    node: &str,
) -> PermissionAnswer {
    if let Some((granted, reason)) = without_nodes(sender) {
        return PermissionAnswer {
            granted,
            set: granted,
            reason,
        };
    }
    let Sender::Player { player, .. } = sender else {
        unreachable!("without_nodes answers every other sender");
    };
    let Some(permissions) = permissions else {
        return PermissionAnswer {
            granted: false,
            set: false,
            reason: format!("The runtime holds no permissions, so no rule names {node}"),
        };
    };
    let decision = permissions.check(&player.uuid, node);
    PermissionAnswer {
        granted: decision.value == Some(true),
        set: decision.value.is_some(),
        reason: decision.reason,
    }
}

impl Context<'_> {
    /// Whether `sender` holds `node`, without the reason, for a check of the server thread.
    ///
    /// The console holds every node. Another source holds every node from the level 2 and none
    /// below it, and never a node of the player it runs as. A player holds what
    /// `golem-permissions` grants them, and a node that no rule names is not held, so a port
    /// whose upstream grants a node by default asks [`Context::check_permission`] and reads
    /// [`PermissionAnswer::set`]. Outside a runtime that holds permissions, a player holds no node.
    pub fn has_permission(&self, sender: &Sender, node: &str) -> bool {
        match self.permissions() {
            Some(resolver) => resolver.has_permission(sender, node),
            None => holds(None, sender, node),
        }
    }

    /// Whether `sender` holds `node`, with the rule that decided, by the rules of
    /// [`Context::has_permission`].
    pub fn check_permission(&self, sender: &Sender, node: &str) -> PermissionAnswer {
        match self.permissions() {
            Some(resolver) => resolver.check_permission(sender, node),
            None => check(None, sender, node),
        }
    }

    /// The resolver of the permissions of the runtime, or `None` outside a runtime or in one that
    /// the program gave none.
    pub fn permissions(&self) -> Option<&Resolver> {
        self.services()
            .and_then(|services| services.permissions.as_ref())
    }
}
