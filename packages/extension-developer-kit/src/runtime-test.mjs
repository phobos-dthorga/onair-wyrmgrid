import { spawn } from "node:child_process";
import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { isDeepStrictEqual } from "node:util";

import {
  EDK_VERSION,
  currentPlatform,
  declaredEntryPoints,
  extensionDefinition,
} from "./catalog.mjs";

const CONTROL_BYTES = 64 * 1024;
const BUFFERED_OUTPUT_BYTES = 128 * 1024;

function scrubbedEnvironment() {
  const allowed = [
    "PATH",
    "Path",
    "PATHEXT",
    "SYSTEMROOT",
    "SystemRoot",
    "WINDIR",
    "TEMP",
    "TMP",
    "TMPDIR",
    "LANG",
    "LC_ALL",
  ];
  const environment = {};
  for (const key of allowed)
    if (process.env[key] !== undefined) environment[key] = process.env[key];
  environment.PYTHONUNBUFFERED = "1";
  return environment;
}

function envelope(protocolVersion, sequence, payload) {
  return { protocol_version: protocolVersion, sequence, payload };
}

function encodeJsonFrame(value) {
  const contents = Buffer.from(JSON.stringify(value), "utf8");
  if (contents.length === 0 || contents.length > CONTROL_BYTES)
    throw new Error("EDK host control frame is outside its bound");
  const length = Buffer.alloc(4);
  length.writeUInt32BE(contents.length);
  return Buffer.concat([length, contents]);
}

function encodeBinaryFrame(value, body = Buffer.alloc(0)) {
  const contents = Buffer.from(JSON.stringify(value), "utf8");
  if (contents.length === 0 || contents.length > CONTROL_BYTES)
    throw new Error("EDK host control frame is outside its bound");
  const lengths = Buffer.alloc(8);
  lengths.writeUInt32BE(contents.length, 0);
  lengths.writeUInt32BE(body.length, 4);
  return Buffer.concat([lengths, contents, body]);
}

function parseFrameJson(contents) {
  return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(contents));
}

class BufferedReader {
  #buffer = Buffer.alloc(0);
  #ended = false;
  #error;
  #notify;

  constructor(stream, child, input) {
    stream.on("data", (chunk) => {
      if (this.#buffer.length + chunk.length > BUFFERED_OUTPUT_BYTES) {
        this.#error = new Error("Extension exceeded the bounded output buffer");
        child.kill();
        this.#wake();
        return;
      }
      this.#buffer = Buffer.concat([this.#buffer, chunk]);
      this.#wake();
    });
    stream.on("end", () => {
      this.#ended = true;
      this.#wake();
    });
    stream.on("error", (error) => {
      this.#error = error;
      this.#wake();
    });
    child.on("error", (error) => {
      this.#error = error;
      this.#wake();
    });
    input.on("error", (error) => {
      this.#error = error;
      this.#wake();
    });
  }

  #wake() {
    if (this.#notify) {
      const notify = this.#notify;
      this.#notify = undefined;
      notify();
    }
  }

  async exact(length, timeoutMs) {
    const deadline = Date.now() + timeoutMs;
    while (this.#buffer.length < length) {
      if (this.#error) throw this.#error;
      if (this.#ended) throw new Error("Extension process closed its output");
      const remaining = deadline - Date.now();
      if (remaining <= 0)
        throw new Error("Extension process response timed out");
      await new Promise((resolveWait, reject) => {
        const timer = setTimeout(() => {
          this.#notify = undefined;
          reject(new Error("Extension process response timed out"));
        }, remaining);
        this.#notify = () => {
          clearTimeout(timer);
          resolveWait();
        };
      });
    }
    const result = this.#buffer.subarray(0, length);
    this.#buffer = this.#buffer.subarray(length);
    return result;
  }

  async jsonFrame(timeoutMs) {
    const length = (await this.exact(4, timeoutMs)).readUInt32BE();
    if (length === 0 || length > CONTROL_BYTES)
      throw new Error("Extension emitted an invalid control frame length");
    return parseFrameJson(await this.exact(length, timeoutMs));
  }

  async binaryFrame(timeoutMs, maximumBody) {
    const lengths = await this.exact(8, timeoutMs);
    const headerLength = lengths.readUInt32BE(0);
    const bodyLength = lengths.readUInt32BE(4);
    if (
      headerLength === 0 ||
      headerLength > CONTROL_BYTES ||
      bodyLength > maximumBody
    )
      throw new Error("Extension emitted an invalid binary frame length");
    const header = parseFrameJson(await this.exact(headerLength, timeoutMs));
    const body = await this.exact(bodyLength, timeoutMs);
    return { header, body };
  }
}

