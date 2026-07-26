import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const [releaseWorkflow, ciWorkflow, securityWorkflow] = await Promise.all([
  readFile(resolve(repositoryRoot, ".github/workflows/release.yml"), "utf8"),
  readFile(resolve(repositoryRoot, ".github/workflows/ci.yml"), "utf8"),
  readFile(resolve(repositoryRoot, ".github/workflows/security.yml"), "utf8"),
]);

function workflowTriggers(workflow) {
  return /^on:\n([\s\S]*?)\npermissions:/m.exec(workflow)?.[1] ?? "";
}

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
  assert.match(
    releaseWorkflow,
    /Published release \$RELEASE_TAG cannot be replaced/,
  );
  assert.match(releaseWorkflow, /release-query-publish\.err/);
  assert.match(releaseWorkflow, /elif grep -q 'HTTP 404'/);
});

test("hosted workflows cannot race Jenkins automation", () => {
  const releaseTriggers = workflowTriggers(releaseWorkflow);
  const ciTriggers = workflowTriggers(ciWorkflow);
  const securityTriggers = workflowTriggers(securityWorkflow);

  assert.match(releaseTriggers, /workflow_dispatch:/);
  assert.doesNotMatch(releaseTriggers, /\bpush:|\bpull_request:|\bschedule:/);
  assert.match(ciTriggers, /workflow_call:/);
  assert.doesNotMatch(
    ciTriggers,
    /\bworkflow_dispatch:|\bpull_request:|\bschedule:/,
  );
  assert.match(securityTriggers, /workflow_call:/);
  assert.doesNotMatch(
    securityTriggers,
    /\bworkflow_dispatch:|\bpull_request:|\bschedule:/,
  );
});

test("manual fallback uses a repository-scoped Teleport Machine ID", () => {
  assert.match(releaseWorkflow, /teleport:\n[\s\S]*needs: policy/);
  assert.match(
    releaseWorkflow,
    /permissions:\n\s+contents: read\n\s+id-token: write/,
  );
  assert.match(
    releaseWorkflow,
    /teleport-actions\/setup@b638ff596557cc3959eb6b5287d5e58e0c8ac6a6 # v1/,
  );
  assert.match(
    releaseWorkflow,
    /teleport-actions\/auth@3b365df2b4f64891358392a444ff34929ba0d0b1 # v2/,
  );
  assert.match(releaseWorkflow, /token: github-actions-onair-wyrmgrid/);
  assert.match(releaseWorkflow, /certificate-ttl: 15m/);
  assert.match(releaseWorkflow, /tsh ssh jenkins@web\.tauryk\.gekkofyre\.io/);
  assert.match(releaseWorkflow, /needs: \[policy, teleport\]/);
  assert.doesNotMatch(
    releaseWorkflow,
    /TELEPORT_(?:SECRET|IDENTITY)|secrets\.[A-Z_]*TELEPORT|jenkins@[^ \n]*:22|root@web\.tauryk/i,
  );
});

test("manual fallback builds only Windows and Linux packages", () => {
  assert.match(releaseWorkflow, /platform: windows-latest/);
  assert.match(releaseWorkflow, /platform: ubuntu-22\.04/);
  assert.doesNotMatch(releaseWorkflow, /macos|--bundles app,dmg|macos-dmg/i);
  assert.match(releaseWorkflow, /EXCEPTION_REASON/);
  assert.match(releaseWorkflow, /\$\{#EXCEPTION_REASON\} < 20/);
  assert.match(releaseWorkflow, /asset\.name\.endsWith\('-setup\.exe'\)/);
  assert.match(releaseWorkflow, /--tag-lines/);
});
