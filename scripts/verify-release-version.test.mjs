import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  isSupportedReleaseVersion,
  verifyReleaseVersion,
} from "./verify-release-version.mjs";

async function releaseFixture(versions) {
  const root = await mkdtemp(join(tmpdir(), "wyrmgrid-release-version-"));
  await mkdir(join(root, "apps/desktop/src-tauri"), { recursive: true });
  await writeFile(
    join(root, "Cargo.toml"),
    `[workspace]\nmembers = []\n\n[workspace.package]\nversion = "${versions.cargo}"\n`,
  );
  await writeFile(
    join(root, "package.json"),
    JSON.stringify({ version: versions.root }),
  );
  await writeFile(
    join(root, "apps/desktop/package.json"),
    JSON.stringify({ version: versions.desktop }),
  );
  await writeFile(
    join(root, "apps/desktop/src-tauri/tauri.conf.json"),
    JSON.stringify({ version: versions.tauri }),
  );
  return root;
}

test("accepts stable and prerelease semantic versions", () => {
  assert.equal(isSupportedReleaseVersion("0.1.0"), true);
  assert.equal(isSupportedReleaseVersion("2.4.1-rc.2"), true);
  assert.equal(isSupportedReleaseVersion("01.1.0"), false);
  assert.equal(isSupportedReleaseVersion("1.0.0-rc.02"), false);
  assert.equal(isSupportedReleaseVersion("1.0.0+local"), false);
});

test("accepts a release only when every application version matches", async (context) => {
  const root = await releaseFixture({
    cargo: "0.2.1",
    root: "0.2.1",
    desktop: "0.2.1",
    tauri: "0.2.1",
  });
  context.after(() => rm(root, { recursive: true, force: true }));

  const versions = await verifyReleaseVersion("0.2.1", root);
  assert.equal(versions.size, 4);
});

test("reports every checked-in version that disagrees with the tag", async (context) => {
  const root = await releaseFixture({
    cargo: "0.2.1",
    root: "0.2.0",
    desktop: "0.2.1",
    tauri: "0.1.0",
  });
  context.after(() => rm(root, { recursive: true, force: true }));

  await assert.rejects(
    verifyReleaseVersion("0.2.1", root),
    /root npm package: 0\.2\.0[\s\S]*Tauri application: 0\.1\.0/,
  );
});
