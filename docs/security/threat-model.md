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
- live debugger state, watch expressions, memory views, and debug screenshots;
- legal-document versions, acknowledgement records, and privacy preferences;
- the SQLCipher device key, portable-backup passwords and files, pending
  restores, rollback databases, and backup-format metadata;
- selected language, imported community language-pack content, translator
  metadata, and the integrity of security-sensitive interface wording;
- accepted flight-operation identities and revisions, sanitized plans, selected
  jobs, and aggregate passenger or freight manifests;
- update integrity and release artifacts.

## Primary threats

- credential disclosure through logs, errors, telemetry, storage, or plugins;
- malicious plugin or simulator-provider manifests, executables, dependencies,
  and messages;
- path traversal and unsafe process arguments;
- unbounded messages, event storms, hangs, and resource exhaustion;
- hostile API payloads, imported files, map styles, and URLs;
- imported themes concealing security text, counterfeiting controls, loading
  remote resources, or exhausting local storage;
- malicious or stale translations mislabelling credentials, permissions,
  Security Centre authority, destructive actions, diagnostics, provenance, or
  legal disclosures;
- malicious or oversized OFPs, flight-plan files, compressed feeds, navigation
  packages, weather geometries, and simulator messages;
- spoofed localhost simulator services, sidecars, callbacks, or OAuth redirect
  state;
- provider schema drift, identifier collisions, AIRAC mismatch, and unit or
  timestamp confusion producing plausible but incorrect plans;
- GPU weather effects implying invented precipitation, lightning, cloud,
  location, precision, or validity beyond the sourced product;
- flashing weather or warning effects causing photosensitive harm, being
  enabled through ambiguous consent, or ignoring reduced-motion/reduced-flash
  preferences;
- personal network data, routes, coordinates, callsigns, or free-form content
  leaking through persistence, plugins, support output, or diagnostics;
- staff names, airport presence, availability, or qualifications leaking through
  raw provider retention, plugins, diagnostics, exports, or misleading UI labels;
- accepted plans, selected jobs, aggregate passenger/freight facts, or revision
  history leaking through diagnostics, plugins, public map requests, backups,
  screenshots, or support material;
- a provider refresh or interface action silently replacing accepted operation
  evidence, or a malformed stored manifest diverging from its retained job;
- undocumented staff avatar references being converted into attacker-controlled
  URLs, remote tracking requests, oversized media, or misleading portraits;
- embedded desktop application secrets being extracted and abused;
- a SayIntentions key leaking through query URLs, redirects, HTTP diagnostics,
  local `flight.json` parsing, support bundles, or automatic retries;
- external writes or simulator commands occurring without explicit user intent;
- dependency or release-pipeline compromise;
- a release tag packaging untested code, a commit outside `main`, or application
  metadata whose version does not match the advertised release;
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
- offline theft of `wyrmgrid.db`, loss or replacement of its device credential,
  weak backup passwords, damaged backups, unintended cloud synchronisation, or
  a malicious restore replacing trusted local history;
- request storms or costly generated actions against SimBrief, SayIntentions,
  weather, VATSIM, IVAO, or navigation providers;

## Initial controls

- secrets wrapped and redacted at the adapter boundary;
- API keys move from the password field into a Rust `SecretString`. Session-only
  keys are dropped on Disconnect or application exit; explicitly remembered
  OnAir keys are stored only in a versioned operating-system credential entry;
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
  application secrets are never embedded in desktop binaries or public sites.
  The encrypted database stores only non-secret account metadata and startup
  choices, and plugins never receive credential-profile data;
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
- the Staff adapter discards salary, birth date, weight, fatigue, happiness,
  avatar URLs and artwork, raw JSON, and other unused employee fields before the
  roster crosses into application services. A bounded avatar image-name may be
  retained only as opaque source evidence and is never interpreted as a path or
  URL. The bounded translated roster is encrypted in
  Hoard, remains unavailable to plugins and Sentry, and displays undocumented
  provider enums as codes rather than invented role or status labels;
- responsive surfaces are bounded to a small transform and non-informational
  glow, can be disabled in Settings, ignore touch and pen movement, preserve
  static keyboard focus cues, and defer to the operating system's reduced-motion
  preference. No data, warning, consent, or authorization state relies on motion;
- Bridge sidecars use a 64 KiB length-prefixed JSON ceiling, independent
  monotonic sequences, a three-second identity/version handshake, explicit
  capabilities, validated provider and simulator provenance, supervised
  lifecycle, and local-only provider connections;
- provider executable paths are host-owned, entry-point names are manifest
  validated, child environments are scrubbed, and only absolute approved
  SimConnect SDK/client paths cross into the first-party provider;
