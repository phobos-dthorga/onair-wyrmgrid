import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import {
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  stat,
  truncate,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { promisify } from "node:util";
import test from "node:test";

import {
  CRITICAL_RULE_SET,
  EVIDENCE_KIND,
  EVIDENCE_SCHEMA_VERSION,
  candidateId,
  criticalRuleIds,
  evidenceStagingPath,
  hashRegularFile,
  inventoryRepository,
  normalizeRepositoryPath,
  parseArguments,
  parseNameStatus,
  parseNumstat,
  parsePorcelainV2,
  parseTrackedModes,
  renameEvidenceDirectory,
  renderEvidenceSummary,
  runGit,
  validateEvidenceDocument,
} from "./local-review.mjs";

const execFileAsync = promisify(execFile);
const repositoryRoot = path.resolve(import.meta.dirname, "..");

async function git(root, ...args) {
  return await execFileAsync(
    "git",
    ["-c", "core.fsmonitor=false", "-c", "core.untrackedCache=false", ...args],
    {
      cwd: root,
      encoding: "utf8",
      windowsHide: true,
    },
  );
}

async function write(root, name, content) {
  const target = path.join(root, ...name.split("/"));
  await mkdir(path.dirname(target), { recursive: true });
  await writeFile(target, content);
  return target;
}

async function createRepository(t) {
  const root = await mkdtemp(path.join(tmpdir(), "wyrmgrid-local-review-"));
  t.after(async () => await rm(root, { recursive: true, force: true }));
  await git(root, "init", "--initial-branch=main");
  await git(root, "config", "user.name", "WyrmGrid fixture");
  await git(root, "config", "user.email", "fixture@example.invalid");
  await write(root, ".gitignore", ".wyrmgrid-local/\n");
  return root;
}

async function commitAll(root, message = "fixture baseline") {
  await git(root, "add", "--all");
  await git(root, "commit", "-m", message);
  return (await git(root, "rev-parse", "HEAD")).stdout.trim();
}

test("parses NUL-delimited status, rename, binary, and tracked-mode evidence", () => {
  const status = Buffer.from(
    [
      "1 .M N... 100644 100644 100644 1111111 2222222 src/file with space.txt",
      "2 R. N... 100644 100644 100644 1111111 2222222 R100 docs/new name.md",
      "docs/old name.md",
      "? notes/évidence.txt",
      "",
    ].join("\0"),
  );
  assert.deepEqual(parsePorcelainV2(status), [
    {
      type: "ordinary",
      xy: ".M",
      submodule: "N...",
      path: "src/file with space.txt",
      old_path: null,
    },
    {
      type: "rename",
      xy: "R.",
      submodule: "N...",
      path: "docs/new name.md",
      old_path: "docs/old name.md",
    },
    {
      type: "untracked",
      xy: "??",
      submodule: "N...",
      path: "notes/évidence.txt",
      old_path: null,
    },
  ]);

  assert.deepEqual(
    parseNameStatus(
      Buffer.from("M\0src/file.txt\0R100\0docs/old.md\0docs/new.md\0"),
    ),
    [
      { status: "M", path: "src/file.txt", old_path: null },
      { status: "R100", path: "docs/new.md", old_path: "docs/old.md" },
    ],
  );
  assert.deepEqual(
    parseNumstat(Buffer.from("1\t2\tsrc/file.txt\0-\t-\tassets/image.bin\0")),
    [
      { path: "src/file.txt", binary: false },
      { path: "assets/image.bin", binary: true },
    ],
  );
  assert.deepEqual(
    [...parseTrackedModes(Buffer.from("100644 abcdef 0\tsrc/file.txt\0"))],
    [["src/file.txt", "100644"]],
  );
});

test("rejects malformed or escaping Git paths and arguments", () => {
  assert.equal(normalizeRepositoryPath("docs/example.md"), "docs/example.md");
  for (const unsafe of [
    "../outside.txt",
    "docs/../outside.txt",
    "/absolute.txt",
    "C:/absolute.txt",
    "windows\\path.txt",
    "line\nbreak.txt",
    "bidi-\u202efile.txt",
  ]) {
    assert.throws(() => normalizeRepositoryPath(unsafe), /path/);
  }
  assert.deepEqual(parseArguments([]), {
    baseRef: undefined,
    outputRoot: undefined,
  });
  assert.deepEqual(
    parseArguments(["--base", "HEAD", "--output", ".wyrmgrid-local/test"]),
    { baseRef: "HEAD", outputRoot: ".wyrmgrid-local/test" },
  );
  assert.throws(() => parseArguments(["--unknown", "value"]), /Usage/);
  assert.throws(() => parseArguments(["--base"]), /Missing value/);
  assert.throws(
    () => parseArguments(["--base", "HEAD", "--base", "main"]),
    /only once/,
  );
});

test("uses stable path identities and conservative critical-path escalation", () => {
  assert.equal(candidateId("docs/example.md"), candidateId("docs/example.md"));
  assert.notEqual(candidateId("docs/example.md"), candidateId("docs/other.md"));
  assert.deepEqual(criticalRuleIds("docs/example.md"), []);
  assert.deepEqual(criticalRuleIds("crates/storage/migrations/0018_test.sql"), [
    "critical-database-migration",
  ]);
  assert.ok(
    criticalRuleIds("docs/operations/local-review-automation.md").includes(
      "critical-optional-ai-governance",
    ),
  );
  assert.ok(
    criticalRuleIds("scripts/local-review.mjs").includes(
      "critical-optional-ai-governance",
    ),
  );
  assert.ok(
    criticalRuleIds("schemas/local-review-evidence-v1.schema.json").includes(
      "critical-protocol-or-schema",
    ),
  );
});

test("keeps the runtime validator, schema, and canonical fixture synchronized", async () => {
  const schema = JSON.parse(
    await readFile(
      path.join(repositoryRoot, "schemas/local-review-evidence-v1.schema.json"),
      "utf8",
    ),
  );
  const fixture = JSON.parse(
    await readFile(
      path.join(
        repositoryRoot,
        "schemas/fixtures/local-review-evidence-v1.json",
      ),
      "utf8",
    ),
  );
  assert.equal(schema.properties.schema_version.const, EVIDENCE_SCHEMA_VERSION);
  assert.equal(schema.properties.kind.const, EVIDENCE_KIND);
  assert.equal(schema.properties.rule_set.const, CRITICAL_RULE_SET);
  assert.equal(validateEvidenceDocument(fixture), fixture);
  assert.match(renderEvidenceSummary(fixture), /routine-candidate/);

  assert.throws(
    () => validateEvidenceDocument({ ...fixture, schema_version: 2 }),
    /unsupported schema version/,
  );
  assert.throws(
    () => validateEvidenceDocument({ ...fixture, unexpected: true }),
    /unsupported fields/,
  );
  assert.throws(
    () =>
      validateEvidenceDocument({
        ...fixture,
        privacy: { ...fixture.privacy, model_used: true },
      }),
    /did not match the Stage 1 boundary/,
  );
});

test("detects a regular file that changes during hashing", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "wyrmgrid-hash-race-"));
  t.after(async () => await rm(root, { recursive: true, force: true }));
  const target = await write(root, "evidence.txt", "before");
  await assert.rejects(
    hashRegularFile(target, {
      afterRead: async () => await writeFile(target, "after and larger"),
    }),
    /changed while it was being hashed/,
  );
});

