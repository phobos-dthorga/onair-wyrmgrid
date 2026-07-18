import assert from "node:assert/strict";
import { generateKeyPairSync } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  assertChangesWithinScopes,
  buildCommitMessage,
  buildPullRequestBody,
  createGitHubAppJwt,
  extractGeneratedPatchFromDraft,
  isProtectedContributionPath,
  parseGeneratedPatch,
  prepareContributionManifest,
  publishContribution,
  sha256,
  validateContributionManifest,
  validateGitHubAppConfig,
} from "./optional-ai-contribution.mjs";

const repositoryRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

const modifiedPatch = `diff --git a/src/example.txt b/src/example.txt
index 257cc56..3bd1f0e 100644
--- a/src/example.txt
+++ b/src/example.txt
@@ -1 +1 @@
-old
+new
`;

const newFilePatch = `diff --git a/tests/generated.test.ts b/tests/generated.test.ts
new file mode 100644
index 0000000..4ad4d89
--- /dev/null
+++ b/tests/generated.test.ts
@@ -0,0 +1 @@
+export const generated = true;
`;

function contribution(overrides = {}) {
  return {
    contribution_id: "hm-20260718-001",
    assistant_id: "hoardmind",
    assistant_display_name: "Hoardmind",
    model: "qwen3-coder:30b",
    task_contract: "test-matrix-v1",
    base_branch: "main",
    branch_name: "assistant/hoardmind/hm-20260718-001",
    commit_subject: "test: add generated boundary coverage",
    pull_request_title: "Add generated boundary coverage",
    summary: "Adds one generated regression case for maintainer review.",
    allowed_paths: ["src/example.txt", "tests/"],
    ...overrides,
  };
}

function completeManifest(overrides = {}) {
  return {
    schema_version: 1,
    kind: "optional-ai-generated-contribution",
    ...contribution(),
    created_at: "2026-07-18T08:00:00.000Z",
    packet_sha256: "1".repeat(64),
    patch_sha256: "2".repeat(64),
    metrics_sha256: null,
    human_review_required: true,
    merge_authority: false,
    ...overrides,
  };
}

function git(cwd, ...args) {
  return execFileSync("git", args, {
    cwd,
    encoding: "utf8",
    windowsHide: true,
  }).trim();
}

test("accepts bounded text modifications and regular new files", () => {
  assert.deepEqual(parseGeneratedPatch(modifiedPatch), [
    { path: "src/example.txt", is_new: false, hasHunk: true },
  ]);
  assert.deepEqual(parseGeneratedPatch(newFilePatch), [
    { path: "tests/generated.test.ts", is_new: true, hasHunk: true },
  ]);
});

test("extracts exactly one isolated diff from the bounded implementation contract", () => {
  const draft = `# Draft\n\n## Scope interpreted\n\nNarrow scope.\n\n## Proposed patch\n\n\`\`\`diff\n${modifiedPatch}\`\`\`\n\n## Validation plan\n\nRun tests.\n\n## Risks and uncertainty\n\n- None.\n`;
  assert.equal(extractGeneratedPatchFromDraft(draft), modifiedPatch);
  assert.throws(
    () =>
      extractGeneratedPatchFromDraft(
        draft.replace("```diff", "Some prose.\n\n```diff"),
      ),
    /exactly one fenced diff/,
  );
  assert.throws(
    () =>
      extractGeneratedPatchFromDraft(
        draft.replace(
          "```\n\n## Validation",
          `\`\`\`\n\n\`\`\`diff\n${newFilePatch}\`\`\`\n\n## Validation`,
        ),
      ),
    /exactly one fenced diff/,
  );
});

test("rejects deletion, rename, binary, mode, traversal, and protected-path patches", () => {
  const rejected = [
    modifiedPatch.replace(
      "index 257cc56..3bd1f0e 100644",
      "deleted file mode 100644",
    ),
    modifiedPatch.replace(
      "diff --git a/src/example.txt b/src/example.txt",
      "diff --git a/src/example.txt b/src/renamed.txt",
    ),
    modifiedPatch.replace("@@ -1 +1 @@", "GIT binary patch"),
    modifiedPatch.replace(
      "index 257cc56..3bd1f0e 100644",
      "old mode 100644\nnew mode 100755",
    ),
    modifiedPatch.replaceAll("src/example.txt", "src/../secret.txt"),
    modifiedPatch.replaceAll(
      "src/example.txt",
      ".github/workflows/release.yml",
    ),
    modifiedPatch.replaceAll(
      "src/example.txt",
      "docs/security/threat-model.md",
    ),
    modifiedPatch.replaceAll(
      "src/example.txt",
      "schemas/protocol-v2.schema.json",
    ),
    modifiedPatch.replaceAll(
      "src/example.txt",
      "scripts/release-workflow.test.mjs",
    ),
    modifiedPatch.replaceAll("src/example.txt", "crates/onair-api/src/lib.rs"),
    modifiedPatch.replace("--- a/src/example.txt", "--- a/AGENTS.md"),
    modifiedPatch.replace("+++ b/src/example.txt", "+++ b/AGENTS.md"),
    modifiedPatch.replace(
      "index 257cc56..3bd1f0e 100644",
      "similarity index 100%\ncopy from src/example.txt\ncopy to src/copied.txt",
    ),
  ];
  for (const patch of rejected) {
    assert.throws(() => parseGeneratedPatch(patch));
  }
  assert.equal(
    isProtectedContributionPath("schemas/fixtures/synthetic.json"),
    false,
  );
  assert.equal(
    isProtectedContributionPath("crates/storage/migrations/0015.sql"),
    true,
  );
});

