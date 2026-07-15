import { selectPreviousReleaseTag } from "./verify-release-version.mjs";

try {
  const currentVersion = process.argv[2];
  if (!currentVersion) {
    throw new Error("Usage: select-previous-release.mjs <current-version>");
  }

  let input = "";
  for await (const chunk of process.stdin) input += chunk;
  const releases = JSON.parse(input || "[]");
  const tags = releases.map((release) => release.tagName);
  const previousTag = selectPreviousReleaseTag(currentVersion, tags);
  if (previousTag) process.stdout.write(previousTag);
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
