import { createHash, randomUUID } from "node:crypto";
import {
  lstat,
  mkdir,
  open,
  realpath,
  rename,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { spawn } from "node:child_process";
import { setTimeout as delay } from "node:timers/promises";

export const EVIDENCE_SCHEMA_VERSION = 1;
export const EVIDENCE_KIND = "wyrmgrid-local-review-evidence";
export const CRITICAL_RULE_SET = "wyrmgrid-critical-paths-v1";

const MAX_GIT_OUTPUT_BYTES = 16 * 1024 * 1024;
const MAX_GIT_DURATION_MS = 30_000;
const MAX_HASH_FILE_BYTES = 128 * 1024 * 1024;
const MAX_HASH_DURATION_MS = 30_000;
const WINDOWS_RENAME_RETRY_DELAYS_MS = [
  25, 50, 100, 200, 400, 800, 1_600, 3_200,
];
const UTF8 = new TextDecoder("utf-8", { fatal: true });
const CHANGE_SOURCE_ORDER = [
  "base",
  "index",
  "worktree",
  "untracked",
  "unmerged",
  "submodule",
];

const CRITICAL_PATH_RULES = Object.freeze([
  {
    id: "critical-repository-governance",
    matches: (name) =>
      name === "AGENTS.md" ||
      name === "CONTRIBUTING.md" ||
      name === "GOVERNANCE.md" ||
      name.startsWith(".github/"),
  },
  {
    id: "critical-optional-ai-governance",
    matches: (name) =>
      name === "docs/operations/local-review-automation.md" ||
      name.startsWith("docs/optional-ai/") ||
      name.startsWith("examples/optional-ai/") ||
      name.startsWith("schemas/optional-ai-") ||
      name.startsWith("scripts/local-review") ||
      name.startsWith("scripts/run-optional-ai-task") ||
      name.startsWith("scripts/optional-ai-"),
  },
  {
    id: "critical-database-migration",
    matches: (name) => name.startsWith("crates/storage/migrations/"),
  },
  {
    id: "critical-protocol-or-schema",
    matches: (name) =>
      name.startsWith("schemas/") ||
      name.startsWith("crates/plugin-protocol/") ||
      name.startsWith("crates/bridge-protocol/") ||
      name === "docs/plugins/protocol-v1.md" ||
      name === "docs/integrations/wyrmgrid-bridge.md",
  },
  {
    id: "critical-security-privacy-or-credentials",
    matches: (name) =>
      name === "SECURITY.md" ||
      name.startsWith("docs/security/") ||
      name.startsWith("docs/legal/") ||
      /(?:^|\/)(?:authorization|credentials?|credential_store|database_key|data_protection|cryptography)(?:\.|\/|$)/.test(
        name,
      ),
  },
  {
    id: "critical-release-ci-or-installer",
    matches: (name) =>
      name === "CHANGELOG.md" ||
      name === "docs/release-process.md" ||
      name === "docs/operations/ci-cd-enhancement-plan.md" ||
      name.startsWith("scripts/prepare-release-notes") ||
      name.startsWith("scripts/release-workflow") ||
      name.startsWith("scripts/select-previous-release") ||
      name.startsWith("scripts/verify-installer-contract") ||
      name.startsWith("scripts/verify-release-version") ||
      name.startsWith("scripts/test-nsis-installer") ||
      name.startsWith("apps/desktop/src-tauri/tauri"),
  },
  {
    id: "critical-dependency-or-toolchain",
    matches: (name) =>
      name === "Cargo.toml" ||
      name === "Cargo.lock" ||
      name === "package.json" ||
      name === "package-lock.json" ||
      name === "deny.toml" ||
      name === "rust-toolchain.toml" ||
      name.endsWith("/Cargo.toml") ||
      name.endsWith("/package.json"),
  },
  {
    id: "critical-live-provider-claim",
    matches: (name) =>
      name.startsWith("crates/onair-api/") ||
      name.startsWith("crates/simbrief-api/") ||
      name.startsWith("providers/") ||
      name.startsWith("plugins/") ||
      name.startsWith("docs/integrations/") ||
      name.startsWith("docs/onair/"),
  },
]);

class LocalReviewError extends Error {
  constructor(code, message, options) {
    super(message, options);
    this.name = "LocalReviewError";
    this.code = code;
  }
}

function decodeUtf8(buffer, label) {
  try {
    return UTF8.decode(buffer);
  } catch (error) {
    throw new LocalReviewError(
      "unsupported-path-encoding",
      `${label} contained a path that was not valid UTF-8.`,
      { cause: error },
    );
  }
}

function splitNul(buffer, label) {
  if (buffer.length === 0) return [];
  const parts = [];
  let start = 0;
  for (let index = 0; index < buffer.length; index += 1) {
    if (buffer[index] !== 0) continue;
    parts.push(decodeUtf8(buffer.subarray(start, index), label));
    start = index + 1;
  }
  if (start !== buffer.length) {
    throw new LocalReviewError(
      "malformed-git-output",
      `${label} was not terminated by NUL.`,
    );
  }
  return parts;
}

function splitFixedPrefix(value, separatorCount, label) {
  const fields = [];
  let start = 0;
  for (let count = 0; count < separatorCount; count += 1) {
    const separator = value.indexOf(" ", start);
    if (separator < 0) {
      throw new LocalReviewError(
        "malformed-git-output",
        `${label} did not contain the expected fields.`,
      );
    }
    fields.push(value.slice(start, separator));
    start = separator + 1;
  }
  fields.push(value.slice(start));
  return fields;
}

export function normalizeRepositoryPath(value) {
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    value.includes("\0") ||
    /[\u0000-\u001f\u007f]/.test(value) ||
    /[\u2028\u2029\u202a-\u202e\u2066-\u2069]/u.test(value) ||
    value.includes("\\") ||
    path.posix.isAbsolute(value) ||
    /^[A-Za-z]:/.test(value)
  ) {
    throw new LocalReviewError(
      "unsafe-repository-path",
      "Git returned an unsafe repository path.",
    );
  }
  const normalized = path.posix.normalize(value);
  if (
    normalized === "." ||
    normalized === ".." ||
    normalized.startsWith("../") ||
    normalized !== value
  ) {
    throw new LocalReviewError(
      "unsafe-repository-path",
      "Git returned a path outside the selected repository scope.",
    );
  }
  return normalized;
}

