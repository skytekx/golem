// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The names of the parameters of the methods of the reference, which javap does not print, read
 * from the sources jar. The reader takes the declarations of the types and of their methods and
 * constructors from a Java file and nothing else: it skips every comment, so no text of the
 * javadoc is read past, kept or written, and it skips every body and every initializer.
 */

//! The names of the parameters, from the Java sources of the reference.

use std::collections::BTreeMap;

/// A method or a constructor as a source file declares it: its name, and the name and the type
/// of each parameter, the type cut down to what tells two overloads apart.
pub struct Declared {
    name: String,
    parameters: Vec<(String, String)>,
}

/// The declarations of one source file, by the path of simple names of the type that holds
/// them, such as `["Player", "Spigot"]`.
pub type Declarations = BTreeMap<Vec<String>, Vec<Declared>>;

/// The words that open the declaration of a type. A record is left out, since `record` is a
/// plain name wherever no record is declared, and the reference declares none.
const KINDS: [&str; 3] = ["class", "interface", "enum"];

/// The modifiers of a member, which stand before the type of a method and are no part of it.
const MODIFIERS: [&str; 12] = [
    "public",
    "protected",
    "private",
    "abstract",
    "static",
    "final",
    "strictfp",
    "default",
    "synchronized",
    "native",
    "transient",
    "volatile",
];

/// Splits Java source into its words and its punctuation, without comments and with every
/// literal as one empty word.
fn tokens(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        let rest = &text[at..];
        if byte.is_ascii_whitespace() {
            at += 1;
        } else if rest.starts_with("//") {
            at += rest.find('\n').unwrap_or(rest.len());
        } else if let Some(comment) = rest.strip_prefix("/*") {
            at += comment.find("*/").map_or(rest.len(), |end| end + 4);
        } else if let Some(block) = rest.strip_prefix("\"\"\"") {
            at += block.find("\"\"\"").map_or(rest.len(), |end| end + 6);
            found.push("");
        } else if byte == b'"' || byte == b'\'' {
            let mut end = at + 1;
            while end < bytes.len() && bytes[end] != byte {
                end += if bytes[end] == b'\\' { 2 } else { 1 };
            }
            at = end + 1;
            found.push("");
        } else if byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$' || !byte.is_ascii() {
            let mut end = at + 1;
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric()
                    || bytes[end] == b'_'
                    || bytes[end] == b'$'
                    || !bytes[end].is_ascii())
            {
                end += 1;
            }
            found.push(&text[at..end]);
            at = end;
        } else if rest.starts_with("...") {
            found.push("...");
            at += 3;
        } else {
            found.push(&text[at..=at]);
            at += 1;
        }
    }
    found
}

/// The place after the group that `open` starts at `at` and `close` ends.
fn after_group(tokens: &[&str], at: usize, open: &str, close: &str) -> usize {
    let mut depth = 0usize;
    for (offset, token) in tokens[at..].iter().enumerate() {
        if *token == open {
            depth += 1;
        } else if *token == close {
            depth -= 1;
            if depth == 0 {
                return at + offset + 1;
            }
        }
    }
    tokens.len()
}

/// The place after the annotation whose `@` stands at `at`: after its name, and after its
/// arguments where it has some.
fn after_annotation(tokens: &[&str], at: usize) -> usize {
    let mut end = at + 2;
    while tokens.get(end) == Some(&".") {
        end += 2;
    }
    if tokens.get(end) == Some(&"(") {
        end = after_group(tokens, end, "(", ")");
    }
    end
}

/// Whether the tokens at `at` open the declaration of a type, and how many of them the word
/// of its kind takes.
fn opens_type(tokens: &[&str], at: usize) -> Option<usize> {
    if KINDS.contains(&tokens[at]) {
        return Some(1);
    }
    (tokens[at] == "@" && tokens.get(at + 1) == Some(&"interface")).then_some(2)
}

