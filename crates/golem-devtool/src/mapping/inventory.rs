// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The inventory of the reference of the API: every class of the jar of spigot-api but the
 * package-info of each package, with its line and its public members as javap -public prints
 * them, and with the names of the parameters of its methods from the sources jar. The file is
 * JSON in a layout of its own, one member per line, so that a change of the reference reads as
 * a diff of lines.
 */

//! The inventory of the reference: its model, its writer, its reader and its numbers.

use std::{
    collections::{
        BTreeMap,
        BTreeSet, //
    },
    fs,
    path::Path, //
};

use serde::Deserialize;
use serde_json::{
    Value,
    json, //
};

use super::{
    archive::Archive,
    javap,
    sha256,
    sources, //
};

/// The command that writes the inventory, which its first line names.
pub const WRITTEN_BY: &str = "golem-devtool api inventory";

/// The class that every event of Bukkit extends, and the interface of every entity.
const EVENT: &str = "org.bukkit.event.Event";
const ENTITY: &str = "org.bukkit.entity.Entity";
/// The interface of an event that a listener may cancel.
const CANCELLABLE: &str = "org.bukkit.event.Cancellable";
/// The line of the static method that gives an event class a handler list of its own.
const HANDLER_LIST: &str = "public static org.bukkit.event.HandlerList getHandlerList()";

/// The inventory of one jar.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Inventory {
    /// The command that wrote the file.
    pub generated_by: String,
    /// The name of the jar, which carries the version of the reference.
    pub jar: String,
    /// The SHA-256 of the jar.
    pub jar_sha256: String,
    /// The name of the sources jar that gave the names of the parameters.
    pub sources: String,
    /// The SHA-256 of the sources jar.
    pub sources_sha256: String,
    /// The types, in the order of the bytes of their names.
    pub types: Vec<Type>,
}

/// One class or interface of the jar.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Type {
    /// The binary name, such as `org.bukkit.entity.Player$Spigot`.
    pub name: String,
    /// The line of the type as javap prints it.
    pub line: String,
    /// `class` or `interface`.
    pub kind: String,
    /// The modifiers of the line.
    pub modifiers: Vec<String>,
    /// The types after `extends` of the line.
    pub extends: Vec<String>,
    /// The types after `implements` of the line.
    pub implements: Vec<String>,
    /// The lines of the public fields and constants.
    pub fields: Vec<String>,
    /// The public methods and constructors.
    pub methods: Vec<Method>,
}

/// One method or constructor of a type.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Method {
    /// The line of the method as javap prints it.
    pub line: String,
    /// The names of its parameters in their order, or `None` where the sources jar declares
    /// no such method, as for a method that the compiler adds.
    pub names: Option<Vec<String>>,
}

/// What the writing of an inventory counted beside the inventory itself.
pub struct Written {
    /// The classes of the jar.
    pub classes: usize,
    /// The classes named `package-info`, which the inventory leaves out.
    pub left_out: usize,
}

/// The package of a binary name.
pub fn package(name: &str) -> &str {
    name.rsplit_once('.').map_or("", |(package, _)| package)
}

/// The name of a type without its package, with its outer types where it has some.
pub fn simple(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(_, simple)| simple)
}

/// The name of a type without its type arguments.
fn erased(name: &str) -> &str {
    name.split('<').next().unwrap_or(name)
}

/// Whether a binary name is the one of an anonymous class, whose last part is a number.
pub fn anonymous(name: &str) -> bool {
    name.rsplit_once('$')
        .is_some_and(|(_, last)| !last.is_empty() && last.bytes().all(|byte| byte.is_ascii_digit()))
}

/// The name of a file as a JSON string.
fn file_name(path: &Path) -> Result<String, String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("{}: no name of a file", path.display()))
}

/// `text` as a JSON string.
fn quoted(text: &str) -> String {
    Value::from(text).to_string()
}

/// `items` as a JSON list on one line.
fn listed(items: &[String]) -> String {
    let items: Vec<String> = items.iter().map(|item| quoted(item)).collect();
    format!("[{}]", items.join(", "))
}