export function parsePorcelainV2(buffer) {
  const tokens = splitNul(buffer, "Git status");
  const records = [];
  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token.startsWith("1 ")) {
      const fields = splitFixedPrefix(token, 8, "ordinary status record");
      records.push({
        type: "ordinary",
        xy: fields[1],
        submodule: fields[2],
        path: normalizeRepositoryPath(fields[8]),
        old_path: null,
      });
    } else if (token.startsWith("2 ")) {
      const fields = splitFixedPrefix(token, 9, "rename status record");
      const oldPath = tokens[index + 1];
      if (oldPath === undefined) {
        throw new LocalReviewError(
          "malformed-git-output",
          "Git rename status omitted its original path.",
        );
      }
      index += 1;
      records.push({
        type: "rename",
        xy: fields[1],
        submodule: fields[2],
        path: normalizeRepositoryPath(fields[9]),
        old_path: normalizeRepositoryPath(oldPath),
      });
    } else if (token.startsWith("u ")) {
      const fields = splitFixedPrefix(token, 10, "unmerged status record");
      records.push({
        type: "unmerged",
        xy: fields[1],
        submodule: fields[2],
        path: normalizeRepositoryPath(fields[10]),
        old_path: null,
      });
    } else if (token.startsWith("? ")) {
      records.push({
        type: "untracked",
        xy: "??",
        submodule: "N...",
        path: normalizeRepositoryPath(token.slice(2)),
        old_path: null,
      });
    } else if (token.startsWith("! ")) {
      continue;
    } else {
      throw new LocalReviewError(
        "malformed-git-output",
        "Git status returned an unsupported record type.",
      );
    }
  }
  return records;
}

export function parseNameStatus(buffer) {
  const tokens = splitNul(buffer, "Git name status");
  const records = [];
  for (let index = 0; index < tokens.length;) {
    const status = tokens[index];
    index += 1;
    if (!/^[ACDMRTUXB][0-9]*$/.test(status)) {
      throw new LocalReviewError(
        "malformed-git-output",
        "Git diff returned an unsupported change status.",
      );
    }
    const code = status[0];
    if (code === "R" || code === "C") {
      const oldPath = tokens[index];
      const newPath = tokens[index + 1];
      if (oldPath === undefined || newPath === undefined) {
        throw new LocalReviewError(
          "malformed-git-output",
          "Git diff omitted a rename or copy path.",
        );
      }
      index += 2;
      records.push({
        status,
        path: normalizeRepositoryPath(newPath),
        old_path: normalizeRepositoryPath(oldPath),
      });
    } else {
      const name = tokens[index];
      if (name === undefined) {
        throw new LocalReviewError(
          "malformed-git-output",
          "Git diff omitted a changed path.",
        );
      }
      index += 1;
      records.push({
        status,
        path: normalizeRepositoryPath(name),
        old_path: null,
      });
    }
  }
  return records;
}

export function parseNumstat(buffer) {
  const records = [];
  for (const token of splitNul(buffer, "Git numstat")) {
    const firstTab = token.indexOf("\t");
    const secondTab = token.indexOf("\t", firstTab + 1);
    if (firstTab < 0 || secondTab < 0) {
      throw new LocalReviewError(
        "malformed-git-output",
        "Git numstat returned an unsupported record.",
      );
    }
    const added = token.slice(0, firstTab);
    const deleted = token.slice(firstTab + 1, secondTab);
    if (!(
      (added === "-" && deleted === "-") ||
      (/^\d+$/.test(added) && /^\d+$/.test(deleted))
    )) {
      throw new LocalReviewError(
        "malformed-git-output",
        "Git numstat returned invalid line counts.",
      );
    }
    records.push({
      path: normalizeRepositoryPath(token.slice(secondTab + 1)),
      binary: added === "-",
    });
  }
  return records;
}

export function parseTrackedModes(buffer) {
  const modes = new Map();
  for (const token of splitNul(buffer, "Git tracked modes")) {
    const match = token.match(/^([0-7]{6}) [0-9a-f]+ [0-3]\t([\s\S]+)$/);
    if (!match) {
      throw new LocalReviewError(
        "malformed-git-output",
        "Git index returned an unsupported mode record.",
      );
    }
    const name = normalizeRepositoryPath(match[2]);
    const prior = modes.get(name);
    if (prior && prior !== match[1]) {
      modes.set(name, "unmerged");
    } else {
      modes.set(name, match[1]);
    }
  }
  return modes;
}

export function criticalRuleIds(name) {
  const normalized = normalizeRepositoryPath(name);
  return CRITICAL_PATH_RULES.filter((rule) => rule.matches(normalized))
    .map((rule) => rule.id)
    .sort();
}

export function candidateId(name) {
  const normalized = normalizeRepositoryPath(name);
  return `lr1-${createHash("sha256")
    .update("wyrmgrid-local-review-candidate-v1\0")
    .update(normalized)
    .digest("hex")
    .slice(0, 24)}`;
}

function emptyEntry(name) {
  return {
    path: name,
    old_path: null,
    change_sources: new Set(),
    base_status: null,
    index_status: null,
    worktree_status: null,
    untracked: false,
    conflict: false,
    status_submodule: false,
    binary: "unknown",
  };
}

function entryFor(entries, name) {
  const normalized = normalizeRepositoryPath(name);
  if (!entries.has(normalized)) entries.set(normalized, emptyEntry(normalized));
  return entries.get(normalized);
}

function applyStatusRecords(entries, records) {
  for (const record of records) {
    const entry = entryFor(entries, record.path);
    entry.old_path = record.old_path ?? entry.old_path;
    entry.status_submodule ||= record.submodule !== "N...";
    if (record.type === "untracked") {
      entry.untracked = true;
      entry.change_sources.add("untracked");
      continue;
    }
    if (record.type === "unmerged") {
      entry.conflict = true;
      entry.change_sources.add("unmerged");
    }
    const [indexStatus, worktreeStatus] = record.xy;
    if (indexStatus && indexStatus !== ".") {
      entry.index_status = indexStatus;
      entry.change_sources.add("index");
    }
    if (worktreeStatus && worktreeStatus !== ".") {
      entry.worktree_status = worktreeStatus;
      entry.change_sources.add("worktree");
    }
    if (record.submodule !== "N...") entry.change_sources.add("submodule");
  }
}

function applyBaseRecords(entries, records) {
  for (const record of records) {
    const entry = entryFor(entries, record.path);
    entry.old_path = record.old_path ?? entry.old_path;
    entry.base_status = record.status;
    entry.change_sources.add("base");
  }
}

function applyBinaryRecords(entries, records) {
  for (const record of records) {
    const entry = entries.get(record.path);
    if (!entry) continue;
    if (record.binary) entry.binary = "yes";
    else if (entry.binary !== "yes") entry.binary = "no";
  }
}

function sortChangeSources(sources) {
  return [...sources].sort(
    (left, right) =>
      CHANGE_SOURCE_ORDER.indexOf(left) - CHANGE_SOURCE_ORDER.indexOf(right),
  );
}

function sameFileIdentity(left, right) {
  return (
    left.dev === right.dev &&
    left.ino === right.ino &&
    left.size === right.size &&
    left.mtimeNs === right.mtimeNs
  );
}

