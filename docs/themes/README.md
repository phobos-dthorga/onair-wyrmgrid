# Themes

WyrmGrid separates visual styling from application behaviour. Internal CSS is
split into semantic colour tokens, global foundations, and workspace layout.
Interface components consume roles such as `accent`, `surface`, and `danger`
instead of assuming WyrmGrid Classic's literal colours.

## Built-in themes

The desktop application currently includes WyrmGrid Classic, Phobos D'thorga,
Daylight Dispatch, and High Contrast. Phobos D'thorga uses oxblood surfaces,
ember-red accents, and warm highlights as a dark tribute to the project's
author. The selected theme is stored locally and is applied to the workspace,
dialogs, Atlas markers and labels, and declarative charts.

## Community theme boundary

A community theme is a data-only JSON document conforming to
[`theme-manifest.schema.json`](../../schemas/theme-manifest.schema.json). Import
is deliberately narrower than CSS:

- the manifest is limited to 32 KiB and schema version 1;
- only named colour roles and a three-to-eight-colour chart palette are accepted;
- colours must use `#RRGGBB` notation;
- unknown fields are rejected;
- custom identifiers cannot use the reserved `wyrmgrid-` prefix; and
- Rust enforces minimum text, muted-text, accent, and danger contrast before
  saving the manifest.

Themes cannot contain CSS, JavaScript, HTML, URLs, images, fonts, file paths,
selectors, layout instructions, or map-style sources. The host derives all
translucent colours from the accepted roles and only writes an allowlisted set
of CSS custom properties. This keeps theme imports outside the plugin capability
model and prevents them from becoming an executable extension mechanism.

The fixture at
[`theme-manifest-v1.json`](../../schemas/fixtures/theme-manifest-v1.json) is a
complete example. Importing a valid custom theme selects it immediately; a
failed or missing stored theme safely falls back to WyrmGrid Classic.

## Managing and authoring themes

Every available theme can be exported as a schema-version-1 JSON document.
Locally imported themes can also be edited or deleted; deleting the active
theme selects WyrmGrid Classic atomically with the deletion. Bundled themes
cannot be deleted, but the authoring tool can create a custom copy with a
non-reserved identifier.

WyrmGrid displays provenance separately from manifest content. A theme is
identified as either bundled with WyrmGrid or imported locally, with its local
import and update times. The optional `author` field is always labelled as an
unverified manifest claim: it is not a signature, endorsement, or proof of
origin, and provenance metadata is never added to exported theme manifests.

Import rejects an exact re-import and a different theme identifier that uses
the same colour roles and chart palette as an available theme. A changed
manifest with the same custom identifier is treated as an intentional local
revision and preserves the original import time.

The data-only authoring tool starts from an available theme, edits every
allowlisted role and chart colour, and shows the same contrast thresholds used
by the Rust validator. Its preview is advisory; saving still passes the complete
manifest through the shared validation and persistence service.

## Compatibility decision

Theme manifests are independently versioned. Version 1 is accepted exactly;
newer versions are rejected with a bounded message rather than interpreted
partially. A future schema change requires a new version, fixture, validation
tests, documentation, and an explicit migration or coexistence decision.

## Future work

Document review criteria before introducing a curated gallery. Consider
authenticity metadata only when there is a real distribution channel. Fonts,
images, remote resources, arbitrary CSS, and layout packs must remain prohibited
unless a later threat-model and capability review establishes a genuinely safe,
bounded design.
