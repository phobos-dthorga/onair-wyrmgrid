# Threat model

## Protected assets

- OnAir API credentials and company identifiers;
- fleet, FBO, employee, finance, job, and flight history;
- SimBrief identifiers and OFPs, SayIntentions API keys, account identity,
  communications and active-flight files, imported routes, weather caches,
  online-network activity, and simulator telemetry;
- planned simulator-synchronised recordings: voices, radio or ATC
  communications, simulator and application output, device/application labels,
  source identifiers, codec selections and provider identities, timing
  metadata, transient PCM, encoded segments, and media keys;
- Navigraph, IVAO, community-delivery, and future provider tokens or application
  credentials;
- local files and operating-system access;
- plugin trust decisions and signatures;
- proposed Aerie accounts, stable publisher identities and namespaces,
  publisher keys, moderator authority, scoped grants, validation and moderation
  evidence, repository metadata, offline and online signing keys, package
  digests, immutable targets, revocations, and hosted audit records;
- quarantined community uploads, public catalogue availability, DNS and TLS
  control, deployment and backup credentials, hosted PostgreSQL state, and
  off-site recovery material;
- proposed private-vault accounts, object metadata, encrypted `.wyrmbackup`
  objects, retention and deletion state, access history, and restore evidence;
- diagnostic events, telemetry preferences, and Sentry report identifiers;
- release source maps, native debug information, and telemetry upload credentials;
- maintainer GitHub App private keys, short-lived installation tokens, generated-
  contribution manifests, patch hashes, bot identity, and attribution records;
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
- malicious plugin, simulator-provider, audio-capture-provider, or audio-codec-
  provider manifests, executables, dependencies, and messages;
- malicious, compromised, impersonating, unlicensed, or deceptively named
  Aerie publishers and packages being accepted, signed, cached, installed, or
  mistaken for an endorsement;
- uploaded archive traversal, links, case collisions, alternate data streams,
  Unicode deception, decompression bombs, parser exploits, scanner evasion,
  worker escape, malicious metadata, and CPU, memory, disk or queue exhaustion;
- account takeover, weak recovery, confused OAuth clients, token replay, scope
  escalation, cross-account object access, moderator abuse, shared
  administration, or publisher-key and namespace theft;
- compromise or misuse of DNS, TLS, CDN, deployment, database, object-store,
  identity, mail, container-runtime, audit, backup, or repository-signing
  authority;
- rollback, freeze, mix-and-match, mirror equivocation, metadata expiry, target
  substitution, key loss, key compromise, unsafe rotation, or a desktop client
  accepting a server response without independent verification;
- public catalogue data, account records, IP addresses, upload attribution,
  private-backup metadata, encrypted backup objects, or operational history
  leaking through authorization faults, logs, support, backups or incidents;
- deletion, yanking, revocation, account closure, retention, legal hold, and
  disaster-recovery copies behaving inconsistently or being described
  misleadingly;
- path traversal and unsafe process arguments;
- malicious repository paths, links, junctions, Git metadata, file races, or
  crafted diff records causing local review tooling to read outside the
  repository, execute attacker-controlled arguments, expose sensitive content,
  misclassify a critical change, or present incomplete evidence as a pass;
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
- a microphone, full desktop, wrong device, or unrelated application being
  recorded without narrow and visible consent, including after a device change
  or automatic telemetry start;
- mixed simulator, radio, or third-party-application audio being mislabelled as
  isolated COM1/COM2 evidence, or simulator radio state being mistaken for
  captured samples;
- recorded voices, communications, device/application identities, media paths,
  or plaintext exports leaking through plugins, Sentry, diagnostics, optional
  AI, support bundles, backups, public services, or filesystem recovery;
- a selected third-party codec retaining, relaying, transforming, or otherwise
  misusing transient PCM, or being mistaken for a general plugin that receives
  no audio authority;
- unbounded audio buffers, encoder stalls, disk exhaustion, corrupt segments,
  orphaned media, clock drift, or a blocking in-process X-Plane tap degrading
  the simulator or joining discontinuous evidence;
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
  the provider-controlled local active-flight endpoint or its documented LAN
  exposure, local `flight.json` parsing, support bundles, or automatic retries;
