# OnAir API boundary

The adapter currently records only the minimum observed convention needed for
the authentication probe: JSON over HTTPS, the `oa-apikey` request header, and
read-only company and fleet routes. These assumptions must be checked against
the live official interface before expansion.

Rules:

- Base URLs are configurable and must use HTTPS outside tests.
- API keys are secret values, redacted from debug output, and never persisted in
  SQLite or sent to plugins.
- Non-success responses are classified without echoing sensitive request data.
- Raw response types are private to the adapter.
- Captured fixtures must remove company IDs, registrations, personal names,
  coordinates when identifying, and all credentials.
- Polling uses backoff, cache-aware refresh, and conservative request rates.

No write operation is part of the supported platform until OnAir explicitly
documents one for public API use.
