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

use golem_bridge::{
    Actions,
    Event,
    Sender,
    Verdict, //
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
pub struct Context<'a> {
    actions: &'a Actions,
}

impl<'a> Context<'a> {
    /// A context that queues into `actions`.
    pub fn new(actions: &'a Actions) -> Self {
        Self { actions }
    }

    /// The queue of actions, for what the helpers below do not cover.
    pub fn actions(&self) -> &Actions {
        self.actions
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

/// The time a plugin has cost so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Budget {
    /// The calls.
    pub calls: u64,
    /// The time of all calls, in nanoseconds.
    pub total_nanos: u64,
    /// The longest call, in nanoseconds.
    pub max_nanos: u64,
    /// The calls that took longer than the budget of the runtime.
    pub over_budget: u64,
}

/// The plugin runtime, the handler of golem-bridge.
pub struct Runtime {
    plugins: Vec<Box<dyn Plugin>>,
}

impl Runtime {
    /// A runtime without plugins whose calls each have `budget`.
    pub fn new(budget: std::time::Duration) -> Self {
        let _ = budget;
        todo!("golem-plugin: Runtime::new")
    }

    /// Adds a plugin behind the others. The runtime calls the plugins in that order.
    pub fn add(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    /// The name and the budget of every plugin, in their order.
    pub fn budgets(&self) -> Vec<(String, Budget)> {
        todo!("golem-plugin: Runtime::budgets")
    }
}

/// The source of the running Golem, which section 13 of the AGPL asks it to offer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    /// The version of the crates, such as 0.1.0.
    pub version: String,
    /// The URL of the repository.
    pub repository: String,
    /// The commit the binary was built from, or unknown.
    pub commit: String,
}

/// The core plugin, which answers the command golem with its subcommands source and status.
pub struct CorePlugin {
    source: Source,
}

impl CorePlugin {
    /// The core plugin of the Golem built from `source`.
    pub fn new(source: Source) -> Self {
        Self { source }
    }

    /// The source it names.
    pub fn source(&self) -> &Source {
        &self.source
    }
}