test("requires every changed file to remain inside reviewer-approved scopes", () => {
  const changes = [
    ...parseGeneratedPatch(modifiedPatch),
    ...parseGeneratedPatch(newFilePatch),
  ];
  assert.deepEqual(
    assertChangesWithinScopes(changes, ["src/example.txt", "tests/"]),
    ["src/example.txt", "tests/"],
  );
  assert.throws(() => assertChangesWithinScopes(changes, ["src/"]), /outside/);
  assert.throws(
    () => assertChangesWithinScopes(changes, [".github/"]),
    /protected/,
  );
  assert.throws(
    () => assertChangesWithinScopes(changes, ["src/", "src/"]),
    /unique/,
  );
});

test("validates fail-closed provenance and renders durable attribution", () => {
  const manifest = validateContributionManifest(completeManifest());
  const commit = buildCommitMessage(manifest);
  assert.match(commit, /Generated-by: Hoardmind/);
  assert.match(commit, /Optional-AI-Contribution: hm-20260718-001/);
  assert.match(commit, /Human-Review-Required: true/);
  assert.match(commit, /Merge-Authority: none/);
  const body = buildPullRequestBody(
    manifest,
    [{ path: "src/example.txt", is_new: false, hasHunk: true }],
    "3".repeat(40),
  );
  assert.match(body, /one independently revertable merge unit/);
  assert.throws(
    () =>
      validateContributionManifest(completeManifest({ merge_authority: true })),
    /grant no merge authority/,
  );
  assert.throws(
    () =>
      validateContributionManifest(
        completeManifest({ branch_name: "feature/not-attributed" }),
      ),
    /unsupported format/,
  );
  assert.throws(
    () =>
      validateContributionManifest(
        completeManifest({ base_branch: "development" }),
      ),
    /reviewed main branch/,
  );
});

test("creates a bounded RS256 GitHub App JWT without embedding the private key", () => {
  const { privateKey } = generateKeyPairSync("rsa", { modulusLength: 2048 });
  const pem = privateKey.export({ type: "pkcs8", format: "pem" });
  const jwt = createGitHubAppJwt({
    appId: "12345",
    privateKey: pem,
    nowSeconds: 1_700_000_000,
  });
  const [header, payload, signature] = jwt.split(".");
  assert.deepEqual(JSON.parse(Buffer.from(header, "base64url")), {
    alg: "RS256",
    typ: "JWT",
  });
  assert.deepEqual(JSON.parse(Buffer.from(payload, "base64url")), {
    iat: 1_699_999_940,
    exp: 1_700_000_480,
    iss: "12345",
  });
  assert.ok(signature.length > 100);
  assert.doesNotMatch(jwt, /PRIVATE KEY/);
});

test("keeps the App key outside the repository and rejects unknown configuration", () => {
  const root = join(tmpdir(), "wyrmgrid-config-root");
  const configPath = join(root, ".wyrmgrid-local", "github-app.json");
  const valid = {
    schema_version: 1,
    kind: "optional-ai-github-app-installation",
    repository: "phobos-dthorga/onair-wyrmgrid",
    expected_app_slug: "wyrmgrid-hoardmind",
    app_id: "123",
    installation_id: "456",
    private_key_path: join(tmpdir(), "wyrmgrid-hoardmind.pem"),
  };
  assert.equal(validateGitHubAppConfig(valid, configPath, root).app_id, "123");
  assert.throws(
    () =>
      validateGitHubAppConfig(
        {
          ...valid,
          private_key_path: join(root, ".wyrmgrid-local", "key.pem"),
        },
        configPath,
        root,
      ),
    /outside the repository/,
  );
  assert.throws(
    () =>
      validateGitHubAppConfig(
        { ...valid, token: "not-allowed" },
        configPath,
        root,
      ),
    /Unknown GitHub App configuration field/,
  );
  assert.throws(
    () => validateGitHubAppConfig({ ...valid, app_id: "0" }, configPath, root),
    /positive decimal identifiers/,
  );
  if (process.platform === "win32") {
    const otherDrive = root[0].toUpperCase() === "C" ? "D" : "C";
    const crossVolumeKey = `${otherDrive}:\\private\\wyrmgrid-app.pem`;
    assert.equal(
      validateGitHubAppConfig(
        { ...valid, private_key_path: crossVolumeKey },
        configPath,
        root,
      ).private_key_path,
      crossVolumeKey,
    );
  }
});

