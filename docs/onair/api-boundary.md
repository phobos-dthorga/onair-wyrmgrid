# OnAir API boundary

The adapter implements the minimum convention needed for the connection probe:
JSON over HTTPS, the `oa-apikey` request header, a compatibility
`CompanyUniqueId` request header, and `GET /api/v1/company/{companyId}`. The
endpoint and API-key header were checked against OnAir's published Swagger
document on 2026-07-14. The additional company header is based on a sanitized
community observation described below; it is not part of the published Swagger
contract. Fleet translation remains a later increment and must be verified
independently before it is claimed to work.

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
- Polling uses backoff, cache-aware refresh, and conservative request rates.

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
committed fleet response fixture is synthetic and Swagger-derived; a real,
sanitized authenticated response is still required before claiming complete
live-schema coverage.

References:

- [OnAir public API wiki](https://onaircompany.hostwiki.io/en/Public-APIs)
- [OnAir v1 Swagger document](https://server1.onair.company/swagger/docs/v1)
