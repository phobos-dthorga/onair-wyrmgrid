import { createHash } from "node:crypto";
import { access, readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { EDK_VERSION } from "./catalog.mjs";
import { schemaCatalog } from "./schemas.mjs";

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const schemaDirectory = join(packageRoot, "schemas");

function sha256(contents) {
  return createHash("sha256").update(contents).digest("hex");
}

export async function verifyPackageContents() {
  const packageManifest = JSON.parse(
    await readFile(join(packageRoot, "package.json"), "utf8"),
  );
  if (packageManifest.version !== EDK_VERSION)
    throw new Error("EDK package and runtime versions do not match");

  const catalog = await schemaCatalog();
  if (
    catalog.schema_version !== 1 ||
    !Array.isArray(catalog.schemas) ||
    catalog.schemas.length === 0
  )
    throw new Error("EDK schema catalogue is missing or incompatible");

  for (const entry of catalog.schemas) {
    if (
      typeof entry.filename !== "string" ||
      !/^[a-z0-9][a-z0-9-]*\.schema\.json$/u.test(entry.filename)
    )
      throw new Error("EDK schema catalogue contains an unsafe filename");
    const contents = await readFile(join(schemaDirectory, entry.filename));
    const schema = JSON.parse(
      new TextDecoder("utf-8", { fatal: true }).decode(contents),
    );
    if (schema.$id !== entry.id || sha256(contents) !== entry.sha256)
      throw new Error(`EDK schema bundle mismatch: ${entry.filename}`);
  }

  await Promise.all([
    access(join(packageRoot, "README.md")),
    access(join(packageRoot, "LICENSE")),
    access(join(packageRoot, "sdks", "python", "README.md")),
    access(join(packageRoot, "sdks", "python", "wyrmgrid_sdk", "__init__.py")),
  ]);
  return {
    version: EDK_VERSION,
    schemas: catalog.schemas.length,
  };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  verifyPackageContents()
    .then(({ version, schemas }) => {
      console.log(
        `Verified EDK ${version} release contents with ${schemas} schemas.`,
      );
    })
    .catch((error) => {
      console.error(error instanceof Error ? error.message : error);
      process.exitCode = 1;
    });
}
