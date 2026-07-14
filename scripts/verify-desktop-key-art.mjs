import { createHash } from "node:crypto";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const manifestPath = path.join(
  repositoryRoot,
  "assets",
  "brand",
  "key-art",
  "derivatives",
  "manifest.json",
);
const buildDirectory = path.join(repositoryRoot, "apps", "desktop", "build");
const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
const expected = new Map(
  ["hero-dark", "hero-light"].map((name) => [
    manifest.derivatives[name].sha256,
    name,
  ]),
);

async function filesBelow(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = await Promise.all(
    entries.map(async (entry) => {
      const candidate = path.join(directory, entry.name);
      return entry.isDirectory() ? filesBelow(candidate) : [candidate];
    }),
  );
  return files.flat();
}

for (const file of await filesBelow(buildDirectory)) {
  if (path.extname(file).toLowerCase() !== ".png") continue;
  const hash = createHash("sha256")
    .update(await readFile(file))
    .digest("hex")
    .toUpperCase();
  expected.delete(hash);
}

if (expected.size > 0) {
  throw new Error(
    `Desktop build is missing packaged key art: ${[...expected.values()].join(", ")}`,
  );
}

console.log("Verified packaged desktop key art: hero-dark, hero-light");
