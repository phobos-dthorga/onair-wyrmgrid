import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const defaultRepositoryRoot = resolve(scriptDirectory, "..");

export const installerIdentity = Object.freeze({
  productName: "OnAir WyrmGrid",
  identifier: "io.github.phobosdthorga.onairwyrmgrid",
  installMode: "currentUser",
});

export function verifyInstallerConfiguration(configuration) {
  const failures = [];

  if (configuration.productName !== installerIdentity.productName) {
    failures.push(
      `productName must remain '${installerIdentity.productName}' for in-place upgrades`,
    );
  }
  if (configuration.identifier !== installerIdentity.identifier) {
    failures.push(
      `identifier must remain '${installerIdentity.identifier}' to preserve installer and AppData identity`,
    );
  }
  if (!configuration.bundle?.targets?.includes("nsis")) {
    failures.push("bundle.targets must include nsis");
  }
  if (
    configuration.bundle?.resources?.["extension-developer-kit/"] !==
    "extension-developer-kit/"
  ) {
    failures.push(
      "bundle.resources must install the platform-neutral Extension Developer Kit directory",
    );
  }
  if (configuration.bundle?.windows?.allowDowngrades !== false) {
    failures.push("bundle.windows.allowDowngrades must be false");
  }
  if (
    configuration.bundle?.windows?.nsis?.installMode !==
    installerIdentity.installMode
  ) {
    failures.push(
      `bundle.windows.nsis.installMode must remain '${installerIdentity.installMode}'`,
    );
  }

  if (failures.length > 0) {
    throw new Error(
      `NSIS upgrade contract is invalid:\n${failures.join("\n")}`,
    );
  }

  return configuration;
}

export async function verifyInstallerContract(
  repositoryRoot = defaultRepositoryRoot,
) {
  const configuration = JSON.parse(
    await readFile(
      resolve(repositoryRoot, "apps/desktop/src-tauri/tauri.conf.json"),
      "utf8",
    ),
  );
  return verifyInstallerConfiguration(configuration);
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : undefined;
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    await verifyInstallerContract();
    console.log(
      `Verified NSIS upgrade identity ${installerIdentity.identifier} (${installerIdentity.installMode}).`,
    );
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