function gitEnvironment() {
  const environment = { ...process.env };
  for (const name of Object.keys(environment)) {
    const upper = name.toUpperCase();
    if (
      upper.startsWith("GIT_") ||
      upper.startsWith("GCM_") ||
      upper === "SSH_ASKPASS"
    ) {
      delete environment[name];
    }
  }
  return {
    ...environment,
    GIT_OPTIONAL_LOCKS: "0",
    GIT_TERMINAL_PROMPT: "0",
    GCM_INTERACTIVE: "Never",
    GIT_PAGER: "cat",
    PAGER: "cat",
  };
}

export async function hashRegularFile(filePath, options = {}) {
  const handle = await open(filePath, "r");
  try {
    const before = await handle.stat({ bigint: true });
    if (!before.isFile()) {
      throw new LocalReviewError(
        "unsupported-file-kind",
        "Selected evidence was not a regular file.",
      );
    }
    if (before.size > BigInt(MAX_HASH_FILE_BYTES)) {
      throw new LocalReviewError(
        "unsupported-file-size",
        "Selected evidence exceeded the supported file size.",
      );
    }
    const digest = createHash("sha256");
    const controller = new AbortController();
    let timedOut = false;
    const timeout = setTimeout(() => {
      timedOut = true;
      controller.abort();
    }, MAX_HASH_DURATION_MS);
    try {
      for await (const chunk of handle.createReadStream({
        autoClose: false,
        signal: controller.signal,
      })) {
        digest.update(chunk);
      }
    } catch (error) {
      if (timedOut) {
        throw new LocalReviewError(
          "source-hash-timeout",
          "Selected evidence exceeded the local hashing time limit.",
          { cause: error },
        );
      }
      throw error;
    } finally {
      clearTimeout(timeout);
    }
    if (options.afterRead) await options.afterRead();
    const [afterHandle, afterPath] = await Promise.all([
      handle.stat({ bigint: true }),
      stat(filePath, { bigint: true }),
    ]);
    if (
      !sameFileIdentity(before, afterHandle) ||
      !sameFileIdentity(before, afterPath)
    ) {
      throw new LocalReviewError(
        "source-changed-during-inventory",
        "Selected evidence changed while it was being hashed.",
      );
    }
    return {
      size_bytes: Number(afterHandle.size),
      sha256: digest.digest("hex"),
    };
  } finally {
    await handle.close();
  }
}

function absoluteSelectedPath(repositoryRoot, name) {
  const selected = path.resolve(repositoryRoot, ...name.split("/"));
  const relative = path.relative(repositoryRoot, selected);
  if (
    relative === "" ||
    relative === ".." ||
    relative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relative)
  ) {
    throw new LocalReviewError(
      "unsafe-repository-path",
      "Selected evidence escaped the repository root.",
    );
  }
  return selected;
}

async function inspectEntry(repositoryRoot, entry, trackedModes, hashFile) {
  const mode = trackedModes.get(entry.path) ?? null;
  const submodule = entry.status_submodule || mode === "160000";
  const result = {
    path: entry.path,
    old_path: entry.old_path,
    change_sources: sortChangeSources(entry.change_sources),
    base_status: entry.base_status,
    index_status: entry.index_status,
    worktree_status: entry.worktree_status,
    untracked: entry.untracked,
    conflict: entry.conflict,
    binary: entry.binary,
    submodule: submodule ? "yes" : mode ? "no" : "unknown",
    exists: false,
    kind: "missing",
    size_bytes: null,
    sha256: null,
    hash_status: "not-applicable",
    unavailable_reason: null,
    candidate_id: candidateId(entry.path),
  };

  const selectedPath = absoluteSelectedPath(repositoryRoot, entry.path);
  let fileStatus;
  try {
    fileStatus = await lstat(selectedPath);
  } catch (error) {
    if (error?.code === "ENOENT") {
      const expectedDeletion =
        entry.index_status === "D" ||
        entry.worktree_status === "D" ||
        entry.base_status?.startsWith("D");
      if (!expectedDeletion) {
        result.hash_status = "unavailable";
        result.unavailable_reason = "source-missing-during-inventory";
      }
      return result;
    }
    result.hash_status = "unavailable";
    result.unavailable_reason = "source-metadata-unavailable";
    return result;
  }

  result.exists = true;
  if (submodule) {
    result.kind = "submodule";
    result.binary = "unknown";
    return result;
  }
  if (fileStatus.isSymbolicLink()) {
    result.kind = "unsupported";
    result.hash_status = "unavailable";
    result.unavailable_reason = "symbolic-link-not-followed";
    return result;
  }
  if (!fileStatus.isFile()) {
    result.kind = "unsupported";
    result.hash_status = "unavailable";
    result.unavailable_reason = "unsupported-file-kind";
    return result;
  }

  result.kind = "file";
  try {
    const canonicalPath = await realpath(selectedPath);
    const relativeCanonical = path.relative(repositoryRoot, canonicalPath);
    if (
      relativeCanonical === "" ||
      relativeCanonical === ".." ||
      relativeCanonical.startsWith(`..${path.sep}`) ||
      path.isAbsolute(relativeCanonical)
    ) {
      throw new LocalReviewError(
        "path-escape",
        "Selected evidence resolved outside the repository root.",
      );
    }
    const hashed = await hashFile(canonicalPath);
    result.size_bytes = hashed.size_bytes;
    result.sha256 = hashed.sha256;
    result.hash_status = "hashed";
  } catch (error) {
    result.hash_status = "unavailable";
    result.unavailable_reason =
      error instanceof LocalReviewError
        ? error.code
        : "source-content-unavailable";
  }
  return result;
}

export async function runGit(repositoryRoot, args) {
  return await new Promise((resolvePromise, rejectPromise) => {
    const hardenedArgs = [
      "-c",
      "core.fsmonitor=false",
      "-c",
      "core.untrackedCache=false",
      ...args,
    ];
    const child = spawn("git", hardenedArgs, {
      cwd: repositoryRoot,
      env: gitEnvironment(),
      shell: false,
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    });
    const stdout = [];
    let stdoutBytes = 0;
    let stderrBytes = 0;
    let exceeded = false;
    let timedOut = false;
    const timeout = setTimeout(() => {
      timedOut = true;
      child.kill();
    }, MAX_GIT_DURATION_MS);

    child.stdout.on("data", (chunk) => {
      stdoutBytes += chunk.length;
      if (stdoutBytes > MAX_GIT_OUTPUT_BYTES) {
        exceeded = true;
        child.kill();
      } else {
        stdout.push(chunk);
      }
    });
    child.stderr.on("data", (chunk) => {
      stderrBytes += chunk.length;
      if (stderrBytes > MAX_GIT_OUTPUT_BYTES) {
        exceeded = true;
        child.kill();
      }
    });
    child.on("error", (error) => {
      clearTimeout(timeout);
      rejectPromise(
        new LocalReviewError("git-unavailable", "Git could not be started.", {
          cause: error,
        }),
      );
    });
    child.on("close", (code) => {
      clearTimeout(timeout);
      if (timedOut) {
        rejectPromise(
          new LocalReviewError(
            "git-timeout",
            "Git evidence exceeded the local inventory time limit.",
          ),
        );
        return;
      }
      if (exceeded) {
        rejectPromise(
          new LocalReviewError(
            "git-output-too-large",
            "Git evidence exceeded the local inventory limit.",
          ),
        );
        return;
      }
      resolvePromise({
        ok: code === 0,
        code,
        stdout: Buffer.concat(stdout),
      });
    });
  });
}

