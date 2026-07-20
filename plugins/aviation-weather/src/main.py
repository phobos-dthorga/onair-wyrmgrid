"""Translate AviationWeather.gov METAR and TAF facts into WyrmGrid snapshots."""

from datetime import datetime, timedelta, timezone
import math
import uuid

from wyrmgrid_sdk import Plugin, ProviderError

ORIGIN = "https://aviationweather.gov"
MAX_HISTORICAL_WINDOW_HOURS = 30


def _timestamp(value):
    try:
        if isinstance(value, int) and not isinstance(value, bool):
            return datetime.fromtimestamp(value, timezone.utc)
        if isinstance(value, str) and len(value) <= 40:
            return datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(
                timezone.utc
            )
    except (ValueError, OSError, OverflowError):
        pass
    raise ProviderError("invalid_response")


def _required_text(value, key, maximum):
    text = value.get(key) if isinstance(value, dict) else None
    if (
        not isinstance(text, str)
        or not text.strip()
        or len(text) > maximum
        or any(ord(character) < 32 and character not in "\r\n\t" for character in text)
    ):
        raise ProviderError("invalid_response")
    return text.strip()


def _optional_text(value, key, maximum):
    text = value.get(key) if isinstance(value, dict) else None
    if text is None:
        return None
    if not isinstance(text, str) or not text.strip() or len(text) > maximum:
        return None
    text = text.strip()
    return text if not any(ord(character) < 32 for character in text) else None


def _number(value, key, minimum, maximum):
    number = value.get(key) if isinstance(value, dict) else None
    if isinstance(number, bool) or not isinstance(number, (int, float)):
        return None
    number = float(number)
    return number if math.isfinite(number) and minimum <= number <= maximum else None


def _integer(value, key, maximum):
    number = value.get(key) if isinstance(value, dict) else None
    if isinstance(number, bool) or not isinstance(number, (int, float)):
        return None
    return int(number) if number == int(number) and 0 <= number <= maximum else None


def _freshness(generated_at, retrieved_at, valid_to=None):
    if valid_to is not None:
        return "stale" if valid_to < retrieved_at else "current"
    return "stale" if (retrieved_at - generated_at).total_seconds() > 7200 else "current"


def _provenance(generated_at, retrieved_at, valid_to=None):
    return {
        "kind": "external_fact",
        "provider": "aviationweather.gov",
        "provider_revision": "data-api-v4",
        "generated_at": generated_at.isoformat().replace("+00:00", "Z"),
        "retrieved_at": retrieved_at.isoformat().replace("+00:00", "Z"),
        "transformation_version": 1,
        "freshness": _freshness(generated_at, retrieved_at, valid_to),
    }


def _historical_window(query):
    value = query.get("window")
    if value is None:
        return None
    if not isinstance(value, dict):
        raise ProviderError("invalid_response")
    target = _timestamp(value.get("target_at"))
    starts = _timestamp(value.get("starts_at"))
    ends = _timestamp(value.get("ends_at"))
    if (
        not starts <= target <= ends
        or ends - starts > timedelta(hours=MAX_HISTORICAL_WINDOW_HOURS)
    ):
        raise ProviderError("invalid_response")
    return target, starts, ends


