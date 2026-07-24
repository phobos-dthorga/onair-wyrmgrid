import { basename, resolve } from "node:path";

import {
  EDK_VERSION,
  extensionDefinition,
  inferSourceKind,
} from "./catalog.mjs";
import { buildPackage } from "./package.mjs";
import { scaffoldExtension } from "./scaffold.mjs";
import { copySchemaBundle, schemaCatalog } from "./schemas.mjs";
import { testExtension } from "./test.mjs";
import {
  inspectPackage,
  validateSource,
  writeCompatibilityReport,
} from "./validate.mjs";

const HELP = `WyrmGrid Extension Developer Kit v${EDK_VERSION}

Usage:
  wyrmgrid-extension new --kind KIND --directory DIR --id ID --name NAME --author AUTHOR [--version X.Y.Z]
  wyrmgrid-extension validate (--source DIR | --package FILE) [--kind KIND] [--report FILE] [--json] [--force]
  wyrmgrid-extension package --source DIR --output FILE [--kind KIND] [--include SOURCE=PATH]... [--report FILE] [--json] [--force]
  wyrmgrid-extension test --source DIR [--kind KIND] [--include SOURCE=PATH]... [--command PROGRAM] [--arg VALUE]... [--timeout-ms N] [--skip-runtime] [--report FILE] [--json] [--force]
  wyrmgrid-extension schemas [--output DIR] [--json] [--force]
  wyrmgrid-extension --version

Kinds: plugin, simulator-provider, audio-provider, audio-codec

The test command validates the manifest, builds the package twice, compares the
archive bytes, inspects its exact inventory, and performs a bounded runtime
startup/shutdown handshake unless --skip-runtime is explicit.
`;

function value(arguments_, index, flag, allowFlagValue = false) {
  const result = arguments_[index + 1];
  if (result === undefined || (!allowFlagValue && result.startsWith("--")))
    throw new Error(`${flag} requires a value`);
  return result;
}

function parseOptions(arguments_, allowed) {
  const options = {
    includes: [],
    arguments: [],
    force: false,
    json: false,
    skipRuntime: false,
  };
  for (let index = 0; index < arguments_.length; index += 1) {
    const argument = arguments_[index];
    if (!allowed.has(argument))
      throw new Error(`Unknown argument: ${argument}`);
    if (argument === "--kind")
      options.kind = value(arguments_, index++, argument);
    else if (argument === "--directory")
      options.directory = value(arguments_, index++, argument);
    else if (argument === "--id")
      options.id = value(arguments_, index++, argument);
    else if (argument === "--name")
      options.name = value(arguments_, index++, argument);
    else if (argument === "--author")
      options.author = value(arguments_, index++, argument);
    else if (argument === "--version")
      options.version = value(arguments_, index++, argument);
    else if (argument === "--source")
      options.sourceDirectory = value(arguments_, index++, argument);
    else if (argument === "--package")
      options.packagePath = value(arguments_, index++, argument);
    else if (argument === "--output")
      options.outputPath = value(arguments_, index++, argument);
    else if (argument === "--report")
      options.reportPath = value(arguments_, index++, argument);
    else if (argument === "--command")
      options.command = value(arguments_, index++, argument);
    else if (argument === "--arg")
      options.arguments.push(value(arguments_, index++, argument, true));
    else if (argument === "--timeout-ms") {
      const timeout = Number(value(arguments_, index++, argument));
      if (!Number.isSafeInteger(timeout) || timeout < 100 || timeout > 60_000)
        throw new Error("--timeout-ms must be an integer from 100 to 60000");
      options.timeoutMs = timeout;
    } else if (argument === "--include") {
      const mapping = value(arguments_, index++, argument);
      const separator = mapping.lastIndexOf("=");
      if (separator <= 0 || separator === mapping.length - 1)
        throw new Error("--include requires SOURCE=PACKAGE_PATH");
      options.includes.push({
        source: mapping.slice(0, separator),
        destination: mapping.slice(separator + 1),
      });
    } else if (argument === "--force") options.force = true;
    else if (argument === "--json") options.json = true;
    else if (argument === "--skip-runtime") options.skipRuntime = true;
    else throw new Error(`Unknown argument: ${argument}`);
  }
  return options;
}

function write(io, text) {
  io.stdout.write(`${text}\n`);
}

function renderReport(report, io, json) {
  if (json) {
    write(io, JSON.stringify(report, null, 2));
    return;
  }
  write(
    io,
    `${report.status === "passed" ? "PASS" : "FAIL"} ${report.extension.kind} ${report.extension.id ?? "unknown"} ${report.extension.version ?? ""}`.trim(),
  );
  for (const check of report.checks)
    write(io, `  ${check.status.toUpperCase()} ${check.id}: ${check.summary}`);
  for (const entry of report.issues)
    write(
      io,
      `  ${entry.severity.toUpperCase()} ${entry.code} (${entry.path}): ${entry.message}`,
    );
  if (report.archive_sha256) write(io, `  SHA256 ${report.archive_sha256}`);
}