async function requireRepositoryRoot(repositoryRoot, git) {
  const requestedRoot = await realpath(path.resolve(repositoryRoot));
  const result = await git(requestedRoot, [
    "-c",
    "core.quotepath=false",
    "rev-parse",
    "--show-toplevel",
  ]);
  if (!result.ok) {
    throw new LocalReviewError(
      "not-a-repository",
      "The selected directory is not a Git repository root.",
    );
  }
  const reported = decodeUtf8(result.stdout, "Git repository root").trim();
  const actualRoot = await realpath(reported);
  const equal =
    process.platform === "win32"
      ? actualRoot.toLowerCase() === requestedRoot.toLowerCase()
      : actualRoot === requestedRoot;
  if (!equal) {
    throw new LocalReviewError(
      "repository-root-required",
      "Run the inventory from the exact Git repository root.",
    );
  }
  return actualRoot;
}

function commitFromResult(result) {
  if (!result.ok) return null;
  const commit = decodeUtf8(result.stdout, "Git commit identity").trim();
  return /^[0-9a-f]{40,64}$/.test(commit) ? commit : null;
}

async function collectGitEvidence(repositoryRoot, baseRef, git) {
  const unavailable = [];
  const headResult = await git(repositoryRoot, [
    "rev-parse",
    "--verify",
    "HEAD",
  ]);
  const headCommit = commitFromResult(headResult);
  if (!headCommit) unavailable.push("head-unavailable");

  let baseCommit = null;
  let baseStatus = "not-requested";
  if (baseRef !== undefined) {
    const baseResult = await git(repositoryRoot, [
      "rev-parse",
      "--verify",
      "--end-of-options",
      `${baseRef}^{commit}`,
    ]);
    baseCommit = commitFromResult(baseResult);
    baseStatus = baseCommit ? "available" : "unavailable";
    if (!baseCommit) unavailable.push("base-unavailable");
  }

  const statusResult = await git(repositoryRoot, [
    "-c",
    "core.quotepath=false",
    "status",
    "--porcelain=v2",
    "-z",
    "--untracked-files=all",
  ]);
  const modesResult = await git(repositoryRoot, [
    "-c",
    "core.quotepath=false",
    "ls-files",
    "-s",
    "-z",
  ]);
  if (!statusResult.ok) unavailable.push("status-unavailable");
  if (!modesResult.ok) unavailable.push("tracked-modes-unavailable");

  const baseDiffResult = baseCommit
    ? await git(repositoryRoot, [
        "-c",
        "core.quotepath=false",
        "diff",
        "--no-ext-diff",
        "--no-textconv",
        "--name-status",
        "-z",
        "--find-renames",
        baseCommit,
        "--",
      ])
    : null;
  if (baseCommit && !baseDiffResult.ok)
    unavailable.push("base-diff-unavailable");

  const numstatCommands = baseCommit
    ? [
        [
          "-c",
          "core.quotepath=false",
          "diff",
          "--no-ext-diff",
          "--no-textconv",
          "--numstat",
          "-z",
          "--no-renames",
          baseCommit,
          "--",
        ],
      ]
    : [
        [
          "-c",
          "core.quotepath=false",
          "diff",
          "--no-ext-diff",
          "--no-textconv",
          "--numstat",
          "-z",
          "--no-renames",
          "--",
        ],
        [
          "-c",
          "core.quotepath=false",
          "diff",
          "--no-ext-diff",
          "--no-textconv",
          "--cached",
          "--numstat",
          "-z",
          "--no-renames",
          "--",
        ],
      ];
  const numstatResults = [];
  for (const command of numstatCommands) {
    const result = await git(repositoryRoot, command);
    if (!result.ok) unavailable.push("binary-status-unavailable");
    else numstatResults.push(result);
  }

  return {
    headCommit,
    baseCommit,
    baseStatus,
    statusResult,
    modesResult,
    baseDiffResult,
    numstatResults,
    unavailable: [...new Set(unavailable)].sort(),
  };
}

function countsFor(files) {
  return {
    selected_files: files.length,
    staged: files.filter((file) => file.index_status !== null).length,
    unstaged: files.filter((file) => file.worktree_status !== null).length,
    untracked: files.filter((file) => file.untracked).length,
    renamed: files.filter((file) => file.old_path !== null).length,
    deleted: files.filter((file) => !file.exists).length,
    binary: files.filter((file) => file.binary === "yes").length,
    submodules: files.filter((file) => file.submodule === "yes").length,
    unavailable: files.filter((file) => file.hash_status === "unavailable")
      .length,
  };
}

function candidateFor(file) {
  const ruleIds = criticalRuleIds(file.path);
  const classification =
    file.hash_status === "unavailable" ||
    file.kind === "unsupported" ||
    file.conflict ||
    file.submodule === "yes"
      ? "classification-required"
      : ruleIds.length > 0
        ? "critical-candidate"
        : "routine-candidate";
  return {
    id: file.candidate_id,
    path: file.path,
    classification,
    rule_ids: ruleIds,
  };
}

function classificationFor(candidates, scopeStatus, counts) {
  const critical = candidates.filter(
    (candidate) => candidate.classification === "critical-candidate",
  ).length;
  const routine = candidates.filter(
    (candidate) => candidate.classification === "routine-candidate",
  ).length;
  const required = candidates.filter(
    (candidate) => candidate.classification === "classification-required",
  ).length;
  let status = "no-candidates";
  if (scopeStatus === "unavailable" || counts.unavailable > 0 || required > 0) {
    status = "classification-required";
  } else if (critical > 0) {
    status = "critical-candidate";
  } else if (routine > 0) {
    status = "routine-candidate";
  }
  return {
    status,
    critical_candidates: critical,
    routine_candidates: routine,
    classification_required: required,
    rule_ids: [
      ...new Set(candidates.flatMap((candidate) => candidate.rule_ids)),
    ].sort(),
  };
}

