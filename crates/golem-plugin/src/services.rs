// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * What the runtime of golem-plugin hands every plugin beside the game: the store of the plugins,
 * which the program golem opens before the JVM starts and gives the runtime before it installs
 * it, and the resolver of the permissions over the set of that store. A plugin reaches both
 * through the context of its call, so no plugin holds a handle of its own that could outlive the
 * close of the program.
 *
 * The runtime flushes the store once every plugin saw ServerStopped, the last event of the server
 * thread, where a wait for the disk costs no tick, so every change of the plugins is in the file
 * before the JVM ends, also when nothing closes the store after it, such as a kill of a JVM whose
 * threads keep it from ending. The wait counts in the budget of no plugin.
 */

use golem_bridge::{
    Event,
    log, //
};
use golem_store::Store;

use crate::{
    Context,
    Resolver,
    Runtime, //
};

/// What the runtime holds for its plugins.
#[derive(Default)]
pub(crate) struct Services {
    /// The store of the plugins, once the program gave one.
    pub store: Option<Store>,
    /// The resolver of the permissions, once the program gave one.
    pub permissions: Option<Resolver>,
}

impl Services {
    /// Runs after every plugin saw `event`: flushes the store on `ServerStopped`.
    pub fn after_event(&self, event: &Event) {
        if !matches!(event, Event::ServerStopped) {
            return;
        }
        if let Some(store) = &self.store
            && let Err(error) = store.flush()
        {
            log(format_args!(
                "the server stopped, but the store {} did not flush: {error}",
                store.path().display()
            ));
        }
    }
}

impl Runtime {
    /// Gives the plugins the store, which every later call reaches through
    /// [`Context::store`]. A second call replaces the store. The runtime keeps the handle of
    /// the store that never waits, whatever handle it is given, so no call of a plugin waits
    /// for the writer of the store.
    pub fn set_store(&mut self, store: Store) {
        self.services.store = Some(store.never_waiting());
    }

    /// Gives the plugins the resolver of the permissions, which every later call reaches
    /// through [`Context::has_permission`], [`Context::check_permission`] and
    /// [`Context::permissions`]. A second call replaces the resolver.
    pub fn set_permissions(&mut self, resolver: Resolver) {
        self.services.permissions = Some(resolver);
    }
}

impl Context<'_> {
    /// The store of the plugins, or `None` outside a runtime or in a runtime that the program
    /// gave none.
    ///
    /// Every read answers from the cache of the store and every change queues one operation for
    /// its writer, so a call of the server thread waits for no disk, as okf/store.md says. The
    /// handle never waits for the writer either: a change that finds the writer as far behind
    /// as the store lets it fall answers `StoreError::Behind` and changes nothing, so a command
    /// tells its sender that the server is busy, and one that makes several changes asks
    /// `Store::room` first. A plugin changes the store inside its calls alone and on no thread
    /// of its own, since the answer of `room` holds only while nothing else changes the store.
    /// A flush, a close, a check and a read through the writer wait for the disk and belong on
    /// another thread or on `ServerStopped`.
    pub fn store(&self) -> Option<&Store> {
        self.services.and_then(|services| services.store.as_ref())
    }

    /// What the runtime holds for its plugins, or `None` outside a runtime.
    pub(crate) fn services(&self) -> Option<&Services> {
        self.services
    }
}
