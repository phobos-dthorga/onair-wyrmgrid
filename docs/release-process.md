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
