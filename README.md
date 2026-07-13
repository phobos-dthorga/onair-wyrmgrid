# OnAir WyrmGrid

**A modular, open-source operations and intelligence platform for OnAir Airline Manager.**

OnAir WyrmGrid is a local-first desktop application for understanding fleets,
jobs, FBO networks, maintenance, finance, flight history, and simulator context
through one connected world map. It is designed as a community platform: most
first-party intelligence modules will use the same versioned, permission-aware
plugin boundary available to third-party developers.

> **Project status:** foundation stage. The application shell, domain boundary,
> read-only OnAir adapter, SQLite migration owner, and plugin manifest contract
> exist. Live OnAir authentication and data synchronization are the first
> vertical slice, not a completed feature.

## Vision

- **WyrmGrid Atlas** — universal operations map
- **WyrmGrid Hoard** — local history and cached data
- **WyrmGrid Forge** — plugin SDK and developer tools
- **WyrmGrid Aerie** — future community plugin catalogue
- **WyrmGrid Bridge** — simulator and hardware integration
- **WyrmGrid Oracle** — explainable scoring and recommendations
- **WyrmGrid Watch** — alerts and monitoring
- **WyrmGrid Dispatch** — job and route planning

## Technical foundation

| Area                         | Initial choice                          |
| ---------------------------- | --------------------------------------- |
| Desktop shell                | Tauri 2                                 |
| Core and services            | Rust 2024                               |
| Interface                    | Svelte 5 and TypeScript                 |
| Map                          | MapLibre GL JS                          |
| Charts                       | Apache ECharts behind WyrmChart         |
| Local storage                | SQLite                                  |
| Plugin boundary              | Out-of-process, versioned JSON messages |
| Native simulator integration | Separate C++ sidecars where justified   |

The Rust workspace deliberately starts with only five core libraries:

- `wyrmgrid-domain` — stable application-owned types and provenance;
- `wyrmgrid-onair-api` — credential-safe, read-only OnAir boundary;
- `wyrmgrid-storage` — SQLite ownership and migrations;
- `wyrmgrid-application` — interface-independent orchestration;
- `wyrmgrid-plugin-protocol` — public manifest and permission contracts.

## Core promises

1. Plugins do not receive the raw OnAir API key.
2. Raw facts, derived calculations, and recommendations remain distinguishable.
3. OnAir access is treated as read-only unless current official documentation
   proves otherwise.
4. First-party modules prove the public plugin surface wherever practical.
5. Cached data records when and where it was observed.
6. Plugin permissions are explicit and deny-by-default.
7. Unsupported browser or UI automation is outside the official platform.

## Development

### Prerequisites

- Rust 1.97 with the MSVC toolchain on Windows;
- Node.js 22 and npm 10 or newer;
- Tauri's platform prerequisites;
- Microsoft Edge WebView2 on Windows.

Then run:

```powershell
npm install
cargo test --workspace --exclude wyrmgrid-desktop
npm run check
npm run dev
```

No OnAir credential is needed to compile or preview the foundation. Never put a
real API key in source code, test fixtures, issue reports, screenshots, or logs.

See [Development](docs/development.md), [Architecture](docs/architecture/overview.md),
and [Contributing](CONTRIBUTING.md) before making structural changes.

## Releases

Version tags matching `v*` trigger reproducible desktop builds for Windows,
Linux, and macOS. Early releases remain drafts and prereleases until signing,
updating, and live OnAir integration are deliberately enabled.

## Licensing and trademarks

Source code and documentation are available under the [MIT License](LICENSE).

OnAir WyrmGrid is an independent community project. It is not affiliated with,
endorsed by, or sponsored by OnAir Company or the developers of OnAir Airline
Manager. “OnAir” and related names may be trademarks of their respective owners.
