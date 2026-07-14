# Third-party service register

This register records external services contacted by the official application.
It is a data-flow and product-governance document, not a substitute for the
third-party software licence bundle shipped with a release.

| Service                      | Trigger                                                                            | Information sent                                                                                             | Region or operator status                                                                                                 | User control                                                           | Release decision                                                                                                                                |
| ---------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| OnAir public API             | User enters credentials and connects; later manual or configured synchronization   | Company ID, API key, requested resource, normal connection metadata                                          | Independent service operated by OnAir; current API host is `server1.onair.company`                                        | Do not connect, disconnect, or disable automatic synchronization       | Retest credentials and public API contract before each supported release                                                                        |
| SimBrief latest-OFP endpoint | User enters a Pilot ID or username and explicitly selects **Import latest OFP**    | Pilot ID or username, JSON request, normal connection metadata; response contains an operational flight plan | Independent service operated by Navigraph/SimBrief; current endpoint is `www.simbrief.com`                                | Do not import, clear the session plan, or close the application        | Obtain an authenticated sanitized capture and reconfirm fields, terms, limits, and privacy posture before claiming certified live compatibility |
| MapLibre demo infrastructure | Atlas first mounts after legal onboarding                                          | Style/tile/font requests and normal connection metadata; no intentional OnAir payload                        | Public demonstration infrastructure at `demotiles.maplibre.org`; production suitability and retention are not established | The current Atlas requires its basemap; closing the app stops requests | Replace with an approved production source, self-hosted assets, or an offline-safe design before stable release                                 |
| Sentry Cloud                 | An unexpected reportable error occurs after user opt-in and build-time enablement  | Redacted error metadata described in the Privacy Notice and normal connection metadata                       | WyrmGrid organization currently uses Sentry's US data region                                                              | Optional, off by default, reversible in Privacy & Terms                | Complete the public-telemetry checklist before embedding release DSNs                                                                           |
| GitHub                       | User independently follows a support, issue, discussion, or private-reporting link | Information the user chooses to submit to GitHub                                                             | Independent website and repository host                                                                                   | Optional; not contacted by the application automatically               | Establish a dedicated privacy contact before broad distribution                                                                                 |

## Adding or changing a service

Before code introduces a new external connection, record its operator, purpose,
data categories, request trigger, countries or regions, retention, contract or
terms, subprocessors where relevant, security controls, user choice, failure
behaviour, and removal plan. Update the Privacy Notice before collection begins
and increase its version when the change is material.

Software libraries that run locally belong in the release's third-party licence
notices. They belong in this register only when they also initiate or enable an
external data flow.
