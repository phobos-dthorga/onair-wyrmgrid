import assert from "node:assert/strict";
import test from "node:test";
import {
  auditTranslationSource,
  declaredVersion,
  missingDispatchFindingMappings,
  sourceLine,
} from "./audit-localization.mjs";

const keys = new Set(["known-key", "mapped-key"]);

test("accepts literal catalogue-backed translation calls", () => {
  assert.deepEqual(auditTranslationSource('$translation("known-key")', keys), {
    unknownKeys: [],
    constructedKeys: [],
  });
});

test("reports unknown literal and constructed translation keys", () => {
  const result = auditTranslationSource(
    '$translation("missing-key")\ntranslate(`known-${suffix}`)',
    keys,
  );
  assert.deepEqual(
    result.unknownKeys.map(({ key }) => key),
    ["missing-key"],
  );
  assert.equal(result.constructedKeys.length, 1);
  assert.equal(sourceLine("first\nsecond", 6), 2);
});

test("reads compatibility versions and reports missing Dispatch mappings", () => {
  assert.equal(
    declaredVersion("SOURCE_CATALOG_VERSION: u32 = 13;", /u32 = (\d+)/),
    13,
  );
  assert.deepEqual(
    missingDispatchFindingMappings(
      'message_key: "dispatch-finding-known"; message_key: "dispatch-finding-missing";',
      '"dispatch-finding-known": keys("known-title", "known-detail")',
    ),
    ["dispatch-finding-missing"],
  );
});
