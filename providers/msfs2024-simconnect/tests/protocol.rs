use std::process::{Command, Stdio};
use wyrmgrid_bridge_protocol::{
    BridgeCapability, BridgeEnvelope, BridgeHostMessage, BridgeProviderMessage,
    ProviderConnectionState, read_frame, write_frame,
};

const PROVIDER_ID: &str = "io.github.phobosdthorga.wyrmgrid-simconnect-msfs2024";

#[test]
fn sidecar_completes_the_handshake_and_stops_cleanly() {
    let executable = env!("CARGO_BIN_EXE_wyrmgrid-simconnect-provider");
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("provider should start");
    let mut stdin = child.stdin.take().expect("provider stdin should be piped");
    let mut stdout = child
        .stdout
        .take()
        .expect("provider stdout should be piped");

    write_frame(
        &mut stdin,
        &BridgeEnvelope::new(
            1,
            BridgeHostMessage::Hello {
                host_version: env!("CARGO_PKG_VERSION").into(),
                provider_id: PROVIDER_ID.into(),
                requested_capabilities: vec![BridgeCapability::TelemetryRead],
            },
        ),
    )
    .expect("host hello should be written");
    let hello: BridgeEnvelope<BridgeProviderMessage> =
        read_frame(&mut stdout).expect("provider hello should be returned");
    assert!(matches!(
        hello.payload,
        BridgeProviderMessage::Hello { provider }
            if provider.id == PROVIDER_ID
                && provider.capabilities == vec![BridgeCapability::TelemetryRead]
    ));

    write_frame(
        &mut stdin,
        &BridgeEnvelope::new(
            2,
            BridgeHostMessage::StartTelemetry {
                maximum_frequency_hz: 1,
            },
        ),
    )
    .expect("telemetry start should be written");
    let starting: BridgeEnvelope<BridgeProviderMessage> =
        read_frame(&mut stdout).expect("starting state should be returned");
    assert!(matches!(
        starting.payload,
        BridgeProviderMessage::State {
            state: ProviderConnectionState::Starting,
            ..
        }
    ));

    let availability: BridgeEnvelope<BridgeProviderMessage> =
        read_frame(&mut stdout).expect("provider availability should be returned");
    let BridgeProviderMessage::State { state, code } = availability.payload else {
        panic!("provider should return an availability state");
    };
    assert!(matches!(
        state,
        ProviderConnectionState::WaitingForSimulator
            | ProviderConnectionState::Connected
            | ProviderConnectionState::Unavailable
    ));
    println!("provider availability: {state:?} ({code})");

    write_frame(
        &mut stdin,
        &BridgeEnvelope::new(3, BridgeHostMessage::Shutdown),
    )
    .expect("shutdown should be written");
    drop(stdin);

    let status = child.wait().expect("provider should stop");
    assert!(status.success());
}
