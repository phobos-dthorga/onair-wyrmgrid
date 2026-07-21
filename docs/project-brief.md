# Project brief

OnAir WyrmGrid is a modular, open-source operations and intelligence platform
for OnAir Airline Manager. Its central workspace is a universal map linking
fleet management, job intelligence, FBO network planning, maintenance, finance,
flight history, route planning, simulator integration, and community-created
extensions.

The project is local-first. It stores timestamped observations in SQLite,
supports useful offline analysis, keeps credentials on the player's machine,
and avoids requiring hosted infrastructure for ordinary use.

Optional integrations add SimBrief operational plans, SayIntentions.AI ATC and
crew context, aviation weather, VATSIM/IVAO network context, navigation data,
portable flight-plan formats, and simulator actuals. MSFS 2024 is the primary
WyrmGrid Bridge target. Every provider remains independently disconnectable and
visibly sourced.

Optional hosted diagnostics must not become an availability dependency. Public
telemetry remains disclosed and user-controlled, and ordinary application work
continues when the diagnostic service is unavailable.

A future public website and WyrmGrid Aerie may provide documentation, release
discovery, and moderated community-package distribution. A separately gated
private vault may store an existing client-encrypted portable backup as an
opaque object. None of these services is required for ordinary use, and they do
not imply live database synchronization. Their proposed trust boundaries and
delivery gates are recorded in
[ADR-0019](architecture/decisions/0019-hosted-web-aerie-and-private-vault.md)
and the [hosted-platform plan](operations/hosted-platform.md).

The core application uses Rust, Tauri, TypeScript, Svelte, MapLibre, Three.js,
and SQLite.
Public plugins run as separate processes over a language-neutral, versioned,
permission-controlled protocol. Python is the first executable SDK;
Rust, C++, and TypeScript SDKs can follow once the protocol is proven.
Every plugin and provider also has an external delivery boundary: it can be
installed, replaced, disabled, updated, and removed independently of the
WyrmGrid build. Payloads may be scripts, executables, native libraries required
by another host such as a simulator, or future validated runtimes. Official
installers may include first-party packages for convenience, but compilation
into the desktop is never the only plugin delivery route.

English (Australia) is the canonical interface source, but language is not a
business-logic concern. Stable semantic keys cross from Rust into Svelte, where
Project Fluent applies a selected, locally stored community language pack with
per-message English fallback. Community packs are data-only and cannot replace
protected legal, credential, permission, destructive-action, or diagnostic
messages without a future reviewed distribution path.

The project grows through narrow vertical slices. The first slice is company
and fleet retrieval, validation, persistence, map display, selection, and a
non-blocking refresh path. The second proves the external plugin system with a
small Fleet Locations map layer built only from known location facts.

The operational track now includes canonical `FlightPlanSnapshot` version 1 and
a read-only SimBrief latest-OFP developer preview rendered in Dispatch. A Pilot
ID or username can be remembered independently while the imported plan remains
session-only. Dispatch also produces explainable aircraft identity, model, and
position findings against the observed OnAir fleet and explicitly identifies
payload and deadline facts that the current OnAir slice cannot compare. A
bounded AviationWeather.gov provider plugin supplies explicitly requested,
session-cached METAR and TAF context for plan airports. Independently approved
Open-Meteo and RainViewer plugins provide coarse global model samples and
recent timestamped RADAR frames through the same host-owned weather contract.
Dispatch and Atlas add coarse, explicitly supported model context along mapped
plan segments without sending the plan to a plugin. Authenticated SimBrief
live-field certification remains outstanding; route-weather advisories,
SayIntentions.AI, MSFS 2024, online networks, and additional
navigation sources follow the same provider-neutral boundary.
