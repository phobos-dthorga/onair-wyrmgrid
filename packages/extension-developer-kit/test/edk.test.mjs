import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import { readZipEntries } from "../src/archive.mjs";
import { runCli } from "../src/cli.mjs";
import { currentPlatform, extensionDefinition } from "../src/catalog.mjs";
import { buildPackage, PACKAGE_MANIFEST_NAME } from "../src/package.mjs";
import { scaffoldExtension } from "../src/scaffold.mjs";
import { copySchemaBundle } from "../src/schemas.mjs";
import { testExtension } from "../src/test.mjs";
import { inspectPackage, validateSource } from "../src/validate.mjs";
import { verifyPackageContents } from "../src/verify-package.mjs";

const testDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(testDirectory, "../../..");
const fixture = join(testDirectory, "fixtures", "handshake-extension.mjs");

async function temporaryDirectory(name) {
  const root = await mkdtemp(join(tmpdir(), "wyrmgrid-edk-"));
  return join(root, name);
}

function memoryIo() {
  let stdout = "";
  let stderr = "";
  return {
    io: {
      stdout: { write: (value) => (stdout += value) },
      stderr: { write: (value) => (stderr += value) },
    },
    stdout: () => stdout,
    stderr: () => stderr,
  };
}

test("standalone CLI scaffolds, validates, packages, and inspects a plugin", async () => {
  const directory = await temporaryDirectory("community-plugin");
  const output = join(dirname(directory), "community-plugin.wyrmplugin");
  const reportPath = join(dirname(directory), "compatibility.json");
  const memory = memoryIo();
  assert.equal(
    await runCli(
      [
        "new",
        "--kind",
        "plugin",
        "--directory",
        directory,
        "--id",
        "org.example.community-plugin",
        "--name",
        "Community plugin",
        "--author",
        "Example Author",
      ],
      memory.io,
    ),
    0,
  );
  assert.match(memory.stdout(), /Created plugin scaffold/);
  assert.equal(
    (
      await validateSource({
        sourceDirectory: directory,
      })
    ).status,
    "passed",
  );
  assert.equal(
    await runCli(
      [
        "package",
        "--source",
        directory,
        "--output",
        output,
        "--report",
        reportPath,
      ],
      memoryIo().io,
    ),
    0,
  );
  const inspection = await inspectPackage({ packagePath: output });
  assert.equal(inspection.status, "passed");
  assert.equal(inspection.extension.id, "org.example.community-plugin");
  assert.match(inspection.archive_sha256, /^[0-9a-f]{64}$/);
  const report = JSON.parse(await readFile(reportPath, "utf8"));
  assert.equal(report.target.name, basename(output));
  assert.doesNotMatch(JSON.stringify(report), /wyrmgrid-edk-/);
  const entries = readZipEntries(await readFile(output));
  assert.ok(entries.has(PACKAGE_MANIFEST_NAME));
  assert.ok(entries.has("plugin.json"));
  assert.ok(entries.has("src/wyrmgrid_sdk/__init__.py"));
  const jsonReportPath = join(
    dirname(directory),
    "compatibility-json-mode.json",
  );
  const jsonMemory = memoryIo();
  assert.equal(
    await runCli(
      ["validate", "--package", output, "--report", jsonReportPath, "--json"],
      jsonMemory.io,
    ),
    0,
  );
  assert.equal(JSON.parse(jsonMemory.stdout()).status, "passed");
  assert.equal(
    await runCli(
      [
        "test",
        "--source",
        directory,
        "--command",
        process.execPath,
        "--arg",
        "--no-warnings",
        "--arg",
        fixture,
        "--arg",
        "plugin",
        "--arg",
        join(directory, "plugin.json"),
      ],
      memoryIo().io,
    ),
    0,
  );
});

