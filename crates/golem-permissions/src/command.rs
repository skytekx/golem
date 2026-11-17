// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
// SPDX-FileCopyrightText: lucko (Luck) <luck@lucko.me>
// SPDX-FileCopyrightText: LuckPerms contributors
/*
 * The command /luckperms of the port of LuckPerms 5.5.71, with its aliases /lp, /perm, /perms,
 * /permission and /permissions, for the operators who read and change the permissions in game
 * and on the console.
 *
 * The port takes the commands of LuckPerms whose data Golem holds: user and group with their
 * info, permission and parent commands and clear, the weight and the members of a group, and
 * creategroup, deletegroup and listgroups. It keeps their names, the convenience aliases of the
 * CommandManager of LuckPerms, their nodes under luckperms. and the English texts of
 * luckperms_en.properties and Message.java, as plain text, since the bridge sends a line as
 * plain text. Golem has no contexts, no expiry, no meta, no tracks and no web editor, so the
 * commands of those name what the port does not take instead of pretending.
 *
 * A command edits the configuration of the permissions that it is given and loads the result
 * with Permissions::from_config, so a change that breaks a rule of golem-permissions is refused
 * before anything is written. The caller writes the changes that the outcome names to the store
 * and installs the permissions of the outcome, so the change takes effect at once. This file
 * names no store and no plugin runtime, so the crate keeps its license and its callers.
 */

use crate::{
    Config,
    DEFAULT_GROUP,
    GroupConfig,
    Permissions,
    PlayerConfig,
    node, //
};

/// The name of the command.
pub const COMMAND: &str = "luckperms";

/// The aliases of the command, as the `plugin.yml` of `LuckPerms` 5.5.71 names them.
pub const ALIASES: [&str; 5] = ["lp", "perm", "perms", "permission", "permissions"];

/// The version of `LuckPerms` that the commands port.
pub const VERSION: &str = "5.5.71";

/// The start of every line of `LuckPerms` that it marks as its own.
const PREFIX: &str = "[LP] ";

/// The context of every node of Golem, which has no contexts, as `LuckPerms` names an empty set.
const GLOBAL: &str = "global";

/// The entries of one page of a list, as `LuckPerms` divides the nodes of a holder.
const PAGE: usize = 19;

/// The entries of one page of the list of groups.
const GROUP_PAGE: usize = 8;

/// The entries of one page of the members of a group.
const MEMBER_PAGE: usize = 15;

/// A player that a command names, by name or UUID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
    /// The UUID in lower case with its dashes.
    pub uuid: String,
    /// The name of the player, when it is known.
    pub name: Option<String>,
}

/// What a command asks of the program that runs it.
pub trait Host {
    /// Whether the sender holds a node of the commands, such as `luckperms.user.info`: by a rule
    /// of the permissions, or, where no rule decides, as an operator of the game, whom
    /// `LuckPerms` lets use every command under its option `commands-allow-op`.
    fn holds(&self, node: &str) -> bool;

    /// The name of the sender, for the first steps of a server without permissions.
    fn sender_name(&self) -> String;

    /// The player whose name is `name`, in any case, among the players online and those the
    /// server knows, or `None`.
    fn player_named(&self, name: &str) -> Option<User>;

    /// The name of the player with `uuid`, or `None` when the server does not know them.
    fn name_of(&self, uuid: &str) -> Option<String>;

    /// Whether the player with `uuid` is online.
    fn online(&self, uuid: &str) -> bool;
}

/// An entry that a command changed, which the caller writes to the store in the order given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// A group that is new or changed, by name, with its entry of the configuration.
    Group(String),
    /// A group that is gone, by name. Every entry that named it changed before.
    RemovedGroup(String),
    /// A player whose groups or nodes changed, by UUID, with their entry of the configuration,
    /// which is gone when both lists are empty.
    Player(String),
}

/// What a command answers.
#[derive(Debug, Default)]
pub struct Outcome {
    /// The lines the sender sees.
    pub lines: Vec<String>,
    /// The entries the command changed, in the order the store takes them.
    pub changes: Vec<Change>,
    /// The permissions after the change, checked, when the command changed any, which the caller
    /// installs once the store took every change.
    pub permissions: Option<Permissions>,
    /// The line that `LuckPerms` sends when the data of the holder does not save, for the caller
    /// to send when the store refuses a change.
    pub save_error: Option<String>,
}

impl Outcome {
    fn lines(lines: Vec<String>) -> Self {
        Self {
            lines,
            ..Self::default()
        }
    }

    fn line(line: String) -> Self {
        Self::lines(vec![line])
    }
}

/// A line of `LuckPerms` with its prefix.
fn prefixed(text: &str) -> String {
    format!("{PREFIX}{text}")
}

/// The line of a command that the port does not take.
fn not_ported(label: &str, path: &str) -> String {
    prefixed(&format!(
        "Golem does not port /{label} {path} of LuckPerms {VERSION}, see okf/plugins/permissions.md."
    ))
}

/// Whether the sender of `host` holds the node of any command of the port.
pub fn holds_any(host: &dyn Host) -> bool {
    MAIN.iter().any(|main| main.authorized(host))
}

/// The line that the caller answers in place of a command while the permissions that the
/// command would build on do not load. The reason can name a group, a node or the UUID of a
/// player of the configuration, so the caller hands it for the console and for a sender who
/// holds a node of the commands alone, and every other sender reads the line without it.
pub fn not_loaded(reason: Option<&str>) -> String {
    match reason {
        Some(reason) => prefixed(&format!(
            "The permissions of Golem do not load, so no command runs: {reason}."
        )),
        None => prefixed("The permissions of Golem do not load, so no command runs."),
    }
}

/// The line that the caller answers in place of the lines of a command whose changes the store
/// does not take now, so that the command changed nothing.
pub fn busy() -> String {
    prefixed(
        "The store of Golem takes no change now, so nothing changed. Run the command again later.",
    )
}

/// The line of arguments that would name a context, which Golem does not have.
fn no_context(arguments: &[&str]) -> String {
    prefixed(&format!(
        "The permissions of Golem have no contexts, so the port takes no {}.",
        arguments.join(" ")
    ))
}

/// Runs the command `/<label> <arguments>` on `permissions` for the sender of `host`.
pub fn execute(
    permissions: &Permissions,
    label: &str,
    arguments: &str,
    host: &dyn Host,
) -> Outcome {
    let mut words: Vec<String> = arguments.split_whitespace().map(str::to_owned).collect();
    convenience_aliases(&mut words);
    let Some(first) = words.first().map(|word| word.to_lowercase()) else {
        return running(permissions, label, host);
    };
    let rest: Vec<&str> = words[1..].iter().map(String::as_str).collect();
    let main = MAIN.iter().find(|main| main.name == first);
    let Some(main) = main.filter(|main| main.authorized(host)) else {
        if UNPORTED_MAIN.contains(&first.as_str()) {
            return Outcome::line(not_ported(label, &first));
        }
        return Outcome::lines(command_usage(label, host));
    };
    let mut editor = Editor::new(permissions, label, host);
    match main.name {
        "user" => editor.user(&rest),
        "group" => editor.group(&rest),
        "creategroup" => editor.create_group(&rest),
        "deletegroup" => editor.delete_group(&rest),
        _ => editor.list_groups(&rest),
    }
    editor.finish()
}