function validateEnvelope(value, protocolVersion, previousSequence) {
  if (
    typeof value !== "object" ||
    value === null ||
    value.protocol_version !== protocolVersion ||
    !Number.isSafeInteger(value.sequence) ||
    value.sequence <= previousSequence ||
    typeof value.payload !== "object" ||
    value.payload === null
  )
    throw new Error("Extension emitted an invalid or non-monotonic envelope");
  return value.sequence;
}

function naturalCommand(definition, manifest, sourceDirectory) {
  if (definition.kind === "plugin") {
    return {
      command:
        process.env.WYRMGRID_EDK_PYTHON ??
        (process.platform === "win32" ? "python" : "python3"),
      arguments: [join(sourceDirectory, manifest.entry_point)],
    };
  }
  const platform = currentPlatform();
  if (!platform)
    throw new Error("EDK runtime testing does not support this host platform");
  if (!manifest.platforms.includes(platform))
    throw new Error(
      `Extension does not declare the current platform: ${platform}`,
    );
  const [entryPoint] = declaredEntryPoints(definition, {
    ...manifest,
    platforms: [platform],
  });
  return {
    command: join(sourceDirectory, entryPoint),
    arguments: [],
  };
}

function descriptorMatches(
  manifest,
  descriptor,
  capabilitiesField = "capabilities",
) {
  return (
    descriptor?.id === manifest.id &&
    descriptor?.name === manifest.name &&
    descriptor?.version === manifest.version &&
    Array.isArray(descriptor?.[capabilitiesField])
  );
}

function exactCapabilities(manifest, descriptor) {
  return (
    descriptor.capabilities.length === manifest.capabilities.length &&
    new Set(descriptor.capabilities).size === descriptor.capabilities.length &&
    descriptor.capabilities.every((capability) =>
      manifest.capabilities.includes(capability),
    )
  );
}

function safeFailureMessage(error) {
  const raw = error instanceof Error ? error.message : String(error);
  const stderrSuffix = "; extension stderr was omitted";
  const stderrWasOmitted = raw.endsWith(stderrSuffix);
  const message = stderrWasOmitted ? raw.slice(0, -stderrSuffix.length) : raw;
  const accepted =
    [
      "EDK host control frame is outside its bound",
      "EDK runtime testing does not support this host platform",
      "Extension exceeded the bounded output buffer",
      "Extension process closed its output",
      "Extension process did not stop after shutdown",
      "Extension process response timed out",
      "Extension emitted an invalid control frame length",
      "Extension emitted an invalid binary frame length",
      "Extension emitted an invalid or non-monotonic envelope",
      "Plugin did not return the expected ready message",
      "Simulator provider did not return a compatible hello",
      "Startup messages must not contain a binary body",
      "Provider hello does not match its manifest",
      "Provider emitted an unexpected startup message",
      "Provider did not complete its hello/starting/ready handshake",
      "Audio provider did not acknowledge shutdown",
      "Audio codec did not acknowledge shutdown",
    ].includes(message) ||
    /^Extension does not declare the current platform: [a-z0-9_]+$/u.test(
      message,
    ) ||
    /^Extension process exited with code -?[0-9]+$/u.test(message) ||
    /^Extension process stopped by [A-Z0-9]+$/u.test(message);
  return `${accepted ? message : "Extension runtime conformance failed"}${
    stderrWasOmitted ? stderrSuffix : ""
  }`;
}

