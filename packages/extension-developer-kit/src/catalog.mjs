import { access } from "node:fs/promises";
import { extname, join } from "node:path";

export const EDK_VERSION = "1.0.0";
export const REPORT_SCHEMA_VERSION = 1;

export const EXTENSIONS = Object.freeze({
  plugin: Object.freeze({
    kind: "plugin",
    displayName: "ordinary plugin",
    packageKind: "ordinary_plugin",
    suffix: ".wyrmplugin",
    mediaType: "application/vnd.wyrmgrid.plugin-package+zip",
    manifestPath: "plugin.json",
    manifestSchema: "plugin-manifest.schema.json",
    packageSchema: "extension-package-manifest-v1.schema.json",
    protocolName: "Plugin API",
    protocolVersion: 1,
  }),
  "simulator-provider": Object.freeze({
    kind: "simulator-provider",
    displayName: "simulator provider",
    packageKind: "simulator_provider",
    suffix: ".wyrmprovider",
    mediaType: "application/vnd.wyrmgrid.simulator-provider-package+zip",
    manifestPath: "provider.json",
    manifestSchema: "simulator-provider-manifest-v1.schema.json",
    packageSchema: "simulator-provider-package-manifest-v1.schema.json",
    protocolName: "Bridge protocol",
    protocolVersion: 1,
  }),
  "audio-provider": Object.freeze({
    kind: "audio-provider",
    displayName: "audio capture provider",
    packageKind: "audio_provider",
    suffix: ".wyrmaudio",
    mediaType: "application/vnd.wyrmgrid.audio-provider-package+zip",
    manifestPath: "audio-provider.json",
    manifestSchema: "audio-provider-manifest-v2.schema.json",
    packageSchema: "audio-provider-package-manifest-v1.schema.json",
    protocolName: "Audio Capture Provider protocol",
    protocolVersion: 2,
  }),
  "audio-codec": Object.freeze({
    kind: "audio-codec",
    displayName: "audio codec provider",
    packageKind: "audio_codec_provider",
    suffix: ".wyrmcodec",
    mediaType: "application/vnd.wyrmgrid.audio-codec-package+zip",
    manifestPath: "audio-codec.json",
    manifestSchema: "audio-codec-manifest-v1.schema.json",
    packageSchema: "audio-codec-package-manifest-v1.schema.json",
    protocolName: "Audio Codec Provider protocol",
    protocolVersion: 1,
  }),
});

const EXTENSIONS_BY_SUFFIX = new Map(
  Object.values(EXTENSIONS).map((definition) => [
    definition.suffix,
    definition,
  ]),
);

export function extensionDefinition(kind) {
  const definition = EXTENSIONS[kind];
  if (!definition) throw new Error(`Unsupported extension kind: ${kind}`);
  return definition;
}

export function extensionDefinitionForPackage(path) {
  const definition = EXTENSIONS_BY_SUFFIX.get(extname(path).toLowerCase());
  if (!definition)
    throw new Error(
      "Package must end in .wyrmplugin, .wyrmprovider, .wyrmaudio, or .wyrmcodec",
    );
  return definition;
}

export async function inferSourceKind(sourceDirectory) {
  const matches = [];
  for (const definition of Object.values(EXTENSIONS)) {
    try {
      await access(join(sourceDirectory, definition.manifestPath));
      matches.push(definition.kind);
    } catch (error) {
      if (error?.code !== "ENOENT") throw error;
    }
  }
  if (matches.length === 0)
    throw new Error("No supported WyrmGrid extension manifest was found");
  if (matches.length > 1)
    throw new Error(
      `Source contains multiple extension manifests: ${matches.join(", ")}`,
    );
  return matches[0];
}

export function currentPlatform() {
  if (process.platform === "win32" && process.arch === "x64")
    return "windows_x86_64";
  if (process.platform === "linux" && process.arch === "x64")
    return "linux_x86_64";
  if (process.platform === "darwin" && process.arch === "arm64")
    return "macos_aarch64";
  if (process.platform === "darwin" && process.arch === "x64")
    return "macos_x86_64";
  return undefined;
}

export function declaredEntryPoints(definition, manifest) {
  if (definition.kind !== "audio-provider" && definition.kind !== "audio-codec")
    return [manifest.entry_point];
  return [
    ...new Set(
      manifest.platforms.map((platform) =>
        platform === "windows_x86_64"
          ? `${manifest.entry_point}.exe`
          : manifest.entry_point,
      ),
    ),
  ];
}
