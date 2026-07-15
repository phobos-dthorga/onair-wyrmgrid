use std::collections::HashMap;

use serde::Serialize;
use wyrmgrid_domain::{
    Coordinates, FlightPlanAirport, FlightPlanSnapshot, MetarObservation, OperationalObservation,
    TafForecast, WeatherSnapshot,
};

pub const FLIGHT_WEATHER_MAP_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightWeatherMapStationRole {
    Origin,
    Destination,
    Alternate,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlightWeatherMapStation {
    pub id: String,
    pub role: FlightWeatherMapStationRole,
    pub station_icao: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Coordinates>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metar: Option<OperationalObservation<MetarObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taf: Option<OperationalObservation<TafForecast>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlightWeatherMapView {
    pub schema_version: u32,
    pub plan_id: String,
    pub weather_snapshot_id: String,
    pub stations: Vec<FlightWeatherMapStation>,
}

pub(crate) fn build_flight_weather_map_view(
    plan: &FlightPlanSnapshot,
    weather: &WeatherSnapshot,
) -> FlightWeatherMapView {
    let reports = weather
        .airports
        .iter()
        .map(|airport| (airport.station_icao.as_str(), airport))
        .collect::<HashMap<_, _>>();
    let airports = &plan.airports.value;
    let mut stations = vec![map_station(
        &airports.origin,
        FlightWeatherMapStationRole::Origin,
        None,
        &reports,
    )];
    stations.push(map_station(
        &airports.destination,
        FlightWeatherMapStationRole::Destination,
        None,
        &reports,
    ));
    stations.extend(
        airports
            .alternates
            .iter()
            .enumerate()
            .map(|(index, airport)| {
                map_station(
                    airport,
                    FlightWeatherMapStationRole::Alternate,
                    Some(index as u32),
                    &reports,
                )
            }),
    );

    FlightWeatherMapView {
        schema_version: FLIGHT_WEATHER_MAP_SCHEMA_VERSION,
        plan_id: plan.id.0.to_string(),
        weather_snapshot_id: weather.id.0.to_string(),
        stations,
    }
}

fn map_station(
    airport: &FlightPlanAirport,
    role: FlightWeatherMapStationRole,
    sequence: Option<u32>,
    reports: &HashMap<&str, &wyrmgrid_domain::AirportWeather>,
) -> FlightWeatherMapStation {
    let role_name = match role {
        FlightWeatherMapStationRole::Origin => "origin",
        FlightWeatherMapStationRole::Destination => "destination",
        FlightWeatherMapStationRole::Alternate => "alternate",
    };
    let suffix = sequence.map_or_else(String::new, |value| format!(":{value:04}"));
    let report = reports.get(airport.icao.as_str()).copied();
    FlightWeatherMapStation {
        id: format!(
            "weather:{role_name}{suffix}:{}",
            airport.icao.to_ascii_lowercase()
        ),
        role,
        station_icao: airport.icao.clone(),
        location: airport.location,
        metar: report.and_then(|report| report.metar.clone()),
        taf: report.and_then(|report| report.taf.clone()),
    }
}

#[cfg(test)]
#[path = "tests/flight_weather_map.rs"]
mod tests;