test("keeps shipped contribution examples synchronized with runtime and JSON schemas", async () => {
  const manifest = JSON.parse(
    await readFile(
      join(
        repositoryRoot,
        "examples/optional-ai/generated-contribution-manifest-v1.json",
      ),
      "utf8",
    ),
  );
  const config = JSON.parse(
    await readFile(
      join(
        repositoryRoot,
        "examples/optional-ai/github-app-installation-config-v1.json",
      ),
      "utf8",
    ),
  );
  const manifestSchema = JSON.parse(
    await readFile(
      join(
        repositoryRoot,
        "schemas/optional-ai-generated-contribution-v1.schema.json",
      ),
      "utf8",
    ),
  );
  const configSchema = JSON.parse(
    await readFile(
      join(
        repositoryRoot,
        "schemas/optional-ai-github-app-config-v1.schema.json",
      ),
      "utf8",
    ),
  );

  const { $schema: manifestSchemaReference, ...manifestFields } = manifest;
  const { $schema: configSchemaReference } = config;
  assert.deepEqual(validateContributionManifest(manifest), manifestFields);
  const copiedConfigPath = join(
    repositoryRoot,
    ".wyrmgrid-local/github-app.json",
  );
  const validatedConfig = validateGitHubAppConfig(
    config,
    copiedConfigPath,
    repositoryRoot,
  );
  assert.equal(validatedConfig.repository, config.repository);
  assert.equal(validatedConfig.expected_app_slug, config.expected_app_slug);
  assert.equal(validatedConfig.app_id, config.app_id);
  assert.equal(validatedConfig.installation_id, config.installation_id);
  assert.equal(
    validatedConfig.private_key_path,
    resolve(dirname(copiedConfigPath), config.private_key_path),
  );
  assert.equal(
    manifestSchemaReference,
    "../../schemas/optional-ai-generated-contribution-v1.schema.json",
  );
  assert.equal(
    configSchemaReference,
    "../../schemas/optional-ai-github-app-config-v1.schema.json",
  );
  assert.equal(manifestSchema.properties.base_branch.const, "main");
  assert.match(
    manifest.branch_name,
    new RegExp(manifestSchema.properties.branch_name.pattern),
  );
  assert.equal(
    configSchema.properties.kind.const,
    "optional-ai-github-app-installation",
  );
  assert.equal(configSchema.additionalProperties, false);
});

test("prepares a private hash-bound manifest and rejects credential-like patches", async () => {
  const root = await mkdtemp(join(tmpdir(), "wyrmgrid-contribution-prepare-"));
  const outside = await mkdtemp(
    join(tmpdir(), "wyrmgrid-contribution-evidence-"),
  );
  try {
    const patchPath = join(outside, "change.patch");
    const packetPath = join(outside, "packet.md");
    const outputPath = join(outside, "manifest.json");
    await Promise.all([
      writeFile(patchPath, modifiedPatch),
      writeFile(packetPath, "# Sanitized packet\n"),
    ]);
    const prepared = await prepareContributionManifest({
      patchPath,
      packetPath,
      metricsPath: null,
      outputPath,
      contribution: contribution(),
      root,
      now: new Date("2026-07-18T08:00:00Z"),
    });
    assert.equal(prepared.manifest.patch_sha256, sha256(modifiedPatch));
    assert.equal(
      prepared.manifest.packet_sha256,
      sha256("# Sanitized packet\n"),
    );
    assert.equal(prepared.manifest_sha256, sha256(await readFile(outputPath)));
    await writeFile(
      patchPath,
      modifiedPatch.replace(
        "+new",
        "+token = 'github_pat_12345678901234567890'",
      ),
    );
    await assert.rejects(
      prepareContributionManifest({
        patchPath,
        packetPath,
        outputPath: join(outside, "unsafe.json"),
        contribution: contribution(),
        root,
      }),
      /credential-like/,
    );
  } finally {
    await Promise.all([
      rm(root, { recursive: true, force: true }),
      rm(outside, { recursive: true, force: true }),
    ]);
  }
});