async function waitForExit(child, timeoutMs) {
  if (child.exitCode !== null) return child.exitCode;
  return new Promise((resolveExit, reject) => {
    const timer = setTimeout(() => {
      reject(new Error("Extension process did not stop after shutdown"));
    }, timeoutMs);
    child.once("exit", (code, signal) => {
      clearTimeout(timer);
      if (signal) reject(new Error(`Extension process stopped by ${signal}`));
      else resolveExit(code);
    });
  });
}

async function testPlugin(child, reader, manifest, timeoutMs) {
  child.stdin.write(
    encodeJsonFrame(
      envelope(1, 1, {
        type: "hello",
        host_version: EDK_VERSION,
        plugin_id: manifest.id,
        granted_permissions: manifest.permissions ?? [],
        weather_capabilities: manifest.weather_capabilities ?? [],
        network_origins: manifest.network_origins ?? [],
      }),
    ),
  );
  const ready = await reader.jsonFrame(timeoutMs);
  validateEnvelope(ready, 1, 0);
  if (
    ready.payload.type !== "ready" ||
    ready.payload.plugin_id !== manifest.id ||
    ready.payload.api_version !== 1
  )
    throw new Error("Plugin did not return the expected ready message");
  child.stdin.end(encodeJsonFrame(envelope(1, 2, { type: "shutdown" })));
}

async function testSimulatorProvider(child, reader, manifest, timeoutMs) {
  child.stdin.write(
    encodeJsonFrame(
      envelope(1, 1, {
        type: "hello",
        host_version: EDK_VERSION,
        provider_id: manifest.id,
        requested_capabilities: manifest.capabilities,
      }),
    ),
  );
  const hello = await reader.jsonFrame(timeoutMs);
  validateEnvelope(hello, 1, 0);
  if (
    hello.payload.type !== "hello" ||
    !descriptorMatches(manifest, hello.payload.provider) ||
    !manifest.simulators.includes(hello.payload.provider.simulator) ||
    !/^[a-z0-9][a-z0-9_.-]*$/u.test(
      hello.payload.provider.architecture ?? "",
    ) ||
    !exactCapabilities(manifest, hello.payload.provider)
  )
    throw new Error("Simulator provider did not return a compatible hello");
  child.stdin.end(encodeJsonFrame(envelope(1, 2, { type: "shutdown" })));
}

async function readAudioStartup(reader, manifest, timeoutMs, codec) {
  const expectedStates = ["starting", "ready"];
  let previousSequence = 0;
  let helloSeen = false;
  for (let index = 0; index < 3; index += 1) {
    const { header, body } = await reader.binaryFrame(
      timeoutMs,
      codec ? 16 * 1024 : 64 * 1024,
    );
    previousSequence = validateEnvelope(
      header,
      codec ? 1 : 2,
      previousSequence,
    );
    if (body.length !== 0)
      throw new Error("Startup messages must not contain a binary body");
    if (header.payload.type === "hello") {
      const descriptor = codec ? header.payload.codec : header.payload.provider;
      const matching = codec
        ? descriptor?.id === manifest.id &&
          descriptor?.name === manifest.name &&
          descriptor?.version === manifest.version &&
          descriptor?.platform === currentPlatform() &&
          isDeepStrictEqual(descriptor?.profiles, manifest.profiles)
        : descriptorMatches(manifest, descriptor) &&
          descriptor.platform === currentPlatform() &&
          exactCapabilities(manifest, descriptor);
      if (!matching)
        throw new Error("Provider hello does not match its manifest");
      helloSeen = true;
      continue;
    }
    if (
      header.payload.type !== "state" ||
      !expectedStates.includes(header.payload.state) ||
      !/^[a-z0-9][a-z0-9_.-]*$/u.test(header.payload.code ?? "")
    )
      throw new Error("Provider emitted an unexpected startup message");
    expectedStates.splice(expectedStates.indexOf(header.payload.state), 1);
  }
  if (!helloSeen || expectedStates.length !== 0)
    throw new Error(
      "Provider did not complete its hello/starting/ready handshake",
    );
  return previousSequence;
}

