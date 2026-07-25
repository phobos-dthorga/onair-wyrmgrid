# Jenkins operations

Jenkins is WyrmGrid's routine CI, snapshot, and release-candidate build system.
The root `Jenkinsfile` is deliberately credential-free. A separate trusted
Pipeline reads `Jenkinsfile.release` and is the only Jenkins job allowed to
receive GitHub release authority.

Neither pipeline invokes Hoardmind, an AI model, or an optional-AI task. Jenkins
receives no OnAir credential, raw provider payload, personal flight data,
database content, signing key, or end-user device key.

## Required Jenkins capabilities

The controller requires Declarative Pipeline, Git, GitHub Branch Source, and
Credentials Binding. The pipelines use built-in checkout, stash, archive,
fingerprinting, timeout, and approval steps; they do not require Copy Artifact,
an AI plugin, or a build-cache plugin.

Configure two agents:

- `linux`: Ubuntu or Debian with Node.js 22, npm 10 or newer, Rust 1.97 with
  rustfmt and Clippy, Python 3 available as `python`, `cargo-deny`, GitHub CLI,
  and the Tauri WebKit/AppIndicator/RSVG/AppImage/Debian prerequisites already
  installed. Builds do not run `sudo` or mutate system packages.
- `windows`: Node.js 22, npm 10 or newer, Rust 1.97 with rustfmt and Clippy,
  PowerShell 7, `cargo-deny`, Visual Studio Desktop development with C++,
  WebView2, and Strawberry Perl. Run the agent as an identity that can access
  that toolchain.

The Windows helper uses `WYRMGRID_CARGO_TARGET_ROOT` when the node defines it,
then creates a bounded hash directory for each complete Jenkins job identity.
If the variable is absent, it uses the agent identity's local application-data
directory. A short dedicated value such as
`C:\JenkinsCache\WyrmGrid\cargo-target` is recommended.

## Multibranch organization job

Configure the GitHub Organization Folder to discover trusted branches and the
project's intended pull-request heads, with `Jenkinsfile` at the repository
root. Keep the source credential read-only for repository contents and webhook
or checks access. Do not make any publication credential available to the
Organization Folder or its children.

Configure GitHub webhook delivery so pushes and pull-request updates re-index
the folder. Keep a periodic organization scan as recovery for missed webhook
events, not as the primary trigger. After the first successful builds, require
the Linux and Windows Jenkins results in the `main` branch protection rules
instead of the former automatically triggered GitHub Actions jobs.

Every discovered revision runs Linux and Windows validation. A newer build of
the same branch cancels the older one. Only `main` and `codex/release-*` build
and retain unsigned snapshots:

- Linux AppImage and Debian package;
- Windows per-user NSIS setup with a clean-install smoke test;
- platform-specific `BUILD-INFO.json` and `SHA256SUMS.txt`.

Artifacts are retained for 14 days or ten artifact-bearing builds, whichever
limit Jenkins reaches first. Build metadata contains the application version,
source ref, exact commit, Jenkins build number, supported platform identifiers,
and explicit unsigned/checksums-only declarations. It excludes usernames,
hostnames, absolute paths, credentials, and raw application data.

## Trusted release job

Create a separate locked Jenkins folder containing one Pipeline job sourced
from `main` at `Jenkinsfile.release`. Do not create this job beneath the GitHub
Organization Folder. Disable ad-hoc Pipeline reconfiguration for users who are
not release maintainers.

Create a dedicated GitHub App installed only on
`phobos-dthorga/onair-wyrmgrid` with repository Contents read/write. Store it as
a Jenkins GitHub App credential named `wyrmgrid-github-release` at the locked
release-folder scope. Do not reuse the Hoardmind generated-contribution App:
that App has a different identity, policy, path boundary, and authority.

The release job accepts:

- `RELEASE_TAG`: an existing `vX.Y.Z` or supported prerelease tag;
- `EXCEPTION_REASON`: at least 20 non-whitespace characters explaining the
  exact tagged build.

The job never creates or moves a tag. It verifies the tag's four application
versions, installer identity, changelog entry, exact commit, and ancestry from
`origin/main`. It refuses to replace a published release. It repeats all Linux
and Windows gates, builds exact-tag packages, and runs the Windows clean-install
or nearest-release upgrade test.

After assembly, Jenkins archives and fingerprints the three packages,
`BUILD-INFO.json`, and `SHA256SUMS.txt`, then pauses for explicit human
approval. Only the narrowly scoped release-state query, previous-installer
download, and final publication commands receive the short-lived App token.
The publication commands create or update a draft prerelease; they never
publish it.

Jenkins releases currently provide checksums and honest build metadata, not
cryptographically signed provenance. GitHub Actions-native attestations cannot
describe a build performed on Jenkins. A future Sigstore or equivalent design
requires its own identity, key or OIDC, verification, recovery, and threat-model
decision. Do not label `BUILD-INFO.json` as an attestation.

## Manual GitHub fallback

The checked-in GitHub CI and security workflows are callable only by the release
workflow. The release workflow has only a manual trigger and accepts an existing
tag plus a meaningful exception reason. It builds Windows and Linux packages
and retains GitHub's native attestation path for an explicitly authorized
emergency rebuild.

Routine pushes, pull requests, schedules, and version tags do not start those
hosted workflows. This prevents GitHub Actions and Jenkins from racing to edit
the same draft release.

## Failure handling

- A missing or offline agent leaves the associated stage queued until its
  timeout; it does not silently downgrade platform coverage.
- A failed validation prevents snapshot and release packaging.
- A missing package, duplicate normalized filename, failed NSIS smoke test, or
  published release blocks publication.
- A failed draft upload leaves the Jenkins artifacts available for diagnosis.
  Do not hand-publish them or bypass the exact-tag job; repair the cause and run
  the same tag with a new documented reason.
- Final GitHub publication remains a separate human decision after installation,
  checksum, notes, licence, and offline-behaviour review.
