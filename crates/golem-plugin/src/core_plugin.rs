// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The core plugin of golem-plugin: the command golem with its subcommands source, status and
 * help, and the source of the running Golem that section 13 of the AGPL asks it to offer.
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
    runtime::milliseconds, //
};

/// What a [`Source`] names when the commit of the build is not known.
pub const UNKNOWN_COMMIT: &str = "unknown";

/// The command of the core plugin.
pub const COMMAND: &str = "golem";

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

    /// Answers the link to the source of the commit, the repository followed by `/tree/` and
    /// the commit, or the repository alone when the commit is unknown.
    pub fn link(&self) -> String {
        let repository = self.repository.trim_end_matches('/');
        let repository = repository.strip_suffix(".git").unwrap_or(repository);
        if self.commit_known() {
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
        vec![
            format!("Golem {}, the subcommands of /golem:", self.source.version),
            "/golem source: the license of Golem and the source of this build".to_owned(),
            "/golem status: the TPS, the MSPT and the time every plugin costs".to_owned(),
            "/golem help: this help".to_owned(),
        ]
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
            summary: "The source, the status and the help of Golem".to_owned(),
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
        _sender: &Sender,
        arguments: &str,
        context: &mut Context<'_>,
    ) -> Vec<String> {
        let subcommand = arguments
            .split_whitespace()
            .next()
            .map(str::to_ascii_lowercase);
        match subcommand.as_deref() {
            None | Some("help") => self.help(),
            Some("source") => self.source.lines(),
            Some("status") => self.status(context),
            Some(other) => {
                let mut lines = vec![format!("The subcommand {other} of /golem is unknown.")];
                lines.extend(self.help());
                lines
            }
        }
    }
}

/// Answers `nanos` in milliseconds.
fn nanos_to_milliseconds(nanos: u64) -> f64 {
    milliseconds(std::time::Duration::from_nanos(nanos))
}
