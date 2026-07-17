import assert from "node:assert/strict";
import test from "node:test";

import { normaliseAdministrativeFeature } from "./prepare-atlas-regions.mjs";

test("normalises sourced ADM1 facts without inventing local labels", () => {
  const feature = normaliseAdministrativeFeature(
    {
      type: "Feature",
      properties: {
        ne_id: 42,
        name: "Sourced name",
        name_en: "English name",
        name_local: null,
        type_en: "Province",
        admin: "Exampleland",
        adm0_a3: "EXP",
        iso_3166_2: "EX-01",
        min_zoom: 3.5,
        labelrank: 4,
      },
      geometry: {
        type: "Polygon",
        coordinates: [
          [
            [0, 0],
            [1, 0],
            [1, 1],
            [0, 0],
          ],
        ],
      },
    },
    0,
  );

  assert.equal(feature.properties.region_id, "natural-earth:42");
  assert.equal(feature.properties.name, "English name");
  assert.equal(feature.properties.name_local, null);
  assert.equal(feature.properties.local_type, "Province");
  assert.equal(feature.properties.country_name, "Exampleland");
  assert.equal(feature.properties.level, "ADM1");
  assert.equal(feature.properties.source, "Natural Earth");
});

test("rejects unnamed and non-polygonal source features", () => {
  assert.equal(
    normaliseAdministrativeFeature(
      {
        properties: {},
        geometry: { type: "Polygon", coordinates: [] },
      },
      0,
    ),
    null,
  );
  assert.equal(
    normaliseAdministrativeFeature(
      {
        properties: { name: "Point region" },
        geometry: { type: "Point", coordinates: [0, 0] },
      },
      0,
    ),
    null,
  );
});
