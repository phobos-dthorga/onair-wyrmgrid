import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  buildSquashCommitMessage,
  assertMergedCommitProvenance,
  landGeneratedContribution,
  previewGeneratedContributionLanding,
  validateLandingEvidence,
} from "./optional-ai-landing.mjs";
import {
  buildCommitMessage,
  sha256,
  validateContributionManifest,
} from "./optional-ai-contribution.mjs";

const headSha = "a".repeat(40);
const mergeSha = "b".repeat(40);

function manifest(overrides = {}) {
  return validateContributionManifest({
    schema_version: 1,
    kind: "optional-ai-generated-contribution",
    contribution_id: "hm-20260718-landing-test",
    assistant_id: "hoardmind",
    assistant_display_name: "Hoardmind",
    model: "qwen3-coder:30b",
    task_contract: "implementation-patch-v1",
    created_at: "2026-07-18T08:00:00.000Z",
    base_branch: "main",
    branch_name: "assistant/hoardmind/hm-20260718-landing-test",
    commit_subject: "test: preserve generated provenance",
    pull_request_title: "Preserve generated provenance",
    summary: "Adds one bounded generated regression test.",
    allowed_paths: ["apps/desktop/src/lib/example.test.ts"],
    packet_sha256: "1".repeat(64),
    patch_sha256: "2".repeat(64),
    metrics_sha256: "3".repeat(64),
    human_review_required: true,
    merge_authority: false,
    ...overrides,
  });
}

function config() {
  return {
    repository: "phobos-dthorga/onair-wyrmgrid",
    expected_app_slug: "wyrmgrid-hoardmind",
  };
}

function pullRequest(overrides = {}) {
  return {
    url: "https://github.com/phobos-dthorga/onair-wyrmgrid/pull/41",
    number: 41,
    state: "OPEN",
    isDraft: false,
    baseRefName: "main",
    headRefName: "assistant/hoardmind/hm-20260718-landing-test",
    headRefOid: headSha,
    mergeable: "MERGEABLE",
    mergeStateStatus: "CLEAN",
    commits: [{ oid: headSha }],
    ...overrides,
  };
}

function botCommit(document = manifest(), overrides = {}) {
  return {
    commit: { message: buildCommitMessage(document) },
    author: { login: "wyrmgrid-hoardmind[bot]", type: "Bot" },
    committer: { login: "wyrmgrid-hoardmind[bot]", type: "Bot" },
    ...overrides,
  };
}