- external writes or simulator commands occurring without explicit user intent;
- dependency or release-pipeline compromise;
- a release tag packaging untested code, a commit outside `main`, or application
  metadata whose version does not match the advertised release;
- incomplete or misleading release notes hiding a removed capability or
  compatibility break, including untrusted commit text influencing a local
  model-assisted changelog draft;
- a generated patch escaping its approved path scope, modifying governance or
  release controls, embedding credentials, replaying a stale approval, spoofing
  an assistant identity, or being mistaken for human review or merge authority;
- compromise of the maintainer GitHub App key, over-broad App installation or
  permissions, branch reuse, base-branch races, or squash merging that discards
  the durable generated-contribution provenance;
- sensitive data escaping through diagnostic payloads, attachments, replay,
  logs, traces, or crash dumps;
- network collection beginning before disclosure or continuing after the user
  withdraws an optional preference;
- forged or flooded diagnostic events consuming quota or obscuring failures;
- telemetry outages delaying application work or degrading offline behaviour;
- compromise of CI-only Sentry upload credentials;
- stale data presented as current fact;
- historical operational state mistaken for the present state;
- a plugin ignoring a historical weather window and current data being
  accepted under a historical label;
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
- historical weather requests use bounded UTC windows, exact response-time and
  layer-classification checks, separate live/historical presentation, and
  renewed grants when a bundled provider adds a network origin;
- the zero-dependency Python SDK keeps TLS certificate and hostname validation
  mandatory and, on Windows, builds server trust from the operating-system root
  store instead of a separately installed OpenSSL CA file;
- imported files, compressed feeds, navigation packages, weather geometries,
  and Bridge messages have strict size, count, nesting, numeric, path, and
  decompression limits;
- user tokens belong in the operating-system credential store; shared
  application secrets are never embedded in desktop binaries or public sites.
  The encrypted database stores only non-secret account metadata and startup
  choices, and plugins never receive credential-profile data;
- SayIntentions local active-flight data is read only after opt-in through a
  reviewed fixed-loopback or documented `flight.json` transport, parsed through
  a strict allowlist, and never persisted raw. WyrmGrid never uses a LAN address
  or hostname from the payload; the API key becomes an in-memory secret
  immediately and the documented SAPI HTTPS origin is pinned independently;
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
- simulator provider packages are bounded exact-inventory archives with
  immutable version identities, staged extraction, host-owned canonical paths,
  explicit unverified-native-code review, default-no-launch installation,
  single-step rollback, disable, and tombstoned removal. The provider manifest,
  package identity, entry point, platform, capabilities, and Bridge hello are
  validated at their respective boundaries;
- audio provider packages use a distinct bounded exact-inventory archive kind,
  explicit unverified-native-code review, staged managed versions, persisted
  selection, rollback, disable, and tombstoned removal. Installation neither
  selects nor launches a provider and grants no device permission or recording
  consent. Disabling/removing the selected provider clears selection; active
  recording blocks package mutation; and provider hello must match the installed
  ID, name, version, platform, and capability set;
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
- ordinary plugin package schema version 1 caps compressed, expanded,
  manifest, per-file, file-count, path-length, and path-depth dimensions. It
  accepts only stored or Deflate ZIP entries, exact ASCII inventory paths and
  SHA-256 content matches, and rejects traversal, links, directories,
  encryption, undeclared entries, duplicates, case collisions, Windows device
  names, identity/version disagreement, and version reuse with different
  archive bytes before create-new extraction into a fresh staging directory;
- managed plugin activation keeps immutable version trees and one explicit
  rollback pointer. Disable removes a package from supervisor discovery without
  rewriting it. Removal uses a database-reconciled tombstone and revokes saved
  host access; interrupted staging and removal have deterministic recovery;
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
- the checked-in changelog is the sole GitHub release-note source; tooling
  requires explicit feature, change, removal, and breaking-change lists and
  rejects declared breaking changes outside a new major-version line. Rebuilds
  reuse the exact tagged text rather than regenerating it;