/// Rewrites the convenience aliases of the `CommandManager` of `LuckPerms`: `u`, `g`, `t` and `i`
/// as the first word, `p`, `perm`, `g`, `group`, `m`, `i` and `e` as the third of a user or a
/// group, and `i` as the fourth after `permission`, `parent` or `meta`.
fn convenience_aliases(words: &mut [String]) {
    fn replace(words: &mut [String], index: usize, rewrite: fn(&str) -> Option<&'static str>) {
        if let Some(word) = words.get(index)
            && let Some(new) = rewrite(&word.to_lowercase())
        {
            new.clone_into(&mut words[index]);
        }
    }
    replace(words, 0, |word| match word {
        "u" => Some("user"),
        "g" => Some("group"),
        "t" => Some("track"),
        "i" => Some("info"),
        _ => None,
    });
    if words.len() >= 3 && matches!(words[0].to_lowercase().as_str(), "user" | "group") {
        replace(words, 2, |word| match word {
            "p" | "perm" => Some("permission"),
            "g" | "group" => Some("parent"),
            "m" => Some("meta"),
            "i" => Some("info"),
            "e" => Some("editor"),
            _ => None,
        });
        if words.len() >= 4
            && matches!(
                words[2].to_lowercase().as_str(),
                "permission" | "parent" | "meta"
            )
        {
            replace(words, 3, |word| (word == "i").then_some("info"));
        }
    }
}

/// A main command of the port, with its node or the kind whose subcommands it holds.
struct Main {
    name: &'static str,
    usage: &'static str,
    /// The node of a command of its own.
    node: &'static str,
    /// The kind of a command whose subcommands name a user or a group, which the sender may use
    /// when they hold the node of any of them.
    kind: Option<Kind>,
}

impl Main {
    fn authorized(&self, host: &dyn Host) -> bool {
        match self.kind {
            Some(kind) => kind.subcommands().any(|sub| host.holds(&kind.node(sub))),
            None => host.holds(self.node),
        }
    }
}

/// The main commands of the port, in the order of the `CommandManager` of `LuckPerms`.
const MAIN: [Main; 5] = [
    Main {
        name: "user",
        usage: "/%s user <user>",
        node: "luckperms.user",
        kind: Some(Kind::User),
    },
    Main {
        name: "group",
        usage: "/%s group <group>",
        node: "luckperms.group",
        kind: Some(Kind::Group),
    },
    Main {
        name: "creategroup",
        usage: "/%s creategroup <group>",
        node: "luckperms.creategroup",
        kind: None,
    },
    Main {
        name: "deletegroup",
        usage: "/%s deletegroup <group>",
        node: "luckperms.deletegroup",
        kind: None,
    },
    Main {
        name: "listgroups",
        usage: "/%s listgroups",
        node: "luckperms.listgroups",
        kind: None,
    },
];

/// The main commands of `LuckPerms` that the port does not take.
const UNPORTED_MAIN: [&str; 19] = [
    "track",
    "log",
    "sync",
    "info",
    "editor",
    "verbose",
    "tree",
    "search",
    "networksync",
    "import",
    "export",
    "reloadconfig",
    "bulkupdate",
    "translations",
    "applyedits",
    "trusteditor",
    "createtrack",
    "deletetrack",
    "listtracks",
];

/// An argument of a usage, required or not, with its description.
struct Argument {
    name: &'static str,
    required: bool,
    description: &'static str,
}

impl Argument {
    fn pretty(&self) -> String {
        if self.required {
            format!("<{}>", self.name)
        } else {
            format!("[{}]", self.name)
        }
    }
}

const fn required(name: &'static str, description: &'static str) -> Argument {
    Argument {
        name,
        required: true,
        description,
    }
}

const fn optional(name: &'static str, description: &'static str) -> Argument {
    Argument {
        name,
        required: false,
        description,
    }
}

/// A subcommand of a user or a group: its name, its node below `luckperms.user.` or
/// `luckperms.group.`, the kinds it serves, its description and its arguments.
struct Sub {
    name: &'static str,
    node: &'static str,
    user: bool,
    group: bool,
    description: &'static str,
    arguments: &'static [Argument],
    /// The most words behind the subcommand that `LuckPerms` takes before it answers the usage,
    /// or `None` where it takes any number, as the contexts of a command.
    most: Option<usize>,
}

const fn sub(
    name: &'static str,
    user: bool,
    group: bool,
    description: &'static str,
    arguments: &'static [Argument],
) -> Sub {
    Sub {
        name,
        node: name,
        user,
        group,
        description,
        arguments,
        most: None,
    }
}

/// A subcommand that answers its usage behind more than `most` words.
const fn bounded(
    name: &'static str,
    user: bool,
    group: bool,
    description: &'static str,
    arguments: &'static [Argument],
    most: usize,
) -> Sub {
    Sub {
        name,
        node: name,
        user,
        group,
        description,
        arguments,
        most: Some(most),
    }
}

/// The subcommands of a user and a group that the port takes, in the order of `LuckPerms`, those
/// of `permission` and `parent` with the name of their parent before their own.
const SUBCOMMANDS: [Sub; 15] = [
    Sub {
        name: "info",
        node: "info",
        user: true,
        group: false,
        description: "Shows info about the user",
        arguments: &[],
        most: None,
    },
    Sub {
        name: "info",
        node: "info",
        user: false,
        group: true,
        description: "Gives info about the group",
        arguments: &[],
        most: None,
    },
    bounded(
        "permission info",
        true,
        true,
        "Lists the permission nodes the object has",
        &[optional("page", "the page to view")],
        2,
    ),
    sub(
        "permission set",
        true,
        true,
        "Sets a permission for the object",
        &[
            required("node", "the permission node to set"),
            optional("true|false", "the value of the node"),
        ],
    ),
    sub(
        "permission unset",
        true,
        true,
        "Unsets a permission for the object",
        &[required("node", "the permission node to unset")],
    ),
    sub(
        "permission check",
        true,
        true,
        "Checks to see if the object has a certain permission node",
        &[required("node", "the permission node to check for")],
    ),
    sub(
        "permission clear",
        true,
        true,
        "Clears all permissions",
        &[],
    ),
    bounded(
        "parent info",
        true,
        true,
        "Lists the groups that this object inherits from",
        &[optional("page", "the page to view")],
        2,
    ),
    sub(
        "parent set",
        true,
        true,
        "Removes all other groups the object inherits already and adds them to the one given",
        &[required("group", "the group to set to")],
    ),
    sub(
        "parent add",
        true,
        true,
        "Sets another group for the object to inherit permissions from",
        &[required("group", "the group to inherit from")],
    ),
    sub(
        "parent remove",
        true,
        true,
        "Removes a previously set inheritance rule",
        &[required("group", "the group to remove")],
    ),
    sub("parent clear", true, true, "Clears all parents", &[]),
    bounded(
        "listmembers",
        false,
        true,
        "Show the users/groups who inherit from this group",
        &[optional("page", "the page to view")],
        2,
    ),
    bounded(
        "setweight",
        false,
        true,
        "Set the groups weight",
        &[required("weight", "the weight to set")],
        1,
    ),
    sub(
        "clear",
        true,
        true,
        "Removes all permissions, parents and meta",
        &[],
    ),
];

