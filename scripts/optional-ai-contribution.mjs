import { createHash, createSign } from "node:crypto";
import {
  access,
  mkdtemp,
  readFile,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, isAbsolute, relative, resolve, sep } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

export const CONTRIBUTION_SCHEMA_VERSION = 1;
export const APP_CONFIG_SCHEMA_VERSION = 1;
export const MAX_PATCH_BYTES = 2 * 1024 * 1024;
export const MAX_CHANGED_FILES = 64;
export const MAX_CHANGED_FILE_BYTES = 2 * 1024 * 1024;
export const MAX_TOTAL_CHANGED_FILE_BYTES = 8 * 1024 * 1024;

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const HEX_SHA256 = /^[a-f0-9]{64}$/;
const SAFE_ID = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const SAFE_BRANCH =
  /^assistant\/[a-z0-9]+(?:-[a-z0-9]+)*\/[a-z0-9]+(?:-[a-z0-9]+)*$/;
const SAFE_REPOSITORY = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/;
const SAFE_PATCH_PATH = /^[A-Za-z0-9_.@+/-]+$/;
const SENSITIVE_PATTERNS = [
  /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/,
  /\bgh[pousr]_[A-Za-z0-9]{20,}\b/,
  /\bgithub_pat_[A-Za-z0-9_]{20,}\b/,
  /\bsk-(?:proj-)?[A-Za-z0-9_-]{20,}\b/,
  /\bAKIA[A-Z0-9]{16}\b/,
  /\bAuthorization\s*:\s*Bearer\s+[A-Za-z0-9._~+/=-]{16,}\b/i,
  /\b(?:api[_-]?key|password|private[_-]?key|secret|token)\s*[:=]\s*["'][A-Za-z0-9_./+=-]{16,}["']/i,
];

export const PROTECTED_PATH_DESCRIPTIONS = Object.freeze([
  ".github/**",
  "AGENTS.md",
  "dependency and workspace manifests",
  "database migrations",
  "legal and security policy",
  "protocol/schema definitions (sanitized fixtures remain eligible)",
  "optional-AI policy, prompts, profiles, schemas, and brokers",
  "release automation",
]);

function fail(message) {
  throw new Error(message);
}

function isOutsideRoot(target, root) {
  const relativeTarget = relative(resolve(root), resolve(target));
  return (
    isAbsolute(relativeTarget) ||
    relativeTarget.startsWith(`..${sep}`) ||
    relativeTarget === ".."
  );
}

function runGit(args, { cwd = repositoryRoot, allowFailure = false } = {}) {
  const result = spawnSync("git", args, {
    cwd,
    encoding: "utf8",
    windowsHide: true,
  });
  if (result.error) {
    throw result.error;
  }
  if (!allowFailure && result.status !== 0) {
    fail(
      `Git command failed (${args[0]}): ${(result.stderr || result.stdout || "no diagnostic").trim()}`,
    );
  }
  return result;
}

export function sha256(content) {
  return createHash("sha256").update(content).digest("hex");
}

function decodeUtf8(content, label) {
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(content);
  } catch {
    fail(`${label} must be valid UTF-8 text.`);
  }
}

function base64Url(content) {
  return Buffer.from(content)
    .toString("base64")
    .replaceAll("=", "")
    .replaceAll("+", "-")
    .replaceAll("/", "_");
}

export function createGitHubAppJwt({ appId, privateKey, nowSeconds }) {
  const issuedAt = Math.floor(nowSeconds ?? Date.now() / 1000) - 60;
  const header = base64Url(JSON.stringify({ alg: "RS256", typ: "JWT" }));
  const payload = base64Url(
    JSON.stringify({
      iat: issuedAt,
      exp: issuedAt + 9 * 60,
      iss: String(appId),
    }),
  );
  const unsigned = `${header}.${payload}`;
  const signer = createSign("RSA-SHA256");
  signer.update(unsigned);
  signer.end();
  return `${unsigned}.${base64Url(signer.sign(privateKey))}`;
}

