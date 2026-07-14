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

The core application uses Rust, Tauri, TypeScript, Svelte, MapLibre, and SQLite.
Public plugins run as separate processes over a language-neutral, versioned,
permission-controlled protocol. Python is the first executable SDK;
Rust, C++, and TypeScript SDKs can follow once the protocol is proven.

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
a session-only, read-only SimBrief latest-OFP developer preview rendered in
Dispatch. Dispatch also produces explainable aircraft identity, model, and
position findings against the observed OnAir fleet and explicitly identifies
payload and deadline facts that the current OnAir slice cannot compare. A
bounded AviationWeather.gov adapter supplies explicitly requested, session-
cached METAR and TAF context for plan airports. Authenticated SimBrief live-field
certification remains outstanding; route-weather advisories, SayIntentions.AI,
MSFS 2024, online networks, and additional navigation sources follow the same
provider-neutral boundary.
