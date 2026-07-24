import { constants } from "node:fs";
import { copyFile, lstat, mkdir, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const schemaDirectory = fileURLToPath(new URL("../schemas/", import.meta.url));

export async function schemaCatalog() {
  return JSON.parse(
    await readFile(join(schemaDirectory, "schema-catalog-v1.json"), "utf8"),
  );
}

export async function copySchemaBundle(outputDirectory, force = false) {
  const output = resolve(outputDirectory);
  await mkdir(output, { recursive: true });
  const catalog = await schemaCatalog();
  const filenames = [
    ...catalog.schemas.map((schema) => schema.filename),
    "schema-catalog-v1.json",
  ];
  for (const filename of filenames)
    try {
      const metadata = await lstat(join(output, filename));
      if (metadata.isSymbolicLink() || !metadata.isFile())
        throw new Error(`Schema output is not a regular file: ${filename}`);
      if (!force) throw new Error(`Schema output already exists: ${filename}`);
    } catch (error) {
      if (error?.code !== "ENOENT") throw error;
    }
  for (const schema of catalog.schemas)
    await copyFile(
      join(schemaDirectory, schema.filename),
      join(output, schema.filename),
      force ? 0 : constants.COPYFILE_EXCL,
    );
  await copyFile(
    join(schemaDirectory, "schema-catalog-v1.json"),
    join(output, "schema-catalog-v1.json"),
    force ? 0 : constants.COPYFILE_EXCL,
  );
  return { output, catalog };
}
