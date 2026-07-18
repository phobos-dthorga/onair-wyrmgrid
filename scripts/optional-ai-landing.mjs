import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import {
  buildCommitMessage,
  isPrivateLocalPath,
  sha256,
  validateContributionManifest,
  validateGitHubAppConfig,
} from "./optional-ai-contribution.mjs";

const HEX_SHA256 = /^[a-f0-9]{64}$/;
const HEX_COMMIT_SHA = /^[a-f0-9]{40}$/;

function fail(message) {
  throw new Error(message);
}

function boundedPositiveInteger(value, label) {
  const normalized = String(value ?? "");
  if (!/^[1-9]\d*$/.test(normalized)) {
    fail(`${label} must be a positive decimal integer.`);
  }
  return Number(normalized);
}

function normalizeCommitMessage(message) {
  return String(message ?? "")
    .replaceAll("\r\n", "\n")
    .trimEnd();
}

export function buildSquashCommitMessage(
  manifest,
  botCommitSha,
  pullRequestNumber,
) {
  if (!HEX_COMMIT_SHA.test(botCommitSha ?? "")) {
    fail("Bot commit must be a full lowercase Git commit SHA.");
  }
  const prNumber = boundedPositiveInteger(
    pullRequestNumber,
    "Pull request number",
  );
  const metrics = manifest.metrics_sha256 ?? "not-retained";
  const subject = manifest.commit_subject;
  const body = `${manifest.summary}

Generated-by: ${manifest.assistant_display_name}
Assistant-ID: ${manifest.assistant_id}
Optional-AI-Contribution: ${manifest.contribution_id}
Model: ${manifest.model}
Task-Contract: ${manifest.task_contract}
Input-Packet-SHA256: ${manifest.packet_sha256}
Output-Patch-SHA256: ${manifest.patch_sha256}
Metrics-SHA256: ${metrics}
Original-Bot-Commit: ${botCommitSha}
Pull-Request: #${prNumber}
Human-Review-Required: true
Merge-Authority: none`;
  return Object.freeze({
    subject,
    body,
    full_message: `${subject}\n\n${body}`,
  });
}

export function assertMergedCommitProvenance(
  message,
  manifest,
  botCommitSha,
  pullRequestNumber,
) {
  const expected = buildSquashCommitMessage(
    manifest,
    botCommitSha,
    pullRequestNumber,
  ).full_message;
  if (normalizeCommitMessage(message) !== normalizeCommitMessage(expected)) {
    fail(
      "Merged squash commit does not preserve the approved provenance message.",
    );
  }
  return true;
}

