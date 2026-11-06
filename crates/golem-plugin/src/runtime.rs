// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The runtime of golem-plugin: the handler of golem-bridge that calls the plugins in their
 * order, times every call against the budget, catches the panic of a plugin and routes every
 * command to the plugin that owns it.
 */

use std::{
    panic::{
        self,
        AssertUnwindSafe, //
    },
    sync::{
        Arc,
        Mutex,
        MutexGuard,
        PoisonError, //
    },
    time::{
        Duration,
        Instant, //
    },
};

use golem_bridge::{
    Actions,
    Event,
    Handler,
    Sender,
    Verdict,
    host::panic_message,
    log, //
};

use crate::{
    CommandSpec,
    Context,
    Plugin, //
};

/// A budget for one call that fits many plugins into one tick of 50 ms, which the whole game
/// shares.
pub const DEFAULT_BUDGET: Duration = Duration::from_millis(1);

/// The least time between two lines on the log about the calls of one plugin over the budget.
pub const WARNING_INTERVAL: Duration = Duration::from_mins(1);

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
    /// The calls that panicked, which the runtime caught.
    pub panics: u64,
}

impl Budget {
    /// Counts one call that took `elapsed`.
    fn count(&mut self, elapsed: Duration, over_budget: bool, panicked: bool) {
        let nanos = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);
        self.calls = self.calls.saturating_add(1);
        self.total_nanos = self.total_nanos.saturating_add(nanos);
        self.max_nanos = self.max_nanos.max(nanos);
        self.over_budget = self.over_budget.saturating_add(u64::from(over_budget));
        self.panics = self.panics.saturating_add(u64::from(panicked));
    }
}

/// The budgets of the plugins of one runtime, readable while the runtime runs.
///
/// The runtime moves into the global of golem-bridge when it is installed, so whoever wants its
/// budgets later, such as the API or the command golem status, holds this handle instead. A
/// clone shares the table. The runtime takes the lock only for the moment it counts a call and
/// never while a plugin runs, so a plugin may read the table during its own call.
#[derive(Clone, Debug)]
pub struct Budgets {
    limit: Duration,
    table: Arc<Mutex<Vec<(String, Budget)>>>,
}

impl Budgets {
    /// The budget of one call.
    pub fn limit(&self) -> Duration {
        self.limit
    }

    /// Answers the name and the budget of every plugin, in their order.
    pub fn snapshot(&self) -> Vec<(String, Budget)> {
        self.lock().clone()
    }

    fn lock(&self) -> MutexGuard<'_, Vec<(String, Budget)>> {
        self.table.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A plugin in the runtime, with the name it gave when it was added.
struct Slot {
    plugin: Box<dyn Plugin>,
    name: String,
    /// When the log last named a call of the plugin over the budget.
    warned: Option<Instant>,
}

impl Slot {
    /// Logs a call over the budget, at most once per [`WARNING_INTERVAL`].
    fn warn(&mut self, what: Call<'_>, elapsed: Duration, limit: Duration) {
        let now = Instant::now();
        if self
            .warned
            .is_some_and(|at| now.saturating_duration_since(at) < WARNING_INTERVAL)
        {
            return;
        }
        self.warned = Some(now);
        log(format_args!(
            "the plugin {} took {:.3} ms on {what}, over the budget of {:.3} ms, and the log names its calls over the budget again in a minute at the earliest",
            self.name,
            milliseconds(elapsed),
            milliseconds(limit)
        ));
    }
}

/// What the runtime calls a plugin for, as the log names it.
#[derive(Clone, Copy)]
enum Call<'a> {
    /// The event of a native method, by its name.
    Event(&'static str),
    /// A command, by its name.
    Command(&'a str),
    /// A node of the `PermissionAPI` of Forge, by its name.
    Permission(&'a str),
}

impl std::fmt::Display for Call<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Event(name) => write!(formatter, "the event {name}"),
            Self::Command(name) => write!(formatter, "the command {name}"),
            Self::Permission(node) => write!(formatter, "the permission {node}"),
        }
    }
}

/// A command and the plugin that owns it.
struct Owner {
    spec: CommandSpec,
    plugin: usize,
    plugin_name: String,
}

/// The plugin runtime, the handler of golem-bridge.
///
/// The runtime calls its plugins in the order they were added. For an event that can be
/// cancelled, the first plugin that cancels it ends the chain, and the runtime answers
/// [`Verdict::Cancel`]. Every other event reaches every plugin and passes, whatever they
/// answer. A plugin that panics is logged and counted, and the chain goes on as if it passed.
/// The first plugin that declares a command owns it, and the runtime routes the command to it.
/// The first plugin that answers a permission decides it, and without one the node falls to
/// its default resolver.
pub struct Runtime {
    plugins: Vec<Slot>,
    commands: Vec<Owner>,
    budgets: Budgets,
}

impl Runtime {
    /// A runtime without plugins whose calls each have `budget`.
    pub fn new(budget: std::time::Duration) -> Self {
        Self {
            plugins: Vec::new(),
            commands: Vec::new(),
            budgets: Budgets {
                limit: budget,
                table: Arc::default(),
            },
        }
    }

