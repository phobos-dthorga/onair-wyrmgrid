import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import { cargoTargetDirectory } from "./cargo-target-directory.mjs";
import { buildSimulatorProviderPackage } from "./package-plugin.mjs";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const distribution = process.argv.includes("--distribution");
const release = process.argv.includes("--release") || distribution;
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

  const targetDirectory = cargoTargetDirectory(
    run("cargo", ["metadata", "--format-version", "1", "--no-deps"], true),
  );

  const executableName = "wyrmgrid-simconnect-provider.exe";
  const builtProvider = path.join(targetDirectory, profile, executableName);
  const providerDirectory = path.join(
    repositoryRoot,
    "providers",
    "msfs2024-simconnect",
  );
  const packagePath = distribution
    ? path.join(
        repositoryRoot,
        "assets",
        "provider-packages",
        "msfs2024-simconnect.wyrmprovider",
      )
    : path.join(
        repositoryRoot,
        "apps",
        "desktop",
        "src-tauri",
        "provider-packages",
        "msfs2024-simconnect.wyrmprovider",
      );
  const packaged = await buildSimulatorProviderPackage({
    sourceDirectory: providerDirectory,
    outputPath: packagePath,
    includeSourceDirectory: false,
    includes: [
      {
        source: path.join(providerDirectory, "provider.json"),
        destination: "provider.json",
      },
      { source: builtProvider, destination: executableName },
    ],
    force: true,
  });
  console.log(
    `Prepared simulator provider package: ${path.relative(repositoryRoot, packagePath)} (sha256 ${packaged.archiveSha256})`,
  );
} else {
  console.log(
    `Skipped the Windows-only simulator provider on ${targetTriple}.`,
  );
}

if (buildFrontend) {
  runNpm(["run", "build", "--workspace", "@wyrmgrid/desktop"]);
}