function assertPlainObject(value, label) {
  if (
    value === null ||
    typeof value !== "object" ||
    Array.isArray(value) ||
    Object.getPrototypeOf(value) !== Object.prototype
  ) {
    throw new LocalReviewError(
      "invalid-evidence",
      `${label} must be a plain object.`,
    );
  }
}

function assertExactKeys(value, keys, label) {
  assertPlainObject(value, label);
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new LocalReviewError(
      "invalid-evidence",
      `${label} contained missing or unsupported fields.`,
    );
  }
}

function assertEnum(value, values, label) {
  if (!values.includes(value)) {
    throw new LocalReviewError(
      "invalid-evidence",
      `${label} contained an unsupported value.`,
    );
  }
}

function assertNullableString(value, label) {
  if (value !== null && typeof value !== "string") {
    throw new LocalReviewError(
      "invalid-evidence",
      `${label} must be a string or null.`,
    );
  }
}

function assertNullableStatus(value, pattern, label) {
  assertNullableString(value, label);
  if (value !== null && !pattern.test(value)) {
    throw new LocalReviewError(
      "invalid-evidence",
      `${label} contained an unsupported Git status.`,
    );
  }
}

function assertSortedUniqueStrings(values, label) {
  if (
    !Array.isArray(values) ||
    values.some((value) => typeof value !== "string") ||
    JSON.stringify(values) !== JSON.stringify([...new Set(values)].sort())
  ) {
    throw new LocalReviewError(
      "invalid-evidence",
      `${label} must contain sorted unique strings.`,
    );
  }
}

