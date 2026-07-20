use super::*;
use chrono::{Duration as ChronoDuration, Timelike, Utc};
use tempfile::tempdir;
use wyrmgrid_domain::{
    AircraftId, AirportId, AirportSummary, FboId, Provenance, ProvenanceKind, StaffMemberId,
    StaffMemberSummary,
};
use wyrmgrid_storage::DatabaseKey;

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

struct UnavailableThemeRepository;

impl ThemeRepository for UnavailableThemeRepository {
    fn load_selected_theme(&self) -> Result<Option<String>, ThemeSettingsError> {
        Err(ThemeSettingsError::StorageUnavailable)
    }

    fn save_selected_theme(&self, _theme_id: &str) -> Result<(), ThemeSettingsError> {
        Err(ThemeSettingsError::StorageUnavailable)
    }

    fn list_custom_themes(&self) -> Result<Vec<PersistedCustomTheme>, ThemeSettingsError> {
        Err(ThemeSettingsError::StorageUnavailable)
    }

    fn save_custom_theme(
        &self,
        _theme_id: &str,
        _manifest_json: &str,
    ) -> Result<(), ThemeSettingsError> {
        Err(ThemeSettingsError::StorageUnavailable)
    }

    fn delete_custom_theme(
        &self,
        _theme_id: &str,
        _fallback_theme_id: &str,
    ) -> Result<(), ThemeSettingsError> {
        Err(ThemeSettingsError::StorageUnavailable)
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
    let imported = status
        .themes
        .iter()
        .find(|theme| theme.manifest.id == "midnight-cargo")
        .expect("imported theme should be listed");
    assert_eq!(imported.provenance.source, ThemeSource::LocalImport);
    assert!(
        imported
            .provenance
            .imported_at
            .as_deref()
            .is_some_and(|value| value.ends_with('Z'))
    );

    let selected = service
        .select(DEFAULT_THEME_ID)
        .expect("built-in theme should be selectable");
    assert_eq!(selected.selected_theme_id, DEFAULT_THEME_ID);
}

#[test]
fn detects_exact_and_visual_theme_duplicates_but_allows_revisions() {
    let service =
        ThemeSettingsService::new(Store::open_in_memory().expect("theme store should initialize"));
    let fixture = include_str!("../../../../schemas/fixtures/theme-manifest-v1.json");
    service
        .import(fixture)
        .expect("first import should succeed");

    assert_eq!(
        service.import(fixture),
        Err(ThemeSettingsError::DuplicateTheme)
    );

    let mut case_only_duplicate = parse_custom_theme(fixture).expect("fixture should parse");
    case_only_duplicate.colors.accent.make_ascii_lowercase();
    let case_only_json =
        serde_json::to_string(&case_only_duplicate).expect("theme should serialize");
    assert_eq!(
        service.import(&case_only_json),
        Err(ThemeSettingsError::DuplicateTheme)
    );

    let mut duplicate = parse_custom_theme(fixture).expect("fixture should parse");
    duplicate.id = "midnight-cargo-copy".into();
    duplicate.name = "Midnight Cargo Copy".into();
    duplicate.colors.accent.make_ascii_lowercase();
    let duplicate_json = serde_json::to_string(&duplicate).expect("theme should serialize");
    assert_eq!(
        service.import(&duplicate_json),
        Err(ThemeSettingsError::DuplicateTheme)
    );

    let mut revision = parse_custom_theme(fixture).expect("fixture should parse");
    revision.name = "Midnight Cargo Revised".into();
    let revision_json = serde_json::to_string(&revision).expect("theme should serialize");
    let revised = service
        .import(&revision_json)
        .expect("same-identifier revision should succeed");
    assert_eq!(revised.active_theme.name, "Midnight Cargo Revised");
    assert_eq!(revised.themes.len(), 5);
}

#[test]
fn exports_any_available_theme_without_host_provenance() {
    let service =
        ThemeSettingsService::new(Store::open_in_memory().expect("theme store should initialize"));
    service
        .import(include_str!(
            "../../../../schemas/fixtures/theme-manifest-v1.json"
        ))
        .expect("fixture should import");

    let exported = service
        .export("midnight-cargo")
        .expect("custom theme should export");
    assert_eq!(exported.filename, "midnight-cargo.wyrmgrid-theme.json");
    assert_eq!(exported.media_type, "application/json");
    assert!(exported.content.ends_with('\n'));
    let manifest: ThemeManifest =
        serde_json::from_str(&exported.content).expect("export should remain a theme manifest");
    assert_eq!(manifest.id, "midnight-cargo");
    assert!(!exported.content.contains("provenance"));
    assert!(service.export(DEFAULT_THEME_ID).is_ok());
}

#[test]
fn saves_exported_themes_to_the_user_selected_file() {
    let service =
        ThemeSettingsService::new(Store::open_in_memory().expect("theme store should initialize"));
    service
        .import(include_str!(
            "../../../../schemas/fixtures/theme-manifest-v1.json"
        ))
        .expect("fixture should import");
    let directory = tempdir().expect("temporary export directory should exist");
    let destination = directory.path().join("exported-theme.json");

    service
        .save_export("midnight-cargo", &destination)
        .expect("selected theme should save");

    let content = std::fs::read_to_string(destination).expect("saved theme should be readable");
    let manifest: ThemeManifest =
        serde_json::from_str(&content).expect("saved export should remain a theme manifest");
    assert_eq!(manifest.id, "midnight-cargo");
    assert!(!content.contains("provenance"));
}

#[test]
fn saves_structurally_valid_low_contrast_drafts_without_importing_them() {
    let service =
        ThemeSettingsService::new(Store::open_in_memory().expect("theme store should initialize"));
    let mut draft = parse_custom_theme(include_str!(
        "../../../../schemas/fixtures/theme-manifest-v1.json"
    ))
    .expect("fixture should parse");
    draft.colors.text = draft.colors.canvas.clone();
    let draft_json = serde_json::to_string(&draft).expect("draft should serialize");
    let directory = tempdir().expect("temporary draft directory should exist");
    let destination = directory.path().join("draft-theme.json");

    service
        .save_draft(&draft_json, &destination)
        .expect("unfinished contrast work should remain downloadable as a draft");

    let content = std::fs::read_to_string(destination).expect("saved draft should be readable");
    let saved: ThemeManifest =
        serde_json::from_str(&content).expect("saved draft should remain structured JSON");
    assert_eq!(saved.colors.text, saved.colors.canvas);
    assert_eq!(
        service.import(&content),
        Err(ThemeSettingsError::InsufficientContrast)
    );
    assert_eq!(service.status().unwrap().themes.len(), 4);
}

#[test]
fn theme_file_saving_rejects_invalid_content_and_unwritable_destinations() {
    let service =
        ThemeSettingsService::new(Store::open_in_memory().expect("theme store should initialize"));
    let directory = tempdir().expect("temporary draft directory should exist");
    assert_eq!(
        service.save_draft("{}", &directory.path().join("invalid.json")),
        Err(ThemeSettingsError::InvalidManifest)
    );
    assert_eq!(
        service.save_draft(
            &" ".repeat(MAX_THEME_MANIFEST_BYTES + 1),
            &directory.path().join("oversized.json")
        ),
        Err(ThemeSettingsError::ManifestTooLarge)
    );
    assert_eq!(
        service.save_export(DEFAULT_THEME_ID, directory.path()),
        Err(ThemeSettingsError::FileSaveFailed)
    );
}

#[test]
fn deletes_only_local_themes_and_falls_back_when_the_active_theme_is_deleted() {
    let service =
        ThemeSettingsService::new(Store::open_in_memory().expect("theme store should initialize"));
    service
        .import(include_str!(
            "../../../../schemas/fixtures/theme-manifest-v1.json"
        ))
        .expect("fixture should import");

    assert_eq!(
        service.delete(DEFAULT_THEME_ID),
        Err(ThemeSettingsError::BundledThemeCannotBeDeleted)
    );
    let status = service
        .delete("midnight-cargo")
        .expect("local theme should delete");
    assert_eq!(status.selected_theme_id, DEFAULT_THEME_ID);
    assert_eq!(status.active_theme.id, DEFAULT_THEME_ID);
    assert_eq!(status.themes.len(), 4);
    assert_eq!(
        service.delete("midnight-cargo"),
        Err(ThemeSettingsError::UnknownTheme)
    );
    assert_eq!(
        service.export("midnight-cargo"),
        Err(ThemeSettingsError::UnknownTheme)
    );
}

#[test]
fn theme_management_reports_unavailable_storage_without_partial_results() {
    let service = ThemeSettingsService::new(UnavailableThemeRepository);
    let fixture = include_str!("../../../../schemas/fixtures/theme-manifest-v1.json");

    assert_eq!(
        service.import(fixture),
        Err(ThemeSettingsError::StorageUnavailable)
    );
    assert_eq!(
        service.export(DEFAULT_THEME_ID),
        Err(ThemeSettingsError::StorageUnavailable)
    );
    assert_eq!(
        service.delete(DEFAULT_THEME_ID),
        Err(ThemeSettingsError::StorageUnavailable)
    );
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
    let database_key = DatabaseKey::from_bytes([17; 32]);
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
    let stored_staff = StoredStaffSnapshot {
        schema_version: wyrmgrid_domain::STAFF_SNAPSHOT_SCHEMA_VERSION,
        company: company.clone(),
        snapshot: Observed {
            value: StaffSnapshot {
                schema_version: wyrmgrid_domain::STAFF_SNAPSHOT_SCHEMA_VERSION,
                staff: vec![StaffMemberSummary {
                    id: StaffMemberId(Uuid::new_v4()),
                    display_name: Some("Synthetic Cached Aviator".into()),
                    avatar_reference: None,
                    category_code: Some(1),
                    status_code: None,
                    current_airport: None,
                    home_airport: None,
                    busy_until: None,
                    is_online: None,
                    class_qualifications: Vec::new(),
                }],
            },
            provenance: Provenance {
                kind: ProvenanceKind::OnAirFact,
                source: "onair:company/employees".into(),
                observed_at: Utc::now(),
            },
        },
    };
    let mut store =
        Store::open(&database_path, &database_key).expect("persistent Hoard should open");
    save_stored_fleet(&mut store, &stored).expect("fleet should persist");
    save_stored_fbos(&mut store, &stored_fbos).expect("FBOs should persist");
    save_stored_jobs(&mut store, &stored_jobs).expect("jobs should persist");
    save_stored_staff(&mut store, &stored_staff).expect("staff should persist");
    drop(store);

    let session = OnAirSession::with_store(
        DEFAULT_BASE_URL,
        Store::open(&database_path, &database_key).expect("persistent Hoard should reopen"),
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
    let staff_view = session
        .staff_snapshot()
        .expect("staff state should be readable")
        .expect("cached staff should restore");

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
    assert_eq!(staff_view.company, ConnectedCompany::from(&company));
    assert_eq!(staff_view.availability, SnapshotAvailability::Offline);
    assert_eq!(staff_view.storage, SnapshotStorage::Hoard);
    assert_eq!(staff_view.snapshot, stored_staff.snapshot);
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
    assert_eq!(
        session
            .staff_snapshot()
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
    assert!(matches!(
        classify_resource_error(ClientError::ApiRejected, CompanyDataResource::Staff),
        ConnectionError::StaffUnavailable
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
    assert_eq!(
        OperationError::from(ThemeSettingsError::DuplicateTheme).code,
        "theme.duplicate"
    );
    assert_eq!(
        OperationError::from(ThemeSettingsError::BundledThemeCannotBeDeleted).code,
        "theme.bundled_delete_forbidden"
    );
    assert_eq!(
        OperationError::from(ThemeSettingsError::FileSaveFailed).code,
        "theme.file_save_failed"
    );
}
