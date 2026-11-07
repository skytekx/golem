// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The styled text of golem-bridge: a text component of the chat of Minecraft 1.20.1, with its
 * colours, decorations and click and hover events, which a plugin builds in Rust and the mod turns
 * into a Component with Component.Serializer.fromJson.
 *
 * A plugin builds a tree of texts. Its own words go in as they are, and whatever a player wrote
 * or named, such as a name, a chat line or the name of a home, goes in through Text::player, which
 * keeps it literal: it becomes a text node of its own, so no player can open or close a style,
 * and it loses every formatting code and control character, so no player can colour the rest of
 * a line or start a line of their own, and every direction control of Unicode. The JSON is
 * written here, with every character that JSON or the wire form of an action reserves escaped, so
 * no text can break the action that carries it.
 */

//! Styled text for the actions `styledMessage` and `styledBroadcast`.

use std::fmt::Write as _;

/// The longest JSON of a text that the game sends a client, in UTF-16 units: the bound of
/// `FriendlyByteBuf.writeComponent` of Minecraft 1.20.1 for the packet of a system message.
pub const MAX_JSON: usize = 262_144;

/// How deep texts may sit inside each other, through the texts after them, the arguments of a
/// translation and the text of a hover, so no text costs a client a deep recursion.
pub const MAX_DEPTH: usize = 32;

/// A colour of a text: one of the 16 colours of the game by name, or any colour by its red,
/// green and blue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    /// `black`, `§0`.
    Black,
    /// `dark_blue`, `§1`.
    DarkBlue,
    /// `dark_green`, `§2`.
    DarkGreen,
    /// `dark_aqua`, `§3`.
    DarkAqua,
    /// `dark_red`, `§4`.
    DarkRed,
    /// `dark_purple`, `§5`.
    DarkPurple,
    /// `gold`, `§6`.
    Gold,
    /// `gray`, `§7`.
    Gray,
    /// `dark_gray`, `§8`.
    DarkGray,
    /// `blue`, `§9`.
    Blue,
    /// `green`, `§a`.
    Green,
    /// `aqua`, `§b`.
    Aqua,
    /// `red`, `§c`.
    Red,
    /// `light_purple`, `§d`.
    LightPurple,
    /// `yellow`, `§e`.
    Yellow,
    /// `white`, `§f`.
    White,
    /// A colour by its red, green and blue, written `#rrggbb`.
    Rgb(u8, u8, u8),
}

/// The colours of the game by name, in the order of their codes.
const NAMED: [(Color, &str); 16] = [
    (Color::Black, "black"),
    (Color::DarkBlue, "dark_blue"),
    (Color::DarkGreen, "dark_green"),
    (Color::DarkAqua, "dark_aqua"),
    (Color::DarkRed, "dark_red"),
    (Color::DarkPurple, "dark_purple"),
    (Color::Gold, "gold"),
    (Color::Gray, "gray"),
    (Color::DarkGray, "dark_gray"),
    (Color::Blue, "blue"),
    (Color::Green, "green"),
    (Color::Aqua, "aqua"),
    (Color::Red, "red"),
    (Color::LightPurple, "light_purple"),
    (Color::Yellow, "yellow"),
    (Color::White, "white"),
];

impl Color {
    /// Answers the colour as the JSON of the game writes it, such as `gold` or `#ffaa00`.
    pub fn name(self) -> String {
        match self {
            Self::Rgb(red, green, blue) => format!("#{red:02x}{green:02x}{blue:02x}"),
            named => NAMED
                .iter()
                .find(|(color, _)| *color == named)
                .map_or_else(String::new, |(_, name)| (*name).to_owned()),
        }
    }

    /// Answers the colour of a name of the game, such as `gold`, or of `#rrggbb` in any case,
    /// as `TextColor.parseColor` of the game reads them.
    pub fn from_name(name: &str) -> Option<Self> {
        if let Some(hex) = name.strip_prefix('#') {
            if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return None;
            }
            let value = u32::from_str_radix(hex, 16).ok()?;
            let [_, red, green, blue] = value.to_be_bytes();
            return Some(Self::Rgb(red, green, blue));
        }
        NAMED
            .iter()
            .find(|(_, known)| *known == name)
            .map(|(color, _)| *color)
    }
}

/// What a click on a text does in the client of Minecraft 1.20.1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Click {
    /// Opens an address after the player confirms it, an address of `http` or `https` alone.
    OpenUrl(String),
    /// Runs a command as the player, which must start with a slash and take no signed argument,
    /// such as the message of `msg`, or the client refuses it.
    RunCommand(String),
    /// Puts the text into the chat field of the player.
    SuggestCommand(String),
    /// Copies the text to the clipboard of the player.
    CopyToClipboard(String),
}

