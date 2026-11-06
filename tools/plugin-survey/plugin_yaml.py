# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
#
# The reader of plugin.yml and paper-plugin.yml for the plugin survey, on the standard library
# alone. It reads the subset of YAML 1.1 that plugin descriptors use and resolves plain scalars
# the way SnakeYAML in Spigot and PyYAML do, so an unquoted `api-version: 1.20` reads as the
# number 1.2, as on the server. It decodes the bytes as the UnicodeReader of SnakeYAML does, by
# the byte order mark, so a descriptor saved as UTF-16 reads as Spigot reads it. A document that
# YAML refuses, such as a stray scalar before the mapping, raises DescriptorError, because Spigot
# refuses the plugin for it.
#
#     load(raw_bytes) -> the document, dict, list, str, int, float, bool or None

import codecs
import re

__all__ = ["DescriptorError", "decode", "load", "parse"]


class DescriptorError(Exception):
    """A descriptor that YAML refuses."""


# The implicit resolvers of YAML 1.1 as PyYAML and SnakeYAML define them.
BOOL = re.compile(r"^(?:yes|Yes|YES|no|No|NO|true|True|TRUE|false|False|FALSE|on|On|ON|off|Off|OFF)$")
TRUE = {"yes", "Yes", "YES", "true", "True", "TRUE", "on", "On", "ON"}
FLOAT = re.compile(
    r"^(?:[-+]?(?:[0-9][0-9_]*)\.[0-9_]*(?:[eE][-+][0-9]+)?|\.[0-9_]+(?:[eE][-+][0-9]+)?"
    r"|[-+]?[0-9][0-9_]*(?::[0-5]?[0-9])+\.[0-9_]*|[-+]?\.(?:inf|Inf|INF)|\.(?:nan|NaN|NAN))$"
)
INT = re.compile(
    r"^(?:[-+]?0b[0-1_]+|[-+]?0[0-7_]+|[-+]?(?:0|[1-9][0-9_]*)|[-+]?0x[0-9a-fA-F_]+"
    r"|[-+]?[1-9][0-9_]*(?::[0-5]?[0-9])+)$"
)
NULL = re.compile(r"^(?:~|null|Null|NULL|)$")

# The characters YAML forbids in a stream, as the reader of PyYAML and SnakeYAML checks them.
FORBIDDEN = re.compile("[^\x09\x0a\x0d\x20-\x7e\x85\xa0-퟿-�\U00010000-\U0010ffff]")


def decode(raw):
    """The text of a descriptor, decoded by its byte order mark as SnakeYAML decodes it."""
    for bom, codec in (
        (codecs.BOM_UTF32_LE, "utf-32-le"),
        (codecs.BOM_UTF32_BE, "utf-32-be"),
        (codecs.BOM_UTF8, "utf-8"),
        (codecs.BOM_UTF16_LE, "utf-16-le"),
        (codecs.BOM_UTF16_BE, "utf-16-be"),
    ):
        if raw.startswith(bom):
            return raw[len(bom):].decode(codec, errors="replace")
    return raw.decode("utf-8", errors="replace")


def resolve(text):
    """A plain scalar as YAML 1.1 reads it."""
    if NULL.match(text):
        return None
    if BOOL.match(text):
        return text in TRUE
    if INT.match(text):
        value = text.replace("_", "")
        sign = -1 if value.startswith("-") else 1
        value = value.lstrip("+-")
        if value == "0":
            return 0
        if value.startswith("0b"):
            return sign * int(value[2:], 2)
        if value.startswith("0x"):
            return sign * int(value[2:], 16)
        if ":" in value:
            total = 0
            for part in value.split(":"):
                total = total * 60 + int(part)
            return sign * total
        if value.startswith("0"):
            return sign * int(value, 8)
        return sign * int(value)
    if FLOAT.match(text):
        value = text.replace("_", "").lower()
        sign = -1.0 if value.startswith("-") else 1.0
        value = value.lstrip("+-")
        if value == ".inf":
            return sign * float("inf")
        if value == ".nan":
            return float("nan")
        if ":" in value:
            total = 0.0
            for part in value.split(":"):
                total = total * 60 + float(part)
            return sign * total
        return sign * float(value)
    return text