/// Reads the body of a type whose opening brace stands at `at`, and answers the place after
/// the brace that closes it.
fn body(tokens: &[&str], at: usize, path: &[String], found: &mut Declarations) -> usize {
    let mut at = at + 1;
    // The words of the member that is being read, up to its parenthesis.
    let mut member: Vec<&str> = Vec::new();
    while at < tokens.len() {
        let token = tokens[at];
        if token == "}" {
            return at + 1;
        }
        if let Some(words) = opens_type(tokens, at) {
            let Some(name) = tokens.get(at + words) else {
                return tokens.len();
            };
            let mut inner = path.to_vec();
            inner.push((*name).to_owned());
            let open = tokens[at..]
                .iter()
                .position(|token| *token == "{")
                .map_or(tokens.len(), |offset| at + offset);
            at = body(tokens, open, &inner, found);
            member.clear();
        } else if token == "@" {
            at = after_annotation(tokens, at);
        } else if token == ";" || token == "," {
            // A comma at this depth parts two constants of an enum or two fields of one type.
            member.clear();
            at += 1;
        } else if token == "=" {
            // An initializer, up to its semicolon outside every group.
            let mut depth = 0usize;
            while at < tokens.len() && !(tokens[at] == ";" && depth == 0) {
                match tokens[at] {
                    "(" | "{" | "[" => depth += 1,
                    ")" | "}" | "]" => depth = depth.saturating_sub(1),
                    _ => {}
                }
                at += 1;
            }
            member.clear();
        } else if token == "{" {
            // An initializer block, or the body of a constant of an enum.
            at = after_group(tokens, at, "{", "}");
            member.clear();
        } else if token == "(" {
            at = declaration(tokens, at, &member, path, found);
            member.clear();
        } else if token == "<" && member.iter().all(|word| MODIFIERS.contains(word)) {
            // The type parameters of a generic method, which stand before its type.
            at = after_group(tokens, at, "<", ">");
            member.push("<>");
        } else {
            member.push(token);
            at += 1;
        }
    }
    at
}

/// Reads what the parenthesis at `at` opens, after the words of `member`: the parameters of a
/// method or of a constructor, which is kept, or the arguments of a constant of an enum.
/// Answers the place after the member.
fn declaration(
    tokens: &[&str],
    at: usize,
    member: &[&str],
    path: &[String],
    found: &mut Declarations,
) -> usize {
    let close = after_group(tokens, at, "(", ")");
    let mut end = close;
    if tokens.get(end) == Some(&"throws") {
        while end < tokens.len() && tokens[end] != "{" && tokens[end] != ";" {
            end += 1;
        }
    }
    // An element of an annotation may carry a default before its semicolon.
    if tokens.get(end) == Some(&"default") {
        while end < tokens.len() && tokens[end] != ";" {
            end += 1;
        }
    }
    let ends_as_method = matches!(tokens.get(end), Some(&"{" | &";"));
    if let Some((name, before)) = member.split_last() {
        // A method has a type before its name, and a constructor has the name of its type.
        let typed = before.iter().any(|word| !MODIFIERS.contains(word));
        let constructs = path.last().is_some_and(|last| last == name);
        if ends_as_method && (typed || constructs) {
            found.entry(path.to_vec()).or_default().push(Declared {
                name: (*name).to_owned(),
                parameters: parameters(tokens.get(at + 1..close - 1).unwrap_or_default()),
            });
        }
    }
    match tokens.get(end) {
        Some(&"{") => after_group(tokens, end, "{", "}"),
        Some(&";" | &",") => end + 1,
        _ => end,
    }
}

/// Reads the parameters between the parentheses of a declaration.
fn parameters(tokens: &[&str]) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut words: Vec<&str> = Vec::new();
    let mut depth = 0usize;
    let mut at = 0;
    while at <= tokens.len() {
        let token = tokens.get(at).copied();
        match token {
            Some("@") => {
                at = after_annotation(tokens, at);
                continue;
            }
            Some("<" | "(" | "[") => depth += 1,
            Some(">" | ")" | "]") => depth = depth.saturating_sub(1),
            _ => {}
        }
        if token.is_none() || (token == Some(",") && depth == 0) {
            if let Some(parameter) = parameter(&words) {
                found.push(parameter);
            }
            words.clear();
        } else if let Some(token) = token
            && token != "final"
        {
            words.push(token);
        }
        at += 1;
    }
    found
}