/// The subcommands of `LuckPerms` for a user that the port does not take.
const UNPORTED_USER_SUB: [&str; 6] = ["meta", "editor", "promote", "demote", "showtracks", "clone"];

/// The subcommands of `LuckPerms` for a group that the port does not take.
const UNPORTED_GROUP_SUB: [&str; 6] = [
    "meta",
    "editor",
    "setdisplayname",
    "showtracks",
    "rename",
    "clone",
];

/// The subcommands of `permission` of `LuckPerms` that the port does not take.
const UNPORTED_PERMISSION_SUB: [&str; 2] = ["settemp", "unsettemp"];

/// The subcommands of `parent` of `LuckPerms` that the port does not take.
const UNPORTED_PARENT_SUB: [&str; 4] = ["settrack", "addtemp", "removetemp", "cleartrack"];

/// The subcommand of `parent` of `LuckPerms` that a user alone has and the port does not take.
const UNPORTED_USER_PARENT_SUB: &str = "switchprimarygroup";

/// What a user or a group subcommand works on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    User,
    Group,
}

impl Kind {
    fn tag(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Group => "group",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::User => "User",
            Self::Group => "Group",
        }
    }

    /// The subcommands of the kind, as the help lists them.
    fn subcommands(self) -> impl Iterator<Item = &'static Sub> {
        SUBCOMMANDS.iter().filter(move |sub| match self {
            Self::User => sub.user,
            Self::Group => sub.group,
        })
    }

    /// The node of a subcommand, such as `luckperms.user.permission.set`.
    fn node(self, sub: &Sub) -> String {
        format!("luckperms.{}.{}", self.tag(), sub.node.replace(' ', "."))
    }

    /// Whether `LuckPerms` has the subcommand `sub`, or the subcommand `child` of `permission` or
    /// `parent`, for this kind, while the port does not take it.
    fn unported(self, sub: &str, child: Option<&str>) -> bool {
        match (sub, child) {
            ("permission", Some(child)) => UNPORTED_PERMISSION_SUB.contains(&child),
            ("parent", Some(child)) => {
                UNPORTED_PARENT_SUB.contains(&child)
                    || (self == Self::User && child == UNPORTED_USER_PARENT_SUB)
            }
            ("permission" | "parent", None) => false,
            _ => match self {
                Self::User => UNPORTED_USER_SUB.contains(&sub),
                Self::Group => UNPORTED_GROUP_SUB.contains(&sub),
            },
        }
    }
}

/// The holder that a user or a group subcommand changes.
#[derive(Clone)]
enum Holder {
    User { uuid: String, display: String },
    Group { name: String },
}

impl Holder {
    fn display(&self) -> &str {
        match self {
            Self::User { display, .. } => display,
            Self::Group { name } => name,
        }
    }
}

/// The state of one command: the configuration it edits, the lines it answers and the entries
/// it changed.
struct Editor<'a> {
    before: &'a Permissions,
    config: Config,
    label: &'a str,
    host: &'a dyn Host,
    lines: Vec<String>,
    changes: Vec<Change>,
    save_error: Option<String>,
}

impl<'a> Editor<'a> {
    fn new(before: &'a Permissions, label: &'a str, host: &'a dyn Host) -> Self {
        Self {
            before,
            config: before.config().clone(),
            label,
            host,
            lines: Vec::new(),
            changes: Vec::new(),
            save_error: None,
        }
    }

    fn say(&mut self, text: &str) {
        self.lines.push(prefixed(text));
    }

    fn denied(&mut self) {
        self.say("You do not have permission to use this command!");
    }

    /// Loads the changed configuration, and answers the outcome: the lines and changes when it
    /// holds every rule, or the refusal of golem-permissions with no change.
    fn finish(self) -> Outcome {
        if self.changes.is_empty() {
            return Outcome::lines(self.lines);
        }
        match Permissions::from_config(self.config) {
            Ok(permissions) => Outcome {
                lines: self.lines,
                changes: self.changes,
                permissions: Some(permissions),
                save_error: self.save_error,
            },
            Err(error) => Outcome::line(prefixed(&format!(
                "The permissions of Golem refuse the change, so nothing changed: {error}."
            ))),
        }
    }

    // The user and group commands.

    fn user(&mut self, words: &[&str]) {
        self.targeted(Kind::User, words);
    }

    fn group(&mut self, words: &[&str]) {
        self.targeted(Kind::Group, words);
    }

    /// Runs `/<label> user <user> <subcommand>` or `/<label> group <group> <subcommand>`.
    fn targeted(&mut self, kind: Kind, words: &[&str]) {
        if words.len() < 2 {
            self.sub_usage(kind);
            return;
        }
        let sub = words[1].to_lowercase();
        let rest = &words[2..];
        let node_sub = rest.first().map(|word| word.to_lowercase());
        let named = match (sub.as_str(), node_sub.as_deref()) {
            ("permission" | "parent", Some(child)) => format!("{sub} {child}"),
            _ => sub.clone(),
        };
        if kind.unported(&sub, node_sub.as_deref()) {
            let path = format!("{} <{}> {named}", kind.tag(), kind.tag());
            self.lines.push(not_ported(self.label, &path));
            return;
        }
        if matches!(sub.as_str(), "permission" | "parent") && node_sub.is_none() {
            self.node_usage(kind, &sub);
            return;
        }
        let Some(spec) = kind.subcommands().find(|spec| spec.name == named) else {
            self.say("Command not recognised.");
            return;
        };
        if !self.host.holds(&kind.node(spec)) {
            self.denied();
            return;
        }
        let arguments: &[&str] = if named.contains(' ') {
            &rest[1..]
        } else {
            rest
        };
        let needed = spec
            .arguments
            .iter()
            .filter(|argument| argument.required)
            .count();
        if arguments.len() < needed || spec.most.is_some_and(|most| arguments.len() > most) {
            self.detailed_usage(spec);
            return;
        }
        let Some(holder) = self.target(kind, words[0]) else {
            return;
        };
        match named.as_str() {
            "info" if kind == Kind::User => self.user_info(&holder),
            "info" => self.group_info(&holder),
            "permission info" => self.permission_info(&holder, arguments),
            "permission set" => self.permission_set(&holder, arguments),
            "permission unset" => self.permission_unset(&holder, arguments),
            "permission check" => self.permission_check(&holder, arguments),
            "permission clear" => self.clear(&holder, arguments, Clear::Permissions),
            "parent info" => self.parent_info(&holder, arguments),
            "parent set" => self.parent_set(&holder, arguments),
            "parent add" => self.parent_add(&holder, arguments),
            "parent remove" => self.parent_remove(&holder, arguments),
            "parent clear" => self.clear(&holder, arguments, Clear::Parents),
            "clear" => self.clear(&holder, arguments, Clear::All),
            "setweight" => self.set_weight(&holder, arguments),
            _ => self.list_members(&holder, arguments),
        }
    }