test("refuses to hash an oversized selected file", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "wyrmgrid-hash-limit-"));
  t.after(async () => await rm(root, { recursive: true, force: true }));
  const target = await write(root, "oversized.bin", "fixture");
  await truncate(target, 128 * 1024 * 1024 + 1);
  await assert.rejects(hashRegularFile(target), /supported file size/);
});

test("inventories a clean repository without candidates or model activity", async (t) => {
  const root = await createRepository(t);
  await write(root, "README.md", "fixture\n");
  const head = await commitAll(root);
  const indexBefore = await stat(path.join(root, ".git", "index"), {
    bigint: true,
  });

  const result = await inventoryRepository({
    repositoryRoot: root,
    outputRoot: ".wyrmgrid-local/review/clean",
    now: () => new Date("2026-07-19T07:10:00.000Z"),
  });
  assert.equal(result.evidence.repository.head.commit, head);
  assert.equal(result.evidence.working_tree.state, "clean");
  assert.equal(result.evidence.classification.status, "no-candidates");
  assert.deepEqual(result.evidence.files, []);
  assert.equal(result.evidence.privacy.model_used, false);
  assert.equal(result.evidence.privacy.network_used, false);
  assert.equal(result.evidence.privacy.local_output_written, true);
  assert.equal(result.evidence.privacy.git_state_mutated, false);
  assert.equal(result.evidence.privacy.tracked_files_mutated, false);
  const indexAfter = await stat(path.join(root, ".git", "index"), {
    bigint: true,
  });
  assert.equal(indexAfter.size, indexBefore.size);
  assert.equal(indexAfter.mtimeNs, indexBefore.mtimeNs);
  assert.equal((await git(root, "status", "--porcelain")).stdout, "");
  assert.ok(await readFile(result.evidencePath, "utf8"));
  assert.match(await readFile(result.summaryPath, "utf8"), /None\./);
  assert.deepEqual(await readdir(path.dirname(result.outputPath)), ["clean"]);
});