async function testAudioProvider(child, reader, manifest, timeoutMs) {
  child.stdin.write(
    encodeJsonFrame(
      envelope(2, 1, {
        type: "hello",
        host_version: EDK_VERSION,
        provider_id: manifest.id,
      }),
    ),
  );
  const previous = await readAudioStartup(reader, manifest, timeoutMs, false);
  child.stdin.end(encodeJsonFrame(envelope(2, 2, { type: "shutdown" })));
  const { header, body } = await reader.binaryFrame(timeoutMs, 64 * 1024);
  validateEnvelope(header, 2, previous);
  if (
    body.length !== 0 ||
    header.payload.type !== "state" ||
    header.payload.state !== "stopped" ||
    !/^[a-z0-9][a-z0-9_.-]*$/u.test(header.payload.code ?? "")
  )
    throw new Error("Audio provider did not acknowledge shutdown");
}

async function testAudioCodec(child, reader, manifest, timeoutMs) {
  child.stdin.write(
    encodeBinaryFrame(
      envelope(1, 1, {
        type: "hello",
        host_version: EDK_VERSION,
        codec_provider_id: manifest.id,
      }),
    ),
  );
  const previous = await readAudioStartup(reader, manifest, timeoutMs, true);
  child.stdin.end(encodeBinaryFrame(envelope(1, 2, { type: "shutdown" })));
  const { header, body } = await reader.binaryFrame(timeoutMs, 16 * 1024);
  validateEnvelope(header, 1, previous);
  if (
    body.length !== 0 ||
    header.payload.type !== "state" ||
    header.payload.state !== "stopped" ||
    !/^[a-z0-9][a-z0-9_.-]*$/u.test(header.payload.code ?? "")
  )
    throw new Error("Audio codec did not acknowledge shutdown");
}

async function executeRuntime({
  sourceDirectory,
  kind,
  command,
  arguments: commandArguments = [],
  timeoutMs = 5_000,
}) {
  const source = resolve(sourceDirectory);
  const definition = extensionDefinition(kind);
  const manifest = JSON.parse(
    await readFile(join(source, definition.manifestPath), "utf8"),
  );
  const launch = command
    ? { command, arguments: commandArguments }
    : naturalCommand(definition, manifest, source);
  const child = spawn(launch.command, launch.arguments, {
    cwd: source,
    env: scrubbedEnvironment(),
    stdio: ["pipe", "pipe", "pipe"],
    windowsHide: true,
  });
  const reader = new BufferedReader(child.stdout, child, child.stdin);
  let stderrSeen = false;
  child.stderr.on("data", () => {
    stderrSeen = true;
  });
  try {
    switch (kind) {
      case "plugin":
        await testPlugin(child, reader, manifest, timeoutMs);
        break;
      case "simulator-provider":
        await testSimulatorProvider(child, reader, manifest, timeoutMs);
        break;
      case "audio-provider":
        await testAudioProvider(child, reader, manifest, timeoutMs);
        break;
      case "audio-codec":
        await testAudioCodec(child, reader, manifest, timeoutMs);
        break;
    }
    const exitCode = await waitForExit(child, timeoutMs);
    if (exitCode !== 0)
      throw new Error(`Extension process exited with code ${exitCode}`);
    return {
      command: launch.command,
      protocol: definition.protocolName,
      protocolVersion: definition.protocolVersion,
    };
  } catch (error) {
    if (child.exitCode === null) child.kill();
    throw new Error(
      `${error instanceof Error ? error.message : error}${
        stderrSeen ? "; extension stderr was omitted" : ""
      }`,
    );
  }
}

export async function testRuntime(options) {
  try {
    return await executeRuntime(options);
  } catch (error) {
    throw new Error(safeFailureMessage(error));
  }
}
