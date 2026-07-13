# Navigation data and flight-plan interchange

WyrmGrid needs a simulator-neutral route representation. SimBrief, Navigraph,
OnAir, MSFS, X-Plane, Little Navmap, and community plugins must not each create a
parallel concept of a flight plan.

## Canonical route boundary

The first route model should represent only demonstrated needs:

- origin, destination, alternates, and optional runway or procedure references;
- the original provider route text;
- ordered legs with identifier, kind, coordinates when sourced, airway or
  procedure membership, altitude constraints, and provenance;
- source AIRAC or dataset revision and transformation version;
- discontinuities, unresolved identifiers, and provider-specific segments
  without silently deleting them; and
- units and coordinate reference explicitly.

The model does not claim that identifiers from different AIRAC cycles refer to
the same fix. Reconciliation reports unresolved or conflicting legs and lets the
user choose the authoritative plan for the current action.

## Initial file interchange

1. Import and export MSFS `.pln` for the MSFS 2024 Bridge path.
2. Import and export X-Plane `.fms` when the X-Plane provider is implemented.
3. Support route-string copy and paste with explicit parse warnings.
4. Test Little Navmap handoff through formats it officially loads, beginning
   with `.pln` and `.fms` rather than coupling to its native format prematurely.
5. Add other formats such as `.lnmpln`, `.fgfp`, `.fpl`, or `.gfp` only when a
   published specification, licence, fixture set, and user need are confirmed.

Parsers run in Rust and treat every file as hostile. They enforce file-size,
leg-count, nesting, string-length, numeric, coordinate, and path limits. Import
never follows embedded external resources automatically. Export uses a new file
or an explicit overwrite confirmation and does not search simulator directories
without user authorization.

Format-specific details stay in adapters. The domain model does not expose XML,
file-format enums that have no operational meaning, simulator installation
paths, or Little Navmap implementation details.

## Navigraph Navdata

Navigraph Navdata is a valuable optional source for AIRAC-consistent fixes,
airways, and procedures. It requires developer approval and access tokens. The
current dataset available to an end user depends on their Navigraph subscription;
the API can return an older default package to an unsubscribed user.

Before implementation WyrmGrid must:

- apply to Navigraph with the desktop platform, purpose, redirect URI, requested
  data, and authentication flow;
- use Authorization Code with PKCE or the approved native flow and keep refresh
  tokens in the operating-system credential store;
- keep client secrets out of the desktop binary and repository;
- record package ID, AIRAC cycle, revision, status, hash, retrieval time, and
  entitlement class without exposing signed download URLs;
- verify package hashes and impose compressed and expanded size limits;
- isolate licensed data from plugins, exports, logs, Sentry, and users without
  the corresponding entitlement;
- follow provider update and retention terms; and
- define behaviour for expired tokens, subscription changes, an outdated
  package, and offline cached use before persisting data.

Navigraph currently states that Charts API access is granted only to
applications running in-process inside a flight simulator. WyrmGrid is an
external desktop application, so embedded Navigraph charts are not an assumed
capability. Prefer opening an approved Navigraph destination or using the user's
Navigraph application unless Navigraph explicitly approves another design.

## OurAirports offline baseline

OurAirports publishes nightly public-domain CSV datasets for airports, runways,
frequencies, navaids, countries, and regions. WyrmGrid may use a pinned snapshot
as an offline reference and identifier crosswalk.

- Keep dataset version, retrieval time, file hash, and source provenance.
- Import through an append-only database migration or a separately versioned
  reference-data loader; do not hide a mutable network download inside a shipped
  migration.
- Never overwrite an OnAir airport fact with an OurAirports value. Reconcile by
  identifier and show conflicts explicitly.
- Treat coordinates, runway details, and frequencies as non-authoritative
  community data with visible age.
- Update on a deliberate reference-data cadence, not merely because the upstream
  file changes nightly.

## AIRAC and identifier policy

- ICAO, IATA, local airport identifiers, provider IDs, callsigns, and simulator
  object IDs remain distinct types until a tested mapping exists.
- Plans display their AIRAC or `unknown`; WyrmGrid does not infer the current
  cycle from today's date for imported data.
- Export warns when source and target datasets differ or a leg cannot be mapped.
- A provider's navdata entitlement never grants a plugin the right to read or
  redistribute that dataset.

## Required validation

- sanitized round-trip and one-way fixtures for every supported format version;
- golden route-order, unit, coordinate, procedure, and unresolved-leg tests;
- path traversal, entity expansion, archive bomb, oversized plan, invalid
  encoding, and non-finite-number tests;
- AIRAC mismatch and identifier-collision tests;
- Navigraph authentication, entitlement, package hash, and token-redaction tests
  before enabling that adapter; and
- OurAirports import reproducibility, source-hash, and conflicting-airport tests.

## References

- [Navigraph developer access requirements](https://developers.navigraph.com/docs/request-access)
- [Navigraph Navigation Data API](https://developers.navigraph.com/docs/navigation-data/api-overview)
- [Navigraph Charts API introduction](https://developers.navigraph.com/docs/charts/introduction)
- [OurAirports open data](https://ourairports.com/data/)
- [OurAirports data dictionary](https://ourairports.com/help/data-dictionary.html)
- [Little Navmap supported flight-plan inputs](https://www.littlenavmap.org/manuals/littlenavmap/release/latest/en/COMMANDLINE.html)
