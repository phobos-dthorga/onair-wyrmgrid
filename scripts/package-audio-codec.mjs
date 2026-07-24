import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { runPackageCli } from "./package-plugin.mjs";

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  runPackageCli("audio-codec", process.argv.slice(2)).catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  });
}
