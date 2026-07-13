# Development environment

## Windows

Install Microsoft C++ Build Tools with Desktop development with C++, WebView2,
Rust through rustup, Node.js 22, and npm 10 or later. The repository pins the
Rust toolchain in `rust-toolchain.toml`.

```powershell
npm ci
cargo test --workspace --exclude wyrmgrid-desktop
npm run check
npm run dev
```

## Repository layout

```text
apps/desktop/          Tauri and Svelte desktop interface
crates/                application-owned Rust libraries
docs/                  durable design and operating documentation
examples/plugins/      public protocol examples
schemas/               language-neutral public contracts
.github/               contribution and automation policy
```

Keep real credentials outside `.env` files in the repository. The committed
`.env.example` contains names only and is not the planned production secret
storage mechanism.

## Authenticated API testing

Use credentials copied strictly from **OnAir Client → Options → Global
Settings**. Do not use the API details displayed by **OnAir Companion**. A live
test on 2026-07-14 found that Companion-provided values were rejected while the
Client-provided Company ID and API Key worked.

Never place either value in source, `.env` files, command history, fixtures,
screenshots, issue reports, or logs. Authenticated observations must be reduced
to sanitized behavior before being committed.
