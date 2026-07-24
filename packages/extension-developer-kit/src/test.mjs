import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";

import { extensionDefinition, inferSourceKind } from "./catalog.mjs";
import { buildPackage } from "./package.mjs";
import { testRuntime } from "./runtime-test.mjs";
import { buildReport, inspectPackage, validateSource } from "./validate.mjs";

function errorIssue(code, path, message) {
  return { severity: "error", code, path, message };
}

export async function testExtension({
  sourceDirectory,
  kind,
  includes = [],
  command,
  arguments: commandArguments = [],
  timeoutMs = 5_000,
  skipRuntime = false,
}) {
  const source = resolve(sourceDirectory);
  const resolvedKind = kind ?? (await inferSourceKind(source));
  const definition = extensionDefinition(resolvedKind);
  const sourceReport = await validateSource({
    sourceDirectory: source,
    kind: resolvedKind,
    command: "test",
  });
  if (sourceReport.status === "failed") return sourceReport;

  const temporary = await mkdtemp(join(tmpdir(), "wyrmgrid-edk-test-"));
  const firstPath = join(temporary, `first${definition.suffix}`);
  const secondPath = join(temporary, `second${definition.suffix}`);
  const issues = [...sourceReport.issues];
  const checks = [...sourceReport.checks];
  let packageReport;
  let manifest;
  try {
    try {
      await buildPackage(resolvedKind, {
        sourceDirectory: source,
        outputPath: firstPath,
        includes,
      });
      await buildPackage(resolvedKind, {
        sourceDirectory: source,
        outputPath: secondPath,
        includes,
      });
      const [first, second] = await Promise.all([
        readFile(firstPath),
        readFile(secondPath),
      ]);
      if (!first.equals(second))
        throw new Error("Two clean package builds produced different bytes");
      checks.push({
        id: "deterministic-package",
        status: "passed",
        summary: "Two package builds are byte-for-byte identical",
      });
      packageReport = await inspectPackage({
        packagePath: firstPath,
        command: "test",
      });
      checks.push(...packageReport.checks);
      issues.push(...packageReport.issues);
      manifest = JSON.parse(
        await readFile(join(source, definition.manifestPath), "utf8"),
      );
    } catch (error) {
      checks.push({
        id: "deterministic-package",
        status: "failed",
        summary: "Package construction or repeatability failed",
      });
      issues.push(
        errorIssue(
          "package_conformance_failed",
          "package",
          error instanceof Error ? error.message : "Package conformance failed",
        ),
      );
    }

    if (skipRuntime) {
      checks.push({
        id: "runtime-handshake",
        status: "skipped",
        summary: "Runtime handshake was explicitly skipped",
      });
    } else if (!issues.some((entry) => entry.severity === "error")) {
      try {
        await testRuntime({
          sourceDirectory: source,
          kind: resolvedKind,
          command,
          arguments: commandArguments,
          timeoutMs,
        });
        checks.push({
          id: "runtime-handshake",
          status: "passed",
          summary: `${definition.protocolName} v${definition.protocolVersion} startup and shutdown completed`,
        });
      } catch (error) {
        checks.push({
          id: "runtime-handshake",
          status: "failed",
          summary: "Runtime startup or shutdown contract failed",
        });
        issues.push(
          errorIssue(
            "runtime_conformance_failed",
            "runtime",
            error instanceof Error
              ? error.message
              : "Runtime conformance failed",
          ),
        );
      }
    }

    return buildReport({
      command: "test",
      target: { type: "source", name: basename(source) },
      definition,
      manifest,
      issues,
      checks,
      archiveSha256: packageReport?.archive_sha256,
    });
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}
