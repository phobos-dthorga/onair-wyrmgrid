use super::*;
use chrono::{Duration as ChronoDuration, Timelike, Utc};
use tempfile::tempdir;
use wyrmgrid_domain::{AircraftId, AirportId, AirportSummary, FboId, Provenance, ProvenanceKind};

#[derive(Default)]
struct MemoryLegalPreferences {
    value: Mutex<Option<PersistedLegalPreferences>>,
}

impl LegalPreferencesRepository for MemoryLegalPreferences {
    fn load_legal_preferences(
        &self,
    ) -> Result<Option<PersistedLegalPreferences>, LegalSettingsError> {
        self.value
            .lock()
            .map(|value| value.clone())
            .map_err(|_| LegalSettingsError::StorageUnavailable)
    }

    fn save_legal_preferences(
        &self,
        terms_version: &str,
        privacy_notice_version: &str,
        telemetry_enabled: bool,
    ) -> Result<(), LegalSettingsError> {
        *self
            .value
            .lock()
            .map_err(|_| LegalSettingsError::StorageUnavailable)? =
            Some(PersistedLegalPreferences {
                terms_version: terms_version.to_owned(),
                privacy_notice_version: privacy_notice_version.to_owned(),
                telemetry_enabled,
                acknowledged_at: "2026-07-14 00:00:00".to_owned(),
            });
        Ok(())
    }
}

#[test]
fn exposes_the_supported_plugin_api() {
    assert_eq!(platform_status().plugin_api_version, 1);
}

#[test]
fn built_in_themes_satisfy_the_same_safety_rules() {
    let themes = built_in_themes();
    assert!(
        themes
            .iter()
            .any(|theme| { theme.id == "wyrmgrid-phobos" && theme.name == "Phobos D'thorga" })
    );
    for theme in themes {
        validate_theme(&theme, true).expect("built-in theme should remain valid");
    }
}

#[test]
fn imports_and_selects_a_data_only_custom_theme() {
    let service =
        ThemeSettingsService::new(Store::open_in_memory().expect("theme store should initialize"));
    let manifest = include_str!("../../../../schemas/fixtures/theme-manifest-v1.json");

    let status = service.import(manifest).expect("valid theme should import");
    assert_eq!(status.selected_theme_id, "midnight-cargo");
    assert_eq!(status.active_theme.name, "Midnight Cargo");
    assert_eq!(status.themes.len(), 5);

    let selected = service
        .select(DEFAULT_THEME_ID)
        .expect("built-in theme should be selectable");
    assert_eq!(selected.selected_theme_id, DEFAULT_THEME_ID);
}

#[test]
fn rejects_theme_code_reserved_identifiers_and_low_contrast() {
    let arbitrary_css = r##"{
        "schema_version":1,"id":"custom","name":"Unsafe","css":"body{}",
        "colors":{},"chart_palette":["#FFFFFF","#000000","#777777"]
    }"##;
    assert_eq!(
        parse_custom_theme(arbitrary_css),
        Err(ThemeSettingsError::InvalidManifest)
    );

    let mut reserved = built_in_themes().remove(0);
    assert_eq!(
        validate_theme(&reserved, false),
        Err(ThemeSettingsError::InvalidIdentifier)
    );
    reserved.id = "low-contrast".into();
    reserved.colors.text = reserved.colors.canvas.clone();
    assert_eq!(
        validate_theme(&reserved, false),
        Err(ThemeSettingsError::InsufficientContrast)
    );
}

#[test]
fn corrupt_or_missing_selected_themes_fall_back_safely() {
    let store = Store::open_in_memory().expect("theme store should initialize");
    store
        .save_custom_theme_record("broken-theme", "{not-json}")
        .expect("corrupt fixture should save at the raw storage boundary");
    store
        .save_selected_theme_record("broken-theme")
        .expect("selected fixture should save");
    let service = ThemeSettingsService::new(store);

    let status = service
        .status()
        .expect("theme status should degrade safely");
    assert_eq!(status.selected_theme_id, DEFAULT_THEME_ID);
    assert_eq!(status.active_theme.id, DEFAULT_THEME_ID);
    assert_eq!(status.themes.len(), 4);
}