export function validateEvidenceDocument(value) {
  assertExactKeys(
    value,
    [
      "schema_version",
      "kind",
      "collected_at",
      "repository",
      "rule_set",
      "working_tree",
      "files",
      "candidates",
      "classification",
      "privacy",
    ],
    "Evidence",
  );
  if (value.schema_version !== EVIDENCE_SCHEMA_VERSION) {
    throw new LocalReviewError(
      "invalid-evidence",
      "Evidence used an unsupported schema version.",
    );
  }
  if (value.kind !== EVIDENCE_KIND || value.rule_set !== CRITICAL_RULE_SET) {
    throw new LocalReviewError(
      "invalid-evidence",
      "Evidence identity did not match the implemented contract.",
    );
  }
  const collectedDate = new Date(value.collected_at);
  if (
    typeof value.collected_at !== "string" ||
    !Number.isFinite(collectedDate.getTime()) ||
    collectedDate.toISOString() !== value.collected_at
  ) {
    throw new LocalReviewError(
      "invalid-evidence",
      "Evidence collection time was invalid.",
    );
  }

  assertExactKeys(
    value.repository,
    ["root_label", "head", "base", "scope_status", "unavailable_reasons"],
    "Repository evidence",
  );
  if (
    typeof value.repository.root_label !== "string" ||
    value.repository.root_label.length === 0 ||
    path.isAbsolute(value.repository.root_label)
  ) {
    throw new LocalReviewError(
      "invalid-evidence",
      "Repository label must not be an absolute path.",
    );
  }
  assertExactKeys(
    value.repository.head,
    ["status", "commit", "reason"],
    "Head evidence",
  );
  assertEnum(
    value.repository.head.status,
    ["available", "unavailable"],
    "Head status",
  );
  assertNullableString(value.repository.head.commit, "Head commit");
  assertNullableString(value.repository.head.reason, "Head reason");
  if (
    value.repository.head.status === "available" &&
    !/^[0-9a-f]{40,64}$/.test(value.repository.head.commit ?? "")
  ) {
    throw new LocalReviewError("invalid-evidence", "Head commit was invalid.");
  }
  if (
    (value.repository.head.status === "available" &&
      value.repository.head.reason !== null) ||
    (value.repository.head.status === "unavailable" &&
      (value.repository.head.commit !== null ||
        value.repository.head.reason === null))
  ) {
    throw new LocalReviewError(
      "invalid-evidence",
      "Head availability fields were inconsistent.",
    );
  }
  assertExactKeys(
    value.repository.base,
    ["requested", "status", "commit", "reason"],
    "Base evidence",
  );
  if (typeof value.repository.base.requested !== "boolean") {
    throw new LocalReviewError(
      "invalid-evidence",
      "Base requested flag was invalid.",
    );
  }
  assertEnum(
    value.repository.base.status,
    ["not-requested", "available", "unavailable"],
    "Base status",
  );
  assertNullableString(value.repository.base.commit, "Base commit");
  assertNullableString(value.repository.base.reason, "Base reason");
  if (
    value.repository.base.status === "available" &&
    !/^[0-9a-f]{40,64}$/.test(value.repository.base.commit ?? "")
  ) {
    throw new LocalReviewError("invalid-evidence", "Base commit was invalid.");
  }
  if (
    (!value.repository.base.requested &&
      (value.repository.base.status !== "not-requested" ||
        value.repository.base.commit !== null ||
        value.repository.base.reason !== null)) ||
    (value.repository.base.requested &&
      value.repository.base.status === "not-requested") ||
    (value.repository.base.status === "available" &&
      value.repository.base.reason !== null) ||
    (value.repository.base.status === "unavailable" &&
      (value.repository.base.commit !== null ||
        value.repository.base.reason === null))
  ) {
    throw new LocalReviewError(
      "invalid-evidence",
      "Base availability fields were inconsistent.",
    );
  }
  assertEnum(
    value.repository.scope_status,
    ["available", "unavailable"],
    "Scope status",
  );
  assertSortedUniqueStrings(
    value.repository.unavailable_reasons,
    "Unavailable reasons",
  );
  if (
    (value.repository.scope_status === "available" &&
      value.repository.unavailable_reasons.length !== 0) ||
    (value.repository.scope_status === "unavailable" &&
      value.repository.unavailable_reasons.length === 0)
  ) {
    throw new LocalReviewError(
      "invalid-evidence",
      "Scope availability did not match unavailable reasons.",
    );
  }

  assertExactKeys(value.working_tree, ["state", "counts"], "Working tree");
  assertEnum(
    value.working_tree.state,
    ["clean", "dirty", "unavailable"],
    "Working-tree state",
  );
  const countKeys = [
    "selected_files",
    "staged",
    "unstaged",
    "untracked",
    "renamed",
    "deleted",
    "binary",
    "submodules",
    "unavailable",
  ];
  assertExactKeys(value.working_tree.counts, countKeys, "Working-tree counts");
  for (const key of countKeys) {
    if (
      !Number.isInteger(value.working_tree.counts[key]) ||
      value.working_tree.counts[key] < 0
    ) {
      throw new LocalReviewError(
        "invalid-evidence",
        `Working-tree count '${key}' was invalid.`,
      );
    }
  }

  if (!Array.isArray(value.files) || !Array.isArray(value.candidates)) {
    throw new LocalReviewError(
      "invalid-evidence",
      "Evidence files and candidates must be arrays.",
    );
  }
  const fileKeys = [
    "path",
    "old_path",
    "change_sources",
    "base_status",
    "index_status",
    "worktree_status",
    "untracked",
    "conflict",
    "binary",
    "submodule",
    "exists",
    "kind",
    "size_bytes",
    "sha256",
    "hash_status",
    "unavailable_reason",
    "candidate_id",
  ];
  for (const file of value.files) {
    assertExactKeys(file, fileKeys, "File evidence");
    normalizeRepositoryPath(file.path);
    if (file.old_path !== null) normalizeRepositoryPath(file.old_path);
    if (
      !Array.isArray(file.change_sources) ||
      file.change_sources.some(
        (source) => !CHANGE_SOURCE_ORDER.includes(source),
      ) ||
      new Set(file.change_sources).size !== file.change_sources.length ||
      JSON.stringify(file.change_sources) !==
        JSON.stringify(sortChangeSources(file.change_sources))
    ) {
      throw new LocalReviewError(
        "invalid-evidence",
        "File change sources were invalid.",
      );
    }
    assertNullableStatus(
      file.base_status,
      /^[ACDMRTUXB][0-9]*$/,
      "Base file status",
    );
    assertNullableStatus(
      file.index_status,
      /^[MADRCUTUXB?!]$/,
      "Index file status",
    );
    assertNullableStatus(
      file.worktree_status,
      /^[MADRCUTUXB?!]$/,
      "Worktree file status",
    );
    if (
      typeof file.untracked !== "boolean" ||
      typeof file.conflict !== "boolean" ||
      typeof file.exists !== "boolean"
    ) {
      throw new LocalReviewError(
        "invalid-evidence",
        "File state flags were invalid.",
      );
    }
    assertEnum(file.binary, ["yes", "no", "unknown"], "Binary state");
    assertEnum(file.submodule, ["yes", "no", "unknown"], "Submodule state");
    assertEnum(
      file.kind,
      ["file", "missing", "submodule", "unsupported"],
      "File kind",
    );
    assertEnum(
      file.hash_status,
      ["hashed", "not-applicable", "unavailable"],
      "Hash status",
    );
    if (
      file.size_bytes !== null &&
      (!Number.isSafeInteger(file.size_bytes) || file.size_bytes < 0)
    ) {
      throw new LocalReviewError("invalid-evidence", "File size was invalid.");
    }
    if (file.sha256 !== null && !/^[0-9a-f]{64}$/.test(file.sha256)) {
      throw new LocalReviewError("invalid-evidence", "File hash was invalid.");
    }
    assertNullableString(file.unavailable_reason, "File unavailable reason");
    if (
      (file.hash_status === "unavailable" &&
        file.unavailable_reason === null) ||
      (file.hash_status !== "unavailable" && file.unavailable_reason !== null)
    ) {
      throw new LocalReviewError(
        "invalid-evidence",
        "File hash availability fields were inconsistent.",
      );
    }
    if (
      (file.kind === "missing" && file.exists) ||
      (file.kind !== "missing" && !file.exists) ||
      (file.submodule === "yes" && file.kind !== "submodule") ||
      (file.kind === "submodule" && file.submodule !== "yes")
    ) {
      throw new LocalReviewError(
        "invalid-evidence",
        "File kind did not match existence or submodule evidence.",
      );
    }
    if (file.candidate_id !== candidateId(file.path)) {
      throw new LocalReviewError(
        "invalid-evidence",
        "File candidate identity was invalid.",
      );
    }
    if (
      file.hash_status === "hashed" &&
      (file.kind !== "file" || file.sha256 === null || file.size_bytes === null)
    ) {
      throw new LocalReviewError(
        "invalid-evidence",
        "Hashed file evidence was incomplete.",
      );
    }
  }
  const orderedPaths = value.files.map((file) => file.path);
  if (
    JSON.stringify(orderedPaths) !== JSON.stringify([...orderedPaths].sort())
  ) {
    throw new LocalReviewError(
      "invalid-evidence",
      "File evidence was not path-sorted.",
    );
  }
  if (new Set(orderedPaths).size !== orderedPaths.length) {
    throw new LocalReviewError(
      "invalid-evidence",
      "File paths were duplicated.",
    );
  }

  const candidateKeys = ["id", "path", "classification", "rule_ids"];
  for (const candidate of value.candidates) {
    assertExactKeys(candidate, candidateKeys, "Candidate evidence");
    normalizeRepositoryPath(candidate.path);
    if (candidate.id !== candidateId(candidate.path)) {
      throw new LocalReviewError(
        "invalid-evidence",
        "Candidate identity was invalid.",
      );
    }
    assertEnum(
      candidate.classification,
      ["routine-candidate", "critical-candidate", "classification-required"],
      "Candidate classification",
    );
    assertSortedUniqueStrings(candidate.rule_ids, "Candidate rule identifiers");
  }
  if (
    JSON.stringify(value.candidates.map((candidate) => candidate.path)) !==
    JSON.stringify(orderedPaths)
  ) {
    throw new LocalReviewError(
      "invalid-evidence",
      "Candidates did not correspond exactly to selected files.",
    );
  }
  const expectedCandidates = value.files.map(candidateFor);
  if (JSON.stringify(value.candidates) !== JSON.stringify(expectedCandidates)) {
    throw new LocalReviewError(
      "invalid-evidence",
      "Candidate classifications did not match file evidence and critical rules.",
    );
  }

  assertExactKeys(
    value.classification,
    [
      "status",
      "critical_candidates",
      "routine_candidates",
      "classification_required",
      "rule_ids",
    ],
    "Classification summary",
  );
  assertEnum(
    value.classification.status,
    [
      "no-candidates",
      "routine-candidate",
      "critical-candidate",
      "classification-required",
    ],
    "Overall classification",
  );
  for (const key of [
    "critical_candidates",
    "routine_candidates",
    "classification_required",
  ]) {
    if (
      !Number.isInteger(value.classification[key]) ||
      value.classification[key] < 0
    ) {
      throw new LocalReviewError(
        "invalid-evidence",
        `Classification count '${key}' was invalid.`,
      );
    }
  }
  assertSortedUniqueStrings(
    value.classification.rule_ids,
    "Classification rule identifiers",
  );

  const expectedCounts = countsFor(value.files);
  if (
    JSON.stringify(value.working_tree.counts) !== JSON.stringify(expectedCounts)
  ) {
    throw new LocalReviewError(
      "invalid-evidence",
      "Working-tree counts did not match file evidence.",
    );
  }
  const expectedWorkingTreeState =
    value.repository.unavailable_reasons.includes("status-unavailable")
      ? "unavailable"
      : value.files.length === 0
        ? "clean"
        : "dirty";
  if (value.working_tree.state !== expectedWorkingTreeState) {
    throw new LocalReviewError(
      "invalid-evidence",
      "Working-tree state did not match collected status evidence.",
    );
  }
  const expectedClassification = classificationFor(
    value.candidates,
    value.repository.scope_status,
    expectedCounts,
  );
  if (
    JSON.stringify(value.classification) !==
    JSON.stringify(expectedClassification)
  ) {
    throw new LocalReviewError(
      "invalid-evidence",
      "Classification summary did not match candidate evidence.",
    );
  }

  const privacyKeys = [
    "file_contents_included",
    "absolute_paths_included",
    "environment_included",
    "credentials_included",
    "raw_provider_payloads_included",
    "databases_included",
    "network_used",
    "model_used",
    "local_output_written",
    "git_state_mutated",
    "tracked_files_mutated",
  ];
  assertExactKeys(value.privacy, privacyKeys, "Privacy evidence");
  for (const key of privacyKeys) {
    const expected = key === "local_output_written";
    if (value.privacy[key] !== expected) {
      throw new LocalReviewError(
        "invalid-evidence",
        `Privacy field '${key}' did not match the Stage 1 boundary.`,
      );
    }
  }
  return value;
}

