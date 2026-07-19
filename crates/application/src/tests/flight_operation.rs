use super::*;
use uuid::Uuid;
use wyrmgrid_domain::{
    FlightPlanAirports, FlightPlanIdentity, FlightPlanSnapshot, FlightPlanSnapshotId, JobSnapshot,
    OperationalObservation,
};
use wyrmgrid_storage::Store;

fn plan(destination: &str) -> FlightPlanSnapshot {
    let observed_at = DateTime::from_timestamp(1_784_236_800, 0).unwrap();
    let provenance = OperationalProvenance {
        kind: ProvenanceKind::ExternalCalculation,
        provider: "SimBrief".into(),
        provider_revision: Some("2607".into()),
        generated_at: Some(observed_at),
        retrieved_at: observed_at,
        transformation_version: 1,
        freshness: SnapshotFreshness::Current,
    };
    FlightPlanSnapshot {
        schema_version: wyrmgrid_domain::FLIGHT_PLAN_SNAPSHOT_SCHEMA_VERSION,
        id: FlightPlanSnapshotId(Uuid::new_v4()),
        identity: OperationalObservation {
            value: FlightPlanIdentity {
                airac: Some("2607".into()),
                provider_plan_reference: None,
            },
            provenance: provenance.clone(),
        },
        airports: OperationalObservation {
            value: FlightPlanAirports {
                origin: wyrmgrid_domain::FlightPlanAirport {
                    icao: "YSSY".into(),
                    name: None,
                    location: None,
                    planned_runway: None,
                },
                destination: wyrmgrid_domain::FlightPlanAirport {
                    icao: destination.into(),
                    name: None,
                    location: None,
                    planned_runway: None,
                },
                alternates: Vec::new(),
            },
            provenance,
        },
        aircraft: None,
        schedule: None,
        weights: None,
        fuel: None,
        route: None,
    }
}

fn selected_job() -> DispatchJobSelection {
    let jobs: JobSnapshot = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/job-snapshot-v1.json"
    ))
    .unwrap();
    DispatchJobSelection {
        company_id: wyrmgrid_domain::CompanyId(Uuid::from_u128(1)),
        job: jobs.jobs[0].clone(),
        observed_at: Utc::now(),
        availability: SnapshotAvailability::Cached,
    }
}

fn status(plan: Option<FlightPlanSnapshot>, job: Option<DispatchJobSelection>) -> DispatchStatus {
    DispatchStatus {
        provider_available: true,
        availability: if plan.is_some() {
            crate::DispatchAvailability::Ready
        } else {
            crate::DispatchAvailability::Empty
        },
        persistence: crate::DispatchPersistence::SessionOnly,
        importing: false,
        snapshot: plan,
        atlas_plan: None,
        atlas_weather: None,
        route_weather: None,
        journey: build_initial_flight_operation_journey(InitialJourneyEvidence {
            plan_provider_available: true,
            plan_available: true,
            weather_provider_available: true,
            weather_available: false,
            weather_stale: false,
            atlas_available: false,
        }),
        atlas_route: None,
        comparison: None,
        selected_job: job,
        operation: None,
        operation_change: FlightOperationContextChange::None,
        weather: crate::DispatchWeatherStatus {
            provider_available: true,
            availability: crate::DispatchWeatherAvailability::NotRequested,
            refreshing: false,
            cache: crate::DispatchWeatherCacheState::None,
            snapshot: None,
        },
    }
}

fn stage_state(
    status: &DispatchStatus,
    expected: FlightOperationStage,
) -> FlightOperationStageState {
    status
        .journey
        .stages
        .iter()
        .find(|stage| stage.stage == expected)
        .unwrap()
        .state
}

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

#[test]
fn starting_an_operation_persists_a_manifest_without_inventing_missing_loads() {
    let store = Store::open_in_memory().unwrap();
    let service = FlightOperationService::new(store);
    let mut dispatch = status(Some(plan("NZAA")), Some(selected_job()));

    service.start_from_dispatch(&dispatch).unwrap();
    service
        .enrich_dispatch_status(
            &mut dispatch,
            FlightOperationAvailability {
                jobs: true,
                fleet: true,
                staff: true,
            },
        )
        .unwrap();

    let operation = dispatch.operation.as_ref().unwrap();
    assert_eq!(operation.revision, 1);
    assert_eq!(operation.reason, FlightOperationRevisionReason::Initial);
    assert_eq!(operation.destination, "NZAA");
    assert_eq!(
        operation.manifest.legs.len(),
        dispatch.selected_job.as_ref().unwrap().job.legs.len()
    );
    assert_eq!(
        stage_state(&dispatch, FlightOperationStage::Jobs),
        FlightOperationStageState::Ready
    );
    assert!(matches!(
        stage_state(&dispatch, FlightOperationStage::Manifest),
        FlightOperationStageState::Ready | FlightOperationStageState::NeedsAttention
    ));
}

#[test]
fn selecting_session_job_marks_jobs_ready_without_claiming_a_manifest() {
    let service = FlightOperationService::new(Store::open_in_memory().unwrap());
    let mut dispatch = status(Some(plan("NZAA")), Some(selected_job()));

    service
        .enrich_dispatch_status(
            &mut dispatch,
            FlightOperationAvailability {
                jobs: true,
                fleet: true,
                staff: true,
            },
        )
        .unwrap();

    assert!(dispatch.operation.is_none());
    assert_eq!(
        stage_state(&dispatch, FlightOperationStage::Jobs),
        FlightOperationStageState::Ready
    );
    assert_eq!(
        stage_state(&dispatch, FlightOperationStage::Manifest),
        FlightOperationStageState::NotStarted
    );
}

