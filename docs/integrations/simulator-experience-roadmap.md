# Simulator connection, recording, and in-game experience

The simulator experience has three separate states. WyrmGrid must name them
honestly instead of treating all three as “telemetry”:

1. **Provider available**: the approved sidecar executable is installed and can
   be launched.
2. **Provider connected**: the sidecar has completed the Bridge handshake and is
   receiving live simulator facts.
3. **Flight recording active**: the user has chosen to persist a bounded session
   for later graphs and planned-versus-actual analysis.

The current slice implements all three states for explicit manual recording.
It displays the latest fresh connected snapshot, persists the selected provider,
offers a default-off provider auto-start preference, and records only after the
user selects **Start recording**. Automatic flight detection is not implemented.

## Launch and automation settings

Connection and recording require separate, explicit settings:

| Setting                                                | Default                                          | Effect                                                                                      |
| ------------------------------------------------------ | ------------------------------------------------ | ------------------------------------------------------------------------------------------- |
| Start the selected simulator provider with WyrmGrid    | Off                                              | Starts the approved provider when the desktop opens; it may wait harmlessly for MSFS.       |
| Begin recording when a flight starts                   | Off                                              | Creates a recording only after a connected provider reports reviewed flight-start evidence. |
| End an automatically started recording with the flight | On, but only when automatic recording is enabled | Closes the session after reviewed flight-end evidence and a bounded settling period.        |

Enabling automatic recording must show what is stored, the retention policy,
and how to stop or delete it. Changing the selected provider, losing the
connection, pausing, teleporting, replaying, or returning to the main menu must
produce explicit session events rather than silently joining unrelated samples.

The provider can subscribe to documented MSFS lifecycle/flow events, but a
shared Rust state machine remains authoritative. It combines events with
bounded telemetry evidence and keeps ambiguous transitions visible for user
confirmation. Automatic retry must never create duplicate recording sessions.

Provider auto-start is intentionally narrower than recording automation. It
launches only the selected, manifest-validated sidecar and does not start MSFS,
record a flight, or broaden plugin permissions. Manually starting a provider
also remembers it as the selected provider. A provider process failure remains
visible and manually restartable rather than entering an unbounded crash loop.

The SimConnect provider retries simulator connections after MSFS closes or
disconnects. While connected, the host ages the latest sample and withholds it
after five seconds without a replacement. The UI reports that condition as
stale rather than continuing to present the previous aircraft state as live.

### Live certification checklist

Before calling MSFS 2024 telemetry certified, exercise and record the result of
each case with the packaged provider:

- WyrmGrid starts with auto-start off and does not launch the provider;
- auto-start on launches the selected provider while MSFS is absent;
- MSFS free flight changes waiting state to connected and supplies fresh facts;
- pausing and resuming preserves an honest live snapshot;
- returning to the main menu removes the previous aircraft snapshot;
- closing and restarting MSFS recovers without restarting WyrmGrid;
- changing aircraft replaces identity and measurements without joining history;
- a provider process failure is visible and can be manually restarted; and
- a stalled connected stream becomes stale and hides the last snapshot.

## In-simulator control surface

The preferred direction is a small MSFS 2024 add-on control surface that shows:

- WyrmGrid desktop/provider presence and Bridge version;
- connected, waiting, recording, paused, degraded, and disconnected state;
- **Start recording**, **Stop recording**, and an optional **Add marker** action;
- elapsed recording time and a deliberately small live summary; and
- a clear instruction when the external WyrmGrid process is not available.

The panel is a controller, not another telemetry implementation. MSFS 2024's
documented CommBus can pass asynchronous messages among an external SimConnect
client, JavaScript gauges, and WASM modules. A feasibility spike should prove an
aircraft-independent in-game or toolbar surface and a named CommBus contract
before a package format is committed.

An in-simulator panel cannot launch an absent Windows executable from the
simulator sandbox. To avoid an alt-tab, the desktop and provider must already be
running—normally through the opt-in **Start the selected simulator provider with
WyrmGrid** setting or a separately reviewed operating-system startup option.
When no trusted listener exists, the panel remains read-only and explains that
WyrmGrid must be opened.

The CommBus control contract needs its own version, sender identity, per-process
nonce, bounded JSON messages, rate limits, replay rejection, fixtures, and
capability checks. It must not expose OnAir credentials, plugin grants, local
paths, raw telemetry, arbitrary Bridge commands, provider installation, or
simulator mutation. Starting and stopping a local recording are distinct from
future simulator-write capabilities.

Microsoft recommends out-of-process SimConnect clients for stability, so the
existing sidecar remains the data authority. A standalone WASM telemetry module
would duplicate the provider and move failure into the simulator; it is not the
default design.

## Recording model

A recording should use append-only SQLite migrations and application-owned
models:

