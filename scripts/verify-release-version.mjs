import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const defaultRepositoryRoot = resolve(scriptDirectory, "..");

function isNumericIdentifier(identifier) {
  return /^[0-9]+$/.test(identifier);
}

export function parseSupportedReleaseVersion(version) {
  const match =
    /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-([0-9A-Za-z.-]+))?$/.exec(
      version,
    );

  if (!match) return undefined;

  const prerelease = match[4];
  const prereleaseIdentifiers = prerelease ? prerelease.split(".") : [];

  const validPrerelease = prereleaseIdentifiers.every((identifier) => {
    if (!identifier || !/^[0-9A-Za-z-]+$/.test(identifier)) return false;
    return !(
      isNumericIdentifier(identifier) &&
      identifier.length > 1 &&
      identifier.startsWith("0")
    );
  });

  if (!validPrerelease) return undefined;

  return {
    core: match.slice(1, 4),
    prerelease: prereleaseIdentifiers,
  };
}

export function isSupportedReleaseVersion(version) {
  return parseSupportedReleaseVersion(version) !== undefined;
}

function compareNumericIdentifiers(left, right) {
  if (left.length !== right.length) return left.length < right.length ? -1 : 1;
  return left === right ? 0 : left < right ? -1 : 1;
}

export function compareSupportedReleaseVersions(left, right) {
  const parsedLeft = parseSupportedReleaseVersion(left);
  const parsedRight = parseSupportedReleaseVersion(right);
  if (!parsedLeft || !parsedRight) {
    throw new Error("Only supported release versions can be compared");
  }

  for (let index = 0; index < parsedLeft.core.length; index += 1) {
    const comparison = compareNumericIdentifiers(
      parsedLeft.core[index],
      parsedRight.core[index],
    );
    if (comparison !== 0) return comparison;
  }

  if (
    parsedLeft.prerelease.length === 0 &&
    parsedRight.prerelease.length === 0
  ) {
    return 0;
  }
  if (parsedLeft.prerelease.length === 0) return 1;
  if (parsedRight.prerelease.length === 0) return -1;

  const identifiers = Math.max(
    parsedLeft.prerelease.length,
    parsedRight.prerelease.length,
  );
  for (let index = 0; index < identifiers; index += 1) {
    const leftIdentifier = parsedLeft.prerelease[index];
    const rightIdentifier = parsedRight.prerelease[index];
    if (leftIdentifier === undefined) return -1;
    if (rightIdentifier === undefined) return 1;
    if (leftIdentifier === rightIdentifier) continue;

    const leftNumeric = isNumericIdentifier(leftIdentifier);
    const rightNumeric = isNumericIdentifier(rightIdentifier);
    if (leftNumeric && rightNumeric) {
      return compareNumericIdentifiers(leftIdentifier, rightIdentifier);
    }
    if (leftNumeric) return -1;
    if (rightNumeric) return 1;
    return leftIdentifier < rightIdentifier ? -1 : 1;
  }

  return 0;
}

export function selectPreviousReleaseTag(currentVersion, tags) {
  if (!isSupportedReleaseVersion(currentVersion)) {
    throw new Error(`Unsupported current release version '${currentVersion}'`);
  }

  return tags
    .filter((tag) => tag.startsWith("v"))
    .map((tag) => ({ tag, version: tag.slice(1) }))
    .filter(
      ({ version }) =>
        isSupportedReleaseVersion(version) &&
        compareSupportedReleaseVersions(version, currentVersion) < 0,
    )
    .sort((left, right) =>
      compareSupportedReleaseVersions(right.version, left.version),
    )[0]?.tag;
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
