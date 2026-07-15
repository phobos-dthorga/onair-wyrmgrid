import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const defaultRepositoryRoot = resolve(scriptDirectory, "..");

function isNumericIdentifier(identifier) {
  return /^[0-9]+$/.test(identifier);
}

export function isSupportedReleaseVersion(version) {
  const match =
    /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-([0-9A-Za-z.-]+))?$/.exec(
      version,
    );

  if (!match) return false;

  const prerelease = match[4];
  if (!prerelease) return true;

  return prerelease.split(".").every((identifier) => {
    if (!identifier || !/^[0-9A-Za-z-]+$/.test(identifier)) return false;
    return !(
      isNumericIdentifier(identifier) &&
      identifier.length > 1 &&
      identifier.startsWith("0")
    );
  });
}

function workspaceVersion(cargoManifest) {
  const workspacePackage = /\[workspace\.package\]([\s\S]*?)(?=\n\[|$)/.exec(
    cargoManifest,
  )?.[1];
  const version = workspacePackage?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];

  if (!version) {
    throw new Error("Cargo.toml does not declare workspace.package.version");
  }

  return version;
}

export async function configuredReleaseVersions(
  repositoryRoot = defaultRepositoryRoot,
) {
  const [cargoManifest, rootPackage, desktopPackage, tauriConfiguration] =
    await Promise.all([
      readFile(resolve(repositoryRoot, "Cargo.toml"), "utf8"),
      readFile(resolve(repositoryRoot, "package.json"), "utf8").then(
        JSON.parse,
      ),
      readFile(
        resolve(repositoryRoot, "apps/desktop/package.json"),
        "utf8",
      ).then(JSON.parse),
      readFile(
        resolve(repositoryRoot, "apps/desktop/src-tauri/tauri.conf.json"),
        "utf8",
      ).then(JSON.parse),
    ]);

  return new Map([
    ["Cargo workspace", workspaceVersion(cargoManifest)],
    ["root npm package", rootPackage.version],
    ["desktop npm package", desktopPackage.version],
    ["Tauri application", tauriConfiguration.version],
  ]);
}

export async function verifyReleaseVersion(
  expectedVersion,
  repositoryRoot = defaultRepositoryRoot,
) {
  if (!isSupportedReleaseVersion(expectedVersion)) {
    throw new Error(
      `Unsupported release version '${expectedVersion}'. Use X.Y.Z or X.Y.Z-prerelease without build metadata.`,
    );
  }

  const versions = await configuredReleaseVersions(repositoryRoot);
  const mismatches = [...versions].filter(
    ([, version]) => version !== expectedVersion,
  );

  if (mismatches.length > 0) {
    const details = mismatches
      .map(([source, version]) => `${source}: ${version ?? "missing"}`)
      .join("\n");
    throw new Error(
      `Release tag version ${expectedVersion} does not match checked-in application versions:\n${details}`,
    );
  }

  return versions;
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : undefined;
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    const expectedVersion = process.argv[2];
    if (!expectedVersion) {
      throw new Error(
        "Usage: node scripts/verify-release-version.mjs <version>",
      );
    }

    const versions = await verifyReleaseVersion(expectedVersion);
    console.log(
      `Release version ${expectedVersion} matches ${versions.size} checked-in version sources.`,
    );
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
