import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { buildPluginPackage } from "./package-plugin.mjs";

const repositoryRoot = resolve(import.meta.dirname, "..");
const sdkSource = resolve(
  repositoryRoot,
  "sdk/python/wyrmgrid_sdk/__init__.py",
);
const packages = [
  {
    source: "examples/plugins/fleet-locations",
    output: "fleet-locations.wyrmplugin",
  },
  {
    source: "plugins/open-meteo",
    output: "open-meteo.wyrmplugin",
  },
  {
    source: "plugins/aviation-weather",
    output: "aviation-weather.wyrmplugin",
  },
  {
    source: "plugins/rainviewer",
    output: "rainviewer.wyrmplugin",
  },
];

export async function prepareFirstPartyPluginPackages() {
  const results = [];
  for (const package_ of packages) {
    results.push(
      await buildPluginPackage({
        sourceDirectory: resolve(repositoryRoot, package_.source),
        outputPath: resolve(
          repositoryRoot,
          "assets/plugin-packages",
          package_.output,
        ),
        includes: [
          {
            source: sdkSource,
            destination: "src/wyrmgrid_sdk/__init__.py",
          },
        ],
        force: true,
      }),
    );
  }
  return results;
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  prepareFirstPartyPluginPackages()
    .then((results) => {
      for (const result of results) {
        console.log(
          `Prepared ${result.id} v${result.version}: ${result.archiveSha256}`,
        );
      }
    })
    .catch((error) => {
      console.error(error instanceof Error ? error.message : error);
      process.exitCode = 1;
    });
}