test("package inspection rejects changed payload bytes", async () => {
  const directory = await temporaryDirectory("tampered-plugin");
  await scaffoldExtension({
    kind: "plugin",
    directory,
    id: "org.example.tampered-plugin",
    name: "Tampered plugin",
    author: "Example Author",
  });
  const output = join(dirname(directory), "tampered.wyrmplugin");
  await buildPackage("plugin", {
    sourceDirectory: directory,
    outputPath: output,
  });
  const archive = await readFile(output);
  archive[Math.floor(archive.length / 3)] ^= 0xff;
  await writeFile(output, archive);
  const report = await inspectPackage({ packagePath: output });
  assert.equal(report.status, "failed");
  assert.ok(
    report.issues.some((entry) =>
      ["invalid_archive", "payload_mismatch"].includes(entry.code),
    ),
  );
});

test("packaging excludes controlled development paths and its own output", async () => {
  const directory = await temporaryDirectory("ignored-plugin");
  await scaffoldExtension({
    kind: "plugin",
    directory,
    id: "org.example.ignored-plugin",
    name: "Ignored paths plugin",
    author: "Example Author",
  });
  const output = join(directory, "dist", "ignored-plugin.wyrmplugin");
  await writeFile(join(directory, ".env"), "TEST_SECRET=never-package-me");
  await buildPackage("plugin", {
    sourceDirectory: directory,
    outputPath: output,
  });
  const first = await readFile(output);
  await buildPackage("plugin", {
    sourceDirectory: directory,
    outputPath: output,
    force: true,
  });
  const second = await readFile(output);
  assert.ok(first.equals(second));
  const entries = readZipEntries(second);
  assert.ok(!entries.has(".wyrmignore"));
  assert.ok(!entries.has(".env"));
  assert.ok([...entries.keys()].every((path) => !path.startsWith("dist/")));

  await writeFile(join(directory, ".wyrmignore"), "../outside\n");
  await assert.rejects(
    buildPackage("plugin", {
      sourceDirectory: directory,
      outputPath: output,
      force: true,
    }),
    /unsafe path/,
  );

  await writeFile(join(directory, ".wyrmignore"), "dist/\n.env\n");
  const nonFileOutput = join(dirname(directory), "directory.wyrmplugin");
  await mkdir(nonFileOutput);
  await assert.rejects(
    buildPackage("plugin", {
      sourceDirectory: directory,
      outputPath: nonFileOutput,
      force: true,
    }),
    /link or non-file/,
  );
  const nonFileReport = join(dirname(directory), "directory-report.json");
  await mkdir(nonFileReport);
  await assert.rejects(
    runCli(
      ["validate", "--source", directory, "--report", nonFileReport, "--force"],
      memoryIo().io,
    ),
    /link or non-file/,
  );
});

test("runtime handshake conformance covers all four extension kinds", async () => {
  const platform = currentPlatform();
  assert.ok(platform, "test host must map to a supported EDK platform");
  for (const kind of [
    "plugin",
    "simulator-provider",
    "audio-provider",
    "audio-codec",
  ]) {
    const directory = await temporaryDirectory(kind);
    await scaffoldExtension({
      kind,
      directory,
      id: `org.example.${kind}`,
      name: `Example ${kind}`,
      author: "Example Author",
    });
    const definition = extensionDefinition(kind);
    const manifestPath = join(directory, definition.manifestPath);
    const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
    const includes = [];
    if (kind !== "plugin") {
      manifest.platforms = [platform];
      await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
      const entryPoint =
        kind === "simulator-provider"
          ? manifest.entry_point
          : `${manifest.entry_point}.exe`;
      const placeholder = join(dirname(directory), `${kind}.bin`);
      await writeFile(placeholder, "test executable placeholder");
      includes.push({ source: placeholder, destination: entryPoint });
    }
    const report = await testExtension({
      sourceDirectory: directory,
      kind,
      includes,
      command: process.execPath,
      arguments: [fixture, kind, manifestPath],
      timeoutMs: 2_000,
    });
    assert.equal(
      report.status,
      "passed",
      `${kind}: ${JSON.stringify(report.issues)}`,
    );
    assert.equal(
      report.checks.find((check) => check.id === "runtime-handshake")?.status,
      "passed",
    );
  }
});

