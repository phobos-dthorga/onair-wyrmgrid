import { basename, resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { buildAudioCodecPackage } from "./package-plugin.mjs";

function parseArguments(arguments_) {
  const options = { includes: [], force: false };
  for (let index = 0; index < arguments_.length; index += 1) {
    const argument = arguments_[index];
    if (argument === "--source") options.sourceDirectory = arguments_[++index];
    else if (argument === "--output") options.outputPath = arguments_[++index];
    else if (argument === "--force") options.force = true;
    else if (argument === "--include") {
      const mapping = arguments_[++index] ?? "";
      const separator = mapping.lastIndexOf("=");
      if (separator <= 0 || separator === mapping.length - 1)
        throw new Error("--include requires SOURCE=PACKAGE_PATH");
      options.includes.push({
        source: mapping.slice(0, separator),
        destination: mapping.slice(separator + 1),
      });
    } else throw new Error(`Unknown argument: ${argument}`);
  }
  if (!options.sourceDirectory || !options.outputPath)
    throw new Error(
      "Usage: --source DIRECTORY --output FILE.wyrmcodec [--include SOURCE=PACKAGE_PATH] [--force]",
    );
  return options;
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  buildAudioCodecPackage(parseArguments(process.argv.slice(2)))
    .then((result) => {
      console.log(
        `Packaged ${result.id} v${result.version}: ${basename(result.output)} (${result.files} files, sha256 ${result.archiveSha256})`,
      );
    })
    .catch((error) => {
      console.error(error instanceof Error ? error.message : error);
      process.exitCode = 1;
    });
}
