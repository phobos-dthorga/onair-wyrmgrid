# OnAir API boundary

The adapter implements the minimum convention needed for the connection probe:
JSON over HTTPS, the `oa-apikey` request header, a compatibility
`CompanyUniqueId` request header, and `GET /api/v1/company/{companyId}`. The
endpoint and API-key header were checked against OnAir's published Swagger
document on 2026-07-14. The additional company header is based on a sanitized
community observation described below; it is not part of the published Swagger
contract. The fleet endpoint has since been validated by an authenticated
outside-repository test; no credential or response capture was committed.

Rules:

- Base URLs are configurable and must use HTTPS outside tests.
- API keys are secret values, redacted from debug output, and never persisted in
  SQLite or sent to plugins.
- The first connection milestone retains a key only in the running Rust process.
  Closing WyrmGrid or selecting Disconnect forgets the session.
- Non-success responses are classified without echoing sensitive request data.
- Raw response types are private to the adapter.
- Captured fixtures must remove company IDs, registrations, personal names,
  coordinates when identifying, and all credentials.
- Company synchronization is serialized and subject to conservative quiet
  periods in the Rust application service.

No write operation is part of the supported platform until OnAir explicitly
documents one for public API use.

## Connection probe

The desktop asks for the company UUID and company-specific API key shown in the
OnAir Client under **Options > Global Settings**. As of 2026-07-14, the
still-developing OnAir Companion is not yet a compatible credential source: an
authenticated test found that its displayed API details were rejected, while
values copied from OnAir Client worked. WyrmGrid sends the key only in the
`oa-apikey` header and sends the same company UUID in both the request path and
the `CompanyUniqueId` header. A successful company envelope is translated into
WyrmGrid's `CompanySummary` domain type.

On 2026-07-14, the OnAir Discord `#web-apis` channel was inspected through the
user's authenticated browser session. The newest complete working example
located there was posted on 2026-02-08 and included both `oa-apikey` and
`CompanyUniqueId`. Older examples and the current Swagger omit the company
header. WyrmGrid therefore sends it as a narrowly scoped compatibility measure
and keeps a request-construction test so it cannot disappear accidentally. No
credential, real response, or identifying company data from Discord is stored
in this repository.

The successful Client-versus-Companion comparison also means the earlier live
rejection does not prove that `CompanyUniqueId` was required. The compatibility
header remains because the newest complete community example included it and
the request test makes that deliberate behavior visible.

OnAir Companion is expected to become the primary OnAir client. Its credential
compatibility must be revalidated when OnAir announces API parity or retires the
older Client. Once a sanitized authenticated test succeeds, this boundary and
the user guidance should move to Companion in the same change.

The committed response fixture is synthetic and Swagger-derived. It exists to
test field translation; it is not evidence of a successful live company query.

## Fleet boundary

The current Swagger document was checked again on 2026-07-14 for
`GET /api/v1/company/{companyId}/fleet`. The first Atlas slice translates only
the verified aircraft ID, identifier, nested aircraft-type display/type name,
direct coordinates, and nested current-airport identity, name, ICAO, and
coordinates.

Raw fleet fields remain optional at the adapter boundary because the Swagger
models declare properties but no required set. WyrmGrid does not manufacture
placeholder registrations, models, airports, or coordinates. Invalid WGS84
coordinates become absent; valid current-airport coordinates are used only when
direct aircraft coordinates are absent.

The translated fleet is wrapped with `on_air_fact` provenance and an observation
timestamp before leaving Rust. Its source label omits the company UUID. The
committed fleet response fixture is synthetic and Swagger-derived; an
authenticated application test on 2026-07-14 successfully mapped 17 aircraft.
That confirms the narrow fields used by this slice, not every field in the live
fleet schema.

## FBO boundary

The published Swagger document was checked on 2026-07-14 for
`GET /api/v1/company/{companyId}/fbos`. WyrmGrid currently translates only FBO
identity and optional name, plus nested airport identity, ICAO, name, and valid
WGS84 coordinates. When the nested airport omits its ID, the top-level
`AirportId` is used; an `AirportId` without nested airport details remains an
honest identity-only airport rather than acquiring invented metadata.

Capacity, fuel, workshop, pricing, and construction fields are deliberately
deferred until a concrete view needs them and a sanitized authenticated
observation confirms their live shape. The committed FBO fixture is synthetic
and Swagger-derived. It is translation evidence, not a claim that the endpoint
has been authenticated against the user's company.

## Pending-jobs boundary

The published Swagger document was checked on 2026-07-14 for
`GET /api/v1/company/{companyId}/jobs/pending`. WyrmGrid translates a deliberately
narrow subset into `JobSnapshot` version 1: mission identity and type, bounded
description, reported pay and timestamps, plus cargo and passenger legs with
airport summaries, reported weight or passenger count, distance, description,
and stable sequence.

The same Swagger also exposes `POST /api/v1/fbo_jobs/{missionId}/accept`.
WyrmGrid does not call, wrap, or expose that operation. Selecting a job in the
interface only attaches the retained read-only observation to Dispatch for
route, payload, and deadline comparison. It cannot accept, modify, dispatch, or
complete work in OnAir.

The committed pending-jobs response fixture is synthetic and Swagger-derived.
No authenticated pending-jobs response has been captured or certified, so the
feature remains a developer preview until an outside-repository authenticated
test confirms the narrow live shape.

The adapter accepts absent or `null` cargo and charter collections as empty and
accepts both RFC 3339 timestamps and the timezone-less date-time strings emitted
by some .NET serializers, interpreting the latter as UTC. Invalid optional
timestamps become unavailable facts rather than invalidating an otherwise
usable mission. These compatibility rules do not expose or retain raw JSON.

## Synchronization policy

OnAir does not currently publish a formal public API rate-limit policy in the
Swagger document or public API wiki. WyrmGrid therefore uses an intentionally
conservative, application-owned policy that can be revised if OnAir provides
official guidance:

- automatic company checks default to every 30 minutes;
- users may select Off, 15 minutes, 30 minutes, 1 hour, or 2 hours;
- the Rust boundary will never accept automatic checks more frequently than
  every 15 minutes;
- manual synchronization has a 60-second quiet period;
- only one company synchronization may be in progress for a connected session;
- requests inside either quiet period return the existing snapshot without
  contacting OnAir or displaying an error.

An accepted synchronization performs fleet, FBO, then pending-job reads sequentially under
that one gate. Each successful resource is timestamped and retained
independently. If fleet authentication is rejected or rate-limited, WyrmGrid
does not make later requests; an authentication or rate-limit failure at the
FBO step skips pending jobs. Other resource failures may still allow subsequent
snapshots to refresh. This preserves useful partial results without multiplying
user-facing synchronization controls.

The interval choice is a non-secret interface preference stored locally. The
authoritative quiet-period enforcement remains in Rust so another interface or
future plugin cannot bypass it. Automatic checks run only while WyrmGrid is open
and the OnAir session is connected.

References:

- [OnAir public API wiki](https://onaircompany.hostwiki.io/en/Public-APIs)
- [OnAir v1 Swagger document](https://server1.onair.company/swagger/docs/v1)