    /// The holder that a user or a group subcommand names, or `None` with the line that says why.
    fn target(&mut self, kind: Kind, word: &str) -> Option<Holder> {
        match kind {
            Kind::User => {
                if let Some(id) = parse_user_uuid(word) {
                    let display = self.host.name_of(&id).unwrap_or_else(|| id.clone());
                    return Some(Holder::User { uuid: id, display });
                }
                if !valid_username(word) {
                    self.say(&format!("{word} is not a valid username/uuid."));
                    return None;
                }
                let Some(user) = self.host.player_named(word) else {
                    self.say(&format!("A user for {word} could not be found."));
                    return None;
                };
                let display = user.name.clone().unwrap_or_else(|| user.uuid.clone());
                Some(Holder::User {
                    uuid: user.uuid,
                    display,
                })
            }
            Kind::Group => {
                let name = word.to_lowercase();
                if !self.config.groups.contains_key(&name) {
                    self.say(&format!("A group named {name} could not be found."));
                    return None;
                }
                Some(Holder::Group { name })
            }
        }
    }

    /// The usage of a user or a group, `<Kind> Sub Commands: (/<label> user <user> ...)` and a line
    /// per subcommand the sender holds.
    fn sub_usage(&mut self, kind: Kind) {
        let held: Vec<&Sub> = kind
            .subcommands()
            .filter(|sub| self.host.holds(&kind.node(sub)))
            .collect();
        if held.is_empty() {
            self.denied();
            return;
        }
        self.say(&format!(
            "{} Sub Commands: (/{} {} <{}> ...)",
            kind.title(),
            self.label,
            kind.tag(),
            kind.tag()
        ));
        let mut shown: Vec<&str> = Vec::new();
        for sub in held {
            let name = sub.name.split(' ').next().unwrap_or(sub.name);
            if shown.contains(&name) {
                continue;
            }
            shown.push(name);
            // The arguments of permission and parent stand in the usage of their own.
            let arguments: Vec<String> = if name == sub.name {
                sub.arguments.iter().map(Argument::pretty).collect()
            } else {
                Vec::new()
            };
            if arguments.is_empty() {
                self.lines.push(format!("> {name}"));
            } else {
                self.lines
                    .push(format!("> {name} - {}", arguments.join(" ")));
            }
        }
    }

    /// The usage of `permission` or `parent`: `Permission Sub Commands: (...)` and a line per
    /// subcommand the sender holds, with its arguments.
    fn node_usage(&mut self, kind: Kind, sub: &str) {
        let held: Vec<&Sub> = kind
            .subcommands()
            .filter(|spec| spec.name.starts_with(&format!("{sub} ")))
            .filter(|spec| self.host.holds(&kind.node(spec)))
            .collect();
        if held.is_empty() {
            self.denied();
            return;
        }
        let title = if sub == "permission" {
            "Permission"
        } else {
            "Parent"
        };
        self.say(&format!(
            "{title} Sub Commands: (/{} {} <{}> {sub} ...)",
            self.label,
            kind.tag(),
            kind.tag()
        ));
        for spec in held {
            let name = &spec.name[sub.len() + 1..];
            let arguments: Vec<String> = spec.arguments.iter().map(Argument::pretty).collect();
            if arguments.is_empty() {
                self.lines.push(format!("> {name}"));
            } else {
                self.lines
                    .push(format!("> {name} - {}", arguments.join(" ")));
            }
        }
    }

    /// The detailed usage of a subcommand that misses an argument.
    fn detailed_usage(&mut self, spec: &Sub) {
        let name = spec.name.rsplit(' ').next().unwrap_or(spec.name);
        self.say(&format!("Command Usage - {name}"));
        self.say(&format!("> {}", spec.description));
        if !spec.arguments.is_empty() {
            self.say("Arguments:");
            for argument in spec.arguments {
                self.say(&format!(
                    "- {} -> {}",
                    argument.pretty(),
                    argument.description
                ));
            }
        }
    }

    /// The nodes of a holder as the configuration writes them.
    fn nodes_of(&self, holder: &Holder) -> Vec<String> {
        match holder {
            Holder::User { uuid, .. } => self
                .config
                .players
                .get(uuid)
                .map(|player| player.nodes.clone())
                .unwrap_or_default(),
            Holder::Group { name } => self
                .config
                .groups
                .get(name)
                .map(|group| group.nodes.clone())
                .unwrap_or_default(),
        }
    }

    /// The parents of a holder, a user with the group default last, which every player holds.
    fn parents_of(&self, holder: &Holder) -> Vec<String> {
        match holder {
            Holder::User { uuid, .. } => {
                let mut groups: Vec<String> = self
                    .config
                    .players
                    .get(uuid)
                    .map(|player| player.groups.clone())
                    .unwrap_or_default();
                if !groups.iter().any(|group| group == DEFAULT_GROUP) {
                    groups.push(DEFAULT_GROUP.to_owned());
                }
                groups
            }
            Holder::Group { name } => self
                .config
                .groups
                .get(name)
                .map(|group| group.inherits.clone())
                .unwrap_or_default(),
        }
    }

    /// Sets the nodes of a holder and records the change.
    fn set_nodes(&mut self, holder: &Holder, nodes: Vec<String>) {
        match holder {
            Holder::User { uuid, .. } => {
                let player = self.config.players.entry(uuid.clone()).or_default();
                player.nodes = nodes;
                self.changed_player(uuid);
            }
            Holder::Group { name } => {
                if let Some(group) = self.config.groups.get_mut(name) {
                    group.nodes = nodes;
                }
                self.changed_group(name);
            }
        }
    }

    /// Sets the parents of a holder and records the change. A user keeps the group default
    /// implicitly, so it is left out of their list.
    fn set_parents(&mut self, holder: &Holder, mut parents: Vec<String>) {
        match holder {
            Holder::User { uuid, .. } => {
                parents.retain(|group| group != DEFAULT_GROUP);
                let player = self.config.players.entry(uuid.clone()).or_default();
                player.groups = parents;
                self.changed_player(uuid);
            }
            Holder::Group { name } => {
                if let Some(group) = self.config.groups.get_mut(name) {
                    group.inherits = parents;
                }
                self.changed_group(name);
            }
        }
    }

    fn changed_player(&mut self, uuid: &str) {
        let empty = self
            .config
            .players
            .get(uuid)
            .is_some_and(|player| player.groups.is_empty() && player.nodes.is_empty());
        if empty {
            self.config.players.remove(uuid);
        }
        let change = Change::Player(uuid.to_owned());
        if !self.changes.contains(&change) {
            self.changes.push(change);
        }
    }

    fn changed_group(&mut self, name: &str) {
        let change = Change::Group(name.to_owned());
        if !self.changes.contains(&change) {
            self.changes.push(change);
        }
    }

    fn save_error_for(&mut self, holder: &Holder) {
        let line = match holder {
            Holder::User { display, .. } => {
                format!("There was an error whilst saving user data for {display}.")
            }
            Holder::Group { name } => {
                format!("There was an error whilst saving group data for {name}.")
            }
        };
        self.save_error = Some(prefixed(&line));
    }

