use super::*;

#[test]
fn initial_journey_exposes_current_and_future_stages_without_false_readiness() {
    let journey = build_initial_flight_operation_journey(InitialJourneyEvidence {
        plan_provider_available: true,
        plan_available: true,
        weather_provider_available: true,
        weather_available: false,
        weather_stale: false,
        atlas_available: true,
    });

    assert_eq!(
        journey.schema_version,
        FLIGHT_OPERATION_JOURNEY_SCHEMA_VERSION
    );
    assert_eq!(journey.stages.len(), 8);
    assert_eq!(journey.stages[0].state, FlightOperationStageState::Ready);
    assert_eq!(
        journey.stages[1].state,
        FlightOperationStageState::Available
    );
    assert_eq!(
        journey.stages[2].state,
        FlightOperationStageState::NotStarted
    );
    assert_eq!(journey.stages[7].state, FlightOperationStageState::Ready);
}

#[test]
fn stale_weather_remains_distinct_from_ready_and_unavailable() {
    let journey = build_initial_flight_operation_journey(InitialJourneyEvidence {
        plan_provider_available: true,
        plan_available: true,
        weather_provider_available: true,
        weather_available: true,
        weather_stale: true,
        atlas_available: true,
    });

    assert_eq!(journey.stages[1].state, FlightOperationStageState::Stale);
}

#[test]
fn weather_is_unavailable_until_a_plan_and_provider_exist() {
    let journey = build_initial_flight_operation_journey(InitialJourneyEvidence {
        plan_provider_available: true,
        plan_available: false,
        weather_provider_available: true,
        weather_available: false,
        weather_stale: false,
        atlas_available: false,
    });

    assert_eq!(
        journey.stages[0].state,
        FlightOperationStageState::Available
    );
    assert_eq!(
        journey.stages[1].state,
        FlightOperationStageState::Unavailable
    );
    assert_eq!(
        journey.stages[7].state,
        FlightOperationStageState::Unavailable
    );
}