class Line:
    __slots__ = ("number", "indent", "text")

    def __init__(self, number, indent, text):
        self.number, self.indent, self.text = number, indent, text


def strip_comment(text):
    """The text of a line without its comment, quotes respected."""
    quote = None
    for i, ch in enumerate(text):
        if quote:
            if ch == quote:
                if quote == "'" and text[i + 1:i + 2] == "'":
                    continue
                if quote == '"' and backslashes(text, i) % 2:
                    continue
                quote = None
        elif ch in "'\"" and (i == 0 or text[i - 1] in " \t[{,:-"):
            quote = ch
        elif ch == "#" and (i == 0 or text[i - 1] in " \t"):
            return text[:i].rstrip()
    return text.rstrip()


def backslashes(text, i):
    count = 0
    while i > 0 and text[i - 1] == "\\":
        count += 1
        i -= 1
    return count


def key_split(text):
    """(key, rest) when the text opens a mapping entry, else None."""
    if text[:1] in "'\"":
        quote = text[0]
        i = 1
        while i < len(text):
            if text[i] == quote:
                if quote == "'" and text[i + 1:i + 2] == "'":
                    i += 2
                    continue
                if quote == '"' and backslashes(text, i) % 2:
                    i += 1
                    continue
                break
            i += 1
        rest = text[i + 1:]
        if rest.startswith(":") and (len(rest) == 1 or rest[1] in " \t"):
            return quoted(text[:i + 1]), rest[1:].strip()
        return None
    if text[:1] in "-?" and text[1:2] in ("", " "):
        return None
    if text[:1] in "[{":
        return None
    match = re.search(r":(?:[ \t]|$)", text)
    if not match:
        return None
    key = text[:match.start()].rstrip()
    return resolve(key), text[match.end():].strip()


def quoted(text):
    """A single or double quoted scalar on one logical line."""
    if text[0] == "'":
        if len(text) < 2 or text[-1] != "'":
            raise DescriptorError("a single quoted scalar is not closed")
        return fold(text[1:-1]).replace("''", "'")
    if len(text) < 2 or text[-1] != '"' or backslashes(text, len(text) - 1) % 2:
        raise DescriptorError("a double quoted scalar is not closed")
    return unescape(fold(text[1:-1], escapes=True))


def fold(text, escapes=False):
    """The line folding of a quoted scalar that spans lines. In a double quoted scalar a
    backslash at the end of a line escapes the line break, which then joins with nothing."""
    if "\n" not in text:
        return text
    lines = text.split("\n")
    out = ""
    blank = 0
    joined = False
    for i, line in enumerate(lines):
        last = i == len(lines) - 1
        part = line if i == 0 else line.lstrip(" \t")
        if not last:
            escaped = escapes and part.endswith("\\") and backslashes(part + "x", len(part)) % 2
            if not escaped:
                part = part.rstrip(" \t")
        else:
            escaped = False
        if i > 0 and not part and not last:
            blank += 1
            continue
        if i > 0:
            out += "" if joined else (("\n" * blank) if blank else " ")
        blank = 0
        if escaped:
            part = part[:-1]
        joined = escaped
        out += part
    return out


ESCAPES = {
    "0": "\0", "a": "\a", "b": "\b", "t": "\t", "\t": "\t", "n": "\n", "v": "\v", "f": "\f",
    "r": "\r", "e": "\x1b", " ": " ", '"': '"', "/": "/", "\\": "\\", "N": "\x85", "_": "\xa0",
    "L": " ", "P": " ",
}