    fn user_info(&mut self, holder: &Holder) {
        let Holder::User { uuid, .. } = holder else {
            return;
        };
        let name = self
            .host
            .name_of(uuid)
            .unwrap_or_else(|| "Unknown".to_owned());
        self.say(&format!("> User Info: {name}"));
        self.say(&format!("- UUID: {uuid}"));
        self.say(&format!("    (type: {})", uuid_type(uuid)));
        let status = if self.host.online(uuid) {
            "Online"
        } else {
            "Offline"
        };
        self.say(&format!("- Status: {status}"));
        self.parent_lines(holder);
    }

    fn group_info(&mut self, holder: &Holder) {
        let Holder::Group { name } = holder else {
            return;
        };
        let weight = self.config.groups.get(name).map_or(0, |group| group.weight);
        self.say(&format!("> Group Info: {name}"));
        self.say(&format!("- Display Name: {name}"));
        self.say(&format!("- Weight: {weight}"));
        self.parent_lines(holder);
    }

    /// The parents of an info, `- Parent Groups:` and a line per group.
    fn parent_lines(&mut self, holder: &Holder) {
        let parents = self.parents_of(holder);
        if parents.is_empty() {
            return;
        }
        self.say("- Parent Groups:");
        for parent in parents {
            self.say(&format!("    > {parent}"));
        }
    }

    fn permission_info(&mut self, holder: &Holder, arguments: &[&str]) {
        let nodes = self.nodes_of(holder);
        if nodes.is_empty() {
            self.say(&format!(
                "{} does not have any permissions set.",
                holder.display()
            ));
            return;
        }
        let (page, pages, entries) = paged(&nodes, arguments.first().copied(), PAGE);
        self.say(&format!(
            "{}'s Permissions:  (page {page} of {pages} - {} entries)",
            holder.display(),
            nodes.len()
        ));
        for node in entries {
            self.lines.push(format!("> {node}"));
        }
    }

    fn permission_set(&mut self, holder: &Holder, arguments: &[&str]) {
        let node = arguments[0];
        let (value, extra) = match arguments.get(1).map(|word| word.to_ascii_lowercase()) {
            Some(word) if word == "true" => (true, &arguments[2..]),
            Some(word) if word == "false" => (false, &arguments[2..]),
            _ => (true, &arguments[1..]),
        };
        if !extra.is_empty() {
            self.lines.push(no_context(extra));
            return;
        }
        if let Some(group) = inheritance(node) {
            if !value {
                self.say(&format!(
                    "The permissions of Golem have no denied parent, so {} keeps its parents. Use /{} {} <{}> parent remove {group} instead.",
                    holder.display(),
                    self.label,
                    kind_of(holder).tag(),
                    kind_of(holder).tag()
                ));
                return;
            }
            let display = holder.display().to_owned();
            self.add_parent(
                holder,
                &group,
                || format!("Set {node} to true for {display} in context {GLOBAL}."),
                || format!("{display} already has {node} set in context {GLOBAL}."),
            );
            return;
        }
        if let Some(refusal) = meta_refusal(node) {
            self.say(&refusal);
            return;
        }
        if let Err(flaw) = node::parse_rule(node) {
            self.say(&format!(
                "{node} is no node of the permissions of Golem: it {flaw}."
            ));
            return;
        }
        if let Some(denied) = node.strip_prefix('-') {
            self.say(&format!(
                "{node} is no node of the permissions of Golem: it starts with -, which marks a deny, so set {denied} to false instead."
            ));
            return;
        }
        let written = if value {
            node.to_owned()
        } else {
            format!("-{node}")
        };
        let mut nodes = self.nodes_of(holder);
        if nodes.contains(&written) && !nodes.contains(&opposite(&written)) {
            self.say(&format!(
                "{} already has {node} set in context {GLOBAL}.",
                holder.display()
            ));
            return;
        }
        nodes.retain(|entry| entry != node && *entry != format!("-{node}"));
        nodes.push(written);
        self.set_nodes(holder, nodes);
        self.save_error_for(holder);
        self.say(&format!(
            "Set {node} to {value} for {} in context {GLOBAL}.",
            holder.display()
        ));
    }

    fn permission_unset(&mut self, holder: &Holder, arguments: &[&str]) {
        let node = arguments[0];
        if arguments.len() > 1 {
            self.lines.push(no_context(&arguments[1..]));
            return;
        }
        if let Some(group) = inheritance(node) {
            let display = holder.display().to_owned();
            self.remove_parent(
                holder,
                &group,
                || format!("Unset {node} for {display} in context {GLOBAL}."),
                || format!("{display} does not have {node} set in context {GLOBAL}."),
            );
            return;
        }
        let mut nodes = self.nodes_of(holder);
        let before = nodes.len();
        nodes.retain(|entry| entry != node && *entry != format!("-{node}"));
        if nodes.len() == before {
            self.say(&format!(
                "{} does not have {node} set in context {GLOBAL}.",
                holder.display()
            ));
            return;
        }
        self.set_nodes(holder, nodes);
        self.save_error_for(holder);
        self.say(&format!(
            "Unset {node} for {} in context {GLOBAL}.",
            holder.display()
        ));
    }

    fn permission_check(&mut self, holder: &Holder, arguments: &[&str]) {
        let node = arguments[0];
        let display = holder.display().to_owned();
        self.say(&format!("Permission information for {node}:"));
        let own: Vec<String> = self
            .nodes_of(holder)
            .into_iter()
            .filter(|entry| entry.trim_start_matches('-') == node)
            .collect();
        if own.is_empty() {
            self.say(&format!("- {display} does not have {node} set."));
        }
        for entry in &own {
            self.say(&format!(
                "- {display} has {node} set to {} in context {GLOBAL}.",
                !entry.starts_with('-')
            ));
        }
        let inherited = self.inherited(holder);
        let exact: Vec<&(String, String)> = inherited
            .iter()
            .filter(|(_, entry)| entry.trim_start_matches('-') == node)
            .collect();
        if exact.is_empty() {
            self.say(&format!("- {display} does not inherit {node}."));
        }
        for (group, entry) in &exact {
            self.say(&format!(
                "- {display} inherits {node} set to {} from {group} in context {GLOBAL}.",
                !entry.starts_with('-')
            ));
        }
        for entry in self.nodes_of(holder) {
            if wildcard_matches(&entry, node) {
                self.say(&format!(
                    "- {display} has {} set to {} in context {GLOBAL}.",
                    entry.trim_start_matches('-'),
                    !entry.starts_with('-')
                ));
            }
        }
        for (group, entry) in &inherited {
            if wildcard_matches(entry, node) {
                self.say(&format!(
                    "- {display} inherits {} set to {} from {group} in context {GLOBAL}.",
                    entry.trim_start_matches('-'),
                    !entry.starts_with('-')
                ));
            }
        }
        self.lines.push(PREFIX.trim_end().to_owned());
        let (result, cause) = match holder {
            Holder::User { uuid, .. } => {
                let decision = self.before.check(uuid, node);
                (decision.value, decision.reason)
            }
            Holder::Group { name } => self.group_decision(name, node),
        };
        let result = match result {
            Some(true) => "true",
            Some(false) => "false",
            None => "undefined",
        };
        self.say(&format!("Permission check for {node}:"));
        self.say(&format!("    Result: {result}"));
        self.say("    Processor: golem-permissions");
        self.say(&format!("    Cause: {cause}"));
        self.say(&format!("    Context: {GLOBAL}"));
    }