- only one selected telemetry provider is active in protocol version 1; the host
  neither merges values nor silently falls back from SimConnect to FSUIPC;
- simulator recording is explicit and local; automation is separately opt-in
  and off by default, only validated translated fields are persisted, active
  sessions resist deletion, completed sessions have a user-visible bounded
  retention period, and recorded history is not covered by the live
  `simulator_telemetry_read` plugin permission;
- provider sequence or observation-time discontinuities become graph gaps, an
  aircraft identity change interrupts the session, and abandoned active rows
  are marked interrupted on the next application start;
- automatic take-off requires two direct increasing airborne facts; landing
  settlement requires continuous direct on-ground facts and is reset by pause
  or telemetry gaps. Automatic stop applies only to an automatically created
  session, so lifecycle evidence cannot stop a manual recording;
- a recording may retain only the validated sanitized SimBrief domain snapshot
  in force, never the entered account reference or raw OFP. Planned and recorded
  values stay separately labelled, missing comparisons stay unavailable, and
  no climb, fuel, or route values are inferred. Debrief reads reject more than
  250,000 source samples and reduce each graph to at most 1,200 points. Omitted
  source gaps propagate to represented points, and missing plan/position facts
  split route geometry;
- the current Dispatch-to-Atlas projection is built in the Rust application
  from the same validated session-only plan. Stable point IDs contain only
  bounded point kind, sequence, and normalized labels. Missing coordinates
  remain inspectable but unplotted, alternates are not joined to the route, and
  the projection is not exposed through existing plugin capabilities,
  diagnostics, Sentry, or public tile requests;
- simulator plan loading and every other external mutation require a distinct
  negotiated capability and explicit user action;
- deny-by-default plugin capabilities persisted separately from manifests; the
  current runtime starts only after every requested capability is approved and
  implements sanitized fleet and simulator reads, data-only Atlas publication,
  and bounded weather requests to declared HTTPS origins. Once grants are
  consumed by one privileged launch, session
  grants remain only in process memory, and standing grants alone persist;
- plugin directories, manifests, and entry points are bounded and canonicalized;
  symlinked folders/files, path escape, malformed metadata, and unsupported
  runtimes or capabilities are rejected;
- plugin protocol v1 uses a 1 MiB length-prefixed JSON ceiling, independent
  monotonic sequences, a three-second identity/version handshake, bounded text
  and WGS84 validation, at most 16 map layers per plugin, at most 10,000 points
  per map layer, and separate station, model-grid, PNG tile, decoded-byte, and
  request-correlation bounds for weather products;
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
- immutable commit pins for workflow dependencies; reusable CI and security
  gates run against the exact release tag before packaging, release tags must
  identify a commit on `main`, and every checked-in application version must
  equal the tag version;
- platform build jobs are read-only and stage packages internally; one final job
  with narrowly scoped write and identity-token permissions generates SHA-256
  checksums and GitHub build-provenance attestations before creating a draft;
- the Windows release runner silently installs the NSIS output and verifies that
  the desktop application and expected SimConnect sidecar were packaged;
- the Windows installer identity and per-user scope are regression-tested,
  downgrades are disabled, and releases after the first must install over the
  closest older published setup without altering application data;
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
- the local diagnostic log accepts only timestamps, severity, stable codes,
  operation names, and bounded application-owned English messages; it rotates
  at 200 entries, is user-clearable, stays outside language packs, and is never
  uploaded or attached automatically;
- checked-in debugger configuration contains no credentials, credential files,
  verbose provider logging, or automatic memory capture; debugger watches,
  consoles, dumps, and screenshots are treated as sensitive local artifacts;
- telemetry is off by default; first-run onboarding prevents Atlas from mounting
  before the current Terms and Privacy Notice are acknowledged, and stale
  document versions suppress both Rust and interface diagnostics until review;
- first-run Terms disclose future flashing weather and warning effects;
  reduced-flash presentation remains an independent default-on safety control,
  and stronger effects require a separate explicit confirmation;
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
- flight-operation schema 1 is created only by an explicit user action and
  stores a sanitized plan, optional validated OnAir job observation, and a
  deterministic aggregate per-leg manifest in SQLCipher. Attached jobs retain
  their originating company identity so a later account change cannot silently
  reattribute them. Missing passenger or freight fields remain explicit gaps.
  Domain validation recomputes the manifest from its retained job evidence and
  rejects any divergence;
- operation changes are append-only and user-reviewed. A changed plan, selected
  job, or same-identity job fact produces a context-change notice instead of
  mutating the accepted revision. The webview never supplies operation IDs,
  revision numbers, timestamps, manifest values, or persistence SQL;
