# Release process

1. Update application version, release notes, compatibility notes, and relevant
   protocol or database version documentation.
2. Run all local checks and confirm CI on `main`.
3. For a minor or major release, create and push an annotated `vX.Y.0` tag.
4. The release workflow builds platform artifacts through Tauri and creates a
   draft prerelease. Patch tags do not trigger installer builds.
5. Verify artifact installation, startup, license notices, checksums, and basic
   offline behavior on every supported platform.
6. Publish the GitHub release only after manual verification.

Early releases stay prereleases. Platform signing and updater signing must be
documented and tested before automatic updates or stable releases are enabled.

## Installer build policy

Routine commits and pull requests compile-check the Windows desktop target but
do not assemble installers. Automatic multi-platform installer builds are
reserved for semantic minor releases (`vX.Y.0`), which includes major releases
(`vX.0.0`). This avoids repeatedly compiling and packaging three platforms for
patch-level development.

The release workflow retains a manual dispatch path for a concrete exception,
such as validating a packaging-system change, signing configuration, updater
behaviour, or an urgent platform-specific release. A manual run requires a
semantic version and a written reason. Installer output remains a CI artifact;
local builds are verification only and are never hand-published.

## Diagnostic artifacts

When Sentry integration is enabled for a release:

- Rust and SvelteKit events use the same canonical release identifier,
  `onair-wyrmgrid@<semver>`, with platform or channel represented separately.
- Browser source maps and native debug information are generated and uploaded by
  the release workflow before packaging or stripping. Source maps are not shipped
  in the public application bundle solely for Sentry's benefit.
- `SENTRY_AUTH_TOKEN` and equivalent organization or project credentials exist
  only as protected CI secrets. Routine pull-request builds do not receive them
  and do not create Sentry releases.
- A failed diagnostic-artifact upload leaves the release as a draft until the
  failure is repaired or a concrete exception is recorded. It does not justify
  rebuilding binaries by hand.
- Before the first stable release on a supported platform, a sanitized synthetic
  failure must demonstrate symbolicated Rust and Svelte stack traces against the
  exact CI-built artifacts.

See the [observability plan](operations/observability.md) for the phased rollout
and [ADR-0007](architecture/decisions/0007-hosted-sentry-observability.md) for the
hosting and privacy decision.

## Sidecars and provider assets

When a release includes WyrmGrid Bridge or another provider adapter:

- version the application, Bridge protocol, plugin protocol, database
  migrations, and external fixture/schema compatibility independently;
- build each platform sidecar in CI from the same tagged commit, sign it under
  the platform policy when signing is enabled, record its checksum, and package
  it through Tauri rather than copying a local executable into an installer;
- include only sidecars supported on that target platform and verify that the
  desktop starts, reports a clear unavailable state, and exits cleanly when no
  simulator is installed;
- smoke-test protocol handshake, version mismatch, supervised shutdown, and
  tampered-sidecar rejection before promotion;
- review redistribution terms for SimConnect, FSUIPC, simulator SDK, reference
  data, and every bundled native dependency before packaging; and
- never bundle provider application secrets, user tokens, private OFPs,
  SayIntentions keys or `flight.json` captures, downloaded Navigraph packages,
  live network captures, or authenticated test data in release artifacts.

Adding or changing a Bridge message or provider snapshot requires fixtures,
validation tests, documentation, and an explicit compatibility decision. A
working local simulator test is not a substitute for those release artifacts.
