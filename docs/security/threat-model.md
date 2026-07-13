# Threat model

## Protected assets

- OnAir API credentials and company identifiers;
- fleet, FBO, employee, finance, job, and flight history;
- local files and operating-system access;
- plugin trust decisions and signatures;
- update integrity and release artifacts.

## Primary threats

- credential disclosure through logs, errors, telemetry, storage, or plugins;
- malicious plugin manifests, executables, dependencies, and messages;
- path traversal and unsafe process arguments;
- unbounded messages, event storms, hangs, and resource exhaustion;
- hostile API payloads, imported files, map styles, and URLs;
- dependency or release-pipeline compromise;
- stale data presented as current fact;
- historical operational state mistaken for the present state;
- recommendations mistaken for OnAir-provided facts;
- accidental or automated request storms against OnAir's public API;
- disclosure of locally cached company, fleet, and location history;

## Initial controls

- secrets wrapped and redacted at the adapter boundary;
- API keys move from the password field into a Rust `SecretString`, remain only
  for the active process, and are dropped on Disconnect or application exit;
- connection errors are mapped to bounded user-facing categories instead of
  relaying remote response bodies;
- current credential guidance directs users to OnAir Client and warns against
  visually similar but not-yet-compatible values from OnAir Companion;
- read-only API design;
- explicit provenance and observation timestamps;
- deny-by-default plugin capabilities;
- relative plugin entry-point validation;
- content security policy for the desktop webview;
- locked dependencies, dependency updates, audit jobs, and CI-built releases;
- no plugin runtime until framing, lifecycle, limits, and permission review are
  specified and tested.
- company synchronization is serialized in Rust; trigger-specific quiet periods
  silently return cached state without making another remote request. Fleet and
  FBO reads are sequential, and an authentication or rate-limit failure stops
  the second request.
- Hoard stores stable domain snapshots rather than raw API payloads, never stores
  credentials, applies bounded retention, and visibly distinguishes live,
  cached, offline, preview, and memory-only data.
- Hoard Timeline remains read-only, persistently identifies mutually exclusive
  LIVE or HISTORICAL workspace mode separately from data availability, shows
  the selected time and each resource's actual observation time, and offers an
  explicit return-to-present action.
- chart contributions are data-only; the host rejects executable callbacks,
  arbitrary ECharts options, HTML tooltips, non-finite values, oversized series,
  and charts published without `charts_publish`.

Before stable release, the project needs operating-system credential storage,
signed updates, hardened plugin supervision, abuse-case tests, and a formal
security review of every external input boundary.

## Residual connection risks

Session-only handling prevents normal disk persistence, but it cannot promise
that a secret is absent from process memory, operating-system crash dumps, or a
compromised host. The frontend necessarily holds the entered value briefly
before invoking Rust and clears it after success, disconnect, or dialog close.
WyrmGrid therefore makes no claim of hardened secret storage until a reviewed
operating-system credential-store implementation is introduced.

Credentials copied from the wrong OnAir product are an availability and support
risk rather than a confidentiality control failure. For now, the interface
identifies OnAir Client and warns that Companion has not reached credential
parity; authentication errors repeat that recovery instruction without echoing
either entered value. When Companion becomes the primary compatible client,
this guidance must change without weakening secret handling.

## Residual Hoard risks

The local SQLite database contains company identifiers, company names, aircraft
and FBO details, locations, and observation history. It is not currently encrypted at
rest and relies on operating-system account and filesystem protections. A user
must therefore sanitize or omit `wyrmgrid.db` from support reports. Retention
limits intraday growth but deliberately preserves one daily historical record,
so sensitive operational history remains until the user deletes the database or
a future data-management feature removes it.
