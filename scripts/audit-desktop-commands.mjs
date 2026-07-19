import { readdir, readFile } from "node:fs/promises";
import { extname, join, relative, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const repositoryRoot = resolve(import.meta.dirname, "..");
const tauriLibraryPath = join(
  repositoryRoot,
  "apps",
  "desktop",
  "src-tauri",
  "src",
  "lib.rs",
);
const frontendRoot = join(repositoryRoot, "apps", "desktop", "src");

export function registeredTauriCommands(source) {
  const handler = source.match(/tauri::generate_handler!\[([\s\S]*?)\]\)/)?.[1];
  if (!handler) throw new Error("Tauri command handler registry was not found");
  return new Set(
    handler
      .split(",")
      .map((entry) => entry.trim())
      .filter((entry) => /^[a-z][a-z0-9_]*$/.test(entry)),
  );
}

export function invokedDesktopCommands(source) {
  const commands = [];
  const call = /invokeDesktop(?:<[^()]*>)?\s*\(\s*(["'])([a-z][a-z0-9_]*)\1/g;
  for (const match of source.matchAll(call)) commands.push(match[2]);
  return commands;
}

async function frontendFiles(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await frontendFiles(path)));
    else if ([".svelte", ".ts"].includes(extname(entry.name))) files.push(path);
  }
  return files;
}

export async function auditDesktopCommands() {
  const registered = registeredTauriCommands(
    await readFile(tauriLibraryPath, "utf8"),
  );
  const invoked = new Set();
  const failures = [];

  for (const path of await frontendFiles(frontendRoot)) {
    const source = await readFile(path, "utf8");
    const name = relative(repositoryRoot, path).replaceAll("\\", "/");
    if (
      name !== "apps/desktop/src/lib/desktop/client.ts" &&
      /(?:^|[^A-Za-z])invoke\s*(?:<[^()]*>)?\s*\(/m.test(source)
    ) {
      failures.push(`${name} bypasses the shared desktop command client`);
    }
    for (const command of invokedDesktopCommands(source)) {
      invoked.add(command);
      if (!registered.has(command)) {
        failures.push(
          `${name} invokes unregistered Tauri command '${command}'`,
        );
      }
    }
  }

  const unused = [...registered].filter((command) => !invoked.has(command));
  if (unused.length > 0) {
    failures.push(
      `Tauri registry exposes commands with no frontend client: ${unused.join(", ")}`,
    );
  }
  if (failures.length > 0) {
    throw new Error(`Desktop command audit failed:\n${failures.join("\n")}`);
  }
  return { registered: registered.size, invoked: invoked.size };
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  auditDesktopCommands()
    .then(({ registered, invoked }) => {
      console.log(
        `Desktop command audit passed: ${invoked} frontend commands match ${registered} registered Tauri handlers.`,
      );
    })
    .catch((error) => {
      console.error(error instanceof Error ? error.message : error);
      process.exitCode = 1;
    });
}
