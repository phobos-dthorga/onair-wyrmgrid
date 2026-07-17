use super::*;
use wyrmgrid_domain::{Coordinates, FlightPlanSnapshot};

fn plan() -> FlightPlanSnapshot {
    serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/flight-plan-snapshot-v1.json"
    ))
    .unwrap()
}

#[test]
fn map_projection_keeps_stable_selectable_points_and_source_provenance() {
    let mut plan = plan();
    plan.airports.value.origin.location = Some(Coordinates {
        latitude: -33.9461,
        longitude: 151.1772,
    });
    plan.airports.value.destination.location = Some(Coordinates {
        latitude: -37.0081,
        longitude: 174.7917,
    });
    plan.airports.value.alternates[0].location = Some(Coordinates {
        latitude: -41.3272,
        longitude: 174.8053,
    });
    let first_leg = &mut plan.route.as_mut().unwrap().value.legs[0];
    first_leg.location = Some(Coordinates {
        latitude: -34.045,
        longitude: 151.03,
    });

    let view = build_flight_plan_map_view(&plan);

    assert_eq!(view.schema_version, FLIGHT_PLAN_MAP_SCHEMA_VERSION);
    assert_eq!(view.provenance.provider, "simbrief");
    assert_eq!(view.points[0].id, "origin:yssy");
    assert_eq!(view.points[1].id, "route:0000:tesat");
    assert_eq!(view.points.last().unwrap().id, "alternate:0000:nzwn");
    assert!(view.points[0].on_route);
    assert!(!view.points.last().unwrap().on_route);
}

#[test]
fn unresolved_fixes_remain_selectable_and_break_the_next_sourced_segment() {
    let mut plan = plan();
    let legs = &mut plan.route.as_mut().unwrap().value.legs;
    legs[0].location = Some(Coordinates {
        latitude: -34.045,
        longitude: 151.03,
    });
    legs[1].location = None;
    legs[2].location = Some(Coordinates {
        latitude: -36.52,
        longitude: 170.9,
    });

    let view = build_flight_plan_map_view(&plan);
    let unresolved = view
        .points
        .iter()
        .find(|point| point.id == "route:0001:lizzi")
        .unwrap();
    let resumed = view
        .points
        .iter()
        .find(|point| point.id == "route:0002:lunbi")
        .unwrap();

    assert_eq!(unresolved.location, None);
    assert!(resumed.gap_before);
}

#[test]
fn absent_route_does_not_claim_a_direct_origin_to_destination_segment() {
    let mut plan = plan();
    plan.route = None;
    plan.airports.value.origin.location = Some(Coordinates {
        latitude: -33.9461,
        longitude: 151.1772,
    });
    plan.airports.value.destination.location = Some(Coordinates {
        latitude: -37.0081,
        longitude: 174.7917,
    });

    let view = build_flight_plan_map_view(&plan);
    let destination = view
        .points
        .iter()
        .find(|point| point.kind == FlightPlanMapPointKind::Destination)
        .unwrap();

    assert!(destination.gap_before);
    assert_eq!(view.provenance, plan.airports.provenance);
}
