# Legal and privacy readiness

This is an engineering readiness checklist, not legal advice. It keeps early
development proportionate while making the points that require professional
review explicit. The maintainer should record completion evidence in the
relevant release issue or architecture decision.

## Current foundation-stage posture

- Keep the source and documentation under MIT.
- Present the versioned Application Terms and Privacy Notice before Atlas or any
  optional diagnostic connection starts.
- Keep optional diagnostics off by default and allow later withdrawal.
- Treat current wording as a transparent engineering draft, not a substitute
  for professional advice.
- Do not enable public Sentry DSNs until the observability checklist and the
  notice's missing operational details are complete.

## Review on every release

- Diff outbound hosts, native sidecars, plugins, imported content, update
  services, support links, and authentication flows against the third-party
  service register.
- Reconfirm AviationWeather.gov's current Data API contract, request guidance,
  availability notices, and public-service attribution before declaring the
  airport-weather integration supported by a release.
- Validate the canonical message catalogue and language-pack schema, confirm
  protected namespaces remain non-overridable by community packs, and record
  whether any bundled translation is reviewed or merely community-supplied.
- Confirm the in-app document versions match the Rust application constants.
- Exercise encrypted startup, missing-device-key failure, portable backup,
  wrong-password rejection, rollback, and cross-version restore fixtures. Check
  that application-data, pending, rollback, and user-selected backup retention
  still match the Privacy Notice.
- Decide whether changes require renewed acknowledgement.
- Verify that no network request occurs before required notice or consent.
- Generate and inspect the complete direct and transitive software-licence
  bundle for packaged artifacts, including SQLCipher and OpenSSL notices.
- Re-run security, dependency, data-flow, and telemetry-redaction tests.

## Before any public Sentry telemetry

- Execute or otherwise accept the current Sentry Data Processing Addendum.
- Review Sentry's current subprocessors and international-transfer mechanism.
- Enable server-side default data scrubbing and IP-address scrubbing for both
  projects; disable public issue sharing, source scraping, stored native crash
  reports, AI processing, replay, profiling, tracing, logs, attachments, and
  feedback unless separately reviewed.
- Record event retention, access roles, two-factor-authentication requirements,
  deletion and data-subject-request handling, quota alerts, and a hard overage
  budget.
- Verify one synthetic Rust event and one synthetic interface event in the
  selected region. Confirm that secrets, company identifiers, fleet data, local
  paths, source lines, and typed user input are absent.
- Add and verify debug-information upload for each release platform without
  shipping private source maps or CI credentials.
- Update and version the Privacy Notice before embedding DSNs in public builds.

## Before the first broadly distributed prerelease

- Establish a private privacy-contact address not dependent on public issues.
- Have the photosensitivity disclosure and the separate default-on reduced-flash
  control reviewed before any lightning or comparable flashing effect ships.
- Decide and document the legal publisher identity and contact jurisdiction.
- Complete a lightweight privacy impact assessment and data inventory.
- Replace or formally approve MapLibre's public demonstration tiles, including
  attribution, acceptable-use limits, retention, and production reliability.
- Review OnAir's current API terms, brand/trademark requirements, rate limits,
  and permitted uses without claiming undocumented behaviour.
- Make local-data deletion discoverable in the application.
- Test the operating-system credential backend and backup/reinstall recovery
  instructions on every supported platform. Confirm uninstall and reinstall do
  not create a misleading promise that old encrypted data is recoverable.
- Validate installer licence notices, attribution, privacy/terms accessibility,
  and acknowledgement persistence on every supported platform.

## Before simulator-synchronised audio recording

- Update the data inventory and versioned Privacy Notice with the exact voice,
  communications, device/application metadata, external-media location,
  purpose, retention, deletion, backup, and export behaviour. Do not describe
  the planned feature as available before it ships.
- Obtain professional review of recording and consent obligations in the
  jurisdictions targeted by the release, including recordings that may contain
  another person's voice or unrelated background speech.
- Review the current rules and terms of every supported captured communications
  service, including VATSIM, IVAO, SayIntentions, or another ATC client. Local
  availability does not establish permission to record or redistribute it.
- Keep microphone and communications consent separate, explicit, and off by
  default. Confirm that legal acknowledgement, provider launch, telemetry
  recording, automatic telemetry recording, and plugin grants cannot enable
  audio.
- Verify persistent, accessible recording indication and permission recovery on
  every packaged Windows, macOS, and Linux target. Full desktop audio must not
  be selected implicitly.
- Complete the Audio Capture Provider, X-Plane in-process tap, Opus and FMOD
  licence, dependency-notice, signing, installation/removal, and third-party-
  aircraft reviews applicable to the shipped source set.
- Complete a focused privacy and security review of media-key separation,
  authenticated segment storage, size quotas, disk-full behaviour, orphan
  cleanup, deletion limitations, default backup omission, restore messaging,
  and deliberate plaintext export.
- Prove that audio, device and application labels, source identifiers, media
  paths, and voice-derived content are excluded from plugins, Sentry,
  diagnostics, optional-AI packets, support bundles, and public services.
- Exercise outside-repository live tests for every claimed simulator,
  operating-system, architecture, source class, and permission path. Do not
  generalise an MSFS or one-platform result to X-Plane or another platform.

## Obtain professional legal review when a trigger is reached

Professional review becomes proportionate before any of the following:

- a stable release or marketing that invites ordinary users to rely on the app;
- payment, subscriptions, donations tied to benefits, sponsorship, advertising,
  or other commercial activity;
- accounts, cloud sync, hosted APIs, mailing lists, or centralized user data;
- automatic updates, signing, an app store, or distribution by another entity;
- a public plugin catalogue, paid plugins, executable community plugins, or
  publisher verification;
- persistent simulator telemetry, precise location, imported personal data,
  recorded voices or communications, weather providers, financial
  integrations, or additional licensed datasets;
- users or targeted distribution in jurisdictions with materially different
  privacy, consumer, accessibility, export, or age-assurance rules;
- formation of a company, hiring contributors or contractors, accepting outside
  code under new terms, or licensing the product commercially;
- a security incident, regulator inquiry, material complaint, or uncertainty
  about whether an external provider is acting as processor or independent
  controller.

The review should cover the Terms, Privacy Notice, consumer-law limitations,
publisher structure, intellectual property and trademark posture, third-party
contracts, international transfers, plugin allocation of responsibility,
incident obligations, and required accessibility statements.

## Versioning rules

Increase the Terms version and require acknowledgement when user obligations,
liability allocation, dispute terms, plugin responsibility, payment, or
termination rights materially change.

Increase the Privacy Notice version and require review when data categories,
purposes, recipients, regions, retention, identity, legal basis, or user choices
materially change. A provider-name change with an equivalent documented data
flow may be recorded as a non-material editorial update, but the decision and
reason must be written down.
