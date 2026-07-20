use super::*;
use crate::AircraftMatchBasis;
use uuid::Uuid;
use wyrmgrid_domain::{
    AircraftId, AircraftSummary, AirportId, AirportSummary, CompanyId, FlightPlanAirports,
    FlightPlanIdentity, FlightPlanSnapshot, FlightPlanSnapshotId, JobSnapshot, Observed,
    OperationalObservation, PlannedAircraft, Provenance,
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

fn plan_with_aircraft(destination: &str, registration: &str) -> FlightPlanSnapshot {
    let mut plan = plan(destination);
    plan.aircraft = Some(OperationalObservation {
        value: PlannedAircraft {
            icao_type: Some("B738".into()),
            registration: Some(registration.into()),
            model: Some("Boeing 737-800".into()),
        },
        provenance: plan.identity.provenance.clone(),
    });
    plan
}

fn fleet(
    registration: &str,
    current_airport: Option<&str>,
    availability: SnapshotAvailability,
) -> FleetSnapshotView {
    let observed_at = DateTime::from_timestamp(1_784_236_800, 0).unwrap();
    FleetSnapshotView {
        company_id: CompanyId(Uuid::from_u128(7)),
        company: crate::ConnectedCompany {
            name: "Synthetic Air".into(),
            airline_code: "WYR".into(),
        },
        snapshot: Observed {
            value: vec![AircraftSummary {
                id: AircraftId(Uuid::from_u128(42)),
                registration: Some(registration.into()),
                model: Some("Boeing 737-800".into()),
                location: None,
                current_airport: current_airport.map(|icao| AirportSummary {
                    id: AirportId(Uuid::from_u128(84)),
                    icao: Some(icao.into()),
                    name: None,
                    location: None,
                }),
            }],
            provenance: Provenance {
                kind: ProvenanceKind::OnAirFact,
                source: "OnAir".into(),
                observed_at,
            },
        },
        availability,
        storage: crate::SnapshotStorage::Hoard,
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
            time_basis: crate::RouteWeatherTemporalMode::Live,
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
                staff: true,
            },
            None,
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
                staff: true,
            },
            None,
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
                staff: false,
            },
            None,
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
                staff: true,
            },
            None,
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
                staff: false,
            },
            None,
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
                staff: true,
            },
            None,
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
                staff: true,
            },
            None,
        )
        .unwrap();

    assert!(restarted.operation.is_some());
    assert_eq!(
        restarted.operation_change,
        FlightOperationContextChange::None
    );
}

#[test]
fn fleet_reconciliation_uses_the_accepted_plan_and_marks_unverified_capacity() {
    let service = FlightOperationService::new(Store::open_in_memory().unwrap());
    let accepted = status(
        Some(plan_with_aircraft("NZAA", "VH-WYR")),
        Some(selected_job()),
    );
    service.start_from_dispatch(&accepted).unwrap();

    let current_fleet = fleet("VH-WYR", Some("YSSY"), SnapshotAvailability::Live);
    let mut changed_session = status(
        Some(plan_with_aircraft("NZAA", "VH-NEW")),
        Some(selected_job()),
    );
    service
        .enrich_dispatch_status(
            &mut changed_session,
            FlightOperationAvailability {
                jobs: true,
                staff: false,
            },
            Some(&current_fleet),
        )
        .unwrap();

    let reconciliation = &changed_session
        .operation
        .as_ref()
        .unwrap()
        .fleet_reconciliation;
    assert_eq!(
        reconciliation.candidate.as_ref().unwrap().registration,
        Some("VH-WYR".into())
    );
    assert_eq!(
        reconciliation.candidate.as_ref().unwrap().id,
        Uuid::from_u128(42).to_string()
    );
    assert!(reconciliation.findings.iter().any(|finding| {
        finding.category == DispatchFindingCategory::AircraftPayloadCapacity
            && finding.status == DispatchFindingStatus::Unavailable
    }));
    assert_eq!(
        stage_state(&changed_session, FlightOperationStage::Fleet),
        FlightOperationStageState::NeedsAttention
    );
}

