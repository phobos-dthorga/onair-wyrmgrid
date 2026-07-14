use super::*;

#[derive(Default)]
struct MemorySimulatorPreferences {
    value: Mutex<Option<SimulatorPreferences>>,
}

impl SimulatorPreferencesRepository for MemorySimulatorPreferences {
    fn load_simulator_preferences(
        &self,
    ) -> Result<Option<SimulatorPreferences>, SimulatorBridgeError> {
        Ok(self.value.lock().unwrap().clone())
    }

    fn save_simulator_preferences(
        &self,
        preferences: &SimulatorPreferences,
    ) -> Result<(), SimulatorBridgeError> {
        *self.value.lock().unwrap() = Some(preferences.clone());
        Ok(())
    }
}

const MANIFEST: &str = include_str!("../../../../providers/msfs2024-simconnect/provider.json");

fn registration(path: PathBuf) -> SimulatorProviderRegistration {
    SimulatorProviderRegistration::from_manifest_json(MANIFEST, path)
        .expect("registration should validate")
}

fn telemetry_envelope() -> BridgeEnvelope<BridgeProviderMessage> {
    serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/simulator-telemetry-v1.json"
    ))
    .expect("telemetry fixture should deserialize")
}

#[test]
fn reports_a_missing_provider_executable_as_unavailable() {
    let service = SimulatorBridgeService::new(vec![registration(PathBuf::from(
        "wyrmgrid-simconnect-provider.exe",
    ))]);
    let status = service.status().expect("status should load");

    assert_eq!(status.providers.len(), 1);
    assert_eq!(
        status.providers[0].process_state,
        SimulatorProviderProcessState::Unavailable
    );
    assert!(status.latest_snapshot.is_none());
}

#[test]
fn simulator_preferences_default_to_the_first_provider_with_auto_start_off() {
    let service = SimulatorSettingsService::new(
        MemorySimulatorPreferences::default(),
        vec!["io.example.simulator".into()],
    );

    assert_eq!(
        service.status().unwrap(),
        SimulatorPreferences {
            selected_provider_id: Some("io.example.simulator".into()),
            start_with_wyrmgrid: false,
        }
    );
}

#[test]
fn simulator_preferences_reject_auto_start_without_an_installed_provider() {
    let service = SimulatorSettingsService::new(MemorySimulatorPreferences::default(), vec![]);

    assert!(matches!(
        service.update(SimulatorPreferences {
            selected_provider_id: None,
            start_with_wyrmgrid: true,
        }),
        Err(SimulatorBridgeError::InvalidPreferences)
    ));
}

#[test]
fn selecting_a_provider_preserves_the_auto_start_choice() {
    let repository = MemorySimulatorPreferences::default();
    *repository.value.lock().unwrap() = Some(SimulatorPreferences {
        selected_provider_id: Some("io.example.first".into()),
        start_with_wyrmgrid: true,
    });
    let service = SimulatorSettingsService::new(
        repository,
        vec!["io.example.first".into(), "io.example.second".into()],
    );

    assert_eq!(
        service.select_provider("io.example.second").unwrap(),
        SimulatorPreferences {
            selected_provider_id: Some("io.example.second".into()),
            start_with_wyrmgrid: true,
        }
    );
    assert_eq!(
        service.startup_provider_id().unwrap(),
        Some("io.example.second".into())
    );
}

#[test]
fn rejects_a_provider_manifest_with_a_mismatched_executable() {
    assert!(matches!(
        SimulatorProviderRegistration::from_manifest_json(
            MANIFEST,
            PathBuf::from("unexpected.exe")
        ),
        Err(SimulatorBridgeError::InvalidProvider)
    ));
}

#[test]
fn validator_requires_hello_before_telemetry() {
    let registration = registration(PathBuf::from("wyrmgrid-simconnect-provider.exe"));
    let mut validator =
        ProviderMessageValidator::new(registration.manifest, vec![BridgeCapability::TelemetryRead]);

    assert!(matches!(
        validator.apply(telemetry_envelope()),
        Err(SimulatorBridgeError::ProtocolViolation)
    ));
}

