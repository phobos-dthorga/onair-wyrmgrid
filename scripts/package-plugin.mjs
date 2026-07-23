import { createHash } from "node:crypto";
import { lstat, mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { basename, dirname, extname, join, relative, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { deflateRawSync } from "node:zlib";

export const PACKAGE_MANIFEST_NAME = "wyrmgrid-package.json";
const MAX_ARCHIVE_BYTES = 32 * 1024 * 1024;
const MAX_EXPANDED_BYTES = 64 * 1024 * 1024;
const MAX_FILE_BYTES = 16 * 1024 * 1024;
const MAX_FILES = 512;
const MAX_PATH_BYTES = 240;
const MAX_PATH_DEPTH = 8;
const MAX_COMPONENT_BYTES = 80;
const ZIP_UTF8_FLAG = 0x0800;
const ZIP_DEFLATE = 8;
const ZIP_DOS_DATE = 33;

function sha256(contents) {
  return createHash("sha256").update(contents).digest("hex");
}

function reservedWindowsComponent(component) {
  const stem = component.split(".", 1)[0].toUpperCase();
  return (
    ["CON", "PRN", "AUX", "NUL"].includes(stem) ||
    /^(?:COM|LPT)[1-9]$/.test(stem)
  );
}

function validatePackagePath(path) {
  const components = path.split("/");
  if (
    path.length === 0 ||
    Buffer.byteLength(path, "ascii") !== path.length ||
    path.length > MAX_PATH_BYTES ||
    path.startsWith("/") ||
    path.endsWith("/") ||
    path.includes("//") ||
    path.includes("\\") ||
    !/^[A-Za-z0-9._/-]+$/.test(path) ||
    components.length > MAX_PATH_DEPTH ||
    components.some(
      (component) =>
        component.length === 0 ||
        component === "." ||
        component === ".." ||
        component.length > MAX_COMPONENT_BYTES ||
        reservedWindowsComponent(component),
    )
  ) {
    throw new Error(`Unsafe package path: ${path}`);
  }
}

function validateExtensionIdentity(manifest, manifestPath) {
  const idPattern =
    /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?){2,}$/;
  const versionPattern =
    /^(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)$/;
  if (
    typeof manifest !== "object" ||
    manifest === null ||
    typeof manifest.id !== "string" ||
    manifest.id.length > 255 ||
    !idPattern.test(manifest.id) ||
    typeof manifest.version !== "string" ||
    !versionPattern.test(manifest.version) ||
    typeof manifest.entry_point !== "string"
  ) {
    throw new Error(
      `${manifestPath} has an invalid package identity or version`,
    );
  }
  validatePackagePath(manifest.entry_point);
}

async function collectDirectory(root, directory = root) {
  const files = [];
  const entries = await readdir(directory, { withFileTypes: true });
  entries.sort((left, right) => left.name.localeCompare(right.name, "en"));
  for (const entry of entries) {
    const absolute = join(directory, entry.name);
    const metadata = await lstat(absolute);
    if (metadata.isSymbolicLink())
      throw new Error(`Symbolic links are not packageable: ${absolute}`);
    if (metadata.isDirectory()) {
      files.push(...(await collectDirectory(root, absolute)));
      continue;
    }
    if (!metadata.isFile())
      throw new Error(`Unsupported source entry: ${absolute}`);
    const path = relative(root, absolute).replaceAll("\\", "/");
    files.push({ path, source: absolute });
  }
  return files;
}

function crc32(contents) {
  let crc = 0xffffffff;
  for (const byte of contents) {
    crc = CRC32_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

const CRC32_TABLE = Array.from({ length: 256 }, (_, value) => {
  let crc = value;
  for (let bit = 0; bit < 8; bit += 1) {
    crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
  }
  return crc >>> 0;
});

function localHeader(name, contents, compressed, checksum) {
  const nameBytes = Buffer.from(name, "utf8");
  const header = Buffer.alloc(30);
  header.writeUInt32LE(0x04034b50, 0);
  header.writeUInt16LE(20, 4);
  header.writeUInt16LE(ZIP_UTF8_FLAG, 6);
  header.writeUInt16LE(ZIP_DEFLATE, 8);
  header.writeUInt16LE(0, 10);
  header.writeUInt16LE(ZIP_DOS_DATE, 12);
  header.writeUInt32LE(checksum, 14);
  header.writeUInt32LE(compressed.length, 18);
  header.writeUInt32LE(contents.length, 22);
  header.writeUInt16LE(nameBytes.length, 26);
  header.writeUInt16LE(0, 28);
  return Buffer.concat([header, nameBytes]);
}

function centralHeader(name, contents, compressed, checksum, offset) {
  const nameBytes = Buffer.from(name, "utf8");
  const header = Buffer.alloc(46);
  header.writeUInt32LE(0x02014b50, 0);
  header.writeUInt16LE(0x0314, 4);
  header.writeUInt16LE(20, 6);
  header.writeUInt16LE(ZIP_UTF8_FLAG, 8);
  header.writeUInt16LE(ZIP_DEFLATE, 10);
  header.writeUInt16LE(0, 12);
  header.writeUInt16LE(ZIP_DOS_DATE, 14);
  header.writeUInt32LE(checksum, 16);
  header.writeUInt32LE(compressed.length, 20);
  header.writeUInt32LE(contents.length, 24);
  header.writeUInt16LE(nameBytes.length, 28);
  header.writeUInt16LE(0, 30);
  header.writeUInt16LE(0, 32);
  header.writeUInt16LE(0, 34);
  header.writeUInt16LE(0, 36);
  header.writeUInt32LE(0o100644 * 65536, 38);
  header.writeUInt32LE(offset, 42);
  return Buffer.concat([header, nameBytes]);
}

function buildZip(entries) {
  const localParts = [];
  const centralParts = [];
  let offset = 0;
  for (const { path, contents } of entries) {
    const compressed = deflateRawSync(contents, { level: 9 });
    const checksum = crc32(contents);
    const local = localHeader(path, contents, compressed, checksum);
    localParts.push(local, compressed);
    centralParts.push(
      centralHeader(path, contents, compressed, checksum, offset),
    );
    offset += local.length + compressed.length;
  }
  const central = Buffer.concat(centralParts);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(entries.length, 8);
  end.writeUInt16LE(entries.length, 10);
  end.writeUInt32LE(central.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...localParts, central, end]);
}

async function buildExtensionPackage({
  sourceDirectory,
  outputPath,
  includes = [],
  force = false,
  packageExtension,
  packageKind,
  manifestPath,
  includeSourceDirectory = true,
  entryPointPaths = (manifest) => [manifest.entry_point],
}) {
  const sourceRoot = resolve(sourceDirectory);
  const output = resolve(outputPath);
  if (extname(output).toLowerCase() !== packageExtension)
    throw new Error(`Package output must end in ${packageExtension}`);

  const sources = includeSourceDirectory
    ? await collectDirectory(sourceRoot)
    : [];
  for (const include of includes) {
    sources.push({
      path: include.destination,
      source: resolve(include.source),
    });
  }
  const paths = new Set();
  const foldedPaths = new Set();
  const payload = [];
  let expandedSize = 0;
  for (const source of sources.sort((left, right) =>
    left.path.localeCompare(right.path, "en"),
  )) {
    validatePackagePath(source.path);
    if (source.path === PACKAGE_MANIFEST_NAME)
      throw new Error(`${PACKAGE_MANIFEST_NAME} is reserved`);
    const folded = source.path.toLowerCase();
    if (paths.has(source.path) || foldedPaths.has(folded))
      throw new Error(
        `Duplicate or case-colliding package path: ${source.path}`,
      );
    paths.add(source.path);
    foldedPaths.add(folded);
    const metadata = await lstat(source.source);
    if (!metadata.isFile() || metadata.isSymbolicLink())
      throw new Error(
        `Included payload is not a regular file: ${source.source}`,
      );
    if (metadata.size === 0 || metadata.size > MAX_FILE_BYTES)
      throw new Error(
        `Payload file is outside supported bounds: ${source.path}`,
      );
    const contents = await readFile(source.source);
    expandedSize += contents.length;
    payload.push({ path: source.path, contents });
  }
  if (
    payload.length === 0 ||
    payload.length > MAX_FILES ||
    expandedSize > MAX_EXPANDED_BYTES ||
    !paths.has(manifestPath)
  ) {
    throw new Error("Extension payload inventory is outside supported bounds");
  }

  const extensionManifest = JSON.parse(
    payload
      .find((file) => file.path === manifestPath)
      .contents.toString("utf8"),
  );
  validateExtensionIdentity(extensionManifest, manifestPath);
  if (!entryPointPaths(extensionManifest).every((path) => paths.has(path)))
    throw new Error("The declared extension entry point is not in the package");
  const manifest = {
    schema_version: 1,
    kind: packageKind,
    id: extensionManifest.id,
    version: extensionManifest.version,
    manifest_path: manifestPath,
    files: payload.map(({ path, contents }) => ({
      path,
      size: contents.length,
      sha256: sha256(contents),
    })),
  };
  const manifestContents = Buffer.from(
    `${JSON.stringify(manifest, null, 2)}\n`,
    "utf8",
  );
  const archive = buildZip([
    { path: PACKAGE_MANIFEST_NAME, contents: manifestContents },
    ...payload,
  ]);
  if (archive.length > MAX_ARCHIVE_BYTES)
    throw new Error("Extension package exceeds the 32 MiB archive limit");
  await mkdir(dirname(output), { recursive: true });
  await writeFile(output, archive, { flag: force ? "w" : "wx" });
  return {
    output,
    id: manifest.id,
    version: manifest.version,
    files: payload.length,
    archiveBytes: archive.length,
    archiveSha256: sha256(archive),
  };
}

export function buildPluginPackage(options) {
  return buildExtensionPackage({
    ...options,
    packageExtension: ".wyrmplugin",
    packageKind: "ordinary_plugin",
    manifestPath: "plugin.json",
  });
}

export function buildSimulatorProviderPackage(options) {
  return buildExtensionPackage({
    ...options,
    packageExtension: ".wyrmprovider",
    packageKind: "simulator_provider",
    manifestPath: "provider.json",
  });
}

export function buildAudioProviderPackage(options) {
  return buildExtensionPackage({
    ...options,
    packageExtension: ".wyrmaudio",
    packageKind: "audio_provider",
    manifestPath: "audio-provider.json",
    entryPointPaths: (manifest) => [
      ...new Set(
        manifest.platforms.map((platform) =>
          platform === "windows_x86_64"
            ? `${manifest.entry_point}.exe`
            : manifest.entry_point,
        ),
      ),
    ],
  });
}

export function buildAudioCodecPackage(options) {
  return buildExtensionPackage({
    ...options,
    packageExtension: ".wyrmcodec",
    packageKind: "audio_codec_provider",
    manifestPath: "audio-codec.json",
    entryPointPaths: (manifest) => [
      ...new Set(
        manifest.platforms.map((platform) =>
          platform === "windows_x86_64"
            ? `${manifest.entry_point}.exe`
            : manifest.entry_point,
        ),
      ),
    ],
  });
}

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
      "Usage: --source DIRECTORY --output FILE.wyrmplugin [--include SOURCE=PACKAGE_PATH] [--force]",
    );
  return options;
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  buildPluginPackage(parseArguments(process.argv.slice(2)))
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
