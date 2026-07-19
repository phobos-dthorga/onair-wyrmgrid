"""Translate the latest RainViewer radar frame into bounded PNG tiles."""

import base64
from datetime import datetime, timezone
import re

from wyrmgrid_sdk import Plugin, ProviderError

METADATA_ORIGIN = "https://api.rainviewer.com"
TILE_ORIGIN = "https://tilecache.rainviewer.com"
RADAR_PATH = re.compile(r"^/v2/radar/[0-9]{1,16}$")
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
MAX_FRAME_OFFSET = 5
MAX_PRODUCT_BYTES = 640 * 1024


def radar_tiles(weather_request, http):
    query = weather_request.get("query") or {}
    addresses = query.get("tiles")
    if not isinstance(addresses, list) or not addresses:
        raise ProviderError("invalid_response")
    frame_offset = query.get("frame_offset", 0)
    if (
        not isinstance(frame_offset, int)
        or isinstance(frame_offset, bool)
        or not 0 <= frame_offset <= MAX_FRAME_OFFSET
    ):
        raise ProviderError("invalid_response")
    metadata = http.get_json(
        METADATA_ORIGIN, "/public/weather-maps.json", maximum_bytes=128 * 1024
    )
    if not isinstance(metadata, dict) or metadata.get("host") != TILE_ORIGIN:
        raise ProviderError("invalid_response")
    radar = metadata.get("radar") if isinstance(metadata, dict) else None
    past = radar.get("past") if isinstance(radar, dict) else None
    if not isinstance(past, list) or not past:
        raise ProviderError("no_data")
    if frame_offset >= len(past):
        raise ProviderError("no_data")
    frame = past[-1 - frame_offset]
    path = frame.get("path") if isinstance(frame, dict) else None
    timestamp = frame.get("time") if isinstance(frame, dict) else None
    if not isinstance(path, str) or not RADAR_PATH.fullmatch(path):
        raise ProviderError("invalid_response")
    if not isinstance(timestamp, int) or isinstance(timestamp, bool):
        raise ProviderError("invalid_response")
    try:
        frame_time = datetime.fromtimestamp(timestamp, timezone.utc)
    except (ValueError, OSError, OverflowError) as cause:
        raise ProviderError("invalid_response") from cause

    tiles = []
    total_bytes = 0
    for address in addresses:
        if not isinstance(address, dict):
            raise ProviderError("invalid_response")
        zoom, x, y = address.get("zoom"), address.get("x"), address.get("y")
        if not all(isinstance(value, int) and not isinstance(value, bool) for value in (zoom, x, y)):
            raise ProviderError("invalid_response")
        png = http.get_bytes(
            TILE_ORIGIN,
            f"{path}/256/{zoom}/{x}/{y}/2/1_1.png",
            maximum_bytes=192 * 1024,
            accepted_content_types=("image/png",),
        )
        if not png.startswith(PNG_SIGNATURE):
            raise ProviderError("invalid_response")
        coverage_png = http.get_bytes(
            TILE_ORIGIN,
            f"/v2/coverage/0/256/{zoom}/{x}/{y}/0/0_0.png",
            maximum_bytes=192 * 1024,
            accepted_content_types=("image/png",),
        )
        if not coverage_png.startswith(PNG_SIGNATURE):
            raise ProviderError("invalid_response")
        total_bytes += len(png) + len(coverage_png)
        if total_bytes > MAX_PRODUCT_BYTES:
            raise ProviderError("invalid_response")
        tiles.append(
            {
                "zoom": zoom,
                "x": x,
                "y": y,
                "png_base64": base64.b64encode(png).decode("ascii"),
                "coverage_png_base64": base64.b64encode(coverage_png).decode(
                    "ascii"
                ),
            }
        )

    retrieved_at = datetime.now(timezone.utc)
    return {
        "kind": "global_layer",
        "layer": {
            "schema_version": 1,
            "id": "rainviewer-radar",
            "title": "Global precipitation radar",
            "data": {
                "kind": "raster_tiles",
                "frame_time": frame_time.isoformat().replace("+00:00", "Z"),
                "tiles": tiles,
            },
            "provenance": {
                "kind": "external_fact",
                "provider": "rainviewer.com",
                "provider_revision": "weather-maps-v2",
                "generated_at": frame_time.isoformat().replace("+00:00", "Z"),
                "retrieved_at": retrieved_at.isoformat().replace("+00:00", "Z"),
                "transformation_version": 1,
                "freshness": "current",
            },
        },
    }


if __name__ == "__main__":
    Plugin(
        plugin_id="org.wyrmgrid.provider.rainviewer",
        on_weather_request=radar_tiles,
    ).run()