    /// The decision for a group, as for a player who holds that group alone and no node of
    /// their own. `LuckPerms` checks a group by itself and its parents, so the group default,
    /// which every player holds, counts only where the group inherits from it.
    fn group_decision(&self, name: &str, node: &str) -> (Option<bool>, String) {
        let mut config = self.before.config().clone();
        let mut reached = vec![name.to_owned()];
        let mut next = 0;
        while let Some(group) = reached.get(next).cloned() {
            next += 1;
            for parent in config
                .groups
                .get(&group)
                .map_or(&[][..], |entry| &entry.inherits)
            {
                if !reached.contains(parent) {
                    reached.push(parent.clone());
                }
            }
        }
        if !reached.iter().any(|group| group == DEFAULT_GROUP)
            && let Some(default) = config.groups.get_mut(DEFAULT_GROUP)
        {
            default.nodes.clear();
            default.inherits.clear();
        }
        let probe = "00000000-0000-0000-0000-000000000000".to_owned();
        config.players.insert(
            probe.clone(),
            PlayerConfig {
                groups: vec![name.to_owned()],
                nodes: Vec::new(),
            },
        );
        match Permissions::from_config(config) {
            Ok(permissions) => {
                let decision = permissions.check(&probe, node);
                (decision.value, decision.reason)
            }
            Err(error) => (None, error.message),
        }
    }

    /// The nodes that a holder inherits from its groups, each with the group that holds it, in
    /// the order a check searches the groups.
    fn inherited(&self, holder: &Holder) -> Vec<(String, String)> {
        let mut groups = Vec::new();
        let mut queue = self.parents_of(holder);
        while !queue.is_empty() {
            let group = queue.remove(0);
            if groups.contains(&group) {
                continue;
            }
            if let Some(entry) = self.config.groups.get(&group) {
                queue.extend(entry.inherits.iter().cloned());
            }
            groups.push(group);
        }
        groups
            .into_iter()
            .flat_map(|group| {
                let nodes = self
                    .config
                    .groups
                    .get(&group)
                    .map(|entry| entry.nodes.clone())
                    .unwrap_or_default();
                nodes.into_iter().map(move |node| (group.clone(), node))
            })
            .collect()
    }

    fn parent_info(&mut self, holder: &Holder, arguments: &[&str]) {
        let parents = self.parents_of(holder);
        if parents.is_empty() {
            self.say(&format!(
                "{} does not have any parents defined.",
                holder.display()
            ));
            return;
        }
        let (page, pages, entries) = paged(&parents, arguments.first().copied(), PAGE);
        self.say(&format!(
            "{}'s Parents:  (page {page} of {pages} - {} entries)",
            holder.display(),
            parents.len()
        ));
        for parent in entries {
            self.lines.push(format!("> {parent}"));
        }
    }

    /// The group that a parent subcommand names, lower case, or `None` with the line that says why.
    fn parent_group(&mut self, word: &str) -> Option<String> {
        let group = word.to_lowercase();
        if !self.config.groups.contains_key(&group) {
            self.say(&format!("A group named {group} could not be found."));
            return None;
        }
        Some(group)
    }

    fn parent_set(&mut self, holder: &Holder, arguments: &[&str]) {
        if arguments.len() > 1 {
            self.lines.push(no_context(&arguments[1..]));
            return;
        }
        let Some(group) = self.parent_group(arguments[0]) else {
            return;
        };
        self.set_parents(holder, vec![group.clone()]);
        self.save_error_for(holder);
        self.say(&format!(
            "{} had their existing parent groups cleared, and now only inherits {group} in context {GLOBAL}.",
            holder.display()
        ));
    }

    fn parent_add(&mut self, holder: &Holder, arguments: &[&str]) {
        if arguments.len() > 1 {
            self.lines.push(no_context(&arguments[1..]));
            return;
        }
        let Some(group) = self.parent_group(arguments[0]) else {
            return;
        };
        let display = holder.display().to_owned();
        self.add_parent(
            holder,
            &group,
            || format!("{display} now inherits permissions from {group} in context {GLOBAL}."),
            || format!("{display} already inherits from {group} in context {GLOBAL}."),
        );
    }

    /// Adds `group` to the parents of a holder and answers `success`, or `held` when the holder
    /// inherits it already.
    fn add_parent(
        &mut self,
        holder: &Holder,
        group: &str,
        success: impl FnOnce() -> String,
        held: impl FnOnce() -> String,
    ) {
        if !self.config.groups.contains_key(group) {
            self.say(&format!("A group named {group} could not be found."));
            return;
        }
        let mut parents = self.parents_of(holder);
        if parents.iter().any(|parent| parent == group) {
            let line = held();
            self.say(&line);
            return;
        }
        if let Holder::User { .. } = holder {
            parents.retain(|parent| parent != DEFAULT_GROUP);
        }
        parents.push(group.to_owned());
        self.set_parents(holder, parents);
        self.save_error_for(holder);
        let line = success();
        self.say(&line);
    }

    fn parent_remove(&mut self, holder: &Holder, arguments: &[&str]) {
        if arguments.len() > 1 {
            self.lines.push(no_context(&arguments[1..]));
            return;
        }
        let group = arguments[0].to_lowercase();
        let display = holder.display().to_owned();
        self.remove_parent(
            holder,
            &group,
            || {
                format!(
                    "{display} no longer inherits permissions from {group} in context {GLOBAL}."
                )
            },
            || format!("{display} does not inherit from {group} in context {GLOBAL}."),
        );
    }

    /// Removes `group` from the parents of a holder and answers `success`, or `missing` when
    /// the holder does not inherit it. Every player keeps the group default.
    fn remove_parent(
        &mut self,
        holder: &Holder,
        group: &str,
        success: impl FnOnce() -> String,
        missing: impl FnOnce() -> String,
    ) {
        if matches!(holder, Holder::User { .. }) && group == DEFAULT_GROUP {
            self.say(&format!(
                "Every player inherits from {DEFAULT_GROUP} in Golem, so {} keeps it.",
                holder.display()
            ));
            return;
        }
        let mut parents = self.parents_of(holder);
        let before = parents.len();
        parents.retain(|parent| parent != group);
        if parents.len() == before {
            let line = missing();
            self.say(&line);
            return;
        }
        self.set_parents(holder, parents);
        self.save_error_for(holder);
        let line = success();
        self.say(&line);
    }

