# Threat model

## Protected assets

- OnAir API credentials and company identifiers;
- fleet, FBO, employee, finance, job, and flight history;
- SimBrief identifiers and OFPs, SayIntentions API keys, account identity,
  communications and active-flight files, imported routes, weather caches,
  online-network activity, and simulator telemetry;
- Navigraph, IVAO, community-delivery, and future provider tokens or application
  credentials;
- local files and operating-system access;
- plugin trust decisions and signatures;
- diagnostic events, telemetry preferences, and Sentry report identifiers;
- release source maps, native debug information, and telemetry upload credentials;
- legal-document versions, acknowledgement records, and privacy preferences;
- selected language, imported community language-pack content, translator
  metadata, and the integrity of security-sensitive interface wording;
- update integrity and release artifacts.

## Primary threats

- credential disclosure through logs, errors, telemetry, storage, or plugins;
- malicious plugin manifests, executables, dependencies, and messages;
- path traversal and unsafe process arguments;
- unbounded messages, event storms, hangs, and resource exhaustion;
- hostile API payloads, imported files, map styles, and URLs;
- imported themes concealing security text, counterfeiting controls, loading
  remote resources, or exhausting local storage;
- malicious or stale translations mislabelling credentials, permissions,
  destructive actions, diagnostics, provenance, or legal disclosures;
- malicious or oversized OFPs, flight-plan files, compressed feeds, navigation
  packages, weather geometries, and simulator messages;
- spoofed localhost simulator services, sidecars, callbacks, or OAuth redirect
  state;
- provider schema drift, identifier collisions, AIRAC mismatch, and unit or
  timestamp confusion producing plausible but incorrect plans;
- personal network data, routes, coordinates, callsigns, or free-form content
  leaking through persistence, plugins, support output, or diagnostics;
- embedded desktop application secrets being extracted and abused;
- a SayIntentions key leaking through query URLs, redirects, HTTP diagnostics,
  local `flight.json` parsing, support bundles, or automatic retries;
- external writes or simulator commands occurring without explicit user intent;
- dependency or release-pipeline compromise;
- sensitive data escaping through diagnostic payloads, attachments, replay,
  logs, traces, or crash dumps;
- network collection beginning before disclosure or continuing after the user
  withdraws an optional preference;
- forged or flooded diagnostic events consuming quota or obscuring failures;
- telemetry outages delaying application work or degrading offline behaviour;
- compromise of CI-only Sentry upload credentials;
- stale data presented as current fact;
- historical operational state mistaken for the present state;
- recommendations mistaken for OnAir-provided facts;
- accidental or automated request storms against OnAir's public API;
- disclosure of locally cached company, fleet, and location history;
- request storms or costly generated actions against SimBrief, SayIntentions,
  weather, VATSIM, IVAO, or navigation providers;

## Initial controls

- secrets wrapped and redacted at the adapter boundary;
- API keys move from the password field into a Rust `SecretString`, remain only
  for the active process, and are dropped on Disconnect or application exit;
- connection errors are mapped to bounded user-facing categories instead of
  relaying remote response bodies;
- current credential guidance directs users to OnAir Client and warns against
  visually similar but not-yet-compatible values from OnAir Companion;
- read-only OnAir API design;
- explicit provenance and observation timestamps;
- raw external payloads remain inside provider adapters and translate into
  immutable, application-owned snapshots with freshness and provider revision;
- provider rate limits, caching, request coalescing, timeouts, bounded retries,
  and offline suspension are enforced in Rust;
- imported files, compressed feeds, navigation packages, weather geometries,
  and Bridge messages have strict size, count, nesting, numeric, path, and
  decompression limits;
- user tokens belong in the operating-system credential store; shared
  application secrets are never embedded in desktop binaries or public sites;
- SayIntentions `flight.json` is read only after opt-in, parsed through a strict
  allowlist, and never persisted raw; its API key becomes an in-memory secret
  immediately and its documented HTTPS origin is pinned independently of any
  hostname in the file;