#[test]
fn starting_requires_a_real_plan_and_revision_requires_a_change() {
    let service = FlightOperationService::new(Store::open_in_memory().unwrap());
    assert_eq!(
        service.start_from_dispatch(&status(None, None)),
        Err(FlightOperationError::PlanRequired)
    );

    let dispatch = status(Some(plan("NZAA")), Some(selected_job()));
    service.start_from_dispatch(&dispatch).unwrap();
    assert_eq!(
        service.start_from_dispatch(&dispatch),
        Err(FlightOperationError::ActiveOperationExists)
    );
    assert_eq!(
        service.revise_from_dispatch(&dispatch),
        Err(FlightOperationError::NoRevisionChange)
    );
}

#[test]
fn revisions_keep_the_operation_identity_and_capture_plan_and_job_changes() {
    let service = FlightOperationService::new(Store::open_in_memory().unwrap());
    let initial = status(Some(plan("NZAA")), Some(selected_job()));
    service.start_from_dispatch(&initial).unwrap();
    let initial_id = service
        .load_active_revision()
        .unwrap()
        .unwrap()
        .operation_id;

    let mut revised_job = initial.selected_job.clone().unwrap();
    revised_job.job.legs[0].cargo_weight_lb = revised_job.job.legs[0]
        .cargo_weight_lb
        .map(|weight| weight + 100.0)
        .or(Some(100.0));
    let mut revised = status(Some(plan("WSSS")), Some(revised_job));
    service.revise_from_dispatch(&revised).unwrap();
    service
        .enrich_dispatch_status(
            &mut revised,
            FlightOperationAvailability {
                jobs: true,
                fleet: false,
                staff: false,
            },
        )
        .unwrap();

    let operation = revised.operation.unwrap();
    assert_eq!(operation.id, initial_id.0.to_string());
    assert_eq!(operation.revision, 2);
    assert_eq!(
        operation.reason,
        FlightOperationRevisionReason::PlanAndJobChanged
    );
    assert_eq!(operation.destination, "WSSS");
    assert_eq!(revised.operation_change, FlightOperationContextChange::None);
}

#[test]
fn a_same_id_job_fact_change_is_still_presented_for_explicit_revision() {
    let service = FlightOperationService::new(Store::open_in_memory().unwrap());
    let initial = status(Some(plan("NZAA")), Some(selected_job()));
    service.start_from_dispatch(&initial).unwrap();

    let mut changed = initial;
    changed.selected_job.as_mut().unwrap().job.legs[0].passengers = Some(99);
    service
        .enrich_dispatch_status(
            &mut changed,
            FlightOperationAvailability {
                jobs: true,
                fleet: true,
                staff: true,
            },
        )
        .unwrap();

    assert_eq!(changed.operation_change, FlightOperationContextChange::Job);
    assert_eq!(changed.operation.unwrap().revision, 1);
}

#[test]
fn a_plan_only_operation_can_reach_review_without_inventing_a_manifest() {
    let service = FlightOperationService::new(Store::open_in_memory().unwrap());
    let mut dispatch = status(Some(plan("NZAA")), None);

    service.start_from_dispatch(&dispatch).unwrap();
    service
        .enrich_dispatch_status(
            &mut dispatch,
            FlightOperationAvailability {
                jobs: true,
                fleet: false,
                staff: false,
            },
        )
        .unwrap();

    assert!(
        dispatch
            .operation
            .as_ref()
            .unwrap()
            .manifest
            .legs
            .is_empty()
    );
    assert_eq!(
        stage_state(&dispatch, FlightOperationStage::Manifest),
        FlightOperationStageState::Available
    );
    assert_eq!(
        stage_state(&dispatch, FlightOperationStage::Review),
        FlightOperationStageState::Available
    );
}

#[test]
fn the_same_job_identity_from_another_company_requires_review() {
    let service = FlightOperationService::new(Store::open_in_memory().unwrap());
    let initial = status(Some(plan("NZAA")), Some(selected_job()));
    service.start_from_dispatch(&initial).unwrap();

    let mut changed = initial;
    changed.selected_job.as_mut().unwrap().company_id =
        wyrmgrid_domain::CompanyId(Uuid::from_u128(2));
    service
        .enrich_dispatch_status(
            &mut changed,
            FlightOperationAvailability {
                jobs: true,
                fleet: true,
                staff: true,
            },
        )
        .unwrap();

    assert_eq!(changed.operation_change, FlightOperationContextChange::Job);
    assert_eq!(changed.operation.unwrap().revision, 1);
}

#[test]
fn restart_without_session_context_does_not_claim_the_operation_changed() {
    let service = FlightOperationService::new(Store::open_in_memory().unwrap());
    service
        .start_from_dispatch(&status(Some(plan("NZAA")), Some(selected_job())))
        .unwrap();

    let mut restarted = status(None, None);
    service
        .enrich_dispatch_status(
            &mut restarted,
            FlightOperationAvailability {
                jobs: true,
                fleet: true,
                staff: true,
            },
        )
        .unwrap();

    assert!(restarted.operation.is_some());
    assert_eq!(
        restarted.operation_change,
        FlightOperationContextChange::None
    );
}