test("publishes one App-authored commit and one draft PR without custom author fields", async () => {
  const sandbox = await mkdtemp(
    join(tmpdir(), "wyrmgrid-contribution-publish-"),
  );
  const root = join(sandbox, "repo");
  const remote = join(sandbox, "remote.git");
  const evidence = join(sandbox, "evidence");
  await Promise.all([mkdir(root), mkdir(evidence), mkdir(remote)]);
  try {
    git(remote, "init", "--bare");
    git(root, "init", "--initial-branch=main");
    git(root, "config", "user.name", "Test Maintainer");
    git(root, "config", "user.email", "maintainer@example.invalid");
    await mkdir(join(root, "src"));
    await writeFile(join(root, "src", "example.txt"), "old\n");
    git(root, "add", "src/example.txt");
    git(root, "commit", "-m", "initial");
    git(root, "remote", "add", "origin", remote);
    git(root, "push", "-u", "origin", "main");
    const baseSha = git(root, "rev-parse", "HEAD");

    const patchPath = join(evidence, "change.patch");
    const packetPath = join(evidence, "packet.md");
    const manifestPath = join(evidence, "manifest.json");
    const configPath = join(evidence, "github-app.json");
    const keyPath = join(sandbox, "github-app.pem");
    await Promise.all([
      writeFile(patchPath, modifiedPatch),
      writeFile(packetPath, "# Sanitized packet\n"),
    ]);
    const prepared = await prepareContributionManifest({
      patchPath,
      packetPath,
      outputPath: manifestPath,
      contribution: contribution(),
      root,
      now: new Date("2026-07-18T08:00:00Z"),
    });
    const { privateKey } = generateKeyPairSync("rsa", { modulusLength: 2048 });
    await writeFile(
      keyPath,
      privateKey.export({ type: "pkcs8", format: "pem" }),
    );
    await writeFile(
      configPath,
      JSON.stringify({
        schema_version: 1,
        kind: "optional-ai-github-app-installation",
        repository: "phobos-dthorga/onair-wyrmgrid",
        expected_app_slug: "wyrmgrid-hoardmind",
        app_id: "123",
        installation_id: "456",
        private_key_path: keyPath,
      }),
    );

    const calls = [];
    const commitSha = "c".repeat(40);
    const jsonResponse = (status, body) =>
      new Response(JSON.stringify(body), {
        status,
        headers: { "Content-Type": "application/json" },
      });
    const fakeFetch = async (url, options) => {
      const path = new URL(url).pathname;
      const body = options.body ? JSON.parse(options.body) : null;
      calls.push({
        path,
        method: options.method,
        authorization: options.headers.Authorization,
        body,
      });
      if (path === "/app")
        return jsonResponse(200, { slug: "wyrmgrid-hoardmind" });
      if (path.endsWith("/access_tokens"))
        return jsonResponse(201, { token: "installation-token" });
      if (path.includes("/git/ref/heads/assistant/"))
        return jsonResponse(404, { message: "Not Found" });
      if (path.endsWith("/git/ref/heads/main"))
        return jsonResponse(200, { object: { sha: baseSha } });
      if (path.endsWith("/git/blobs"))
        return jsonResponse(201, { sha: "a".repeat(40) });
      if (path.endsWith("/git/trees"))
        return jsonResponse(201, { sha: "b".repeat(40) });
      if (path.endsWith("/git/commits"))
        return jsonResponse(201, { sha: commitSha });
      if (path.endsWith("/git/refs"))
        return jsonResponse(201, { ref: body.ref });
      return jsonResponse(500, { message: "unexpected test endpoint" });
    };

    const pullRequests = [];
    const createPullRequestImpl = async (request) => {
      pullRequests.push(request);
      return "https://example.invalid/pr/1";
    };

    const result = await publishContribution({
      manifestPath,
      patchPath,
      configPath,
      expectedManifestSha256: prepared.manifest_sha256,
      root,
      fetchImpl: fakeFetch,
      createPullRequestImpl,
    });
    assert.equal(result.state, "draft_pull_request_created");
    assert.equal(result.commit_sha, commitSha);
    assert.equal(result.pull_request_url, "https://example.invalid/pr/1");
    const commitCall = calls.find(({ path }) => path.endsWith("/git/commits"));
    assert.equal("author" in commitCall.body, false);
    assert.equal("committer" in commitCall.body, false);
    assert.match(commitCall.body.message, /Generated-by: Hoardmind/);
    assert.deepEqual(
      calls.find(({ path }) => path.endsWith("/access_tokens")).body
        .permissions,
      { contents: "write" },
    );
    assert.equal(
      calls.some(({ path }) => path.endsWith("/pulls")),
      false,
    );
    assert.equal(pullRequests.length, 1);
    assert.match(pullRequests[0].title, /^\[Hoardmind\]/);
    assert.match(pullRequests[0].body, /App has no Pull requests permission/);
    assert.equal(
      calls.some(({ authorization }) => authorization.includes("PRIVATE KEY")),
      false,
    );
  } finally {
    await rm(sandbox, { recursive: true, force: true });
  }
});
