# Release process

1. Update application version, release notes, compatibility notes, and relevant
   protocol or database version documentation.
2. Run all local checks and confirm CI on `main`.
3. Create and push an annotated `vX.Y.Z` tag.
4. The release workflow builds platform artifacts through Tauri and creates a
   draft prerelease.
5. Verify artifact installation, startup, license notices, checksums, and basic
   offline behavior on every supported platform.
6. Publish the GitHub release only after manual verification.

Early releases stay prereleases. Platform signing and updater signing must be
documented and tested before automatic updates or stable releases are enabled.