impl Inventory {
    /// Reads the public members of every class of `jar` with `javap`, and the names of the
    /// parameters from `sources`.
    pub fn build(jar: &Path, sources: &Path, javap: &Path) -> Result<(Self, Written), String> {
        let classes_of_jar = Archive::open(jar)?;
        let mut classes: Vec<String> = classes_of_jar
            .names()
            .filter_map(|name| name.strip_suffix(".class"))
            .map(|name| name.replace('/', "."))
            .collect();
        let all = classes.len();
        classes.retain(|name| !name.ends_with(".package-info"));
        classes.sort();
        let java = Archive::open(sources)?;
        let mut files: BTreeMap<String, Option<sources::Declarations>> = BTreeMap::new();
        let mut types = Vec::with_capacity(classes.len());
        for (class, printed) in classes.iter().zip(javap::run(javap, jar, &classes)?) {
            let head = javap::head(&printed.line)?;
            if head.name != *class {
                return Err(format!(
                    "javap printed {} where {class} was asked",
                    head.name
                ));
            }
            let file = format!("{}/{}", package(class).replace('.', "/"), printed.source);
            if !files.contains_key(&file) {
                let text = java
                    .read(&file)?
                    .map(String::from_utf8)
                    .transpose()
                    .map_err(|_| format!("{file}: no UTF-8"))?;
                files.insert(file.clone(), text.map(|text| sources::declarations(&text)));
            }
            let declared = files.get(&file).and_then(Option::as_ref);
            let path: Vec<String> = simple(class).split('$').map(str::to_owned).collect();
            let mut fields = Vec::new();
            let mut methods = Vec::new();
            for line in printed.members {
                let Some((name, parameters)) = javap::method(&line) else {
                    fields.push(line);
                    continue;
                };
                let names = if parameters.is_empty() {
                    Some(Vec::new())
                } else {
                    // javap prints a constructor under the name of its class.
                    let name = if name == class {
                        path.last().map_or(name, String::as_str)
                    } else {
                        name
                    };
                    declared.and_then(|declared| sources::names(declared, &path, name, &parameters))
                };
                methods.push(Method { line, names });
            }
            types.push(Type {
                name: head.name,
                line: printed.line,
                kind: head.kind,
                modifiers: head.modifiers,
                extends: head.extends,
                implements: head.implements,
                fields,
                methods,
            });
        }
        let inventory = Self {
            generated_by: WRITTEN_BY.to_owned(),
            jar: file_name(jar)?,
            jar_sha256: sha256::hex(classes_of_jar.bytes()),
            sources: file_name(sources)?,
            sources_sha256: sha256::hex(java.bytes()),
            types,
        };
        let written = Written {
            classes: all,
            left_out: all - classes.len(),
        };
        Ok((inventory, written))
    }

    /// Reads the inventory at `path`.
    pub fn read(path: &Path) -> Result<Self, String> {
        let text =
            fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let inventory: Self =
            serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        if inventory.generated_by != WRITTEN_BY {
            return Err(format!(
                "{}: written by {}, not by {WRITTEN_BY}",
                path.display(),
                inventory.generated_by
            ));
        }
        // No type of Java extends itself, so a file that says so is no inventory of a jar.
        let index = Index::new(&inventory);
        if let Some(kind) = inventory
            .types
            .iter()
            .find(|kind| index.ancestors(&kind.name).contains(kind.name.as_str()))
        {
            return Err(format!(
                "{}: {} stands among the types that it extends or implements",
                path.display(),
                kind.name
            ));
        }
        Ok(inventory)
    }

