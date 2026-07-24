import { inflateRawSync } from "node:zlib";

import { crc32 } from "./binary.mjs";
import { PACKAGE_LIMITS, validatePackagePath } from "./contract.mjs";

const END_SIGNATURE = 0x06054b50;
const CENTRAL_SIGNATURE = 0x02014b50;
const LOCAL_SIGNATURE = 0x04034b50;
const STORED = 0;
const DEFLATED = 8;
const MAX_EOCD_SEARCH = 65_557;

function findEndOfCentralDirectory(archive) {
  const minimum = Math.max(0, archive.length - MAX_EOCD_SEARCH);
  for (let offset = archive.length - 22; offset >= minimum; offset -= 1)
    if (archive.readUInt32LE(offset) === END_SIGNATURE) return offset;
  throw new Error("Archive has no valid ZIP end record");
}

function readEntryContents(archive, entry) {
  if (entry.localOffset + 30 > archive.length)
    throw new Error(`Truncated local ZIP header for ${entry.path}`);
  if (archive.readUInt32LE(entry.localOffset) !== LOCAL_SIGNATURE)
    throw new Error(`Invalid local ZIP header for ${entry.path}`);
  const localFlags = archive.readUInt16LE(entry.localOffset + 6);
  const localMethod = archive.readUInt16LE(entry.localOffset + 8);
  const nameLength = archive.readUInt16LE(entry.localOffset + 26);
  const extraLength = archive.readUInt16LE(entry.localOffset + 28);
  const nameStart = entry.localOffset + 30;
  const nameEnd = nameStart + nameLength;
  const dataStart = nameEnd + extraLength;
  const dataEnd = dataStart + entry.compressedSize;
  if (
    dataEnd > archive.length ||
    localFlags !== entry.flags ||
    localMethod !== entry.method ||
    archive.subarray(nameStart, nameEnd).toString("utf8") !== entry.path
  )
    throw new Error(`Local ZIP metadata does not match ${entry.path}`);
  const compressed = archive.subarray(dataStart, dataEnd);
  let contents;
  if (entry.method === STORED) contents = Buffer.from(compressed);
  else {
    try {
      contents = inflateRawSync(compressed, {
        maxOutputLength: PACKAGE_LIMITS.fileBytes,
      });
    } catch {
      throw new Error(`Could not decompress ${entry.path}`);
    }
  }
  if (
    contents.length !== entry.expandedSize ||
    crc32(contents) !== entry.checksum
  )
    throw new Error(`ZIP size or checksum mismatch for ${entry.path}`);
  return contents;
}

export function readZipEntries(archive) {
  if (!Buffer.isBuffer(archive)) throw new Error("Archive must be a Buffer");
  if (archive.length === 0 || archive.length > PACKAGE_LIMITS.archiveBytes)
    throw new Error("Archive is outside the 32 MiB package bound");
  const endOffset = findEndOfCentralDirectory(archive);
  const disk = archive.readUInt16LE(endOffset + 4);
  const centralDisk = archive.readUInt16LE(endOffset + 6);
  const entriesOnDisk = archive.readUInt16LE(endOffset + 8);
  const entryCount = archive.readUInt16LE(endOffset + 10);
  const centralSize = archive.readUInt32LE(endOffset + 12);
  const centralOffset = archive.readUInt32LE(endOffset + 16);
  const commentLength = archive.readUInt16LE(endOffset + 20);
  if (
    disk !== 0 ||
    centralDisk !== 0 ||
    entriesOnDisk !== entryCount ||
    entryCount < 2 ||
    entryCount > PACKAGE_LIMITS.files + 1 ||
    centralOffset + centralSize !== endOffset ||
    endOffset + 22 + commentLength !== archive.length
  )
    throw new Error("ZIP central directory is invalid or unsupported");

  const metadata = [];
  const exactPaths = new Set();
  const foldedPaths = new Set();
  let offset = centralOffset;
  let expandedBytes = 0;
  for (let index = 0; index < entryCount; index += 1) {
    if (
      offset + 46 > endOffset ||
      archive.readUInt32LE(offset) !== CENTRAL_SIGNATURE
    )
      throw new Error("ZIP central directory entry is truncated or invalid");
    const flags = archive.readUInt16LE(offset + 8);
    const method = archive.readUInt16LE(offset + 10);
    const checksum = archive.readUInt32LE(offset + 16);
    const compressedSize = archive.readUInt32LE(offset + 20);
    const expandedSize = archive.readUInt32LE(offset + 24);
    const nameLength = archive.readUInt16LE(offset + 28);
    const extraLength = archive.readUInt16LE(offset + 30);
    const entryCommentLength = archive.readUInt16LE(offset + 32);
    const diskStart = archive.readUInt16LE(offset + 34);
    const externalAttributes = archive.readUInt32LE(offset + 38);
    const localOffset = archive.readUInt32LE(offset + 42);
    const nameStart = offset + 46;
    const next = nameStart + nameLength + extraLength + entryCommentLength;
    if (next > endOffset)
      throw new Error("ZIP central directory entry extends past its bound");
    const path = archive
      .subarray(nameStart, nameStart + nameLength)
      .toString("utf8");
    validatePackagePath(path);
    const unixMode = externalAttributes >>> 16;
    const fileType = unixMode & 0xf000;
    if (
      flags & 0x0001 ||
      ![STORED, DEFLATED].includes(method) ||
      diskStart !== 0 ||
      expandedSize < 1 ||
      expandedSize > PACKAGE_LIMITS.fileBytes ||
      compressedSize < 1 ||
      fileType === 0xa000 ||
      fileType === 0x4000
    )
      throw new Error(`Unsafe or unsupported ZIP entry: ${path}`);
    const folded = path.toLowerCase();
    if (exactPaths.has(path) || foldedPaths.has(folded))
      throw new Error(`Duplicate or case-colliding ZIP path: ${path}`);
    exactPaths.add(path);
    foldedPaths.add(folded);
    expandedBytes += expandedSize;
    if (expandedBytes > PACKAGE_LIMITS.expandedBytes)
      throw new Error("Expanded package exceeds the 64 MiB bound");
    metadata.push({
      path,
      flags,
      method,
      checksum,
      compressedSize,
      expandedSize,
      localOffset,
    });
    offset = next;
  }
  if (offset !== endOffset)
    throw new Error("ZIP central directory contains trailing data");

  return new Map(
    metadata.map((entry) => [entry.path, readEntryContents(archive, entry)]),
  );
}