    fn clear(&mut self, holder: &Holder, arguments: &[&str], what: Clear) {
        if !arguments.is_empty() {
            self.lines.push(no_context(arguments));
            return;
        }
        let nodes = self.nodes_of(holder).len();
        let parents = match holder {
            Holder::User { uuid, .. } => self
                .config
                .players
                .get(uuid)
                .map_or(0, |player| player.groups.len()),
            Holder::Group { name } => self
                .config
                .groups
                .get(name)
                .map_or(0, |group| group.inherits.len()),
        };
        let removed = match what {
            Clear::Permissions => nodes,
            Clear::Parents => parents,
            Clear::All => nodes + parents,
        };
        if removed > 0 {
            if what != Clear::Parents {
                self.set_nodes(holder, Vec::new());
            }
            if what != Clear::Permissions {
                self.set_parents(holder, Vec::new());
            }
            self.save_error_for(holder);
        }
        let kind = match what {
            Clear::Permissions => "permissions",
            Clear::Parents => "parents",
            Clear::All => "nodes",
        };
        let count = if removed == 1 {
            "1 node was removed".to_owned()
        } else {
            format!("{removed} nodes were removed")
        };
        self.say(&format!(
            "{}'s {kind} were cleared in context {GLOBAL}. ({count}.)",
            holder.display()
        ));
    }

    fn set_weight(&mut self, holder: &Holder, arguments: &[&str]) {
        let Holder::Group { name } = holder else {
            return;
        };
        let Ok(weight) = arguments[0].parse::<i32>() else {
            self.say(&format!(
                "Invalid priority {}. Expected a number.",
                arguments[0]
            ));
            return;
        };
        if let Some(group) = self.config.groups.get_mut(name) {
            group.weight = weight;
        }
        self.changed_group(name);
        self.save_error_for(holder);
        self.say(&format!("Set weight to {weight} for group {name}."));
    }

    fn list_members(&mut self, holder: &Holder, arguments: &[&str]) {
        let Holder::Group { name } = holder else {
            return;
        };
        if arguments.len() > 1 {
            self.lines.push(no_context(&arguments[1..]));
            return;
        }
        self.say(&format!(
            "Searching for users and groups who inherit from {name}..."
        ));
        // Every player is in the group default, so its members are every player the
        // configuration names. LuckPerms lists the users in the order of their UUIDs.
        let mut members: Vec<&String> = self
            .config
            .players
            .iter()
            .filter(|(_, player)| {
                name == DEFAULT_GROUP || player.groups.iter().any(|group| group == name)
            })
            .map(|(uuid, _)| uuid)
            .collect();
        members.sort_by_key(|uuid| java_uuid_order(uuid));
        let users: Vec<String> = members
            .into_iter()
            .map(|uuid| self.host.name_of(uuid).unwrap_or_else(|| uuid.clone()))
            .collect();
        if name == DEFAULT_GROUP {
            self.say("Note: when searching for members of the default group, offline players with no other permissions will not be shown!");
        }
        let groups: Vec<String> = self
            .config
            .groups
            .iter()
            .filter(|(_, group)| group.inherits.iter().any(|parent| parent == name))
            .map(|(group, _)| group.clone())
            .collect();
        self.say(&format!(
            "Found {} entries from {} users and {} groups.",
            users.len() + groups.len(),
            users.len(),
            groups.len()
        ));
        let page = arguments.first().copied();
        for (title, entries) in [("user", &users), ("group", &groups)] {
            if entries.is_empty() {
                continue;
            }
            let (shown, pages, content) = paged(entries, page, MEMBER_PAGE);
            self.say(&format!(
                "Showing {title} entries:    (page {shown} of {pages} - {} entries)",
                entries.len()
            ));
            for entry in content {
                self.lines.push(format!("> {entry}"));
            }
        }
    }

    // The commands without a target.

    fn create_group(&mut self, words: &[&str]) {
        let ([first] | [first, _] | [first, _, _]) = words else {
            self.main_usage(
                "CreateGroup",
                "Create a new group",
                &[
                    required("name", "the name of the group"),
                    optional("weight", "the weight of the group"),
                ],
            );
            return;
        };
        let name = first.to_lowercase();
        if node::check_name(&name).is_err() {
            self.say(&format!("{name} is not a valid group name."));
            return;
        }
        if self.config.groups.contains_key(&name) {
            self.say(&format!("{name} already exists!."));
            return;
        }
        let weight = match words.get(1) {
            None => 0,
            Some(word) => {
                let Ok(weight) = word.parse::<i32>() else {
                    self.say(&format!(
                        "The groups of Golem have no display name, so the port takes no {word}."
                    ));
                    return;
                };
                weight
            }
        };
        if let Some(extra) = words.get(2) {
            self.say(&format!(
                "The groups of Golem have no display name, so the port takes no {extra}."
            ));
            return;
        }
        self.config.groups.insert(
            name.clone(),
            GroupConfig {
                weight,
                ..GroupConfig::default()
            },
        );
        self.changed_group(&name);
        self.save_error = Some(prefixed(&format!(
            "There was an error whilst creating {name}."
        )));
        self.say(&format!("{name} was successfully created."));
    }

    fn delete_group(&mut self, words: &[&str]) {
        // LuckPerms takes the name and one more word, its flag --update-parent-lists, which
        // asks for what the port always does.
        let ([first] | [first, _]) = words else {
            self.main_usage(
                "DeleteGroup",
                "Delete a group",
                &[required("name", "the name of the group")],
            );
            return;
        };
        let name = first.to_lowercase();
        if name == DEFAULT_GROUP {
            self.say("You cannot delete the default group.");
            return;
        }
        if !self.config.groups.contains_key(&name) {
            self.say("An unexpected error occurred. Group not loaded.");
            return;
        }
        // LuckPerms leaves a node that names a deleted group, which then grants nothing. The store
        // keeps no such row, so every entry that names the group loses it first.
        let groups: Vec<String> = self
            .config
            .groups
            .iter()
            .filter(|(_, group)| group.inherits.contains(&name))
            .map(|(other, _)| other.clone())
            .collect();
        for other in groups {
            if let Some(group) = self.config.groups.get_mut(&other) {
                group.inherits.retain(|parent| *parent != name);
            }
            self.changed_group(&other);
        }
        let players: Vec<String> = self
            .config
            .players
            .iter()
            .filter(|(_, player)| player.groups.contains(&name))
            .map(|(uuid, _)| uuid.clone())
            .collect();
        for uuid in players {
            if let Some(player) = self.config.players.get_mut(&uuid) {
                player.groups.retain(|group| *group != name);
            }
            self.changed_player(&uuid);
        }
        self.config.groups.remove(&name);
        self.changes.push(Change::RemovedGroup(name.clone()));
        self.save_error = Some(prefixed(&format!(
            "There was an error whilst deleting {name}."
        )));
        self.say(&format!("{name} was successfully deleted."));
    }

