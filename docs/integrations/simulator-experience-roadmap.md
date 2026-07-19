# Simulator connection, recording, and in-game experience

The simulator experience has three separate states. WyrmGrid must name them
honestly instead of treating all three as “telemetry”:

1. **Provider available**: the approved sidecar executable is installed and can
   be launched.
2. **Provider connected**: the sidecar has completed the Bridge handshake and is
   receiving live simulator facts.
3. **Flight recording active**: the user has chosen to persist a bounded session
   for later graphs and planned-versus-actual analysis.

The current slice implements all three states for manual recording and for an
opt-in evidence-led automatic lifecycle. It displays the latest fresh connected
snapshot, persists the selected provider, and keeps provider auto-start and
recording automation as separate default-off choices.

Planned simulator-synchronised audio adds a fourth independent state: **audio
recording active**. It is never implied by provider availability, connection,
manual telemetry recording, or automatic telemetry recording. Audio has its own
default-off consent, negotiated sources, failures, retention, and visible
indicator as defined in the
[audio-recording plan](simulator-audio-recording.md).

## Simulator weather evidence

A recording must not equate the simulated atmosphere with external real-world
weather. Future Bridge telemetry should preserve three independent facts:

1. the simulator-reported weather mode or scenario (`live`,
   `preset_or_custom`, or `unknown` after SDK verification);
2. bounded ambient conditions observed at the aircraft, with simulator and
   observation time; and
3. any separately retrieved external weather snapshot used for planning or
   later comparison.

MSFS 2024 documents a weather-mode event and readable ambient variables such as
precipitation, visibility, pressure, temperature, wind, and cloud density. A
dedicated SDK spike must prove which values and transitions are reliable in
free flight, presets, custom weather, replay, pause, menu transitions, and
reconnection before the sidecar protocol changes. Ambient similarity must
never be used to infer that Live Weather was active.

When implemented, Hoard records mode transitions as symbolic events and
ambient values as simulator observations alongside the normal gap rules. It
keeps external METAR/TAF/radar products separately attributed so a debrief can
compare the weather the user chose, what the aircraft experienced, and what an
external provider reported without calling any stream the other. This requires
a protocol revision, sanitized fixtures, old/new compatibility decision,
recording schema review, and threat-model update; no fields are added to the
current protocol spec merely on assumption.

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

The Rust recording service is authoritative. Version 1 confirms take-off from
two increasing, unpaused direct `on_ground = false` observations and confirms
landing only after continuous `on_ground = true` evidence for the configured
settling period. A pause, zero simulation rate, identity change, or telemetry
gap resets the relevant candidate. Automatic retry cannot create a second
session while any recording is active.

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

These models describe translated telemetry, not encoded media. Planned audio
links to a `SimulatorSession` through separate metadata while segmented Opus
content remains in encrypted external media files. Audio bytes do not become
SQLite BLOBs, Bridge messages, telemetry exports, or plugin history. See
[ADR-0017](../architecture/decisions/0017-simulator-synchronised-audio-recording.md).

At the current one-hertz host rate, storage is manageable, but retention is
still user-owned. WyrmGrid should offer per-session deletion, delete-all,
export, a visible retention setting, and bounded automatic pruning. Raw native
SimConnect messages are never persisted.

Migration `0008_simulator_recordings.sql` supplies sessions, bounded
translated samples, and a separate retention preference. The initial interface
offers 7, 30, 90, and 365-day retention, defaults to 30 days, prunes completed
or interrupted sessions, and provides per-session and delete-all controls.
Active sessions cannot be deleted. Append-only migration
`0010_simulator_evidence.sql` adds automatic preferences, plan association,
pinning, direct position/lifecycle facts, and symbolic session events without
rewriting the shipped recording tables. Hoard provides JSON/CSV export and
exact older/newer windows. Pinned sessions survive automatic retention pruning
but remain subject to explicit deletion.

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

Hoard Flight Debrief schema 1 renders whole-flight altitude; indicated, true,
and ground speed; fuel weight; and pitch/bank. Traces up to 1,200 points remain
exact. Longer traces use a deterministic, tested min/max envelope capped at
1,200 display points per graph while preserving endpoints, each graph series'
extrema, and every source or missing-value gap. Source reads are capped at
250,000 samples. The selected 600-sample exact window remains available for
forensic inspection and exact JSON/CSV exports are unchanged.

Correlation version 2 adds labelled SimBrief reference lines for initial
altitude, duration, and take-off/landing fuel weight. It does not generate a
planned climb profile or fuel-burn curve. A historical flight can pass its
bounded plan-and-recording route view to Atlas, where unresolved plan legs and
recording gaps split the geometry rather than being joined.

Plugins do not automatically receive recorded sessions. Any future history or
aggregate permission must be separate from `simulator_telemetry_read`, scoped to
user-selected sessions, and exclude raw high-frequency data by default.

## Delivery sequence

1. Finish live certification of connected MSFS 2024 telemetry using the
   documented recovery matrix. Provider recovery states, persisted provider
   selection, stale-snapshot suppression, and default-off provider auto-start
   are implemented.
2. Introduce the recording/session schemas, retention controls, explicit manual
   recording, local deletion, export, pinning, and bounded graph windows.
   Implemented.
3. Add tested lifecycle evidence, the default-off automatic recording setting,
   and reviewable take-off, gap, plan-association, and landing events.
   Implemented.
4. Correlate sanitized SimBrief plans with exact session facts, browse older
   windows, review tested whole-flight debriefs, and open planned/recorded route
   overlays in Atlas. Implemented for correlation version 2 and debrief/route
   schemas 1.
5. Prove the MSFS in-simulator CommBus control surface, then package it only
   after compatibility, signing, installation, and removal are documented.
6. Prove simulator weather-mode and ambient-condition observability, then add a
   versioned, gap-preserving recording contract that remains distinct from
   external weather.
7. Add the separately consented Audio Capture Provider contract, Opus media
   store, and fake-provider tests before native capture. Deliver Windows/MSFS
   sources first, then X-Plane on its supported desktop systems; isolated
   X-Plane radio tracks wait for the plugin feasibility and licensing decision.

## Questions and suggestions

- Should later policy choices add engine-start or block-off boundaries alongside
  the shipped take-off default? Suggestion: retain candidates as named events,
  require an explicit user policy, and never silently rewrite an old boundary.
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
- [MSFS 2024 receive IDs, including weather-mode events](https://docs.flightsimulator.com/msfs2024/html/6_Programming_APIs/SimConnect/API_Reference/Structures_And_Enumerations/SIMCONNECT_RECV_ID.htm)
- [MSFS 2024 ambient variables](https://docs.flightsimulator.com/msfs2024/html/6_Programming_APIs/SimVars/Miscellaneous_Variables.htm)
- [MSFS 2024 JavaScript instruments](https://docs.flightsimulator.com/msfs2024/html/6_Programming_APIs/JavaScript/JavaScript.htm)
