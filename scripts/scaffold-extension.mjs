import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

export * from "../packages/extension-developer-kit/src/scaffold.mjs";
import { runScaffoldCli } from "../packages/extension-developer-kit/src/scaffold.mjs";

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  runScaffoldCli(process.argv.slice(2)).catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  });
}
