use super::*;

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