- the Stage 1 local review inventory is deterministic, local-only, and
  dependency-free. It requires the exact Git repository root; invokes only
  fixed read-only Git argument arrays without a shell; consumes bounded,
  NUL-delimited status, diff, numstat, and tracked-mode output; rejects invalid
  UTF-8 and unsafe repository-relative paths; disables optional Git locks,
  prompts, filesystem-monitor hooks, untracked-cache updates, external diffs,
  and text conversion; bounds Git output and duration; limits each selected
  content hash to 128 MiB and 30 seconds; refuses to follow selected file links;
  resolves regular files within the repository; and performs a stat-hash-stat
  identity check while streaming SHA-256. It records only
  repository-relative metadata and hashes in a new atomically renamed directory
  beneath ignored `.wyrmgrid-local/`. A strict version-1 schema, canonical
  fixture, runtime validator, stable candidate IDs, and conservative critical
  path rules reject inconsistent counts, unknown fields, changed rule identity,
  privacy-claim drift, or malformed evidence. Missing Git or file facts remain
  explicitly unavailable and force classification review. The inventory runs
  no validation command, network request, model, cache, patch, Git mutation, or
  external action, and its path classification can escalate but never establish
  semantic safety;
- AI-assisted development tasks are optional and outside the WyrmGrid product.
  Hoardmind is the maintainer's private local assistant rather than a bundled
  component or required service. Change-impact, test-matrix, documentation-sync,
  synthetic-fixture, bounded implementation-patch, sanitized failure-triage,
  and release-curation tasks each use an explicit versioned packet and output
  contract with no tools, repository access, durable memory, or change
  authority. Diffs, logs, schemas, fixtures, documentation, and commit text
  remain untrusted evidence and sensitive provider or user data is excluded.
  Codex semantic review is reserved for high-benefit output and critical
  security, privacy, legal, credential, authorization, cryptographic,
  destructive, migration, protocol/schema, release, signing, installer,
  live-provider, or governance boundaries. Valid lower-benefit output passes
  without Codex re-analysis but still receives every applicable deterministic
  contract, schema, format, test, build, audit, path, and branch-protection gate.
  Model drafts are never chained automatically. GitHub CI performs no model call
  and receives no inference credential;
- the optional local-AI measurement wrapper uses a versioned profile and accepts
  only unauthenticated loopback Ollama or OpenAI-compatible chat origins in
  schema version 1. It pins the advertised and returned model, requires
  one-invocation approval, refuses CI, bounds packet size, checks common
  credential signatures, validates the selected boundary prompt, and rejects
  missing, duplicated, or reordered packet and response headings. The
  compatibility adapter sends no authorization header or tools and refuses a
  response without internally consistent exact token usage. Its metrics exclude
  prompt and response content; non-portable timings, RAM/VRAM observations, and
  unload state remain explicitly unreported. Plaintext packets, profiles, and
  drafts remain private temporary artifacts outside the product and receive no
  automatic retention or publication. LAN, authenticated, or hosted adapters
  remain unsupported pending a separate privacy, authentication, data-flow, and
  threat-model decision;
- the optional generated-contribution broker is a separate maintainer-side
  boundary. It refuses CI and missing one-invocation approval, binds a reviewed
  manifest and patch with SHA-256, validates an exact current base, enforces an
  identity-bound branch, rejects branch reuse, and accepts only bounded regular
  text modifications or additions inside reviewer-approved paths. It rejects
  deletions, renames, copies, mode changes, path traversal, binary data,
  credential signatures, dependency manifests, migrations, `.github`, legal,
  security, protocol/schema, release, and optional-AI governance paths. The
  broker may deterministically recount generated hunk line metadata but never
  infer or rewrite source content. The
  assistant never receives the App private key or installation token. The key
  must remain outside the repository, the App slug is pinned, and the requested
  installation token is narrowed to Contents on this repository. A `main`
  ruleset requires pull requests and grants the App no bypass. GitHub creates the
  commit without custom author or committer fields, and the App creates only an
  identity-labelled branch. After its token is discarded, the human
  maintainer's authenticated GitHub CLI opens the draft PR. The App has no Pull
  requests permission and cannot review, merge, version, tag, release,
  administer, modify workflows, access secrets, or start and rerun Actions. A
  person runs normal local gates, preserves provenance trailers in the one-task/
  one-squash merge unit, and makes every landing decision;