export function isProtectedContributionPath(path) {
  const lower = path.toLowerCase();
  const basename = lower.split("/").at(-1);
  return (
    lower === ".github" ||
    lower.startsWith(".github/") ||
    lower === "agents.md" ||
    lower === ".gitmodules" ||
    lower === ".gitattributes" ||
    lower === ".gitignore" ||
    lower === "changelog.md" ||
    lower === "cargo.toml" ||
    lower === "cargo.lock" ||
    lower === "package.json" ||
    lower === "package-lock.json" ||
    basename === "cargo.toml" ||
    lower === "apps/desktop/src-tauri/tauri.conf.json" ||
    lower.includes("/capabilities/") ||
    lower.includes("/migrations/") ||
    lower.startsWith("crates/onair-api/") ||
    lower.startsWith("crates/plugin-protocol/") ||
    lower.startsWith("crates/bridge-protocol/") ||
    lower.startsWith("locales/") ||
    lower === "docs/legal" ||
    lower.startsWith("docs/legal/") ||
    lower === "docs/security" ||
    lower.startsWith("docs/security/") ||
    lower === "docs/optional-ai" ||
    lower.startsWith("docs/optional-ai/") ||
    lower === "schemas" ||
    (lower.startsWith("schemas/") &&
      lower !== "schemas/fixtures" &&
      !lower.startsWith("schemas/fixtures/")) ||
    lower === "examples/optional-ai" ||
    lower.startsWith("examples/optional-ai/") ||
    lower === "docs/release-process.md" ||
    lower === "scripts" ||
    lower.startsWith("scripts/")
  );
}

export function normalizeContributionPath(value, label = "path") {
  if (typeof value !== "string" || !value || value.includes("\\")) {
    fail(`${label} must be a non-empty repository path using forward slashes.`);
  }
  if (
    value.startsWith("/") ||
    value.endsWith("/") ||
    value.includes("//") ||
    value.split("/").some((part) => part === "." || part === "..") ||
    !SAFE_PATCH_PATH.test(value)
  ) {
    fail(`${label} is not a safe normalized repository path: ${value}`);
  }
  return value;
}

function parseDiffHeader(line) {
  const match = /^diff --git a\/([^ ]+) b\/([^ ]+)$/.exec(line);
  if (!match) {
    fail("Patch paths must be unquoted, space-free, normalized Git paths.");
  }
  const from = normalizeContributionPath(match[1], "patch source path");
  const to = normalizeContributionPath(match[2], "patch destination path");
  if (from !== to) {
    fail(
      "Renames and path changes are not permitted in generated contributions.",
    );
  }
  return to;
}

export function parseGeneratedPatch(patch) {
  const patchBytes = Buffer.byteLength(patch);
  if (!patchBytes || patchBytes > MAX_PATCH_BYTES) {
    fail(`Patch must contain between 1 and ${MAX_PATCH_BYTES} bytes.`);
  }
  if (patch.includes("\0")) {
    fail("Binary patch data is not permitted.");
  }
  assertNoSensitiveContent(patch, "patch");

  const lines = patch.replaceAll("\r\n", "\n").split("\n");
  const changes = [];
  let current = null;

  const finishCurrent = () => {
    if (!current) return;
    if (!current.hasHunk) {
      fail(
        `Patch entry '${current.path}' does not contain a textual diff hunk.`,
      );
    }
    const expectedOldMarker = current.is_new
      ? "--- /dev/null"
      : `--- a/${current.path}`;
    if (
      current.oldMarker !== expectedOldMarker ||
      current.newMarker !== `+++ b/${current.path}`
    ) {
      fail(
        `Patch entry '${current.path}' has mismatched or non-canonical file markers.`,
      );
    }
    delete current.oldMarker;
    delete current.newMarker;
    changes.push(Object.freeze(current));
  };

  for (const line of lines) {
    if (line.startsWith("diff --git ")) {
      finishCurrent();
      current = {
        path: parseDiffHeader(line),
        is_new: false,
        hasHunk: false,
        oldMarker: null,
        newMarker: null,
      };
      continue;
    }
    if (!current) {
      if (line.trim()) {
        fail("Patch must begin with a canonical 'diff --git' header.");
      }
      continue;
    }
    if (line === "GIT binary patch" || line.startsWith("Binary files ")) {
      fail("Binary patches are not permitted.");
    }
    if (
      line.startsWith("deleted file mode ") ||
      line.startsWith("rename from ") ||
      line.startsWith("rename to ") ||
      line.startsWith("copy from ") ||
      line.startsWith("copy to ") ||
      line.startsWith("similarity index ") ||
      line.startsWith("old mode ") ||
      line.startsWith("new mode ")
    ) {
      fail(
        "Deletions, renames, copies, and file-mode changes are not permitted.",
      );
    }
    if (line.startsWith("new file mode ")) {
      if (line !== "new file mode 100644") {
        fail(
          "Generated files must be regular non-executable text files (100644).",
        );
      }
      current.is_new = true;
    }
    if (line.startsWith("--- ")) {
      if (current.oldMarker !== null) {
        fail(`Patch entry '${current.path}' has duplicate old-file markers.`);
      }
      current.oldMarker = line;
    }
    if (line.startsWith("+++ ")) {
      if (current.newMarker !== null) {
        fail(`Patch entry '${current.path}' has duplicate new-file markers.`);
      }
      current.newMarker = line;
    }
    if (line.startsWith("@@ ")) {
      current.hasHunk = true;
    }
  }
  finishCurrent();

  if (!changes.length || changes.length > MAX_CHANGED_FILES) {
    fail(`Patch must change between 1 and ${MAX_CHANGED_FILES} files.`);
  }
  const paths = changes.map(({ path }) => path);
  if (new Set(paths).size !== paths.length) {
    fail("Each generated path may appear in only one diff entry.");
  }
  for (const path of paths) {
    if (isProtectedContributionPath(path)) {
      fail(`Generated contribution targets protected path '${path}'.`);
    }
  }
  return Object.freeze(changes);
}