def unescape(text):
    out, i = [], 0
    while i < len(text):
        ch = text[i]
        if ch != "\\":
            out.append(ch)
            i += 1
            continue
        code = text[i + 1:i + 2]
        if code in ESCAPES:
            out.append(ESCAPES[code])
            i += 2
        elif code in ("x", "u", "U"):
            width = {"x": 2, "u": 4, "U": 8}[code]
            digits = text[i + 2:i + 2 + width]
            if len(digits) != width or not re.fullmatch(r"[0-9A-Fa-f]+", digits):
                raise DescriptorError("a bad escape in a double quoted scalar")
            out.append(chr(int(digits, 16)))
            i += 2 + width
        else:
            raise DescriptorError("an unknown escape \\%s in a double quoted scalar" % code)
    return "".join(out)


class Parser:
    def __init__(self, text):
        if FORBIDDEN.search(text):
            raise DescriptorError("special characters are not allowed")
        raw = text.replace("\r\n", "\n").replace("\r", "\n").split("\n")
        self.raw = raw
        self.lines = []
        started = False
        for number, line in enumerate(raw):
            body = line.lstrip(" ")
            if body.startswith("\t") and strip_comment(body.lstrip(" \t")):
                raise DescriptorError("a tab character indents line %d" % (number + 1))
            content = strip_comment(body)
            if not content:
                continue
            if line.startswith("%"):
                if started:
                    raise DescriptorError("a directive inside the document")
                continue
            if line.rstrip() == "---" or line.startswith("--- "):
                if started:
                    raise DescriptorError("expected a single document in the stream")
                started = True
                rest = line[4:].strip()
                if rest:
                    self.lines.append(Line(number, 4, strip_comment(rest)))
                continue
            if line.rstrip() == "...":
                break
            started = True
            self.lines.append(Line(number, len(line) - len(body), content))
        self.pos = 0
        self.anchors = {}

    def peek(self):
        return self.lines[self.pos] if self.pos < len(self.lines) else None

    def document(self):
        if not self.lines:
            return None
        value = self.node(-1)
        if self.peek() is not None:
            raise DescriptorError(
                "expected '<document start>', but found more content on line %d" % (self.peek().number + 1))
        return value

    def node(self, parent):
        """The block node whose lines are indented more than `parent`."""
        line = self.peek()
        if line is None or line.indent <= parent:
            return None
        if line.text == "-" or line.text.startswith("- "):
            return self.sequence(line.indent)
        if key_split(line.text) is not None:
            return self.mapping(line.indent)
        self.pos += 1
        return self.scalar(line.text, line, parent)

    def mapping(self, indent):
        result = {}
        while True:
            line = self.peek()
            if line is None or line.indent < indent:
                return result
            if line.indent > indent:
                raise DescriptorError("bad indentation of a mapping entry on line %d" % (line.number + 1))
            split = key_split(line.text)
            if split is None:
                raise DescriptorError("could not find the expected ':' on line %d" % (line.number + 1))
            key, rest = split
            self.pos += 1
            result[key] = self.value(rest, line, indent)

    def sequence(self, indent):
        result = []
        while True:
            line = self.peek()
            if line is None or line.indent < indent:
                return result
            if line.indent > indent or not (line.text == "-" or line.text.startswith("- ")):
                if line.indent == indent:
                    return result
                raise DescriptorError("bad indentation of a sequence entry on line %d" % (line.number + 1))
            rest = line.text[1:].lstrip(" ")
            column = line.indent + len(line.text) - len(rest)
            if not rest:
                self.pos += 1
                result.append(self.node(indent))
            elif rest == "-" or rest.startswith("- ") or key_split(rest) is not None:
                self.lines[self.pos] = Line(line.number, column, rest)
                result.append(self.node(indent))
            else:
                self.pos += 1
                result.append(self.value(rest, line, indent))

    def value(self, rest, line, indent):
        """The value of a mapping entry or a sequence entry that starts with `rest`."""
        if not rest:
            following = self.peek()
            if following is not None and following.indent == indent and (
                    following.text == "-" or following.text.startswith("- ")):
                return self.sequence(indent)
            return self.node(indent)
        return self.scalar(rest, line, indent)

    def scalar(self, text, line, indent):
        if text[0] == "&":
            name, _, rest = text[1:].partition(" ")
            rest = rest.strip()
            value = self.scalar(rest, line, indent) if rest else self.node(indent)
            self.anchors[name] = value
            return value
        if text[0] == "*":
            name = text[1:].strip()
            if name not in self.anchors:
                raise DescriptorError("found undefined alias %r" % name)
            return self.anchors[name]
        if text[0] == "!":
            raise DescriptorError("tags are outside the subset")
        if text[0] in "@`%":
            raise DescriptorError("found character %r that cannot start any token" % text[0])
        if text[0] in "|>":
            return self.block_scalar(text, line, indent)
        if text[0] in "[{":
            return self.flow(text, line)
        if text[0] in "'\"":
            return self.quoted_span(text, line)
        parts = [text]
        while True:
            following = self.peek()
            if following is None or following.indent <= indent:
                break
            if key_split(following.text) is not None and following.indent > indent:
                raise DescriptorError("mapping values are not allowed here, line %d" % (following.number + 1))
            parts.append(following.text)
            self.pos += 1
        joined = self.join_plain(parts, line)
        if re.search(r":(?:[ \t]|$)", joined):
            raise DescriptorError("mapping values are not allowed here, line %d" % (line.number + 1))
        if len(parts) > 1:
            return joined
        return resolve(joined)

    def join_plain(self, parts, line):
        """A plain scalar over several lines, folded with the empty lines between them kept."""
        if len(parts) == 1:
            return parts[0]
        out = parts[0]
        first = line.number
        lines = self.raw
        number = first
        for part in parts[1:]:
            number += 1
            blank = 0
            while lines[number].strip() == "" or strip_comment(lines[number].strip()) == "":
                blank += 1
                number += 1
            out += ("\n" * blank) if blank else " "
            out += part
        return out

    def quoted_span(self, text, line):
        """A quoted scalar, which may run over the following lines."""
        quote = text[0]
        source = self.raw[line.number]
        start = len(strip_comment(source)) - len(text)
        buffer = source[start:]
        number = line.number
        while True:
            end = closing(buffer, quote)
            if end is not None:
                tail = strip_comment(buffer[end + 1:]).strip()
                if tail:
                    raise DescriptorError("text after a quoted scalar on line %d" % (number + 1))
                while self.peek() is not None and self.peek().number <= number:
                    self.pos += 1
                return quoted(buffer[:end + 1])
            number += 1
            if number >= len(self.raw):
                raise DescriptorError("a quoted scalar is not closed")
            buffer += "\n" + self.raw[number]

    def flow(self, text, line):
        """A flow sequence or mapping, which may run over the following lines."""
        buffer = text
        number = line.number
        while not balanced(buffer):
            number += 1
            if number >= len(self.raw):
                raise DescriptorError("a flow collection is not closed")
            buffer += "\n" + strip_comment(self.raw[number])
        while self.peek() is not None and self.peek().number <= number:
            self.pos += 1
        value, end = FlowParser(buffer).parse()
        if buffer[end:].strip():
            raise DescriptorError("text after a flow collection on line %d" % (number + 1))
        return value

    def block_scalar(self, text, line, indent):
        header = text.split("#")[0].strip()
        match = re.fullmatch(r"([|>])([+-]?)([1-9]?)([+-]?)", header)
        if not match:
            raise DescriptorError("a bad block scalar header on line %d" % (line.number + 1))
        style, chomp, width = match.group(1), match.group(2) or match.group(4), match.group(3)
        lines = []
        number = line.number + 1
        block_indent = max(indent, 0) + int(width) if width else None
        while number < len(self.raw):
            raw = self.raw[number]
            if raw.strip() == "" and (block_indent is None or len(raw) <= block_indent):
                lines.append("")
                number += 1
                continue
            current = len(raw) - len(raw.lstrip(" "))
            if block_indent is None:
                if current <= indent:
                    break
                block_indent = current
            if current < block_indent:
                break
            lines.append(raw[block_indent:])
            number += 1
        while self.peek() is not None and self.peek().number < number:
            self.pos += 1
        trailing = 0
        while lines and lines[-1] == "":
            lines.pop()
            trailing += 1
        if style == "|":
            body = "\n".join(lines)
        else:
            body = ""
            previous_more = False
            for i, part in enumerate(lines):
                more = part.startswith((" ", "\t"))
                if i == 0:
                    body = part
                elif part == "":
                    body += "\n"
                elif lines[i - 1] == "" or more or previous_more:
                    body += ("" if lines[i - 1] == "" else "\n") + part
                else:
                    body += " " + part
                previous_more = more
        if not lines:
            return "" if chomp != "+" else "\n" * trailing
        if chomp == "-":
            return body
        if chomp == "+":
            return body + "\n" + "\n" * trailing
        return body + "\n"


