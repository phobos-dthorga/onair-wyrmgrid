# Development environment

## Windows

Install Microsoft C++ Build Tools with Desktop development with C++, WebView2,
Rust through rustup, Node.js 22, npm 10 or later, and a complete Perl
distribution such as Strawberry Perl. The repository pins the Rust toolchain
in `rust-toolchain.toml`. Perl is a build-time requirement for vendored OpenSSL;
it is not installed or invoked by a released WyrmGrid application.

The repository's synchronized path can exceed limits encountered by OpenSSL's
MSVC build and debug-symbol tooling. Keep Cargo output in a short local cache
while building WyrmGrid from a long checkout:

```powershell
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\WyrmGrid\cargo-target"
$env:OPENSSL_SRC_PERL = "C:\Strawberry\perl\bin\perl.exe"
```

Run Rust compilation from a Visual Studio developer terminal, or initialise
the matching Developer PowerShell before invoking Cargo. The short target path
is local build output only and must never be committed.

The repository includes a Windows launcher that performs those steps, verifies
Visual Studio and Strawberry Perl, and then starts the Tauri development app:

```powershell
.\scripts\dev-windows.ps1
```

If the local PowerShell execution policy prevents direct script invocation,
use a process-scoped bypass without changing the machine policy:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\dev-windows.ps1
```

The script accepts `-PerlPath` and `-CargoTargetDir` overrides when a developer
uses non-standard locations. Run it with `-ValidateOnly` to verify the toolchain
and environment without launching WyrmGrid.

```powershell
npm ci
cargo test --workspace
npm test --workspace @wyrmgrid/desktop
npm run check
npm run dev
```

See the [testing strategy](testing.md) for test placement, required cases, CI
gates, and the safe scope for automated test-writing agents.

For breakpoint-based investigation, use the checked-in VS Code configurations
described in the [debugging guide](debugging.md). They support launching or
attaching to the Tauri backend, focused Rust test debugging, and the WebView
inspector without placing credentials in project configuration.

## MSFS 2024 SimConnect provider

The desktop and the simulator integration are separate executables. `npm run
dev` now builds the debug provider and stages its target-suffixed Tauri sidecar
automatically before the desktop starts:

```powershell
npm run dev
```

Run `npm run provider:prepare` to perform that preparation without launching the
desktop. The command is covered by the Windows CI smoke path. Tauri release
preparation builds the release provider, stages it under the ignored
`apps/desktop/src-tauri/binaries` directory, then builds the interface. The
provider executable is therefore a Windows-only declared Tauri external binary
rather than a hand-copied release artifact. Non-Windows builds skip the native
provider.

The desktop discovers the development binary under `target/debug`. Set
`WYRMGRID_SIMULATOR_PROVIDER_PATH` to an absolute provider executable path only
when testing another approved build. The provider discovers `SimConnect.dll`
beside itself, through an absolute `WYRMGRID_SIMCONNECT_DLL`, through an absolute
`MSFS2024_SDK` root, or in the standard MSFS 2024 SDK installation directory.
The provider safely reports unavailable if the SDK client or simulator is
absent; neither is needed for normal non-Windows core builds.

Do not copy Microsoft SDK files into the repository or release artifacts. The
first release bundle must follow an explicit redistribution review. See
[simulator provider authoring](integrations/simulator-provider-authoring.md) for
the protocol, FSUIPC path, live-validation requirements, and community-provider
release gate.

## Repository layout

```text
apps/desktop/          Tauri and Svelte desktop interface
crates/                application-owned Rust libraries
docs/                  durable design and operating documentation
examples/plugins/      public protocol examples
providers/             approved simulator provider sidecars
schemas/               language-neutral public contracts
locales/               canonical interface message catalogues
.github/               contribution and automation policy
```

When adding user-facing interface text, add a stable semantic key to
`locales/en-AU.json` and resolve it through the localization runtime. Rust
services should return a semantic code and arguments plus a temporary bounded
English fallback when compatibility requires one; do not choose a locale in a
domain or application service. Update source-catalogue compatibility and
community-pack fixtures when variables or message meaning change.

Keep real credentials outside `.env` files in the repository. The committed
`.env.example` contains names only and is not the planned production secret
storage mechanism.

## Encrypted storage development

Persistent desktop storage is SQLCipher-encrypted. A 32-byte random database
key is stored through the platform credential service (Windows Credential
Manager, macOS Keychain, or a supported Linux Secret Service backend). Existing
encrypted database or recovery state without that credential fails closed; do
not add plaintext SQLite or memory-only fallback paths to make local startup
appear successful.

Portable backup tests create temporary encrypted databases and never use the
developer's credential store. See [ADR-0013](architecture/decisions/0013-sqlcipher-device-keys-and-portable-backups.md)
and the [backup and recovery guide](user-guide/backups-and-recovery.md). The
first full build of vendored OpenSSL is intentionally slower; subsequent builds
reuse the Cargo cache.

### Local cache and data locations on Windows

Do not confuse disposable compiler output with persistent WyrmGrid data:

- `%LOCALAPPDATA%\WyrmGrid\cargo-target` is a disposable build cache. Deleting
  it while WyrmGrid and Cargo are closed loses no user data, but forces a full
  SQLCipher/OpenSSL rebuild and may take several minutes. Repository-local
  `target\debug` output is likewise disposable.
- `%APPDATA%\io.github.phobosdthorga.onairwyrmgrid` contains the encrypted
  application database and recovery state. It is not a cache and must not be
  used as routine space-reclamation material.
- Windows Credential Manager holds the database key separately. Copying only
  the application-data directory does not create a recoverable or portable
  backup.

On development machines where directory cleanup is routine, a local warning
notice may be placed at the root of each WyrmGrid directory. Such notices are
advisory and are not application-managed files. Use the in-app portable-backup
flow before reinstalling Windows, moving machines, or deliberately resetting
local data.

## Authenticated API testing

For current testing, use credentials copied strictly from **OnAir Client →
Options → Global Settings**. A live test on 2026-07-14 found that API details
from the still-developing **OnAir Companion** were rejected while the
Client-provided Company ID and API Key worked.

Companion is expected to become OnAir's primary client. Revalidate this rule
when OnAir announces API credential parity or Companion replaces the older
Client; update the interface, tests, and API-boundary documentation together.

Never place either value in source, `.env` files, command history, fixtures,
screenshots, issue reports, or logs. Authenticated observations must be reduced
to sanitized behavior before being committed.

## Maintainer-only Sentry testing

Sentry is disabled unless both its DSN and an explicit enable flag are present.
For a deliberate local test from PowerShell, set the values only in the current
terminal session before starting Tauri:

```powershell
$env:WYRMGRID_SENTRY_ENABLED = "true"
$env:WYRMGRID_SENTRY_TEST_EVENT = "true"
$env:SENTRY_RUST_DSN = "<Rust project DSN>"
$env:SENTRY_ENVIRONMENT = "maintainer"
$env:VITE_WYRMGRID_SENTRY_ENABLED = "true"
$env:VITE_WYRMGRID_SENTRY_TEST_EVENT = "true"
$env:VITE_SENTRY_DSN = "<UI project DSN>"
$env:VITE_SENTRY_ENVIRONMENT = "maintainer"
npm run dev
```

Close that terminal to discard the variables. Never place
`SENTRY_AUTH_TOKEN` in the desktop runtime environment; it belongs only in
protected GitHub Actions secret storage for release and source-map operations.
Ordinary development, preview, CI, and public builds keep transmission off.
The test-event flags emit one bounded synthetic event per runtime at startup;
remove them after verifying project routing, redaction, and stack traces.
