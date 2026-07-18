"""Offline contract tests for the bundled weather-provider translators."""

from __future__ import annotations

import base64
import importlib.util
import json
from pathlib import Path
import sys
import unittest


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "sdk" / "python"))


def load_provider(name: str, relative_path: str):
    specification = importlib.util.spec_from_file_location(
        name, ROOT / relative_path
    )
    if specification is None or specification.loader is None:
        raise RuntimeError(f"Unable to load provider fixture: {name}")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


OPEN_METEO = load_provider("open_meteo_provider", "plugins/open-meteo/src/main.py")
AVIATION_WEATHER = load_provider(
    "aviation_weather_provider", "plugins/aviation-weather/src/main.py"
)
RAINVIEWER = load_provider("rainviewer_provider", "plugins/rainviewer/src/main.py")
AVIATION_WEATHER_FIXTURES = (
    ROOT / "plugins" / "aviation-weather" / "tests" / "fixtures"
)


class OpenMeteoClient:
    def get_json(self, origin, path, query):
        self.call = (origin, path, query)
        return [
            {
                "current": {
                    "time": "2026-07-17T12:00",
                    "temperature_2m": 18.5,
                    "precipitation": 1.25,
                    "weather_code": 61,
                    "cloud_cover": 82,
                    "wind_speed_10m": 14.0,
                    "wind_direction_10m": 245,
                    "provider_only": "must-not-cross-the-boundary",
                }
            }
        ]


class AviationWeatherClient:
    def get_json(self, origin, path, query):
        if path.endswith("/metar"):
            fixture = AVIATION_WEATHER_FIXTURES / "sanitized-metars.json"
            return json.loads(fixture.read_text(encoding="utf-8"))
        if path.endswith("/taf"):
            fixture = AVIATION_WEATHER_FIXTURES / "sanitized-tafs.json"
            return json.loads(fixture.read_text(encoding="utf-8"))
        raise AssertionError(path)


class RainViewerClient:
    def get_json(self, origin, path, maximum_bytes):
        return {
            "host": "https://tilecache.rainviewer.com",
            "radar": {
                "past": [
                    {"time": 1784290800, "path": "/v2/radar/1784290800"}
                ]
            },
            "provider_only": "must-not-cross-the-boundary",
        }

    def get_bytes(
        self, origin, path, maximum_bytes, accepted_content_types
    ):
        self.tile_call = (origin, path, maximum_bytes, accepted_content_types)
        return b"\x89PNG\r\n\x1a\nsynthetic-provider-test"


class WeatherProviderTests(unittest.TestCase):
    def test_open_meteo_translates_only_the_host_selected_grid(self):
        client = OpenMeteoClient()
        request = {
            "query": {
                "kind": "forecast_grid",
                "points": [
                    {
                        "id": "grid-01",
                        "location": {"latitude": -33.8688, "longitude": 151.2093},
                    }
                ],
            }
        }

        product = OPEN_METEO.forecast_grid(request, client)

        point = product["layer"]["data"]["points"][0]
        self.assertEqual(point["id"], "grid-01")
        self.assertEqual(point["condition"], "rain")
        self.assertEqual(point["wind_speed_kt"], 14.0)
        self.assertNotIn("provider_only", json.dumps(product))
        self.assertEqual(client.call[1], "/v1/forecast")

    def test_aviation_weather_translates_metar_and_taf_without_raw_objects(self):
        product = AVIATION_WEATHER.airport_reports(
            {"query": {"kind": "airport_reports", "stations": ["YSSY"]}},
            AviationWeatherClient(),
        )

        airport = product["snapshot"]["airports"][0]
        self.assertEqual(airport["station_icao"], "YSSY")
        self.assertEqual(airport["metar"]["value"]["report_type"], "METAR")
        self.assertEqual(airport["taf"]["value"]["valid_to"], "2026-07-15T06:00:00Z")
        self.assertNotIn("private", json.dumps(product).lower())

    def test_rainviewer_translates_only_requested_bounded_tiles(self):
        client = RainViewerClient()
        product = RAINVIEWER.radar_tiles(
            {
                "query": {
                    "kind": "radar_tiles",
                    "tiles": [{"zoom": 1, "x": 1, "y": 0}],
                }
            },
            client,
        )

        tile = product["layer"]["data"]["tiles"][0]
        self.assertEqual((tile["zoom"], tile["x"], tile["y"]), (1, 1, 0))
        self.assertTrue(base64.b64decode(tile["png_base64"]).startswith(b"\x89PNG"))
        self.assertNotIn("provider_only", json.dumps(product))
        self.assertEqual(client.tile_call[0], "https://tilecache.rainviewer.com")


if __name__ == "__main__":
    unittest.main()
