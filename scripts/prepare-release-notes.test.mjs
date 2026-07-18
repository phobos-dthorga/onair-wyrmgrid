import assert from "node:assert/strict";
import test from "node:test";

import {
  renderReleaseNotes,
  validateReleaseChangelog,
} from "./prepare-release-notes.mjs";

function entry(
  version,
  {
    date = "2026-07-18",
    newFeatures = ["A useful feature."],
    changes = ["A useful improvement."],
    removed = ["None."],
    breaking = ["None."],
  } = {},
) {
  const heading =
    version === "Unreleased" ? "## [Unreleased]" : `## [${version}] - ${date}`;
  const list = (items) => items.map((item) => `- ${item}`).join("\n");

  return `${heading}

### New features

${list(newFeatures)}

### Changes

${list(changes)}

### Removed

${list(removed)}

### 🚨 Breaking changes

${list(breaking)}`;
}

function changelog(...entries) {
  return `# Changelog

All notable changes to OnAir WyrmGrid are documented here.

${[entry("Unreleased"), ...entries].join("\n\n")}
`;
}

test("validates every required release category and renders GitHub notes", () => {
  const validation = validateReleaseChangelog(
    changelog(entry("0.2.1"), entry("0.2.0")),
    "0.2.1",
    "0.2.0",
  );
  const notes = renderReleaseNotes(validation);

  assert.equal(validation.hasBreakingChanges, false);
  assert.match(notes, /^# OnAir WyrmGrid v0\.2\.1/m);
  assert.match(notes, /### New features[\s\S]*### Changes/);
  assert.match(notes, /### Removed[\s\S]*### 🚨 Breaking changes/);
  assert.match(notes, /> \*\*Breaking changes:\*\* None\./);
});

test("rejects a release entry with a missing required category", () => {
  const malformed = changelog(entry("0.2.1"), entry("0.2.0")).replace(
    "### Removed\n\n- None.\n\n",
    "",
  );

  assert.throws(
    () => validateReleaseChangelog(malformed, "0.2.1", "0.2.0"),
    /must contain these sections in order/,
  );
});

test("requires explicit None entries instead of empty categories", () => {
  const malformed = changelog(entry("0.2.1", { removed: [] }), entry("0.2.0"));

  assert.throws(
    () => validateReleaseChangelog(malformed, "0.2.1", "0.2.0"),
    /must contain a list item or '- None\.'/,
  );
});

test("rejects duplicate categories and mixed None entries", () => {
  const duplicate = changelog(entry("0.2.1"), entry("0.2.0")).replace(
    "### Changes\n\n- A useful improvement.",
    "### Changes\n\n- A useful improvement.\n\n### Changes\n\n- Another change.",
  );
  const mixedNone = changelog(
    entry("0.2.1", { removed: ["None.", "An obsolete option."] }),
    entry("0.2.0"),
  );

  assert.throws(
    () => validateReleaseChangelog(duplicate, "0.2.1", "0.2.0"),
    /repeats section 'Changes'/,
  );
  assert.throws(
    () => validateReleaseChangelog(mixedNone, "0.2.1", "0.2.0"),
    /cannot combine '- None\.' with real entries/,
  );
});

test("rejects breaking changes in patch and minor versions", () => {
  const patchRelease = changelog(
    entry("0.2.1", { breaking: ["Existing plugins need migration."] }),
    entry("0.2.0"),
  );
  const minorRelease = changelog(
    entry("0.3.0", { breaking: ["Existing plugins need migration."] }),
    entry("0.2.0"),
  );

  assert.throws(
    () => validateReleaseChangelog(patchRelease, "0.2.1", "0.2.0"),
    /not a new X\.0\.0 major release/,
  );
  assert.throws(
    () => validateReleaseChangelog(minorRelease, "0.3.0", "0.2.0"),
    /not a new X\.0\.0 major release/,
  );
});

test("accepts breaking changes only on a new major release line", () => {
  const validation = validateReleaseChangelog(
    changelog(
      entry("1.0.0", {
        breaking: ["Plugin authors must adopt protocol version 2."],
      }),
      entry("0.2.0"),
    ),
    "1.0.0",
    "0.2.0",
  );
  const notes = renderReleaseNotes(validation);

  assert.equal(validation.hasBreakingChanges, true);
  assert.match(notes, /> 🚨 \*\*BREAKING CHANGES — MAJOR VERSION\*\*/);
  assert.match(notes, /Plugin authors must adopt protocol version 2\./);
});

test("keeps breaking-change warnings through a major prerelease series", () => {
  const validation = validateReleaseChangelog(
    changelog(
      entry("1.0.0-rc.2", {
        breaking: ["Plugin authors must adopt protocol version 2."],
      }),
      entry("1.0.0-rc.1", {
        breaking: ["Plugin authors must adopt protocol version 2."],
      }),
      entry("0.2.0"),
    ),
    "1.0.0-rc.2",
    "1.0.0-rc.1",
  );

  assert.equal(validation.hasBreakingChanges, true);
});

test("derives the nearest prior changelog version when one is not supplied", () => {
  const validation = validateReleaseChangelog(
    changelog(entry("0.3.0"), entry("0.2.1"), entry("0.2.0")),
    "0.3.0",
  );

  assert.equal(validation.previousVersion, "0.2.1");
});

test("allows comparison-link definitions after the oldest release entry", () => {
  const withLinks = `${changelog(entry("0.2.0"))}
[Unreleased]: https://example.test/compare/v0.2.0...HEAD
[0.2.0]: https://example.test/releases/v0.2.0
`;

  const validation = validateReleaseChangelog(withLinks, "0.2.0");
  assert.equal(validation.hasBreakingChanges, false);
});
