import { spawnSync } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { cargoTargetDirectory } from "./cargo-target-directory.mjs";
import { buildAudioCodecPackage } from "./package-plugin.mjs";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const distribution = process.argv.includes("--distribution");
const release = process.argv.includes("--release") || distribution;
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

const platform =
  targetTriple.startsWith("x86_64-") && targetTriple.includes("windows")
    ? "windows_x86_64"
    : targetTriple.startsWith("x86_64-") && targetTriple.includes("linux")
      ? "linux_x86_64"
      : targetTriple === "aarch64-apple-darwin"
        ? "macos_aarch64"
        : targetTriple === "x86_64-apple-darwin"
          ? "macos_x86_64"
          : undefined;
if (!platform) {
  throw new Error(`Unsupported audio codec package target: ${targetTriple}`);
}
const executableName =
  platform === "windows_x86_64"
    ? "wyrmgrid-opus-codec.exe"
    : "wyrmgrid-opus-codec";
const cargoArguments = ["build", "-p", "wyrmgrid-opus-codec"];
if (release) cargoArguments.push("--release");
run("cargo", cargoArguments);

const targetDirectory = cargoTargetDirectory(
  run("cargo", ["metadata", "--format-version", "1", "--no-deps"], true),
);
const builtCodec = path.join(targetDirectory, profile, executableName);
const codecDirectory = path.join(repositoryRoot, "codecs", "opus");
const sourceManifest = JSON.parse(
  await readFile(path.join(codecDirectory, "codec.json"), "utf8"),
);
const packageManifest = {
  ...sourceManifest,
  $schema: "../../schemas/audio-codec-manifest-v1.schema.json",
  platforms: [platform],
};
const staging = await mkdtemp(path.join(os.tmpdir(), "wyrmgrid-audio-codec-"));
try {
  const manifestPath = path.join(staging, "audio-codec.json");
  await writeFile(
    manifestPath,
    `${JSON.stringify(packageManifest, null, 2)}\n`,
    "utf8",
  );
  const packagePath = distribution
    ? path.join(repositoryRoot, "assets", "codec-packages", "opus.wyrmcodec")
    : path.join(
        repositoryRoot,
        "apps",
        "desktop",
        "src-tauri",
        "codec-packages",
        "opus.wyrmcodec",
      );
  const packaged = await buildAudioCodecPackage({
    sourceDirectory: codecDirectory,
    outputPath: packagePath,
    includeSourceDirectory: false,
    includes: [
      { source: manifestPath, destination: "audio-codec.json" },
      { source: builtCodec, destination: executableName },
    ],
    force: true,
  });
  console.log(
    `Prepared audio codec package: ${path.relative(repositoryRoot, packagePath)} (sha256 ${packaged.archiveSha256})`,
  );
} finally {
  await rm(staging, { recursive: true, force: true });
}
