// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * golem-plugin: the plugin runtime of Golem.
 *
 * A Golem plugin is Rust code that the runtime calls on the server thread, for the events that
 * can be cancelled or changed and for the commands it declares. Every call is timed against the
 * budget of the runtime, and the runtime keeps the time of every plugin, so the API can report
 * which plugin costs the server how much. A plugin that only observes uses the subscriptions of
 * the GraphQL API instead.
 *
 * The runtime is the handler of golem-bridge. The core plugin answers the command golem.
 */

//! The plugin runtime of Golem and its core plugin.
//!
//! | Module | Holds |
//! | --- | --- |
//! | `runtime` | [`Runtime`], the handler of golem-bridge, and [`Budget`] and [`Budgets`], the time of every plugin |
//! | `core_plugin` | [`CorePlugin`], the command golem, and [`Source`], the source that the AGPL asks Golem to offer |
//! | `operator` | The subcommands of /golem that ask the game, most of them for operators |
//!
//! okf/plugins/index.md describes the runtime and okf/plugins/core.md the core plugin. The crate
//! holds no test, which is the rule of CONTRIBUTING.md section 5. The notebook
//! `tests/notebooks/04_plugins.ipynb` drives the runtime through golem-devtool.

mod core_plugin;
mod operator;
mod runtime;

use golem_bridge::{
    Action,
    Actions,
    Event,
    Game,
    Location,
    NoGame,
    Sender,
    Verdict,
    log, //
};

pub use crate::{
    core_plugin::{
        COMMAND,
        CorePlugin,
        Source,
        UNKNOWN_COMMIT, //
    },
    operator::{
        OPERATOR_LEVEL,
        SAFE_RADIUS, //
    },
    runtime::{
        Budget,
        Budgets,
        DEFAULT_BUDGET,
        Runtime,
        WARNING_INTERVAL, //
    },
};

/// A command that a plugin declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandSpec {
    /// The name without its slash, such as golem.
    pub name: String,
    /// A short line for the help.
    pub summary: String,
}

/// What a plugin may do while the runtime calls it.
///
/// A plugin never changes the game itself. It queues actions, which the mod runs on the server
/// thread at the end of the tick, oldest first. It may ask the game during its call through
/// [`Context::game`], which answers on the server thread alone.
pub struct Context<'a> {
    actions: &'a Actions,
    budgets: Option<&'a Budgets>,
    game: &'a dyn Game,
}

impl<'a> Context<'a> {
    /// A context that queues into `actions`, with a game that answers nothing.
    pub fn new(actions: &'a Actions) -> Self {
        Self::with_game(actions, &NoGame)
    }

    /// A context that queues into `actions` and asks `game`.
    pub fn with_game(actions: &'a Actions, game: &'a dyn Game) -> Self {
        Self {
            actions,
            budgets: None,
            game,
        }
    }

    /// A context of a call of the runtime, which also reads the budgets of its plugins.
    pub(crate) fn in_runtime(
        actions: &'a Actions,
        budgets: &'a Budgets,
        game: &'a dyn Game,
    ) -> Self {
        Self {
            actions,
            budgets: Some(budgets),
            game,
        }
    }

    /// The game, which answers while the call runs on the server thread and answers none on
    /// any other thread, such as for a chat message, or without a game.
    pub fn game(&self) -> &dyn Game {
        self.game
    }

    /// The queue of actions, for what the helpers below do not cover.
    pub fn actions(&self) -> &Actions {
        self.actions
    }

    /// The budgets of the plugins of the runtime that makes the call, or `None` for a context
    /// made with [`Context::new`].
    pub fn budgets(&self) -> Option<&Budgets> {
        self.budgets
    }

    /// Queues a system message to the player with `uuid`, which the mod sends if they are
    /// online.
    pub fn send_message(&self, uuid: &str, text: &str) {
        self.actions.push(Action::Message {
            uuid: uuid.to_owned(),
            text: text.to_owned(),
        });
    }

    /// Queues a system message to every player and the console.
    pub fn broadcast(&self, text: &str) {
        self.actions.push(Action::Broadcast(text.to_owned()));
    }

    /// Queues a command that the mod runs as the console. A leading slash is dropped, since the
    /// console reads a command without it.
    pub fn run_command(&self, command: &str) {
        let command = command.strip_prefix('/').unwrap_or(command);
        self.actions.push(Action::Command(command.to_owned()));
    }

    /// Queues a teleport of the player with `uuid` to `location`, also into another dimension,
    /// which the mod runs if they are online, and answers whether it queued one.
    ///
    /// A location with a coordinate or an angle that is no finite number, or whose block lies
    /// outside the bounds of the world that the command tp takes, is refused with a line on the
    /// log, since the game cannot hold the player there. The place is taken as it is, so a plugin
    /// that wants a safe one asks the game for it first.
    pub fn teleport(&self, uuid: &str, location: &Location) -> bool {
        if !location.is_finite() {
            log(format_args!(
                "a plugin asked to teleport {uuid} to {location}, which is no finite place, so the runtime refuses it"
            ));
            return false;
        }
        if !location.is_in_world() {
            log(format_args!(
                "a plugin asked to teleport {uuid} to {location}, which lies outside the bounds of the world, so the runtime refuses it"
            ));
            return false;
        }
        self.actions.push(Action::Teleport {
            uuid: uuid.to_owned(),
            location: location.clone(),
        });
        true
    }

    /// Queues the name that the player list shows for the player with `uuid` until they leave
    /// or a plugin clears it, such as their name with a mark for away. An empty name clears it.
    pub fn set_list_name(&self, uuid: &str, name: &str) {
        self.actions.push(Action::ListName {
            uuid: uuid.to_owned(),
            name: (!name.is_empty()).then(|| name.to_owned()),
        });
    }

    /// Queues the clearing of the name in the player list of the player with `uuid`, so the
    /// list shows what it showed before a plugin named them.
    pub fn clear_list_name(&self, uuid: &str) {
        self.actions.push(Action::ListName {
            uuid: uuid.to_owned(),
            name: None,
        });
    }
}

/// A Golem plugin.
pub trait Plugin: Send {
    /// The name, such as golem-essentials.
    fn name(&self) -> &str;

    /// The commands the plugin answers.
    fn commands(&self) -> Vec<CommandSpec> {
        Vec::new()
    }

    /// Answers an event. The default lets it pass.
    fn on_event(&mut self, event: &Event, context: &mut Context<'_>) -> Verdict {
        let _ = (event, context);
        Verdict::Pass
    }

    /// Runs one of the commands of `commands` and answers the lines the sender sees.
    fn on_command(
        &mut self,
        name: &str,
        sender: &Sender,
        arguments: &str,
        context: &mut Context<'_>,
    ) -> Vec<String> {
        let _ = (name, sender, arguments, context);
        Vec::new()
    }

    /// Answers whether the player with `uuid` holds `node` of the `PermissionAPI` of Forge, such as
    /// `forge.use_entity_selectors`, or `None` to let the next plugin decide. The default answers
    /// `None`. Mods ask Forge on any thread, so this may run beside the server thread.
    fn on_permission(&mut self, uuid: &str, node: &str, context: &mut Context<'_>) -> Option<bool> {
        let _ = (uuid, node, context);
        None
    }
}