- operation data is not exposed to current plugin capabilities, Sentry,
  diagnostics, or public Atlas tile requests. Migration 13 stores one active
  pointer and immutable revision rows; it contains no provider credential or raw
  response;
- Hoard stores stable domain snapshots rather than raw API payloads, never stores
  credentials, applies bounded retention, and visibly distinguishes live,
  cached, offline, preview, and memory-only data.
- persistent SQLite storage is encrypted with SQLCipher using a random 256-bit
  key held separately by the operating-system credential service. Existing
  encrypted or recovery state without the exact key fails closed; startup does
  not create a replacement key, retry plaintext, or hide failure behind a new
  memory-only store;
- SQLCipher device keys, portable-backup passwords, and remembered user
  credentials are generated or entered per installation and never become CI
  secrets. CI uses only disposable test keys; future code-signing, updater, and
  notarisation credentials are distinct release-authentication secrets confined
  to protected signing jobs;
- remembered OnAir persistence is validate-first. The host writes the OS secret
  before metadata and attempts rollback if metadata fails; missing entries and
  unavailable credential stores fail closed without SQLite, browser-storage, or
  logging fallback. Disconnect and Forget remain distinct operations;
- automatic OnAir connection is a separate default-off preference evaluated
  only after current legal acknowledgement. SimBrief Pilot IDs and usernames
  are stored only as explicitly selected encrypted metadata and never treated
  as passwords or authorization tokens;
- portable backup version 1 is a complete SQLCipher export under a distinct
  user password. The host refuses overwrite, validates the encrypted manifest,
  schema and cipher integrity, re-encrypts restored data with the destination
  device key, activates only on restart, and retains the prior database for
  rollback until the replacement opens successfully;
- backup and restore passwords are bounded, cleared from Rust command-owned
  strings after the operation, never persisted, logged, sent to plugins, or
  recoverable by WyrmGrid. Restore requires a separate momentary destructive
  confirmation and runs outside the interface thread;
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
  legal, privacy, credential, telemetry, plugin-permission, data-protection,
  destructive-action, or diagnostic namespaces and cannot load resources or
  execute code.

Before stable release, the project needs a cross-platform review of its
operating-system credential-store backends and recovery messaging, signed
updates, hardened plugin supervision, abuse-case tests, and a formal security
review of every external input boundary.

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
compromised host. The frontend necessarily holds a newly entered value briefly
before invoking Rust and clears it after success, disconnect, or dialog close.
Remembered storage protects against casual database-file disclosure, not a
malicious process or logged-in account able to use Windows Credential Manager.

The SQLCipher database and Windows credential store are not one transactional
system. WyrmGrid rolls back a new secret when metadata saving fails, but a host
crash or OS-store failure can leave metadata without a key or an orphaned
versioned entry. Metadata without a key is shown as unusable and requires
replacement or an explicit Forget; it never causes plaintext fallback. A
portable restore intentionally recreates metadata without transferring the
OnAir key, so cross-device recovery requires re-entry.

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

The local SQLCipher database contains company identifiers, company names,
aircraft and FBO details, locations, observation history, accepted
flight-operation plans, selected jobs, aggregate manifests, and other local state.
At-rest encryption reduces exposure from a copied file but does not protect
against a process or logged-in account that can retrieve the device key, memory
inspection, crash dumps, screenshots, or deliberate export. A user must still
omit `wyrmgrid.db`, rollback/pending files, and portable backups from support
reports unless they intentionally mean to disclose their contents.

Retention limits intraday growth but deliberately preserves one daily
historical record, so sensitive operational history remains inside encrypted
storage until the user deletes the database or a future data-management feature
removes it. The first flight-operation foundation likewise has no individual
archive or deletion control, so accepted revisions follow that database-level
retention boundary. Portable backups are complete, user-controlled copies: WyrmGrid
cannot rotate, revoke, erase, or recover their passwords. Filesystem snapshots,
cloud services, and deleted-file recovery may retain both databases and backups.
Loss of the operating-system credential entry without a usable portable backup
makes local data unrecoverable by design.

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
response bodies, identifiers, or plan content. The entered account reference is
session-only and excluded from plugins, persistence, and Sentry. A sanitized
plan snapshot may be retained only when associated with a local recording; it
is encrypted, deleted with that recording, and remains excluded from plugins
and Sentry. Clearing Dispatch prevents future association without rewriting an
existing recording's historical context.

The AviationWeather.gov provider plugin accepts at most ten normalized four-character
station identifiers, follows no redirects, bounds each streamed JSON product to
512 KiB, uses a 15-second timeout, and translates only allowlisted METAR and TAF
fields into a validated `WeatherSnapshot`. Dispatch sends no account reference,
route, fleet record, or OnAir credential. Concurrent refreshes are coalesced,
successful data is reused for ten minutes, failed attempts have a one-minute
retry floor, and response bodies and URLs never cross safe errors. Only the
approved provider plugin receives the station identifiers; translated weather
remains excluded from other plugins and Sentry.

