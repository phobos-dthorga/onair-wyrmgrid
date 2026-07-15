use super::*;
use std::sync::atomic::AtomicUsize;
use uuid::Uuid;
use wyrmgrid_domain::{
    AircraftId, AirportId, AirportSummary, FlightPlanAirport, FlightPlanAirports,
    FlightPlanIdentity, FlightPlanSnapshotId, JobSnapshot, Mass, MassUnit, Observed,
    OperationalObservation, PlannedAircraft, PlannedSchedule, PlannedWeights, Provenance,
    SnapshotFreshness, WEATHER_SNAPSHOT_SCHEMA_VERSION, WeatherSnapshotId,
};

struct FakeProvider {
    responses: Mutex<Vec<FlightPlanSnapshot>>,
}

impl FlightPlanProvider for FakeProvider {
    fn fetch_latest<'a>(
        &'a self,
        _kind: SimBriefReferenceKind,
        _value: &'a str,
    ) -> ProviderFuture<'a> {
        Box::pin(async move {
            self.responses
                .lock()
                .unwrap()
                .pop()
                .ok_or(ClientError::NoPlan)
        })
    }
}

struct FakeWeatherProvider {
    calls: AtomicUsize,
    snapshot: WeatherSnapshot,
}

impl WeatherProvider for FakeWeatherProvider {
    fn fetch_airports<'a>(&'a self, _stations: &'a [String]) -> WeatherProviderFuture<'a> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::AcqRel);
            Ok(self.snapshot.clone())
        })
    }
}

struct FailingWeatherProvider {
    calls: AtomicUsize,
}

impl WeatherProvider for FailingWeatherProvider {
    fn fetch_airports<'a>(&'a self, _stations: &'a [String]) -> WeatherProviderFuture<'a> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::AcqRel);
            Err(WeatherClientError::Offline)
        })
    }
}

fn snapshot(destination: &str) -> FlightPlanSnapshot {
    let retrieved_at = DateTime::from_timestamp(1_783_214_400, 0).unwrap();
    let provenance = OperationalProvenance {
        kind: ProvenanceKind::ExternalCalculation,
        provider: "simbrief".into(),
        provider_revision: Some("2607".into()),
        generated_at: Some(retrieved_at),
        retrieved_at,
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
                origin: FlightPlanAirport {
                    icao: "YSSY".into(),
                    name: None,
                    location: None,
                    planned_runway: None,
                },
                destination: FlightPlanAirport {
                    icao: destination.into(),
                    name: None,
                    location: None,
                    planned_runway: None,
                },
                alternates: vec![FlightPlanAirport {
                    icao: "NZWN".into(),
                    name: None,
                    location: None,
                    planned_runway: None,
                }],
            },
            provenance: provenance.clone(),
        },
        aircraft: Some(OperationalObservation {
            value: PlannedAircraft {
                icao_type: Some("B738".into()),
                registration: Some("VH-WYR".into()),
                model: Some("Boeing 737-800".into()),
            },
            provenance,
        }),
        schedule: None,
        weights: None,
        fuel: None,
        route: None,
    }
}

fn fleet() -> FleetSnapshotView {
    FleetSnapshotView {
        company: crate::ConnectedCompany {
            name: "Synthetic".into(),
            airline_code: "WYR".into(),
        },
        snapshot: Observed {
            value: vec![AircraftSummary {
                id: AircraftId(Uuid::new_v4()),
                registration: Some("VH-WYR".into()),
                model: Some("Boeing 737-800".into()),
                location: None,
                current_airport: Some(AirportSummary {
                    id: AirportId(Uuid::new_v4()),
                    icao: Some("YSSY".into()),
                    name: None,
                    location: None,
                }),
            }],
            provenance: Provenance {
                kind: ProvenanceKind::OnAirFact,
                source: "onair:company/fleet".into(),
                observed_at: Utc::now(),
            },
        },
        availability: SnapshotAvailability::Live,
        storage: crate::SnapshotStorage::Hoard,
    }
}

#[tokio::test]
async fn replaces_plan_without_retaining_reference_and_explains_fleet_match() {
    let provider = FakeProvider {
        responses: Mutex::new(vec![snapshot("NZAA"), snapshot("YMML")]),
    };
    let session = DispatchSession::with_providers(Some(Arc::new(provider)), None);

    session
        .import_latest(SimBriefReferenceKind::Username, "private-user")
        .await
        .unwrap();
    let status = session.briefing(Some(&fleet())).unwrap();
    let atlas_plan = status.atlas_plan.as_ref().unwrap();
    assert_eq!(atlas_plan.origin_icao, "YSSY");
    assert!(
        atlas_plan
            .points
            .iter()
            .any(|point| point.kind == crate::FlightPlanMapPointKind::Destination)
    );
    assert_eq!(
        status
            .comparison
            .as_ref()
            .unwrap()
            .matched_aircraft
            .as_ref()
            .unwrap()
            .basis,
        AircraftMatchBasis::Registration
    );
    assert!(
        status
            .comparison
            .as_ref()
            .unwrap()
            .findings
            .iter()
            .any(|finding| {
                finding.category == DispatchFindingCategory::AircraftPosition
                    && finding.status == DispatchFindingStatus::Match
            })
    );
    let serialized = serde_json::to_string(&status).unwrap();
    assert!(!serialized.contains("private-user"));

    session
        .import_latest(SimBriefReferenceKind::PilotId, "1234567")
        .await
        .unwrap();
    assert_eq!(
        session.clear().unwrap().availability,
        DispatchAvailability::Empty
    );
}

