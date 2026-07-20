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
        times = [f"2026-07-17T{hour:02d}:00" for hour in range(24)]
        location_count = len(query["latitude"].split(","))
        return [
            {
                "hourly": {
                    "time": times,
                    "temperature_2m": [18.5] * len(times),
                    "precipitation": [1.25] * len(times),
                    "weather_code": [61] * len(times),
                    "cloud_cover": [82] * len(times),
                    "wind_speed_10m": [14.0] * len(times),
                    "wind_direction_10m": [245] * len(times),
                    "provider_only": "must-not-cross-the-boundary",
                }
            }
            for _ in range(location_count)
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
                    {"time": 1784290200, "path": "/v2/radar/1784290200"},
                    {"time": 1784290800, "path": "/v2/radar/1784290800"},
                ]
            },
            "provider_only": "must-not-cross-the-boundary",
        }

    def get_bytes(
        self, origin, path, maximum_bytes, accepted_content_types
    ):
        self.tile_calls = getattr(self, "tile_calls", [])
        self.tile_calls.append((origin, path, maximum_bytes, accepted_content_types))
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

        points = product["layer"]["data"]["points"]
        self.assertEqual(len(points), 6)
        point = points[0]
        self.assertEqual(point["id"], "grid-01-h00")
        self.assertEqual(point["valid_at"], "2026-07-17T00:00:00Z")
        self.assertEqual(point["condition"], "rain")
        self.assertEqual(point["wind_speed_kt"], 14.0)
        self.assertNotIn("provider_only", json.dumps(product))
        self.assertEqual(client.call[1], "/v1/forecast")
        self.assertEqual(client.call[2]["hourly"], OPEN_METEO.FIELDS)
        self.assertEqual(client.call[2]["forecast_hours"], 19)
        self.assertNotIn("current", client.call[2])

    def test_open_meteo_rejects_malformed_hourly_series(self):
        class MalformedOpenMeteoClient(OpenMeteoClient):
            def get_json(self, origin, path, query):
                payload = super().get_json(origin, path, query)
                payload[0]["hourly"]["cloud_cover"] = [82]
                return payload

        with self.assertRaises(OPEN_METEO.ProviderError) as failure:
            OPEN_METEO.forecast_grid(
                {
                    "query": {
                        "kind": "forecast_grid",
                        "points": [
                            {
                                "id": "grid-01",
                                "location": {
                                    "latitude": -33.8688,
                                    "longitude": 151.2093,
                                },
                            }
                        ],
                    }
                },
                MalformedOpenMeteoClient(),
            )
        self.assertEqual(failure.exception.code, "invalid_response")

    def test_open_meteo_uses_the_bounded_historical_service_without_invented_extent(self):
        client = OpenMeteoClient()
        window = {
            "target_at": "2026-07-17T12:00:00Z",
            "starts_at": "2026-07-17T08:00:00Z",
            "ends_at": "2026-07-17T16:00:00Z",
        }
        product = OPEN_METEO.forecast_grid(
            {
                "query": {
                    "kind": "forecast_grid",
                    "points": [
                        {
                            "id": "grid-01",
                            "location": {
                                "latitude": -33.8688,
                                "longitude": 151.2093,
                            },
                        }
                    ],
                    "window": window,
                }
            },
            client,
        )

        self.assertEqual(client.call[0], OPEN_METEO.HISTORICAL_ORIGIN)
        self.assertNotIn("forecast_hours", client.call[2])
        self.assertEqual(client.call[2]["start_date"], "2026-07-17")
        self.assertEqual(product["layer"]["time_scope"]["kind"], "historical_model")
        self.assertEqual(product["layer"]["time_scope"]["target_at"], window["target_at"])
        self.assertEqual(len(product["layer"]["data"]["points"]), 6)
        self.assertTrue(
            all(
                "provider_extent_radius_nm" not in point
                for point in product["layer"]["data"]["points"]
            )
        )

    def test_open_meteo_keeps_six_horizons_under_the_domain_point_ceiling(self):
        requested = [
            {
                "id": f"grid-{index:02d}",
                "location": {"latitude": -75 + index, "longitude": -165},
            }
            for index in range(84)
        ]
        product = OPEN_METEO.forecast_grid(
            {"query": {"kind": "forecast_grid", "points": requested}},
            OpenMeteoClient(),
        )

        self.assertEqual(len(product["layer"]["data"]["points"]), 504)
        self.assertLessEqual(
            len(product["layer"]["data"]["points"]),
            512,
        )
        self.assertLess(len(json.dumps(product).encode("utf-8")), 1_048_576)

        with self.assertRaises(OPEN_METEO.ProviderError) as failure:
            OPEN_METEO.forecast_grid(
                {
                    "query": {
                        "kind": "forecast_grid",
                        "points": requested
                        + [
                            {
                                "id": "grid-84",
                                "location": {"latitude": 9, "longitude": -165},
                            },
                        ],
                    }
                },
                OpenMeteoClient(),
            )
        self.assertEqual(failure.exception.code, "invalid_response")

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

    def test_aviation_weather_historical_request_returns_only_windowed_observations(self):
        client = AviationWeatherClient()
        product = AVIATION_WEATHER.airport_reports(
            {
                "query": {
                    "kind": "airport_reports",
                    "stations": ["YSSY"],
                    "window": {
                        "target_at": "2026-07-14T01:00:00Z",
                        "starts_at": "2026-07-14T00:00:00Z",
                        "ends_at": "2026-07-14T03:00:00Z",
                    },
                }
            },
            client,
        )

        airport = product["snapshot"]["airports"][0]
        self.assertEqual(airport["metar"]["value"]["observed_at"], "2026-07-14T01:00:00Z")
        self.assertNotIn("taf", airport)

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
        self.assertTrue(
            base64.b64decode(tile["coverage_png_base64"]).startswith(b"\x89PNG")
        )
        self.assertNotIn("provider_only", json.dumps(product))
        self.assertEqual(client.tile_calls[0][0], "https://tilecache.rainviewer.com")
        self.assertIn("/v2/radar/1784290800", client.tile_calls[0][1])
        self.assertIn("/v2/coverage/0/", client.tile_calls[1][1])

    def test_rainviewer_selects_a_bounded_factual_past_frame(self):
        client = RainViewerClient()
        product = RAINVIEWER.radar_tiles(
            {
                "query": {
                    "kind": "radar_tiles",
                    "frame_offset": 1,
                    "tiles": [{"zoom": 1, "x": 1, "y": 0}],
                }
            },
            client,
        )

        self.assertEqual(
            product["layer"]["data"]["frame_time"], "2026-07-17T12:10:00Z"
        )
        self.assertIn("/v2/radar/1784290200", client.tile_calls[0][1])

    def test_rainviewer_rejects_an_unavailable_or_excessive_offset(self):
        for offset, code in ((2, "no_data"), (6, "invalid_response")):
            with self.subTest(offset=offset):
                with self.assertRaises(RAINVIEWER.ProviderError) as failure:
                    RAINVIEWER.radar_tiles(
                        {
                            "query": {
                                "kind": "radar_tiles",
                                "frame_offset": offset,
                                "tiles": [{"zoom": 1, "x": 1, "y": 0}],
                            }
                        },
                        RainViewerClient(),
                    )
                self.assertEqual(failure.exception.code, code)


if __name__ == "__main__":
    unittest.main()
