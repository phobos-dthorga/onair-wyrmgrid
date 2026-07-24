import assert from "node:assert/strict";
import { readFile, rm } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { join } from "node:path";
import test from "node:test";
import { prepareBundledEdk } from "./prepare-bundled-edk.mjs";

test("prepares the exact installable EDK surface for desktop bundles", async () => {
  const target = join(
    process.cwd(),
    "apps",
    "desktop",
    "src-tauri",
    `.test-edk-${randomUUID()}`,
  );

  try {
    const prepared = await prepareBundledEdk(target);
    assert.equal(prepared.name, "@wyrmgrid/extension-developer-kit");
    assert.equal(prepared.version, "1.0.0");

    const manifest = JSON.parse(
      await readFile(join(target, "package.json"), "utf8"),
    );
    assert.equal(manifest.version, prepared.version);
    assert.match(
      await readFile(join(target, "BUNDLED-README.txt"), "utf8"),
      /npm install --global "<this directory>"/,
    );
    await readFile(join(target, "bin", "wyrmgrid-extension.mjs"));
    await readFile(join(target, "schemas", "schema-catalog-v1.json"));
    await readFile(
      join(target, "sdks", "python", "wyrmgrid_sdk", "__init__.py"),
    );
  } finally {
    await rm(target, { force: true, recursive: true });
  }
});

test("refuses to replace any other desktop source directory", async () => {
  await assert.rejects(
    prepareBundledEdk(
      join(process.cwd(), "apps", "desktop", "src-tauri", "src"),
    ),
    /must be the dedicated generated resource directory/,
  );
});
