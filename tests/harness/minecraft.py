# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
"""Golem harness: a player of Minecraft 1.20.1 for the notebooks that boot a server.

    from harness.minecraft import Client, offline_uuid

    player = Client(25633, "Tester")
    player.login()
    player.chat("hello")
    player.command("golem source")
    player.close()

The client speaks protocol 763 in offline mode: the handshake, the login with the plugin
requests of Forge answered as a vanilla client would, the keep alive, chat and commands without
signatures, the respawn and the close. A thread reads its packets as they come and records every
system chat message as raw text in `messages`, every place the server puts the player in
`positions`, the dimension of every respawn or change of dimension in `dimensions` and the name
of every player in the player list in `list_names`. It confirms every teleport, as a vanilla
client does. It is no game client: it sends no movement and loads no chunk, which a server in
offline mode on localhost accepts.
"""

from __future__ import annotations

import hashlib
import socket
import struct
import threading
import time
import uuid
import zlib

#: The protocol number of Minecraft 1.20.1.
PROTOCOL = 763


def varint(value: int) -> bytes:
    """`value` as a VarInt of the protocol."""
    out, value = bytearray(), value & 0xFFFFFFFF
    while True:
        byte, value = value & 0x7F, value >> 7
        out.append(byte | (0x80 if value else 0))
        if not value:
            return bytes(out)


def string(text: str) -> bytes:
    """`text` as a String of the protocol, its length in bytes first."""
    data = text.encode()
    return varint(len(data)) + data


def parse_varint(data: bytes, offset: int = 0) -> tuple[int, int]:
    """The VarInt at `offset` of `data` and the offset after it."""
    result, shift = 0, 0
    while True:
        byte = data[offset]
        offset += 1
        result |= (byte & 0x7F) << shift
        if not byte & 0x80:
            return result, offset
        shift += 7


def parse_string(data: bytes, offset: int = 0) -> tuple[str, int]:
    """The String at `offset` of `data` and the offset after it."""
    length, offset = parse_varint(data, offset)
    return data[offset:offset + length].decode("utf-8", "replace"), offset + length


def parse_player_info(payload: bytes) -> list:
    """The players of a Player Info Update whose name in the list it sets, each as its UUID and
    the JSON text of the name, or None when the client shows the name of the profile."""
    actions, offset = payload[0], 1
    count, offset = parse_varint(payload, offset)
    named = []
    for _ in range(count):
        player = str(uuid.UUID(bytes=payload[offset:offset + 16]))
        offset += 16
        if actions & 0x01:
            _, offset = parse_string(payload, offset)
            properties, offset = parse_varint(payload, offset)
            for _ in range(properties):
                _, offset = parse_string(payload, offset)
                _, offset = parse_string(payload, offset)
                signed, offset = payload[offset], offset + 1
                if signed:
                    _, offset = parse_string(payload, offset)
        if actions & 0x02:
            session, offset = payload[offset], offset + 1
            if session:
                offset += 16 + 8
                for _ in range(2):
                    length, offset = parse_varint(payload, offset)
                    offset += length
        if actions & 0x04:
            _, offset = parse_varint(payload, offset)
        if actions & 0x08:
            offset += 1
        if actions & 0x10:
            _, offset = parse_varint(payload, offset)
        if actions & 0x20:
            present, offset = payload[offset], offset + 1
            name = None
            if present:
                name, offset = parse_string(payload, offset)
            named.append((player, name))
    return named


