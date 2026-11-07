// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The subcommand text-json of the devtool: it builds styled texts of golem-bridge from a
 * description in JSON, as a plugin builds them in Rust, and prints the JSON of the component that
 * each text writes for the mod, so a notebook holds the builder to the form that the mod reads.
 */

//! The subcommand `text-json` and the description of a styled text that scripts use.
//!
//! A text is described as one object: what it shows, `{"plain": "..."}`, `{"player": "..."}` or
//! `{"translate": "key", "with": [...]}`, and its style beside it: `color` as a name of the game
//! or `#rrggbb`, `bold`, `italic`, `underlined`, `strikethrough` and `obfuscated` as booleans,
//! `click` as `{"action": "open_url", "value": "..."}` with the actions `open_url`, `run_command`,
//! `suggest_command` and `copy_to_clipboard`, `hover` as a text, `insertion` and `extra`, a list
//! of texts after it. An entry `{"strip": "..."}` answers the text without its codes, as
//! `strip_codes` makes it.

use golem_bridge::{
    Click,
    Color,
    MAX_DEPTH,
    MAX_JSON,
    Text,
    strip_codes, //
};
use serde::Deserialize;
use serde_json::{
    Value,
    json, //
};

use crate::{
    Subcommand,
    bridge::{
        finish,
        read, //
    },
};

pub const TEXT_JSON: Subcommand = (
    "text-json",
    "<texts.json> the JSON of styled texts, as the mod reads it",
    text_json,
);

/// A styled text as a script describes it.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextSpec {
    plain: Option<String>,
    player: Option<String>,
    translate: Option<String>,
    #[serde(default)]
    with: Vec<TextSpec>,
    color: Option<String>,
    bold: Option<bool>,
    italic: Option<bool>,
    underlined: Option<bool>,
    strikethrough: Option<bool>,
    obfuscated: Option<bool>,
    click: Option<ClickSpec>,
    hover: Option<Box<TextSpec>>,
    insertion: Option<String>,
    #[serde(default)]
    extra: Vec<TextSpec>,
}

/// A click of a text as a script describes it.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClickSpec {
    action: String,
    value: String,
}

impl TextSpec {
    /// Builds the text through the builder of golem-bridge, or answers why it cannot.
    pub fn build(&self) -> Result<Text, String> {
        let mut text = match (&self.plain, &self.player, &self.translate) {
            (Some(plain), None, None) => Text::plain(plain.clone()),
            (None, Some(player), None) => Text::player(player),
            (None, None, Some(key)) => Text::translate(
                key.clone(),
                self.with
                    .iter()
                    .map(TextSpec::build)
                    .collect::<Result<_, _>>()?,
            ),
            _ => return Err("a text needs one of plain, player and translate".to_owned()),
        };
        if let Some(color) = &self.color {
            text = text.color(Color::from_name(color).ok_or(format!("no colour {color}"))?);
        }
        for (on, set) in [
            (self.bold, Text::bold as fn(Text, bool) -> Text),
            (self.italic, Text::italic),
            (self.underlined, Text::underlined),
            (self.strikethrough, Text::strikethrough),
            (self.obfuscated, Text::obfuscated),
        ] {
            if let Some(on) = on {
                text = set(text, on);
            }
        }
        if let Some(click) = &self.click {
            let value = click.value.clone();
            text = text.click(match click.action.as_str() {
                "open_url" => Click::OpenUrl(value),
                "run_command" => Click::RunCommand(value),
                "suggest_command" => Click::SuggestCommand(value),
                "copy_to_clipboard" => Click::CopyToClipboard(value),
                other => return Err(format!("no click {other}")),
            });
        }
        if let Some(hover) = &self.hover {
            text = text.hover(hover.build()?);
        }
        if let Some(insertion) = &self.insertion {
            text = text.insertion(insertion.clone());
        }
        for extra in &self.extra {
            text = text.append(extra.build()?);
        }
        Ok(text)
    }
}

/// An entry of the input of text-json: a text to write, or a text to strip.
#[derive(Deserialize)]
#[serde(untagged)]
enum Entry {
    Strip { strip: String },
    Text(Box<TextSpec>),
}

fn text_json(arguments: &[String]) -> i32 {
    finish(
        "text-json",
        read::<Vec<Entry>>("text-json", arguments).and_then(|entries| {
            let rows: Result<Vec<Value>, String> = entries.iter().map(row).collect();
            rows.map(|rows| json!({"maxJson": MAX_JSON, "maxDepth": MAX_DEPTH, "texts": rows}))
        }),
    )
}

/// Answers the JSON of one text, the JSON read back, its plain text, its depth and its length
/// in UTF-16 units, or the text without its codes.
fn row(entry: &Entry) -> Result<Value, String> {
    match entry {
        Entry::Strip { strip } => Ok(json!({"stripped": strip_codes(strip)})),
        Entry::Text(spec) => {
            let text = spec.build()?;
            let written = text.to_json();
            let parsed: Value = serde_json::from_str(&written)
                .map_err(|error| format!("the JSON {written} does not read: {error}"))?;
            Ok(json!({
                "json": written,
                "parsed": parsed,
                "plain": text.to_plain(),
                "depth": text.depth(),
                "utf16": written.encode_utf16().count(),
            }))
        }
    }
}