- secret-bearing SAPI URLs, redirects, and HTTP client errors are converted to
  bounded codes before logging, telemetry, persistence, or display;
- SayIntentions communications and airport mutations require an explicit user
  action, have no ambiguous automatic retry, and are subject to cooldown,
  duplicate suppression, content limits, and later per-flight automation
  budgets;
- OAuth and browser-return flows require PKCE where supported, exact redirect
  validation, state verification, and system-browser authentication;
- public online-network adapters discard names, member IDs, remarks, and other
  fields not required by the implemented view before persistence or display;
- Bridge sidecars use versioned handshakes, explicit capabilities, bounded
  framing, supervised lifecycle, and local-only provider connections;
- simulator plan loading and every other external mutation require a distinct
  negotiated capability and explicit user action;
- deny-by-default plugin capabilities persisted separately from manifests; the
  current runtime starts only after every requested capability is approved and
  implements only sanitized fleet reads and data-only Atlas publication;
- plugin directories, manifests, and entry points are bounded and canonicalized;
  symlinked folders/files, path escape, malformed metadata, and unsupported
  runtimes or capabilities are rejected;
- plugin protocol v1 uses a 1 MiB length-prefixed JSON ceiling, independent
  monotonic sequences, a three-second identity/version handshake, bounded text
  and WGS84 validation, at most 16 layers per plugin, and at most 10,000 points
  per layer;
- Python launches in isolated mode with a scrubbed environment and receives only
  translated stable domain snapshots; the host does not place the OnAir key,
  raw OnAir JSON, provider credentials, Sentry settings, or another plugin's
  traffic in child messages or environment variables;
- plugin stdout is protocol-only and stderr is discarded; untrusted output,
  message bodies, coordinates, identifiers, and paths are not relayed into the
  interface, normal logs, or telemetry;
- supervised children receive a bounded graceful shutdown and are forcibly
  terminated after the deadline or when the host exits;
- content security policy for the desktop webview;
- locked dependencies, dependency updates, audit jobs, and CI-built releases;
- Dependency Review allows one documented low-severity SvelteKit `cookie`
  advisory only while WyrmGrid remains a static desktop client with no HTTP
  cookie-writing surface; the exception must be removed when a compatible fix
  exists or that boundary changes;
- Sentry is an optional outer adapter; domain and application services do not
  depend on it, and telemetry failure never blocks normal application work;
- initial collection is error-only; replay, logs, performance tracing,
  profiling, attachments, and native minidumps remain disabled by default;
- diagnostic payloads use an allowlist and redaction tests; OnAir keys, raw
  payloads, database rows, local paths, plugin traffic, and simulator data are
  excluded;
- telemetry is off by default; first-run onboarding prevents Atlas from mounting
  before the current Terms and Privacy Notice are acknowledged, and stale
  document versions suppress both Rust and interface diagnostics until review;
- an optional user preference and a deliberately configured build are both
  required before diagnostics can be transmitted;
- Sentry authentication tokens remain CI secrets; DSNs are treated as public
  submission endpoints and restricted to the narrowest required ingress origin
  in desktop content-security policy;
- Forge labels the Python runtime a developer preview and states that capability
  review is not an operating-system sandbox;
- company synchronization is serialized in Rust; trigger-specific quiet periods
  silently return cached state without making another remote request. Fleet,
  FBO, and pending-job reads are sequential, and an authentication or rate-limit
  failure stops later requests. Raw recursive mission objects stay in the adapter;
  stable job snapshots enforce job, leg, text, numeric, coordinate, and schema limits.
- Dispatch job selection carries only a validated Hoard observation. It exposes
  no OnAir acceptance command, and route, payload, and expiry findings remain
  calculated comparisons rather than OnAir instructions or guarantees.
- Hoard stores stable domain snapshots rather than raw API payloads, never stores
  credentials, applies bounded retention, and visibly distinguishes live,
  cached, offline, preview, and memory-only data.
