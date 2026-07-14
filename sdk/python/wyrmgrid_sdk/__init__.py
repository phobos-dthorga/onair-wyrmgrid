"""Tiny standard-library SDK for the WyrmGrid plugin protocol v1."""

from __future__ import annotations

import json
import struct
import sys
from collections.abc import Callable
from typing import Any

API_VERSION = 1
PROTOCOL_VERSION = 1
MAX_FRAME_BYTES = 1024 * 1024


class ProtocolError(RuntimeError):
    """Raised when the host sends a malformed or incompatible message."""


def _read_exact(size: int) -> bytes:
    data = bytearray()
    while len(data) < size:
        chunk = sys.stdin.buffer.read(size - len(data))
        if not chunk:
            raise EOFError
        data.extend(chunk)
    return bytes(data)


def _read_frame() -> dict[str, Any]:
    length = struct.unpack(">I", _read_exact(4))[0]
    if length == 0 or length > MAX_FRAME_BYTES:
        raise ProtocolError("invalid frame length")
    try:
        message = json.loads(_read_exact(length).decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ProtocolError("invalid JSON frame") from error
    if not isinstance(message, dict):
        raise ProtocolError("message envelope must be an object")
    return message


def _write_frame(sequence: int, payload: dict[str, Any]) -> None:
    message = {
        "protocol_version": PROTOCOL_VERSION,
        "sequence": sequence,
        "payload": payload,
    }
    encoded = json.dumps(message, separators=(",", ":")).encode("utf-8")
    if not encoded or len(encoded) > MAX_FRAME_BYTES:
        raise ProtocolError("outgoing frame exceeds the protocol limit")
    sys.stdout.buffer.write(struct.pack(">I", len(encoded)))
    sys.stdout.buffer.write(encoded)
    sys.stdout.buffer.flush()


class Plugin:
    """Run one callback-driven WyrmGrid plugin on stdin/stdout."""

    def __init__(
        self,
        plugin_id: str,
        on_fleet_snapshot: Callable[[dict[str, Any]], dict[str, Any] | None],
    ) -> None:
        self.plugin_id = plugin_id
        self.on_fleet_snapshot = on_fleet_snapshot
        self._outgoing_sequence = 1
        self._incoming_sequence = 0
        self._grants: set[str] = set()

    def run(self) -> None:
        hello = self._receive("hello")
        if hello.get("plugin_id") != self.plugin_id:
            raise ProtocolError("host addressed a different plugin")
        self._grants = set(hello.get("granted_permissions", []))
        self._send(
            {
                "type": "ready",
                "plugin_id": self.plugin_id,
                "api_version": API_VERSION,
            }
        )

        while True:
            try:
                payload = self._receive()
            except EOFError:
                return
            message_type = payload.get("type")
            if message_type == "shutdown":
                return
            if message_type != "fleet_snapshot":
                raise ProtocolError("unsupported host message")
            if "on_air_fleet_read" not in self._grants:
                raise ProtocolError("fleet snapshot received without permission")

            layer = self.on_fleet_snapshot(payload["snapshot"])
            if layer is not None:
                if "map_layers_publish" not in self._grants:
                    raise ProtocolError("map layer publication is not granted")
                self._send({"type": "publish_map_layer", "layer": layer})

    def _receive(self, expected_type: str | None = None) -> dict[str, Any]:
        envelope = _read_frame()
        if envelope.get("protocol_version") != PROTOCOL_VERSION:
            raise ProtocolError("unsupported protocol version")
        sequence = envelope.get("sequence")
        if not isinstance(sequence, int) or sequence <= self._incoming_sequence:
            raise ProtocolError("message sequence is not increasing")
        self._incoming_sequence = sequence
        payload = envelope.get("payload")
        if not isinstance(payload, dict):
            raise ProtocolError("message payload must be an object")
        if expected_type is not None and payload.get("type") != expected_type:
            raise ProtocolError(f"expected {expected_type} message")
        return payload

    def _send(self, payload: dict[str, Any]) -> None:
        _write_frame(self._outgoing_sequence, payload)
        self._outgoing_sequence += 1
