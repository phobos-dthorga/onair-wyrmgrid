import { lstat, mkdir, readFile, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";

import { readZipEntries } from "./archive.mjs";
import { sha256 } from "./binary.mjs";
import {
  EDK_VERSION,
  REPORT_SCHEMA_VERSION,
  declaredEntryPoints,
  extensionDefinition,
  extensionDefinitionForPackage,
  inferSourceKind,
} from "./catalog.mjs";
import {
  issue,
  validateExtensionManifest,
  validatePackageManifest,
} from "./contract.mjs";
import { PACKAGE_MANIFEST_NAME } from "./package.mjs";

function parseJson(contents, label) {
  try {
    const text =
      typeof contents === "string"
        ? contents
        : new TextDecoder("utf-8", { fatal: true }).decode(contents);
    return JSON.parse(text);
  } catch {
    throw new Error(`${label} is not valid UTF-8 JSON`);
  }
}

function compatibility(definition) {
  return {
    package_schema_version: 1,
    manifest_schema: definition.manifestSchema,
    protocol: {
      name: definition.protocolName,
      version: definition.protocolVersion,
    },
  };
}

function warning(code, path, message) {
  return { severity: "warning", code, path, message };
}

function errors(issues) {
  return issues.map((entry) => ({ severity: "error", ...entry }));
}

export function buildReport({
  command = "validate",
  target,
  definition,
  manifest,
  issues,
  checks,
  archiveSha256,
}) {
  return {
    schema_version: REPORT_SCHEMA_VERSION,
    tool: {
      name: "@wyrmgrid/extension-developer-kit",
      version: EDK_VERSION,
    },
    command,
    target,
    status: issues.some((entry) => entry.severity === "error")
      ? "failed"
      : "passed",
    extension: manifest
      ? {
          kind: definition.kind,
          id: manifest.id ?? null,
          version: manifest.version ?? null,
        }
      : {
          kind: definition.kind,
          id: null,
          version: null,
        },
    compatibility: compatibility(definition),
    archive_sha256: archiveSha256 ?? null,
    checks,
    issues,
  };
}

function nativeTrustWarning(definition) {
  if (definition.kind === "plugin") return [];
  return [
    warning(
      "unsigned_native_code",
      "package",
      "Package format v1 proves local integrity, not publisher identity or native-code safety",
    ),
  ];
}

export async function validateSource({
  sourceDirectory,
  kind,
  command = "validate",
}) {
  const source = resolve(sourceDirectory);
  const resolvedKind = kind ?? (await inferSourceKind(source));
  const definition = extensionDefinition(resolvedKind);
  let manifest;
  const issues = [];
  try {
    manifest = parseJson(
      await readFile(join(source, definition.manifestPath), "utf8"),
      definition.manifestPath,
    );
    issues.push(...errors(validateExtensionManifest(resolvedKind, manifest)));
  } catch (error) {
    issues.push(
      errors([
        issue(
          "manifest_unavailable",
          "manifest",
          error instanceof Error ? error.message : "Manifest is unavailable",
        ),
      ])[0],
    );
  }
  if (manifest?.runtime === undefined && definition.kind === "plugin")
    issues.push(
      warning(
        "metadata_only_plugin",
        "manifest.runtime",
        'Plugin is valid metadata but is not executable until runtime is set to "python"',
      ),
    );
  return buildReport({
    command,
    target: { type: "source", name: basename(source) },
    definition,
    manifest,
    issues,
    checks: [
      {
        id: "manifest-contract",
        status: issues.some((entry) => entry.severity === "error")
          ? "failed"
          : "passed",
        summary: `${definition.manifestPath} matches the EDK v1 contract`,
      },
    ],
  });
}

export async function inspectPackage({ packagePath, command = "validate" }) {
  const path = resolve(packagePath);
  const definition = extensionDefinitionForPackage(path);
  const archive = await readFile(path);
  const archiveDigest = sha256(archive);
  let entries;
  let packageManifest;
  let manifest;
  const foundIssues = [];
  const checks = [];
  try {
    entries = readZipEntries(archive);
    checks.push({
      id: "bounded-archive",
      status: "passed",
      summary:
        "ZIP framing, paths, compression, sizes, and checksums are bounded",
    });
  } catch (error) {
    foundIssues.push(
      ...errors([
        issue(
          "invalid_archive",
          "package",
          error instanceof Error ? error.message : "Package archive is invalid",
        ),
      ]),
    );
  }
  if (entries) {
    try {
      const rawPackageManifest = entries.get(PACKAGE_MANIFEST_NAME);
      if (!rawPackageManifest)
        throw new Error(`${PACKAGE_MANIFEST_NAME} is missing`);
      packageManifest = parseJson(rawPackageManifest, PACKAGE_MANIFEST_NAME);
      foundIssues.push(
        ...errors(validatePackageManifest(definition, packageManifest)),
      );
      checks.push({
        id: "package-manifest",
        status: foundIssues.some(
          (entry) =>
            entry.severity === "error" && entry.path.startsWith("package"),
        )
          ? "failed"
          : "passed",
        summary: `${PACKAGE_MANIFEST_NAME} matches package schema v1`,
      });
    } catch (error) {
      foundIssues.push(
        ...errors([
          issue(
            "invalid_package_manifest",
            "package",
            error instanceof Error
              ? error.message
              : "Package manifest is invalid",
          ),
        ]),
      );
    }
  }
  if (entries && packageManifest) {
    const declared = new Map(
      Array.isArray(packageManifest.files)
        ? packageManifest.files.map((file) => [file.path, file])
        : [],
    );
    const expectedPaths = new Set([PACKAGE_MANIFEST_NAME, ...declared.keys()]);
    const inventoryMatches =
      expectedPaths.size === entries.size &&
      [...expectedPaths].every((path) => entries.has(path));
    if (!inventoryMatches)
      foundIssues.push(
        ...errors([
          issue(
            "inventory_mismatch",
            "package.files",
            "Declared inventory does not exactly match the archive",
          ),
        ]),
      );
    for (const [path, declaration] of declared) {
      const contents = entries.get(path);
      if (
        !contents ||
        contents.length !== declaration.size ||
        sha256(contents) !== declaration.sha256
      )
        foundIssues.push(
          ...errors([
            issue(
              "payload_mismatch",
              `package.files.${path}`,
              "Payload bytes do not match the declared size and digest",
            ),
          ]),
        );
    }
    checks.push({
      id: "exact-inventory",
      status: foundIssues.some((entry) =>
        ["inventory_mismatch", "payload_mismatch"].includes(entry.code),
      )
        ? "failed"
        : "passed",
      summary: "Every payload path, size, and SHA-256 digest matches",
    });
    try {
      const rawManifest = entries.get(definition.manifestPath);
      if (!rawManifest)
        throw new Error(`${definition.manifestPath} is missing`);
      manifest = parseJson(rawManifest, definition.manifestPath);
      foundIssues.push(
        ...errors(validateExtensionManifest(definition.kind, manifest)),
      );
      if (
        manifest.id !== packageManifest.id ||
        manifest.version !== packageManifest.version
      )
        foundIssues.push(
          ...errors([
            issue(
              "identity_mismatch",
              "manifest",
              "Package and extension manifest identity/version must match",
            ),
          ]),
        );
      const entryPoints = declaredEntryPoints(definition, manifest);
      if (!entryPoints.every((entryPoint) => entries.has(entryPoint)))
        foundIssues.push(
          ...errors([
            issue(
              "missing_entry_point",
              "manifest.entry_point",
              "Every declared platform entry point must be packaged",
            ),
          ]),
        );
      checks.push({
        id: "extension-contract",
        status: foundIssues.some(
          (entry) =>
            entry.severity === "error" &&
            (entry.path.startsWith("manifest") ||
              ["identity_mismatch", "missing_entry_point"].includes(
                entry.code,
              )),
        )
          ? "failed"
          : "passed",
        summary: `${definition.manifestPath} identity, compatibility, and entry points match`,
      });
    } catch (error) {
      foundIssues.push(
        ...errors([
          issue(
            "invalid_extension_manifest",
            "manifest",
            error instanceof Error
              ? error.message
              : "Extension manifest is invalid",
          ),
        ]),
      );
    }
  }
  foundIssues.push(...nativeTrustWarning(definition));
  return buildReport({
    command,
    target: { type: "package", name: basename(path) },
    definition,
    manifest,
    issues: foundIssues,
    checks,
    archiveSha256: archiveDigest,
  });
}

export async function writeCompatibilityReport(
  report,
  outputPath,
  force = false,
) {
  const output = resolve(outputPath);
  await mkdir(dirname(output), { recursive: true });
  try {
    const metadata = await lstat(output);
    if (metadata.isSymbolicLink() || !metadata.isFile())
      throw new Error(
        "Compatibility report must not replace a link or non-file",
      );
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
  await writeFile(output, `${JSON.stringify(report, null, 2)}\n`, {
    flag: force ? "w" : "wx",
  });
  return output;
}