test("builds and verifies a deterministic provenance-preserving squash message", () => {
  const document = manifest();
  const message = buildSquashCommitMessage(document, headSha, 41);
  assert.equal(message.subject, document.commit_subject);
  assert.match(
    message.body,
    /Optional-AI-Contribution: hm-20260718-landing-test/,
  );
  assert.match(message.body, new RegExp(`Original-Bot-Commit: ${headSha}`));
  assert.match(message.body, /Pull-Request: #41/);
  assert.match(message.body, /Metrics-SHA256: 3{64}/);
  assert.equal(
    assertMergedCommitProvenance(message.full_message, document, headSha, 41),
    true,
  );
  assert.throws(
    () =>
      assertMergedCommitProvenance(
        message.full_message.replace("Merge-Authority: none", ""),
        document,
        headSha,
        41,
      ),
    /does not preserve/,
  );
});

test("rejects branch drift, extra commits, non-clean state, and spoofed bot identity", () => {
  const document = manifest();
  const evidence = {
    manifest: document,
    config: config(),
    pullRequest: pullRequest(),
    botCommit: botCommit(document),
    expectedHeadSha: headSha,
    pullRequestNumber: 41,
  };
  assert.equal(
    validateLandingEvidence(evidence).subject,
    document.commit_subject,
  );
  assert.throws(
    () =>
      validateLandingEvidence({
        ...evidence,
        pullRequest: pullRequest({ headRefOid: "c".repeat(40) }),
      }),
    /exact head/,
  );
  assert.throws(
    () =>
      validateLandingEvidence({
        ...evidence,
        pullRequest: pullRequest({
          commits: [{ oid: headSha }, { oid: "c".repeat(40) }],
        }),
      }),
    /exactly the approved bot commit/,
  );
  assert.throws(
    () =>
      validateLandingEvidence({
        ...evidence,
        pullRequest: pullRequest({ mergeStateStatus: "BLOCKED" }),
      }),
    /not cleanly mergeable/,
  );
  assert.throws(
    () =>
      validateLandingEvidence({
        ...evidence,
        botCommit: botCommit(document, {
          author: { login: "phobos-dthorga", type: "User" },
          committer: { login: "phobos-dthorga", type: "User" },
        }),
      }),
    /not attributed/,
  );
});

test("previews and lands through an exact-head human squash without admin bypass", async () => {
  const sandbox = await mkdtemp(join(tmpdir(), "wyrmgrid-landing-"));
  const root = join(sandbox, "repository");
  const localRoot = join(root, ".wyrmgrid-local");
  const manifestPath = join(localRoot, "manifest.json");
  const configPath = join(localRoot, "github-app.json");
  const keyPath = join(sandbox, "private-key.pem");
  const document = manifest();
  const manifestBytes = `${JSON.stringify(document, null, 2)}\n`;
  try {
    await mkdir(localRoot, { recursive: true });
    await writeFile(manifestPath, manifestBytes);
    await writeFile(keyPath, "unused-test-key");
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
    let merged = false;
    const squash = buildSquashCommitMessage(document, headSha, 41);
    const ghImpl = async (args) => {
      calls.push(args);
      if (args[0] === "api" && args[1] === "user") {
        return { login: "phobos-dthorga", type: "User" };
      }
      if (args[0] === "api" && args[1].endsWith(`/commits/${headSha}`)) {
        return botCommit(document);
      }
      if (
        args[0] === "pr" &&
        args[1] === "view" &&
        args.some((argument) => argument.includes("commits"))
      ) {
        return pullRequest();
      }
      if (args[0] === "pr" && args[1] === "merge") {
        merged = true;
        return "";
      }
      if (
        args[0] === "pr" &&
        args[1] === "view" &&
        args.some((argument) => argument.includes("mergedAt"))
      ) {
        return {
          state: merged ? "MERGED" : "OPEN",
          mergedAt: "2026-07-18T09:00:00Z",
          mergeCommit: { oid: mergeSha },
          url: pullRequest().url,
        };
      }
      if (args[0] === "api" && args[1].endsWith(`/commits/${mergeSha}`)) {
        return { commit: { message: squash.full_message } };
      }
      throw new Error(`Unexpected fake gh call: ${args.join(" ")}`);
    };
    const common = {
      manifestPath,
      configPath,
      expectedManifestSha256: sha256(await readFile(manifestPath)),
      expectedHeadSha: headSha,
      pullRequestNumber: 41,
      root,
      ghImpl,
    };
    const preview = await previewGeneratedContributionLanding(common);
    assert.equal(preview.state, "landing_preview_ready");
    assert.equal(preview.human_actor, "phobos-dthorga");
    const result = await landGeneratedContribution({
      ...common,
      approveOnce: true,
    });
    assert.equal(result.state, "merged_and_provenance_verified");
    const mergeCall = calls.find(
      (args) => args[0] === "pr" && args[1] === "merge",
    );
    assert.ok(mergeCall.includes("--squash"));
    assert.ok(mergeCall.includes("--match-head-commit"));
    assert.ok(mergeCall.includes("--subject"));
    assert.ok(mergeCall.includes("--body"));
    assert.equal(mergeCall.includes("--admin"), false);
    assert.equal(mergeCall[mergeCall.indexOf("--body") + 1], squash.body);
  } finally {
    await rm(sandbox, { recursive: true, force: true });
  }
});

test("requires a fresh one-invocation landing approval", async () => {
  await assert.rejects(
    landGeneratedContribution({ approveOnce: false }),
    /requires --approve-once/,
  );
});
