// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The tokenizer of an argument file, the state machine of nextToken and readArgFile in
 * src/java.base/share/native/libjli/args.c of JDK 17, byte for byte.
 *
 * The launcher reads a file in blocks of 4096 bytes and keeps the state of its machine across
 * the blocks. One quirk depends on the blocks: a # outside quotes starts a comment even in the
 * middle of a token, drops the part of the token that lies in the current block before it, and
 * keeps the parts that an earlier block or a quote left behind, which then join the next token.
 * The tokenizer reads in the same blocks, so it keeps that quirk where the launcher has it.
 */

use crate::ArgError;

/// The size of the blocks in which the launcher reads an argument file.
const BLOCK: usize = 4096;

/// The states of the machine, the enum STATE of args.c.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// Between tokens, skipping white space.
    FindNext,
    /// In a comment, up to the end of the line.
    InComment,
    /// Inside quotes.
    InQuote,
    /// After a backslash inside quotes.
    InEscape,
    /// After a backslash at the end of a line inside quotes, skipping the indentation of the next.
    SkipLeadWs,
    /// In a token outside quotes.
    InToken,
}

/// The machine with the tokens it has found so far.
struct Tokenizer {
    state: State,
    /// The quote that opened the current quoted part.
    quote: u8,
    /// The parts of the current token that earlier blocks, quotes and escapes left.
    parts: Vec<u8>,
    /// How many parts the launcher holds in its list, which decides whether a token is left at
    /// the end of the file even when the parts are empty strings.
    pieces: usize,
    tokens: Vec<Vec<u8>>,
}

/// The white space of args.c, which is not the white space of isspace: it leaves out \v.
fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\n' | b'\r' | b'\t' | 0x0c)
}

/// The character an escape inside quotes stands for.
fn unescape(byte: u8) -> u8 {
    match byte {
        b'n' => b'\n',
        b'r' => b'\r',
        b't' => b'\t',
        b'f' => 0x0c,
        other => other,
    }
}

impl Tokenizer {
    fn new() -> Self {
        Self {
            state: State::FindNext,
            quote: b'"',
            parts: Vec::new(),
            pieces: 0,
            tokens: Vec::new(),
        }
    }

    fn add_part(&mut self, part: &[u8]) {
        self.parts.extend_from_slice(part);
        self.pieces += 1;
    }

    fn end_token(&mut self, rest: &[u8]) {
        let mut token = std::mem::take(&mut self.parts);
        token.extend_from_slice(rest);
        self.pieces = 0;
        self.tokens.push(token);
        self.state = State::FindNext;
    }

    /// Runs the machine over one block, as the calls of nextToken on one buffer do.
    fn feed(&mut self, block: &[u8]) {
        let mut anchor = 0;
        let mut i = 0;
        while i < block.len() {
            let mut byte = block[i];
            match self.state {
                State::FindNext | State::SkipLeadWs => {
                    while is_space(byte) {
                        i += 1;
                        if i >= block.len() {
                            return;
                        }
                        byte = block[i];
                    }
                    self.state = if self.state == State::FindNext {
                        State::InToken
                    } else {
                        State::InQuote
                    };
                    anchor = i;
                }
                State::InEscape => {
                    if byte == b'\n' || byte == b'\r' {
                        self.state = State::SkipLeadWs;
                    } else {
                        self.add_part(&[unescape(byte)]);
                        self.state = State::InQuote;
                    }
                    i += 1;
                    anchor = i;
                    continue;
                }
                State::InComment => {
                    while byte != b'\n' && byte != b'\r' {
                        i += 1;
                        if i >= block.len() {
                            return;
                        }
                        byte = block[i];
                    }
                    i += 1;
                    anchor = i;
                    self.state = State::FindNext;
                    continue;
                }
                State::InQuote | State::InToken => {}
            }
            let quoted = self.state == State::InQuote;
            match byte {
                b' ' | b'\t' | 0x0c if quoted => {}
                b' ' | b'\t' | 0x0c | b'\n' | b'\r' => {
                    self.end_token(&block[anchor..i]);
                    i += 1;
                    anchor = i;
                    continue;
                }
                b'#' if !quoted => {
                    self.state = State::InComment;
                    anchor = i + 1;
                }
                b'\\' if quoted => {
                    self.add_part(&block[anchor..i]);
                    self.state = State::InEscape;
                    anchor = i + 1;
                }
                b'\'' | b'"' if !quoted || byte == self.quote => {
                    if anchor != i {
                        self.add_part(&block[anchor..i]);
                    }
                    anchor = i + 1;
                    if quoted {
                        self.state = State::InToken;
                    } else {
                        self.quote = byte;
                        self.state = State::InQuote;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        if matches!(self.state, State::InToken | State::InQuote) && anchor < i {
            self.add_part(&block[anchor..i]);
        }
    }

    /// The tokens, with the token that the end of the file leaves open.
    fn finish(mut self) -> Vec<Vec<u8>> {
        if matches!(self.state, State::InToken | State::InQuote) && self.pieces != 0 {
            let token = std::mem::take(&mut self.parts);
            self.tokens.push(token);
        }
        self.tokens
    }
}

/// The tokens of the text of an argument file, as the launcher of JDK 17 reads them.
pub(crate) fn tokens(text: &str) -> Result<Vec<String>, ArgError> {
    if text.contains('\0') {
        return Err(ArgError::new(
            "the argument file holds a NUL byte, which no argument can carry",
        ));
    }
    let mut tokenizer = Tokenizer::new();
    for block in text.as_bytes().chunks(BLOCK) {
        tokenizer.feed(block);
    }
    tokenizer
        .finish()
        .into_iter()
        .map(|token| {
            String::from_utf8(token).map_err(|_| {
                ArgError::new("a comment splits a character of the argument file in two")
            })
        })
        .collect()
}
