# ADR-0010: Canonical English catalogue and data-only community language packs

## Status

Accepted.

## Context

WyrmGrid began with English prose embedded in Svelte components and, in a few
places, application-service results. Continued growth in Dispatch, Atlas,
Hoard, and Forge would make those strings expensive to extract later and would
prevent users from supplying their own translations.

Translation is also a trust boundary. A language pack can mislabel a credential
field, permission, destructive action, telemetry choice, or legal statement even
when it cannot execute code. Arbitrary HTML, JavaScript, remote resources, CSS,
or plugin privileges are therefore inappropriate for localization.

## Decision

English (Australia), identified by `en-AU`, is WyrmGrid's canonical source
locale. The versioned source catalogue lives at `locales/en-AU.json`; message
values use Project Fluent patterns so translations may express their own plural,
grammatical, number, date, and bidirectional rules rather than copying English
sentence structure.

Application and domain services expose stable semantic codes and interpolation
arguments. They do not choose the user's language. During migration, typed
operation errors and Dispatch findings retain bounded English fallback prose for
compatibility. The Svelte presentation layer resolves semantic message keys
against the selected pack and falls back per message to the canonical English
catalogue.

Community language packs are versioned, data-only JSON manifests. Rust is the
validation and persistence authority. It enforces manifest and source-catalogue
versions, bounded metadata and message counts, BCP 47-style locale identifiers,
known source keys, compatible Fluent variables, valid Fluent syntax, and
resource and Unicode-control limits. Packs cannot contain markup or executable
content. Selection and imported packs are stored locally through append-only
SQLite migration 0005.

Unreviewed community packs cannot replace message namespaces for legal and
privacy content, credentials, telemetry, plugin permissions, destructive
actions, or diagnostics. Reviewed bundled translations may eventually cover
those namespaces through a separate release process. Controlling Application
Terms and Privacy Notices remain separately versioned documents; an unofficial
translation may assist a reader but does not silently replace the controlling
English document.

The fallback order is selected locale, a future compatible base-locale pack,
then `en-AU`. Version 1 implements selected-pack-to-English fallback. Partial
community packs are valid and expose coverage against the community-eligible
source catalogue.

Raw provider and user facts are not translated: ICAO identifiers,
registrations, aircraft model labels, METAR and TAF text, company names, plugin
identifiers, and diagnostic logs remain source data. Locale-aware presentation
formats derived numbers, dates, durations, units, and lists without mutating the
underlying facts.

Language-pack schema and source-catalogue versions are independent of the
application version, plugin protocol, database migration, themes, and domain
snapshot schemas. The initial `message_key` addition is an internal Tauri view
change and does not change plugin protocol version 1.

## Consequences

Community translators receive a stable, reviewable source and can ship partial
packs without waiting for WyrmGrid releases. Business rules remain in Rust while
language choice stays presentational. The application pays a small runtime and
catalogue-maintenance cost, and existing hard-coded interface text must be
migrated incrementally. Right-to-left layout, pseudo-locales, catalogue
extraction, reviewed translations, pack deletion/export, and plugin-owned
namespaced catalogues remain explicit follow-up work rather than implicit
capabilities.
