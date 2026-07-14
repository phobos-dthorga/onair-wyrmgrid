import { copyFile, mkdir } from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const release = process.argv.includes("--release");
const buildFrontend = process.argv.includes("--build-frontend");
const profile = release ? "release" : "debug";

function run(command, args, capture = false) {
  const result = spawnSync(command, args, {
    cwd: repositoryRoot,
    encoding: capture ? "utf8" : undefined,
    stdio: capture ? ["ignore", "pipe", "inherit"] : "inherit",
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
  return capture ? result.stdout.trim() : "";
}

function runNpm(args) {
  const npmEntrypoint = process.env.npm_execpath;
  if (!npmEntrypoint) {
    throw new Error(
      "npm did not report its JavaScript entry point. Run this preparation through an npm script.",
    );
  }
  run(process.execPath, [npmEntrypoint, ...args]);
}

const targetTriple = run("rustc", ["--print", "host-tuple"], true);
if (!targetTriple) throw new Error("Rust did not report a host target triple.");
if (targetTriple.includes("windows")) {
  const cargoArguments = ["build", "-p", "wyrmgrid-simconnect-provider"];
  if (release) cargoArguments.push("--release");
  run("cargo", cargoArguments);

  const executableName = "wyrmgrid-simconnect-provider.exe";
  const builtProvider = path.join(
    repositoryRoot,
    "target",
    profile,
    executableName,
  );
  const bundleDirectory = path.join(
    repositoryRoot,
    "apps",
    "desktop",
    "src-tauri",
    "binaries",
  );
  const bundledProvider = path.join(
    bundleDirectory,
    `wyrmgrid-simconnect-provider-${targetTriple}.exe`,
  );
  await mkdir(bundleDirectory, { recursive: true });
  await copyFile(builtProvider, bundledProvider);
  console.log(
    `Prepared simulator provider: ${path.relative(repositoryRoot, bundledProvider)}`,
  );
} else {
  console.log(
    `Skipped the Windows-only simulator provider on ${targetTriple}.`,
  );
}

if (buildFrontend) {
  runNpm(["run", "build", "--workspace", "@wyrmgrid/desktop"]);
}