    /// Adds a plugin behind the others. The runtime calls the plugins in that order.
    ///
    /// The runtime reads the commands of the plugin once, here. A command that an earlier
    /// plugin owns, or whose name is empty or holds a space or a slash, is logged and ignored.
    pub fn add(&mut self, plugin: Box<dyn Plugin>) {
        let name = plugin.name().to_owned();
        let index = self.plugins.len();
        let specs = match panic::catch_unwind(AssertUnwindSafe(|| plugin.commands())) {
            Ok(specs) => specs,
            Err(payload) => {
                log(format_args!(
                    "the plugin {name} panicked when it named its commands with \"{}\", so it gets none",
                    panic_message(payload.as_ref())
                ));
                Vec::new()
            }
        };
        for spec in specs {
            self.claim(index, &name, spec);
        }
        self.budgets.lock().push((name.clone(), Budget::default()));
        self.plugins.push(Slot {
            plugin,
            name,
            warned: None,
        });
    }

    /// The name and the budget of every plugin, in their order.
    pub fn budgets(&self) -> Vec<(String, Budget)> {
        self.budgets.snapshot()
    }

    /// Answers the handle on the budgets, which stays valid after the runtime is installed.
    pub fn budgets_handle(&self) -> Budgets {
        self.budgets.clone()
    }

    /// Answers the commands of the plugins with their summaries, each owned by one plugin.
    pub fn commands(&self) -> Vec<CommandSpec> {
        self.commands
            .iter()
            .map(|owner| owner.spec.clone())
            .collect()
    }

    /// Gives the command of `spec` to the plugin at `index`, unless it is taken or invalid.
    fn claim(&mut self, index: usize, plugin_name: &str, spec: CommandSpec) {
        let valid = !spec.name.is_empty()
            && !spec
                .name
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || c == '/');
        if !valid {
            log(format_args!(
                "the plugin {plugin_name} declares the command {:?}, which is no valid name, so the runtime ignores it",
                spec.name
            ));
            return;
        }
        if let Some(owner) = self
            .commands
            .iter()
            .find(|owner| owner.spec.name == spec.name)
        {
            log(format_args!(
                "the plugin {plugin_name} declares the command {}, which the plugin {} owns, so the runtime ignores it",
                spec.name, owner.plugin_name
            ));
            return;
        }
        self.commands.push(Owner {
            spec,
            plugin: index,
            plugin_name: plugin_name.to_owned(),
        });
    }

    /// Calls the plugin at `index` with `body`, timed and counted, and answers its value, or
    /// `None` when it panicked.
    fn call<T>(
        &mut self,
        index: usize,
        what: Call<'_>,
        actions: &Actions,
        body: impl FnOnce(&mut dyn Plugin, &mut Context<'_>) -> T,
    ) -> Option<T> {
        let Self {
            plugins, budgets, ..
        } = self;
        let slot = plugins.get_mut(index)?;
        let mut context = Context::in_runtime(actions, budgets);
        let started = Instant::now();
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
            body(slot.plugin.as_mut(), &mut context)
        }));
        let elapsed = started.elapsed();
        let over_budget = elapsed > budgets.limit;
        if let Some((_, budget)) = budgets.lock().get_mut(index) {
            budget.count(elapsed, over_budget, outcome.is_err());
        }
        if let Err(payload) = &outcome {
            log(format_args!(
                "the plugin {} panicked on {what} with \"{}\", so the runtime goes on as if it passed",
                slot.name,
                panic_message(payload.as_ref())
            ));
        }
        if over_budget {
            slot.warn(what, elapsed, budgets.limit);
        }
        outcome.ok()
    }
}

impl Handler for Runtime {
    fn event(&mut self, event: &Event, actions: &Actions) -> Verdict {
        let what = Call::Event(event.name());
        for index in 0..self.plugins.len() {
            let verdict = self.call(index, what, actions, |plugin, context| {
                plugin.on_event(event, context)
            });
            if event.cancellable() && verdict == Some(Verdict::Cancel) {
                return Verdict::Cancel;
            }
        }
        Verdict::Pass
    }

    fn command_names(&self) -> Vec<String> {
        self.commands
            .iter()
            .map(|owner| owner.spec.name.clone())
            .collect()
    }

    fn command(
        &mut self,
        name: &str,
        sender: &Sender,
        arguments: &str,
        actions: &Actions,
    ) -> Vec<String> {
        let Some(index) = self
            .commands
            .iter()
            .find(|owner| owner.spec.name == name)
            .map(|owner| owner.plugin)
        else {
            return vec![format!("The command {name} is unknown.")];
        };
        self.call(index, Call::Command(name), actions, |plugin, context| {
            plugin.on_command(name, sender, arguments, context)
        })
        .unwrap_or_else(|| {
            vec![format!(
                "The command {name} failed, and the log of the server holds the cause."
            )]
        })
    }

    fn permission(&mut self, uuid: &str, node: &str, actions: &Actions) -> Option<bool> {
        (0..self.plugins.len()).find_map(|index| {
            self.call(index, Call::Permission(node), actions, |plugin, context| {
                plugin.on_permission(uuid, node, context)
            })
            .flatten()
        })
    }
}

/// Answers `duration` in milliseconds, for a line on the log or in the game.
pub(crate) fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}