test("runtime launch failures remain bounded and privacy-reduced", async () => {
  const directory = await temporaryDirectory("missing-runtime-plugin");
  await scaffoldExtension({
    kind: "plugin",
    directory,
    id: "org.example.missing-runtime",
    name: "Missing runtime plugin",
    author: "Example Author",
  });
  const missingCommand = join(
    dirname(directory),
    "private-machine-path",
    "missing-runtime.exe",
  );
  const report = await testExtension({
    sourceDirectory: directory,
    command: missingCommand,
    timeoutMs: 500,
  });
  assert.equal(report.status, "failed");
  assert.ok(
    report.issues.some(
      (entry) =>
        entry.code === "runtime_conformance_failed" &&
        entry.message === "Extension runtime conformance failed",
    ),
  );
  assert.doesNotMatch(JSON.stringify(report), /private-machine-path/);
  assert.doesNotMatch(JSON.stringify(report), /wyrmgrid-edk-/);
});

test("schema bundle is exact and no-overwrite by default", async () => {
  assert.deepEqual(await verifyPackageContents(), {
    version: "1.0.0",
    schemas: 9,
  });
  const output = await temporaryDirectory("schemas");
  const result = await copySchemaBundle(output);
  assert.equal(result.catalog.schema_version, 1);
  assert.equal(result.catalog.schemas.length, 9);
  await assert.rejects(copySchemaBundle(output), /exist/i);
  await copySchemaBundle(output, true);

  const partial = await temporaryDirectory("partial-schemas");
  await mkdir(partial);
  await writeFile(
    join(partial, "simulator-provider-package-manifest-v1.schema.json"),
    "occupied",
  );
  await assert.rejects(copySchemaBundle(partial), /already exists/i);
  await assert.rejects(
    readFile(join(partial, "audio-codec-manifest-v1.schema.json")),
    /ENOENT/,
  );
});

test("manifest failures use stable report issue codes", async () => {
  const directory = await temporaryDirectory("invalid-codec");
  await scaffoldExtension({
    kind: "audio-codec",
    directory,
    id: "org.example.invalid-codec",
    name: "Invalid codec",
    author: "Example Author",
  });
  const manifestPath = join(directory, "audio-codec.json");
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
  manifest.codec_protocol_version = 999;
  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  const report = await validateSource({ sourceDirectory: directory });
  assert.equal(report.status, "failed");
  assert.ok(
    report.issues.some((entry) => entry.code === "unsupported_protocol"),
  );
});

test("shipped example and compatibility report fixture stay usable", async () => {
  const exampleReport = await testExtension({
    sourceDirectory: join(
      repositoryRoot,
      "examples",
      "plugins",
      "fleet-locations",
    ),
    skipRuntime: true,
  });
  assert.equal(
    exampleReport.status,
    "passed",
    JSON.stringify(exampleReport.issues),
  );
  assert.equal(
    exampleReport.checks.find((check) => check.id === "runtime-handshake")
      ?.status,
    "skipped",
  );

  const schema = JSON.parse(
    await readFile(
      join(
        repositoryRoot,
        "schemas",
        "extension-compatibility-report-v1.schema.json",
      ),
      "utf8",
    ),
  );
  const canonical = JSON.parse(
    await readFile(
      join(
        repositoryRoot,
        "schemas",
        "fixtures",
        "extension-compatibility-report-v1.json",
      ),
      "utf8",
    ),
  );
  assert.equal(canonical.schema_version, 1);
  assert.equal(canonical.tool.version, "1.0.0");
  assert.deepEqual(Object.keys(canonical), schema.required);
  assert.equal(
    schema.$id,
    "https://wyrmgrid.dev/schemas/extension-compatibility-report-v1.json",
  );
});