    /// The text of the file: JSON whose first line names the command that wrote it, with one
    /// line per field and per method.
    pub fn text(&self) -> String {
        let mut lines = vec![
            format!("{{ \"generated-by\": {},", quoted(&self.generated_by)),
            format!("  \"jar\": {},", quoted(&self.jar)),
            format!("  \"jar-sha256\": {},", quoted(&self.jar_sha256)),
            format!("  \"sources\": {},", quoted(&self.sources)),
            format!("  \"sources-sha256\": {},", quoted(&self.sources_sha256)),
            String::from("  \"types\": ["),
        ];
        for (index, kind) in self.types.iter().enumerate() {
            lines.push(String::from("    {"));
            lines.push(format!("      \"name\": {},", quoted(&kind.name)));
            lines.push(format!("      \"line\": {},", quoted(&kind.line)));
            lines.push(format!("      \"kind\": {},", quoted(&kind.kind)));
            lines.push(format!("      \"modifiers\": {},", listed(&kind.modifiers)));
            lines.push(format!("      \"extends\": {},", listed(&kind.extends)));
            lines.push(format!(
                "      \"implements\": {},",
                listed(&kind.implements)
            ));
            let fields: Vec<String> = kind.fields.iter().map(|line| quoted(line)).collect();
            lines.push(block("fields", &fields, ","));
            let methods: Vec<String> = kind
                .methods
                .iter()
                .map(|method| {
                    let names = method
                        .names
                        .as_ref()
                        .map_or_else(|| String::from("null"), |names| listed(names));
                    format!("{{\"line\": {}, \"names\": {names}}}", quoted(&method.line))
                })
                .collect();
            lines.push(block("methods", &methods, ""));
            let last = index + 1 == self.types.len();
            lines.push(String::from(if last { "    }" } else { "    }," }));
        }
        lines.push(String::from("  ]"));
        lines.push(String::from("}"));
        lines.join("\n") + "\n"
    }

    /// How many types `keep` holds.
    fn count(&self, keep: &dyn Fn(&Type) -> bool) -> usize {
        self.types.iter().filter(|kind| keep(kind)).count()
    }

    /// How many methods the types have that `keep` holds.
    fn methods(&self, keep: &dyn Fn(&Type) -> bool) -> usize {
        self.types
            .iter()
            .filter(|kind| keep(kind))
            .map(|kind| kind.methods.len())
            .sum()
    }

    /// The numbers of the inventory that the pages of the wiki name.
    pub fn numbers(&self) -> Value {
        let index = Index::new(self);
        let methods = self.methods(&|_| true);
        let fields: usize = self.types.iter().map(|kind| kind.fields.len()).sum();
        let packages: BTreeSet<&str> = self.types.iter().map(|kind| package(&kind.name)).collect();
        let mut homes: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for kind in &self.types {
            homes
                .entry(simple(&kind.name))
                .or_default()
                .insert(package(&kind.name));
        }
        let with_parameters = |method: &&Method| {
            javap::method(&method.line).is_some_and(|(_, list)| !list.is_empty())
        };
        let asked: Vec<&Method> = self
            .types
            .iter()
            .flat_map(|kind| &kind.methods)
            .filter(with_parameters)
            .collect();
        json!({
            "jar": self.jar,
            "jarSha256": self.jar_sha256,
            "sources": self.sources,
            "sourcesSha256": self.sources_sha256,
            "types": self.types.len(),
            "methods": methods,
            "fields": fields,
            "members": self.types.len() + methods + fields,
            "classes": self.count(&|kind| kind.kind == "class"),
            "interfaces": self.count(&|kind| kind.kind == "interface"),
            "public": self.count(&|kind| kind.modifiers.iter().any(|modifier| modifier == "public")),
            "anonymous": {
                "types": self.count(&|kind| anonymous(&kind.name)),
                "methods": self.methods(&|kind| anonymous(&kind.name)),
            },
            "packages": packages.len(),
            "orgBukkit": self.count(&|kind| package(&kind.name) == "org.bukkit"),
            "sharedNames": homes.values().filter(|packages| packages.len() > 1).count(),
            "entity": self.entity_numbers(&index),
            "events": self.event_numbers(&index),
            "names": {
                "asked": asked.len(),
                "named": asked.iter().filter(|method| method.names.is_some()).count(),
            },
        })
    }

    /// The numbers of the types of `org.bukkit.entity` and its subpackages.
    fn entity_numbers(&self, index: &Index<'_>) -> Value {
        let of_entities = |kind: &Type| {
            let home = package(&kind.name);
            home == "org.bukkit.entity" || home.starts_with("org.bukkit.entity.")
        };
        let interface = |kind: &Type| of_entities(kind) && kind.kind == "interface";
        let tree: BTreeSet<&str> = self
            .types
            .iter()
            .filter(|kind| interface(kind))
            .filter(|kind| kind.name == ENTITY || index.ancestors(&kind.name).contains(ENTITY))
            .map(|kind| kind.name.as_str())
            .collect();
        // A leaf is an interface of the tree that no other interface of the tree extends.
        let parents: BTreeSet<&str> = tree
            .iter()
            .flat_map(|name| index.ancestors(name))
            .filter(|name| tree.contains(name))
            .collect();
        json!({
            "types": self.count(&of_entities),
            "methods": self.methods(&of_entities),
            "interfaces": self.count(&interface),
            "interfaceMethods": self.methods(&interface),
            "tree": tree.len(),
            "leaves": tree.len() - parents.len(),
        })
    }

