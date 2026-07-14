use super::ProviderWriter;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;
use wyrmgrid_bridge_protocol::{BridgeProviderMessage, ProviderConnectionState};
use wyrmgrid_domain::{
    Coordinates, OperationalProvenance, ProvenanceKind, SIMULATOR_TELEMETRY_SCHEMA_VERSION,
    SimulatorAircraftIdentity, SimulatorIdentity, SimulatorTelemetrySnapshot, SnapshotFreshness,
};

const RECONNECT_INTERVAL: Duration = Duration::from_secs(5);
const DISPATCH_INTERVAL: Duration = Duration::from_millis(20);
const SIMCONNECT_OBJECT_ID_USER: u32 = 0;
const SIMCONNECT_OPEN_CONFIGINDEX_LOCAL: u32 = u32::MAX;
const SIMCONNECT_UNUSED: u32 = u32::MAX;
const SIMCONNECT_DATATYPE_FLOAT64: i32 = 4;
const SIMCONNECT_DATATYPE_STRING32: i32 = 6;
const SIMCONNECT_DATATYPE_STRING256: i32 = 9;
const SIMCONNECT_PERIOD_SECOND: i32 = 4;
const RECV_ID_EXCEPTION: u32 = 1;
const RECV_ID_OPEN: u32 = 2;
const RECV_ID_QUIT: u32 = 3;
const RECV_ID_EVENT: u32 = 4;
const RECV_ID_SIMOBJECT_DATA: u32 = 8;
const EVENT_PAUSE: u32 = 1;
const EVENT_SIM: u32 = 2;
const DEFINITION_TELEMETRY: u32 = 1;
const REQUEST_TELEMETRY: u32 = 1;

pub(super) fn run(
    writer: &mut ProviderWriter,
    shutdown: &Receiver<()>,
    maximum_frequency_hz: u8,
) -> Result<(), ()> {
    let dll_path = match discover_simconnect_dll() {
        Some(path) => path,
        None => {
            writer
                .send(BridgeProviderMessage::State {
                    state: ProviderConnectionState::Unavailable,
                    code: "simconnect.client_unavailable".into(),
                })
                .map_err(|_| ())?;
            wait_for_shutdown(shutdown);
            return Ok(());
        }
    };
    let api = match SimConnectApi::load(&dll_path) {
        Ok(api) => api,
        Err(_) => {
            writer
                .send(BridgeProviderMessage::State {
                    state: ProviderConnectionState::Unavailable,
                    code: "simconnect.client_load_failed".into(),
                })
                .map_err(|_| ())?;
            wait_for_shutdown(shutdown);
            return Ok(());
        }
    };

    let mut snapshot_sequence = 1_u64;
    while shutdown.try_recv().is_err() {
        writer
            .send(BridgeProviderMessage::State {
                state: ProviderConnectionState::WaitingForSimulator,
                code: "simconnect.waiting_for_simulator".into(),
            })
            .map_err(|_| ())?;
        let mut client = match api.connect() {
            Ok(client) => client,
            Err(SimConnectError::ConnectionUnavailable) => {
                if wait_or_shutdown(shutdown, RECONNECT_INTERVAL) {
                    break;
                }
                continue;
            }
            Err(_) => {
                writer
                    .send(BridgeProviderMessage::State {
                        state: ProviderConnectionState::Failed,
                        code: "simconnect.setup_failed".into(),
                    })
                    .map_err(|_| ())?;
                if wait_or_shutdown(shutdown, RECONNECT_INTERVAL) {
                    break;
                }
                continue;
            }
        };
        writer
            .send(BridgeProviderMessage::State {
                state: ProviderConnectionState::Connected,
                code: "simconnect.connected".into(),
            })
            .map_err(|_| ())?;

        let outcome = client.pump(
            writer,
            shutdown,
            maximum_frequency_hz,
            &mut snapshot_sequence,
        );
        if shutdown.try_recv().is_ok() {
            break;
        }
        let (state, code) = match outcome {
            Ok(()) | Err(SimConnectError::Disconnected) => (
                ProviderConnectionState::Disconnected,
                "simconnect.disconnected",
            ),
            Err(_) => (ProviderConnectionState::Failed, "simconnect.protocol_error"),
        };
        writer
            .send(BridgeProviderMessage::State {
                state,
                code: code.into(),
            })
            .map_err(|_| ())?;
        if wait_or_shutdown(shutdown, RECONNECT_INTERVAL) {
            break;
        }
    }

    writer
        .send(BridgeProviderMessage::State {
            state: ProviderConnectionState::Stopped,
            code: "provider.stopped".into(),
        })
        .map_err(|_| ())
}

