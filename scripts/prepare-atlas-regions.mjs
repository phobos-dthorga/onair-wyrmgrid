import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const NATURAL_EARTH_VERSION = "5.1.2";
const NATURAL_EARTH_COMMIT = "9380cca83db5f9aef52d5e762765100745f84b27";
const NATURAL_EARTH_SOURCE_SHA256 =
  "22d0e3ad85eb3e27f17cabf8ba2d50e554fbc27a87796ff891d958185da62fb5";
const MAPSHAPER_VERSION = "0.7.45";
const SIMPLIFICATION_PERCENT = "8%";
const SOURCE_URL = `https://raw.githubusercontent.com/nvkelso/natural-earth-vector/${NATURAL_EARTH_COMMIT}/geojson/ne_10m_admin_1_states_provinces.geojson`;

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const outputDirectory = path.join(
  repositoryRoot,
  "apps",
  "desktop",
  "static",
  "data",
  "atlas",
);
const outputFile = path.join(
  outputDirectory,
  `admin1-natural-earth-${NATURAL_EARTH_VERSION}.geojson`,
);
const manifestFile = path.join(outputDirectory, "admin1-manifest.json");

function optionalString(value) {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function optionalNumber(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

export function normaliseAdministrativeFeature(feature, index) {
  const source = feature?.properties ?? {};
  const name = optionalString(source.name_en) ?? optionalString(source.name);
  const geometry = feature?.geometry;
  if (
    !name ||
    !geometry ||
    !["Polygon", "MultiPolygon"].includes(geometry.type)
  ) {
    return null;
  }

  const naturalEarthId = source.ne_id ?? source.adm1_code ?? index;
  const regionId = `natural-earth:${String(naturalEarthId)}`;
  return {
    type: "Feature",
    properties: {
      region_id: regionId,
      level: "ADM1",
      name,
      name_local: optionalString(source.name_local),
      local_type: optionalString(source.type_en) ?? optionalString(source.type),
      country_name: optionalString(source.admin),
      country_code: optionalString(source.adm0_a3),
      subdivision_code: optionalString(source.iso_3166_2),
      label_min_zoom: optionalNumber(source.min_zoom),
      label_rank: optionalNumber(source.labelrank),
      source: "Natural Earth",
      source_version: NATURAL_EARTH_VERSION,
    },
    geometry,
  };
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function runMapshaper(inputFile, simplifiedFile) {
  const npmEntrypoint = process.env.npm_execpath;
  if (!npmEntrypoint) {
    throw new Error(
      "npm did not report its JavaScript entry point. Run this preparation through npm run atlas:regions.",
    );
  }
  const result = spawnSync(
    process.execPath,
    [
      npmEntrypoint,
      "exec",
      "--yes",
      "--package",
      `mapshaper@${MAPSHAPER_VERSION}`,
      "--",
      "mapshaper",
      inputFile,
      "-simplify",
      SIMPLIFICATION_PERCENT,
      "weighted",
      "keep-shapes",
      "-filter-fields",
      "ne_id,adm1_code,name,name_en,name_local,type,type_en,admin,adm0_a3,iso_3166_2,min_zoom,labelrank",
      "-o",
      "format=geojson",
      "precision=0.00001",
      simplifiedFile,
    ],
    { cwd: repositoryRoot, stdio: "inherit" },
  );
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(
      `Mapshaper exited with code ${result.status ?? "unknown"}.`,
    );
  }
}

async function main() {
  const response = await fetch(SOURCE_URL);
  if (!response.ok) {
    throw new Error(
      `Natural Earth download failed with HTTP ${response.status}.`,
    );
  }
  const sourceBytes = Buffer.from(await response.arrayBuffer());
  const sourceHash = sha256(sourceBytes);
  if (sourceHash !== NATURAL_EARTH_SOURCE_SHA256) {
    throw new Error(
      `Natural Earth source hash mismatch: expected ${NATURAL_EARTH_SOURCE_SHA256}, received ${sourceHash}.`,
    );
  }

  const temporaryDirectory = await mkdtemp(
    path.join(os.tmpdir(), "wyrmgrid-atlas-regions-"),
  );
  try {
    const sourceFile = path.join(temporaryDirectory, "admin1-source.geojson");
    const simplifiedFile = path.join(
      temporaryDirectory,
      "admin1-simplified.geojson",
    );
    await writeFile(sourceFile, sourceBytes);
    runMapshaper(sourceFile, simplifiedFile);

    const simplified = JSON.parse(await readFile(simplifiedFile, "utf8"));
    const features = simplified.features
      .map(normaliseAdministrativeFeature)
      .filter(Boolean);
    const dataset = {
      type: "FeatureCollection",
      wyrmgrid: {
        schema_version: 1,
        level: "ADM1",
        source: "Natural Earth",
        source_version: NATURAL_EARTH_VERSION,
        boundary_view: "de_facto",
      },
      features,
    };
    const output = `${JSON.stringify(dataset)}\n`;
    const outputBytes = Buffer.from(output);

    await mkdir(outputDirectory, { recursive: true });
    await writeFile(outputFile, outputBytes);
    await writeFile(
      manifestFile,
      `${JSON.stringify(
        {
          schema_version: 1,
          generated_file: path.basename(outputFile),
          generated_sha256: sha256(outputBytes),
          feature_count: features.length,
          administrative_level: "ADM1",
          source: "Natural Earth",
          source_version: NATURAL_EARTH_VERSION,
          source_commit: NATURAL_EARTH_COMMIT,
          source_url: SOURCE_URL,
          source_sha256: NATURAL_EARTH_SOURCE_SHA256,
          source_scale: "1:10m",
          simplification: {
            tool: "mapshaper",
            version: MAPSHAPER_VERSION,
            retained: SIMPLIFICATION_PERCENT,
            method: "weighted",
            keep_shapes: true,
            coordinate_precision: 0.00001,
          },
          licence: "Public domain",
          attribution: "Made with Natural Earth",
          boundary_view: "de_facto",
          navigation_authority: false,
        },
        null,
        2,
      )}\n`,
    );
    console.log(
      `Prepared ${features.length.toLocaleString()} ADM1 regions at ${path.relative(repositoryRoot, outputFile)}.`,
    );
  } finally {
    await rm(temporaryDirectory, { recursive: true, force: true });
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await main();
}
