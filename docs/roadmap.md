# Roadmap

## Foundation

- Repository, governance, security, CI, and release automation
- Tauri/Svelte/MapLibre application shell
- Stable domain provenance and plugin manifest v1 groundwork
- Read-only credential-safe OnAir adapter and SQLite migration ownership

## Vertical slice 1: company and fleet

- Session-only connection probe with sanitized diagnostics (implemented)
- Optional operating-system credential store
- Company connection plus initial fleet, aircraft, and airport translation
  (implemented); FBO translation remains
- Timestamped in-memory fleet refresh and data age (implemented); SQLite
  snapshots and restart-time offline fallback remain
- Atlas fleet markers, layer toggle, map fitting, and linked aircraft inspector
  (implemented)

## Vertical slice 2: external plugin proof

- Process supervisor and framed protocol messages
- Permission review and persisted grants
- Fleet read and map layer publication capabilities
- Python SDK and idle-aircraft example plugin

## Later modules

- Dispatch and explainable job scoring
- FBO network planning and coverage analysis
- Maintenance, finance, and flight history
- WyrmGrid Bridge simulator telemetry
- Signed plugin packages and WyrmGrid Aerie discovery

Stable plugin APIs, automatic updates, signing, and a public plugin catalogue
require separate readiness reviews; they are not implied by the initial shell.