def _metars(values, requested, retrieved_at, historical=None):
    translated = {}
    if values is None:
        return translated
    if not isinstance(values, list):
        raise ProviderError("invalid_response")
    for value in values:
        station = _required_text(value, "icaoId", 4).upper()
        if station not in requested:
            continue
        observed_at = _timestamp(value.get("obsTime"))
        if historical is not None and not historical[1] <= observed_at <= historical[2]:
            continue
        direction_value = value.get("wdir")
        direction = None
        if isinstance(direction_value, str) and direction_value.upper() == "VRB":
            direction = {"kind": "variable"}
        elif (degrees := _integer(value, "wdir", 360)) is not None:
            direction = {"kind": "degrees", "value": degrees}
        category = _optional_text(value, "fltCat", 8)
        category = category.lower() if category and category.upper() in {"VFR", "MVFR", "IFR", "LIFR"} else None
        report = {
            "observed_at": observed_at.isoformat().replace("+00:00", "Z"),
            "raw_text": _required_text(value, "rawOb", 2048),
        }
        optional = {
            "report_type": (
                report_type.upper()
                if (report_type := _optional_text(value, "metarType", 16))
                else None
            ),
            "flight_category": category,
            "wind_direction": direction,
            "wind_speed_kt": _integer(value, "wspd", 300),
            "wind_gust_kt": _integer(value, "wgst", 400),
            "visibility_sm": str(value["visib"])[:24] if value.get("visib") is not None else None,
            "temperature_c": _number(value, "temp", -150, 100),
            "dewpoint_c": _number(value, "dewp", -150, 100),
            "altimeter_hpa": _number(value, "altim", 800, 1200),
            "present_weather": _optional_text(value, "wxString", 128),
        }
        report.update({key: item for key, item in optional.items() if item is not None})
        product = {
            "value": report,
            "provenance": _provenance(observed_at, retrieved_at),
        }
        existing = translated.get(station)
        rank = (
            observed_at
            if historical is None
            else -abs((observed_at - historical[0]).total_seconds())
        )
        if existing is None or existing[0] < rank:
            translated[station] = (rank, product)
    return {station: product for station, (_, product) in translated.items()}


def _tafs(values, requested, retrieved_at):
    translated = {}
    if values is None:
        return translated
    if not isinstance(values, list):
        raise ProviderError("invalid_response")
    for value in values:
        station = _required_text(value, "icaoId", 4).upper()
        if station not in requested:
            continue
        issued_at = _timestamp(value.get("issueTime"))
        valid_from = _timestamp(value.get("validTimeFrom"))
        valid_to = _timestamp(value.get("validTimeTo"))
        if valid_from >= valid_to:
            raise ProviderError("invalid_response")
        product = {
            "value": {
                "issued_at": issued_at.isoformat().replace("+00:00", "Z"),
                "valid_from": valid_from.isoformat().replace("+00:00", "Z"),
                "valid_to": valid_to.isoformat().replace("+00:00", "Z"),
                "raw_text": _required_text(value, "rawTAF", 32768),
            },
            "provenance": _provenance(issued_at, retrieved_at, valid_to),
        }
        existing = translated.get(station)
        if existing is None or existing[0] < issued_at:
            translated[station] = (issued_at, product)
    return {station: product for station, (_, product) in translated.items()}


def airport_reports(weather_request, http):
    query = weather_request.get("query") or {}
    historical = _historical_window(query)
    stations = query.get("stations")
    if not isinstance(stations, list) or not stations:
        raise ProviderError("invalid_response")
    requested = {station.upper() for station in stations if isinstance(station, str)}
    if len(requested) != len(stations):
        raise ProviderError("invalid_response")
    ids = ",".join(sorted(requested))
    retrieved_at = datetime.now(timezone.utc)
    metar_parameters = {"ids": ids, "format": "json"}
    if historical is not None:
        _, starts, ends = historical
        metar_parameters.update(
            {
                "date": ends.isoformat().replace("+00:00", "Z"),
                "hours": max(1, math.ceil((ends - starts).total_seconds() / 3600)),
            }
        )
    metar_values = http.get_json(
        ORIGIN, "/api/data/metar", metar_parameters
    )
    taf_values = (
        http.get_json(ORIGIN, "/api/data/taf", {"ids": ids, "format": "json"})
        if historical is None
        else None
    )
    metars = _metars(metar_values, requested, retrieved_at, historical)
    tafs = _tafs(taf_values, requested, retrieved_at)
    airports = []
    for station in sorted(requested):
        airport = {"station_icao": station}
        if station in metars:
            airport["metar"] = metars[station]
        if station in tafs:
            airport["taf"] = tafs[station]
        airports.append(airport)
    return {
        "kind": "airport_reports",
        "snapshot": {
            "schema_version": 1,
            "id": str(uuid.uuid4()),
            "airports": airports,
        },
    }


if __name__ == "__main__":
    Plugin(
        plugin_id="org.wyrmgrid.provider.aviation-weather",
        on_weather_request=airport_reports,
    ).run()
