import { spawnSync } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { cargoTargetDirectory } from "./cargo-target-directory.mjs";
import { buildAudioProviderPackage } from "./package-plugin.mjs";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const release = process.argv.includes("--distribution");
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

const targetTriple = run("rustc", ["--print", "host-tuple"], true);
if (!targetTriple) throw new Error("Rust did not report a host target triple.");
if (!targetTriple.includes("windows")) {
  throw new Error(
    `The current reference audio provider artifact is Windows-only; received ${targetTriple}.`,
  );
}

const cargoArguments = ["build", "-p", "wyrmgrid-fake-audio-provider"];
if (release) cargoArguments.push("--release");
run("cargo", cargoArguments);

const targetDirectory = cargoTargetDirectory(
  run("cargo", ["metadata", "--format-version", "1", "--no-deps"], true),
);
const executableName = "wyrmgrid-fake-audio-provider.exe";
const builtProvider = path.join(targetDirectory, profile, executableName);
const providerDirectory = path.join(repositoryRoot, "providers", "fake-audio");
const sourceManifest = JSON.parse(
  await readFile(path.join(providerDirectory, "provider.json"), "utf8"),
);
const packageManifest = {
  ...sourceManifest,
  platforms: ["windows_x86_64"],
};
const staging = await mkdtemp(
  path.join(os.tmpdir(), "wyrmgrid-audio-provider-"),
);
try {
  const manifestPath = path.join(staging, "audio-provider.json");
  await writeFile(
    manifestPath,
    `${JSON.stringify(packageManifest, null, 2)}\n`,
    "utf8",
  );
  const packagePath = path.join(
    repositoryRoot,
    "assets",
    "audio-provider-packages",
    "deterministic-fake-audio.wyrmaudio",
  );
  const packaged = await buildAudioProviderPackage({
    sourceDirectory: providerDirectory,
    outputPath: packagePath,
    includeSourceDirectory: false,
    includes: [
      { source: manifestPath, destination: "audio-provider.json" },
      { source: builtProvider, destination: executableName },
    ],
    force: true,
  });
  console.log(
    `Prepared audio provider package: ${path.relative(repositoryRoot, packagePath)} (sha256 ${packaged.archiveSha256})`,
  );
} finally {
  await rm(staging, { recursive: true, force: true });
}