function markdownCell(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll("|", "\\|")
    .replaceAll("\n", " ");
}

export function renderEvidenceSummary(evidence) {
  validateEvidenceDocument(evidence);
  const lines = [
    "# Local review evidence",
    "",
    `Collected ${evidence.collected_at} for **${markdownCell(evidence.repository.root_label)}**.`,
    "",
    `Overall classification: **${evidence.classification.status}**. This mechanical classification is not semantic approval.`,
    "",
    "## Repository state",
    "",
    `- Head: ${evidence.repository.head.status}${evidence.repository.head.commit ? ` (${evidence.repository.head.commit})` : ""}`,
    `- Base: ${evidence.repository.base.status}${evidence.repository.base.commit ? ` (${evidence.repository.base.commit})` : ""}`,
    `- Scope evidence: ${evidence.repository.scope_status}`,
    `- Working tree: ${evidence.working_tree.state}`,
    "",
    "## Counts",
    "",
    "| Measurement | Count |",
    "| --- | ---: |",
    ...Object.entries(evidence.working_tree.counts).map(
      ([key, count]) => `| ${key.replaceAll("_", " ")} | ${count} |`,
    ),
    "",
    "## Candidates",
    "",
  ];
  if (evidence.candidates.length === 0) {
    lines.push("- None.");
  } else {
    lines.push("| Candidate | Path | Classification | Rules |");
    lines.push("| --- | --- | --- | --- |");
    for (const candidate of evidence.candidates) {
      lines.push(
        `| ${candidate.id} | ${markdownCell(candidate.path)} | ${candidate.classification} | ${candidate.rule_ids.join(", ") || "None"} |`,
      );
    }
  }
  lines.push("", "## Unavailable or unbound evidence", "");
  const unavailableItems = [
    ...evidence.repository.unavailable_reasons,
    ...evidence.files
      .filter((file) => file.unavailable_reason !== null)
      .map((file) => `${markdownCell(file.path)}: ${file.unavailable_reason}`),
    ...evidence.files
      .filter((file) => file.conflict)
      .map((file) => `${markdownCell(file.path)}: unmerged-conflict`),
    ...evidence.files
      .filter((file) => file.submodule === "yes")
      .map((file) => `${markdownCell(file.path)}: submodule-content-not-bound`),
  ];
  if (unavailableItems.length === 0) {
    lines.push("- None.");
  } else {
    for (const reason of unavailableItems) {
      lines.push(`- ${reason}`);
    }
  }
  lines.push(
    "",
    "## Privacy boundary",
    "",
    "This inventory contains repository-relative metadata and content hashes only. It writes this ignored local bundle but contains no file contents, personal absolute paths, environment dump, credentials, raw provider payloads, databases, network result, model result, Git-state mutation, or tracked-file mutation.",
    "",
  );
  return lines.join("\n");
}

async function ensureOutputPath(
  repositoryRoot,
  outputRoot,
  collectedAt,
  headCommit,
) {
  const localRoot = path.join(repositoryRoot, ".wyrmgrid-local");
  await mkdir(localRoot, { recursive: true });
  const realLocalRoot = await realpath(localRoot);
  const relativeLocal = path.relative(repositoryRoot, realLocalRoot);
  if (
    relativeLocal === "" ||
    relativeLocal === ".." ||
    relativeLocal.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relativeLocal)
  ) {
    throw new LocalReviewError(
      "unsafe-output-root",
      "The local review output root escaped the repository.",
    );
  }

  const runId = `${collectedAt.replaceAll(/[^0-9A-Za-z]/g, "")}-${(headCommit ?? "nohead").slice(0, 12)}`;
  const selected = outputRoot
    ? path.resolve(repositoryRoot, outputRoot)
    : path.join(localRoot, "review", runId);
  const relative = path.relative(realLocalRoot, selected);
  if (
    relative === "" ||
    relative === ".." ||
    relative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relative)
  ) {
    throw new LocalReviewError(
      "unsafe-output-root",
      "Inventory output must be a new directory inside .wyrmgrid-local.",
    );
  }
  const parent = path.dirname(selected);
  await mkdir(parent, { recursive: true });
  const realParent = await realpath(parent);
  const parentRelative = path.relative(realLocalRoot, realParent);
  if (
    parentRelative === ".." ||
    parentRelative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(parentRelative)
  ) {
    throw new LocalReviewError(
      "unsafe-output-root",
      "Inventory output parent escaped .wyrmgrid-local.",
    );
  }
  try {
    await lstat(selected);
    throw new LocalReviewError(
      "output-already-exists",
      "Inventory output directory already exists.",
    );
  } catch (error) {
    if (error instanceof LocalReviewError) throw error;
    if (error?.code !== "ENOENT") throw error;
  }
  return selected;
}

async function writeEvidenceBundle(outputPath, evidence, summary) {
  const parent = path.dirname(outputPath);
  const staging = evidenceStagingPath(outputPath);
  await mkdir(staging, { recursive: false, mode: 0o700 });
  try {
    await Promise.all([
      writeFile(
        path.join(staging, "evidence.json"),
        `${JSON.stringify(evidence, null, 2)}\n`,
        { encoding: "utf8", flag: "wx", mode: 0o600 },
      ),
      writeFile(path.join(staging, "summary.md"), summary, {
        encoding: "utf8",
        flag: "wx",
        mode: 0o600,
      }),
    ]);
    await renameEvidenceDirectory(staging, outputPath);
  } catch (error) {
    await rm(staging, { recursive: true, force: true });
    throw new LocalReviewError(
      "output-write-failed",
      "Local review evidence could not be written atomically.",
      { cause: error },
    );
  }
}

