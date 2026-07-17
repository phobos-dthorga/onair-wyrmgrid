import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const launcherPath = new URL("./dev-windows.ps1", import.meta.url);

test("Windows launcher restores locked dependencies before starting Tauri", async () => {
  const launcher = await readFile(launcherPath, "utf8");

  assert.match(launcher, /node_modules\\\.bin\\tauri\.cmd/);
  assert.match(launcher, /package-lock\.json/);
  assert.match(launcher, /& npm ci/);

  const restoreCall = launcher.indexOf(
    "Restore-DevelopmentDependenciesIfNeeded -RepositoryRoot $repositoryRoot",
  );
  const developmentCall = launcher.indexOf("& npm run dev");

  assert.notEqual(restoreCall, -1);
  assert.notEqual(developmentCall, -1);
  assert.ok(restoreCall < developmentCall);
});

test("validation-only mode returns before dependency restoration", async () => {
  const launcher = await readFile(launcherPath, "utf8");
  const validationGuard = launcher.indexOf("if ($ValidateOnly)");
  const restoreCall = launcher.indexOf(
    "Restore-DevelopmentDependenciesIfNeeded -RepositoryRoot $repositoryRoot",
  );

  assert.notEqual(validationGuard, -1);
  assert.notEqual(restoreCall, -1);
  assert.ok(validationGuard < restoreCall);
});

test("Windows launcher isolates Cargo output by worktree", async () => {
  const launcher = await readFile(launcherPath, "utf8");

  assert.match(launcher, /Split-Path -Leaf \$RepositoryRoot/);
  assert.match(launcher, /WyrmGrid\\cargo-target/);
  assert.match(
    launcher,
    /Get-WorktreeCargoTargetDirectory -RepositoryRoot \$repositoryRoot/,
  );

  const repositoryRoot = launcher.indexOf(
    "$repositoryRoot = Split-Path -Parent $PSScriptRoot",
  );
  const targetSelection = launcher.indexOf(
    "$CargoTargetDir = Get-WorktreeCargoTargetDirectory",
  );
  const environmentAssignment = launcher.indexOf(
    "$env:CARGO_TARGET_DIR = $CargoTargetDir",
  );

  assert.notEqual(repositoryRoot, -1);
  assert.notEqual(targetSelection, -1);
  assert.notEqual(environmentAssignment, -1);
  assert.ok(repositoryRoot < targetSelection);
  assert.ok(targetSelection < environmentAssignment);
});
