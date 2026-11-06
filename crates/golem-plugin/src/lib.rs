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
//!
//! okf/plugins/index.md describes the runtime and okf/plugins/core.md the core plugin. The crate
//! holds no test, which is the rule of CONTRIBUTING.md section 5. The notebook
//! `tests/notebooks/04_plugins.ipynb` drives the runtime through golem-devtool.

mod core_plugin;
mod runtime;

use golem_bridge::{
    Action,
    Actions,
    Event,
    Sender,
    Verdict, //
};

pub use crate::{
    core_plugin::{
        COMMAND,
        CorePlugin,
        Source,
        UNKNOWN_COMMIT, //
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
/// A plugin never touches the game itself. It queues actions, which the mod runs on the server
/// thread at the end of the tick, oldest first.
pub struct Context<'a> {
    actions: &'a Actions,
    budgets: Option<&'a Budgets>,
}

impl<'a> Context<'a> {
    /// A context that queues into `actions`.
    pub fn new(actions: &'a Actions) -> Self {
        Self {
            actions,
            budgets: None,
        }
    }

    /// A context of a call of the runtime, which also reads the budgets of its plugins.
    pub(crate) fn in_runtime(actions: &'a Actions, budgets: &'a Budgets) -> Self {
        Self {
            actions,
            budgets: Some(budgets),
        }
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
}
