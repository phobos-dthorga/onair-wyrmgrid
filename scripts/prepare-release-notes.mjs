import { readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  compareSupportedReleaseVersions,
  isSupportedReleaseVersion,
  parseSupportedReleaseVersion,
} from "./verify-release-version.mjs";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const defaultChangelogPath = resolve(scriptDirectory, "..", "CHANGELOG.md");

export const RELEASE_SECTIONS = [
  "New features",
  "Changes",
  "Removed",
  "🚨 Breaking changes",
];

function parseItems(sectionName, body) {
  const items = [];

  for (const line of body.split("\n")) {
    if (!line.trim()) continue;

    if (line.startsWith("- ")) {
      items.push(line.slice(2).trim());
      continue;
    }

    if (/^\s{2,}\S/.test(line) && items.length > 0) {
      items[items.length - 1] += ` ${line.trim()}`;
      continue;
    }

    throw new Error(
      `Changelog section '${sectionName}' must contain only Markdown list items`,
    );
  }

  if (items.length === 0) {
    throw new Error(
      `Changelog section '${sectionName}' must contain a list item or '- None.'`,
    );
  }

  const noneItems = items.filter((item) => item === "None.");
  if (noneItems.length > 0 && items.length !== 1) {
    throw new Error(
      `Changelog section '${sectionName}' cannot combine '- None.' with real entries`,
    );
  }

  return items;
}