- `SimulatorSession`: stable ID, provider/simulator/aircraft provenance,
  start/end evidence, selected plan reference, recording origin, and status;
- `SimulatorSample`: session ID, monotonic sequence, simulator and observation
  time, bounded telemetry values, and gap/discontinuity markers;
- `SimulatorSessionEvent`: connect, disconnect, pause, resume, take-off,
  landing, block events, user markers, and user corrections; and
- `SimulatorSessionSummary`: duration, distance, fuel used, phase totals, and
  explicitly versioned calculations.

At the current one-hertz host rate, storage is manageable, but retention is
still user-owned. WyrmGrid should offer per-session deletion, delete-all,
export, a visible retention setting, and bounded automatic pruning. Raw native
SimConnect messages are never persisted.

Migration `0008_simulator_recordings.sql` now supplies sessions, bounded
translated samples, and a separate retention preference. The initial interface
offers 7, 30, 90, and 365-day retention, defaults to 30 days, prunes completed
or interrupted sessions, and provides per-session and delete-all controls.
Active sessions cannot be deleted. Export, pinning, lifecycle events, summaries,
and automatic recording remain follow-ups.

An aircraft or registration change interrupts an active session instead of
joining unrelated samples. A provider sequence discontinuity or observation
gap longer than three seconds marks the next sample as a gap. An application
restart marks any abandoned active session interrupted.

## Graphs

WyrmChart should render host-owned, provenance-aware series rather than raw
ECharts options. The first useful graphs are:

1. altitude with take-off/landing and pause/disconnect markers;
2. indicated airspeed, true airspeed, and ground speed;
3. fuel weight and gross weight;
4. pitch and bank for a selected time window; and
5. planned versus actual altitude, speed, time, and fuel when a correlated plan
   exists.

Vertical speed, engine detail, control inputs, weather along track, and
aircraft-specific variables should be added only after their contracts are
verified. Graphs must show missing intervals rather than interpolating through a
disconnect. Long sessions should use a tested min/max-envelope or equivalent
downsampling service while retaining exact values for a bounded inspection
window.

The initial WyrmChart view renders altitude plus indicated, true, and ground
speed for the latest 600 exact samples. Gap markers insert a visible break and
the renderer does not connect across it. This bounded window does not yet
downsample or browse older windows; min/max-envelope downsampling remains
required before an entire long session can be graphed honestly.

Plugins do not automatically receive recorded sessions. Any future history or
aggregate permission must be separate from `simulator_telemetry_read`, scoped to
user-selected sessions, and exclude raw high-frequency data by default.

## Delivery sequence

1. Finish live certification of connected MSFS 2024 telemetry using the
   documented recovery matrix. Provider recovery states, persisted provider
   selection, stale-snapshot suppression, and default-off provider auto-start
   are implemented.
2. Introduce the recording/session schemas, retention controls, and explicit
   manual recording. Implemented with local deletion and bounded graph windows;
   export and pinning remain.
3. Add tested lifecycle evidence and the default-off automatic recording
   setting.
4. Expand the implemented altitude/speed WyrmChart window with gap-aware
   min/max-envelope downsampling, event markers, and older-window navigation.
5. Prove the MSFS in-simulator CommBus control surface, then package it only
   after compatibility, signing, installation, and removal are documented.

## Questions and suggestions

- Should automatic recording begin at flight-start, first engine start, or
  block-off? Suggestion: store the raw lifecycle candidates, let the user choose
  a default policy, and never silently rewrite the session boundary.
- Should WyrmGrid retain full one-hertz sessions indefinitely? Suggestion: no;
  start with user-visible age/size retention and pinning for named flights.
- Should the in-game surface ship through a Community-folder package or another
  supported distribution path? This needs an SDK/package and signing spike.
- Should recordings begin when WyrmGrid is minimized? Suggestion: yes only after
  the user enables automatic recording and the system tray/status treatment is
  explicit.
- Which graph should lead the first release? Suggestion: altitude plus speed and
  event markers, because it tests the session timeline, gaps, lifecycle events,
  and downsampling without requiring aircraft-specific facts.

## Official references

- [MSFS 2024 SimConnect SDK](https://docs.flightsimulator.com/msfs2024/html/6_Programming_APIs/SimConnect/SimConnect_SDK.htm)
- [MSFS 2024 Communication API](https://docs.flightsimulator.com/msfs2024/flighting/html/6_Programming_APIs/SimConnect/API_Reference/Communication/Communication_API.htm)
- [MSFS 2024 flow events](https://docs.flightsimulator.com/msfs2024/html/6_Programming_APIs/SimConnect/API_Reference/Structures_And_Enumerations/SIMCONNECT_FLOW_EVENT.htm)
- [MSFS 2024 JavaScript instruments](https://docs.flightsimulator.com/msfs2024/html/6_Programming_APIs/JavaScript/JavaScript.htm)
