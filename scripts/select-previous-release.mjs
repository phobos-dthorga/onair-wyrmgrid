import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { selectPreviousReleaseTag } from "./verify-release-version.mjs";

export function releaseTagsFromInput(input, format = "json") {
  if (format === "tag-lines") {
    return input
      .split(/\r?\n/)
      .map((tag) => tag.trim())
      .filter(Boolean);
  }

  if (format !== "json") {
    throw new Error(`Unsupported release-tag input format '${format}'`);
  }

  const releases = JSON.parse(input || "[]");
  return releases.map((release) => release.tagName);
}

export async function selectPreviousReleaseFromInput(
  currentVersion,
  format,
  stream,
) {
  let input = "";
  for await (const chunk of stream) input += chunk;
  const tags = releaseTagsFromInput(input, format);
  return selectPreviousReleaseTag(currentVersion, tags);
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : undefined;
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    const currentVersion = process.argv[2];
    if (!currentVersion) {
      throw new Error(
        "Usage: select-previous-release.mjs <current-version> [--tag-lines]",
      );
    }

    const format = process.argv[3] === "--tag-lines" ? "tag-lines" : "json";
    const previousTag = await selectPreviousReleaseFromInput(
      currentVersion,
      format,
      process.stdin,
    );
    if (previousTag) process.stdout.write(previousTag);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