- Hoard Timeline remains read-only, persistently identifies mutually exclusive
  LIVE or HISTORICAL workspace mode separately from data availability, shows
  the selected time and each resource's actual observation time, and offers an
  explicit return-to-present action. Historical selection is not restored after
  restart; startup deliberately returns to LIVE mode to prevent stale context
  from silently carrying into a new session.
- chart contributions are data-only; the host rejects executable callbacks,
  arbitrary ECharts options, HTML tooltips, non-finite values, oversized series,
  and charts published without `charts_publish`.
- community themes are data-only, limited to 32 KiB, parsed with a strict
  versioned schema, restricted to fixed hexadecimal colour roles and a bounded
  chart palette, and contrast-checked in Rust. Unknown fields, arbitrary CSS,
  code, markup, URLs, fonts, images, paths, selectors, layout, and reserved host
  identifiers are rejected before persistence.
- community language packs are data-only, limited to 256 KiB, parsed and
  canonicalized in Rust, restricted to known source-catalogue keys, and checked
  for schema/source version, metadata, Fluent syntax, variable parity, message
  counts, markup delimiters, and dangerous bidirectional controls. Partial packs
  fall back per message to canonical English. Unreviewed packs cannot replace
  legal, privacy, credential, telemetry, plugin-permission, destructive-action,
  or diagnostic namespaces and cannot load resources or execute code.

Before stable release, the project needs operating-system credential storage,
signed updates, hardened plugin supervision, abuse-case tests, and a formal
security review of every external input boundary.

The current Atlas basemap is downloaded from MapLibre's public demonstration
infrastructure after onboarding. WyrmGrid does not intentionally include OnAir
payloads in those requests, but ordinary network metadata reaches that service.
Production suitability, retention, attribution, availability limits, and a
replacement or approval decision remain stable-release requirements.

## Residual localization risks

A translation can be grammatically valid and still be inaccurate, incomplete,
offensive, or intentionally misleading. Protected namespaces reduce the most
serious risks but do not make ordinary community wording trustworthy. WyrmGrid
therefore identifies pack provenance, reports coverage, preserves English
fallback, and keeps a built-in English selection available. Reviewed translation
governance, signatures, revocation, update compatibility, complete right-to-left
testing, font/script coverage, and a dedicated legal-document process remain
required before WyrmGrid endorses non-English packs.

Locale direction can materially rearrange presentation. Version 1 changes the
root writing direction but the existing interface has not completed logical-CSS
or RTL visual certification, so community RTL packs are an authoring preview.
Fluent isolation protects interpolated mixed-direction values but cannot correct
the meaning of a poor translation.

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

## Residual plugin risks

- Process separation and host capability checks are not an operating-system
  sandbox. A Python plugin still runs as the current user and may access files,
  processes, and ambient network facilities allowed to that account even when
  `external_network` is not granted. Run only trusted code in this preview.
- Plugin packages and publishers are not signed, updates are not authenticated,
  and installed files have no tamper-evident integrity record. The bundled proof
  is copied only when absent so user files are not silently overwritten.
- The supervisor bounds frames, layers, points, paths, startup, and shutdown but
  does not yet impose CPU, memory, total-process, message-rate, restart, or disk
  quotas. A hostile plugin can still exhaust local resources within operating-
  system limits.
- Permission approval controls only host-provided WyrmGrid capabilities. It does
  not inspect dependencies, prove code intent, or prevent a plugin from using
  Python's own libraries and operating-system interfaces.
- Python discovery and isolated launch depend on the locally installed runtime.
  Signed runtime packaging, supported-version policy, dependency locking, SDK
  conformance testing, sandbox profiles, revocation testing, and safe
  update/rollback are required before recommending unreviewed community plugins.

The exact implemented boundary and deferred hardening are recorded in
[plugin protocol version 1](../plugins/protocol-v1.md).

## Residual Hoard risks