class Client:
    """A player in offline mode, whose packets a thread reads as they come."""

    def __init__(self, port: int, name: str, timeout: float = 60):
        self.port, self.name, self.timeout = port, name, timeout
        self.socket = socket.create_connection(("127.0.0.1", port), timeout=timeout)
        self.stream = self.socket.makefile("rb")
        self.threshold, self.uuid = -1, None
        self.lock, self.messages, self.play = threading.Lock(), [], threading.Event()
        self.positions, self.dimensions, self.list_names = [], [], {}
        self.closed = threading.Event()

    def send(self, packet: int, payload: bytes = b"") -> None:
        body = varint(packet) + payload
        if self.threshold >= 0:
            body = varint(len(body)) + zlib.compress(body) if len(body) >= self.threshold else varint(0) + body
        with self.lock:
            self.socket.sendall(varint(len(body)) + body)

    def receive(self) -> tuple[int, bytes]:
        length, shift = 0, 0
        while True:
            byte = self.stream.read(1)
            if not byte:
                raise EOFError("the server closed the connection")
            length |= (byte[0] & 0x7F) << shift
            shift += 7
            if not byte[0] & 0x80:
                break
        data = self.stream.read(length)
        if self.threshold >= 0:
            size, offset = parse_varint(data)
            data = zlib.decompress(data[offset:]) if size else data[offset:]
        packet, offset = parse_varint(data)
        return packet, data[offset:]

    def login(self) -> None:
        """Logs in and waits for the login packet of the play state."""
        self.send(0x00, varint(PROTOCOL) + string("127.0.0.1") + struct.pack(">H", self.port) + varint(2))
        self.send(0x00, string(self.name) + b"\x00")
        while self.uuid is None:
            packet, payload = self.receive()
            if packet == 0x03:
                self.threshold = parse_varint(payload)[0]
            elif packet == 0x04:
                self.send(0x02, varint(parse_varint(payload)[0]) + b"\x00")
            elif packet == 0x02:
                self.uuid = str(uuid.UUID(bytes=payload[:16]))
            elif packet == 0x00:
                raise AssertionError("the server refused the login: %r" % payload)
        # The play state may be quiet for a long time, so only the login waits with a timeout.
        self.socket.settimeout(None)
        threading.Thread(target=self.pump, name="client-" + self.name, daemon=True).start()
        assert self.play.wait(self.timeout), "the server sent no login of the play state"

    def pump(self) -> None:
        try:
            while True:
                packet, payload = self.receive()
                if packet == 0x23:
                    self.send(0x12, payload)
                elif packet == 0x28:
                    self.play.set()
                elif packet == 0x64:
                    self.messages.append(payload.decode("utf-8", "replace"))
                elif packet == 0x3C:
                    self.positions.append(struct.unpack(">dddff", payload[:32]))
                    self.send(0x00, varint(parse_varint(payload, 33)[0]))
                elif packet == 0x41:
                    _, offset = parse_string(payload)
                    self.dimensions.append(parse_string(payload, offset)[0])
                elif packet == 0x3A:
                    self.list_names.update(parse_player_info(payload))
        except (EOFError, OSError, ValueError, zlib.error):
            pass
        finally:
            self.closed.set()

    def heard(self, text: str, timeout: float = 30) -> bool:
        """Waits until a system message holds `text`, and answers whether one did."""
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if any(text in message for message in list(self.messages)):
                return True
            time.sleep(0.1)
        return False

    def chat(self, text: str) -> None:
        stamp = struct.pack(">qq", int(time.time() * 1000), 0)
        self.send(0x05, string(text) + stamp + b"\x00" + varint(0) + bytes(3))

    def command(self, text: str) -> None:
        """Runs a command, without its slash."""
        stamp = struct.pack(">qq", int(time.time() * 1000), 0)
        self.send(0x04, string(text) + stamp + varint(0) + varint(0) + bytes(3))

    def respawn(self) -> None:
        self.send(0x07, varint(0))

    def close(self) -> None:
        try:
            self.socket.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass
        self.stream.close()
        self.socket.close()


def offline_uuid(name: str) -> str:
    """The UUID that an offline server gives a name, as UUID.nameUUIDFromBytes of Java computes it."""
    digest = bytearray(hashlib.md5(("OfflinePlayer:" + name).encode()).digest())
    digest[6], digest[8] = (digest[6] & 0x0F) | 0x30, (digest[8] & 0x3F) | 0x80
    return str(uuid.UUID(bytes=bytes(digest)))
