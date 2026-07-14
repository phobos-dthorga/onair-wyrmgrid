"""Publish known fleet positions as a host-rendered Atlas layer."""

from wyrmgrid_sdk import Plugin


def fleet_locations(snapshot):
    points = []
    for aircraft in snapshot.get("aircraft", []):
        location = aircraft.get("location")
        if not location:
            airport = aircraft.get("current_airport") or {}
            location = airport.get("location")
        if not location:
            continue

        aircraft_id = aircraft["id"]
        registration = aircraft.get("registration") or "Unregistered aircraft"
        model = aircraft.get("model") or "Unknown model"
        points.append(
            {
                "id": aircraft_id,
                "label": f"{registration} · {model}",
                "location": location,
            }
        )

    return {
        "id": "fleet-locations",
        "title": "Fleet locations",
        "points": points,
        "provenance": {
            "kind": "calculated",
            "source": "Fleet Locations example plugin",
            "observed_at": snapshot["provenance"]["observed_at"],
        },
    }


Plugin(
    plugin_id="org.wyrmgrid.example.fleet-locations",
    on_fleet_snapshot=fleet_locations,
).run()
