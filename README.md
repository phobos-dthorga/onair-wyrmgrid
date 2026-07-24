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
plugin boundary available to third-party developers. Every component presented
as a plugin or provider is intended to remain an external, independently
installable artifact rather than a feature that exists only when compiled into
WyrmGrid.

> **Project status:** foundation stage. Session-only OnAir connection, Atlas,
> Hoard, read-only Jobs-to-Dispatch comparison, and the first supervised Python
> plugin proof are implemented. The versioned Bridge supervisor, read-only MSFS
> 2024 SimConnect provider, desktop telemetry view, and permission-filtered
> plugin snapshots are also implemented. Ordinary plugin and simulator
> provider package version 1 add offline inspection and installation, immutable
> managed versions, disable, rollback, removal, and separately distributable
> first-party artifacts. Audio capture and codec providers now have matching
> `.wyrmaudio` and `.wyrmcodec` lifecycles with synthetic reference grounding.
> Live native certification, broader user-token credential support, publisher
> signing, sandboxing, and broader operational integrations remain ahead.

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
| Detailed weather             | Three.js WebGPU with WebGL2 fallback    |
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
- `wyrmgrid-audio-provider-protocol` — versioned audio-provider control and
  bounded PCM framing;
- `wyrmgrid-audio-codec-protocol` — versioned out-of-process encoding control
  and bounded packet framing;
- `wyrmgrid-plugin-protocol` — public manifest and permission contracts;
- `wyrmgrid-simbrief-api` — bounded private SimBrief response translation;
- `wyrmgrid-weather-api` — bounded AviationWeather.gov METAR/TAF translation.

`wyrmgrid-simconnect-provider` is the first supervised Bridge sidecar and ships
as a separately installable `.wyrmprovider` artifact. Other providers such as
FSUIPC use the same stable boundary rather than linking a native simulator ABI
into the desktop process.

Audio Capture Providers use the separate supervised audio protocol and ship as
independently installable `.wyrmaudio` artifacts. The current deterministic
reference package is synthetic and does not claim native capture.

Audio Codec Providers ship as independently installable `.wyrmcodec`
artifacts. The first-party Opus codec is packaged, seeded, updated, disabled,
rolled back, and removed through the same public lifecycle as a local community
codec; this does not claim publisher verification or live-device certification.

Extension Developer Kit v1 provides the separately versioned
`wyrmgrid-extension` command for scaffolding, validating, reproducibly
packaging, and locally testing every extension kind without compiling or
checking out WyrmGrid. See the
[extension author guide](docs/integrations/extension-authoring.md). Desktop
installers include the same platform-neutral npm package; Forge can open its
installed directory and the current author-documentation site. Node.js remains
an explicit author prerequisite rather than an application runtime.

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
10. Plugins and providers can be installed, replaced, disabled, updated, and
    removed without rebuilding WyrmGrid; first-party bundling is optional
    convenience, not architectural privilege.

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
[Plugin platform](docs/plugins/overview.md),
[display and performance launch options](docs/user-guide/display-and-performance.md),
[settings and measurement units](docs/user-guide/settings-and-units.md),
[remembered accounts and credentials](docs/user-guide/accounts-and-credentials.md),
[External integrations](docs/integrations/README.md),
[Simulator provider authoring](docs/integrations/simulator-provider-authoring.md),
[external extension authoring](docs/integrations/extension-authoring.md),
the [simulator experience roadmap](docs/integrations/simulator-experience-roadmap.md),
and [Contributing](CONTRIBUTING.md) before making structural changes.

## Releases

Routine commits and pull requests compile-check the desktop without packaging
installers. Every intentional semantic-version tag (`vX.Y.Z`, including
supported prereleases) runs the complete CI and security gates before GitHub
builds platform packages and an NSIS setup executable. CI publishes checksums
and build provenance into a draft prerelease for manual installation review.
The matching reviewed [changelog](CHANGELOG.md) entry supplies the GitHub
release notes, including explicit new-feature, change, removal, and breaking-
change lists.

The WyrmGrid application does not contain, connect to, or require Hoardmind or
any other AI assistant. Hoardmind is the maintainer's private local helper,
outside the application and its release infrastructure. Contributors may work
entirely by hand or, if they choose, use the optional bounded development-task
helpers with their own loopback Ollama or OpenAI-compatible local server. Those
review-only helpers cover change impact, test matrices, documentation sync,
synthetic fixtures, bounded implementation patches, sanitized failure triage,
and release curation without entering the application. A maintainer may
separately publish a wholly generated patch as a bot-attributed commit and
branch through a least-privileged GitHub App, then open its draft PR under the
human maintainer identity. The assistant never receives GitHub credentials or
merge authority. GitHub CI reads only reviewed, checked-in content and never
calls an AI service.

Newer NSIS setups install over the existing per-user application and preserve
its encrypted data; release CI verifies that path against the closest older
published setup.
Early releases remain drafts and prereleases until signing, updating, and live
OnAir integration are deliberately enabled.

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
