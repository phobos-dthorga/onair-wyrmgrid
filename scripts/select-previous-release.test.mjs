import assert from "node:assert/strict";
import { Readable } from "node:stream";
import test from "node:test";

import {
  releaseTagsFromInput,
  selectPreviousReleaseFromInput,
} from "./select-previous-release.mjs";

test("reads GitHub release JSON used by the installer-upgrade check", () => {
  assert.deepEqual(
    releaseTagsFromInput(
      JSON.stringify([{ tagName: "v0.1.0" }, { tagName: "v0.2.0" }]),
    ),
    ["v0.1.0", "v0.2.0"],
  );
});

test("selects the previous tag from newline-delimited Git tags", async () => {
  const previousTag = await selectPreviousReleaseFromInput(
    "0.3.0",
    "tag-lines",
    Readable.from(["v0.1.0\nv0.2.0\nv0.3.0\nnotes\n"]),
  );

  assert.equal(previousTag, "v0.2.0");
});
