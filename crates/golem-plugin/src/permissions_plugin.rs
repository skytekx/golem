// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The plugin golem-permissions of the runtime: it answers the PermissionAPI of Forge for every
 * Boolean node of an online player as LuckPerms 5.5.71 answered it through Arclight on the live
 * server, runs the command /luckperms of the port of LuckPerms with its aliases, writes what a
 * command changes to the store and installs it at once, and looks at the generation of the store
 * on every tick, so the resolver loads a change of another writer of the store. A command writes
 * through the handle of the store that never waits, which the resolver keeps, and asks the store
 * for room for all of its changes before the first, so it never waits for the writer of the store
 * on the server thread and makes all of its changes or none.
 *
 * A sender runs a command by its node. Where no rule of the permissions decides the node of a
 * player, an operator of the game runs the command, as LuckPerms lets the operators use every
 * command under its option commands-allow-op, which is on by default. holds_command is the one
 * place that says so, for the plugin and for the devtool.
 *
 * The native of the PermissionAPI carries the UUID and the node alone, so the plugin keeps the
 * operator level of every player online from their join, from the game on the server thread and
 * from the list of players once a second, and answers a player that is not online with none, so
 * the node falls to its default resolver, as Arclight left getOfflinePermission to it. A check of
 * another thread, such as the network thread of a mod, answers from what the plugin kept and asks
 * the game nothing, which would answer none there.
 *
 * The commands themselves are the code of the port in golem-permissions, under the MIT license of
 * LuckPerms. This file is the part of the runtime that hands them the sender, the players and the
 * store.
 */

use std::{
    collections::HashMap,
    thread::ThreadId, //
};

use golem_bridge::{
    Event,
    Sender,
    Verdict,
    log, //
};
use golem_permissions::{
    Config,
    command::{
        self,
        Change,
        Host,
        User, //
    },
};
use golem_store::{
    PermissionGroup,
    PlayerPermissions,
    Store,
    Uuid, //
};

use crate::{
    CommandSpec,
    Context,
    Plugin,
    Resolver,
    permissions::holds, //
};

/// The ticks between two reads of the operator levels of the players online.
const LEVEL_TICKS: u64 = 20;

/// The lowest operator level of a player whom the game lists as an operator. `LuckPerms` asked
/// Bukkit whether a player is an operator, and Bukkit counts every player on the list of
/// operators, which gives a level of 1 or more.
const LISTED_OPERATOR: u8 = 1;

/// Whether `sender` runs a command of the port of `LuckPerms` by its `node`, such as
/// `luckperms.user.info`.
///
/// The console and every other source hold the node as they hold every node, by the rules of
/// [`Context::has_permission`]. For a player a rule of `permissions` decides first, so a deny
/// takes a command from an operator too. Where no rule decides, an operator of the game runs
/// the command, as `LuckPerms` 5.5.71 lets the operators use every command under its option
/// `commands-allow-op`, which is on by default. `operator_level` is the operator level of the
/// player as the game lists it, or `None` when the game lists no such player. The lower of it
/// and the level of the source decides, so a command gains nothing by the level of its source,
/// and a command that runs below the level of its player is judged at its own.
pub fn holds_command(
    permissions: &golem_permissions::Permissions,
    sender: &Sender,
    node: &str,
    operator_level: Option<u8>,
) -> bool {
    match sender {
        Sender::Player { player, level } => {
            permissions.value(&player.uuid, node).unwrap_or_else(|| {
                operator_level.is_some_and(|own| own.min(*level) >= LISTED_OPERATOR)
            })
        }
        _ => holds(Some(permissions), sender, node),
    }
}

/// The plugin of the permissions, the port of `LuckPerms` in the runtime.
pub struct PermissionsPlugin {
    resolver: Resolver,
    /// The operator level of every player online, by their UUID in lower case.
    online: HashMap<String, u8>,
    ticks: u64,
    /// The server thread, from its events, on which the game answers. A check of another thread
    /// answers from what the plugin kept, so it asks the game nothing there.
    server_thread: Option<ThreadId>,
}

impl PermissionsPlugin {
    /// The name of the plugin.
    pub const NAME: &'static str = "golem-permissions";