#[test]
fn stale_fleet_evidence_keeps_reconciliation_distinct_from_current_attention() {
    let service = FlightOperationService::new(Store::open_in_memory().unwrap());
    let mut dispatch = status(Some(plan_with_aircraft("NZAA", "VH-WYR")), None);
    service.start_from_dispatch(&dispatch).unwrap();
    let current_fleet = fleet("VH-WYR", Some("YSSY"), SnapshotAvailability::Offline);

    service
        .enrich_dispatch_status(
            &mut dispatch,
            FlightOperationAvailability {
                jobs: false,
                staff: false,
            },
            Some(&current_fleet),
        )
        .unwrap();

    assert_eq!(
        stage_state(&dispatch, FlightOperationStage::Fleet),
        FlightOperationStageState::Stale
    );
    assert_eq!(
        dispatch
            .operation
            .unwrap()
            .fleet_reconciliation
            .provenance
            .freshness,
        SnapshotFreshness::Stale
    );
}

#[test]
fn missing_or_unmatched_fleet_evidence_never_claims_reconciliation_ready() {
    let service = FlightOperationService::new(Store::open_in_memory().unwrap());
    let mut dispatch = status(Some(plan_with_aircraft("NZAA", "VH-WYR")), None);
    service.start_from_dispatch(&dispatch).unwrap();

    service
        .enrich_dispatch_status(
            &mut dispatch,
            FlightOperationAvailability {
                jobs: false,
                staff: false,
            },
            None,
        )
        .unwrap();
    assert_eq!(
        stage_state(&dispatch, FlightOperationStage::Fleet),
        FlightOperationStageState::Unavailable
    );

    let unmatched_fleet = fleet("VH-OTHER", Some("YSSY"), SnapshotAvailability::Live);
    service
        .enrich_dispatch_status(
            &mut dispatch,
            FlightOperationAvailability {
                jobs: false,
                staff: false,
            },
            Some(&unmatched_fleet),
        )
        .unwrap();
    assert_eq!(
        stage_state(&dispatch, FlightOperationStage::Fleet),
        FlightOperationStageState::NeedsAttention
    );
    assert!(
        dispatch
            .operation
            .unwrap()
            .fleet_reconciliation
            .candidate
            .is_none()
    );
}

#[test]
fn reviewed_aircraft_assignment_survives_restart_and_later_plan_revisions() {
    let store = Store::open_in_memory().unwrap();
    let service = FlightOperationService::new(store.clone());
    let mut dispatch = status(Some(plan_with_aircraft("NZAA", "VH-WYR")), None);
    service.start_from_dispatch(&dispatch).unwrap();
    let current_fleet = fleet("VH-WYR", Some("YSSY"), SnapshotAvailability::Live);
    let aircraft_id = Uuid::from_u128(42).to_string();
    service
        .assign_aircraft(&aircraft_id, Some(&current_fleet))
        .unwrap();

    dispatch.snapshot = Some(plan_with_aircraft("NZAA", "VH-NEW"));
    service.revise_from_dispatch(&dispatch).unwrap();

    let restarted = FlightOperationService::new(store);
    let mut restarted_status = status(None, None);
    restarted
        .enrich_dispatch_status(
            &mut restarted_status,
            FlightOperationAvailability {
                jobs: false,
                staff: false,
            },
            Some(&current_fleet),
        )
        .unwrap();
    let operation = restarted_status.operation.unwrap();
    let assignment = operation.aircraft_assignment.unwrap();
    assert_eq!(assignment.revision, 1);
    assert_eq!(assignment.id, aircraft_id);
    assert_eq!(operation.revision, 2);
    assert_eq!(
        operation
            .fleet_reconciliation
            .candidate
            .as_ref()
            .unwrap()
            .basis,
        AircraftMatchBasis::ReviewedAssignment
    );
}