#[tokio::test]
async fn compares_a_selected_read_only_job_with_the_imported_plan() {
    let provider = Arc::new(FakeProvider {
        responses: Mutex::new(vec![snapshot("NZAA")]),
    });
    let session = DispatchSession::with_providers(Some(provider), None);
    let jobs: JobSnapshot = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/job-snapshot-v1.json"
    ))
    .unwrap();
    session
        .select_job(DispatchJobSelection {
            job: jobs.jobs[0].clone(),
            observed_at: Utc::now(),
            availability: SnapshotAvailability::Cached,
        })
        .unwrap();
    session
        .import_latest(SimBriefReferenceKind::PilotId, "123456")
        .await
        .unwrap();

    let status = session.briefing(Some(&fleet())).unwrap();
    assert!(status.selected_job.is_some());
    assert!(status.comparison.unwrap().findings.iter().any(|finding| {
        finding.category == DispatchFindingCategory::JobRoute
            && finding.status == DispatchFindingStatus::Match
    }));
}

#[test]
fn compares_job_payload_units_and_deadlines_without_inference() {
    let jobs: JobSnapshot = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/job-snapshot-v1.json"
    ))
    .unwrap();
    let job = &jobs.jobs[0];
    let mut plan = snapshot("NZAA");
    let provenance = plan.airports.provenance.clone();
    plan.weights = Some(OperationalObservation {
        value: PlannedWeights {
            payload: Some(Mass {
                value: 1_814.369_48,
                unit: MassUnit::Kilograms,
            }),
            zero_fuel: None,
            takeoff: None,
            landing: None,
        },
        provenance: provenance.clone(),
    });
    plan.schedule = Some(OperationalObservation {
        value: PlannedSchedule {
            scheduled_out: None,
            scheduled_off: None,
            scheduled_on: None,
            scheduled_in: job
                .expires_at
                .map(|expiry| expiry - chrono::Duration::minutes(1)),
            estimated_enroute_seconds: None,
        },
        provenance,
    });

    let comparison = compare_plan_to_fleet(
        &plan,
        None,
        Some(&DispatchJobSelection {
            job: job.clone(),
            observed_at: Utc::now(),
            availability: SnapshotAvailability::Cached,
        }),
    );
    assert!(comparison.findings.iter().any(|finding| {
        finding.category == DispatchFindingCategory::Payload
            && finding.status == DispatchFindingStatus::Match
    }));
    assert!(comparison.findings.iter().any(|finding| {
        finding.category == DispatchFindingCategory::Schedule
            && finding.status == DispatchFindingStatus::Match
    }));

    plan.weights.as_mut().unwrap().value.payload = Some(Mass {
        value: 3_500.0,
        unit: MassUnit::Pounds,
    });
    plan.schedule.as_mut().unwrap().value.scheduled_in = job
        .expires_at
        .map(|expiry| expiry + chrono::Duration::seconds(1));
    let comparison = compare_plan_to_fleet(
        &plan,
        None,
        Some(&DispatchJobSelection {
            job: job.clone(),
            observed_at: Utc::now(),
            availability: SnapshotAvailability::Cached,
        }),
    );
    assert!(comparison.findings.iter().any(|finding| {
        finding.category == DispatchFindingCategory::Payload
            && finding.status == DispatchFindingStatus::Difference
    }));
    assert!(comparison.findings.iter().any(|finding| {
        finding.category == DispatchFindingCategory::Schedule
            && finding.status == DispatchFindingStatus::Difference
    }));
}

#[tokio::test]
async fn caches_weather_and_clears_it_with_the_session_plan() {
    let weather = WeatherSnapshot {
        schema_version: WEATHER_SNAPSHOT_SCHEMA_VERSION,
        id: WeatherSnapshotId(Uuid::new_v4()),
        airports: vec![],
    };
    let weather_provider = Arc::new(FakeWeatherProvider {
        calls: AtomicUsize::new(0),
        snapshot: weather,
    });
    let session = DispatchSession::with_providers(
        Some(Arc::new(FakeProvider {
            responses: Mutex::new(vec![snapshot("NZAA")]),
        })),
        Some(weather_provider.clone()),
    );
    session
        .import_latest(SimBriefReferenceKind::PilotId, "1234567")
        .await
        .unwrap();

    session.refresh_weather().await.unwrap();
    session.refresh_weather().await.unwrap();
    assert_eq!(weather_provider.calls.load(Ordering::Acquire), 1);
    assert_eq!(
        session.status().unwrap().weather.cache,
        DispatchWeatherCacheState::Fresh
    );

    session.clear().unwrap();
    assert_eq!(
        session.status().unwrap().weather.cache,
        DispatchWeatherCacheState::None
    );
}

#[tokio::test]
async fn rate_protects_retries_after_a_failed_weather_request() {
    let weather_provider = Arc::new(FailingWeatherProvider {
        calls: AtomicUsize::new(0),
    });
    let session = DispatchSession::with_providers(
        Some(Arc::new(FakeProvider {
            responses: Mutex::new(vec![snapshot("NZAA")]),
        })),
        Some(weather_provider.clone()),
    );
    session
        .import_latest(SimBriefReferenceKind::PilotId, "1234567")
        .await
        .unwrap();

    assert!(matches!(
        session.refresh_weather().await,
        Err(DispatchError::WeatherProvider(WeatherClientError::Offline))
    ));
    assert_eq!(
        session.refresh_weather().await,
        Err(DispatchError::WeatherRefreshTooSoon)
    );
    assert_eq!(weather_provider.calls.load(Ordering::Acquire), 1);
}
