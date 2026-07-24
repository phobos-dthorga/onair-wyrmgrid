import { readFile } from "node:fs/promises";

const [, , kind, manifestPath] = process.argv;
const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
let buffer = Buffer.alloc(0);

async function exact(length) {
  while (buffer.length < length) {
    const chunk = process.stdin.read();
    if (chunk) buffer = Buffer.concat([buffer, chunk]);
    else
      await new Promise((resolve) => {
        process.stdin.once("readable", resolve);
      });
  }
  const result = buffer.subarray(0, length);
  buffer = buffer.subarray(length);
  return result;
}

async function readJsonFrame() {
  const length = (await exact(4)).readUInt32BE();
  return JSON.parse((await exact(length)).toString("utf8"));
}

async function readBinaryFrame() {
  const lengths = await exact(8);
  const headerLength = lengths.readUInt32BE(0);
  const bodyLength = lengths.readUInt32BE(4);
  return {
    header: JSON.parse((await exact(headerLength)).toString("utf8")),
    body: await exact(bodyLength),
  };
}

function framed(value, body) {
  const header = Buffer.from(JSON.stringify(value));
  if (body === undefined) {
    const length = Buffer.alloc(4);
    length.writeUInt32BE(header.length);
    return Buffer.concat([length, header]);
  }
  const lengths = Buffer.alloc(8);
  lengths.writeUInt32BE(header.length, 0);
  lengths.writeUInt32BE(body.length, 4);
  return Buffer.concat([lengths, header, body]);
}

function send(protocolVersion, sequence, payload, binary = false) {
  process.stdout.write(
    framed(
      { protocol_version: protocolVersion, sequence, payload },
      binary ? Buffer.alloc(0) : undefined,
    ),
  );
}

if (kind === "plugin") {
  const hello = await readJsonFrame();
  if (hello.payload.plugin_id !== manifest.id) process.exit(2);
  send(1, 1, {
    type: "ready",
    plugin_id: manifest.id,
    api_version: 1,
  });
  await readJsonFrame();
} else if (kind === "simulator-provider") {
  const hello = await readJsonFrame();
  if (hello.payload.provider_id !== manifest.id) process.exit(2);
  send(1, 1, {
    type: "hello",
    provider: {
      id: manifest.id,
      name: manifest.name,
      version: manifest.version,
      simulator: manifest.simulators[0],
      architecture: manifest.platforms[0],
      capabilities: manifest.capabilities,
    },
  });
  await readJsonFrame();
} else if (kind === "audio-provider") {
  const hello = await readJsonFrame();
  if (hello.payload.provider_id !== manifest.id) process.exit(2);
  send(
    2,
    1,
    {
      type: "hello",
      provider: {
        id: manifest.id,
        name: manifest.name,
        version: manifest.version,
        platform: manifest.platforms[0],
        capabilities: manifest.capabilities,
      },
    },
    true,
  );
  send(2, 2, { type: "state", state: "starting", code: "starting" }, true);
  send(2, 3, { type: "state", state: "ready", code: "ready" }, true);
  await readJsonFrame();
  send(2, 4, { type: "state", state: "stopped", code: "stopped" }, true);
} else if (kind === "audio-codec") {
  const hello = await readBinaryFrame();
  if (hello.header.payload.codec_provider_id !== manifest.id) process.exit(2);
  send(
    1,
    1,
    {
      type: "hello",
      codec: {
        id: manifest.id,
        name: manifest.name,
        version: manifest.version,
        platform: manifest.platforms[0],
        profiles: manifest.profiles,
      },
    },
    true,
  );
  send(1, 2, { type: "state", state: "starting", code: "starting" }, true);
  send(1, 3, { type: "state", state: "ready", code: "ready" }, true);
  await readBinaryFrame();
  send(1, 4, { type: "state", state: "stopped", code: "stopped" }, true);
} else process.exit(3);