The Open-Meteo plugin receives only an 84-point host-selected global grid and
publishes bounded numeric samples. The RainViewer plugin receives four
host-selected zoom-one addresses and publishes validated PNG bytes rather than
remote URLs. Both refresh in the background, preserve the last valid layer on a
provider failure, and are independently stoppable. Neither receives a plan,
OnAir fact, account reference, or credential.

Atlas receives host-built weather projections rather than raw weather payloads
or arbitrary provider map resources. Missing reports remain unknown,
and missing coordinates remain unplotted. Future external radar frames,
simulator-selected weather mode, and ambient simulator observations are three
distinct evidence classes: none may impersonate or silently overwrite another.
Radar adoption requires approved access/licensing, bounded decoded dimensions,
projection and geometry validation, no-data masks, cache/retention limits, and
GPU resource-loss fallbacks. Simulator weather recording requires a versioned
Bridge compatibility decision and must not infer Live Weather from resemblance
to an external report.

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
- A provider is a native executable with the ambient rights of the user's
  account. Process separation and protocol validation contain malformed output
  but do not make an unreviewed provider safe. Community provider loading stays
  disabled until publisher identity, signing, tamper checks, install-root
  controls, resource limits, and safe update/rollback exist.
- Provider auto-start is user-controlled, default-off, and limited to the
  persisted ID of an installed manifest-validated provider. It cannot accept an
  arbitrary executable path from the frontend or a community plugin.
- A failed provider does not enter an automatic crash loop; the failure remains
  visible for an explicit restart. Connected snapshots are withheld after the
  bounded freshness window so stale aircraft state is not presented as live.
- Local simulator recordings reveal operational timing and aircraft behaviour.
  SQLCipher protects a database copied while closed, but deletion may remain
  recoverable in filesystem or portable backups. A whole-flight debrief and
  Atlas overlay can expose precise routes, altitude, fuel weight, and attitude
  while WyrmGrid is open. The host bounds the interface projection, preserves
  gaps, and excludes it from plugins, Sentry, and public tile requests, but
  screenshots and deliberate exports remain disclosures. Users must omit
  databases and backups from support bundles unless they intend to share
  recordings.
- JSON and CSV recording exports are deliberate plaintext disclosures outside
  SQLCipher. Pinning protects against automatic pruning only; explicit deletion
  and copies made by the user remain outside WyrmGrid's recovery control.
- Licensed navigation data may remain accessible in local caches to a user or
  process with filesystem access. Entitlement checks and application controls do
  not replace operating-system security or provider licence compliance.
- A future serverless SimBrief broker would create a public abuse and cost
  surface. It remains prohibited until Navigraph confirms the required flow and
  a separate hosting decision defines authentication, quotas, retention,
  monitoring, incident response, and shutdown controls.

## Core authorization controls

- Legal acknowledgement, feature preferences, capability grants, and momentary
  confirmations are distinct policy decisions and cannot authorize one another.
- Grants are denied by default and bound to actor kind, actor ID, exact
  capabilities, a scope revision, and an explicit lifetime. Plugin version or
  permission-set changes require a fresh review.
- `Once` is consumed at the privileged launch boundary, `Session` exists only
  in the shared in-memory authorization runtime, and only `Standing` is written
  to encrypted storage. A new process therefore starts without temporary
  authority.
- Feature services enforce decisions through the Rust authorization module;
  Tauri commands and Svelte controls are not trusted enforcement boundaries.
- Standing grant and revoke events append bounded symbolic metadata to the
  encrypted local authorization audit trail. Temporary decisions are visible
  only in the current session's bounded in-memory trail. Neither contains API
  keys, raw provider payloads, or plugin output.
- Revocation stops an active plugin before its capabilities are removed.
- The Security Centre reads its grouped, validated status through the Rust
  service, shows at most 100 recent decisions, and routes plugin revocation
  through supervised Forge shutdown. Svelte and the Tauri command do not decide
  whether an operation is authorized.
- Security Centre labels and capability descriptions use a protected canonical
  localization namespace that unreviewed community packs cannot replace.
- The migration-4 preview grant table remains for append-only migration
  integrity but is no longer authoritative after migration 9.

Residual risk: SQLCipher does not protect authorization history from code or a
logged-in account that can use the device key, a malicious plugin may retain
facts it received before revocation, and process separation is not a complete
operating-system sandbox. Users should revoke unneeded grants, review permission
changes after updates, and run only trusted code.

Provider-specific controls and validation gates are recorded in the
[external integrations programme](../integrations/README.md).