test("inventories staged, unstaged, untracked, renamed, deleted, and binary changes", async (t) => {
  const root = await createRepository(t);
  await write(root, "docs/old.md", "old name\n");
  await write(root, "src/modify.txt", "before\n");
  await write(root, "src/delete.txt", "delete me\n");
  await write(root, "assets/binary.bin", Buffer.from([0, 1, 2, 3]));
  const head = await commitAll(root);

  await git(root, "mv", "docs/old.md", "docs/renamed name.md");
  await rm(path.join(root, "src/delete.txt"));
  await write(root, "assets/binary.bin", Buffer.from([0, 255, 0, 254, 0]));
  await git(root, "add", "--all");
  await write(root, "src/modify.txt", "after, but unstaged\n");
  const privateMarker = "instruction-like text that must not enter evidence";
  await write(root, "notes/évidence file.txt", privateMarker);

  const result = await inventoryRepository({
    repositoryRoot: root,
    baseRef: "HEAD",
    outputRoot: ".wyrmgrid-local/review/dirty",
    now: () => new Date("2026-07-19T07:11:00.000Z"),
  });
  assert.equal(result.evidence.repository.base.commit, head);
  assert.equal(result.evidence.repository.scope_status, "available");
  assert.equal(result.evidence.working_tree.state, "dirty");
  assert.equal(result.evidence.working_tree.counts.selected_files, 5);
  assert.equal(result.evidence.working_tree.counts.staged, 3);
  assert.equal(result.evidence.working_tree.counts.unstaged, 1);
  assert.equal(result.evidence.working_tree.counts.untracked, 1);
  assert.equal(result.evidence.working_tree.counts.renamed, 1);
  assert.equal(result.evidence.working_tree.counts.deleted, 1);
  assert.equal(result.evidence.working_tree.counts.binary, 1);

  const byPath = new Map(
    result.evidence.files.map((file) => [file.path, file]),
  );
  assert.equal(byPath.get("assets/binary.bin").binary, "yes");
  assert.equal(byPath.get("docs/renamed name.md").old_path, "docs/old.md");
  assert.equal(byPath.get("src/delete.txt").exists, false);
  assert.equal(byPath.get("notes/évidence file.txt").hash_status, "hashed");
  assert.deepEqual(
    result.evidence.files.map((file) => file.path),
    [...result.evidence.files.map((file) => file.path)].sort(),
  );

  const durable = await readFile(result.evidencePath, "utf8");
  assert.equal(durable.includes(root), false);
  assert.equal(durable.includes(privateMarker), false);
  assert.equal(durable.includes("fixture@example.invalid"), false);
  validateEvidenceDocument(JSON.parse(durable));
});

