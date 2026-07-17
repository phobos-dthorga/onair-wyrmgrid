use serde::Serialize;
use wyrmgrid_domain::{Coordinates, FlightPlanSnapshot, OperationalProvenance};

pub const FLIGHT_PLAN_MAP_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightPlanMapPointKind {
    Origin,
    RouteLeg,
    Destination,
    Alternate,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlightPlanMapPoint {
    pub id: String,
    pub kind: FlightPlanMapPointKind,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub airway: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Coordinates>,
    pub on_route: bool,
    pub gap_before: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlightPlanMapView {
    pub schema_version: u32,
    pub plan_id: String,
    pub origin_icao: String,
    pub destination_icao: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub airac: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
    pub provenance: OperationalProvenance,
    pub points: Vec<FlightPlanMapPoint>,
}

pub(crate) fn build_flight_plan_map_view(plan: &FlightPlanSnapshot) -> FlightPlanMapView {
    let airports = &plan.airports.value;
    let route = plan.route.as_ref();
    let mut points = Vec::new();
    let mut gap_pending = airports.origin.location.is_none();

    points.push(FlightPlanMapPoint {
        id: point_id("origin", None, &airports.origin.icao),
        kind: FlightPlanMapPointKind::Origin,
        label: airports.origin.icao.clone(),
        sequence: None,
        airway: None,
        location: airports.origin.location,
        on_route: true,
        gap_before: false,
    });

    if let Some(route) = route {
        if route.value.legs.is_empty() {
            gap_pending = true;
        }
        for leg in &route.value.legs {
            let location = leg.location;
            points.push(FlightPlanMapPoint {
                id: point_id("route", Some(leg.sequence), &leg.ident),
                kind: FlightPlanMapPointKind::RouteLeg,
                label: leg.ident.clone(),
                sequence: Some(leg.sequence),
                airway: leg.airway.clone(),
                location,
                on_route: true,
                gap_before: gap_pending,
            });
            gap_pending = location.is_none();
        }
    } else {
        gap_pending = true;
    }

    points.push(FlightPlanMapPoint {
        id: point_id("destination", None, &airports.destination.icao),
        kind: FlightPlanMapPointKind::Destination,
        label: airports.destination.icao.clone(),
        sequence: None,
        airway: None,
        location: airports.destination.location,
        on_route: true,
        gap_before: gap_pending,
    });

    points.extend(
        airports
            .alternates
            .iter()
            .enumerate()
            .map(|(index, airport)| FlightPlanMapPoint {
                id: point_id("alternate", Some(index as u32), &airport.icao),
                kind: FlightPlanMapPointKind::Alternate,
                label: airport.icao.clone(),
                sequence: Some(index as u32),
                airway: None,
                location: airport.location,
                on_route: false,
                gap_before: false,
            }),
    );

    FlightPlanMapView {
        schema_version: FLIGHT_PLAN_MAP_SCHEMA_VERSION,
        plan_id: plan.id.0.to_string(),
        origin_icao: airports.origin.icao.clone(),
        destination_icao: airports.destination.icao.clone(),
        airac: plan.identity.value.airac.clone(),
        source_text: route.and_then(|route| route.value.source_text.clone()),
        provenance: route
            .map(|route| route.provenance.clone())
            .unwrap_or_else(|| plan.airports.provenance.clone()),
        points,
    }
}

fn point_id(kind: &str, sequence: Option<u32>, label: &str) -> String {
    let normalized = label
        .chars()
        .filter_map(|character| {
            if character.is_ascii_alphanumeric() {
                Some(character.to_ascii_lowercase())
            } else if character == '-' || character == '_' {
                Some('-')
            } else {
                None
            }
        })
        .take(32)
        .collect::<String>();
    match sequence {
        Some(sequence) => format!("{kind}:{sequence:04}:{normalized}"),
        None => format!("{kind}:{normalized}"),
    }
}

#[cfg(test)]
#[path = "tests/flight_plan_map.rs"]
mod tests;