function defaultGh(args) {
  const result = spawnSync("gh", args, {
    encoding: "utf8",
    windowsHide: true,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    fail(
      `GitHub CLI command failed (${args.slice(0, 2).join(" ")}): ${(result.stderr || "no diagnostic").trim()}`,
    );
  }
  return result.stdout;
}

async function ghJson(ghImpl, args) {
  const result = await ghImpl(args);
  if (typeof result === "string") {
    try {
      return JSON.parse(result);
    } catch {
      fail(
        `GitHub CLI returned malformed JSON for '${args.slice(0, 2).join(" ")}'.`,
      );
    }
  }
  return result;
}

export function validateLandingEvidence({
  manifest,
  config,
  pullRequest,
  botCommit,
  expectedHeadSha,
  pullRequestNumber,
}) {
  const prNumber = boundedPositiveInteger(
    pullRequestNumber,
    "Pull request number",
  );
  if (!HEX_COMMIT_SHA.test(expectedHeadSha ?? "")) {
    fail("Expected head must be a full lowercase Git commit SHA.");
  }
  if (
    pullRequest.number !== prNumber ||
    pullRequest.state !== "OPEN" ||
    pullRequest.isDraft !== false
  ) {
    fail(
      "Generated contribution PR must be the expected open, non-draft pull request.",
    );
  }
  if (
    pullRequest.baseRefName !== manifest.base_branch ||
    pullRequest.headRefName !== manifest.branch_name ||
    pullRequest.headRefOid !== expectedHeadSha
  ) {
    fail(
      "Generated contribution PR base, branch, or exact head does not match approval.",
    );
  }
  if (
    pullRequest.mergeable !== "MERGEABLE" ||
    pullRequest.mergeStateStatus !== "CLEAN"
  ) {
    fail(
      "Generated contribution PR is not cleanly mergeable through normal protection.",
    );
  }
  if (
    !Array.isArray(pullRequest.commits) ||
    pullRequest.commits.length !== 1 ||
    pullRequest.commits[0]?.oid !== expectedHeadSha
  ) {
    fail(
      "Generated contribution PR must contain exactly the approved bot commit.",
    );
  }
  if (
    normalizeCommitMessage(botCommit.commit?.message) !==
    buildCommitMessage(manifest)
  ) {
    fail(
      "Bot commit message does not match the hash-bound contribution manifest.",
    );
  }
  const expectedBotLogin = `${config.expected_app_slug}[bot]`.toLowerCase();
  const botActors = [botCommit.author, botCommit.committer]
    .filter(Boolean)
    .map(({ login, type }) => ({
      login: String(login ?? "").toLowerCase(),
      type,
    }));
  if (
    !botActors.some(
      ({ login, type }) => login === expectedBotLogin && type === "Bot",
    )
  ) {
    fail(
      `Approved head is not attributed to '${config.expected_app_slug}[bot]'.`,
    );
  }
  return buildSquashCommitMessage(manifest, expectedHeadSha, prNumber);
}

async function loadLandingInputs({
  manifestPath,
  configPath,
  expectedManifestSha256,
  root,
}) {
  if (
    !isPrivateLocalPath(manifestPath, root) ||
    !isPrivateLocalPath(configPath, root)
  ) {
    fail(
      "Landing manifest and App configuration must remain outside the repository or under .wyrmgrid-local/.",
    );
  }
  if (!HEX_SHA256.test(expectedManifestSha256 ?? "")) {
    fail("Expected manifest digest must be a lowercase SHA-256 value.");
  }
  const [manifestBytes, configBytes] = await Promise.all([
    readFile(manifestPath),
    readFile(configPath),
  ]);
  if (sha256(manifestBytes) !== expectedManifestSha256) {
    fail(
      "Landing manifest no longer matches the explicitly approved SHA-256 digest.",
    );
  }
  return Object.freeze({
    manifest: validateContributionManifest(JSON.parse(manifestBytes)),
    config: validateGitHubAppConfig(JSON.parse(configBytes), configPath, root),
  });
}

export async function previewGeneratedContributionLanding({
  manifestPath,
  configPath,
  expectedManifestSha256,
  expectedHeadSha,
  pullRequestNumber,
  root = resolve(fileURLToPath(new URL("..", import.meta.url))),
  ghImpl = defaultGh,
}) {
  const { manifest, config } = await loadLandingInputs({
    manifestPath,
    configPath,
    expectedManifestSha256,
    root,
  });
  const prNumber = boundedPositiveInteger(
    pullRequestNumber,
    "Pull request number",
  );
  const actor = await ghJson(ghImpl, ["api", "user"]);
  if (actor.type !== "User" || !actor.login || actor.login.endsWith("[bot]")) {
    fail("Landing requires an authenticated human GitHub user.");
  }
  const pullRequest = await ghJson(ghImpl, [
    "pr",
    "view",
    String(prNumber),
    "--repo",
    config.repository,
    "--json",
    "url,number,state,isDraft,baseRefName,headRefName,headRefOid,mergeable,mergeStateStatus,commits",
  ]);
  const botCommit = await ghJson(ghImpl, [
    "api",
    `repos/${config.repository}/commits/${expectedHeadSha}`,
  ]);
  const message = validateLandingEvidence({
    manifest,
    config,
    pullRequest,
    botCommit,
    expectedHeadSha,
    pullRequestNumber: prNumber,
  });
  return Object.freeze({
    state: "landing_preview_ready",
    repository: config.repository,
    pull_request_url: pullRequest.url,
    pull_request_number: prNumber,
    head_sha: expectedHeadSha,
    human_actor: actor.login,
    subject: message.subject,
    body: message.body,
  });
}

export async function landGeneratedContribution(options) {
  if (process.env.CI)
    fail("Generated contribution landing must not run in CI.");
  if (options.approveOnce !== true) {
    fail("Generated contribution landing requires --approve-once.");
  }
  const ghImpl = options.ghImpl ?? defaultGh;
  const preview = await previewGeneratedContributionLanding({
    ...options,
    ghImpl,
  });
  const mergeArgs = [
    "pr",
    "merge",
    String(preview.pull_request_number),
    "--repo",
    preview.repository,
    "--squash",
    "--match-head-commit",
    preview.head_sha,
    "--subject",
    preview.subject,
    "--body",
    preview.body,
  ];
  await ghImpl(mergeArgs);
  if (mergeArgs.includes("--admin")) {
    fail("Administrative merge bypass is forbidden.");
  }
  const landed = await ghJson(ghImpl, [
    "pr",
    "view",
    String(preview.pull_request_number),
    "--repo",
    preview.repository,
    "--json",
    "state,mergedAt,mergeCommit,url",
  ]);
  const mergeCommitSha = landed.mergeCommit?.oid;
  if (landed.state !== "MERGED" || !HEX_COMMIT_SHA.test(mergeCommitSha ?? "")) {
    fail("GitHub did not confirm an immediate protected squash merge.");
  }
  const mergedCommit = await ghJson(ghImpl, [
    "api",
    `repos/${preview.repository}/commits/${mergeCommitSha}`,
  ]);
  const { manifest } = await loadLandingInputs({
    manifestPath: options.manifestPath,
    configPath: options.configPath,
    expectedManifestSha256: options.expectedManifestSha256,
    root: options.root,
  });
  assertMergedCommitProvenance(
    mergedCommit.commit?.message,
    manifest,
    preview.head_sha,
    preview.pull_request_number,
  );
  return Object.freeze({
    state: "merged_and_provenance_verified",
    repository: preview.repository,
    pull_request_url: landed.url,
    head_sha: preview.head_sha,
    merge_commit_sha: mergeCommitSha,
    merged_at: landed.mergedAt,
    human_actor: preview.human_actor,
  });
}

function parseCli(argv) {
  const [command, ...rest] = argv;
  if (!new Set(["preview", "land"]).has(command)) {
    fail("Usage: optional-ai-landing.mjs <preview|land> [options]");
  }
  const values = new Map();
  let approveOnce = false;
  for (let index = 0; index < rest.length; index += 1) {
    const argument = rest[index];
    if (argument === "--approve-once") {
      approveOnce = true;
      continue;
    }
    if (!argument.startsWith("--")) fail(`Unexpected argument '${argument}'.`);
    const name = argument.slice(2);
    const value = rest[index + 1];
    if (!value || value.startsWith("--")) fail(`Missing value for --${name}.`);
    if (values.has(name)) fail(`Duplicate --${name} option.`);
    values.set(name, value);
    index += 1;
  }
  return { command, values, approveOnce };
}

function requireOption(values, name) {
  const value = values.get(name);
  if (!value) fail(`Missing required --${name} option.`);
  return value;
}

async function main(argv) {
  const { command, values, approveOnce } = parseCli(argv);
  const options = {
    manifestPath: resolve(requireOption(values, "manifest")),
    configPath: resolve(requireOption(values, "config")),
    expectedManifestSha256: requireOption(values, "expected-manifest-sha256"),
    expectedHeadSha: requireOption(values, "expected-head-sha"),
    pullRequestNumber: requireOption(values, "pr"),
  };
  const result =
    command === "preview"
      ? await previewGeneratedContributionLanding(options)
      : await landGeneratedContribution({ ...options, approveOnce });
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  main(process.argv.slice(2)).catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}