test("records a missing requested base as unavailable instead of assuming a pass", async (t) => {
  const root = await createRepository(t);
  await write(root, "README.md", "fixture\n");
  await commitAll(root);

  const result = await inventoryRepository({
    repositoryRoot: root,
    baseRef: "missing-base",
    outputRoot: ".wyrmgrid-local/review/missing-base",
    now: () => new Date("2026-07-19T07:12:00.000Z"),
  });
  assert.equal(result.evidence.repository.base.status, "unavailable");
  assert.equal(result.evidence.repository.scope_status, "unavailable");
  assert.equal(
    result.evidence.classification.status,
    "classification-required",
  );
  assert.deepEqual(result.evidence.repository.unavailable_reasons, [
    "base-unavailable",
  ]);
});

test("uses CLI exit status 2 when an unavailable evidence bundle was written", async (t) => {
  const root = await createRepository(t);
  await write(root, "README.md", "fixture\n");
  await commitAll(root);
  await assert.rejects(
    execFileAsync(
      process.execPath,
      [
        path.join(repositoryRoot, "scripts", "local-review.mjs"),
        "--base",
        "missing-base",
        "--output",
        ".wyrmgrid-local/review/cli-unavailable",
      ],
      { cwd: root, encoding: "utf8", windowsHide: true },
    ),
    (error) => error.code === 2,
  );
  const evidence = JSON.parse(
    await readFile(
      path.join(
        root,
        ".wyrmgrid-local",
        "review",
        "cli-unavailable",
        "evidence.json",
      ),
      "utf8",
    ),
  );
  assert.equal(evidence.repository.scope_status, "unavailable");
});

test("records unavailable Git status without reporting a clean pass", async (t) => {
  const root = await createRepository(t);
  await write(root, "README.md", "fixture\n");
  await commitAll(root);
  const statusUnavailable = async (repository, args) => {
    if (args.includes("status")) {
      return { ok: false, code: 1, stdout: Buffer.alloc(0) };
    }
    return await runGit(repository, args);
  };

  const result = await inventoryRepository({
    repositoryRoot: root,
    outputRoot: ".wyrmgrid-local/review/status-unavailable",
    now: () => new Date("2026-07-19T07:13:00.000Z"),
    git: statusUnavailable,
  });
  assert.equal(result.evidence.working_tree.state, "unavailable");
  assert.equal(result.evidence.repository.scope_status, "unavailable");
  assert.equal(
    result.evidence.classification.status,
    "classification-required",
  );
  assert.ok(
    result.evidence.repository.unavailable_reasons.includes(
      "status-unavailable",
    ),
  );
});

test("removes inherited Git control variables and external diff execution", async (t) => {
  const root = await createRepository(t);
  await write(root, "README.md", "before\n");
  await commitAll(root);
  await write(root, "README.md", "after\n");
  const priorGitDirectory = process.env.GIT_DIR;
  const priorExternalDiff = process.env.GIT_EXTERNAL_DIFF;
  process.env.GIT_DIR = path.join(root, "missing-git-directory");
  process.env.GIT_EXTERNAL_DIFF = "missing-external-diff-command";
  try {
    const result = await inventoryRepository({
      repositoryRoot: root,
      baseRef: "HEAD",
      outputRoot: ".wyrmgrid-local/review/sanitized-git-environment",
      now: () => new Date("2026-07-19T07:13:30.000Z"),
    });
    assert.equal(result.evidence.repository.scope_status, "available");
    assert.deepEqual(
      result.evidence.files.map((file) => file.path),
      ["README.md"],
    );
  } finally {
    if (priorGitDirectory === undefined) delete process.env.GIT_DIR;
    else process.env.GIT_DIR = priorGitDirectory;
    if (priorExternalDiff === undefined) delete process.env.GIT_EXTERNAL_DIFF;
    else process.env.GIT_EXTERNAL_DIFF = priorExternalDiff;
  }
});