    /// The plugin of the permissions of `resolver`.
    pub fn new(resolver: Resolver) -> Self {
        Self {
            resolver,
            online: HashMap::new(),
            ticks: 0,
            server_thread: None,
        }
    }

    /// Reads the operator level of every player online from the game.
    fn read_levels(&mut self, context: &Context<'_>) {
        if let Some(players) = context.game().players() {
            self.online = players
                .into_iter()
                .map(|player| (player.uuid.to_ascii_lowercase(), player.level))
                .collect();
        }
    }

    /// The operator level of the online player with the UUID `key`, from the game on the server
    /// thread and from what the plugin kept on any other, or `None` when they are not online.
    fn level(&mut self, key: &str, context: &Context<'_>) -> Option<u8> {
        if self.server_thread != Some(std::thread::current().id()) {
            return self.online.get(key).copied();
        }
        match context.game().player(key) {
            Some(Some(player)) => {
                self.online.insert(key.to_owned(), player.level);
                Some(player.level)
            }
            Some(None) => {
                self.online.remove(key);
                None
            }
            None => self.online.get(key).copied(),
        }
    }
}

impl Plugin for PermissionsPlugin {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn commands(&self) -> Vec<CommandSpec> {
        let summary = format!(
            "The permissions of Golem, the port of LuckPerms {}",
            command::VERSION
        );
        std::iter::once(command::COMMAND)
            .chain(command::ALIASES)
            .map(|name| CommandSpec {
                name: name.to_owned(),
                summary: summary.clone(),
            })
            .collect()
    }

    fn on_event(&mut self, event: &Event, context: &mut Context<'_>) -> Verdict {
        // Every event but a chat message runs on the server thread.
        if !matches!(event, Event::Chat { .. }) {
            self.server_thread = Some(std::thread::current().id());
        }
        match event {
            Event::Tick { .. } => {
                self.resolver.refresh();
                self.ticks += 1;
                if self.ticks.is_multiple_of(LEVEL_TICKS) && !self.online.is_empty() {
                    self.read_levels(context);
                }
            }
            Event::PlayerJoined(player) => {
                let key = player.uuid.to_ascii_lowercase();
                let level = match context.game().player(&key) {
                    Some(Some(online)) => online.level,
                    _ => 0,
                };
                self.online.insert(key, level);
            }
            Event::PlayerLeft(player) => {
                self.online.remove(&player.uuid.to_ascii_lowercase());
            }
            // A player still connected at the stop may get no event of their logout.
            Event::ServerStopping => self.online.clear(),
            _ => {}
        }
        Verdict::Pass
    }

    fn on_command(
        &mut self,
        name: &str,
        sender: &Sender,
        arguments: &str,
        context: &mut Context<'_>,
    ) -> Vec<String> {
        // The operator level of a player who made the command run, as the game lists it now on
        // the server thread, or as the plugin kept it on any other.
        let operator_level = match sender {
            Sender::Player { player, .. } => self.level(&player.uuid.to_ascii_lowercase(), context),
            _ => None,
        };
        // A set of the store that does not load leaves the resolver an older set, and a command
        // that built on it would write that older set back over the store. The reason can name a
        // group, a node or the UUID of a player, so it goes to the console and to a sender who
        // may run a command of the port by that older set, and every other sender reads the line
        // without it.
        let (generation, before) = match self.resolver.current() {
            Ok(current) => current,
            Err(reason) => {
                let older = self.resolver.permissions();
                let host = CommandHost {
                    sender,
                    context,
                    permissions: &older,
                    operator_level,
                };
                let shown = command::holds_any(&host).then_some(reason.as_str());
                return vec![command::not_loaded(shown)];
            }
        };
        let host = CommandHost {
            sender,
            context,
            permissions: &before,
            operator_level,
        };
        let outcome = command::execute(&before, name, arguments, &host);
        let mut lines = outcome.lines;
        let Some(after) = outcome.permissions else {
            return lines;
        };
        let store = self.resolver.store();
        // A command makes all of its changes or none. Where the store has no room for all of them
        // now, since its writer is behind, nothing is written and nothing installed, and the
        // sender reads that in place of the lines of the command.
        if let Some(store) = store
            && !outcome.changes.is_empty()
            && !store.room(outcome.changes.len())
        {
            log(format_args!(
                "the store has no room for the changes of /{name} {arguments} now, so the command changed nothing"
            ));
            return vec![command::busy()];
        }
        let written = match store {
            Some(store) => write(store, after.config(), &outcome.changes),
            None => Ok(()),
        };
        match written {
            Ok(()) => self.resolver.install(generation, after),
            Err(error) => {
                log(format_args!(
                    "the store did not take a change of /{name} {arguments}: {error}"
                ));
                // The store took the changes before the one it refused, so the resolver loads it
                // again instead of keeping a set that the store does not hold.
                if let Err(reason) = self.resolver.current() {
                    log(format_args!(
                        "the permissions of the store do not load behind that change: {reason}"
                    ));
                }
                lines.extend(outcome.save_error);
            }
        }
        lines
    }

