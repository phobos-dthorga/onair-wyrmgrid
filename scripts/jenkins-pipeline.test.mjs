import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const [multibranchPipeline, releasePipeline, packageMetadata, tauriConfig] =
  await Promise.all([
    readFile(resolve(repositoryRoot, "Jenkinsfile"), "utf8"),
    readFile(resolve(repositoryRoot, "Jenkinsfile.release"), "utf8"),
    readFile(resolve(repositoryRoot, "package.json"), "utf8").then(JSON.parse),
    readFile(
      resolve(repositoryRoot, "apps/desktop/src-tauri/tauri.conf.json"),
      "utf8",
    ).then(JSON.parse),
  ]);

test("multibranch pipeline runs the complete credential-free Linux and Windows gates", () => {
  assert.match(multibranchPipeline, /agent \{ label 'linux' \}/);
  assert.match(multibranchPipeline, /agent \{ label 'windows' \}/);
  assert.match(multibranchPipeline, /npm run ci:frontend/);
  assert.match(multibranchPipeline, /npm run ci:python/);
  assert.match(multibranchPipeline, /npm run ci:rust/);
  assert.match(multibranchPipeline, /npm run ci:dependencies/);
  assert.match(multibranchPipeline, /npm run ci:prepare/);
  assert.doesNotMatch(multibranchPipeline, /timeout\(time: 2,/);
  assert.equal(
    [...multibranchPipeline.matchAll(/timeout\(time: 3, unit: 'HOURS'\)/g)]
      .length,
    4,
  );
  assert.match(multibranchPipeline, /timeout\(time: 7, unit: 'HOURS'\)/);
  assert.match(
    multibranchPipeline,
    /disableConcurrentBuilds\(abortPrevious: true\)/,
  );

  assert.doesNotMatch(
    multibranchPipeline,
    /withCredentials|credentialsId|GH_TOKEN|gh release|SENTRY_AUTH_TOKEN/,
  );
  assert.doesNotMatch(
    multibranchPipeline,
    /hoardmind|ollama|openai-compatible|optional-ai|api\/chat|chat\/completions|model api/i,
  );
});

test("snapshot packages are limited to main and release branches", () => {
  assert.match(multibranchPipeline, /env\.BRANCH_NAME == 'main'/);
  assert.match(
    multibranchPipeline,
    /env\.BRANCH_NAME ==~ \/\^codex\\\/release-\.\+\//,
  );
  assert.match(multibranchPipeline, /expression \{ isSnapshotBranch\(\) \}/);
  assert.match(multibranchPipeline, /--bundles appimage,deb/);
  assert.match(multibranchPipeline, /--bundles nsis/);
  assert.match(multibranchPipeline, /test-nsis-installer\.ps1/);
  assert.match(multibranchPipeline, /BUILD-INFO\.json/);
  assert.match(multibranchPipeline, /SHA256SUMS\.txt/);
  assert.doesNotMatch(multibranchPipeline, /--bundles [^\n]*(?:dmg|app,)/i);
});

test("trusted release pipeline validates exact tags before credential use", () => {
  const tagValidation = releasePipeline.indexOf(
    "RELEASE_TAG must be a supported vX.Y.Z or prerelease tag.",
  );
  const versionValidation = releasePipeline.indexOf(
    "node scripts/verify-release-version.mjs",
  );
  const ancestryValidation = releasePipeline.indexOf(
    "git merge-base --is-ancestor",
  );
  const credentialBinding = releasePipeline.indexOf("withCredentials");

  assert.notEqual(tagValidation, -1);
  assert.notEqual(versionValidation, -1);
  assert.notEqual(ancestryValidation, -1);
  assert.notEqual(credentialBinding, -1);
  assert.ok(tagValidation < credentialBinding);
  assert.ok(versionValidation < credentialBinding);
  assert.ok(ancestryValidation < credentialBinding);
  assert.match(releasePipeline, /EXCEPTION_REASON.*20/);
  assert.match(
    releasePipeline,
    /Published release \$RELEASE_TAG cannot be replaced/,
  );
  assert.match(releasePipeline, /release-query\.err/);
  assert.match(releasePipeline, /elif ! grep -q 'HTTP 404'/);
  assert.match(releasePipeline, /credentialsId: 'wyrmgrid-github-release'/);
});

test("release publication remains an approved draft prerelease", () => {
  assert.equal([...releasePipeline.matchAll(/npm run ci:prepare/g)].length, 2);
  assert.match(releasePipeline, /stage\('Approve draft publication'\)/);
  assert.match(releasePipeline, /input\(/);
  assert.match(releasePipeline, /gh release create/);
  assert.match(releasePipeline, /gh release edit/);
  assert.match(releasePipeline, /--draft/);
  assert.match(releasePipeline, /--prerelease/);
  assert.match(releasePipeline, /--paginate --slurp/);
  assert.match(releasePipeline, /asset\.name\.endsWith\('-setup\.exe'\)/);
  assert.match(releasePipeline, /SHA256SUMS\.txt/);
  assert.match(releasePipeline, /provenance: 'checksums-only'/);
  assert.match(
    releasePipeline,
    /platforms: \['linux_x86_64', 'windows_x86_64'\]/,
  );
  assert.doesNotMatch(releasePipeline, /--bundles [^\n]*(?:dmg|app,)/i);
  assert.doesNotMatch(
    releasePipeline,
    /hoardmind|ollama|openai-compatible|optional-ai|api\/chat|chat\/completions|model api/i,
  );
});

test("application packaging supports Windows and Linux without macOS bundles", () => {
  assert.deepEqual(tauriConfig.bundle.targets, ["nsis", "appimage", "deb"]);
  assert.equal(tauriConfig.bundle.windows.nsis.installMode, "currentUser");
  assert.equal(tauriConfig.productName, "OnAir WyrmGrid");
  assert.equal(tauriConfig.identifier, "io.github.phobosdthorga.onairwyrmgrid");
});

test("package scripts are the shared CI command contract", () => {
  assert.equal(
    packageMetadata.scripts["ci:python"],
    "python -m unittest discover -s sdk/python/tests -v && python -m unittest discover -s plugins/tests -v",
  );
  assert.match(packageMetadata.scripts["ci:frontend"], /test:tooling/);
  assert.match(packageMetadata.scripts["ci:frontend"], /audit:boundaries/);
  assert.equal(
    packageMetadata.scripts["ci:prepare"],
    "npm run provider:prepare && npm run audio-codec:prepare",
  );
  assert.match(packageMetadata.scripts["ci:rust"], /--locked/);
  assert.match(packageMetadata.scripts["ci:rust"], /-D warnings/);
  assert.match(packageMetadata.scripts["ci:dependencies"], /cargo deny check/);
  assert.match(
    packageMetadata.scripts["ci:dependencies"],
    /npm audit --audit-level=high/,
  );
});