test("records a changed submodule without reading its contents", async (t) => {
  const child = await createRepository(t);
  await write(child, "child.txt", "child baseline\n");
  await commitAll(child, "child baseline");

  const root = await createRepository(t);
  await write(root, "README.md", "parent\n");
  await commitAll(root, "parent baseline");
  await git(
    root,
    "-c",
    "protocol.file.allow=always",
    "submodule",
    "add",
    child,
    "modules/fixture",
  );
  await commitAll(root, "add fixture submodule");
  await write(root, "modules/fixture/child.txt", "changed locally\n");

  const result = await inventoryRepository({
    repositoryRoot: root,
    outputRoot: ".wyrmgrid-local/review/submodule",
    now: () => new Date("2026-07-19T07:14:00.000Z"),
  });
  const submodule = result.evidence.files.find(
    (file) => file.path === "modules/fixture",
  );
  assert.equal(submodule.submodule, "yes");
  assert.equal(submodule.kind, "submodule");
  assert.equal(submodule.hash_status, "not-applicable");
  assert.equal(submodule.sha256, null);
  assert.equal(result.evidence.working_tree.counts.submodules, 1);
  assert.equal(
    result.evidence.classification.status,
    "classification-required",
  );
  assert.match(
    await readFile(result.summaryPath, "utf8"),
    /submodule-content-not-bound/,
  );
});

test("requires the exact repository root and an unused local output directory", async (t) => {
  const root = await createRepository(t);
  await write(root, "README.md", "fixture\n");
  await commitAll(root);
  await mkdir(path.join(root, "nested"));

  await assert.rejects(
    inventoryRepository({
      repositoryRoot: path.join(root, "nested"),
      outputRoot: ".wyrmgrid-local/review/nested",
    }),
    /exact Git repository root/,
  );
  await assert.rejects(
    inventoryRepository({
      repositoryRoot: root,
      outputRoot: "outside-review",
    }),
    /inside .wyrmgrid-local/,
  );

  await inventoryRepository({
    repositoryRoot: root,
    outputRoot: ".wyrmgrid-local/review/once",
  });
  await assert.rejects(
    inventoryRepository({
      repositoryRoot: root,
      outputRoot: ".wyrmgrid-local/review/once",
    }),
    /already exists/,
  );
});

test("keeps the atomic staging directory independent of a long output name", () => {
  const outputPath = path.join(
    "repository",
    ".wyrmgrid-local",
    "review",
    "x".repeat(80),
  );
  const stagingPath = evidenceStagingPath(
    outputPath,
    "11111111-2222-4333-8444-555555555555",
  );

  assert.equal(path.dirname(stagingPath), path.dirname(outputPath));
  assert.equal(
    path.basename(stagingPath),
    ".review-tmp-11111111-2222-4333-8444-555555555555",
  );
  assert.ok(stagingPath.length < outputPath.length);
});

test("retries transient Windows directory locks without weakening atomic writes", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "wyrmgrid-review-rename-"));
  t.after(async () => await rm(root, { recursive: true, force: true }));
  const stagingPath = path.join(root, "staging");
  const outputPath = path.join(root, "output");
  const waits = [];
  let attempts = 0;

  await renameEvidenceDirectory(stagingPath, outputPath, {
    platform: "win32",
    renameDirectory: async () => {
      attempts += 1;
      if (attempts < 3) {
        const error = new Error("temporarily locked");
        error.code = "EPERM";
        throw error;
      }
    },
    wait: async (milliseconds) => waits.push(milliseconds),
  });

  assert.equal(attempts, 3);
  assert.deepEqual(waits, [25, 50]);
});
