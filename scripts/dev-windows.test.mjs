import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const launcherPath = new URL("./dev-windows.ps1", import.meta.url);
const environmentPath = new URL(
  "./windows-build-environment.ps1",
  import.meta.url,
);

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
  const [launcher, environment] = await Promise.all([
    readFile(launcherPath, "utf8"),
    readFile(environmentPath, "utf8"),
  ]);

  assert.match(environment, /Split-Path -Leaf \$RepositoryRoot/);
  assert.match(environment, /WyrmGrid\\cargo-target/);
  assert.match(launcher, /Enter-WyrmGridWindowsBuildEnvironment/);

  const repositoryRoot = launcher.indexOf(
    "$repositoryRoot = Split-Path -Parent $PSScriptRoot",
  );
  const environmentEntry = launcher.indexOf(
    "Enter-WyrmGridWindowsBuildEnvironment",
  );

  assert.notEqual(repositoryRoot, -1);
  assert.notEqual(environmentEntry, -1);
  assert.ok(repositoryRoot < environmentEntry);
  assert.match(environment, /\$env:CARGO_TARGET_DIR = \$CargoTargetDir/);
});

test("shared Windows environment validates native and language toolchains", async () => {
  const environment = await readFile(environmentPath, "utf8");

  assert.match(environment, /Microsoft\.VisualStudio\.Component\.VC\.Tools/);
  assert.match(environment, /Strawberry Perl was not found/);
  for (const command of ["node", "npm", "rustc", "cargo"]) {
    assert.match(environment, new RegExp(`'${command}'`));
  }
});

test("Jenkins Cargo targets use a bounded hash of the complete job identity", async () => {
  const environment = await readFile(environmentPath, "utf8");

  assert.match(environment, /function Get-WyrmGridJenkinsCargoTargetDirectory/);
  assert.match(environment, /SHA256/);
  assert.match(environment, /Substring\(0, 16\)/);
  assert.match(environment, /WyrmGrid\\jenkins-cargo-target/);
});
