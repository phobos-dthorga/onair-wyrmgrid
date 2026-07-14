# Development environment

## Windows

Install Microsoft C++ Build Tools with Desktop development with C++, WebView2,
Rust through rustup, Node.js 22, and npm 10 or later. The repository pins the
Rust toolchain in `rust-toolchain.toml`.

```powershell
npm ci
cargo test --workspace
npm test --workspace @wyrmgrid/desktop
npm run check
npm run dev
```

See the [testing strategy](testing.md) for test placement, required cases, CI
gates, and the safe scope for automated test-writing agents.

## Repository layout

```text
apps/desktop/          Tauri and Svelte desktop interface
crates/                application-owned Rust libraries
docs/                  durable design and operating documentation
examples/plugins/      public protocol examples
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
