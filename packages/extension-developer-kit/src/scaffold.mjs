import { mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { EXTENSIONS } from "./catalog.mjs";
import { ID_PATTERN, VERSION_PATTERN } from "./contract.mjs";

const KINDS = new Set(Object.keys(EXTENSIONS));
const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const pythonSdkPath = join(
  packageRoot,
  "sdks",
  "python",
  "wyrmgrid_sdk",
  "__init__.py",
);

function json(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function executableStem(id, suffix) {
  const leaf = id.split(".").at(-1);
  return `wyrmgrid-${leaf}-${suffix}`.replaceAll(/[^a-z0-9-]/g, "-");
}

const IGNORE_FILE = `.git/
dist/
target/
node_modules/
.venv/
__pycache__/
.env
`;

function commonReadme({ kind, id, name, packageCommand, extension, notes }) {
  return `# ${name}

This is a WyrmGrid ${kind} scaffold for \`${id}\`.

## Before packaging

${notes}

The manifest is part of the public compatibility contract. Keep its protocol
version compatible with the WyrmGrid documentation and increment your package
version whenever published contents change.

## Package

With Extension Developer Kit v1 installed:

\`\`\`powershell
${packageCommand}
\`\`\`

The resulting \`${extension}\` file is the independently installable artifact
to distribute. Test it in a disposable WyrmGrid profile before publishing it.
Do not place credentials, personal data, symlinks, or unrelated files in the
package directory.

\`.wyrmignore\` excludes exact project-relative files or directories (written
with a trailing slash). It intentionally has no glob or negation syntax.

See <https://github.com/phobos-dthorga/onair-wyrmgrid/blob/main/docs/integrations/extension-authoring.md>
for the complete trust, compatibility, and release checklist.
`;
}

async function pluginFiles(options) {
  const manifest = {
    id: options.id,
    name: options.name,
    version: options.version,
    api_version: 1,
    author: options.author,
    runtime: "python",
    entry_point: "src/main.py",
    permissions: [],
  };
  const pythonSdk = await readFile(pythonSdkPath, "utf8");
  return new Map([
    ["plugin.json", json(manifest)],
    [
      "src/main.py",
      `"""WyrmGrid plugin entry point for ${options.name}."""

from wyrmgrid_sdk import Plugin


Plugin(
    plugin_id="${options.id}",
    on_fleet_snapshot=lambda _snapshot: None,
).run()
`,
    ],
    ["src/wyrmgrid_sdk/__init__.py", pythonSdk],
    [
      "README.md",
      commonReadme({
        ...options,
        kind: "ordinary plugin",
        extension: `${basename(options.directory)}.wyrmplugin`,
        packageCommand: `wyrmgrid-extension package --source . --output "dist/${basename(options.directory)}.wyrmplugin"`,
        notes:
          "Add only the deny-by-default permissions your plugin needs, then implement its callbacks in `src/main.py`. WyrmGrid owns rendering and never exposes the OnAir API key.",
      }),
    ],
    [".gitignore", "dist/\n__pycache__/\n*.pyc\n"],
    [".wyrmignore", IGNORE_FILE],
  ]);
}

function simulatorProviderFiles(options) {
  const entryPoint = `${executableStem(options.id, "provider")}.exe`;
  const manifest = {
    $schema: "https://wyrmgrid.dev/schemas/simulator-provider-manifest-v1.json",
    schema_version: 1,
    id: options.id,
    name: options.name,
    version: options.version,
    bridge_protocol_version: 1,
    author: options.author,
    entry_point: entryPoint,
    platforms: ["windows_x86_64"],
    simulators: ["msfs_2024"],
    capabilities: ["telemetry_read"],
  };
  return new Map([
    ["provider.json", json(manifest)],
    [
      "README.md",
      commonReadme({
        ...options,
        kind: "simulator provider",
        extension: `${basename(options.directory)}.wyrmprovider`,
        packageCommand: `wyrmgrid-extension package --source . --output "dist/${basename(options.directory)}.wyrmprovider" --include "path/to/${entryPoint}=${entryPoint}"`,
        notes:
          "Implement an out-of-process executable that speaks bridge protocol v1 over its standard streams. Keep native simulator SDK and operating-system integration inside that executable. Remove the placeholder platform or simulator entries you do not actually support.",
      }),
    ],
    [".gitignore", "dist/\ntarget/\n"],
    [".wyrmignore", IGNORE_FILE],
  ]);
}

function audioProviderFiles(options) {
  const entryPoint = executableStem(options.id, "audio-provider");
  const manifest = {
    $schema: "https://wyrmgrid.dev/schemas/audio-provider-manifest-v2.json",
    schema_version: 2,
    id: options.id,
    name: options.name,
    version: options.version,
    audio_protocol_version: 2,
    author: options.author,
    entry_point: entryPoint,
    platforms: ["windows_x86_64"],
    capabilities: [
      "source_enumeration",
      "permission_requests",
      "pcm_s16le_capture",
    ],
  };
  return new Map([
    ["audio-provider.json", json(manifest)],
    [
      "README.md",
      commonReadme({
        ...options,
        kind: "audio capture provider",
        extension: `${basename(options.directory)}.wyrmaudio`,
        packageCommand: `wyrmgrid-extension package --source . --output "dist/${basename(options.directory)}.wyrmaudio" --include "path/to/${entryPoint}.exe=${entryPoint}.exe"`,
        notes:
          "Implement an out-of-process executable that speaks audio provider protocol v2 and emits bounded PCM S16LE frames. Permission requests must remain explicit. Remove capabilities and platforms the executable does not implement.",
      }),
    ],
    [".gitignore", "dist/\ntarget/\n"],
    [".wyrmignore", IGNORE_FILE],
  ]);
}

function audioCodecFiles(options) {
  const entryPoint = executableStem(options.id, "codec");
  const manifest = {
    $schema: "https://wyrmgrid.dev/schemas/audio-codec-manifest-v1.json",
    schema_version: 1,
    id: options.id,
    name: options.name,
    version: options.version,
    codec_protocol_version: 1,
    author: options.author,
    entry_point: entryPoint,
    platforms: ["windows_x86_64"],
    capabilities: ["encode_pcm_s16le"],
    profiles: [
      {
        id: "pilot_microphone_v1",
        codec_id: "replace-me",
        media_type: "application/octet-stream",
        channels: 1,
        sample_rate_hz: 48000,
        target_bitrate_bps: 48000,
        packet_duration_48khz_frames: 960,
      },
    ],
  };
  return new Map([
    ["audio-codec.json", json(manifest)],
    [
      "README.md",
      commonReadme({
        ...options,
        kind: "audio codec provider",
        extension: `${basename(options.directory)}.wyrmcodec`,
        packageCommand: `wyrmgrid-extension package --source . --output "dist/${basename(options.directory)}.wyrmcodec" --include "path/to/${entryPoint}.exe=${entryPoint}.exe"`,
        notes:
          "Replace the placeholder codec ID and media type, then implement an out-of-process executable that speaks audio codec protocol v1. It receives raw PCM only for an active recording and must return one bounded encoded packet for each accepted frame. Remove profiles and platforms you do not implement.",
      }),
    ],
    [".gitignore", "dist/\ntarget/\n"],
    [".wyrmignore", IGNORE_FILE],
  ]);
}

async function scaffoldFiles(options) {
  switch (options.kind) {
    case "plugin":
      return await pluginFiles(options);
    case "simulator-provider":
      return simulatorProviderFiles(options);
    case "audio-provider":
      return audioProviderFiles(options);
    case "audio-codec":
      return audioCodecFiles(options);
    default:
      throw new Error(`Unsupported extension kind: ${options.kind}`);
  }
}

function validateOptions(options) {
  if (!KINDS.has(options.kind))
    throw new Error(
      "Kind must be plugin, simulator-provider, audio-provider, or audio-codec",
    );
  if (!ID_PATTERN.test(options.id ?? "") || options.id.length > 255)
    throw new Error("ID must be a lowercase reverse-domain identifier");
  if (!VERSION_PATTERN.test(options.version ?? ""))
    throw new Error("Version must be an X.Y.Z semantic version");
  for (const field of ["name", "author"]) {
    const value = options[field];
    if (
      typeof value !== "string" ||
      value.trim() !== value ||
      value.length < 1 ||
      value.length > 120
    )
      throw new Error(`${field} must be 1-120 trimmed characters`);
  }
  if (typeof options.directory !== "string" || options.directory.length === 0)
    throw new Error("A target directory is required");
}

export async function scaffoldExtension(options) {
  const normalized = { version: "0.1.0", ...options };
  validateOptions(normalized);
  normalized.directory = resolve(normalized.directory);

  try {
    const metadata = await stat(normalized.directory);
    if (!metadata.isDirectory())
      throw new Error("The scaffold target exists and is not a directory");
    if ((await readdir(normalized.directory)).length > 0)
      throw new Error("The scaffold target directory must be empty");
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }

  const files = await scaffoldFiles(normalized);
  await mkdir(normalized.directory, { recursive: true });
  for (const [path, contents] of files) {
    const destination = join(normalized.directory, path);
    await mkdir(resolve(destination, ".."), { recursive: true });
    await writeFile(destination, contents, { flag: "wx" });
  }
  return {
    directory: normalized.directory,
    kind: normalized.kind,
    id: normalized.id,
    files: [...files.keys()],
  };
}

export function parseScaffoldArguments(arguments_) {
  const options = { version: "0.1.0" };
  for (let index = 0; index < arguments_.length; index += 1) {
    const argument = arguments_[index];
    if (argument === "--kind") options.kind = arguments_[++index];
    else if (argument === "--directory")
      options.directory = arguments_[++index];
    else if (argument === "--id") options.id = arguments_[++index];
    else if (argument === "--name") options.name = arguments_[++index];
    else if (argument === "--author") options.author = arguments_[++index];
    else if (argument === "--version") options.version = arguments_[++index];
    else throw new Error(`Unknown argument: ${argument}`);
  }
  if (
    !options.kind ||
    !options.directory ||
    !options.id ||
    !options.name ||
    !options.author
  )
    throw new Error(
      "Usage: --kind KIND --directory DIRECTORY --id ID --name NAME --author AUTHOR [--version X.Y.Z]",
    );
  return options;
}

export async function runScaffoldCli(arguments_) {
  const result = await scaffoldExtension(parseScaffoldArguments(arguments_));
  console.log(
    `Created ${result.kind} scaffold for ${result.id} in ${result.directory}`,
  );
  return result;
}