The local SQLite database contains company identifiers, company names, aircraft
and FBO details, locations, and observation history. It is not currently encrypted at
rest and relies on operating-system account and filesystem protections. A user
must therefore sanitize or omit `wyrmgrid.db` from support reports. Retention
limits intraday growth but deliberately preserves one daily historical record,
so sensitive operational history remains until the user deletes the database or
a future data-management feature removes it.

## Residual telemetry risks

- Redaction reduces but cannot prove the absence of accidental disclosure. Keep
  payloads small and structured, and test filters with secret-like canaries
  before enabling public telemetry.
- A public DSN can receive spoofed or abusive events. Use Sentry spike controls,
  project quotas, and alerting, and treat event contents and report identifiers
  as untrusted input.
- Source maps and native debug information expose implementation metadata to the
  telemetry provider. Upload them only from protected CI jobs with
  least-privilege credentials and controlled retention.
- Hosted Sentry is a third-party data processor. The organisation currently uses
  the U.S. data region; record retention, access roles, and privacy disclosures
  before a public release sends events.

The detailed collection boundary and rollout gates are defined in
[ADR-0007](../architecture/decisions/0007-hosted-sentry-observability.md) and the
[observability plan](../operations/observability.md).

## Residual external-integration risks

The SimBrief preview follows no redirects, bounds streamed JSON to 2 MiB, uses a
15-second timeout, validates the account-reference shape, normalizes only
allowlisted fields, validates the canonical snapshot again in the application
service, serializes concurrent imports, and returns stable errors without URLs,
response bodies, identifiers, or plan content. Imported plans and identifiers
are session-only, excluded from plugins and Sentry, and removable from Dispatch.

The AviationWeather.gov adapter accepts at most ten normalized four-character
station identifiers, follows no redirects, bounds each streamed JSON product to
512 KiB, uses a 15-second timeout, and translates only allowlisted METAR and TAF
fields into a validated `WeatherSnapshot`. Dispatch sends no account reference,
route, fleet record, or OnAir credential. Concurrent refreshes are coalesced,
successful data is reused for ten minutes, failed attempts have a one-minute
retry floor, response bodies and URLs never cross safe errors, and weather is
excluded from plugins and Sentry.

- A translated snapshot can still be wrong because the provider, captured
  fixture, mapping, unit conversion, identifier correlation, or local clock is
  wrong. WyrmGrid exposes source and age and does not market these integrations
  for real-world operational use.
- METAR and TAF availability, provider flight categories, and raw coded text do
  not establish aircraft-specific suitability or a complete briefing. Missing
  data is displayed as missing, regional/route hazards are not yet assessed,
  and the interface makes no real-world safety claim.
- SimBrief's public latest-OFP lookup makes a Pilot ID or username capable of
  revealing operational plan data. Treat the identifier as private, minimize
  persistence, and never expose it through telemetry or plugins.
- SayIntentions places account credentials, email, active-flight identity,
  route, callsign, coordinates, and configuration in one local file. Allowlisted
  parsing reduces exposure but cannot protect that file from another process or
  a compromised local account. Its current query-parameter authentication may
  also expose the key to infrastructure outside WyrmGrid's control.
- Public VATSIM and IVAO feeds contain information about identifiable people and
  current activity. Discarding unnecessary fields reduces but does not eliminate
  the privacy implications of displaying live callsigns and positions.
- A localhost API is not inherently trustworthy. Another local process or a
  compromised host may impersonate a sidecar or simulator, observe traffic, or
  alter data where the provider has no authentication mechanism.
- Licensed navigation data may remain accessible in local caches to a user or
  process with filesystem access. Entitlement checks and application controls do
  not replace operating-system security or provider licence compliance.
- A future serverless SimBrief broker would create a public abuse and cost
  surface. It remains prohibited until Navigraph confirms the required flow and
  a separate hosting decision defines authentication, quotas, retention,
  monitoring, incident response, and shutdown controls.

Provider-specific controls and validation gates are recorded in the
[external integrations programme](../integrations/README.md).
