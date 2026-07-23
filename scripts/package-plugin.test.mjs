import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { inflateRawSync } from "node:zlib";
import {
  PACKAGE_MANIFEST_NAME,
  buildAudioCodecPackage,
  buildAudioProviderPackage,
  buildPluginPackage,
  buildSimulatorProviderPackage,
} from "./package-plugin.mjs";

async function fixture() {
  const root = await mkdtemp(join(tmpdir(), "wyrmgrid-plugin-package-"));
  const source = join(root, "source");
  await mkdir(join(source, "src"), { recursive: true });
  await writeFile(
    join(source, "plugin.json"),
    JSON.stringify({
      id: "org.wyrmgrid.test.packager",
      name: "Packager test",
      version: "1.2.3",
      api_version: 1,
      author: "Tests",
      runtime: "python",
      entry_point: "src/main.py",
      permissions: [],
    }),
  );
  await writeFile(join(source, "src", "main.py"), "print('ready')\n");
  const sdk = join(root, "sdk.py");
  await writeFile(sdk, "# sdk\n");
  return { root, source, sdk, output: join(root, "test.wyrmplugin") };
}

function zipEntries(archive) {
  const entries = new Map();
  let offset = 0;
  while (archive.readUInt32LE(offset) === 0x04034b50) {
    const method = archive.readUInt16LE(offset + 8);
    const compressedSize = archive.readUInt32LE(offset + 18);
    const nameLength = archive.readUInt16LE(offset + 26);
    const extraLength = archive.readUInt16LE(offset + 28);
    const nameStart = offset + 30;
    const name = archive
      .subarray(nameStart, nameStart + nameLength)
      .toString("utf8");
    const contentsStart = nameStart + nameLength + extraLength;
    const compressed = archive.subarray(
      contentsStart,
      contentsStart + compressedSize,
    );
    assert.equal(method, 8);
    entries.set(name, inflateRawSync(compressed));
    offset = contentsStart + compressedSize;
  }
  return entries;
}

test("builds a deterministic exact-inventory ordinary plugin package", async (t) => {
  const example = await fixture();
  t.after(() => rm(example.root, { recursive: true, force: true }));
  const options = {
    sourceDirectory: example.source,
    outputPath: example.output,
    includes: [
      {
        source: example.sdk,
        destination: "src/wyrmgrid_sdk/__init__.py",
      },
    ],
    force: true,
  };
  const first = await buildPluginPackage(options);
  const firstBytes = await readFile(example.output);
  const second = await buildPluginPackage(options);
  const secondBytes = await readFile(example.output);
  assert.deepEqual(secondBytes, firstBytes);
  assert.equal(second.archiveSha256, first.archiveSha256);

  const entries = zipEntries(firstBytes);
  assert.deepEqual(
    [...entries.keys()],
    [
      PACKAGE_MANIFEST_NAME,
      "plugin.json",
      "src/main.py",
      "src/wyrmgrid_sdk/__init__.py",
    ],
  );
  const manifest = JSON.parse(entries.get(PACKAGE_MANIFEST_NAME).toString());
  assert.equal(manifest.schema_version, 1);
  assert.equal(manifest.kind, "ordinary_plugin");
  assert.equal(manifest.id, "org.wyrmgrid.test.packager");
  assert.equal(manifest.version, "1.2.3");
  assert.equal(manifest.files.length, 3);
  assert.ok(manifest.files.every((file) => /^[0-9a-f]{64}$/.test(file.sha256)));
});

test("rejects an unsafe included destination", async (t) => {
  const example = await fixture();
  t.after(() => rm(example.root, { recursive: true, force: true }));
  await assert.rejects(
    buildPluginPackage({
      sourceDirectory: example.source,
      outputPath: example.output,
      includes: [{ source: example.sdk, destination: "../escape.py" }],
    }),
    /Unsafe package path/,
  );
});

test("does not replace an existing package unless force is explicit", async (t) => {
  const example = await fixture();
  t.after(() => rm(example.root, { recursive: true, force: true }));
  await buildPluginPackage({
    sourceDirectory: example.source,
    outputPath: example.output,
  });
  await assert.rejects(
    buildPluginPackage({
      sourceDirectory: example.source,
      outputPath: example.output,
    }),
    /EEXIST/,
  );
});

