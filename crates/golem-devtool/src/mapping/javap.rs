// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * What javap -public prints of the classes of a jar: one block per class, with the file it was
 * compiled from, its head and one line per public member. The reader refuses every line that it
 * does not know, so a javap that prints another form fails here and writes no inventory.
 */

//! The output of `javap -public`, read strictly.

use std::{
    path::Path,
    process::Command, //
};

/// What javap prints of one class.
pub struct Printed {
    /// The file the class was compiled from, such as `Art.java`.
    pub source: String,
    /// The line of the class, without the brace that opens its body.
    pub line: String,
    /// The lines of its public members, without their indent and their semicolon.
    pub members: Vec<String>,
}

/// The parts of the line of a class.
pub struct Head {
    /// The binary name, such as `org.bukkit.entity.Player$Spigot`.
    pub name: String,
    /// `class` or `interface`. javap prints an enum as a class and an annotation as an interface.
    pub kind: String,
    /// The modifiers before the kind, in the order javap prints them.
    pub modifiers: Vec<String>,
    /// The types after `extends`, one for a class and any number for an interface.
    pub extends: Vec<String>,
    /// The types after `implements`.
    pub implements: Vec<String>,
}

/// The modifiers that javap prints before `class` or `interface`.
const MODIFIERS: [&str; 9] = [
    "public",
    "protected",
    "private",
    "abstract",
    "static",
    "final",
    "strictfp",
    "sealed",
    "non-sealed",
];

/// Runs `javap -public` over `classes` of `jar`, and answers one block per class in their order.
///
/// The language and the encoding of the JVM are set, so that the lines are the same on every
/// machine.
pub fn run(javap: &Path, jar: &Path, classes: &[String]) -> Result<Vec<Printed>, String> {
    let output = Command::new(javap)
        .args([
            "-J-Duser.language=en",
            "-J-Duser.country=US",
            "-J-Dfile.encoding=UTF-8",
            "-public",
            "-cp",
        ])
        .arg(jar)
        .args(classes)
        .output()
        .map_err(|error| format!("{}: {error}", javap.display()))?;
    if !output.status.success() || !output.stderr.is_empty() {
        return Err(format!(
            "javap ended with {} and wrote: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8(output.stdout).map_err(|_| "javap printed no UTF-8")?;
    let printed = parse(&text)?;
    if printed.len() != classes.len() {
        return Err(format!(
            "javap printed {} classes of the {} asked",
            printed.len(),
            classes.len()
        ));
    }
    Ok(printed)
}

/// Splits the output of javap into its blocks.
fn parse(text: &str) -> Result<Vec<Printed>, String> {
    let mut printed = Vec::new();
    let mut lines = text.lines();
    while let Some(first) = lines.next() {
        let source = first
            .strip_prefix("Compiled from \"")
            .and_then(|rest| rest.strip_suffix('"'))
            .ok_or_else(|| format!("javap: no file of a class in the line: {first}"))?;
        let line = lines
            .next()
            .and_then(|line| line.strip_suffix(" {"))
            .ok_or_else(|| format!("javap: no head of a class after {source}"))?;
        let mut members = Vec::new();
        loop {
            let member = lines
                .next()
                .ok_or_else(|| format!("javap: the class of {source} does not end"))?;
            if member == "}" {
                break;
            }
            let member = member
                .strip_prefix("  public ")
                .and_then(|rest| rest.strip_suffix(';'))
                .ok_or_else(|| format!("javap: no public member in the line: {member}"))?;
            members.push(format!("public {member}"));
        }
        printed.push(Printed {
            source: source.to_owned(),
            line: line.to_owned(),
            members,
        });
    }
    Ok(printed)
}

/// Splits `text` at every `separator` that stands outside angle brackets.
pub fn split_outside(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (at, character) in text.char_indices() {
        match character {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if character == separator && depth == 0 => {
                parts.push(text[start..at].trim());
                start = at + character.len_utf8();
            }
            _ => {}
        }
    }
    if !text[start..].trim().is_empty() {
        parts.push(text[start..].trim());
    }
    parts
}

/// Where `word` stands in `text` between two spaces and outside angle brackets.
fn find_outside(text: &str, word: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (at, character) in text.char_indices() {
        match character {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ' ' if depth == 0
                && text[at + 1..].starts_with(word)
                && text[at + 1 + word.len()..].starts_with(' ') =>
            {
                return Some(at);
            }
            _ => {}
        }
    }
    None
}

/// Reads the line of a class into its parts.
pub fn head(line: &str) -> Result<Head, String> {
    let broken = || format!("javap: a line of a class that is not read: {line}");
    let mut modifiers = Vec::new();
    let mut rest = line;
    let kind = loop {
        let (word, after) = rest.split_once(' ').ok_or_else(broken)?;
        rest = after;
        if word == "class" || word == "interface" {
            break word;
        }
        if !MODIFIERS.contains(&word) {
            return Err(broken());
        }
        modifiers.push(word.to_owned());
    };
    // The name ends at its type parameters or at the first space, which `extends` or
    // `implements` follows.
    let padded = format!(" {rest}");
    let extends_at = find_outside(&padded, "extends");
    let implements_at = find_outside(&padded, "implements");
    let declared_end = extends_at.or(implements_at).unwrap_or(padded.len());
    let declared = padded[..declared_end].trim();
    let name = declared.split('<').next().unwrap_or(declared);
    if name.is_empty() || name.contains(' ') {
        return Err(broken());
    }
    let list = |from: Option<usize>, word: &str, to: usize| -> Vec<String> {
        from.map_or_else(Vec::new, |at| {
            split_outside(&padded[at + 1 + word.len()..to], ',')
                .into_iter()
                .map(str::to_owned)
                .collect()
        })
    };
    let extends_end = match (extends_at, implements_at) {
        (Some(extends), Some(implements)) if implements > extends => implements,
        _ => padded.len(),
    };
    Ok(Head {
        name: name.to_owned(),
        kind: kind.to_owned(),
        modifiers,
        extends: list(extends_at, "extends", extends_end),
        implements: list(implements_at, "implements", padded.len()),
    })
}

/// The name of the method of a member line, and the types of its parameters as javap prints
/// them. A field has no parenthesis and answers `None`.
pub fn method(line: &str) -> Option<(&str, Vec<&str>)> {
    let open = line.find('(')?;
    let close = open + line[open..].find(')')?;
    let name = line[..open].rsplit(' ').next()?;
    Some((name, split_outside(&line[open + 1..close], ',')))
}
