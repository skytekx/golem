// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
// SPDX-FileCopyrightText: lucko (Luck) <luck@lucko.me>
// SPDX-FileCopyrightText: LuckPerms contributors
/*
 * The syntax of golem-permissions: the nodes of the configuration with their wildcards and
 * denies, the node a check names, the name of a group and the UUID of a player.
 */

/// What a node of the configuration matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pattern<'a> {
    /// The node itself, such as `essentials.home`.
    Exact(&'a str),
    /// Every node below the prefix, written `essentials.signs.use.*`, held without its `.*`.
    Below(&'a str),
    /// Every node, written `*`.
    Every,
}

impl std::fmt::Display for Pattern<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exact(node) => formatter.write_str(node),
            Self::Below(prefix) => write!(formatter, "{prefix}.*"),
            Self::Every => formatter.write_str("*"),
        }
    }
}

/// A node of the configuration: a pattern that a source grants or denies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rule<'a> {
    /// True for a node written with a leading `-`.
    pub deny: bool,
    /// What the node matches.
    pub pattern: Pattern<'a>,
}

/// The characters a segment may hold, for the messages.
const SEGMENT_CHARACTERS: &str = "a to z, 0 to 9, _ and -";

/// Why a text is no node or no name of a group. It borrows from the text, so a check finds it
/// without allocating, and it prints as the end of a sentence that names the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flaw<'a> {
    /// The text is empty.
    Empty,
    /// A segment is empty, as in `a..b`.
    EmptySegment,
    /// A segment starts with `-`.
    DashSegment(&'a str),
    /// A `*` stands where no wildcard may.
    Star,
    /// A segment holds a character it does not take.
    Character(char),
    /// A node of a check starts with `-`.
    Deny,
    /// A node of a check is a wildcard.
    Wildcard,
    /// A name of a group holds a dot.
    Dot,
}

impl std::fmt::Display for Flaw<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("is empty"),
            Self::EmptySegment => formatter.write_str("holds an empty segment"),
            Self::DashSegment(segment) => write!(
                formatter,
                "holds the segment {segment}, which starts with -, but a segment starts with a to z, 0 to 9 or _"
            ),
            Self::Star => formatter
                .write_str("holds a * that is not a wildcard, which is * alone or .* at the end"),
            Self::Character(bad) => write!(
                formatter,
                "holds the character {bad:?}, but a segment takes only {SEGMENT_CHARACTERS}"
            ),
            Self::Deny => {
                formatter.write_str("starts with -, which marks a deny in the configuration")
            }
            Self::Wildcard => formatter.write_str("is a wildcard, and a check names one node"),
            Self::Dot => formatter.write_str("holds a dot, and a group name is one segment"),
        }
    }
}

/// Reads a node of the configuration, such as `essentials.home`, `-essentials.*` or `*`.
///
/// # Errors
///
/// Returns why `text` is no node.
pub(crate) fn parse_rule(text: &str) -> Result<Rule<'_>, Flaw<'_>> {
    let (deny, body) = match text.strip_prefix('-') {
        Some(body) => (true, body),
        None => (false, text),
    };
    if body == "*" {
        return Ok(Rule {
            deny,
            pattern: Pattern::Every,
        });
    }
    if let Some(prefix) = body.strip_suffix(".*") {
        if prefix.is_empty() {
            return Err(Flaw::EmptySegment);
        }
        check_segments(prefix)?;
        return Ok(Rule {
            deny,
            pattern: Pattern::Below(prefix),
        });
    }
    check_segments(body)?;
    Ok(Rule {
        deny,
        pattern: Pattern::Exact(body),
    })
}

/// Checks that `text` is a node a check may name: segments alone, with no wildcard and no deny.
/// It allocates nothing, since every check of the server thread runs it.
///
/// # Errors
///
/// Returns why `text` is no such node.
pub(crate) fn check_node(text: &str) -> Result<(), Flaw<'_>> {
    if text.starts_with('-') {
        return Err(Flaw::Deny);
    }
    if text == "*" || text.ends_with(".*") {
        return Err(Flaw::Wildcard);
    }
    check_segments(text)
}

/// Checks that `text` is a name of a group: one segment.
///
/// # Errors
///
/// Returns why `text` is no name.
pub(crate) fn check_name(text: &str) -> Result<(), Flaw<'_>> {
    if text.contains('.') {
        return Err(Flaw::Dot);
    }
    check_segments(text)
}

/// Checks that `text` is segments joined by dots, each of [a-z0-9_-] and starting with one of
/// [a-z0-9_], since a leading `-` marks a deny.
fn check_segments(text: &str) -> Result<(), Flaw<'_>> {
    if text.is_empty() {
        return Err(Flaw::Empty);
    }
    for segment in text.split('.') {
        let Some(first) = segment.chars().next() else {
            return Err(Flaw::EmptySegment);
        };
        if first == '-' {
            return Err(Flaw::DashSegment(segment));
        }
        if let Some(bad) = segment.chars().find(|&c| !is_segment_character(c)) {
            return Err(if bad == '*' {
                Flaw::Star
            } else {
                Flaw::Character(bad)
            });
        }
    }
    Ok(())
}

fn is_segment_character(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'
}

/// Reads a UUID in its usual form of 36 characters, 32 hexadecimal digits in groups of 8, 4,
/// 4, 4 and 12 joined by dashes. A check reads either case. The configuration takes lower case
/// alone, so that a player has one spelling there.
pub(crate) fn parse_uuid(text: &str) -> Option<u128> {
    let bytes = text.as_bytes();
    if bytes.len() != 36 {
        return None;
    }
    let mut value: u128 = 0;
    for (position, &byte) in bytes.iter().enumerate() {
        if matches!(position, 8 | 13 | 18 | 23) {
            if byte != b'-' {
                return None;
            }
            continue;
        }
        let digit = char::from(byte).to_digit(16)?;
        value = (value << 4) | u128::from(digit);
    }
    Some(value)
}