export function extractGeneratedPatchFromDraft(draft) {
  if (typeof draft !== "string" || Buffer.byteLength(draft) > 3 * 1024 * 1024) {
    fail("Generated draft must be bounded UTF-8 text.");
  }
  const headings = [...draft.matchAll(/^## Proposed patch[ \t]*$/gm)];
  if (headings.length !== 1) {
    fail(
      "Generated draft must contain exactly one '## Proposed patch' section.",
    );
  }
  const contentStart = headings[0].index + headings[0][0].length;
  const remainder = draft.slice(contentStart).replace(/^\r?\n/, "");
  const nextHeading = /^##[ \t]+/m.exec(remainder);
  const section = nextHeading
    ? remainder.slice(0, nextHeading.index)
    : remainder;
  const fences = [
    ...section.matchAll(/^```diff[ \t]*\r?\n([\s\S]*?)^```[ \t]*$/gm),
  ];
  if (fences.length !== 1 || section.trim() !== fences[0][0].trim()) {
    fail(
      "The proposed-patch section must contain exactly one fenced diff and no other content.",
    );
  }
  const patch = fences[0][1].endsWith("\n")
    ? fences[0][1]
    : `${fences[0][1]}\n`;
  parseGeneratedPatch(patch);
  return patch;
}

export function assertNoSensitiveContent(content, label) {
  for (const pattern of SENSITIVE_PATTERNS) {
    if (pattern.test(content)) {
      fail(
        `${label} contains a credential-like value and cannot be published.`,
      );
    }
  }
}

export function validateAllowedScopes(scopes) {
  if (!Array.isArray(scopes) || scopes.length === 0 || scopes.length > 32) {
    fail(
      "At least one and at most 32 reviewer-approved path scopes are required.",
    );
  }
  const normalizedScopes = scopes.map((scope) => {
    if (typeof scope !== "string" || !scope) {
      fail("Approved path scopes must be non-empty strings.");
    }
    const prefix = scope.endsWith("/");
    const normalized = normalizeContributionPath(
      prefix ? scope.slice(0, -1) : scope,
      "approved path scope",
    );
    if (isProtectedContributionPath(normalized)) {
      fail(`Approved path scope enters protected policy path '${scope}'.`);
    }
    return `${normalized}${prefix ? "/" : ""}`;
  });
  if (new Set(normalizedScopes).size !== normalizedScopes.length) {
    fail("Reviewer-approved path scopes must be unique.");
  }
  return Object.freeze(normalizedScopes);
}

export function assertChangesWithinScopes(changes, scopes) {
  const normalizedScopes = validateAllowedScopes(scopes);
  for (const { path } of changes) {
    const allowed = normalizedScopes.some((scope) =>
      scope.endsWith("/") ? path.startsWith(scope) : path === scope,
    );
    if (!allowed) {
      fail(`Generated path '${path}' is outside the reviewer-approved scopes.`);
    }
  }
  return normalizedScopes;
}

function boundedString(value, label, maximum, pattern) {
  if (typeof value !== "string" || !value.trim() || value.length > maximum) {
    fail(`${label} must contain between 1 and ${maximum} characters.`);
  }
  if (/\r|\n|[\u0000-\u001f\u007f]/u.test(value)) {
    fail(`${label} must be a single safe text line.`);
  }
  if (pattern && !pattern.test(value)) {
    fail(`${label} has an unsupported format.`);
  }
  return value.trim();
}

export function validateContributionManifest(document) {
  if (!document || typeof document !== "object" || Array.isArray(document)) {
    fail("Contribution manifest must be a JSON object.");
  }
  const allowedKeys = new Set([
    "$schema",
    "schema_version",
    "kind",
    "contribution_id",
    "assistant_id",
    "assistant_display_name",
    "model",
    "task_contract",
    "created_at",
    "base_branch",
    "branch_name",
    "commit_subject",
    "pull_request_title",
    "summary",
    "allowed_paths",
    "packet_sha256",
    "patch_sha256",
    "metrics_sha256",
    "human_review_required",
    "merge_authority",
  ]);
  for (const key of Object.keys(document)) {
    if (!allowedKeys.has(key))
      fail(`Unknown contribution manifest field '${key}'.`);
  }
  if (
    document.schema_version !== CONTRIBUTION_SCHEMA_VERSION ||
    document.kind !== "optional-ai-generated-contribution"
  ) {
    fail("Unsupported generated-contribution schema or kind.");
  }
  const normalized = {
    schema_version: 1,
    kind: document.kind,
    contribution_id: boundedString(
      document.contribution_id,
      "contribution_id",
      80,
      SAFE_ID,
    ),
    assistant_id: boundedString(
      document.assistant_id,
      "assistant_id",
      80,
      SAFE_ID,
    ),
    assistant_display_name: boundedString(
      document.assistant_display_name,
      "assistant_display_name",
      80,
    ),
    model: boundedString(document.model, "model", 200),
    task_contract: boundedString(
      document.task_contract,
      "task_contract",
      100,
      SAFE_ID,
    ),
    created_at: document.created_at,
    base_branch: boundedString(
      document.base_branch,
      "base_branch",
      100,
      SAFE_ID,
    ),
    branch_name: boundedString(
      document.branch_name,
      "branch_name",
      120,
      SAFE_BRANCH,
    ),
    commit_subject: boundedString(
      document.commit_subject,
      "commit_subject",
      72,
    ),
    pull_request_title: boundedString(
      document.pull_request_title,
      "pull_request_title",
      120,
    ),
    summary: boundedString(document.summary, "summary", 500),
    allowed_paths: validateAllowedScopes(document.allowed_paths),
    packet_sha256: document.packet_sha256,
    patch_sha256: document.patch_sha256,
    metrics_sha256: document.metrics_sha256 ?? null,
    human_review_required: document.human_review_required,
    merge_authority: document.merge_authority,
  };
  if (!Number.isFinite(Date.parse(normalized.created_at))) {
    fail("created_at must be an ISO 8601 date-time.");
  }
  if (normalized.base_branch !== "main") {
    fail("Generated contributions must target the reviewed main branch.");
  }
  const expectedBranch = `assistant/${normalized.assistant_id}/${normalized.contribution_id}`;
  if (normalized.branch_name !== expectedBranch) {
    fail(`branch_name must be the identity-bound branch '${expectedBranch}'.`);
  }
  for (const field of ["packet_sha256", "patch_sha256"]) {
    if (!HEX_SHA256.test(normalized[field]))
      fail(`${field} must be a SHA-256 digest.`);
  }
  if (
    normalized.metrics_sha256 !== null &&
    !HEX_SHA256.test(normalized.metrics_sha256)
  ) {
    fail("metrics_sha256 must be null or a SHA-256 digest.");
  }
  if (
    normalized.human_review_required !== true ||
    normalized.merge_authority !== false
  ) {
    fail(
      "Generated contributions must require human review and grant no merge authority.",
    );
  }
  return Object.freeze(normalized);
}

export function validateGitHubAppConfig(
  document,
  configPath,
  root = repositoryRoot,
) {
  if (!document || typeof document !== "object" || Array.isArray(document)) {
    fail("GitHub App configuration must be a JSON object.");
  }
  const keys = new Set([
    "$schema",
    "schema_version",
    "kind",
    "repository",
    "expected_app_slug",
    "app_id",
    "installation_id",
    "private_key_path",
  ]);
  for (const key of Object.keys(document)) {
    if (!keys.has(key))
      fail(`Unknown GitHub App configuration field '${key}'.`);
  }
  if (
    document.schema_version !== APP_CONFIG_SCHEMA_VERSION ||
    document.kind !== "optional-ai-github-app-installation"
  ) {
    fail("Unsupported GitHub App configuration schema or kind.");
  }
  if (!SAFE_REPOSITORY.test(document.repository ?? "")) {
    fail("repository must use the GitHub owner/name form.");
  }
  const configDirectory = dirname(resolve(configPath));
  const privateKeyPath = resolve(
    configDirectory,
    document.private_key_path ?? "",
  );
  if (!isOutsideRoot(privateKeyPath, root)) {
    fail("The GitHub App private key must be stored outside the repository.");
  }
  const appId = String(document.app_id ?? "");
  const installationId = String(document.installation_id ?? "");
  if (!/^[1-9]\d*$/.test(appId) || !/^[1-9]\d*$/.test(installationId)) {
    fail("app_id and installation_id must be positive decimal identifiers.");
  }
  return Object.freeze({
    repository: document.repository,
    expected_app_slug: boundedString(
      document.expected_app_slug,
      "expected_app_slug",
      100,
      SAFE_ID,
    ),
    app_id: appId,
    installation_id: installationId,
    private_key_path: privateKeyPath,
  });
}

function isPrivateLocalPath(path, root = repositoryRoot) {
  const target = resolve(path);
  const relativeTarget = relative(resolve(root), target);
  return (
    isOutsideRoot(target, root) ||
    relativeTarget === ".wyrmgrid-local" ||
    relativeTarget.startsWith(`.wyrmgrid-local${sep}`)
  );
}

function parseCli(argv) {
  const [command, ...rest] = argv;
  if (!new Set(["extract", "prepare", "publish"]).has(command)) {
    fail(
      "Usage: optional-ai-contribution.mjs <extract|prepare|publish> [options]",
    );
  }
  const values = new Map();
  const repeated = new Map([["allow", []]]);
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
    index += 1;
    if (repeated.has(name)) repeated.get(name).push(value);
    else if (values.has(name)) fail(`Duplicate --${name} option.`);
    else values.set(name, value);
  }
  return { command, values, repeated, approveOnce };
}

function requireOption(values, name) {
  const value = values.get(name);
  if (!value) fail(`Missing required --${name} option.`);
  return value;
}

function assertExplicitLocalApproval(approveOnce) {
  if (process.env.CI)
    fail("Generated contribution tooling must not run in CI.");
  if (!approveOnce)
    fail("Generated contribution tooling requires --approve-once.");
}

export async function prepareContributionManifest({
  patchPath,
  packetPath,
  metricsPath,
  outputPath,
  contribution,
  root = repositoryRoot,
  now = new Date(),
}) {
  if (!isPrivateLocalPath(outputPath, root)) {
    fail(
      "Contribution manifests must remain outside the repository or under .wyrmgrid-local/.",
    );
  }
  const [patch, packet, metrics] = await Promise.all([
    readFile(patchPath),
    readFile(packetPath),
    metricsPath ? readFile(metricsPath) : Promise.resolve(null),
  ]);
  const patchText = decodeUtf8(patch, "patch");
  const changes = parseGeneratedPatch(patchText);
  const allowedPaths = assertChangesWithinScopes(
    changes,
    contribution.allowed_paths,
  );
  const manifest = validateContributionManifest({
    schema_version: 1,
    kind: "optional-ai-generated-contribution",
    ...contribution,
    created_at: now.toISOString(),
    allowed_paths: allowedPaths,
    packet_sha256: sha256(packet),
    patch_sha256: sha256(patch),
    metrics_sha256: metrics ? sha256(metrics) : null,
    human_review_required: true,
    merge_authority: false,
  });
  const serialized = `${JSON.stringify(manifest, null, 2)}\n`;
  await writeFile(outputPath, serialized, { encoding: "utf8", flag: "wx" });
  return Object.freeze({
    manifest,
    manifest_sha256: sha256(serialized),
    changes,
  });
}

export function buildCommitMessage(manifest) {
  const metrics = manifest.metrics_sha256 ?? "not-retained";
  return `${manifest.commit_subject}\n\n${manifest.summary}\n\nGenerated-by: ${manifest.assistant_display_name}\nAssistant-ID: ${manifest.assistant_id}\nOptional-AI-Contribution: ${manifest.contribution_id}\nModel: ${manifest.model}\nTask-Contract: ${manifest.task_contract}\nInput-Packet-SHA256: ${manifest.packet_sha256}\nOutput-Patch-SHA256: ${manifest.patch_sha256}\nMetrics-SHA256: ${metrics}\nHuman-Review-Required: true\nMerge-Authority: none`;
}

export function buildPullRequestBody(manifest, changes, commitSha) {
  const files = changes.map(({ path }) => `- \`${path}\``).join("\n");
  return `## Generated contribution\n\n${manifest.summary}\n\nThe dedicated GitHub App published the bot-attributed commit and identity-bound branch from a hash-bound, reviewer-approved local patch. After its short-lived token was discarded, the human maintainer's GitHub CLI opened this draft PR. The assistant had no GitHub credentials or merge authority, and the App has no Pull requests permission.\n\n## Provenance\n\n| Field | Value |\n| --- | --- |\n| Assistant | ${manifest.assistant_display_name} (\`${manifest.assistant_id}\`) |\n| Contribution | \`${manifest.contribution_id}\` |\n| Model | \`${manifest.model}\` |\n| Task contract | \`${manifest.task_contract}\` |\n| Input packet SHA-256 | \`${manifest.packet_sha256}\` |\n| Output patch SHA-256 | \`${manifest.patch_sha256}\` |\n| Bot commit | \`${commitSha}\` |\n\n## Changed paths\n\n${files}\n\n## Required review\n\n- [ ] Reconcile every change against repository evidence.\n- [ ] Run the normal local quality gates.\n- [ ] Confirm changelog and compatibility impact.\n- [ ] Preserve the provenance trailers in the squash message.\n- [ ] Keep this PR as one independently revertable merge unit.\n\nThe generated commit and this PR are evidence, not approval. A human maintainer must decide whether and how to merge it.`;
}

async function githubRequest(
  fetchImpl,
  path,
  { token, method = "GET", body, allow404 = false } = {},
) {
  const response = await fetchImpl(`https://api.github.com${path}`, {
    method,
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${token}`,
      "X-GitHub-Api-Version": "2022-11-28",
      "User-Agent": "wyrmgrid-optional-ai-contribution-broker",
      ...(body ? { "Content-Type": "application/json" } : {}),
    },
    ...(body ? { body: JSON.stringify(body) } : {}),
  });
  if (allow404 && response.status === 404) return null;
  if (!response.ok) {
    fail(
      `GitHub request ${method} ${path} failed with HTTP ${response.status}. Response content was withheld.`,
    );
  }
  return response.status === 204 ? null : response.json();
}

async function mintInstallationToken({ config, privateKey, fetchImpl }) {
  const jwt = createGitHubAppJwt({ appId: config.app_id, privateKey });
  const app = await githubRequest(fetchImpl, "/app", { token: jwt });
  if (app.slug !== config.expected_app_slug) {
    fail(
      `Authenticated GitHub App slug '${app.slug}' does not match the approved configuration.`,
    );
  }
  const [, repositoryName] = config.repository.split("/");
  const installation = await githubRequest(
    fetchImpl,
    `/app/installations/${config.installation_id}/access_tokens`,
    {
      token: jwt,
      method: "POST",
      body: {
        repositories: [repositoryName],
        permissions: { contents: "write" },
      },
    },
  );
  if (typeof installation.token !== "string" || !installation.token) {
    fail("GitHub did not return an installation access token.");
  }
  return installation.token;
}

function assertCleanWorktree(root) {
  const status = runGit(["status", "--porcelain=v1", "--untracked-files=all"], {
    cwd: root,
  });
  if (status.stdout.trim()) fail("Publish requires a clean local worktree.");
}

function getBaseMode(root, baseSha, path, isNew) {
  if (isNew) return "100644";
  const result = runGit(["ls-tree", baseSha, "--", path], { cwd: root });
  const match = /^100644 blob [a-f0-9]+\t/.exec(result.stdout);
  if (!match)
    fail(
      `Existing path '${path}' is not a non-executable regular file at the approved base.`,
    );
  return "100644";
}

async function withPatchedWorktree({ root, baseSha, patchPath }, callback) {
  const worktree = await mkdtemp(
    resolve(tmpdir(), "wyrmgrid-generated-contribution-"),
  );
  let added = false;
  try {
    runGit(["worktree", "add", "--detach", worktree, baseSha], { cwd: root });
    added = true;
    runGit(["apply", "--check", "--whitespace=error-all", patchPath], {
      cwd: worktree,
    });
    runGit(["apply", "--whitespace=error-all", patchPath], { cwd: worktree });
    return await callback(worktree);
  } finally {
    if (added) {
      runGit(["worktree", "remove", "--force", worktree], {
        cwd: root,
        allowFailure: true,
      });
    }
    await rm(worktree, { recursive: true, force: true });
  }
}

export async function createMaintainerDraftPullRequest({
  repository,
  branch,
  base,
  title,
  body,
}) {
  const temporaryRoot = await mkdtemp(
    resolve(tmpdir(), "wyrmgrid-generated-pr-"),
  );
  const bodyPath = resolve(temporaryRoot, "pull-request.md");
  try {
    await writeFile(bodyPath, body, "utf8");
    const result = spawnSync(
      "gh",
      [
        "pr",
        "create",
        "--repo",
        repository,
        "--base",
        base,
        "--head",
        branch,
        "--title",
        title,
        "--body-file",
        bodyPath,
        "--draft",
      ],
      { encoding: "utf8", windowsHide: true },
    );
    if (result.error) throw result.error;
    if (result.status !== 0) {
      fail(
        `Maintainer draft-PR creation failed: ${(result.stderr || "no diagnostic").trim()}`,
      );
    }
    const url = result.stdout
      .trim()
      .split(/\r?\n/)
      .find((line) => /^https:\/\/github\.com\//.test(line));
    if (!url)
      fail("GitHub CLI did not return the created draft pull-request URL.");
    return url;
  } finally {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}

export async function publishContribution({
  manifestPath,
  patchPath,
  configPath,
  expectedManifestSha256,
  root = repositoryRoot,
  fetchImpl = fetch,
  createPullRequestImpl = createMaintainerDraftPullRequest,
}) {
  for (const privatePath of [manifestPath, patchPath, configPath]) {
    if (!isPrivateLocalPath(privatePath, root)) {
      fail(
        "Manifest, patch, and App configuration must remain outside the repository or under .wyrmgrid-local/.",
      );
    }
  }
  assertCleanWorktree(root);
  const [manifestBytes, patchBytes, configBytes] = await Promise.all([
    readFile(manifestPath),
    readFile(patchPath),
    readFile(configPath),
  ]);
  if (sha256(manifestBytes) !== expectedManifestSha256) {
    fail(
      "Contribution manifest no longer matches the explicitly approved SHA-256 digest.",
    );
  }
  const manifest = validateContributionManifest(JSON.parse(manifestBytes));
  const config = validateGitHubAppConfig(
    JSON.parse(configBytes),
    configPath,
    root,
  );
  if (sha256(patchBytes) !== manifest.patch_sha256) {
    fail("Patch no longer matches the approved contribution manifest.");
  }
  const changes = parseGeneratedPatch(decodeUtf8(patchBytes, "patch"));
  assertChangesWithinScopes(changes, manifest.allowed_paths);

  await access(config.private_key_path);
  const keyStats = await stat(config.private_key_path);
  if (!keyStats.isFile() || keyStats.size > 32 * 1024) {
    fail("GitHub App private key path must identify a bounded regular file.");
  }
  const privateKey = await readFile(config.private_key_path, "utf8");
  const installationToken = await mintInstallationToken({
    config,
    privateKey,
    fetchImpl,
  });
  const [owner, repository] = config.repository.split("/");
  const encodedBranch = manifest.branch_name
    .split("/")
    .map(encodeURIComponent)
    .join("/");
  const existingBranch = await githubRequest(
    fetchImpl,
    `/repos/${owner}/${repository}/git/ref/heads/${encodedBranch}`,
    { token: installationToken, allow404: true },
  );
  if (existingBranch)
    fail(`Remote branch '${manifest.branch_name}' already exists.`);

  const baseRef = await githubRequest(
    fetchImpl,
    `/repos/${owner}/${repository}/git/ref/heads/${encodeURIComponent(manifest.base_branch)}`,
    { token: installationToken },
  );
  const baseSha = baseRef?.object?.sha;
  if (!/^[a-f0-9]{40}$/.test(baseSha ?? ""))
    fail("GitHub returned an invalid base commit identity.");

  runGit(["fetch", "origin", manifest.base_branch], { cwd: root });
  const localBase = runGit(["rev-parse", `origin/${manifest.base_branch}`], {
    cwd: root,
  }).stdout.trim();
  if (localBase !== baseSha) {
    fail(
      "Local and GitHub base commits disagree; refusing to validate or publish the patch.",
    );
  }

  return withPatchedWorktree(
    { root, baseSha, patchPath: resolve(patchPath) },
    async (worktree) => {
      const treeEntries = [];
      let totalBytes = 0;
      for (const change of changes) {
        const content = await readFile(
          resolve(worktree, ...change.path.split("/")),
        );
        if (!content.length || content.length > MAX_CHANGED_FILE_BYTES) {
          fail(
            `Generated file '${change.path}' exceeds the per-file publication limit.`,
          );
        }
        totalBytes += content.length;
        if (totalBytes > MAX_TOTAL_CHANGED_FILE_BYTES) {
          fail(
            "Generated contribution exceeds the aggregate publication limit.",
          );
        }
        const contentText = decodeUtf8(
          content,
          `generated file '${change.path}'`,
        );
        if (
          /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/u.test(contentText)
        ) {
          fail(
            `Generated file '${change.path}' contains non-text control characters.`,
          );
        }
        assertNoSensitiveContent(
          contentText,
          `generated file '${change.path}'`,
        );
        const blob = await githubRequest(
          fetchImpl,
          `/repos/${owner}/${repository}/git/blobs`,
          {
            token: installationToken,
            method: "POST",
            body: { content: content.toString("base64"), encoding: "base64" },
          },
        );
        treeEntries.push({
          path: change.path,
          mode: getBaseMode(root, baseSha, change.path, change.is_new),
          type: "blob",
          sha: blob.sha,
        });
      }
      const tree = await githubRequest(
        fetchImpl,
        `/repos/${owner}/${repository}/git/trees`,
        {
          token: installationToken,
          method: "POST",
          body: { base_tree: baseSha, tree: treeEntries },
        },
      );
      const commit = await githubRequest(
        fetchImpl,
        `/repos/${owner}/${repository}/git/commits`,
        {
          token: installationToken,
          method: "POST",
          body: {
            message: buildCommitMessage(manifest),
            tree: tree.sha,
            parents: [baseSha],
          },
        },
      );
      await githubRequest(fetchImpl, `/repos/${owner}/${repository}/git/refs`, {
        token: installationToken,
        method: "POST",
        body: { ref: `refs/heads/${manifest.branch_name}`, sha: commit.sha },
      });
      const pullRequestUrl = await createPullRequestImpl({
        repository: config.repository,
        branch: manifest.branch_name,
        base: manifest.base_branch,
        title: `[${manifest.assistant_display_name}] ${manifest.pull_request_title}`,
        body: buildPullRequestBody(manifest, changes, commit.sha),
      });
      return Object.freeze({
        state: "draft_pull_request_created",
        repository: config.repository,
        branch: manifest.branch_name,
        base_sha: baseSha,
        commit_sha: commit.sha,
        pull_request_url: pullRequestUrl,
        manifest_sha256: expectedManifestSha256,
        patch_sha256: manifest.patch_sha256,
      });
    },
  );
}

async function main(argv) {
  const { command, values, repeated, approveOnce } = parseCli(argv);
  assertExplicitLocalApproval(approveOnce);
  if (command === "extract") {
    const outputPath = resolve(requireOption(values, "output"));
    if (!isPrivateLocalPath(outputPath)) {
      fail(
        "Extracted patches must remain outside the repository or under .wyrmgrid-local/.",
      );
    }
    const draft = await readFile(
      resolve(requireOption(values, "draft")),
      "utf8",
    );
    const patch = extractGeneratedPatchFromDraft(draft);
    await writeFile(outputPath, patch, { encoding: "utf8", flag: "wx" });
    process.stdout.write(
      `${JSON.stringify({ state: "patch_extracted", patch_sha256: sha256(patch), changed_paths: parseGeneratedPatch(patch).map(({ path }) => path) }, null, 2)}\n`,
    );
    return;
  }
  if (command === "prepare") {
    const result = await prepareContributionManifest({
      patchPath: resolve(requireOption(values, "patch")),
      packetPath: resolve(requireOption(values, "packet")),
      metricsPath: values.get("metrics")
        ? resolve(values.get("metrics"))
        : null,
      outputPath: resolve(requireOption(values, "output")),
      contribution: {
        contribution_id: requireOption(values, "contribution-id"),
        assistant_id: requireOption(values, "assistant-id"),
        assistant_display_name: requireOption(values, "assistant-name"),
        model: requireOption(values, "model"),
        task_contract: requireOption(values, "task-contract"),
        base_branch: values.get("base") ?? "main",
        branch_name: requireOption(values, "branch"),
        commit_subject: requireOption(values, "commit-subject"),
        pull_request_title: requireOption(values, "pr-title"),
        summary: requireOption(values, "summary"),
        allowed_paths: repeated.get("allow"),
      },
    });
    process.stdout.write(
      `${JSON.stringify({ state: "manifest_prepared", ...result, manifest: undefined }, null, 2)}\n`,
    );
    return;
  }
  const result = await publishContribution({
    manifestPath: resolve(requireOption(values, "manifest")),
    patchPath: resolve(requireOption(values, "patch")),
    configPath: resolve(requireOption(values, "config")),
    expectedManifestSha256: requireOption(values, "expected-manifest-sha256"),
  });
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