impl Click {
    /// Answers the action and the value of the click, as the JSON of the game names them.
    fn parts(&self) -> (&'static str, &str) {
        match self {
            Self::OpenUrl(value) => ("open_url", value),
            Self::RunCommand(value) => ("run_command", value),
            Self::SuggestCommand(value) => ("suggest_command", value),
            Self::CopyToClipboard(value) => ("copy_to_clipboard", value),
        }
    }
}

/// What a text shows.
#[derive(Clone, Debug, PartialEq)]
enum Content {
    /// Text as it is.
    Literal(String),
    /// A key of the language of the client, such as `item.minecraft.diamond`, whose `%s` the
    /// arguments fill.
    Translate { key: String, with: Vec<Text> },
}

/// A styled text: what it shows, its style and the texts after it, which take its style where
/// they set none of their own.
///
/// ```text
/// Text::plain("Teleporting to ")
///     .color(Color::Rgb(0xff, 0xaa, 0x00))
///     .append(Text::player(home).color(Color::Rgb(0xff, 0x55, 0x55)))
///     .append(Text::plain("."))
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    content: Content,
    color: Option<Color>,
    bold: Option<bool>,
    italic: Option<bool>,
    underlined: Option<bool>,
    strikethrough: Option<bool>,
    obfuscated: Option<bool>,
    click: Option<Click>,
    hover: Option<Box<Text>>,
    insertion: Option<String>,
    extra: Vec<Text>,
}

impl Text {
    fn of(content: Content) -> Self {
        Self {
            content,
            color: None,
            bold: None,
            italic: None,
            underlined: None,
            strikethrough: None,
            obfuscated: None,
            click: None,
            hover: None,
            insertion: None,
            extra: Vec::new(),
        }
    }

    /// The words of a plugin, as they are, newlines included. A formatting code in them is the
    /// choice of the plugin, so a text that a player wrote or named goes through
    /// [`Text::player`] instead.
    pub fn plain(text: impl Into<String>) -> Self {
        Self::of(Content::Literal(text.into()))
    }

    /// What a player wrote or named, such as a name, a chat line or the name of a home, kept
    /// literal: a text of its own, without the formatting codes, the control characters and the
    /// direction controls that [`strip_codes`] drops, so it opens or closes no style and starts
    /// no line of its own.
    pub fn player(text: &str) -> Self {
        Self::plain(strip_codes(text))
    }

    /// A key of the language of the client, which translates it, whose `%s` the arguments
    /// fill in their order, such as `death.attack.generic` with the name of a player.
    pub fn translate(key: impl Into<String>, with: Vec<Text>) -> Self {
        Self::of(Content::Translate {
            key: key.into(),
            with,
        })
    }

    /// Sets the colour.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets or clears the bold decoration.
    #[must_use]
    pub fn bold(mut self, on: bool) -> Self {
        self.bold = Some(on);
        self
    }

    /// Sets or clears the italic decoration.
    #[must_use]
    pub fn italic(mut self, on: bool) -> Self {
        self.italic = Some(on);
        self
    }

    /// Sets or clears the underline.
    #[must_use]
    pub fn underlined(mut self, on: bool) -> Self {
        self.underlined = Some(on);
        self
    }

    /// Sets or clears the line through the text.
    #[must_use]
    pub fn strikethrough(mut self, on: bool) -> Self {
        self.strikethrough = Some(on);
        self
    }

    /// Sets or clears the decoration that scrambles the letters.
    #[must_use]
    pub fn obfuscated(mut self, on: bool) -> Self {
        self.obfuscated = Some(on);
        self
    }

    /// Sets what a click does.
    #[must_use]
    pub fn click(mut self, click: Click) -> Self {
        self.click = Some(click);
        self
    }

    /// Sets the text that the client shows while the pointer rests on this one.
    #[must_use]
    pub fn hover(mut self, text: Text) -> Self {
        self.hover = Some(Box::new(text));
        self
    }

    /// Sets the text that a click with shift puts into the chat field of the player.
    #[must_use]
    pub fn insertion(mut self, text: impl Into<String>) -> Self {
        self.insertion = Some(text.into());
        self
    }

    /// Adds a text after this one and the texts after it, which takes the style of this one
    /// where it sets none of its own.
    #[must_use]
    pub fn append(mut self, text: Text) -> Self {
        self.extra.push(text);
        self
    }

    /// Adds a text after this one and the texts after it, in place.
    pub fn push(&mut self, text: Text) {
        self.extra.push(text);
    }