export function evidenceStagingPath(outputPath, id = randomUUID()) {
  return path.join(path.dirname(outputPath), `.review-tmp-${id}`);
}

export async function renameEvidenceDirectory(
  stagingPath,
  outputPath,
  options = {},
) {
  const renameDirectory = options.renameDirectory ?? rename;
  const wait = options.wait ?? delay;
  const platform = options.platform ?? process.platform;

  for (let attempt = 0; ; attempt += 1) {
    try {
      await renameDirectory(stagingPath, outputPath);
      return;
    } catch (error) {
      const retryDelay = WINDOWS_RENAME_RETRY_DELAYS_MS[attempt];
      const retryable =
        platform === "win32" &&
        ["EACCES", "EBUSY", "EPERM"].includes(error?.code) &&
        retryDelay !== undefined;
      if (!retryable) throw error;

      try {
        await lstat(outputPath);
        throw new LocalReviewError(
          "output-already-exists",
          "Inventory output directory appeared while evidence was being written.",
        );
      } catch (destinationError) {
        if (destinationError instanceof LocalReviewError) {
          throw destinationError;
        }
        if (destinationError?.code !== "ENOENT") throw destinationError;
      }
      await wait(retryDelay);
    }
  }
}

export async function inventoryRepository(options = {}) {
  const git = options.git ?? runGit;
  const hashFile = options.hashFile ?? hashRegularFile;
  const repositoryRoot = await requireRepositoryRoot(
    options.repositoryRoot ?? process.cwd(),
    git,
  );
  const collectedAt = (options.now ? options.now() : new Date()).toISOString();
  const gitEvidence = await collectGitEvidence(
    repositoryRoot,
    options.baseRef,
    git,
  );
  const entries = new Map();
  if (gitEvidence.statusResult.ok) {
    applyStatusRecords(
      entries,
      parsePorcelainV2(gitEvidence.statusResult.stdout),
    );
  }
  if (gitEvidence.baseDiffResult?.ok) {
    applyBaseRecords(
      entries,
      parseNameStatus(gitEvidence.baseDiffResult.stdout),
    );
  }
  for (const result of gitEvidence.numstatResults) {
    applyBinaryRecords(entries, parseNumstat(result.stdout));
  }
  const trackedModes = gitEvidence.modesResult.ok
    ? parseTrackedModes(gitEvidence.modesResult.stdout)
    : new Map();

  const files = [];
  const comparePaths = (left, right) =>
    left.path < right.path ? -1 : left.path > right.path ? 1 : 0;
  for (const entry of [...entries.values()].sort(comparePaths)) {
    files.push(
      await inspectEntry(repositoryRoot, entry, trackedModes, hashFile),
    );
  }
  files.sort(comparePaths);
  const counts = countsFor(files);
  const candidates = files.map(candidateFor);
  const scopeStatus =
    gitEvidence.unavailable.length > 0 ? "unavailable" : "available";
  const evidence = {
    schema_version: EVIDENCE_SCHEMA_VERSION,
    kind: EVIDENCE_KIND,
    collected_at: collectedAt,
    repository: {
      root_label: path.basename(repositoryRoot),
      head: {
        status: gitEvidence.headCommit ? "available" : "unavailable",
        commit: gitEvidence.headCommit,
        reason: gitEvidence.headCommit ? null : "head-unavailable",
      },
      base: {
        requested: options.baseRef !== undefined,
        status: gitEvidence.baseStatus,
        commit: gitEvidence.baseCommit,
        reason:
          gitEvidence.baseStatus === "unavailable" ? "base-unavailable" : null,
      },
      scope_status: scopeStatus,
      unavailable_reasons: gitEvidence.unavailable,
    },
    rule_set: CRITICAL_RULE_SET,
    working_tree: {
      state: gitEvidence.unavailable.includes("status-unavailable")
        ? "unavailable"
        : files.length === 0
          ? "clean"
          : "dirty",
      counts,
    },
    files,
    candidates,
    classification: classificationFor(candidates, scopeStatus, counts),
    privacy: {
      file_contents_included: false,
      absolute_paths_included: false,
      environment_included: false,
      credentials_included: false,
      raw_provider_payloads_included: false,
      databases_included: false,
      network_used: false,
      model_used: false,
      local_output_written: true,
      git_state_mutated: false,
      tracked_files_mutated: false,
    },
  };
  validateEvidenceDocument(evidence);
  const summary = renderEvidenceSummary(evidence);
  const outputPath = await ensureOutputPath(
    repositoryRoot,
    options.outputRoot,
    collectedAt,
    gitEvidence.headCommit,
  );
  await writeEvidenceBundle(outputPath, evidence, summary);
  return {
    evidence,
    evidencePath: path.join(outputPath, "evidence.json"),
    summaryPath: path.join(outputPath, "summary.md"),
    outputPath,
  };
}

export function parseArguments(argv) {
  const result = { baseRef: undefined, outputRoot: undefined };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument !== "--base" && argument !== "--output") {
      throw new LocalReviewError(
        "invalid-arguments",
        "Usage: node scripts/local-review.mjs [--base <git-ref>] [--output <.wyrmgrid-local/path>]",
      );
    }
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) {
      throw new LocalReviewError(
        "invalid-arguments",
        `Missing value for ${argument}.`,
      );
    }
    index += 1;
    if (argument === "--base") {
      if (result.baseRef !== undefined) {
        throw new LocalReviewError(
          "invalid-arguments",
          "The --base argument may be supplied only once.",
        );
      }
      result.baseRef = value;
    } else {
      if (result.outputRoot !== undefined) {
        throw new LocalReviewError(
          "invalid-arguments",
          "The --output argument may be supplied only once.",
        );
      }
      result.outputRoot = value;
    }
  }
  return result;
}

async function main() {
  const args = parseArguments(process.argv.slice(2));
  const result = await inventoryRepository({
    repositoryRoot: process.cwd(),
    baseRef: args.baseRef,
    outputRoot: args.outputRoot,
  });
  const relativeOutput = path
    .relative(process.cwd(), result.outputPath)
    .replaceAll(path.sep, "/");
  console.log(
    `Local review inventory wrote ${result.evidence.files.length} candidate(s) to ${relativeOutput}.`,
  );
  if (result.evidence.repository.scope_status === "unavailable") {
    console.error(
      "Local review inventory completed with unavailable evidence; review summary.md before continuing.",
    );
    process.exitCode = 2;
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href
) {
  main().catch((error) => {
    const code =
      error instanceof LocalReviewError ? error.code : "unexpected-error";
    console.error(`Local review inventory failed (${code}).`);
    process.exitCode = 1;
  });
}
