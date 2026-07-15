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

test("publishes without relying on a checked-out Git repository", () => {
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
