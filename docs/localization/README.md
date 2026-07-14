# Localization and community language packs

WyrmGrid uses English (Australia), `en-AU`, as its canonical reference locale.
Translations may depart from English grammar as long as they preserve the
message's product meaning. Message patterns use
[Project Fluent](https://projectfluent.org/), which supports locale-specific
grammar, plurals, number and date formatting, and bidirectional text.

## Implemented foundation

- `locales/en-AU.json` is source catalogue version 1.
- Language-pack manifest schema version 1 is defined in
  `schemas/language-pack-v1.schema.json`.
- Rust validates and canonicalizes imported packs before SQLite persistence.
- The interface layers the selected partial pack over English per message.
- The application shell, Theme settings, Language settings, and explanatory
  Dispatch findings use stable message keys.
- Fluent's isolation markers remain enabled for safe mixed-direction values.
- English compatibility prose remains in typed Rust results during the
  incremental migration.

The locale field uses a hyphenated BCP 47-style identifier such as `fr`, `de`,
`pt-BR`, or `ar`. Unicode's locale model and language-tag conventions are
documented in [UTS #35](https://unicode.org/reports/tr35/).

## Authoring a version 1 pack

Start from `schemas/fixtures/language-pack-v1.json`, then:

1. Choose a lowercase pack identifier that does not begin with `wyrmgrid-`.
2. Set the target locale, display name, optional author, and text direction.
3. Keep `source_locale` as `en-AU` and target the current
   `source_catalog_version`.
4. Copy only the message keys being translated from `locales/en-AU.json`.
5. Preserve every Fluent variable used by the English message. For example,
   `{ $translated }` and `{ $total }` must both remain present in a translation
   of `language-coverage`.
6. Import the JSON file from **Language** in the WyrmGrid header. The pack is
   selected immediately after successful validation.

Partial packs are expected. Missing messages fall back individually to English,
and the Language dialog reports translated versus eligible message counts.

## Safety and trust boundary

Version 1 language packs are text data. They cannot include HTML, CSS,
JavaScript, files, images, fonts, URLs that WyrmGrid loads, or plugin
capabilities. Manifest size is limited to 256 KiB, message count to 2,048, and
each pattern to 2 KiB. Unknown keys, incompatible variables, invalid Fluent
syntax, markup delimiters, dangerous bidirectional controls, reserved pack IDs,
and unsupported versions are rejected.

Community packs cannot override keys beginning with `legal-`, `privacy-`,
`credential-`, `telemetry-`, `plugin-permission-`, `destructive-`, or `error-`.
These prompts remain canonical English until a translation is reviewed and
bundled through a future trusted release path.

Imported pack contents and author metadata remain in the local WyrmGrid
database. They are not sent to Sentry, external providers, or plugins. A pack
author should not place contact details or other private information in a
manifest they intend to share.

## Compatibility and future work

The source-catalogue version changes when message meaning, variables, or the
compatibility contract changes. Schema changes use a new language-pack schema
version. Older packs must never be reinterpreted silently.

Planned follow-up work includes:

- migrating the remaining Svelte labels and structured Rust outcomes;
- locale-aware number, list, duration, unit, and date formatting everywhere;
- `en-XA` expansion and an RTL pseudo-locale in CI;
- logical CSS properties and full right-to-left visual verification;
- pack export, deletion, diagnostics, and upgrade reports;
- reviewed bundled translations and a separate legal-document workflow; and
- plugin-owned message namespaces that cannot override host messages.