test("builds a distinct simulator provider package contract", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "wyrmgrid-provider-package-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const manifestPath = join(root, "provider.json");
  const executablePath = join(root, "provider.exe");
  const outputPath = join(root, "test.wyrmprovider");
  await writeFile(
    manifestPath,
    JSON.stringify({
      schema_version: 2,
      id: "org.wyrmgrid.test.simulator-provider",
      name: "Provider package test",
      version: "1.0.0",
      bridge_protocol_version: 1,
      author: "Tests",
      entry_point: "provider.exe",
      platforms: ["windows_x86_64"],
      simulators: ["test_simulator"],
      capabilities: ["telemetry_read"],
    }),
  );
  await writeFile(executablePath, "sanitized executable fixture");

  await buildSimulatorProviderPackage({
    sourceDirectory: root,
    outputPath,
    includeSourceDirectory: false,
    includes: [
      { source: manifestPath, destination: "provider.json" },
      { source: executablePath, destination: "provider.exe" },
    ],
  });
  const entries = zipEntries(await readFile(outputPath));
  const manifest = JSON.parse(entries.get(PACKAGE_MANIFEST_NAME).toString());
  assert.equal(manifest.kind, "simulator_provider");
  assert.equal(manifest.manifest_path, "provider.json");
  assert.deepEqual(
    [...entries.keys()],
    [PACKAGE_MANIFEST_NAME, "provider.exe", "provider.json"],
  );
});

test("builds a distinct audio provider package contract", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "wyrmgrid-audio-package-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const manifestPath = join(root, "audio-provider.json");
  const executablePath = join(root, "audio-provider.exe");
  const outputPath = join(root, "test.wyrmaudio");
  await writeFile(
    manifestPath,
    JSON.stringify({
      schema_version: 2,
      id: "org.wyrmgrid.test.audio-provider",
      name: "Audio provider package test",
      version: "1.0.0",
      audio_protocol_version: 2,
      author: "Tests",
      entry_point: "audio-provider",
      platforms: ["windows_x86_64"],
      capabilities: ["source_enumeration", "pcm_s16le_capture"],
    }),
  );
  await writeFile(executablePath, "sanitized audio executable fixture");

  await buildAudioProviderPackage({
    sourceDirectory: root,
    outputPath,
    includeSourceDirectory: false,
    includes: [
      { source: manifestPath, destination: "audio-provider.json" },
      { source: executablePath, destination: "audio-provider.exe" },
    ],
  });
  const entries = zipEntries(await readFile(outputPath));
  const manifest = JSON.parse(entries.get(PACKAGE_MANIFEST_NAME).toString());
  assert.equal(manifest.kind, "audio_provider");
  assert.equal(manifest.manifest_path, "audio-provider.json");
  assert.deepEqual(
    [...entries.keys()],
    [PACKAGE_MANIFEST_NAME, "audio-provider.exe", "audio-provider.json"],
  );
});

test("builds a distinct audio codec package contract", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "wyrmgrid-codec-package-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const manifestPath = join(root, "audio-codec.json");
  const executablePath = join(root, "audio-codec.exe");
  const outputPath = join(root, "test.wyrmcodec");
  await writeFile(
    manifestPath,
    JSON.stringify({
      schema_version: 1,
      id: "org.wyrmgrid.test.audio-codec",
      name: "Audio codec package test",
      version: "1.0.0",
      codec_protocol_version: 1,
      author: "Tests",
      entry_point: "audio-codec",
      platforms: ["windows_x86_64"],
      capabilities: ["encode_pcm_s16le"],
      profiles: [
        {
          id: "pilot_microphone_v1",
          codec_id: "test-codec",
          media_type: "audio/test",
          channels: 1,
          sample_rate_hz: 48000,
          target_bitrate_bps: 32000,
          packet_duration_48khz_frames: 960,
        },
      ],
    }),
  );
  await writeFile(executablePath, "sanitized codec executable fixture");

  await buildAudioCodecPackage({
    sourceDirectory: root,
    outputPath,
    includeSourceDirectory: false,
    includes: [
      { source: manifestPath, destination: "audio-codec.json" },
      { source: executablePath, destination: "audio-codec.exe" },
    ],
  });
  const entries = zipEntries(await readFile(outputPath));
  const manifest = JSON.parse(entries.get(PACKAGE_MANIFEST_NAME).toString());
  assert.equal(manifest.kind, "audio_codec_provider");
  assert.equal(manifest.manifest_path, "audio-codec.json");
  assert.deepEqual(
    [...entries.keys()],
    [PACKAGE_MANIFEST_NAME, "audio-codec.exe", "audio-codec.json"],
  );
});
