import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const pythonExecutable = process.platform === "win32" ? "python" : "python3";

test("WyrmGrid AI Agent Python contracts pass", () => {
  const result = spawnSync(
    pythonExecutable,
    ["-m", "unittest", "scripts/ai-agent/test_wyrmgrid_ai_agent.py", "-v"],
    {
      cwd: repositoryRoot,
      encoding: "utf8",
      windowsHide: true,
    },
  );
  assert.ifError(result.error);
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
  assert.ok(policy.toolchain.required_commands.includes("bash"));
  assert.ok(policy.toolchain.required_commands.includes("install"));
  assert.ok(policy.toolchain.required_commands.includes("tee"));
  assert.deepEqual(policy.test_profiles.DOCUMENTATION.commands, [
    ["npm", "run", "format:frontend:check"],
  ]);
  assert.equal(policy.publication.repository, "phobos-dthorga/onair-wyrmgrid");
  assert.equal(policy.publication.branch_namespace, "jenkins-ai-agent");
  assert.deepEqual(policy.publication.required_github_app_permissions, {
    metadata: "read",
    contents: "write",
    pull_requests: "write",
    workflows: "write",
  });
  assert.equal(
    policy.model_profiles.SCOPED_BUILDER_LOCAL.selected_model,
    "qwen3-coder:30b",
  );
  assert.ok(
    policy.local_model_inventory["qwen3.6:35b"].capabilities.includes(
      "thinking",
    ),
  );
  assert.ok(
    !policy.local_model_inventory["qwen3-coder:30b"].capabilities.includes(
      "thinking",
    ),
  );
  assert.deepEqual(policy.job.local_reasoning_efforts, [
    "LOW",
    "MEDIUM",
    "HIGH",
  ]);
  assert.deepEqual(policy.job.answer_word_limits, {
    ASK: 200,
    DECISION_TRACE: 500,
    CONSISTENCY_AUDIT: 650,
    ROADMAP_STATUS: 500,
    PATCH: 250,
    FEATURE: 400,
  });
  assert.equal(policy.phase_routing.PLANNER.model, "qwen3.6:35b");
  assert.equal(policy.phase_routing.BUILDER.model, "qwen3-coder:30b");
  assert.equal(policy.phase_routing.REPAIR.reasoning, "NONE");
  assert.equal(policy.phase_routing.REVIEW.reasoning, "PARAMETER");
  assert.equal(policy.job.opencode_compaction.tail_turns, 1);
  assert.equal(policy.job.opencode_compaction.preserve_recent_tokens, 2000);
  assert.equal(policy.job.opencode_compaction.reserved, 4096);
});
