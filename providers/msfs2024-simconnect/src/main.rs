use std::io::{BufWriter, Stdin, Stdout};
use std::sync::mpsc::{self, Receiver};
use std::thread;
#[cfg(not(windows))]
use std::time::Duration;
use wyrmgrid_bridge_protocol::{
    BRIDGE_PROTOCOL_VERSION, BridgeCapability, BridgeEnvelope, BridgeHostMessage,
    BridgeProviderMessage, ProviderConnectionState, ProviderDescriptor, read_frame, write_frame,
};

#[cfg(windows)]
mod simconnect;

const PROVIDER_ID: &str = "io.github.phobosdthorga.wyrmgrid-simconnect-msfs2024";

fn main() {
    if run().is_err() {
        std::process::exit(1);
    }
}

fn run() -> Result<(), ()> {
    let stdin = std::io::stdin();
    let mut reader = stdin;
    let first: BridgeEnvelope<BridgeHostMessage> = read_frame(&mut reader).map_err(|_| ())?;
    first.validate_header().map_err(|_| ())?;
    let requested_capabilities = match first.payload {
        BridgeHostMessage::Hello {
            provider_id,
            requested_capabilities,
            ..
        } if provider_id == PROVIDER_ID
            && requested_capabilities.contains(&BridgeCapability::TelemetryRead) =>
        {
            requested_capabilities
        }
        _ => return Err(()),
    };

    let mut writer = ProviderWriter::new(std::io::stdout());
    writer
        .send(BridgeProviderMessage::Hello {
            provider: ProviderDescriptor {
                id: PROVIDER_ID.into(),
                name: "MSFS 2024 SimConnect".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                simulator: "msfs_2024".into(),
                simulator_version: None,
                architecture: architecture().into(),
                capabilities: requested_capabilities,
            },
        })
        .map_err(|_| ())?;

    let second: BridgeEnvelope<BridgeHostMessage> = read_frame(&mut reader).map_err(|_| ())?;
    second.validate_header().map_err(|_| ())?;
    if second.sequence <= first.sequence {
        return Err(());
    }
    let maximum_frequency_hz = match second.payload {
        BridgeHostMessage::StartTelemetry {
            maximum_frequency_hz,
        } if maximum_frequency_hz > 0 => maximum_frequency_hz,
        BridgeHostMessage::Shutdown => return Ok(()),
        _ => return Err(()),
    };

    let (command_sender, command_receiver) = mpsc::channel();
    spawn_command_reader(reader, second.sequence, command_sender);
    writer
        .send(BridgeProviderMessage::State {
            state: ProviderConnectionState::Starting,
            code: "provider.starting".into(),
        })
        .map_err(|_| ())?;

    run_platform_provider(&mut writer, &command_receiver, maximum_frequency_hz)
}

fn spawn_command_reader(mut stdin: Stdin, mut last_sequence: u64, sender: mpsc::Sender<()>) {
    thread::spawn(move || {
        loop {
            let envelope: BridgeEnvelope<BridgeHostMessage> = match read_frame(&mut stdin) {
                Ok(envelope) => envelope,
                Err(_) => {
                    let _ = sender.send(());
                    return;
                }
            };
            if envelope.validate_header().is_err() || envelope.sequence <= last_sequence {
                let _ = sender.send(());
                return;
            }
            last_sequence = envelope.sequence;
            if matches!(envelope.payload, BridgeHostMessage::Shutdown) {
                let _ = sender.send(());
                return;
            }
        }
    });
}

#[cfg(windows)]
fn run_platform_provider(
    writer: &mut ProviderWriter,
    shutdown: &Receiver<()>,
    maximum_frequency_hz: u8,
) -> Result<(), ()> {
    simconnect::run(writer, shutdown, maximum_frequency_hz)
}

#[cfg(not(windows))]
fn run_platform_provider(
    writer: &mut ProviderWriter,
    shutdown: &Receiver<()>,
    _maximum_frequency_hz: u8,
) -> Result<(), ()> {
    writer
        .send(BridgeProviderMessage::State {
            state: ProviderConnectionState::Unavailable,
            code: "provider.unsupported_platform".into(),
        })
        .map_err(|_| ())?;
    while shutdown.recv_timeout(Duration::from_secs(1)).is_err() {}
    Ok(())
}

struct ProviderWriter {
    stdout: BufWriter<Stdout>,
    sequence: u64,
}

impl ProviderWriter {
    fn new(stdout: Stdout) -> Self {
        Self {
            stdout: BufWriter::new(stdout),
            sequence: 1,
        }
    }

    fn send(
        &mut self,
        message: BridgeProviderMessage,
    ) -> Result<(), wyrmgrid_bridge_protocol::BridgeFrameError> {
        write_frame(
            &mut self.stdout,
            &BridgeEnvelope {
                protocol_version: BRIDGE_PROTOCOL_VERSION,
                sequence: self.sequence,
                payload: message,
            },
        )?;
        self.sequence += 1;
        Ok(())
    }
}

fn architecture() -> &'static str {
    #[cfg(all(windows, target_arch = "x86_64"))]
    return "windows_x86_64";
    #[cfg(all(windows, target_arch = "aarch64"))]
    return "windows_aarch64";
    #[cfg(not(windows))]
    return "unsupported";
}