#[test]
fn validator_accepts_hello_then_telemetry_and_rejects_replay() {
    let registration = registration(PathBuf::from("wyrmgrid-simconnect-provider.exe"));
    let mut validator =
        ProviderMessageValidator::new(registration.manifest, vec![BridgeCapability::TelemetryRead]);
    let hello: BridgeEnvelope<BridgeProviderMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/bridge-provider-hello-v1.json"
    ))
    .expect("hello fixture should deserialize");
    assert!(matches!(
        validator.apply(hello),
        Ok(ValidatedProviderEvent::Hello(_))
    ));

    let telemetry = telemetry_envelope();
    assert!(matches!(
        validator.apply(telemetry.clone()),
        Ok(ValidatedProviderEvent::Telemetry(_))
    ));
    assert!(matches!(
        validator.apply(telemetry),
        Err(SimulatorBridgeError::ProtocolViolation)
    ));
}

#[test]
fn validator_rejects_an_undeclared_provider_capability() {
    let registration = registration(PathBuf::from("wyrmgrid-simconnect-provider.exe"));
    let mut validator =
        ProviderMessageValidator::new(registration.manifest, vec![BridgeCapability::TelemetryRead]);
    let mut hello: BridgeEnvelope<BridgeProviderMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/bridge-provider-hello-v1.json"
    ))
    .expect("hello fixture should deserialize");
    let BridgeProviderMessage::Hello { provider } = &mut hello.payload else {
        panic!("fixture should contain a provider hello");
    };
    provider.capabilities.push(BridgeCapability::CommandExecute);

    assert!(matches!(
        validator.apply(hello),
        Err(SimulatorBridgeError::ProtocolViolation)
    ));
}

#[test]
fn validator_rejects_telemetry_with_another_providers_provenance() {
    let registration = registration(PathBuf::from("wyrmgrid-simconnect-provider.exe"));
    let mut validator =
        ProviderMessageValidator::new(registration.manifest, vec![BridgeCapability::TelemetryRead]);
    let hello: BridgeEnvelope<BridgeProviderMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/bridge-provider-hello-v1.json"
    ))
    .expect("hello fixture should deserialize");
    validator.apply(hello).expect("hello should validate");
    let mut telemetry = telemetry_envelope();
    let BridgeProviderMessage::Telemetry { snapshot } = &mut telemetry.payload else {
        panic!("fixture should contain telemetry");
    };
    snapshot.provenance.provider = "io.example.another-provider".into();

    assert!(matches!(
        validator.apply(telemetry),
        Err(SimulatorBridgeError::ProtocolViolation)
    ));
}

#[test]
fn publishes_snapshots_only_while_the_provider_is_running_and_connected() {
    assert!(snapshot_is_publishable(
        SimulatorProviderProcessState::Running,
        ProviderConnectionState::Connected
    ));
    for process_state in [
        SimulatorProviderProcessState::Starting,
        SimulatorProviderProcessState::Stopping,
        SimulatorProviderProcessState::Stopped,
        SimulatorProviderProcessState::Failed,
        SimulatorProviderProcessState::Unavailable,
    ] {
        assert!(!snapshot_is_publishable(
            process_state,
            ProviderConnectionState::Connected
        ));
    }
    for connection_state in [
        ProviderConnectionState::Starting,
        ProviderConnectionState::WaitingForSimulator,
        ProviderConnectionState::Disconnected,
        ProviderConnectionState::Stopped,
        ProviderConnectionState::Failed,
        ProviderConnectionState::Unavailable,
    ] {
        assert!(!snapshot_is_publishable(
            SimulatorProviderProcessState::Running,
            connection_state
        ));
    }
}

#[test]
fn connected_telemetry_becomes_stale_after_the_bounded_freshness_window() {
    assert!(!telemetry_is_stale(
        ProviderConnectionState::Connected,
        None,
        Some(Duration::from_secs(5))
    ));
    assert!(telemetry_is_stale(
        ProviderConnectionState::Connected,
        None,
        Some(Duration::from_secs(6))
    ));
    assert!(!telemetry_is_stale(
        ProviderConnectionState::Connected,
        Some(Duration::from_secs(5)),
        Some(Duration::from_secs(30))
    ));
    assert!(telemetry_is_stale(
        ProviderConnectionState::Connected,
        Some(Duration::from_secs(6)),
        Some(Duration::from_secs(6))
    ));
    assert!(!telemetry_is_stale(
        ProviderConnectionState::WaitingForSimulator,
        None,
        Some(Duration::from_secs(30))
    ));
}
