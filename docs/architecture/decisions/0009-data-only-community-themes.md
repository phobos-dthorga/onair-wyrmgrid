# ADR-0009: Data-only community themes

## Status

Accepted.

## Context

WyrmGrid needs multiple first-party appearances and should let users share
themes. Arbitrary imported CSS is effectively executable presentation code: it
can conceal security messages, counterfeit controls, load remote resources, and
become coupled to private markup that changes every release.

## Decision

The host owns all CSS, component layout, and interaction. Themes are versioned,
strictly validated JSON manifests containing only a fixed set of semantic
colours and a bounded chart palette. Rust is the validation and persistence
authority. The frontend maps accepted roles onto allowlisted CSS variables and
the Atlas and chart adapters.

Built-in identifiers use the reserved `wyrmgrid-` namespace. Invalid, corrupt,
missing, oversized, low-contrast, or unsupported manifests are not applied, and
the interface degrades to WyrmGrid Classic. Theme schema compatibility is
independent of the application, plugin protocol, and database schema versions.

## Consequences

Community themes can be portable without gaining access to application data or
private component structure. Theme authors have less freedom than CSS would
provide, but themes remain stable and reviewable. Any future font, image,
resource, or layout capability requires a new explicit architecture and
security decision.