- generated-contribution landing is a separate human-authenticated operation.
  The local guard revalidates the reviewed manifest hash, repository, base,
  identity-bound branch, exact one-commit head, App-bot attribution and commit
  message, and clean protected merge state. It requires fresh one-invocation
  approval, uses exact-head matching without administrative bypass, supplies a
  deterministic squash message containing the input, output, metrics, bot-
  commit, PR, review, and authority trailers, and verifies that GitHub retained
  that message on the resulting merge commit before reporting success;
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
- automatic synchronization and Atlas layer choices are host-validated,
  bounded preferences in SQLCipher. Last-map restoration is off by default,
  accepts only finite camera values within map bounds, and clears the retained
  camera when disabled. An older webview interval is copied once through the
  same host validation and removed only after the encrypted save succeeds;
- per-plugin configuration is defined, validated, scheduled, and rendered by
  the host using fixed non-secret choices. A plugin cannot declare controls,
  access the configuration table, write values, or receive them through plugin
  API version 1, so this store is not a credential or covert host-data channel;
- portable backup version 1 is a complete SQLCipher export under a distinct
  user password. The host refuses overwrite, validates the encrypted manifest,
  schema and cipher integrity, re-encrypts restored data with the destination
  device key, activates only on restart, and retains the prior database for
  rollback until the replacement opens successfully;
- backup and restore passwords are bounded, cleared from Rust command-owned
  strings after the operation, never persisted, logged, sent to plugins, or
  recoverable by WyrmGrid. Restore requires a separate momentary destructive
  confirmation and runs outside the interface thread;
- a whole-database reset requires both an explicit acknowledgement and an exact
  host-validated phrase. The application service writes a versioned marker,
  restart closes every database user, and startup validates the marker before
  deleting only the active, pending, rollback, WAL, and shared-memory database
  files. An invalid marker fails closed without deletion, and the marker is
  removed last so an interrupted reset can safely resume. Portable backups,
  plugin files, diagnostics, sidecars, browser-webview local storage, the device
  key, and separately stored provider credentials are outside this operation;
  Atlas and host-owned plugin preferences inside SQLite are erased.
  Filesystem recovery and external copies remain a disclosed residual risk
  rather than a secure-erasure claim;
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
  identifiers are rejected before persistence. The authoring preview is
  advisory and cannot bypass the shared Rust validator; exact or visually
  duplicate imports are rejected. Host-owned bundled/local provenance and local
  timestamps are displayed separately from the unverified manifest author
  claim and are excluded from exports. Only local imports can be deleted, and
  deleting an active theme atomically restores the bundled default selection.
- community language packs are data-only, limited to 256 KiB, parsed and
  canonicalized in Rust, restricted to known source-catalogue keys, and checked
  for schema/source version, metadata, Fluent syntax, variable parity, message
  counts, markup delimiters, and dangerous bidirectional controls. Partial packs
  fall back per message to canonical English. Unreviewed packs cannot replace
  legal, privacy, credential, telemetry, plugin-permission, data-protection,
  destructive-action, or diagnostic namespaces and cannot load resources or
  execute code.

## Implemented simulator-audio application controls and remaining release gates

Simulator-synchronised native audio remains unavailable to users. Its
independently versioned capture and codec contracts, external capture-provider
package lifecycle, application services, debug-only Windows microphone
provider, and first-party Opus provider are implemented. The synthetic capture
provider has a separately installable reference package; native release, codec
package lifecycle, legal approval, and live-device support are not complete.
Before any native capture path ships, the remaining controls are gates rather
than claims about the current application:

- Codec choice is explicit and per source. The first-party Opus implementation
  is an out-of-process Audio Codec Provider with no privileged in-process path;
  a missing or incompatible codec fails closed. SQLite migration 20 records
  codec-provider identity/version and codec ID/media type while existing
  schema-19 tracks default to the previously implied Opus format with an
  explicit `legacy-unversioned` provider-version marker.
- Audio Capture Provider version 2 emits bounded PCM to WyrmGrid; Audio Codec
  Provider version 1 receives only selected PCM and returns bounded encoded
  packets. Neither uses Bridge protocol version 1's JSON channel. Neither
  receives media keys or storage paths, and only WyrmGrid encrypts or stores
  packets. Audio failure cannot block telemetry or simulator operation.
- Both protocols reject oversized lengths before allocation, require declared
  and actual binary sizes to match, reject unknown JSON fields, and fail closed
  on unsupported versions, non-increasing sequences, timeouts, or identity
  mismatches. Capture PCM and codec packets have independent bounds. Protocol
  errors contain neither source labels nor media bytes.
- The deterministic fake capture provider is never staged as live and uses only
  synthetic identities, timestamps, events, and PCM. Its `.wyrmaudio` package
  exercises the same offline inspection, managed storage, enable/disable,
  selection, rollback, and removal path available to other capture providers.
  The Windows microphone provider hashes raw device IDs, uses bounded
  non-blocking callback queues, and is never opened by automated tests. Neither
  establishes released or live-certified capture.
- The first-party Opus provider uses the same codec protocol intended for
  future end-user choices. Its synthetic encode/decode test establishes
  integration only, not independent conformance, quality, or security
  certification.
- Microphone and communications consent is separate, explicit, default-off,
  source-specific, and visibly active. Telemetry recording and its automation
  grant no audio authority, and full desktop audio is never implicit.
- The provider protocol keeps operating-system permission requests separate
  from capture start, preventing automatic recording from implicitly prompting
  for a microphone or communications source.
- Providers label sources as isolated, mixed output, or metadata only. COM
  telemetry never proves the provenance of an audible sample. A disappearing
  device cannot silently switch to a default source.
- Monotonic anchors, sample-frame ranges, gaps, dropouts, drift observations,
  permission delays, source changes, and interruptions remain explicit. Pause
  or disconnection never compresses or silently joins the evidence timeline.
- Audio segments use XChaCha20-Poly1305 with a fresh 24-byte random nonce and a
  versioned HKDF-SHA256 purpose key derived from the uniformly random device
  database key. Authenticated data binds the header, opaque key, session,
  track, segment index, and frame range. Create-new pending writes are synced
  and renamed before metadata completion; wrong context, digest mismatch, or
  AEAD failure rejects playback and export.
- Storage budgets, active- and pinned-session protection, retention,
  tombstoned deletion, bounded pending/orphan cleanup, and the limits of secure
  erasure are explicit. Default portable backups retain metadata while marking
  copied sessions `not_in_backup` and segments `unavailable`; deliberate packet
  exports warn that their plaintext copies are outside WyrmGrid's protection.
  Cleanup recognises only correctly sharded opaque media names and rejects a
  redirected or non-directory media root before reading, writing, or deleting.
- Project policy excludes audio content, labels, identifiers, and media paths
  from general plugins, Sentry, diagnostics, optional-AI packets, support
  bundles, and public services. A deliberately selected codec provider is the
  narrow exception for its selected source's transient PCM; community codec
  installation still requires signing, integrity, trust, resource, update,
  rollback, removal, and privacy presentation. Stable bounded status codes may
  describe failures without carrying private values.
- A first-party X-Plane in-process tap can proceed only after stability,
  licensing, signing, installation/removal, local authentication, backpressure,
  third-party-aircraft, and cross-platform review. It has no business logic and
  drops bounded samples rather than blocking X-Plane.
- The Privacy Notice, data inventory, legal versions, recording-law review,
  captured-service rules, licence bundle, user guide, and platform permission
  instructions are updated before release, not while the capability is merely
  planned.

