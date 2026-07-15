import assert from "node:assert/strict";
import path from "node:path";
import test from "node:test";

import { cargoTargetDirectory } from "./cargo-target-directory.mjs";

test("uses the absolute target directory reported by Cargo", () => {
  const targetDirectory = path.resolve("fixture-cargo-target");

  assert.equal(
    cargoTargetDirectory(JSON.stringify({ target_directory: targetDirectory })),
    targetDirectory,
  );
});

test("rejects missing or relative Cargo target directories", () => {
  assert.throws(
    () => cargoTargetDirectory(JSON.stringify({})),
    /absolute target directory/,
  );
  assert.throws(
    () => cargoTargetDirectory(JSON.stringify({ target_directory: "target" })),
    /absolute target directory/,
  );
});

test("rejects malformed Cargo metadata", () => {
  assert.throws(
    () => cargoTargetDirectory("not json"),
    /invalid metadata JSON/,
  );
});
