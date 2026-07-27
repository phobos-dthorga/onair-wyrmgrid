import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

test("WyrmGrid AI Agent Python contracts pass", () => {
  const result = spawnSync(
    "python",
    ["-m", "unittest", "scripts/ai-agent/test_wyrmgrid_ai_agent.py", "-v"],
    {
      cwd: repositoryRoot,
      encoding: "utf8",
      windowsHide: true,
    },
  );
  assert.equal(result.status, 0, result.stderr || result.stdout);
});

test("WyrmGrid AI Agent policy starts with a bounded visible context", async () => {
  const policy = JSON.parse(
    await readFile(resolve(repositoryRoot, "ci/ai-agent-policy.yml"), "utf8"),
  );
  assert.equal(policy.context_limits.active_profile, "SMALL_FILES");
  const active =
    policy.context_limits.profiles[policy.context_limits.active_profile];
  assert.equal(active.maximum_visible_file_bytes, 32 * 1024);
  assert.equal(active.maximum_visible_file_lines, 800);
  assert.equal(active.maximum_visible_total_bytes, 512 * 1024);
  assert.deepEqual(Object.keys(policy.context_limits.profiles), [
    "SMALL_FILES",
    "MEDIUM_FILES",
    "LARGE_FILES_RESEARCH",
  ]);
  assert.deepEqual(policy.context_limits.promotion_order, [
    "SMALL_FILES",
    "MEDIUM_FILES",
    "LARGE_FILES_RESEARCH",
  ]);
  assert.equal(policy.toolchain.opencode_version, "1.18.5");
  assert.equal(policy.toolchain.codex_cli_version, "0.145.0");
  assert.equal(
    policy.model_profiles.SCOPED_BUILDER_LOCAL.selected_model,
    "qwen3-coder:30b",
  );
});
