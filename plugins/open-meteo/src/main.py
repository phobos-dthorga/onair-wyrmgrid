"""Translate host-selected Open-Meteo model samples into WyrmGrid weather."""

from datetime import datetime, timezone
import math

from wyrmgrid_sdk import Plugin, ProviderError

ORIGIN = "https://api.open-meteo.com"
FIELDS = (
    "temperature_2m,precipitation,weather_code,cloud_cover,"
    "wind_speed_10m,wind_direction_10m"
)
FORECAST_HOURS = 19
FORECAST_HORIZON_INDEXES = (0, 3, 6, 9, 12, 18)
MAX_REQUESTED_LOCATIONS = 84


def _number(value, minimum, maximum):
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    value = float(value)
    return value if math.isfinite(value) and minimum <= value <= maximum else None


def _condition(code):
    if code == 0:
        return "clear"
    if code in (1, 2, 3):
        return "cloud"
    if code in (45, 48):
        return "obscuration"
    if code in (51, 53, 55, 56, 57, 61, 63, 65, 66, 67, 80, 81, 82):
        return "rain"
    if code in (71, 73, 75, 77, 85, 86):
        return "snow"
    if code in (95, 96, 99):
        return "convective"
    return "unknown"


def _timestamp(value):
    if not isinstance(value, str) or len(value) > 40:
        return None
    candidate = value
    if not value.endswith("Z") and "+" not in value[10:] and "-" not in value[10:]:
        candidate += "Z"
    try:
        return datetime.fromisoformat(candidate.replace("Z", "+00:00")).astimezone(
            timezone.utc
        )
    except ValueError:
        return None


def _hourly_series(hourly, key, expected_length):
    values = hourly.get(key)
    if not isinstance(values, list) or len(values) != expected_length:
        raise ProviderError("invalid_response")
    return values


def forecast_grid(weather_request, http):
    query = weather_request.get("query") or {}
    requested = query.get("points")
    if (
        not isinstance(requested, list)
        or not requested
        or len(requested) > MAX_REQUESTED_LOCATIONS
    ):
        raise ProviderError("invalid_response")

    translated = []
    for offset in range(0, len(requested), 40):
        chunk = requested[offset : offset + 40]
        latitudes = []
        longitudes = []
        for point in chunk:
            location = point.get("location") if isinstance(point, dict) else None
            if not isinstance(location, dict):
                raise ProviderError("invalid_response")
            latitudes.append(str(location.get("latitude")))
            longitudes.append(str(location.get("longitude")))
        payload = http.get_json(
            ORIGIN,
            "/v1/forecast",
            {
                "latitude": ",".join(latitudes),
                "longitude": ",".join(longitudes),
                "hourly": FIELDS,
                "forecast_hours": FORECAST_HOURS,
                "wind_speed_unit": "kn",
                "timezone": "UTC",
            },
        )
        responses = payload if isinstance(payload, list) else [payload]
        if len(responses) != len(chunk):
            raise ProviderError("invalid_response")
        for point, response in zip(chunk, responses, strict=True):
            hourly = response.get("hourly") if isinstance(response, dict) else None
            if not isinstance(hourly, dict):
                raise ProviderError("invalid_response")
            times = hourly.get("time")
            if not isinstance(times, list) or len(times) < FORECAST_HOURS:
                raise ProviderError("invalid_response")
            series = {
                key: _hourly_series(hourly, key, len(times))
                for key in FIELDS.split(",")
            }
            for horizon_index in FORECAST_HORIZON_INDEXES:
                valid_at = _timestamp(times[horizon_index])
                if valid_at is None:
                    raise ProviderError("invalid_response")
                code_value = series["weather_code"][horizon_index]
                code = (
                    int(code_value)
                    if isinstance(code_value, (int, float))
                    and not isinstance(code_value, bool)
                    and 0 <= code_value <= 65535
                    else None
                )
                sample = {
                    "id": f"{point.get('id')}-h{horizon_index:02d}",
                    "location": point.get("location"),
                    "valid_at": valid_at.isoformat().replace("+00:00", "Z"),
                    "condition": _condition(code),
                }
                optional = {
                    "temperature_c": _number(
                        series["temperature_2m"][horizon_index], -150, 100
                    ),
                    "precipitation_mm": _number(
                        series["precipitation"][horizon_index], 0, 1000
                    ),
                    "cloud_cover_percent": _number(
                        series["cloud_cover"][horizon_index], 0, 100
                    ),
                    "wind_direction_degrees": _number(
                        series["wind_direction_10m"][horizon_index], 0, 360
                    ),
                    "wind_speed_kt": _number(
                        series["wind_speed_10m"][horizon_index], 0, 500
                    ),
                    "provider_weather_code": code,
                }
                sample.update(
                    {key: value for key, value in optional.items() if value is not None}
                )
                translated.append(sample)

    if not translated:
        return None
    retrieved = datetime.now(timezone.utc)
    provenance = {
        "kind": "external_calculation",
        "provider": "open-meteo.com",
        "provider_revision": "forecast-api-v1-hourly",
        "retrieved_at": retrieved.isoformat().replace("+00:00", "Z"),
        "transformation_version": 2,
        "freshness": "current",
    }
    return {
        "kind": "global_layer",
        "layer": {
            "schema_version": 1,
            "id": "open-meteo-global",
            "title": "Global model weather",
            "data": {"kind": "grid", "points": translated},
            "provenance": provenance,
        },
    }


if __name__ == "__main__":
    Plugin(
        plugin_id="org.wyrmgrid.provider.open-meteo",
        on_weather_request=forecast_grid,
    ).run()
