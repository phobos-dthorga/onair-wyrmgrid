import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { prepareEdk } from "./prepare-edk.mjs";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(scriptDirectory, "..");
const defaultTargetDirectory = join(
  repositoryRoot,
  "apps",
  "desktop",
  "src-tauri",
  "extension-developer-kit",
);
const bundledEntries = [
  "bin",
  "src",
  "schemas",
  "sdks",
  "LICENSE",
  "package.json",
  "README.md",
];

function assertSafeTarget(targetDirectory) {
  const expectedParent = resolve(
    repositoryRoot,
    "apps",
    "desktop",
    "src-tauri",
  );
  const resolvedTarget = resolve(targetDirectory);
  const relativeTarget = relative(expectedParent, resolvedTarget);
  const allowedTestTarget =
    !relativeTarget.includes(sep) &&
    /^\.test-edk-[0-9a-f]{8}-[0-9a-f-]{27}$/u.test(relativeTarget);
  if (
    isAbsolute(relativeTarget) ||
    (relativeTarget !== "extension-developer-kit" && !allowedTestTarget)
  ) {
    throw new Error(
      "The bundled EDK target must be the dedicated generated resource directory",
    );
  }
  return resolvedTarget;
}

export async function prepareBundledEdk(
  targetDirectory = defaultTargetDirectory,
) {
  const target = assertSafeTarget(targetDirectory);
  const { packageRoot } = await prepareEdk();
  const packageManifest = JSON.parse(
    await readFile(join(packageRoot, "package.json"), "utf8"),
  );

  await rm(target, { force: true, recursive: true });
  await mkdir(target, { recursive: true });
  for (const entry of bundledEntries) {
    await cp(join(packageRoot, entry), join(target, entry), {
      recursive: true,
    });
  }
  await writeFile(
    join(target, "BUNDLED-README.txt"),
    [
      `WyrmGrid Extension Developer Kit ${packageManifest.version}`,
      "",
      "This is the same platform-neutral npm package distributed separately.",
      "It includes the zero-dependency Python plugin SDK.",
      "Node.js 22.12 or newer is required.",
      "",
      "Install this bundled copy from a terminal:",
      '  npm install --global "<this directory>"',
      "",
      "Open https://wyrmgr.id/ for current WyrmGrid documentation.",
      "",
    ].join("\n"),
    "utf8",
  );

  return {
    name: packageManifest.name,
    version: packageManifest.version,
    target,
    entries: [...bundledEntries, "BUNDLED-README.txt"],
  };
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : undefined;
if (invokedPath && import.meta.url === pathToFileURL(invokedPath).href) {
  prepareBundledEdk()
    .then(({ version }) => {
      console.log(`Prepared bundled Extension Developer Kit v${version}.`);
    })
    .catch((error) => {
      console.error(error instanceof Error ? error.message : String(error));
      process.exitCode = 1;
    });
}