def closing(buffer, quote):
    i = 1
    while i < len(buffer):
        if buffer[i] == quote:
            if quote == "'" and buffer[i + 1:i + 2] == "'":
                i += 2
                continue
            if quote == '"' and backslashes(buffer, i) % 2:
                i += 1
                continue
            return i
        i += 1
    return None


def balanced(text):
    depth, quote, i = 0, None, 0
    while i < len(text):
        ch = text[i]
        if quote:
            if ch == quote:
                if quote == "'" and text[i + 1:i + 2] == "'":
                    i += 2
                    continue
                if quote == '"' and backslashes(text, i) % 2:
                    i += 1
                    continue
                quote = None
        elif ch in "'\"":
            quote = ch
        elif ch in "[{":
            depth += 1
        elif ch in "]}":
            depth -= 1
            if depth == 0:
                return True
        i += 1
    return False


class FlowParser:
    """Flow sequences and mappings: [a, b] and {a: b}."""

    def __init__(self, text):
        self.text, self.i = text, 0

    def parse(self):
        value = self.node()
        return value, self.i

    def space(self):
        while self.i < len(self.text) and self.text[self.i] in " \t\n":
            self.i += 1

    def node(self):
        self.space()
        ch = self.text[self.i:self.i + 1]
        if ch == "[":
            return self.sequence()
        if ch == "{":
            return self.mapping()
        if ch in ("'", '"'):
            end = closing(self.text[self.i:], ch)
            if end is None:
                raise DescriptorError("a quoted scalar in a flow collection is not closed")
            value = quoted(self.text[self.i:self.i + end + 1])
            self.i += end + 1
            return value
        start = self.i
        while self.i < len(self.text):
            ch = self.text[self.i]
            if ch in ",[]{}":
                break
            if ch == ":" and self.text[self.i + 1:self.i + 2] in (" ", "\n", ",", "]", "}", ""):
                break
            self.i += 1
        plain = " ".join(self.text[start:self.i].split())
        return resolve(plain)

    def sequence(self):
        self.i += 1
        result = []
        while True:
            self.space()
            if self.text[self.i:self.i + 1] == "]":
                self.i += 1
                return result
            item = self.node()
            self.space()
            if self.text[self.i:self.i + 1] == ":":
                self.i += 1
                item = {item: self.node()}
                self.space()
            result.append(item)
            ch = self.text[self.i:self.i + 1]
            if ch == ",":
                self.i += 1
            elif ch != "]":
                raise DescriptorError("expected ',' or ']' in a flow sequence")

    def mapping(self):
        self.i += 1
        result = {}
        while True:
            self.space()
            if self.text[self.i:self.i + 1] == "}":
                self.i += 1
                return result
            key = self.node()
            self.space()
            value = None
            if self.text[self.i:self.i + 1] == ":":
                self.i += 1
                self.space()
                if self.text[self.i:self.i + 1] not in (",", "}"):
                    value = self.node()
            result[key] = value
            self.space()
            ch = self.text[self.i:self.i + 1]
            if ch == ",":
                self.i += 1
            elif ch != "}":
                raise DescriptorError("expected ',' or '}' in a flow mapping")


def parse(text):
    """The document of a descriptor text."""
    return Parser(text).document()


def load(raw):
    """The document of descriptor bytes."""
    return parse(decode(raw))
