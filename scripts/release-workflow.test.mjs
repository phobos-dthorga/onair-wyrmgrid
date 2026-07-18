import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const releaseWorkflow = await readFile(
  resolve(repositoryRoot, ".github/workflows/release.yml"),
  "utf8",
);

test("publishes against the explicit GitHub repository identity", () => {
  assert.match(releaseWorkflow, /GH_REPO: \$\{\{ github\.repository \}\}/);
});

test("generates checksums before adding the checksum list to release assets", () => {
  assert.match(releaseWorkflow, /checksum_file="\$\(mktemp\)"/);
  assert.match(
    releaseWorkflow,
    /mv "\$checksum_file" release-assets\/SHA256SUMS\.txt/,
  );
  assert.doesNotMatch(releaseWorkflow, /> SHA256SUMS\.txt/);
});

test("normalizes package names before checksumming and GitHub upload", () => {
  assert.match(releaseWorkflow, /name="\$\{name\/\/ \/\.\}"/);
});

test("validates the curated changelog against the previous release tag", () => {
  assert.match(
    releaseWorkflow,
    /git tag --list 'v\*' --merged origin\/main[\s\S]*select-previous-release\.mjs "\$RELEASE_VERSION" --tag-lines/,
  );
  assert.match(
    releaseWorkflow,
    /node scripts\/prepare-release-notes\.mjs "\$\{release_notes_args\[@\]\}"/,
  );
});

test("uses curated release notes for new and rebuilt GitHub releases", () => {
  assert.match(
    releaseWorkflow,
    /gh release edit "\$RELEASE_TAG"[\s\S]*--notes-file "\$RELEASE_NOTES"/,
  );
  assert.match(
    releaseWorkflow,
    /gh release create "\$RELEASE_TAG"[\s\S]*--notes-file "\$RELEASE_NOTES"/,
  );
  assert.doesNotMatch(releaseWorkflow, /--notes "CI-built platform packages/);
  assert.doesNotMatch(
    releaseWorkflow,
    /hoardmind|ollama|openai-compatible|optional-ai|api\/chat|chat\/completions|model api/i,
  );
});
