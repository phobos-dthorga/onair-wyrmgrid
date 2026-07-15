# OnAir WyrmGrid

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/key-art/derivatives/hero-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="assets/brand/key-art/derivatives/hero-light.png">
  <img alt="OnAir WyrmGrid — See the network. Command the skies." src="assets/brand/key-art/derivatives/hero-dark.png">
</picture>

**A modular, open-source operations and intelligence platform for OnAir Airline Manager.**

OnAir WyrmGrid is a local-first desktop application for understanding fleets,
jobs, FBO networks, maintenance, finance, flight history, and simulator context
through one connected world map. It is designed as a community platform: most
first-party intelligence modules will use the same versioned, permission-aware
plugin boundary available to third-party developers.

> **Project status:** foundation stage. Session-only OnAir connection, Atlas,
> Hoard, read-only Jobs-to-Dispatch comparison, and the first supervised Python
> plugin proof are implemented. The versioned Bridge supervisor, read-only MSFS
> 2024 SimConnect provider, desktop telemetry view, and permission-filtered
> plugin snapshots are also implemented; live simulator certification, release
> bundling, user-token credential storage, plugin/provider signing and
> sandboxing, and broader operational integrations remain ahead.

## Vision

- **WyrmGrid Atlas** — universal operations map
- **WyrmGrid Hoard** — local history and cached data
- **WyrmGrid Forge** — plugin SDK and developer tools
- **WyrmGrid Aerie** — future community plugin catalogue
- **WyrmGrid Bridge** — MSFS 2024-first simulator and hardware integration
- **WyrmGrid Oracle** — explainable scoring and recommendations
- **WyrmGrid Watch** — alerts and monitoring
- **WyrmGrid Dispatch** — canonical flight-plan inspection with a session-only,
  read-only SimBrief latest-OFP developer preview, explainable OnAir fleet
  and selected-job cross-checks, and session-cached airport METAR/TAF context

## Technical foundation

| Area                         | Initial choice                          |
| ---------------------------- | --------------------------------------- |
| Desktop shell                | Tauri 2                                 |
| Core and services            | Rust 2024                               |
| Interface                    | Svelte 5 and TypeScript                 |
| Map                          | MapLibre GL JS                          |
| Charts                       | Apache ECharts behind WyrmChart         |
| Localization                 | Fluent with canonical `en-AU` fallback  |
| Local storage                | SQLCipher-encrypted SQLite              |
| Plugin boundary              | Out-of-process, versioned JSON messages |
| Native simulator integration | Separate versioned provider sidecars    |

The Rust workspace currently uses eight cohesive libraries plus the desktop and
SimConnect provider executables:

- `wyrmgrid-domain` — stable application-owned types and provenance;
- `wyrmgrid-onair-api` — credential-safe, read-only OnAir boundary;
- `wyrmgrid-storage` — SQLCipher ownership, migrations, and portable backups;
- `wyrmgrid-application` — interface-independent orchestration;
- `wyrmgrid-bridge-protocol` — versioned simulator-provider framing and
  manifests;
- `wyrmgrid-plugin-protocol` — public manifest and permission contracts;
- `wyrmgrid-simbrief-api` — bounded private SimBrief response translation;
- `wyrmgrid-weather-api` — bounded AviationWeather.gov METAR/TAF translation.

`wyrmgrid-simconnect-provider` is the first supervised Bridge sidecar. Other
providers such as FSUIPC use the same stable boundary rather than linking a
native simulator ABI into the desktop process.

## Core promises

1. Plugins do not receive the raw OnAir API key.
2. Raw facts, derived calculations, and recommendations remain distinguishable.
3. OnAir access is treated as read-only unless current official documentation
   proves otherwise.
4. First-party modules prove the public plugin surface wherever practical.
5. Cached data records when and where it was observed.
6. Plugin permissions are explicit and deny-by-default.
7. Unsupported browser or UI automation is outside the official platform.
8. SimBrief, weather, online networks, navigation data, and simulators remain
   optional providers behind application-owned models; the current SimBrief and
   airport-weather flows are explicit, session-only, and read-only.
9. Community language packs are bounded, data-only, locally validated, and
   unable to replace protected legal, credential, permission, or error text.

## Development

### Prerequisites

- Rust 1.97 with the MSVC toolchain on Windows;
- Node.js 22 and npm 10 or newer;
- Tauri's platform prerequisites;
- Microsoft Edge WebView2 on Windows.

The MSFS 2024 SDK is optional for ordinary builds. It is needed for local
SimConnect development and live provider validation; Microsoft SDK binaries are
not stored in this repository.

Then run:

```powershell
npm install
cargo test --workspace --exclude wyrmgrid-desktop
npm run check
npm run dev
```

No OnAir credential is needed to compile or preview the foundation. Never put a
real API key in source code, test fixtures, issue reports, screenshots, or logs.

### Connecting to OnAir

For now, obtain both the Company ID and API Key from **OnAir Client → Options →
Global Settings**. During an authenticated WyrmGrid test on 2026-07-14, the
still-developing **OnAir Companion** supplied values that OnAir's public API
rejected, while the Client-supplied values worked. Companion is expected to
become OnAir's primary client, so this is a temporary compatibility rule that
must be retested when its API credential support reaches parity.

See [Development](docs/development.md), [Architecture](docs/architecture/overview.md),
[display and performance launch options](docs/user-guide/display-and-performance.md),
[settings and measurement units](docs/user-guide/settings-and-units.md),
[External integrations](docs/integrations/README.md),
[Simulator provider authoring](docs/integrations/simulator-provider-authoring.md),
the [simulator experience roadmap](docs/integrations/simulator-experience-roadmap.md),
and [Contributing](CONTRIBUTING.md) before making structural changes.

## Releases

Routine commits and pull requests compile-check the desktop without packaging
installers. Release installers are produced for `vX.Y.0` tags; a deliberately
justified manual exception is available when an intermediate patch genuinely
needs packaging. Early releases remain drafts and prereleases until signing,
updating, and live OnAir integration are deliberately enabled.

## Licensing and trademarks

Source code and documentation are available under the [MIT License](LICENSE).
Official desktop builds present the current
[Application Terms](docs/legal/terms-of-use.md) and
[Privacy Notice](docs/legal/privacy-notice.md) before external map or optional
diagnostic connections begin. The Terms govern the official application without
withdrawing rights already granted under MIT.

OnAir WyrmGrid is an independent community project. It is not affiliated with,
endorsed by, or sponsored by OnAir Company or the developers of OnAir Airline
Manager. “OnAir” and related names may be trademarks of their respective owners.