fn wait_for_shutdown(shutdown: &Receiver<()>) {
    while shutdown.recv_timeout(Duration::from_secs(1)).is_err() {}
}

fn wait_or_shutdown(shutdown: &Receiver<()>, duration: Duration) -> bool {
    shutdown.recv_timeout(duration).is_ok()
}

fn discover_simconnect_dll() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        candidates.push(directory.join("SimConnect.dll"));
    }
    if let Some(path) = std::env::var_os("WYRMGRID_SIMCONNECT_DLL") {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            candidates.push(path);
        }
    }
    if let Some(root) = std::env::var_os("MSFS2024_SDK") {
        let root = PathBuf::from(root);
        if root.is_absolute() {
            candidates.push(root.join("SimConnect SDK/lib/SimConnect.dll"));
        }
    }
    candidates.push(PathBuf::from(
        r"C:\MSFS 2024 SDK\SimConnect SDK\lib\SimConnect.dll",
    ));
    candidates.into_iter().find(|path| path.is_file())
}

#[derive(Debug, Error)]
enum SimConnectError {
    #[error("SimConnect client could not be loaded")]
    ClientUnavailable,
    #[error("Flight Simulator is not accepting SimConnect connections")]
    ConnectionUnavailable,
    #[error("SimConnect telemetry definition failed")]
    DefinitionFailed,
    #[error("SimConnect disconnected")]
    Disconnected,
    #[error("SimConnect returned an invalid telemetry frame")]
    InvalidTelemetry,
}

type Handle = *mut c_void;
type HResult = i32;
type SimConnectOpen =
    unsafe extern "C" fn(*mut Handle, *const c_char, *mut c_void, u32, Handle, u32) -> HResult;
type SimConnectClose = unsafe extern "C" fn(Handle) -> HResult;
type SimConnectAddToDataDefinition =
    unsafe extern "C" fn(Handle, u32, *const c_char, *const c_char, i32, f32, u32) -> HResult;
type SimConnectRequestDataOnSimObject =
    unsafe extern "C" fn(Handle, u32, u32, u32, i32, u32, u32, u32, u32) -> HResult;
type SimConnectSubscribeToSystemEvent = unsafe extern "C" fn(Handle, u32, *const c_char) -> HResult;
type SimConnectGetNextDispatch =
    unsafe extern "C" fn(Handle, *mut *const SimConnectRecv, *mut u32) -> HResult;

struct SimConnectApi {
    _library: Library,
    open: SimConnectOpen,
    close: SimConnectClose,
    add_to_data_definition: SimConnectAddToDataDefinition,
    request_data_on_sim_object: SimConnectRequestDataOnSimObject,
    subscribe_to_system_event: SimConnectSubscribeToSystemEvent,
    get_next_dispatch: SimConnectGetNextDispatch,
}

impl SimConnectApi {
    fn load(path: &Path) -> Result<Self, SimConnectError> {
        // SAFETY: the path is absolute and host-owned; symbols are copied while the library
        // remains stored in this value for at least as long as every function pointer.
        unsafe {
            let library = Library::new(path).map_err(|_| SimConnectError::ClientUnavailable)?;
            let open = *library
                .get(b"SimConnect_Open\0")
                .map_err(|_| SimConnectError::ClientUnavailable)?;
            let close = *library
                .get(b"SimConnect_Close\0")
                .map_err(|_| SimConnectError::ClientUnavailable)?;
            let add_to_data_definition = *library
                .get(b"SimConnect_AddToDataDefinition\0")
                .map_err(|_| SimConnectError::ClientUnavailable)?;
            let request_data_on_sim_object = *library
                .get(b"SimConnect_RequestDataOnSimObject\0")
                .map_err(|_| SimConnectError::ClientUnavailable)?;
            let subscribe_to_system_event = *library
                .get(b"SimConnect_SubscribeToSystemEvent\0")
                .map_err(|_| SimConnectError::ClientUnavailable)?;
            let get_next_dispatch = *library
                .get(b"SimConnect_GetNextDispatch\0")
                .map_err(|_| SimConnectError::ClientUnavailable)?;
            Ok(Self {
                _library: library,
                open,
                close,
                add_to_data_definition,
                request_data_on_sim_object,
                subscribe_to_system_event,
                get_next_dispatch,
            })
        }
    }