    /// The numbers of the event classes, `Event` itself included.
    fn event_numbers(&self, index: &Index<'_>) -> Value {
        let events: Vec<&Type> = self
            .types
            .iter()
            .filter(|kind| kind.name == EVENT || index.ancestors(&kind.name).contains(EVENT))
            .collect();
        let concrete: Vec<&Type> = events
            .iter()
            .filter(|kind| !kind.modifiers.iter().any(|modifier| modifier == "abstract"))
            .copied()
            .collect();
        let cancellable = concrete
            .iter()
            .filter(|kind| index.ancestors(&kind.name).contains(CANCELLABLE))
            .count();
        let listenable = events
            .iter()
            .filter(|kind| index.chain(&kind.name).is_some())
            .count();
        let own_list = events
            .iter()
            .filter(|kind| index.chain(&kind.name) == Some(kind.name.as_str()))
            .count();
        // A setter of an event is a method whose name starts with `set` and a capital letter.
        // `setCancelled` is the one that every cancellable event shares.
        let setters = events
            .iter()
            .flat_map(|kind| &kind.methods)
            .filter_map(|method| javap::method(&method.line))
            .filter(|(name, _)| {
                name.strip_prefix("set")
                    .is_some_and(|rest| rest.starts_with(|first: char| first.is_ascii_uppercase()))
                    && *name != "setCancelled"
            })
            .count();
        json!({
            "classes": events.len(),
            "concrete": concrete.len(),
            "concreteCancellable": cancellable,
            "listenable": listenable,
            "ownHandlerList": own_list,
            "setters": setters,
        })
    }
}

/// The JSON list `key` of a type with one item per line, closed with `end` after its bracket.
fn block(key: &str, items: &[String], end: &str) -> String {
    if items.is_empty() {
        return format!("      \"{key}\": []{end}");
    }
    format!(
        "      \"{key}\": [\n        {}\n      ]{end}",
        items.join(",\n        ")
    )
}

/// The types of an inventory by their names, for the questions about their ancestors.
pub struct Index<'a> {
    types: BTreeMap<&'a str, &'a Type>,
}

impl<'a> Index<'a> {
    /// The index of `inventory`.
    pub fn new(inventory: &'a Inventory) -> Self {
        Self {
            types: inventory
                .types
                .iter()
                .map(|kind| (kind.name.as_str(), kind))
                .collect(),
        }
    }

    /// Every type that `name` extends or implements, however far up, the types outside the
    /// inventory included.
    pub fn ancestors(&self, name: &str) -> BTreeSet<&'a str> {
        let mut found = BTreeSet::new();
        let mut open = vec![name];
        while let Some(next) = open.pop() {
            let Some(kind) = self.types.get(next) else {
                continue;
            };
            for parent in kind.extends.iter().chain(&kind.implements) {
                let parent = erased(parent);
                if found.insert(parent) {
                    open.push(parent);
                }
            }
        }
        found
    }

    /// The class whose handler list the event class `name` joins: itself where it declares the
    /// static `getHandlerList`, else the nearest class above it that does. `None` for a class
    /// that no listener can name.
    pub fn chain(&self, name: &str) -> Option<&'a str> {
        let mut next = self.types.get(name)?;
        // The classes seen, so that the way up ends even where two classes name each other.
        let mut seen = BTreeSet::new();
        while seen.insert(next.name.as_str()) {
            if next
                .methods
                .iter()
                .any(|method| method.line == HANDLER_LIST)
            {
                return Some(next.name.as_str());
            }
            next = self.types.get(erased(next.extends.first()?))?;
        }
        None
    }
}
