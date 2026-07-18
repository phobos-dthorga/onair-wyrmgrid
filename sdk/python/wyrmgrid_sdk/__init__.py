"""Tiny standard-library SDK for the WyrmGrid plugin protocol v1."""

from __future__ import annotations

import json
import socket
import struct
import sys
from collections.abc import Callable
from typing import Any
from urllib import error, parse, request

API_VERSION = 1
PROTOCOL_VERSION = 1
MAX_FRAME_BYTES = 1024 * 1024
MAX_HTTP_RESPONSE_BYTES = 768 * 1024
HTTP_TIMEOUT_SECONDS = 15


class ProtocolError(RuntimeError):
    """Raised when the host sends a malformed or incompatible message."""


class ProviderError(RuntimeError):
    """A body-free provider failure safe to return to the WyrmGrid host."""

    SAFE_CODES = {
        "offline",
        "timed_out",
        "rate_limited",
        "provider_unavailable",
        "invalid_response",
        "no_data",
    }

    def __init__(self, code: str) -> None:
        if code not in self.SAFE_CODES:
            code = "provider_unavailable"
        super().__init__(code)
        self.code = code


class _NoRedirect(request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


class HttpsClient:
    """Bounded HTTPS access restricted to host-approved exact origins."""

    def __init__(self, origins: list[str], user_agent: str) -> None:
        self._origins = {_normalize_origin(origin) for origin in origins}
        self._opener = request.build_opener(_NoRedirect())
        self._user_agent = user_agent

    def get_json(
        self,
        origin: str,
        path: str,
        query: dict[str, str] | None = None,
        maximum_bytes: int = 512 * 1024,
    ) -> Any:
        body = self.get_bytes(
            origin,
            path,
            query=query,
            maximum_bytes=maximum_bytes,
            accepted_content_types=("application/json", "text/json"),
        )
        if body == b"":
            return None
        try:
            return json.loads(body.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as cause:
            raise ProviderError("invalid_response") from cause

    def get_bytes(
        self,
        origin: str,
        path: str,
        query: dict[str, str] | None = None,
        maximum_bytes: int = 512 * 1024,
        accepted_content_types: tuple[str, ...] = (),
    ) -> bytes:
        normalized_origin = _normalize_origin(origin)
        if normalized_origin not in self._origins:
            raise ProviderError("provider_unavailable")
        if (
            not isinstance(path, str)
            or not path.startswith("/")
            or path.startswith("//")
            or "#" in path
            or "?" in path
        ):
            raise ProviderError("invalid_response")
        if not isinstance(maximum_bytes, int) or not (
            1 <= maximum_bytes <= MAX_HTTP_RESPONSE_BYTES
        ):
            raise ProviderError("invalid_response")

        url = normalized_origin + path
        if query:
            url += "?" + parse.urlencode(query)
        outgoing = request.Request(
            url,
            headers={
                "Accept": ", ".join(accepted_content_types) if accepted_content_types else "*/*",
                "User-Agent": self._user_agent,
            },
            method="GET",
        )
        try:
            with self._opener.open(outgoing, timeout=HTTP_TIMEOUT_SECONDS) as response:
                status = response.getcode()
                if status == 204:
                    return b""
                if status != 200:
                    raise ProviderError(_status_code(status))
                content_length = response.headers.get("Content-Length")
                if content_length is not None:
                    try:
                        if int(content_length) > maximum_bytes:
                            raise ProviderError("invalid_response")
                    except ValueError as cause:
                        raise ProviderError("invalid_response") from cause
                content_type = response.headers.get_content_type().lower()
                if accepted_content_types and content_type not in accepted_content_types:
                    raise ProviderError("invalid_response")
                body = response.read(maximum_bytes + 1)
                if len(body) > maximum_bytes:
                    raise ProviderError("invalid_response")
                return body
        except ProviderError:
            raise
        except error.HTTPError as cause:
            raise ProviderError(_status_code(cause.code)) from cause
        except (TimeoutError, socket.timeout) as cause:
            raise ProviderError("timed_out") from cause
        except (error.URLError, OSError) as cause:
            reason = getattr(cause, "reason", None)
            if isinstance(reason, (TimeoutError, socket.timeout)):
                raise ProviderError("timed_out") from cause
            raise ProviderError("offline") from cause


def _normalize_origin(origin: str) -> str:
    if not isinstance(origin, str):
        raise ProviderError("provider_unavailable")
    parsed = parse.urlsplit(origin)
    if (
        parsed.scheme != "https"
        or not parsed.hostname
        or parsed.username is not None
        or parsed.password is not None
        or parsed.path not in ("", "/")
        or parsed.query
        or parsed.fragment
    ):
        raise ProviderError("provider_unavailable")
    port = f":{parsed.port}" if parsed.port is not None else ""
    return f"https://{parsed.hostname.lower()}{port}"


def _status_code(status: int) -> str:
    if status == 429:
        return "rate_limited"
    if status >= 500:
        return "provider_unavailable"
    return "invalid_response"


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
        on_fleet_snapshot: Callable[[dict[str, Any]], dict[str, Any] | None]
        | None = None,
        on_simulator_telemetry: Callable[
            [dict[str, Any]], dict[str, Any] | None
        ]
        | None = None,
        on_weather_request: Callable[
            [dict[str, Any], HttpsClient], dict[str, Any] | None
        ]
        | None = None,
    ) -> None:
        if (
            on_fleet_snapshot is None
            and on_simulator_telemetry is None
            and on_weather_request is None
        ):
            raise ValueError("at least one plugin callback is required")
        self.plugin_id = plugin_id
        self.on_fleet_snapshot = on_fleet_snapshot
        self.on_simulator_telemetry = on_simulator_telemetry
        self.on_weather_request = on_weather_request
        self._outgoing_sequence = 1
        self._incoming_sequence = 0
        self._grants: set[str] = set()
        self._weather_capabilities: set[str] = set()
        self._http: HttpsClient | None = None

    def run(self) -> None:
        hello = self._receive("hello")
        if hello.get("plugin_id") != self.plugin_id:
            raise ProtocolError("host addressed a different plugin")
        self._grants = set(hello.get("granted_permissions", []))
        self._weather_capabilities = set(hello.get("weather_capabilities", []))
        self._http = HttpsClient(
            hello.get("network_origins", []),
            f"OnAir-WyrmGrid/{hello.get('host_version', 'unknown')} ({self.plugin_id})",
        )
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
            if message_type == "fleet_snapshot":
                if "on_air_fleet_read" not in self._grants:
                    raise ProtocolError("fleet snapshot received without permission")
                if self.on_fleet_snapshot is None:
                    continue
                layer = self.on_fleet_snapshot(payload["snapshot"])
                self._publish_layer(layer)
                continue
            if message_type == "simulator_telemetry_snapshot":
                if "simulator_telemetry_read" not in self._grants:
                    raise ProtocolError(
                        "simulator telemetry received without permission"
                    )
                if self.on_simulator_telemetry is None:
                    continue
                layer = self.on_simulator_telemetry(payload["snapshot"])
                self._publish_layer(layer)
                continue
            if message_type == "weather_request":
                if "weather_data_publish" not in self._grants:
                    raise ProtocolError("weather request received without permission")
                weather_request = payload.get("request")
                if not isinstance(weather_request, dict):
                    raise ProtocolError("weather request is malformed")
                query = weather_request.get("query")
                capability = query.get("kind") if isinstance(query, dict) else None
                if capability not in self._weather_capabilities:
                    raise ProtocolError("weather request exceeds declared capabilities")
                if self.on_weather_request is None or self._http is None:
                    self._publish_weather(weather_request, None, "provider_unavailable")
                    continue
                try:
                    product = self.on_weather_request(weather_request, self._http)
                    self._publish_weather(weather_request, product)
                except ProviderError as failure:
                    self._publish_weather(weather_request, None, failure.code)
                except Exception:
                    self._publish_weather(weather_request, None, "invalid_response")
                continue
            else:
                raise ProtocolError("unsupported host message")

    def _publish_layer(self, layer: dict[str, Any] | None) -> None:
        if layer is not None:
            if "map_layers_publish" not in self._grants:
                raise ProtocolError("map layer publication is not granted")
            self._send({"type": "publish_map_layer", "layer": layer})

    def _publish_weather(
        self,
        weather_request: dict[str, Any],
        product: dict[str, Any] | None,
        unavailable_code: str = "no_data",
    ) -> None:
        request_id = weather_request.get("id")
        if not isinstance(request_id, str):
            raise ProtocolError("weather request id is malformed")
        if product is None:
            response = {"status": "unavailable", "code": unavailable_code}
        else:
            response = {"status": "complete", "product": product}
        self._send(
            {
                "type": "publish_weather",
                "request_id": request_id,
                "response": response,
            }
        )

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
