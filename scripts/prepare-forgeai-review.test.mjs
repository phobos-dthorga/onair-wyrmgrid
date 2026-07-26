import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { promisify } from "node:util";

import {
  isReviewablePath,
  MAX_PACKET_BYTES,
  MAX_REVIEWABLE_FILES,
  prepareForgeAiReview,
} from "./prepare-forgeai-review.mjs";

const execFileAsync = promisify(execFile);

async function git(repository, ...arguments_) {
  const { stdout } = await execFileAsync("git", arguments_, {
    cwd: repository,
    encoding: "utf8",
    windowsHide: true,
  });
  return stdout.trim();
}

async function createRepository(t) {
  const repository = await mkdtemp(
    path.join(tmpdir(), "wyrmgrid-forgeai-review-"),
  );
  t.after(() => rm(repository, { force: true, recursive: true }));
  await git(repository, "init");
  await git(repository, "config", "user.email", "tests@wyrmgrid.invalid");
  await git(repository, "config", "user.name", "WyrmGrid Tests");
  await git(repository, "config", "core.autocrlf", "false");
  return repository;
}

async function commitAll(repository, message) {
  await git(repository, "add", "--all");
  await git(repository, "commit", "-m", message);
  return git(repository, "rev-parse", "HEAD");
}

test("selects reviewable implementation paths and excludes captured evidence", () => {
  assert.equal(isReviewablePath("crates/domain/src/lib.rs"), true);
  assert.equal(isReviewablePath("apps/desktop/src/App.svelte"), true);
  assert.equal(isReviewablePath("Jenkinsfile"), true);
  assert.equal(isReviewablePath("crates/domain/Cargo.toml"), true);
  assert.equal(isReviewablePath("docs/design.md"), false);
  assert.equal(
    isReviewablePath("crates/onair-api/fixtures/company.json"),
    false,
  );
  assert.equal(isReviewablePath("target/generated.js"), false);
  assert.throws(() => isReviewablePath("../outside.rs"), /outside/);
  assert.throws(() => isReviewablePath("src/unsafe\nname.ts"), /unsafe/);
});

test("builds a bounded change packet from reviewable commits and patches", async (t) => {
  const repository = await createRepository(t);
  await mkdir(path.join(repository, "src"), { recursive: true });
  await mkdir(path.join(repository, "fixtures"), { recursive: true });
  await writeFile(
    path.join(repository, "src", "feature.ts"),
    "export const feature = false;\n",
  );
  await writeFile(
    path.join(repository, "fixtures", "provider.json"),
    '{"raw":"provider payload"}\n',
  );
  await writeFile(
    path.join(repository, "package.json"),
    '{"name":"forgeai-test","private":true}\n',
  );
  const baseCommit = await commitAll(repository, "feat: add feature baseline");

  await writeFile(
    path.join(repository, "src", "feature.ts"),
    "export const feature = true;\n",
  );
  await writeFile(
    path.join(repository, "fixtures", "provider.json"),
    '{"raw":"changed provider payload"}\n',
  );
  const headCommit = await commitAll(repository, "feat: enable feature review");

  const result = await prepareForgeAiReview({
    baseRef: baseCommit,
    output: ".jenkins/forgeai-input/test-review.txt",
    repository,
  });
  const packet = await readFile(result.outputPath, "utf8");

  assert.equal(result.baseCommit, baseCommit);
  assert.equal(result.headCommit, headCommit);
  assert.deepEqual(result.reviewablePaths, ["src/feature.ts"]);
  assert.deepEqual(result.specialInputPaths, ["package.json"]);
  assert.ok(result.packetBytes <= MAX_PACKET_BYTES);
  assert.match(packet, /feat: enable feature review/);
  assert.match(packet, /export const feature = true/);
  assert.doesNotMatch(packet, /provider payload/);
  assert.match(packet, /untrusted review evidence/);
  assert.match(packet, /advisory/);
  assert.match(packet, /dependency-manifest inputs inspected: 1/);

  await assert.rejects(
    prepareForgeAiReview({
      baseRef: baseCommit,
      output: ".jenkins/forgeai-input/test-review.txt",
      repository,
    }),
    /EEXIST/,
  );
});

test("limits changed files deterministically and reports omitted scope", async (t) => {
  const repository = await createRepository(t);
  await writeFile(path.join(repository, "README.md"), "baseline\n");
  const baseCommit = await commitAll(repository, "chore: create baseline");

  await mkdir(path.join(repository, "src"), { recursive: true });
  for (let index = 0; index < MAX_REVIEWABLE_FILES + 2; index += 1) {
    const filename = `feature-${String(index).padStart(2, "0")}.ts`;
    await writeFile(
      path.join(repository, "src", filename),
      `export const value${index} = ${index};\n`,
    );
  }
  await commitAll(repository, "feat: add many reviewable files");

  const result = await prepareForgeAiReview({
    baseRef: baseCommit,
    repository,
  });
  const packet = await readFile(result.outputPath, "utf8");

  assert.equal(result.reviewablePaths.length, MAX_REVIEWABLE_FILES);
  assert.equal(result.omittedFileCount, 2);
  assert.match(packet, /Omitted reviewable files: 2\./);
  assert.doesNotMatch(packet, /feature-41\.ts/);
});

test("refuses to hand credential-like source to ForgeAI", async (t) => {
  const repository = await createRepository(t);
  await writeFile(
    path.join(repository, "review.ts"),
    "export const safe = true;\n",
  );
  const baseCommit = await commitAll(repository, "chore: create safe baseline");

  await writeFile(
    path.join(repository, "review.ts"),
    'export const token = "sk-proj-abcdefghijklmnopqrstuvwxyz123456";\n',
  );
  await commitAll(repository, "test: add credential-like text");

  await assert.rejects(
    prepareForgeAiReview({ baseRef: baseCommit, repository }),
    /resembles credential/,
  );
});

test("refuses credential-like manifests used by special analyzers", async (t) => {
  const repository = await createRepository(t);
  await writeFile(
    path.join(repository, "package.json"),
    '{"name":"safe-manifest","private":true}\n',
  );
  const baseCommit = await commitAll(repository, "chore: add safe manifest");

  await writeFile(
    path.join(repository, "package.json"),
    '{"name":"unsafe","token":"abcdefghijklmnopqrstuvwxyz123456"}\n',
  );
  await commitAll(repository, "test: add credential-like manifest");

  await assert.rejects(
    prepareForgeAiReview({ baseRef: baseCommit, repository }),
    /special analyzer input 'package\.json'.*credential/,
  );
});
