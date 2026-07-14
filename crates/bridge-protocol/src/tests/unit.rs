use super::*;

fn manifest() -> ProviderManifest {
    serde_json::from_str(include_str!(
        "../../../../providers/msfs2024-simconnect/provider.json"
    ))
    .expect("bundled provider manifest should deserialize")
}

#[test]
fn validates_the_bundled_provider_manifest() {
    assert_eq!(manifest().validate(), Ok(()));
}

#[test]
fn rejects_parent_directory_provider_entry_points() {
    let mut candidate = manifest();
    candidate.entry_point = "../outside.exe".into();
    assert_eq!(
        candidate.validate(),
        Err(ProviderManifestError::UnsafeEntryPoint)
    );
}

#[test]
fn round_trips_a_bounded_bridge_frame() {
    let message = BridgeEnvelope::new(
        1,
        BridgeHostMessage::Hello {
            host_version: "0.1.0".into(),
            provider_id: manifest().id,
            requested_capabilities: vec![BridgeCapability::TelemetryRead],
        },
    );
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &message).expect("frame should encode");
    let decoded: BridgeEnvelope<BridgeHostMessage> =
        read_frame(&mut bytes.as_slice()).expect("frame should decode");

    assert_eq!(decoded, message);
    assert_eq!(decoded.validate_header(), Ok(()));
}

#[test]
fn rejects_an_oversized_frame_before_allocating_payload() {
    let bytes = ((MAX_BRIDGE_FRAME_BYTES as u32) + 1).to_be_bytes();
    assert!(matches!(
        read_frame::<_, serde_json::Value>(&mut bytes.as_slice()),
        Err(BridgeFrameError::TooLarge { .. })
    ));
}

#[test]
fn validates_the_version_one_bridge_fixtures() {
    let hello: BridgeEnvelope<BridgeHostMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/bridge-host-hello-v1.json"
    ))
    .expect("host hello fixture should deserialize");
    hello.validate_header().expect("header should validate");

    let provider: BridgeEnvelope<BridgeProviderMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/bridge-provider-hello-v1.json"
    ))
    .expect("provider hello fixture should deserialize");
    provider.validate_header().expect("header should validate");
    match provider.payload {
        BridgeProviderMessage::Hello { provider } => assert!(provider.validate()),
        _ => panic!("fixture should contain provider hello"),
    }

    let telemetry: BridgeEnvelope<BridgeProviderMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/simulator-telemetry-v1.json"
    ))
    .expect("telemetry fixture should deserialize");
    match telemetry.payload {
        BridgeProviderMessage::Telemetry { snapshot } => snapshot
            .validate()
            .expect("telemetry snapshot should validate"),
        _ => panic!("fixture should contain telemetry"),
    }
}