The accepted boundary and full test matrix are recorded in
[ADR-0017](../architecture/decisions/0017-simulator-synchronised-audio-recording.md)
and [ADR-0020](../architecture/decisions/0020-out-of-process-audio-codec-providers.md),
plus the
[simulator-audio plan](../integrations/simulator-audio-recording.md).

Before stable release, the project needs a cross-platform review of its
operating-system credential-store backends and recovery messaging, signed
updates, hardened plugin supervision, abuse-case tests, and a formal security
review of every external input boundary.

The current Atlas basemap is downloaded from MapLibre's public demonstration
infrastructure after onboarding. WyrmGrid does not intentionally include OnAir
payloads in those requests, but ordinary network metadata reaches that service.
Production suitability, retention, attribution, availability limits, and a
replacement or approval decision remain stable-release requirements.

## Planned hosted-platform controls

The website, Aerie catalogue, public uploads, repository signing, and private
vault are proposals, not implemented controls. If a phase proceeds, its
exposure is blocked until that phase's controls and tests in the
[hosted-platform implementation plan](../operations/hosted-platform.md) are
complete.

- WyrmGrid Web is presentational. A separate Rust service owns publisher,
  compatibility, upload, moderation, authorization, revocation, vault, and
  installation rules. Browser, desktop, database, scanner, identity and object-
  store representations translate into stable application contracts.
- Public catalogue storage, inbound quarantine, disposable validation work,
  repository metadata, signing authority, audit evidence, and private encrypted
  backups use separate roots and least-privileged service identities. A
  catalogue compromise must not grant private-vault or offline-key access.
- Public reads and downloads remain anonymous. Publisher, moderator, desktop,
  vault and service scopes are distinct. Native authorization uses the system
  browser with Authorization Code and PKCE; short-lived audience-restricted
  tokens are retained only in the operating-system credential store when
  necessary.
- Stable Aerie publisher IDs are distinct from mutable social or email
  identities. Key enrolment, rotation, loss, compromise, recovery, revocation,
  namespace transfer and account closure are explicit, audited workflows.
  Moderators have named least-privileged accounts, phishing-resistant
  multifactor authentication and step-up checks; shared administrators are
  prohibited.
- Uploads are bounded and streamed into non-public immutable quarantine under
  application-generated identifiers. Publication requires successful
  deterministic validation and an individual moderation decision bound to the
  exact digest. Upload, validation, approval and publication are different
  states.
- Validators enforce compressed, expanded, per-file, file-count, path-depth,
  process, CPU, memory, disk, output and time ceilings. They reject traversal,
  absolute paths, links, device names, alternate data streams, case collisions,
  duplicate entries, dangerous Unicode controls, sparse tricks and install
  hooks without executing or importing package code.
- Validation workers have disposable work directories, a read-only root where
  possible, restricted system calls and processes, no runtime socket, no
  production or signing credentials, and no outbound network by default.
  Scanner updates are a separate operation. Scanner failure or a clean result
  cannot silently approve a package.
- Public targets are immutable and content-addressed. A reviewed TUF-compatible
  profile protects root, targets, snapshot and timestamp metadata with defined
  thresholds, expiry, rollback, freeze, mix-and-match, rotation, delegation and
  recovery behaviour. Offline or hardware-backed root and publication
  authority never enters the public host or its routine backups.
- The Rust desktop independently verifies trusted metadata, exact target length
  and digest, manifest, package kind, compatibility, publisher identity and
  permissions; stages and revalidates the archive; requires user approval for
  new capabilities; installs atomically; and retains a bounded rollback. No
  install script or undeclared runtime dependency download is allowed.
- If the private vault is approved, the client uploads only an authenticated,
  password-encrypted `.wyrmbackup` as an opaque object. The server never
  receives its password or plaintext. Vault authorization, roles, storage,
  logs, backup and incident access remain separate from Aerie, with tested
  quotas, generations, corruption, replay, deletion, export and versioned
  restore behaviour.
- Record-level synchronization is denied until a later ADR, privacy assessment,
  device and key model, versioned protocol and schemas, provenance policy,
  conflict and tombstone rules, deletion contract and mixed-version fixtures
  exist. Raw OnAir payloads, credentials, provider tokens, device trust, audio,
  plugin grants and local authorization decisions are denied by default.
