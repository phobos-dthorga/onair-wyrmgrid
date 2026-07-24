import { lstat, mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { basename, dirname, extname, join, relative, resolve } from "node:path";
import { deflateRawSync } from "node:zlib";

import { crc32, sha256 } from "./binary.mjs";
import { declaredEntryPoints, extensionDefinition } from "./catalog.mjs";
import {
  PACKAGE_LIMITS,
  validateExtensionManifest,
  validatePackagePath,
} from "./contract.mjs";

export const PACKAGE_MANIFEST_NAME = "wyrmgrid-package.json";
export const PACKAGE_IGNORE_NAME = ".wyrmignore";
const ZIP_UTF8_FLAG = 0x0800;
const ZIP_DEFLATE = 8;
const ZIP_DOS_DATE = 33;

function sourcePath(root, absolute) {
  return relative(root, absolute).replaceAll("\\", "/");
}

function pathIdentity(path) {
  const absolute = resolve(path);
  return process.platform === "win32" ? absolute.toLowerCase() : absolute;
}

function portableCompare(left, right) {
  return left < right ? -1 : left > right ? 1 : 0;
}

function parseIgnoreRules(contents) {
  const rules = [
    { path: ".git", directory: true },
    { path: ".hg", directory: true },
    { path: ".svn", directory: true },
    { path: PACKAGE_IGNORE_NAME, directory: false },
  ];
  for (const [index, rawLine] of contents.split(/\r?\n/u).entries()) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) continue;
    if (
      line.startsWith("/") ||
      line.startsWith("!") ||
      line.includes("\\") ||
      /[*?[\]]/u.test(line)
    )
      throw new Error(
        `${PACKAGE_IGNORE_NAME}:${index + 1} must be an exact relative path`,
      );
    const directory = line.endsWith("/");
    const path = directory ? line.slice(0, -1) : line;
    try {
      validatePackagePath(path);
    } catch {
      throw new Error(
        `${PACKAGE_IGNORE_NAME}:${index + 1} contains an unsafe path`,
      );
    }
    rules.push({ path, directory });
  }
  return rules;
}

async function loadIgnoreRules(root) {
  try {
    return parseIgnoreRules(
      new TextDecoder("utf-8", { fatal: true }).decode(
        await readFile(join(root, PACKAGE_IGNORE_NAME)),
      ),
    );
  } catch (error) {
    if (error?.code === "ENOENT") return parseIgnoreRules("");
    throw error;
  }
}

function ignored(path, rules) {
  return rules.some(
    (rule) =>
      path === rule.path ||
      (rule.directory && path.startsWith(`${rule.path}/`)),
  );
}

async function collectDirectory(root, directory, rules, excludedAbsolutePaths) {
  const files = [];
  const entries = await readdir(directory, { withFileTypes: true });
  entries.sort((left, right) => portableCompare(left.name, right.name));
  for (const entry of entries) {
    const absolute = join(directory, entry.name);
    const path = sourcePath(root, absolute);
    if (
      excludedAbsolutePaths.has(pathIdentity(absolute)) ||
      ignored(path, rules)
    )
      continue;
    const metadata = await lstat(absolute);
    if (metadata.isSymbolicLink())
      throw new Error(`Symbolic links are not packageable: ${absolute}`);
    if (metadata.isDirectory()) {
      files.push(
        ...(await collectDirectory(
          root,
          absolute,
          rules,
          excludedAbsolutePaths,
        )),
      );
      continue;
    }
    if (!metadata.isFile())
      throw new Error(`Unsupported source entry: ${absolute}`);
    files.push({ path, source: absolute });
  }
  return files;
}

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
  extensionKind,
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

  const ignoredSourcePaths = await loadIgnoreRules(sourceRoot);
  const sources = includeSourceDirectory
    ? await collectDirectory(
        sourceRoot,
        sourceRoot,
        ignoredSourcePaths,
        new Set([pathIdentity(output)]),
      )
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
    portableCompare(left.path, right.path),
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
    if (metadata.size === 0 || metadata.size > PACKAGE_LIMITS.fileBytes)
      throw new Error(
        `Payload file is outside supported bounds: ${source.path}`,
      );
    const contents = await readFile(source.source);
    expandedSize += contents.length;
    payload.push({ path: source.path, contents });
  }
  if (
    payload.length === 0 ||
    payload.length > PACKAGE_LIMITS.files ||
    expandedSize > PACKAGE_LIMITS.expandedBytes ||
    !paths.has(manifestPath)
  ) {
    throw new Error("Extension payload inventory is outside supported bounds");
  }

  const extensionManifest = JSON.parse(
    new TextDecoder("utf-8", { fatal: true }).decode(
      payload.find((file) => file.path === manifestPath).contents,
    ),
  );
  const manifestIssues = validateExtensionManifest(
    extensionKind,
    extensionManifest,
  );
  if (manifestIssues.length > 0)
    throw new Error(
      `${manifestPath} is invalid: ${manifestIssues[0].path} ${manifestIssues[0].message}`,
    );
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
  if (archive.length > PACKAGE_LIMITS.archiveBytes)
    throw new Error("Extension package exceeds the 32 MiB archive limit");
  await mkdir(dirname(output), { recursive: true });
  try {
    const metadata = await lstat(output);
    if (metadata.isSymbolicLink() || !metadata.isFile())
      throw new Error("Package output must not replace a link or non-file");
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
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
  const definition = extensionDefinition("plugin");
  return buildExtensionPackage({
    ...options,
    extensionKind: definition.kind,
    packageExtension: definition.suffix,
    packageKind: definition.packageKind,
    manifestPath: definition.manifestPath,
  });
}

export function buildSimulatorProviderPackage(options) {
  const definition = extensionDefinition("simulator-provider");
  return buildExtensionPackage({
    ...options,
    extensionKind: definition.kind,
    packageExtension: definition.suffix,
    packageKind: definition.packageKind,
    manifestPath: definition.manifestPath,
  });
}

export function buildAudioProviderPackage(options) {
  const definition = extensionDefinition("audio-provider");
  return buildExtensionPackage({
    ...options,
    extensionKind: definition.kind,
    packageExtension: definition.suffix,
    packageKind: definition.packageKind,
    manifestPath: definition.manifestPath,
    entryPointPaths: (manifest) => declaredEntryPoints(definition, manifest),
  });
}

export function buildAudioCodecPackage(options) {
  const definition = extensionDefinition("audio-codec");
  return buildExtensionPackage({
    ...options,
    extensionKind: definition.kind,
    packageExtension: definition.suffix,
    packageKind: definition.packageKind,
    manifestPath: definition.manifestPath,
    entryPointPaths: (manifest) => declaredEntryPoints(definition, manifest),
  });
}

export function buildPackage(kind, options) {
  switch (kind) {
    case "plugin":
      return buildPluginPackage(options);
    case "simulator-provider":
      return buildSimulatorProviderPackage(options);
    case "audio-provider":
      return buildAudioProviderPackage(options);
    case "audio-codec":
      return buildAudioCodecPackage(options);
    default:
      throw new Error(`Unsupported extension kind: ${kind}`);
  }
}

export function parsePackageArguments(arguments_) {
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
      "Usage: --source DIRECTORY --output FILE [--include SOURCE=PACKAGE_PATH] [--force]",
    );
  return options;
}

export async function runPackageCli(kind, arguments_) {
  const result = await buildPackage(kind, parsePackageArguments(arguments_));
  console.log(
    `Packaged ${result.id} v${result.version}: ${basename(result.output)} (${result.files} files, sha256 ${result.archiveSha256})`,
  );
  return result;
}