    fn on_permission(&mut self, uuid: &str, node: &str, context: &mut Context<'_>) -> Option<bool> {
        let key = uuid.to_ascii_lowercase();
        let level = self.level(&key, context)?;
        // LuckPerms granted an operator every node that nothing names.
        let operator = level >= LISTED_OPERATOR;
        Some(
            self.resolver
                .permissions()
                .has_permission(&key, node, operator),
        )
    }
}

/// Writes the changes of a command to the store, in their order, from the configuration after
/// the command.
fn write(store: &Store, config: &Config, changes: &[Change]) -> Result<(), String> {
    for change in changes {
        match change {
            Change::Group(name) => {
                let Some(group) = config.groups.get(name) else {
                    continue;
                };
                store
                    .put_permission_group(PermissionGroup {
                        name: name.clone(),
                        weight: group.weight,
                        parents: group.inherits.clone(),
                        nodes: group.nodes.clone(),
                    })
                    .map_err(|error| error.to_string())?;
            }
            Change::RemovedGroup(name) => {
                store
                    .remove_permission_group(name)
                    .map_err(|error| error.to_string())?;
            }
            Change::Player(uuid) => {
                let Some(id) = Uuid::parse(uuid) else {
                    return Err(format!("{uuid} is no UUID"));
                };
                let player = config.players.get(uuid);
                store
                    .put_player_permissions(PlayerPermissions {
                        uuid: id,
                        groups: player
                            .map(|player| player.groups.clone())
                            .unwrap_or_default(),
                        nodes: player
                            .map(|player| player.nodes.clone())
                            .unwrap_or_default(),
                    })
                    .map_err(|error| error.to_string())?;
            }
        }
    }
    Ok(())
}

/// What a command of the port asks the runtime: the nodes of its sender, the players and the
/// store.
struct CommandHost<'a, 'c> {
    sender: &'a Sender,
    context: &'a Context<'c>,
    permissions: &'a golem_permissions::Permissions,
    /// The operator level of a player who made the command run, as the game lists it.
    operator_level: Option<u8>,
}

impl CommandHost<'_, '_> {
    fn store(&self) -> Option<&Store> {
        self.context.store()
    }
}

impl Host for CommandHost<'_, '_> {
    fn holds(&self, node: &str) -> bool {
        holds_command(self.permissions, self.sender, node, self.operator_level)
    }

    fn sender_name(&self) -> String {
        match self.sender {
            Sender::Console => "Console".to_owned(),
            Sender::Player { player, .. } => player.name.clone(),
            Sender::Other { name, .. } => name.clone(),
        }
    }

    fn player_named(&self, name: &str) -> Option<User> {
        if let Some(Some(online)) = self.context.game().player(name)
            && online.name.eq_ignore_ascii_case(name)
        {
            return Some(User {
                uuid: online.uuid.to_ascii_lowercase(),
                name: Some(online.name),
            });
        }
        let known = self.store()?.player_named(name)?;
        Some(User {
            uuid: known.uuid.to_string(),
            name: Some(known.name),
        })
    }

    fn name_of(&self, uuid: &str) -> Option<String> {
        if let Some(Some(online)) = self.context.game().player(uuid) {
            return Some(online.name);
        }
        let id = Uuid::parse(uuid)?;
        Some(self.store()?.player(id)?.name)
    }

    fn online(&self, uuid: &str) -> bool {
        matches!(self.context.game().player(uuid), Some(Some(_)))
    }
}