- Only the hardened edge proxy is public. Databases, workers, metrics, storage
  control and container administration remain private. Services run as
  dedicated non-root identities with pinned artifacts, restricted capabilities,
  resource limits and no runtime socket. Administrative access uses individual
  keys through a reviewed private path.
- Request, upload, download, account, authorization and moderation endpoints
  receive separate rate, concurrency and body limits. Browser mutations use
  anti-CSRF protection and restrictive content security headers. Logs are
  bounded and exclude tokens, cookies, authorization headers, backup contents,
  archive contents, API keys and raw OnAir data.
- Encrypted off-site backups, clean-host reconstruction, independent availability
  observation, repository-metadata expiry alerts, database and object restore,
  key-loss exercises, incident playbooks, abuse and takedown processes, and
  service-shutdown export are launch gates rather than post-launch work.
- Hosted-service failure never blocks local startup, existing plugin use,
  ordinary Hoard access, local backup creation, or manual installation of an
  already verified package.

## Residual hosted-platform risks

- One dedicated server remains a common availability and compromise domain.
  Separation on the host limits credentials and paths but is not equivalent to
  independent physical security boundaries. Off-site recovery cannot provide
  uninterrupted service.
- A correctly signed package can still be malicious, vulnerable, misleading,
  incompatible, or unlawfully distributed. Signatures, scanning, schemas,
  moderation and process separation reduce different risks; none proves safety
  or rights ownership.
- A compromised online service can deny service, collect traffic metadata,
  present stale public content, or distribute malicious bytes. Independent
  desktop verification limits target substitution but cannot prevent outages,
  phishing, traffic analysis or every client-verification defect.
- Root and publication key custody moves risk to people and ceremonies. Lost
  keys can make recovery difficult; compromised keys can create apparently
  valid malicious metadata; excessive recovery authority can defeat thresholds.
- Revocation cannot instantly reach an offline client. Existing installations
  may retain vulnerable code, and expiry-based fail-closed behaviour can itself
  deny new installs during a prolonged outage.
- Identity-provider compromise, mutable external identities, recovery abuse,
  email interception, moderator coercion and support impersonation remain
  account risks even with PKCE and multifactor authentication.
- Client-side backup encryption does not conceal account identity, address,
  timing, object size, access, quota and retention metadata. Weak or forgotten
  passwords can expose or permanently lose user data; server backups and legal
  holds can delay physical deletion.
- Moderation, privacy requests, takedowns, abuse, incident response, dependency
  review and restore exercises create continuing human obligations. A free
  software stack does not remove those costs or guarantee the operator can meet
  them.
- Record-level sync would expose substantially greater metadata, compatibility,
  conflict, deletion and recovery risk than opaque backup storage and remains
  outside this proposal.

## Residual local-review automation risks

The version-1 inventory is a source snapshot, not a repository lock. A
concurrent local process with sufficient filesystem control may still replace
or mutate a path around operating-system metadata checks, and a content hash
does not prove that the source is correct, safe, reviewed, or unchanged later.
Evidence must therefore be regenerated immediately before a later bounded use,
and a stale or unavailable result cannot authorize work.

Critical path rules are deliberately conservative but cannot infer security,
privacy, legal, destructive, provider, or compatibility meaning from every
ordinary filename. A `routine-candidate` still requires human scope review; an
unknown or incomplete scope requires classification rather than a downgrade.
Repository-relative filenames and content hashes can themselves disclose
project structure or confirm known content, so ignored evidence remains private
maintainer material and must not be copied into AI packets, diagnostics,
issues, or releases without a separate bounded review.

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
- Plugin packages and publishers are not signed and updates are not
  authenticated. Ordinary package installation records the accepted archive
  digest and exact payload inventory, but the supervisor does not yet re-hash
  every installed file before each launch. First-party Python plugins use the
  same deterministic external package and managed installer as local community
  files. The first-party SimConnect sidecar likewise uses the same deterministic
  `.wyrmprovider` and managed installer as a local community provider. Audio
  provider packaging remains deferred.