#[test]
fn legal_documents_require_versioned_acknowledgement() {
    let service = LegalSettingsService::new(MemoryLegalPreferences::default());
    assert_eq!(
        service.status().expect("status should be available"),
        LegalStatus {
            terms_version: TERMS_VERSION,
            privacy_notice_version: PRIVACY_NOTICE_VERSION,
            acknowledged: false,
            telemetry_enabled: false,
            acknowledged_at: None,
        }
    );

    let accepted = service
        .acknowledge(true)
        .expect("preferences should be saved");
    assert!(accepted.acknowledged);
    assert!(accepted.telemetry_enabled);
    assert_eq!(
        accepted.acknowledged_at.as_deref(),
        Some("2026-07-14 00:00:00")
    );

    let updated = service
        .update_telemetry(false)
        .expect("telemetry preference should be saved");
    assert!(!updated.telemetry_enabled);
}

#[test]
fn old_legal_versions_disable_telemetry_until_reviewed() {
    let repository = MemoryLegalPreferences::default();
    repository
        .save_legal_preferences("2026-01-01", "2026-01-01", true)
        .expect("fixture should be saved");
    let service = LegalSettingsService::new(repository);

    let status = service.status().expect("status should be available");
    assert!(!status.acknowledged);
    assert!(!status.telemetry_enabled);
    assert!(matches!(
        service.update_telemetry(true),
        Err(LegalSettingsError::AcknowledgementRequired)
    ));
}

#[test]
fn starts_disconnected_without_persistent_credentials() {
    let session = OnAirSession::default();
    assert_eq!(
        session.status().expect("status should be available"),
        ConnectionStatus {
            connected: false,
            company: None,
            credential_storage: "session_only",
        }
    );
}

#[test]
fn restores_the_latest_persistent_company_data_as_offline() {
    let directory = tempdir().expect("temporary Hoard directory should exist");
    let database_path = directory.path().join("wyrmgrid.db");
    let company = CompanySummary {
        id: CompanyId(Uuid::new_v4()),
        name: "Cached Charter".into(),
        airline_code: "CCH".into(),
    };
    let stored = StoredFleetSnapshot {
        schema_version: FLEET_SNAPSHOT_SCHEMA_VERSION,
        company: company.clone(),
        snapshot: Observed {
            value: vec![AircraftSummary {
                id: AircraftId(Uuid::new_v4()),
                registration: Some("CACHE-1".into()),
                model: Some("Stored Aircraft".into()),
                location: None,
                current_airport: None,
            }],
            provenance: Provenance {
                kind: ProvenanceKind::OnAirFact,
                source: "onair:company/fleet".into(),
                observed_at: Utc::now(),
            },
        },
    };
    let stored_fbos = StoredFboSnapshot {
        schema_version: FBOS_SNAPSHOT_SCHEMA_VERSION,
        company: company.clone(),
        snapshot: Observed {
            value: vec![FboSummary {
                id: FboId(Uuid::new_v4()),
                name: Some("Cached Aerie".into()),
                airport: Some(AirportSummary {
                    id: AirportId(Uuid::new_v4()),
                    icao: Some("YTEST".into()),
                    name: Some("Stored Airport".into()),
                    location: None,
                }),
            }],
            provenance: Provenance {
                kind: ProvenanceKind::OnAirFact,
                source: "onair:company/fbos".into(),
                observed_at: Utc::now(),
            },
        },
    };
    let stored_jobs = StoredJobSnapshot {
        schema_version: JOBS_SNAPSHOT_SCHEMA_VERSION,
        company: company.clone(),
        snapshot: Observed {
            value: serde_json::from_str(include_str!(
                "../../../../schemas/fixtures/job-snapshot-v1.json"
            ))
            .expect("job fixture should deserialize"),
            provenance: Provenance {
                kind: ProvenanceKind::OnAirFact,
                source: "onair:company/jobs/pending".into(),
                observed_at: Utc::now(),
            },
        },
    };
    let mut store = Store::open(&database_path).expect("persistent Hoard should open");
    save_stored_fleet(&mut store, &stored).expect("fleet should persist");
    save_stored_fbos(&mut store, &stored_fbos).expect("FBOs should persist");
    save_stored_jobs(&mut store, &stored_jobs).expect("jobs should persist");
    drop(store);

    let session = OnAirSession::with_store(
        DEFAULT_BASE_URL,
        Store::open(&database_path).expect("persistent Hoard should reopen"),
    );
    let fleet_view = session
        .fleet_snapshot()
        .expect("fleet state should be readable")
        .expect("cached fleet should restore");
    let fbo_view = session
        .fbo_snapshot()
        .expect("FBO state should be readable")
        .expect("cached FBOs should restore");
    let job_view = session
        .job_snapshot()
        .expect("job state should be readable")
        .expect("cached jobs should restore");

    assert_eq!(fleet_view.company, ConnectedCompany::from(&company));
    assert_eq!(fleet_view.availability, SnapshotAvailability::Offline);
    assert_eq!(fleet_view.storage, SnapshotStorage::Hoard);
    assert_eq!(fleet_view.snapshot, stored.snapshot);
    assert_eq!(fbo_view.company, ConnectedCompany::from(&company));
    assert_eq!(fbo_view.availability, SnapshotAvailability::Offline);
    assert_eq!(fbo_view.storage, SnapshotStorage::Hoard);
    assert_eq!(fbo_view.snapshot, stored_fbos.snapshot);
    assert_eq!(job_view.company, ConnectedCompany::from(&company));
    assert_eq!(job_view.availability, SnapshotAvailability::Offline);
    assert_eq!(job_view.storage, SnapshotStorage::Hoard);
    assert_eq!(job_view.snapshot, stored_jobs.snapshot);
}