#[test]
fn aircraft_reassignment_and_clearing_are_append_only_reviewed_decisions() {
    let service = FlightOperationService::new(Store::open_in_memory().unwrap());
    let mut dispatch = status(Some(plan_with_aircraft("NZAA", "VH-WYR")), None);
    service.start_from_dispatch(&dispatch).unwrap();
    let first_fleet = fleet("VH-WYR", Some("YSSY"), SnapshotAvailability::Live);
    service
        .assign_aircraft(&Uuid::from_u128(42).to_string(), Some(&first_fleet))
        .unwrap();

    let mut second_fleet = first_fleet.clone();
    second_fleet.snapshot.value[0].id = AircraftId(Uuid::from_u128(43));
    second_fleet.snapshot.value[0].registration = Some("VH-NEW".into());
    service
        .assign_aircraft(&Uuid::from_u128(43).to_string(), Some(&second_fleet))
        .unwrap();
    service
        .enrich_dispatch_status(
            &mut dispatch,
            FlightOperationAvailability {
                jobs: false,
                staff: false,
            },
            Some(&second_fleet),
        )
        .unwrap();
    assert_eq!(
        dispatch
            .operation
            .as_ref()
            .unwrap()
            .aircraft_assignment
            .as_ref()
            .unwrap()
            .revision,
        2
    );

    service.clear_aircraft_assignment().unwrap();
    assert_eq!(
        service.clear_aircraft_assignment(),
        Err(FlightOperationError::NoAircraftAssignment)
    );
    service
        .enrich_dispatch_status(
            &mut dispatch,
            FlightOperationAvailability {
                jobs: false,
                staff: false,
            },
            Some(&second_fleet),
        )
        .unwrap();
    let operation = dispatch.operation.unwrap();
    assert!(operation.aircraft_assignment.is_none());
    assert!(
        operation
            .fleet_reconciliation
            .findings
            .iter()
            .any(|finding| {
                finding.category == DispatchFindingCategory::AircraftAssignment
                    && finding.status == DispatchFindingStatus::Unavailable
            })
    );
}

#[test]
fn aircraft_assignment_rejects_stale_missing_duplicate_and_unobserved_evidence() {
    let service = FlightOperationService::new(Store::open_in_memory().unwrap());
    let mut dispatch = status(Some(plan_with_aircraft("NZAA", "VH-WYR")), None);
    service.start_from_dispatch(&dispatch).unwrap();
    assert_eq!(
        service.assign_aircraft(&Uuid::from_u128(42).to_string(), None),
        Err(FlightOperationError::FleetUnavailable)
    );
    let stale_fleet = fleet("VH-WYR", Some("YSSY"), SnapshotAvailability::Offline);
    assert_eq!(
        service.assign_aircraft(&Uuid::from_u128(42).to_string(), Some(&stale_fleet)),
        Err(FlightOperationError::FleetEvidenceStale)
    );
    let cached_fleet = fleet("VH-WYR", Some("YSSY"), SnapshotAvailability::Cached);
    assert_eq!(
        service.assign_aircraft(&Uuid::from_u128(42).to_string(), Some(&cached_fleet)),
        Err(FlightOperationError::FleetEvidenceStale)
    );
    let current_fleet = fleet("VH-WYR", Some("YSSY"), SnapshotAvailability::Live);
    assert_eq!(
        service.assign_aircraft(&Uuid::from_u128(99).to_string(), Some(&current_fleet)),
        Err(FlightOperationError::AircraftNotFound)
    );
    service
        .assign_aircraft(&Uuid::from_u128(42).to_string(), Some(&current_fleet))
        .unwrap();
    assert_eq!(
        service.assign_aircraft(&Uuid::from_u128(42).to_string(), Some(&current_fleet)),
        Err(FlightOperationError::AircraftAlreadyAssigned)
    );

    let mut missing_fleet = fleet("VH-OTHER", Some("YSSY"), SnapshotAvailability::Live);
    missing_fleet.snapshot.value[0].id = AircraftId(Uuid::from_u128(99));
    service
        .enrich_dispatch_status(
            &mut dispatch,
            FlightOperationAvailability {
                jobs: false,
                staff: false,
            },
            Some(&missing_fleet),
        )
        .unwrap();
    let operation = dispatch.operation.unwrap();
    assert!(operation.aircraft_assignment.is_some());
    assert!(operation.fleet_reconciliation.candidate.is_none());
    assert!(
        operation
            .fleet_reconciliation
            .findings
            .iter()
            .any(|finding| {
                finding.category == DispatchFindingCategory::AircraftAssignment
                    && finding.status == DispatchFindingStatus::Difference
            })
    );
}