async function persistReport(options, report, io) {
  if (!options.reportPath) return;
  const output = await writeCompatibilityReport(
    report,
    options.reportPath,
    options.force,
  );
  if (!options.json) write(io, `Compatibility report: ${output}`);
}

async function runNew(arguments_, io) {
  const options = parseOptions(
    arguments_,
    new Set([
      "--kind",
      "--directory",
      "--id",
      "--name",
      "--author",
      "--version",
    ]),
  );
  for (const name of ["kind", "directory", "id", "name", "author"])
    if (!options[name]) throw new Error(`--${name} is required`);
  const result = await scaffoldExtension({
    kind: options.kind,
    directory: options.directory,
    id: options.id,
    name: options.name,
    author: options.author,
    ...(options.version ? { version: options.version } : {}),
  });
  write(
    io,
    `Created ${result.kind} scaffold for ${result.id} in ${result.directory}`,
  );
  write(io, `Next: cd "${result.directory}"`);
  return 0;
}

async function runValidate(arguments_, io) {
  const options = parseOptions(
    arguments_,
    new Set([
      "--source",
      "--package",
      "--kind",
      "--report",
      "--json",
      "--force",
    ]),
  );
  if (Boolean(options.sourceDirectory) === Boolean(options.packagePath))
    throw new Error("Choose exactly one of --source or --package");
  const report = options.sourceDirectory
    ? await validateSource(options)
    : await inspectPackage(options);
  renderReport(report, io, options.json);
  await persistReport(options, report, io);
  return report.status === "passed" ? 0 : 1;
}

async function runPackage(arguments_, io) {
  const options = parseOptions(
    arguments_,
    new Set([
      "--source",
      "--output",
      "--kind",
      "--include",
      "--report",
      "--json",
      "--force",
    ]),
  );
  if (!options.sourceDirectory || !options.outputPath)
    throw new Error("--source and --output are required");
  const kind =
    options.kind ?? (await inferSourceKind(resolve(options.sourceDirectory)));
  const definition = extensionDefinition(kind);
  const result = await buildPackage(kind, options);
  const report = await inspectPackage({
    packagePath: result.output,
    command: "package",
  });
  renderReport(report, io, options.json);
  if (!options.json)
    write(
      io,
      `Package: ${basename(result.output)} (${result.files} files, ${definition.mediaType})`,
    );
  await persistReport(options, report, io);
  return report.status === "passed" ? 0 : 1;
}

async function runTest(arguments_, io) {
  const options = parseOptions(
    arguments_,
    new Set([
      "--source",
      "--kind",
      "--include",
      "--command",
      "--arg",
      "--timeout-ms",
      "--skip-runtime",
      "--report",
      "--json",
      "--force",
    ]),
  );
  if (!options.sourceDirectory) throw new Error("--source is required");
  const report = await testExtension(options);
  renderReport(report, io, options.json);
  await persistReport(options, report, io);
  return report.status === "passed" ? 0 : 1;
}

async function runSchemas(arguments_, io) {
  const options = parseOptions(
    arguments_,
    new Set(["--output", "--json", "--force"]),
  );
  if (options.outputPath) {
    const result = await copySchemaBundle(options.outputPath, options.force);
    if (options.json) write(io, JSON.stringify(result.catalog, null, 2));
    else
      write(
        io,
        `Copied ${result.catalog.schemas.length} schemas to ${result.output}`,
      );
    return 0;
  }
  const catalog = await schemaCatalog();
  if (options.json) write(io, JSON.stringify(catalog, null, 2));
  else
    for (const schema of catalog.schemas)
      write(io, `${schema.filename}  ${schema.sha256}`);
  return 0;
}

export async function runCli(
  arguments_,
  io = { stdout: process.stdout, stderr: process.stderr },
) {
  const [command, ...rest] = arguments_;
  if (!command || command === "--help" || command === "-h") {
    io.stdout.write(HELP);
    return 0;
  }
  if (command === "--version" || command === "-V") {
    write(io, EDK_VERSION);
    return 0;
  }
  switch (command) {
    case "new":
    case "create":
      return runNew(rest, io);
    case "validate":
      return runValidate(rest, io);
    case "package":
      return runPackage(rest, io);
    case "test":
      return runTest(rest, io);
    case "schemas":
      return runSchemas(rest, io);
    default:
      throw new Error(`Unknown command: ${command}\n\n${HELP}`);
  }
}

export async function main(arguments_ = process.argv.slice(2)) {
  try {
    process.exitCode = await runCli(arguments_);
  } catch (error) {
    process.stderr.write(
      `${error instanceof Error ? error.message : String(error)}\n`,
    );
    process.exitCode = 1;
  }
}
