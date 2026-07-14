# Dependency security notes

## 2026-07-15: SQLCipher Community Edition and vendored OpenSSL

`rusqlite` is built with `bundled-sqlcipher-vendored-openssl` so persistent
storage does not silently vary with a host SQLite installation. SQLCipher
Community Edition supplies at-rest encryption but not SQLCipher's commercial
support, FIPS packages, or enterprise extensions. WyrmGrid must not imply those
properties.

The vendored build improves reproducibility at the cost of larger and slower
native builds plus responsibility for monitoring and updating both SQLCipher
and OpenSSL. `cargo-deny`, dependency review, release licence inventory, and
binary notices all remain required. Windows developers need a complete Perl
distribution at build time; released applications do not.

The device key is supplied through SQLCipher's binary key API rather than SQL
text. Portable-backup passwords are bound parameters. Neither choice eliminates
plaintext in process memory or protects a compromised logged-in account; those
limits remain explicit in the threat model.

## 2026-07-13: SvelteKit development dependency

`npm audit` reports a low-severity advisory in `cookie` 0.6 through SvelteKit.
The affected package is development tooling for a static, client-only Tauri
frontend; WyrmGrid does not run SvelteKit as an HTTP server and does not use it
to construct cookie names, paths, or domains.

CI fails for high or critical npm advisories and still displays lower-severity
findings for review. Dependency Review narrowly allows
`GHSA-pxg6-pf52-xh8x` because the current SvelteKit release still declares
`cookie ^0.6.0`, npm reports no compatible fix, and WyrmGrid has no server-side
cookie surface. This is an advisory-specific exception rather than a reduced
repository-wide severity threshold.

Remove the exception and this note when SvelteKit permits `cookie` 0.7.0 or
later, when the dependency is eliminated, or before introducing any HTTP server
or cookie-writing behavior. Reassess it during every dependency update.

## 2026-07-13: Tauri transitive maintenance advisories

Tauri's cross-platform dependency graph currently includes GTK3 and `rust-unic`
crates marked unmaintained by RustSec. They are transitive framework dependencies,
not WyrmGrid's direct choices, and no safe compatible upgrade is available.

`cargo-deny` therefore rejects unmaintained direct dependencies while reporting
transitive findings for review. Known vulnerabilities and unsoundness advisories
remain blocking regardless of dependency depth.
