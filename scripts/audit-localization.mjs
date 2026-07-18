import { readdir, readFile } from "node:fs/promises";
import { extname, join, relative, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const repositoryRoot = resolve(import.meta.dirname, "..");
const frontendRoot = join(repositoryRoot, "apps", "desktop", "src");
const sourceCataloguePath = join(repositoryRoot, "locales", "en-AU.json");
const sourceExtensions = new Set([".svelte", ".ts"]);

export function auditTranslationSource(source, catalogueKeys) {
  const unknownKeys = [];
  const constructedKeys = [];
  const literalCall =
    /(?:\$translation|\btranslate)\s*\(\s*(["'])([a-z0-9-]+)\1/g;
  const constructedCall = /(?:\$translation|\btranslate)\s*\(\s*`/g;

  for (const match of source.matchAll(literalCall)) {
    if (!catalogueKeys.has(match[2])) {
      unknownKeys.push({ key: match[2], index: match.index ?? 0 });
    }
  }
  for (const match of source.matchAll(constructedCall)) {
    constructedKeys.push({ index: match.index ?? 0 });
  }

  return { unknownKeys, constructedKeys };
}

export function sourceLine(source, index) {
  return source.slice(0, index).split("\n").length;
}

export function declaredVersion(source, pattern) {
  return Number(source.match(pattern)?.[1]);
}

export function missingDispatchFindingMappings(
  dispatchSource,
  localizationSource,
) {
  const keys = new Set(
    [...dispatchSource.matchAll(/"(dispatch-finding-[a-z-]+)"/g)].map(
      (match) => match[1],
    ),
  );
  return [...keys].filter((key) => !localizationSource.includes(`"${key}"`));
}

async function sourceFiles(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await sourceFiles(path)));
    else if (
      sourceExtensions.has(extname(entry.name)) &&
      !entry.name.endsWith(".test.ts")
    ) {
      files.push(path);
    }
  }
  return files;
}

export async function auditLocalization() {
  const catalogue = JSON.parse(await readFile(sourceCataloguePath, "utf8"));
  const catalogueKeys = new Set(Object.keys(catalogue.messages));
  const failures = [];

  for (const path of await sourceFiles(frontendRoot)) {
    const source = await readFile(path, "utf8");
    const result = auditTranslationSource(source, catalogueKeys);
    const name = relative(repositoryRoot, path).replaceAll("\\", "/");
    for (const finding of result.unknownKeys) {
      failures.push(
        `${name}:${sourceLine(source, finding.index)} unknown translation key '${finding.key}'`,
      );
    }
    for (const finding of result.constructedKeys) {
      failures.push(
        `${name}:${sourceLine(source, finding.index)} constructs a translation key; use an explicit typed mapping`,
      );
    }
  }

  const version = catalogue.source_catalog_version;
  const versionSources = [
    [
      "crates/application/src/localization.rs",
      /SOURCE_CATALOG_VERSION: u32 = (\d+)/,
    ],
    [
      "schemas/language-pack-v1.schema.json",
      /"source_catalog_version"\s*:\s*\{\s*"const"\s*:\s*(\d+)/,
    ],
    [
      "schemas/fixtures/language-pack-v1.json",
      /"source_catalog_version"\s*:\s*(\d+)/,
    ],
  ];
  for (const [name, pattern] of versionSources) {
    const source = await readFile(join(repositoryRoot, name), "utf8");
    const candidate = declaredVersion(source, pattern);
    if (candidate !== version) {
      failures.push(
        `${name} declares source catalogue version ${candidate || "unknown"}; expected ${version}`,
      );
    }
  }

  const dispatchSource = await readFile(
    join(repositoryRoot, "crates", "application", "src", "dispatch.rs"),
    "utf8",
  );
  const dispatchLocalization = await readFile(
    join(frontendRoot, "lib", "dispatch", "localization.ts"),
    "utf8",
  );
  for (const key of missingDispatchFindingMappings(
    dispatchSource,
    dispatchLocalization,
  )) {
    failures.push(
      `apps/desktop/src/lib/dispatch/localization.ts does not map application finding '${key}'`,
    );
  }

  if (failures.length > 0) {
    throw new Error(`Localization audit failed:\n${failures.join("\n")}`);
  }
  return {
    files: (await sourceFiles(frontendRoot)).length,
    keys: catalogueKeys.size,
    version,
  };
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  auditLocalization()
    .then(({ files, keys, version }) => {
      console.log(
        `Localization audit passed: ${keys} catalogue keys across ${files} frontend sources (catalogue v${version}).`,
      );
    })
    .catch((error) => {
      console.error(error instanceof Error ? error.message : error);
      process.exitCode = 1;
    });
}