function parseSections(entry) {
  const matches = [...entry.body.matchAll(/^### ([^\r\n]+)\r?$/gm)];
  const sections = new Map();

  for (let index = 0; index < matches.length; index += 1) {
    const heading = matches[index][1];
    if (sections.has(heading)) {
      throw new Error(
        `Changelog entry '${entry.version}' repeats section '${heading}'`,
      );
    }

    const start = matches[index].index + matches[index][0].length;
    const end = matches[index + 1]?.index ?? entry.body.length;
    sections.set(heading, {
      body: entry.body.slice(start, end).trim(),
      items: parseItems(heading, entry.body.slice(start, end).trim()),
    });
  }

  const actualSections = [...sections.keys()];
  if (
    actualSections.length !== RELEASE_SECTIONS.length ||
    actualSections.some((section, index) => section !== RELEASE_SECTIONS[index])
  ) {
    throw new Error(
      `Changelog entry '${entry.version}' must contain these sections in order: ${RELEASE_SECTIONS.join(", ")}`,
    );
  }

  return sections;
}

export function parseChangelog(changelog) {
  const normalized = changelog.replaceAll("\r\n", "\n");
  const matches = [
    ...normalized.matchAll(/^## \[([^\]]+)\](?: - (\d{4}-\d{2}-\d{2}))?\s*$/gm),
  ];

  if (matches.length === 0) {
    throw new Error("CHANGELOG.md contains no version entries");
  }

  const entries = matches.map((match, index) => {
    const start = match.index + match[0].length;
    let end = matches[index + 1]?.index ?? normalized.length;
    if (!matches[index + 1]) {
      const footer = /^\[[^\]]+\]:\s+\S+/m.exec(normalized.slice(start));
      if (footer) end = start + footer.index;
    }
    return {
      version: match[1],
      date: match[2],
      body: normalized.slice(start, end).trim(),
    };
  });

  if (entries[0].version !== "Unreleased") {
    throw new Error("CHANGELOG.md must begin with an [Unreleased] entry");
  }

  const versions = new Set();
  for (const entry of entries) {
    if (versions.has(entry.version)) {
      throw new Error(`CHANGELOG.md repeats version '${entry.version}'`);
    }
    versions.add(entry.version);

    if (entry.version === "Unreleased") {
      if (entry.date) {
        throw new Error("The [Unreleased] changelog entry cannot have a date");
      }
    } else {
      if (!isSupportedReleaseVersion(entry.version)) {
        throw new Error(
          `CHANGELOG.md contains unsupported version '${entry.version}'`,
        );
      }
      if (!entry.date) {
        throw new Error(
          `Released changelog entry '${entry.version}' must include a YYYY-MM-DD date`,
        );
      }
    }

    entry.sections = parseSections(entry);
  }

  return entries;
}

function nearestPreviousVersion(entries, currentVersion) {
  return entries
    .map((entry) => entry.version)
    .filter(
      (version) =>
        version !== "Unreleased" &&
        version !== currentVersion &&
        compareSupportedReleaseVersions(version, currentVersion) < 0,
    )
    .sort(compareSupportedReleaseVersions)
    .at(-1);
}

function isMajorReleaseLine(currentVersion, previousVersion) {
  if (!previousVersion) return true;

  const current = parseSupportedReleaseVersion(currentVersion);
  const previous = parseSupportedReleaseVersion(previousVersion);
  if (!current || !previous) return false;

  const [currentMajor, currentMinor, currentPatch] = current.core.map(Number);
  const [previousMajor, previousMinor, previousPatch] =
    previous.core.map(Number);
  const sameCore =
    currentMajor === previousMajor &&
    currentMinor === previousMinor &&
    currentPatch === previousPatch;

  if (sameCore && previous.prerelease.length > 0) return true;

  return (
    currentMajor > previousMajor && currentMinor === 0 && currentPatch === 0
  );
}

export function validateReleaseChangelog(
  changelog,
  currentVersion,
  previousVersion,
) {
  if (!isSupportedReleaseVersion(currentVersion)) {
    throw new Error(`Unsupported release version '${currentVersion}'`);
  }
  if (previousVersion && !isSupportedReleaseVersion(previousVersion)) {
    throw new Error(
      `Unsupported previous release version '${previousVersion}'`,
    );
  }
  if (
    previousVersion &&
    compareSupportedReleaseVersions(previousVersion, currentVersion) >= 0
  ) {
    throw new Error(
      `Previous release version '${previousVersion}' must be older than '${currentVersion}'`,
    );
  }

  const entries = parseChangelog(changelog);
  const entry = entries.find(
    (candidate) => candidate.version === currentVersion,
  );
  if (!entry) {
    throw new Error(
      `CHANGELOG.md has no release entry for version '${currentVersion}'`,
    );
  }

  const resolvedPreviousVersion =
    previousVersion ?? nearestPreviousVersion(entries, currentVersion);
  const breakingItems = entry.sections.get("🚨 Breaking changes").items;
  const hasBreakingChanges = breakingItems[0] !== "None.";

  if (
    hasBreakingChanges &&
    !isMajorReleaseLine(currentVersion, resolvedPreviousVersion)
  ) {
    throw new Error(
      `Version '${currentVersion}' declares breaking changes but is not a new X.0.0 major release after '${resolvedPreviousVersion}'`,
    );
  }

  return {
    entry,
    previousVersion: resolvedPreviousVersion,
    hasBreakingChanges,
  };
}

export function renderReleaseNotes(validation) {
  const { entry, hasBreakingChanges } = validation;
  const breakingBanner = hasBreakingChanges
    ? [
        "> [!CAUTION]",
        "> 🚨 **BREAKING CHANGES — MAJOR VERSION**",
        "> This release contains compatibility breaks. Read the breaking-change list and migration guidance before upgrading.",
      ].join("\n")
    : ["> [!NOTE]", "> **Breaking changes:** None."].join("\n");

  return [
    `# OnAir WyrmGrid v${entry.version}`,
    "",
    breakingBanner,
    "",
    entry.body,
    "",
    "---",
    "",
    "CI-built platform packages include SHA-256 checksums and GitHub build provenance. Verify installation, startup, licence notices, checksums, and offline behaviour before publishing this prerelease.",
    "",
  ].join("\n");
}

export async function prepareReleaseNotes({
  currentVersion,
  previousVersion,
  changelogPath = defaultChangelogPath,
  outputPath,
}) {
  const changelog = await readFile(changelogPath, "utf8");
  const validation = validateReleaseChangelog(
    changelog,
    currentVersion,
    previousVersion,
  );
  const notes = renderReleaseNotes(validation);

  if (outputPath) await writeFile(outputPath, notes, "utf8");
  return { ...validation, notes };
}

function parseArguments(args) {
  const [currentVersion, ...options] = args;
  if (!currentVersion) {
    throw new Error(
      "Usage: node scripts/prepare-release-notes.mjs <version> [--previous <version>] [--output <path>]",
    );
  }

  const parsed = { currentVersion };
  for (let index = 0; index < options.length; index += 2) {
    const option = options[index];
    const value = options[index + 1];
    if (!value || !["--previous", "--output"].includes(option)) {
      throw new Error(`Unsupported or incomplete option '${option ?? ""}'`);
    }
    if (option === "--previous") parsed.previousVersion = value;
    if (option === "--output") parsed.outputPath = resolve(value);
  }
  return parsed;
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : undefined;
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    const result = await prepareReleaseNotes(
      parseArguments(process.argv.slice(2)),
    );
    const destination = result.notes
      ? " and prepared GitHub release notes"
      : "";
    console.log(
      `Validated CHANGELOG.md for ${result.entry.version}${destination}.`,
    );
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