    fn connect(&self) -> Result<SimConnectClient<'_>, SimConnectError> {
        let mut handle = ptr::null_mut();
        let client_name = CString::new("OnAir WyrmGrid Bridge").expect("static name is valid");
        // SAFETY: all pointers are valid for the duration of the call and the returned handle is
        // closed by SimConnectClient::drop.
        let result = unsafe {
            (self.open)(
                &mut handle,
                client_name.as_ptr(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                SIMCONNECT_OPEN_CONFIGINDEX_LOCAL,
            )
        };
        if failed(result) || handle.is_null() {
            return Err(SimConnectError::ConnectionUnavailable);
        }
        let mut client = SimConnectClient {
            api: self,
            handle,
            paused: None,
            simulator_version: None,
            simconnect_version: None,
        };
        client.configure()?;
        Ok(client)
    }
}

struct SimConnectClient<'a> {
    api: &'a SimConnectApi,
    handle: Handle,
    paused: Option<bool>,
    simulator_version: Option<String>,
    simconnect_version: Option<String>,
}

impl SimConnectClient<'_> {
    fn configure(&mut self) -> Result<(), SimConnectError> {
        let definitions = [
            ("TITLE", None, SIMCONNECT_DATATYPE_STRING256),
            ("ATC ID", None, SIMCONNECT_DATATYPE_STRING32),
            (
                "PLANE LATITUDE",
                Some("degrees"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
            (
                "PLANE LONGITUDE",
                Some("degrees"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
            ("PLANE ALTITUDE", Some("feet"), SIMCONNECT_DATATYPE_FLOAT64),
            (
                "PLANE PITCH DEGREES",
                Some("degrees"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
            (
                "PLANE BANK DEGREES",
                Some("degrees"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
            (
                "PLANE HEADING DEGREES TRUE",
                Some("degrees"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
            (
                "AIRSPEED INDICATED",
                Some("knots"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
            ("AIRSPEED TRUE", Some("knots"), SIMCONNECT_DATATYPE_FLOAT64),
            (
                "GROUND VELOCITY",
                Some("knots"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
            ("SIM ON GROUND", Some("bool"), SIMCONNECT_DATATYPE_FLOAT64),
            ("ZULU YEAR", Some("number"), SIMCONNECT_DATATYPE_FLOAT64),
            (
                "ZULU MONTH OF YEAR",
                Some("number"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
            (
                "ZULU DAY OF MONTH",
                Some("number"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
            ("ZULU TIME", Some("seconds"), SIMCONNECT_DATATYPE_FLOAT64),
            (
                "FUEL TOTAL QUANTITY EX1",
                Some("gallons"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
            (
                "FUEL TOTAL QUANTITY WEIGHT EX1",
                Some("pounds"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
            ("TOTAL WEIGHT", Some("pounds"), SIMCONNECT_DATATYPE_FLOAT64),
            (
                "GENERAL ENG COMBUSTION:1",
                Some("bool"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
            (
                "GENERAL ENG COMBUSTION:2",
                Some("bool"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
            (
                "GENERAL ENG COMBUSTION:3",
                Some("bool"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
            (
                "GENERAL ENG COMBUSTION:4",
                Some("bool"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
            (
                "BRAKE PARKING POSITION",
                Some("bool"),
                SIMCONNECT_DATATYPE_FLOAT64,
            ),
        ];
        for (name, units, data_type) in definitions {
            let name = CString::new(name).expect("static datum name is valid");
            let units = units.map(|units| CString::new(units).expect("static units are valid"));
            // SAFETY: strings and handle remain valid for the complete call.
            let result = unsafe {
                (self.api.add_to_data_definition)(
                    self.handle,
                    DEFINITION_TELEMETRY,
                    name.as_ptr(),
                    units.as_ref().map_or(ptr::null(), |units| units.as_ptr()),
                    data_type,
                    0.0,
                    SIMCONNECT_UNUSED,
                )
            };
            if failed(result) {
                return Err(SimConnectError::DefinitionFailed);
            }
        }
        for (event_id, event_name) in [(EVENT_PAUSE, "Pause"), (EVENT_SIM, "Sim")] {
            let event_name = CString::new(event_name).expect("static event name is valid");
            // SAFETY: string and handle remain valid for the complete call.
            let result = unsafe {
                (self.api.subscribe_to_system_event)(self.handle, event_id, event_name.as_ptr())
            };
            if failed(result) {
                return Err(SimConnectError::DefinitionFailed);
            }
        }
        // SAFETY: the handle is open and all numeric identifiers are provider-owned.
        let result = unsafe {
            (self.api.request_data_on_sim_object)(
                self.handle,
                REQUEST_TELEMETRY,
                DEFINITION_TELEMETRY,
                SIMCONNECT_OBJECT_ID_USER,
                SIMCONNECT_PERIOD_SECOND,
                0,
                0,
                0,
                0,
            )
        };
        if failed(result) {
            return Err(SimConnectError::DefinitionFailed);
        }
        Ok(())
    }

    fn pump(
        &mut self,
        writer: &mut ProviderWriter,
        shutdown: &Receiver<()>,
        maximum_frequency_hz: u8,
        snapshot_sequence: &mut u64,
    ) -> Result<(), SimConnectError> {
        let minimum_interval = Duration::from_secs_f64(1.0 / f64::from(maximum_frequency_hz));
        let mut last_emitted = Instant::now() - minimum_interval;
        let mut consecutive_dispatch_errors = 0_u16;
        while shutdown.try_recv().is_err() {
            let mut data = ptr::null();
            let mut size = 0_u32;
            // SAFETY: SimConnect owns the returned pointer until the next dispatch call; it is
            // inspected and copied before this loop continues.
            let result = unsafe { (self.api.get_next_dispatch)(self.handle, &mut data, &mut size) };
            if failed(result) || data.is_null() {
                consecutive_dispatch_errors = consecutive_dispatch_errors.saturating_add(1);
                if consecutive_dispatch_errors >= 500 {
                    return Err(SimConnectError::Disconnected);
                }
                thread::sleep(DISPATCH_INTERVAL);
                continue;
            }
            consecutive_dispatch_errors = 0;
            // SAFETY: SimConnect returned a non-null buffer of at least the base structure size.
            let receive = unsafe { &*data };
            if receive.size < std::mem::size_of::<SimConnectRecv>() as u32 || receive.size > size {
                return Err(SimConnectError::InvalidTelemetry);
            }
            match receive.id {
                RECV_ID_OPEN => self.capture_versions(data, receive.size)?,
                RECV_ID_QUIT => return Err(SimConnectError::Disconnected),
                RECV_ID_EVENT => self.capture_event(data, receive.size)?,
                RECV_ID_SIMOBJECT_DATA if last_emitted.elapsed() >= minimum_interval => {
                    let snapshot = self.read_snapshot(data, receive.size, *snapshot_sequence)?;
                    snapshot
                        .validate()
                        .map_err(|_| SimConnectError::InvalidTelemetry)?;
                    writer
                        .send(BridgeProviderMessage::Telemetry { snapshot })
                        .map_err(|_| SimConnectError::Disconnected)?;
                    *snapshot_sequence += 1;
                    last_emitted = Instant::now();
                }
                RECV_ID_EXCEPTION => return Err(SimConnectError::DefinitionFailed),
                _ => {}
            }
        }
        Ok(())
    }

    fn capture_versions(
        &mut self,
        data: *const SimConnectRecv,
        size: u32,
    ) -> Result<(), SimConnectError> {
        if size < std::mem::size_of::<SimConnectRecvOpen>() as u32 {
            return Err(SimConnectError::InvalidTelemetry);
        }
        // SAFETY: size was checked against the complete structure before the cast.
        let open = unsafe { &*(data.cast::<SimConnectRecvOpen>()) };
        self.simulator_version = Some(format!(
            "{}.{}.{}.{}",
            open.application_version_major,
            open.application_version_minor,
            open.application_build_major,
            open.application_build_minor
        ));
        self.simconnect_version = Some(format!(
            "{}.{}.{}.{}",
            open.simconnect_version_major,
            open.simconnect_version_minor,
            open.simconnect_build_major,
            open.simconnect_build_minor
        ));
        Ok(())
    }

    fn capture_event(
        &mut self,
        data: *const SimConnectRecv,
        size: u32,
    ) -> Result<(), SimConnectError> {
        if size < std::mem::size_of::<SimConnectRecvEvent>() as u32 {
            return Err(SimConnectError::InvalidTelemetry);
        }
        // SAFETY: size was checked against the complete structure before the cast.
        let event = unsafe { &*(data.cast::<SimConnectRecvEvent>()) };
        match event.event_id {
            EVENT_PAUSE => self.paused = Some(event.data != 0),
            EVENT_SIM if event.data == 0 => self.paused = Some(true),
            EVENT_SIM => self.paused = Some(false),
            _ => {}
        }
        Ok(())
    }

    fn read_snapshot(
        &self,
        data: *const SimConnectRecv,
        size: u32,
        sequence: u64,
    ) -> Result<SimulatorTelemetrySnapshot, SimConnectError> {
        if size
            < std::mem::size_of::<SimConnectRecvSimObjectData>() as u32 - 4
                + std::mem::size_of::<RawTelemetry>() as u32
        {
            return Err(SimConnectError::InvalidTelemetry);
        }
        // SAFETY: size covers the fixed response header and full packed telemetry payload. The
        // payload begins at dw_data and is explicitly read unaligned per the SimConnect contract.
        let response = unsafe { &*(data.cast::<SimConnectRecvSimObjectData>()) };
        if response.request_id != REQUEST_TELEMETRY
            || response.definition_id != DEFINITION_TELEMETRY
        {
            return Err(SimConnectError::InvalidTelemetry);
        }
        let raw =
            unsafe { ptr::read_unaligned(ptr::addr_of!(response.data).cast::<RawTelemetry>()) };
        raw.into_snapshot(
            sequence,
            self.paused,
            self.simulator_version.clone(),
            self.simconnect_version.clone(),
        )
    }
}

impl Drop for SimConnectClient<'_> {
    fn drop(&mut self) {
        // SAFETY: this handle was created by SimConnect_Open and is closed exactly once here.
        let _ = unsafe { (self.api.close)(self.handle) };
    }
}

#[repr(C)]
struct SimConnectRecv {
    size: u32,
    version: u32,
    id: u32,
}

#[repr(C)]
struct SimConnectRecvOpen {
    base: SimConnectRecv,
    application_name: [c_char; 256],
    application_version_major: u32,
    application_version_minor: u32,
    application_build_major: u32,
    application_build_minor: u32,
    simconnect_version_major: u32,
    simconnect_version_minor: u32,
    simconnect_build_major: u32,
    simconnect_build_minor: u32,
    reserved_1: u32,
    reserved_2: u32,
}

#[repr(C)]
struct SimConnectRecvEvent {
    base: SimConnectRecv,
    group_id: u32,
    event_id: u32,
    data: u32,
}

#[repr(C)]
struct SimConnectRecvSimObjectData {
    base: SimConnectRecv,
    request_id: u32,
    object_id: u32,
    definition_id: u32,
    flags: u32,
    entry_number: u32,
    out_of: u32,
    definition_count: u32,
    data: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RawTelemetry {
    title: [c_char; 256],
    registration: [c_char; 32],
    latitude: f64,
    longitude: f64,
    altitude_feet: f64,
    pitch_degrees: f64,
    bank_degrees: f64,
    true_heading_degrees: f64,
    indicated_airspeed_knots: f64,
    true_airspeed_knots: f64,
    ground_speed_knots: f64,
    on_ground: f64,
    zulu_year: f64,
    zulu_month: f64,
    zulu_day: f64,
    zulu_seconds: f64,
    fuel_total_gallons: f64,
    fuel_total_weight_pounds: f64,
    gross_weight_pounds: f64,
    engine_1_combustion: f64,
    engine_2_combustion: f64,
    engine_3_combustion: f64,
    engine_4_combustion: f64,
    parking_brake: f64,
}

impl RawTelemetry {
    fn into_snapshot(
        self,
        sequence: u64,
        paused: Option<bool>,
        simulator_version: Option<String>,
        simconnect_version: Option<String>,
    ) -> Result<SimulatorTelemetrySnapshot, SimConnectError> {
        let title = c_string(&self.title).unwrap_or_else(|| "Unknown aircraft".into());
        let registration = c_string(&self.registration);
        let simulation_time_utc = simulation_time(
            self.zulu_year,
            self.zulu_month,
            self.zulu_day,
            self.zulu_seconds,
        );
        let now = Utc::now();
        let snapshot = SimulatorTelemetrySnapshot {
            schema_version: SIMULATOR_TELEMETRY_SCHEMA_VERSION,
            sequence,
            provenance: OperationalProvenance {
                kind: ProvenanceKind::ExternalFact,
                provider: super::PROVIDER_ID.into(),
                provider_revision: simconnect_version,
                generated_at: simulation_time_utc,
                retrieved_at: now,
                transformation_version: 1,
                freshness: SnapshotFreshness::Current,
            },
            simulator: SimulatorIdentity {
                family: "msfs_2024".into(),
                version: simulator_version,
            },
            aircraft: SimulatorAircraftIdentity {
                title,
                registration,
            },
            position: Coordinates {
                latitude: self.latitude,
                longitude: self.longitude,
            },
            altitude_feet: self.altitude_feet,
            pitch_degrees: self.pitch_degrees,
            bank_degrees: self.bank_degrees,
            true_heading_degrees: self.true_heading_degrees.rem_euclid(360.0),
            indicated_airspeed_knots: self.indicated_airspeed_knots,
            true_airspeed_knots: self.true_airspeed_knots,
            ground_speed_knots: self.ground_speed_knots,
            on_ground: self.on_ground > 0.0,
            simulation_time_utc,
            fuel_total_gallons: non_negative(self.fuel_total_gallons),
            fuel_total_weight_pounds: non_negative(self.fuel_total_weight_pounds),
            gross_weight_pounds: non_negative(self.gross_weight_pounds),
            engines_running: Some(
                [
                    self.engine_1_combustion,
                    self.engine_2_combustion,
                    self.engine_3_combustion,
                    self.engine_4_combustion,
                ]
                .into_iter()
                .any(|value| value > 0.0),
            ),
            parking_brake_set: Some(self.parking_brake > 0.0),
            paused,
            simulation_rate: None,
        };
        snapshot
            .validate()
            .map_err(|_| SimConnectError::InvalidTelemetry)?;
        Ok(snapshot)
    }
}

fn c_string<const N: usize>(value: &[c_char; N]) -> Option<String> {
    let bytes = value
        .iter()
        .map(|character| *character as u8)
        .collect::<Vec<_>>();
    let terminator = bytes.iter().position(|byte| *byte == 0).unwrap_or(N);
    let terminated = &bytes[..terminator];
    if terminated.is_empty() {
        return None;
    }
    let mut owned = terminated.to_vec();
    owned.push(0);
    let text = CStr::from_bytes_with_nul(&owned)
        .ok()?
        .to_str()
        .ok()?
        .trim();
    (!text.is_empty()).then(|| text.to_owned())
}

fn simulation_time(year: f64, month: f64, day: f64, seconds: f64) -> Option<DateTime<Utc>> {
    if !year.is_finite() || !month.is_finite() || !day.is_finite() || !seconds.is_finite() {
        return None;
    }
    let date = NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)?;
    let whole_seconds = seconds.clamp(0.0, 86_399.999).floor() as u32;
    let nanoseconds = ((seconds.fract().max(0.0)) * 1_000_000_000.0) as u32;
    let time = NaiveTime::from_num_seconds_from_midnight_opt(whole_seconds, nanoseconds)?;
    Some(DateTime::from_naive_utc_and_offset(
        date.and_time(time),
        Utc,
    ))
}

fn non_negative(value: f64) -> Option<f64> {
    (value.is_finite() && value >= 0.0).then_some(value)
}

fn failed(result: HResult) -> bool {
    result < 0
}

#[cfg(test)]
#[path = "tests/simconnect.rs"]
mod tests;
