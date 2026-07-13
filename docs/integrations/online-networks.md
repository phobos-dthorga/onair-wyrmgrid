# VATSIM and IVAO integration plan

VATSIM and IVAO are optional online-network providers for Atlas and dispatch
context. Their public activity is an `external_fact`, not evidence of OnAir
demand, real-world traffic, or a user's identity.

The initial integration reads public feeds without asking the user to connect a
network account. Authenticated identity, flight-plan filing, or other writes are
separate future capabilities.

SayIntentions.AI exposes its own account-gated VATSIM view through SAPI. That
does not replace WyrmGrid's direct public VATSIM adapter. If adopted later, it is
labelled as a SayIntentions-derived snapshot and is never merged silently with
the direct feed.

## Shared product capabilities

- Atlas layers for active pilots, controllers, ATIS, and coverage where the
  provider supplies enough validated data.
- Airport and route context showing whether relevant ATC positions are online.
- Network-event discovery and optional planning hints.
- Matching the user's own simulator session to an online callsign only after the
  user explicitly supplies or confirms that callsign.
- A visible provider selector; VATSIM and IVAO observations are never merged into
  an invented combined network.

## VATSIM public adapter

The official VATSIM Data API exposes a public live feed without authentication.
The feed is regenerated every 15 seconds and contains pilots, controllers, ATIS,
prefiles, callsigns, positions, flight plans, timestamps, and personal fields.
The Events API is also public.

WyrmGrid will:

- fetch in Rust and poll no more frequently than every 30 seconds while the
  layer is visible, unless a later measured need justifies the documented
  15-second floor;
- cache events on a much longer interval appropriate to their schedule;
- discard names, CIDs, server addresses, ratings, remarks, and other unnecessary
  fields during translation;
- retain callsign, aircraft category, position, altitude, heading, groundspeed,
  relevant airport identifiers, controller position, frequency, and provider
  timestamps only where the implemented view needs them;
- treat route, remarks, and ATIS text as untrusted free-form input; and
- avoid long-term storage of the global activity feed.

VATSIM Connect/OAuth is not required for the public overlay and will not be
introduced merely to identify the user. Any later authenticated feature needs
scope review, operating-system token storage, redirect validation, and a
provider-approved application registration.

## IVAO public adapter

IVAO's public Whazzup feed is available without approval as gzip-compressed
JSON. IVAO prohibits fetching it more frequently than once per 15 seconds and
may ban abusive sources.

WyrmGrid will:

- poll no more frequently than every 30 seconds while the IVAO layer is visible;
- impose compressed and decompressed size limits before parsing;
- translate the same minimal operational field set used for VATSIM where the
  provider semantics genuinely match;
- preserve IVAO-specific meanings rather than forcing them into VATSIM enums;
- discard personal identifiers and free-form content not required by the view;
  and
- keep IVAO's broader Data API, OAuth, user records, flight-plan writes, and API
  keys outside the first slice.

Private IVAO APIs and Login API access have their own approval and secret-handling
rules. A public Whazzup overlay does not authorize WyrmGrid to use them.

## Plugin and privacy boundary

The host may later expose sanitized, bounded network snapshots through separate
`vatsim_activity_read` and `ivao_activity_read` capabilities or a versioned
provider-neutral capability with explicit source fields. This is a plugin
protocol change and requires fixtures, validation, documentation, and a
compatibility decision before implementation.

Plugins do not receive raw global feeds, personal names, network member IDs,
OAuth tokens, API keys, or an endpoint proxy. Map contributions remain
declarative and subject to host density and update limits.

Network-derived fields, callsigns, flight plans, coordinates, and free-form
text are excluded from Sentry. Support diagnostics use only provider name,
operation code, response category, and bounded counts.

## Required validation

- provider-specific sanitized fixtures with all personal values synthetic;
- schema-version, missing-section, partial-record, and unknown-enum tests;
- compressed-input, maximum-record, malformed-coordinate, and free-form-text
  abuse tests;
- polling, visibility suspension, offline, backoff, and stale-overlay tests;
- antimeridian, map clustering, event storm, and memory-bound tests; and
- a privacy review of every retained or displayed field before enabling each
  layer publicly.

## References

- [VATSIM API overview](https://vatsim.dev/services/apis/)
- [VATSIM live network feed](https://vatsim.dev/api/data-api/get-network-data/)
- [VATSIM Events API](https://vatsim.dev/api/events-api/list-all-events/)
- [IVAO API documentation](https://api.ivao.aero/docs)
- [IVAO API and Whazzup rules](https://wiki.ivao.aero/en/home/ivao/regulations)
- [Retrieving IVAO Whazzup v2](https://wiki.ivao.aero/en/home/devops/api/whazuup/how-to-retrieve-v2)
