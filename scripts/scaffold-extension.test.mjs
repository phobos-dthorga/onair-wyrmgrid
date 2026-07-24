import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";

import { scaffoldExtension } from "./scaffold-extension.mjs";

async function temporaryTarget(name) {
  const root = await mkdtemp(join(tmpdir(), "wyrmgrid-scaffold-"));
  return join(root, name);
}

test("scaffolds each independently installable extension kind", async () => {
  const cases = [
    ["plugin", "plugin.json"],
    ["simulator-provider", "provider.json"],
    ["audio-provider", "audio-provider.json"],
    ["audio-codec", "audio-codec.json"],
  ];
  for (const [kind, manifestName] of cases) {
    const directory = await temporaryTarget(kind);
    const result = await scaffoldExtension({
      kind,
      directory,
      id: `org.example.wyrmgrid-${kind}`,
      name: `Example ${kind}`,
      author: "Example Author",
    });
    assert.equal(result.kind, kind);
    const manifest = JSON.parse(
      await readFile(join(directory, manifestName), "utf8"),
    );
    assert.equal(manifest.id, `org.example.wyrmgrid-${kind}`);
    assert.equal(manifest.version, "0.1.0");
    assert.match(
      await readFile(join(directory, "README.md"), "utf8"),
      /independently installable artifact/,
    );
  }
});

test("refuses to overwrite a non-empty target", async () => {
  const directory = await temporaryTarget("occupied");
  await mkdir(directory);
  await writeFile(join(directory, "keep.txt"), "keep me");
  await assert.rejects(
    scaffoldExtension({
      kind: "audio-codec",
      directory,
      id: "org.example.codec",
      name: "Example codec",
      author: "Example Author",
    }),
    /must be empty/,
  );
  assert.equal(await readFile(join(directory, "keep.txt"), "utf8"), "keep me");
});

test("rejects unsafe identities before creating files", async () => {
  const directory = await temporaryTarget("invalid");
  await assert.rejects(
    scaffoldExtension({
      kind: "plugin",
      directory,
      id: "../unsafe",
      name: "Unsafe",
      author: "Example Author",
    }),
    /reverse-domain/,
  );
});