/// The name of one parameter and its type, from its words without annotations.
fn parameter(words: &[&str]) -> Option<(String, String)> {
    // An array may be marked behind the name, as C marks it.
    let mut end = words.len();
    let mut behind = 0;
    while end >= 2 && words[end - 2..end] == ["[", "]"] {
        end -= 2;
        behind += 1;
    }
    let (name, kind) = words[..end].split_last()?;
    let mut outside = Vec::new();
    let mut depth = 0usize;
    for word in kind {
        match *word {
            "<" => depth += 1,
            ">" => depth = depth.saturating_sub(1),
            _ if depth == 0 => outside.push(*word),
            _ => {}
        }
    }
    let arrays = behind
        + outside
            .iter()
            .filter(|word| matches!(**word, "[" | "..."))
            .count();
    let last = outside
        .iter()
        .rev()
        .find(|word| !matches!(**word, "[" | "]" | "..." | "."))?;
    Some(((*name).to_owned(), format!("{last}{}", "[]".repeat(arrays))))
}

/// What tells the type of a parameter apart as javap prints it: its last name, without its
/// package, its outer types and its type arguments, and its arrays.
fn printed_kind(printed: &str) -> String {
    let mut outside = String::new();
    let mut depth = 0usize;
    for character in printed.chars() {
        match character {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => outside.push(character),
            _ => {}
        }
    }
    let arrays = outside.matches("[]").count() + outside.matches("...").count();
    let plain = outside.replace("[]", "").replace("...", "");
    let last = plain.trim().rsplit(['.', '$']).next().unwrap_or_default();
    format!("{last}{}", "[]".repeat(arrays))
}

/// Reads the declarations of the Java file `text`.
pub fn declarations(text: &str) -> Declarations {
    let tokens = tokens(text);
    let mut found = Declarations::new();
    let mut at = 0;
    while at < tokens.len() {
        if let Some(words) = opens_type(&tokens, at)
            && let Some(name) = tokens.get(at + words)
        {
            let open = tokens[at..]
                .iter()
                .position(|token| *token == "{")
                .map_or(tokens.len(), |offset| at + offset);
            at = body(&tokens, open, &[(*name).to_owned()], &mut found);
        } else if tokens[at] == "@" {
            at = after_annotation(&tokens, at);
        } else {
            at += 1;
        }
    }
    found
}

/// The names of the parameters of the method `name` of the type at `path`, whose parameters
/// javap prints as `printed`, or `None` where the file declares no such method.
///
/// A declaration matches by its name, the count of its parameters and their types. Where one
/// declaration alone has the name and the count, it also matches a line whose types differ
/// only where javap prints `java.lang.Object`, which is the bridge method that the compiler
/// adds for a method with a type parameter.
pub fn names(
    declarations: &Declarations,
    path: &[String],
    name: &str,
    printed: &[&str],
) -> Option<Vec<String>> {
    let candidates: Vec<&Declared> = declarations
        .get(path)?
        .iter()
        .filter(|declared| declared.name == name && declared.parameters.len() == printed.len())
        .collect();
    let kinds: Vec<String> = printed.iter().map(|kind| printed_kind(kind)).collect();
    let names_of = |declared: &Declared| -> Vec<String> {
        declared
            .parameters
            .iter()
            .map(|(name, _)| name.clone())
            .collect()
    };
    let mut exact = candidates
        .iter()
        .filter(|declared| {
            declared
                .parameters
                .iter()
                .zip(&kinds)
                .all(|((_, kind), printed)| kind == printed)
        })
        .map(|declared| names_of(declared));
    if let Some(first) = exact.next() {
        // Two declarations with the same types are told apart by nothing that javap prints, so
        // they name the parameters only where they agree.
        return exact.all(|other| other == first).then_some(first);
    }
    let [single] = candidates.as_slice() else {
        return None;
    };
    single
        .parameters
        .iter()
        .zip(printed.iter().zip(&kinds))
        .all(|((_, kind), (printed, short))| kind == short || *printed == "java.lang.Object")
        .then(|| names_of(single))
}