    /// Answers how deep the texts sit inside this one, 1 for a text alone.
    pub fn depth(&self) -> usize {
        let inner = self
            .extra
            .iter()
            .chain(self.hover.as_deref())
            .chain(match &self.content {
                Content::Translate { with, .. } => with.as_slice(),
                Content::Literal(_) => &[],
            })
            .map(Text::depth)
            .max()
            .unwrap_or(0);
        inner + 1
    }

    /// Answers the text without its style, as the console of the server shows it, with the key
    /// of a translation and its arguments in brackets, since the language of the client is not
    /// known here.
    pub fn to_plain(&self) -> String {
        let mut plain = String::new();
        self.write_plain(&mut plain);
        plain
    }

    fn write_plain(&self, out: &mut String) {
        match &self.content {
            Content::Literal(text) => out.push_str(text),
            Content::Translate { key, with } => {
                out.push_str(key);
                if !with.is_empty() {
                    out.push('[');
                    for (index, argument) in with.iter().enumerate() {
                        if index > 0 {
                            out.push_str(", ");
                        }
                        argument.write_plain(out);
                    }
                    out.push(']');
                }
            }
        }
        for text in &self.extra {
            text.write_plain(out);
        }
    }

    /// Answers the text as the JSON of a component that `Component.Serializer` of Minecraft
    /// 1.20.1 reads, with every key in the order of this type and every string escaped as JSON
    /// asks, every control character among them, so the JSON holds no U+001F of the wire form.
    pub fn to_json(&self) -> String {
        let mut json = String::new();
        self.write_json(&mut json);
        json
    }

    fn write_json(&self, out: &mut String) {
        out.push('{');
        match &self.content {
            Content::Literal(text) => {
                out.push_str("\"text\":");
                string(out, text);
            }
            Content::Translate { key, with } => {
                out.push_str("\"translate\":");
                string(out, key);
                if !with.is_empty() {
                    out.push_str(",\"with\":");
                    list(out, with);
                }
            }
        }
        if let Some(color) = self.color {
            out.push_str(",\"color\":");
            string(out, &color.name());
        }
        for (key, value) in [
            ("bold", self.bold),
            ("italic", self.italic),
            ("underlined", self.underlined),
            ("strikethrough", self.strikethrough),
            ("obfuscated", self.obfuscated),
        ] {
            if let Some(on) = value {
                let _ = write!(out, ",\"{key}\":{on}");
            }
        }
        if let Some(click) = &self.click {
            let (action, value) = click.parts();
            let _ = write!(out, ",\"clickEvent\":{{\"action\":\"{action}\",\"value\":");
            string(out, value);
            out.push('}');
        }
        if let Some(hover) = &self.hover {
            out.push_str(",\"hoverEvent\":{\"action\":\"show_text\",\"contents\":");
            hover.write_json(out);
            out.push('}');
        }
        if let Some(insertion) = &self.insertion {
            out.push_str(",\"insertion\":");
            string(out, insertion);
        }
        if !self.extra.is_empty() {
            out.push_str(",\"extra\":");
            list(out, &self.extra);
        }
        out.push('}');
    }
}

/// Writes a list of texts as a JSON array.
fn list(out: &mut String, texts: &[Text]) {
    out.push('[');
    for (index, text) in texts.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        text.write_json(out);
    }
    out.push(']');
}

/// Writes a string as JSON: the quote and the backslash escaped, every control character below
/// U+0020 as `\n`, `\t` or `\u00XX`, and U+2028 and U+2029 as Gson writes them, so the JSON is as
/// long as the one the game writes for the same text.
fn string(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if u32::from(c) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Answers text that a player wrote or named without what would make it more than text: every
/// formatting code, the character U+00A7 with the character after it, as the client of
/// Minecraft hides both when it draws a text, every control character that the game refuses in
/// a chat line, below U+0020 and U+007F, a newline among them, and every direction control of
/// Unicode, U+202A to U+202E and U+2066 to U+2069.
///
/// The game itself refuses a formatting code and a control character in a chat line or a command
/// of a player, so what a player typed passes unchanged but for a direction control, and what a
/// mod or a file made, such as a display name with a colour code, reads as the client would draw
/// it without its colour.
pub fn strip_codes(text: &str) -> String {
    let mut stripped = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\u{a7}' {
            chars.next();
        } else if c >= ' ' && c != '\u{7f}' && !direction_control(c) {
            stripped.push(c);
        }
    }
    stripped
}

/// Answers whether a character is a direction control of Unicode: an embedding, an override or
/// the end of one, U+202A to U+202E, or an isolate or the end of one, U+2066 to U+2069.
fn direction_control(c: char) -> bool {
    matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}
