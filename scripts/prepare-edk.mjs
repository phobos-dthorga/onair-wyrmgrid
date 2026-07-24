import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const packageRoot = join(repositoryRoot, "packages", "extension-developer-kit");
const sourceSchemaDirectory = join(repositoryRoot, "schemas");
const targetSchemaDirectory = join(packageRoot, "schemas");
const schemaFiles = [
  "plugin-manifest.schema.json",
  "extension-package-manifest-v1.schema.json",
  "simulator-provider-manifest-v1.schema.json",
  "simulator-provider-package-manifest-v1.schema.json",
  "audio-provider-manifest-v2.schema.json",
  "audio-provider-package-manifest-v1.schema.json",
  "audio-codec-manifest-v1.schema.json",
  "audio-codec-package-manifest-v1.schema.json",
  "extension-compatibility-report-v1.schema.json",
];

function sha256(contents) {
  return createHash("sha256").update(contents).digest("hex");
}

export async function prepareEdk() {
  await mkdir(targetSchemaDirectory, { recursive: true });
  const schemas = [];
  for (const filename of schemaFiles) {
    const source = join(sourceSchemaDirectory, filename);
    const target = join(targetSchemaDirectory, filename);
    const contents = await readFile(source);
    const schema = JSON.parse(contents.toString("utf8"));
    await writeFile(target, contents);
    schemas.push({
      filename,
      id: schema.$id,
      sha256: sha256(contents),
    });
  }
  await writeFile(
    join(targetSchemaDirectory, "schema-catalog-v1.json"),
    `${JSON.stringify({ schema_version: 1, schemas }, null, 2)}\n`,
    "utf8",
  );
  await copyFile(join(repositoryRoot, "LICENSE"), join(packageRoot, "LICENSE"));
  return { packageRoot, schemas };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  prepareEdk()
    .then(({ schemas }) => {
      console.log(`Prepared EDK v1 with ${schemas.length} public schemas.`);
    })
    .catch((error) => {
      console.error(error instanceof Error ? error.message : error);
      process.exitCode = 1;
    });
}