#[test]
fn builds_a_timeline_and_resolves_company_data_as_of_a_retained_time() {
    let company = CompanySummary {
        id: CompanyId(Uuid::new_v4()),
        name: "Timeline Charter".into(),
        airline_code: "TLC".into(),
    };
    let latest_hour = Utc::now()
        .with_minute(0)
        .and_then(|value| value.with_second(0))
        .and_then(|value| value.with_nanosecond(0))
        .expect("current hour should be representable");
    let mut store = Store::open_in_memory().expect("timeline store should initialize");
    for (offset, models) in [
        (-2, vec!["Cessna 172"]),
        (-1, vec!["Cessna 172", "Beechcraft King Air"]),
        (0, vec!["Cessna 172", "Beechcraft King Air", "Cessna 172"]),
    ] {
        let observed_at = latest_hour + ChronoDuration::hours(offset);
        let aircraft = models
            .into_iter()
            .map(|model| AircraftSummary {
                id: AircraftId(Uuid::new_v4()),
                registration: None,
                model: Some(model.into()),
                location: None,
                current_airport: None,
            })
            .collect();
        save_stored_fleet(
            &mut store,
            &StoredFleetSnapshot {
                schema_version: FLEET_SNAPSHOT_SCHEMA_VERSION,
                company: company.clone(),
                snapshot: Observed {
                    value: aircraft,
                    provenance: Provenance {
                        kind: ProvenanceKind::OnAirFact,
                        source: "onair:company/fleet".into(),
                        observed_at,
                    },
                },
            },
        )
        .expect("fleet history should save");
    }
    let fbo_observed_at = latest_hour - ChronoDuration::minutes(90);
    save_stored_fbos(
        &mut store,
        &StoredFboSnapshot {
            schema_version: FBOS_SNAPSHOT_SCHEMA_VERSION,
            company: company.clone(),
            snapshot: Observed {
                value: (0..2)
                    .map(|index| FboSummary {
                        id: FboId(Uuid::new_v4()),
                        name: Some(format!("Timeline FBO {index}")),
                        airport: None,
                    })
                    .collect(),
                provenance: Provenance {
                    kind: ProvenanceKind::OnAirFact,
                    source: "onair:company/fbos".into(),
                    observed_at: fbo_observed_at,
                },
            },
        },
    )
    .expect("FBO history should save");

    let mut corruptible_store = store.clone();
    let session = OnAirSession::with_store(DEFAULT_BASE_URL, store);
    corruptible_store
        .save_api_snapshot(
            FLEET_RESOURCE_KIND,
            &company.id.0.to_string(),
            &format_timeline_time(latest_hour + ChronoDuration::hours(1)),
            "{\"unsupported\":true}",
        )
        .expect("incompatible historical fixture should save");
    let timeline = session
        .hoard_timeline_index()
        .expect("timeline should be readable");
    assert_eq!(timeline.company, Some(ConnectedCompany::from(&company)));
    assert_eq!(timeline.observation_times.len(), 4);
    assert_eq!(
        timeline
            .fleet_history
            .iter()
            .map(|point| point.aircraft_count)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(
        timeline
            .fbo_history
            .iter()
            .map(|point| point.fbo_count)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert_eq!(
        timeline.current_fleet_composition,
        vec![
            FleetCompositionPoint {
                model: "Cessna 172".into(),
                aircraft_count: 2,
            },
            FleetCompositionPoint {
                model: "Beechcraft King Air".into(),
                aircraft_count: 1,
            },
        ]
    );

    let historical = session
        .historical_company_data(&format_timeline_time(
            latest_hour - ChronoDuration::minutes(30),
        ))
        .expect("historical company data should be available");
    assert_eq!(
        historical
            .fleet
            .as_ref()
            .expect("historical fleet should exist")
            .snapshot
            .value
            .len(),
        2
    );
    assert_eq!(
        historical
            .fbos
            .as_ref()
            .expect("historical FBOs should exist")
            .snapshot
            .provenance
            .observed_at,
        fbo_observed_at
    );
    let compatible_fallback = session
        .historical_company_data(&format_timeline_time(
            latest_hour + ChronoDuration::hours(2),
        ))
        .expect("an incompatible record should not hide older compatible history");
    assert_eq!(
        compatible_fallback
            .fleet
            .expect("compatible fleet fallback should exist")
            .snapshot
            .value
            .len(),
        3
    );
    assert!(matches!(
        session.historical_company_data("not-a-time"),
        Err(HoardTimelineError::InvalidSelection)
    ));
}

#[tokio::test]
async fn rejects_invalid_credentials_before_network_access() {
    let session = OnAirSession::default();
    assert!(matches!(
        session.connect("not-a-uuid".into(), "secret".into()).await,
        Err(ConnectionError::InvalidCompanyId)
    ));
    assert!(matches!(
        session.connect(Uuid::nil().to_string(), "  ".into()).await,
        Err(ConnectionError::EmptyApiKey)
    ));
}

#[tokio::test]
async fn refuses_company_sync_without_a_connected_session() {
    let session = OnAirSession::default();
    assert!(matches!(
        session
            .synchronize_company_data(DataSyncTrigger::Manual)
            .await,
        Err(ConnectionError::NotConnected)
    ));
    assert_eq!(
        session
            .fleet_snapshot()
            .expect("snapshot state should be readable"),
        None
    );
    assert_eq!(
        session
            .fbo_snapshot()
            .expect("snapshot state should be readable"),
        None
    );
}

#[test]
fn data_sync_gate_enforces_trigger_specific_quiet_periods() {
    let started = Instant::now();
    let mut gate = DataSyncGate::default();

    assert!(gate.try_start(DataSyncTrigger::Initial, started));
    assert!(!gate.try_start(DataSyncTrigger::Manual, started));
    gate.finish();
    assert!(!gate.try_start(
        DataSyncTrigger::Manual,
        started + MANUAL_SYNC_COOLDOWN - Duration::from_secs(1)
    ));
    assert!(gate.try_start(DataSyncTrigger::Manual, started + MANUAL_SYNC_COOLDOWN));
    gate.finish();
    assert!(!gate.try_start(
        DataSyncTrigger::Automatic,
        started + MANUAL_SYNC_COOLDOWN + Duration::from_secs(1)
    ));
    assert!(gate.try_start(
        DataSyncTrigger::Automatic,
        started + MANUAL_SYNC_COOLDOWN + MINIMUM_AUTOMATIC_SYNC_INTERVAL
    ));
}

#[test]
fn maps_adapter_failures_to_bounded_user_messages() {
    assert!(matches!(
        classify_client_error(ClientError::AuthenticationRejected),
        ConnectionError::AuthenticationRejected
    ));
    assert!(matches!(
        classify_client_error(ClientError::RateLimited),
        ConnectionError::RateLimited
    ));
    assert!(matches!(
        classify_client_error(ClientError::CompanyNotFound),
        ConnectionError::CompanyNotFound
    ));
    let message = ConnectionError::AuthenticationRejected.to_string();
    assert!(message.contains("For now"));
    assert!(message.contains("not OnAir Companion"));
    assert!(matches!(
        classify_client_error(ClientError::MissingContent),
        ConnectionError::ServiceUnavailable
    ));
    assert!(matches!(
        classify_resource_error(ClientError::ApiRejected, CompanyDataResource::Fleet),
        ConnectionError::FleetUnavailable
    ));
    assert!(matches!(
        classify_resource_error(ClientError::ApiRejected, CompanyDataResource::Fbos),
        ConnectionError::FbosUnavailable
    ));
}

#[test]
fn exposes_stable_safe_operation_errors() {
    assert_eq!(
        OperationError::from(ConnectionError::RateLimited),
        OperationError {
            code: "onair.rate_limited",
            message: ConnectionError::RateLimited.to_string(),
            retryable: true,
            reportable: false,
            report_id: None,
        }
    );
    assert!(OperationError::from(ConnectionError::StateUnavailable).reportable);
    assert!(!OperationError::from(ConnectionError::AuthenticationRejected).reportable);
    assert_eq!(
        OperationError::from(LanguageSettingsError::ProtectedMessage).code,
        "language.protected_message"
    );
}
