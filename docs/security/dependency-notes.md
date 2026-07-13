# Dependency security notes

## 2026-07-13: SvelteKit development dependency

`npm audit` reports a low-severity advisory in `cookie` 0.6 through SvelteKit.
The affected package is development tooling for a static, client-only Tauri
frontend; WyrmGrid does not run SvelteKit as an HTTP server and does not use it
to construct cookie names, paths, or domains.

CI fails for high or critical npm advisories and still displays lower-severity
findings for review. This note should be removed when SvelteKit accepts a fixed
`cookie` release or the dependency is otherwise eliminated.

## 2026-07-13: Tauri transitive maintenance advisories

Tauri's cross-platform dependency graph currently includes GTK3 and `rust-unic`
crates marked unmaintained by RustSec. They are transitive framework dependencies,
not WyrmGrid's direct choices, and no safe compatible upgrade is available.

`cargo-deny` therefore rejects unmaintained direct dependencies while reporting
transitive findings for review. Known vulnerabilities and unsoundness advisories
remain blocking regardless of dependency depth.
