// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The core plugin of golem-plugin: the command golem with its subcommands source, status, whoami
 * and help, the subcommands of operators of operator.rs, and the source of the running Golem that
 * section 13 of the AGPL asks it to offer.
 */

use golem_bridge::{
    Event,
    Sender,
    Verdict,
    log, //
};

use crate::{
    CommandSpec,
    Context,
    Plugin,
    operator,
    runtime::milliseconds, //
};

/// What a [`Source`] names when the commit of the build is not known.
pub const UNKNOWN_COMMIT: &str = "unknown";

/// The end of the commit of a build whose files differ from that commit, as the build script of
/// the program golem names it.
pub const DIRTY_SUFFIX: &str = "-dirty";

/// The command of the core plugin.
pub const COMMAND: &str = "golem";

/// The lines of the help after its first, one per subcommand.
const HELP: [&str; 8] = [
    "/golem source: the license of Golem and the source of this build",
    "/golem status: the TPS, the MSPT and the time every plugin costs",
    "/golem whoami: who Golem sees run the command, and at which level",
    "/golem player <name>: where a player is and how they play, for operators",
    "/golem world [<dimension>]: the spawn and the heights of a dimension, for operators",
    "/golem teleport <name> <dimension> <x> <y> <z>: a teleport to the nearest safe place, for operators",
    "/golem listname <name> [<text>]: the name of a player in the player list, for operators",
    "/golem help: this help",
];

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

impl Source {
    /// The source of this build: the version and the repository of the workspace, and the
    /// commit that the variable `GOLEM_COMMIT` named at build time, or unknown without it.
    pub fn of_build() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            repository: env!("CARGO_PKG_REPOSITORY").to_owned(),
            commit: option_env!("GOLEM_COMMIT")
                .filter(|commit| !commit.trim().is_empty())
                .unwrap_or(UNKNOWN_COMMIT)
                .to_owned(),
        }
    }

    /// Answers whether the commit of the build is known.
    pub fn commit_known(&self) -> bool {
        let commit = self.commit.trim();
        !commit.is_empty() && commit != UNKNOWN_COMMIT
    }

    /// Answers whether the build holds changes that its commit does not, so that the tree of the
    /// commit is not its source.
    pub fn dirty(&self) -> bool {
        self.commit.trim().ends_with(DIRTY_SUFFIX)
    }

    /// Answers the link to the source of the commit, the repository followed by `/tree/` and
    /// the commit, or the repository alone when the commit is unknown or the build is dirty, since
    /// no tree of the repository holds that source.
    pub fn link(&self) -> String {
        let repository = self.repository.trim_end_matches('/');
        let repository = repository.strip_suffix(".git").unwrap_or(repository);
        if self.commit_known() && !self.dirty() {
            format!("{repository}/tree/{}", self.commit.trim())
        } else {
            repository.to_owned()
        }
    }

    /// Answers the lines that name the license and the source, which `/golem source` and the
    /// program golem print.
    pub fn lines(&self) -> Vec<String> {
        vec![
            format!(
                "Golem {} is free software under the GNU Affero General Public License 3.0 only, AGPL-3.0-only.",
                self.version
            ),
            "Section 13 of the AGPL gives every player of this server the right to the source of the Golem it runs.".to_owned(),
            format!("Repository: {}", self.repository),
            format!("Commit: {}", self.commit),
            format!("Source of this build: {}", self.link()),
        ]
    }
}

/// The core plugin, which answers the command golem with its subcommands source and status.
pub struct CorePlugin {
    source: Source,
    /// Whether the log already named the connection of the bridge.
    announced: bool,
}

impl CorePlugin {
    /// The name of the core plugin.
    pub const NAME: &'static str = "golem-core";

    /// The core plugin of the Golem built from `source`.
    pub fn new(source: Source) -> Self {
        Self {
            source,
            announced: false,
        }
    }

    /// The source it names.
    pub fn source(&self) -> &Source {
        &self.source
    }

    /// Answers the lines of `/golem help`.
    fn help(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "Golem {}, the subcommands of /golem:",
            self.source.version
        )];
        lines.extend(HELP.map(str::to_owned));
        lines
    }

    /// Answers the lines of `/golem status`, from the tick statistics of the bridge and the
    /// budgets of the runtime.
    fn status(&self, context: &Context<'_>) -> Vec<String> {
        let stats = golem_bridge::stats();
        let mut lines = vec![format!(
            "Golem {}: TPS {:.2}, MSPT {:.2}, ticks {}",
            self.source.version, stats.tps, stats.mspt, stats.ticks
        )];
        let Some(budgets) = context.budgets() else {
            lines.push("The budgets of the plugins are known inside the runtime alone.".to_owned());
            return lines;
        };
        let table = budgets.snapshot();
        lines.push(format!(
            "Plugins: {}, with a budget of {:.3} ms per call",
            table.len(),
            milliseconds(budgets.limit())
        ));
        for (name, budget) in table {
            lines.push(format!(
                "{name}: calls {}, total {:.3} ms, longest {:.3} ms, over budget {}, panics {}",
                budget.calls,
                nanos_to_milliseconds(budget.total_nanos),
                nanos_to_milliseconds(budget.max_nanos),
                budget.over_budget,
                budget.panics
            ));
        }
        lines
    }
}

impl Plugin for CorePlugin {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![CommandSpec {
            name: COMMAND.to_owned(),
            summary: "The source, the status, the help and the tools of operators of Golem"
                .to_owned(),
        }]
    }

    fn on_event(&mut self, event: &Event, _context: &mut Context<'_>) -> Verdict {
        if let Event::ServerStarting {
            forge_version,
            minecraft_version,
        } = event
            && !self.announced
        {
            self.announced = true;
            log(format_args!(
                "the bridge is connected: Forge {forge_version}, Minecraft {minecraft_version}"
            ));
        }
        Verdict::Pass
    }

    fn on_command(
        &mut self,
        _name: &str,
        sender: &Sender,
        arguments: &str,
        context: &mut Context<'_>,
    ) -> Vec<String> {
        let words: Vec<&str> = arguments.split_whitespace().collect();
        let subcommand = words.first().map(|word| word.to_ascii_lowercase());
        let rest = words.get(1..).unwrap_or_default();
        if subcommand.as_deref() == Some("listname") {
            return operator::refusal("listname", sender, context.game())
                .unwrap_or_else(|| operator::list_name(after_first_word(arguments), context));
        }
        let operation: fn(&[&str], &Context<'_>) -> Vec<String> = match subcommand.as_deref() {
            None | Some("help") => return self.help(),
            Some("source") => return self.source.lines(),
            Some("status") => return self.status(context),
            Some("whoami") => return operator::whoami(sender, context.game()),
            Some("player") => operator::player,
            Some("world") => operator::world,
            Some("teleport") => operator::teleport,
            Some(_) => {
                // The match ignores the case, but the answer names the word as it was typed.
                let typed = words.first().copied().unwrap_or_default();
                let mut lines = vec![format!("The subcommand {typed} of /golem is unknown.")];
                lines.extend(self.help());
                return lines;
            }
        };
        let name = subcommand.unwrap_or_default();
        operator::refusal(&name, sender, context.game()).unwrap_or_else(|| operation(rest, context))
    }
}

/// Answers the arguments after their first word, as they were typed, with the whitespace before
/// the next word.
fn after_first_word(arguments: &str) -> &str {
    let trimmed = arguments.trim_start();
    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    &trimmed[end..]
}

/// Answers `nanos` in milliseconds.
fn nanos_to_milliseconds(nanos: u64) -> f64 {
    milliseconds(std::time::Duration::from_nanos(nanos))
}