    fn list_groups(&mut self, words: &[&str]) {
        if words.len() > 1 {
            self.main_usage(
                "ListGroups",
                "List all groups on the platform",
                &[optional("page", "the page to view")],
            );
            return;
        }
        let mut groups: Vec<(&String, &GroupConfig)> = self.config.groups.iter().collect();
        groups.sort_by(|(a, one), (b, other)| {
            other
                .weight
                .cmp(&one.weight)
                .then_with(|| a.to_lowercase().cmp(&b.to_lowercase()))
        });
        let entries: Vec<String> = groups
            .iter()
            .map(|(name, group)| format!("-  {name} - {}", group.weight))
            .collect();
        let (page, pages, content) = paged(&entries, words.first().copied(), GROUP_PAGE);
        self.say(&format!(
            "Showing group entries:    (page {page} of {pages} - {} entries)",
            entries.len()
        ));
        self.say("Groups: (name, weight, tracks)");
        for entry in content {
            self.say(entry);
        }
    }

    /// The detailed usage of a main command behind too few or too many words, under the name
    /// that `LuckPerms` gives the command.
    fn main_usage(&mut self, name: &str, description: &str, arguments: &[Argument]) {
        self.say(&format!("Command Usage - {name}"));
        self.say(&format!("> {description}"));
        self.say("Arguments:");
        for argument in arguments {
            self.say(&format!(
                "- {} -> {}",
                argument.pretty(),
                argument.description
            ));
        }
    }
}

/// What a clear removes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Clear {
    Permissions,
    Parents,
    All,
}

fn kind_of(holder: &Holder) -> Kind {
    match holder {
        Holder::User { .. } => Kind::User,
        Holder::Group { .. } => Kind::Group,
    }
}

/// The answer of `/<label>` alone: the version and the way to the help, or why the sender may use
/// nothing.
fn running(permissions: &Permissions, label: &str, host: &dyn Host) -> Outcome {
    let mut lines = vec![running_line()];
    if MAIN.iter().any(|main| main.authorized(host)) {
        lines.push(prefixed(&format!(
            "Use /{label} help to view available commands."
        )));
        return Outcome::lines(lines);
    }
    let config = permissions.config();
    let unset =
        config.groups.len() <= 1 && config.groups.values().all(|group| group.nodes.is_empty());
    if unset {
        let name = host.sender_name();
        lines.extend([
            prefixed("It seems that no permissions have been setup yet!"),
            prefixed("Before you can use any of the LuckPerms commands in-game, you need to use the console to give yourself access."),
            prefixed("Open your console and run:"),
            prefixed(&format!(" > {label} user {name} permission set luckperms.* true")),
            prefixed("After you've done this, you can begin to define your permission assignments and groups."),
            prefixed("Don't know where to start? Check here: https://luckperms.net/wiki/Usage"),
        ]);
    } else {
        lines.push(prefixed(
            "You do not have permission to use any sub commands.",
        ));
    }
    Outcome::lines(lines)
}

fn running_line() -> String {
    prefixed(&format!(
        "Running LuckPerms v{VERSION}, as ported to Golem."
    ))
}

/// The usage of the main commands that the sender holds, which `LuckPerms` answers for an unknown
/// command or one the sender does not hold.
fn command_usage(label: &str, host: &dyn Host) -> Vec<String> {
    let mut lines = vec![running_line()];
    for main in MAIN.iter().filter(|main| main.authorized(host)) {
        lines.push(format!("> {}", main.usage.replace("%s", label)));
    }
    lines
}

/// The page that a word names of `entries`, the number of pages and the entries of that page,
/// where a word that is no page in range names the first, as in `LuckPerms`.
fn paged<'e, T>(entries: &'e [T], word: Option<&str>, size: usize) -> (usize, usize, &'e [T]) {
    let pages = entries.len().div_ceil(size).max(1);
    let page = word
        .and_then(|word| word.parse::<usize>().ok())
        .filter(|page| (1..=pages).contains(page))
        .unwrap_or(1);
    let start = (page - 1) * size;
    let end = (start + size).min(entries.len());
    (page, pages, &entries[start..end])
}

/// The group that a node `group.<name>` names, as `LuckPerms` reads such a node as a parent.
fn inheritance(node: &str) -> Option<String> {
    let rest = node
        .get(..6)?
        .eq_ignore_ascii_case("group.")
        .then(|| &node[6..])?;
    (!rest.is_empty()).then(|| rest.to_ascii_lowercase())
}

/// The refusal of a node that `LuckPerms` reads as meta, which Golem does not hold.
fn meta_refusal(node: &str) -> Option<String> {
    let lower = node.to_ascii_lowercase();
    ["weight.", "prefix.", "suffix.", "meta.", "displayname."]
        .iter()
        .find(|marker| lower.starts_with(*marker))
        .map(|marker| {
            format!(
                "{node} is a node of meta in LuckPerms, {}, which the permissions of Golem do not hold.",
                marker.trim_end_matches('.')
            )
        })
}

/// The other value of a node as the configuration writes it.
fn opposite(written: &str) -> String {
    written
        .strip_prefix('-')
        .map_or_else(|| format!("-{written}"), str::to_owned)
}

/// Whether a wildcard of the configuration, a deny with its `-`, names `node`.
fn wildcard_matches(entry: &str, node: &str) -> bool {
    let pattern = entry.trim_start_matches('-');
    if pattern == "*" {
        return true;
    }
    pattern.strip_suffix(".*").is_some_and(|prefix| {
        node.len() > prefix.len() + 1
            && node.starts_with(prefix)
            && node.as_bytes()[prefix.len()] == b'.'
    })
}

/// The UUID that a user argument names, in lower case with its dashes, also from the 32 digits
/// without dashes that `LuckPerms` takes.
fn parse_user_uuid(word: &str) -> Option<String> {
    let digits: String = match word.len() {
        36 => {
            node::parse_uuid(word)?;
            word.chars().filter(|c| *c != '-').collect()
        }
        32 if word.bytes().all(|byte| byte.is_ascii_hexdigit()) => word.to_owned(),
        _ => return None,
    };
    let digits = digits.to_ascii_lowercase();
    Some(format!(
        "{}-{}-{}-{}-{}",
        &digits[0..8],
        &digits[8..12],
        &digits[12..16],
        &digits[16..20],
        &digits[20..32]
    ))
}

/// Whether a word is a name of a player as `LuckPerms` takes it, 1 to 16 of `A` to `Z`, `a` to `z`,
/// `0` to `9` and `_`.
fn valid_username(word: &str) -> bool {
    (1..=16).contains(&word.len())
        && word
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// The place of a UUID in the order of `UUID.compareTo` of Java, which compares the two halves
/// as signed numbers, the first half first. A text that is no UUID sorts as the UUID of zeros,
/// and no key of the configuration is such a text.
fn java_uuid_order(uuid: &str) -> (i64, i64) {
    let digits: String = uuid.chars().filter(|c| *c != '-').collect();
    let bytes = u128::from_str_radix(&digits, 16).unwrap_or(0).to_be_bytes();
    let half = |bytes: &[u8]| i64::from_be_bytes(bytes.try_into().unwrap_or([0; 8]));
    (half(&bytes[..8]), half(&bytes[8..]))
}

/// The kind of a UUID as `LuckPerms` names it in the info of a user.
fn uuid_type(uuid: &str) -> &'static str {
    match uuid.as_bytes().get(14) {
        Some(b'4') => "official",
        Some(b'3') => "offline",
        Some(b'2') => "npc",
        _ => "unknown",
    }
}