- External local installation deliberately permits packages obtained without
  Aerie. Local structural validation cannot establish publisher identity, code
  intent, safety, or rights. The installer must show unverified provenance and
  requested authority clearly, require deliberate approval, record the exact
  accepted content, and never convert an unsigned package into a trusted one.
- Supporting scripts, executables, and simulator-native libraries expands the
  platform-specific attack surface. Unknown package kinds remain inert; each
  executable kind needs bounded staging, canonical paths, content inventory,
  atomic activation, rollback, disable/removal, and its own isolation and
  conformance evidence. A simulator-loaded library has the simulator's
  privileges even though it never enters the WyrmGrid desktop process.
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
[plugin protocol version 1](../plugins/protocol-v1.md) and
[ADR-0021](../architecture/decisions/0021-externally-installable-extensions.md),
with the ordinary package security and compatibility decision in
[ADR-0022](../architecture/decisions/0022-ordinary-plugin-package-format-v1.md)
and the native simulator-provider decision in
[ADR-0023](../architecture/decisions/0023-simulator-provider-package-format-v1.md).

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
- A hostile or defective plugin can deliberately trigger supervisor failures.
  WyrmGrid records only a validated manifest ID plus host-owned codes and
  messages in the bounded local log; raw plugin output and weather products are
  excluded. A shared desktop broker records command, startup, partial-sync, and
  plugin diagnostics locally before applying the application-owned reportability
  decision. The Sentry adapter receives only a low-cardinality stable failure
  code, remains consent/build/DSN gated, and the supervisor kills the failed
  runtime, limiting both disclosure and event amplification to one report per
  stopped instance.
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
publishes six UTC forecast horizons per location, capped at 504 validated
numeric samples. Route coordinates, schedule times, account references, and
plan identity remain inside the host. The RainViewer plugin receives four
host-selected zoom-one addresses plus one bounded recent-frame offset and
publishes validated RADAR and coverage-mask PNG bytes rather than remote URLs.
The host retains at most six distinct frames in memory. Both refresh in the background, preserve the last valid layer on a
provider failure, and are independently stoppable. Neither receives a plan,
OnAir fact, account reference, or credential.

Along-route analysis runs after publication in the Rust application service.
The plugin never receives plan coordinates or schedule data. The service
samples only mapped, continuous plan segments, caps the checkpoint count,
derives ETAs only from validated plan timing, and accepts forecast support only
inside fixed spatial and temporal limits. Point-level valid times, offsets,
distances, legacy current-context classification, and missing support cross to
the UI. This limits provider disclosure and prevents unresolved route gaps,
distant model points, old plugin data, or an exhausted forecast horizon from
being presented as ETA-matched corridor conditions. RADAR frame metadata is
observation-only and no cell motion or future image is inferred.

Atlas receives host-built weather projections rather than raw weather payloads
or arbitrary provider map resources. Missing reports remain unknown,
and missing coordinates remain unplotted. External RADAR frames,
simulator-selected weather mode, and ambient simulator observations are three
distinct evidence classes: none may impersonate or silently overwrite another.
RADAR publication retains bounded decoded dimensions, tile-address validation,
explicit no-data masks, in-memory retention limits, and GPU resource-loss
fallbacks. Persisted history still requires a separate licence, storage,
deletion, and backup decision. Simulator weather recording requires a versioned
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
  but do not make an unreviewed provider safe. Deliberate local package
  installation is supported with bounded validation, managed install-root
  controls, immutable updates, rollback, disable, and removal, but WyrmGrid
  must not recommend unreviewed providers until publisher identity, signing,
  authenticated updates, pre-launch tamper checks, resource limits, and
  revocation exist.
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
- plugin automatic start is a separate default-off preference available only
  with a standing grant. It is stored by the host, bound to the plugin version,
  capabilities, weather products, and network origins, evaluated only after
  current legal acknowledgement, and cleared on revocation. Each startup is
  isolated so one plugin failure cannot block the core or another plugin;
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
