import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const [
  multibranchPipeline,
  releasePipeline,
  aiAgentPipeline,
  aiAgentWrapper,
  aiAgentRuntimeTests,
  aiAgentPolicy,
  packageMetadata,
  tauriConfig,
] = await Promise.all([
  readFile(resolve(repositoryRoot, "Jenkinsfile"), "utf8"),
  readFile(resolve(repositoryRoot, "Jenkinsfile.release"), "utf8"),
  readFile(resolve(repositoryRoot, "Jenkinsfile.ai-agent"), "utf8"),
  readFile(
    resolve(repositoryRoot, "scripts/ai-agent/run_opencode_phase.sh"),
    "utf8",
  ),
  readFile(
    resolve(repositoryRoot, "scripts/ai-agent-runtime.test.mjs"),
    "utf8",
  ),
  readFile(resolve(repositoryRoot, "ci/ai-agent-policy.yml"), "utf8").then(
    JSON.parse,
  ),
  readFile(resolve(repositoryRoot, "package.json"), "utf8").then(JSON.parse),
  readFile(
    resolve(repositoryRoot, "apps/desktop/src-tauri/tauri.conf.json"),
    "utf8",
  ).then(JSON.parse),
]);

test("manual AI Agent is bounded, repairable, and draft-only", () => {
  for (const mode of [
    "ASK",
    "DECISION_TRACE",
    "CONSISTENCY_AUDIT",
    "ROADMAP_STATUS",
    "PATCH",
    "FEATURE",
  ]) {
    assert.match(aiAgentPipeline, new RegExp(`'${mode}'`));
  }
  for (const parameter of [
    "SOURCE_REVISION",
    "REQUEST",
    "READ_SCOPE",
    "ALLOWED_PATHS",
    "MAX_CHANGED_FILES",
    "MAX_CHANGED_LINES",
    "TEST_PROFILE",
    "LOCAL_MODEL_PROFILE",
    "REASONING_EFFORT",
    "HOSTED_REVIEW",
  ]) {
    assert.match(aiAgentPipeline, new RegExp(`name: '${parameter}'`));
  }
  assert.match(aiAgentPipeline, /agent \{ label 'ai-agent' \}/);
  assert.match(aiAgentPipeline, /disableConcurrentBuilds\(\)/);
  assert.match(aiAgentPipeline, /buildDiscarder\(logRotator\(/);
  assert.match(aiAgentPipeline, /wyrmgrid-ai-agent-gateway/);
  assert.match(aiAgentPipeline, /params\.REASONING_EFFORT \?: 'LOW'/);
  assert.match(aiAgentPipeline, /hoardmind-jenkins-ai-contributor/);
  assert.match(aiAgentPipeline, /Codex-Jenkins-Tauryk-Gk-Io/);
  assert.match(aiAgentPipeline, /phobos-dthorga\/onair-wyrmgrid/);
  assert.match(aiAgentPipeline, /validate-toolchain/);
  assert.match(aiAgentPipeline, /prepareLocalPhase\('PLANNER', 1\)/);
  assert.match(aiAgentPipeline, /prepareLocalPhase\('BUILDER', 2\)/);
  assert.match(aiAgentPipeline, /prepareLocalPhase\(\s*'REPAIR'/);
  assert.match(aiAgentPipeline, /prepareLocalPhase\('REVIEW', 5\)/);
  assert.match(aiAgentPipeline, /format-changes/);
  assert.match(aiAgentPipeline, /\.jenkins-ai-opencode\/\$\{phaseSlug\}/);
  assert.match(
    aiAgentPipeline,
    /commandOverride: '''bash "\$WORKSPACE\/\.jenkins-ai-runtime\/run_opencode_phase\.sh"'''/,
  );
  assert.doesNotMatch(
    aiAgentPipeline.match(/commandOverride:[^\n]+/)?.[0] ?? "",
    /AI_AGENT_PROMPT/,
  );
  assert.match(
    aiAgentPipeline,
    /node --test scripts\/ai-agent-runtime\.test\.mjs/,
  );
  assert.match(aiAgentPipeline, /--repository \. \\\s+--policy/);
  assert.match(
    aiAgentWrapper,
    /prompt="\$\(cat -- "\$AI_AGENT_PROMPT_FILE"\)"/,
  );
  assert.match(aiAgentWrapper, /"\$prompt" \|/);
  assert.doesNotMatch(aiAgentWrapper, /eval|AI_AGENT_PROMPT[^_]/);
  assert.match(
    aiAgentRuntimeTests,
    /process\.platform === "win32" \? "python" : "python3"/,
  );
  assert.match(
    aiAgentPipeline,
    /def runLocalPhase[\s\S]*failOnAgentError: false/,
  );
  assert.match(
    aiAgentPipeline,
    /while \(testStatus == 1 && repairs < repairLimit\)/,
  );
  assert.equal(
    [
      ...aiAgentPipeline.matchAll(
        /while \(testStatus == 1 && repairs < repairLimit\)/g,
      ),
    ].length,
    1,
  );
  assert.match(aiAgentPipeline, /local-repair-limit\.txt/);
  assert.match(aiAgentPipeline, /timeout\(time: 24, unit: 'HOURS'\)/);
  assert.match(aiAgentPipeline, /'ARCHIVE_ONLY'/);
  assert.match(aiAgentPipeline, /'OPEN_FAILING_DRAFT_PR'/);
  assert.match(aiAgentPipeline, /gh pr create/);
  assert.match(aiAgentPipeline, /--draft/);
  assert.doesNotMatch(
    aiAgentPipeline,
    /gh pr (?:merge|approve)|gh release|--admin|--auto|mark-ready/,
  );
  assert.doesNotMatch(aiAgentPipeline, /forgeAI\(/);
  assert.equal(aiAgentPolicy.repository, "phobos-dthorga/onair-wyrmgrid");
  assert.equal(aiAgentPolicy.context_limits.active_profile, "SMALL_FILES");
  assert.equal(aiAgentPolicy.toolchain.opencode_version, "1.18.5");
  assert.equal(aiAgentPolicy.toolchain.codex_cli_version, "0.145.0");
  assert.ok(aiAgentPolicy.toolchain.required_commands.includes("bash"));
  assert.ok(aiAgentPolicy.toolchain.required_commands.includes("install"));
  assert.ok(aiAgentPolicy.toolchain.required_commands.includes("tee"));
  assert.deepEqual(aiAgentPolicy.test_profiles.DOCUMENTATION.commands, [
    ["npm", "run", "format:frontend:check"],
  ]);
  assert.equal(aiAgentPolicy.job.local_test_repair_attempts, 2);
  assert.equal(aiAgentPolicy.job.hosted_repair_attempts, 1);
  assert.equal(aiAgentPolicy.phase_routing.BUILDER.model, "qwen3-coder:30b");
  assert.equal(aiAgentPolicy.phase_routing.BUILDER.reasoning, "NONE");
  assert.equal(aiAgentPolicy.phase_routing.REPAIR.reasoning, "NONE");
  assert.equal(aiAgentPolicy.phase_routing.PLANNER.model, "qwen3.6:35b");
  assert.equal(aiAgentPolicy.phase_routing.REVIEW.model, "qwen3.6:35b");
  assert.equal(
    aiAgentPolicy.model_profiles.REPOSITORY_SCHOLAR_LOCAL.context_tokens,
    12288,
  );
  assert.equal(aiAgentPolicy.job.phase_limits.maximum_checkpoint_bytes, 24576);
  assert.equal(aiAgentPolicy.job.opencode_compaction.tail_turns, 1);
  assert.deepEqual(aiAgentPolicy.job.local_reasoning_efforts, [
    "LOW",
    "MEDIUM",
    "HIGH",
  ]);
  assert.equal(aiAgentPolicy.job.answer_word_limits.ASK, 200);
  assert.equal(aiAgentPolicy.job.answer_word_limits.FEATURE, 400);
});

test("multibranch pipeline runs the complete credential-free Linux and Windows gates", () => {
  assert.match(multibranchPipeline, /agent \{ label 'linux' \}/);
  assert.match(multibranchPipeline, /agent \{ label 'windows' \}/);
  assert.match(multibranchPipeline, /npm run ci:frontend/);
  assert.match(multibranchPipeline, /npm run ci:python/);
  assert.match(multibranchPipeline, /npm run ci:rust/);
  assert.match(multibranchPipeline, /npm run ci:dependencies/);
  assert.match(multibranchPipeline, /npm run ci:prepare/);
  assert.doesNotMatch(multibranchPipeline, /timeout\(time: 2,/);
  assert.equal(
    [...multibranchPipeline.matchAll(/timeout\(time: 3, unit: 'HOURS'\)/g)]
      .length,
    4,
  );
  assert.match(multibranchPipeline, /timeout\(time: 7, unit: 'HOURS'\)/);
  assert.match(
    multibranchPipeline,
    /disableConcurrentBuilds\(abortPrevious: true\)/,
  );
  assert.match(multibranchPipeline, /preserveStashes\(buildCount: 5\)/);
  assert.doesNotMatch(multibranchPipeline, /\bparallel\s*\{/);
  for (const stage of [
    "Linux validation",
    "Windows validation",
    "Linux snapshot",
    "Windows snapshot",
  ]) {
    assert.match(
      multibranchPipeline,
      new RegExp(`^ {8}stage\\('${stage}'\\)`, "m"),
    );
  }

  assert.doesNotMatch(
    multibranchPipeline,
    /withCredentials|credentialsId|GH_TOKEN|gh release|SENTRY_AUTH_TOKEN/,
  );
});

test("snapshot packages are limited to main and release branches", () => {
  assert.match(multibranchPipeline, /env\.BRANCH_NAME == 'main'/);
  assert.match(
    multibranchPipeline,
    /env\.BRANCH_NAME ==~ \/\^codex\\\/release-\.\+\//,
  );
  assert.match(multibranchPipeline, /expression \{ isSnapshotBranch\(\) \}/);
  assert.match(multibranchPipeline, /--bundles appimage,deb/);
  assert.match(multibranchPipeline, /--bundles nsis/);
  assert.match(multibranchPipeline, /test-nsis-installer\.ps1/);
  assert.match(multibranchPipeline, /BUILD-INFO\.json/);
  assert.match(multibranchPipeline, /SHA256SUMS\.txt/);
  assert.doesNotMatch(multibranchPipeline, /--bundles [^\n]*(?:dmg|app,)/i);
  assert.match(
    multibranchPipeline,
    /stage\('Linux snapshot'\)[\s\S]*sh '''#!\/usr\/bin\/env bash\s+set -euo pipefail/,
  );
  assert.doesNotMatch(
    multibranchPipeline,
    /stage\('Linux snapshot'\)[\s\S]*sh '''\s+set -euo pipefail/,
  );
  assert.equal(
    [...multibranchPipeline.matchAll(/^NODE$/gm)].length,
    1,
    "the Linux metadata heredoc must close at the shell's first column",
  );
});

test("release pipelines preserve restart inputs and expose each platform as a restart stage", () => {
  assert.match(releasePipeline, /preserveStashes\(buildCount: 5\)/);
  assert.doesNotMatch(releasePipeline, /\bparallel\s*\{/);
  assert.match(releasePipeline, /^ {8}stage\('Linux release package'\)/m);
  assert.match(releasePipeline, /^ {8}stage\('Windows release package'\)/m);
  assert.match(
    releasePipeline,
    /stage\('Linux release package'\)[\s\S]*unstash 'release-policy'[\s\S]*RELEASE_COMMIT="\$\(cat \.jenkins\/release-policy\/commit\)"/,
  );
  assert.match(
    releasePipeline,
    /stage\('Windows release package'\)[\s\S]*unstash 'release-policy'[\s\S]*Get-Content -Raw '\.jenkins\\release-policy\\commit'/,
  );
  assert.match(
    releasePipeline,
    /stage\('Assemble release'\)[\s\S]*export RELEASE_VERSION="\$\(cat \.jenkins\/release-policy\/version\)"[\s\S]*export RELEASE_COMMIT="\$\(cat \.jenkins\/release-policy\/commit\)"/,
  );
  assert.doesNotMatch(releasePipeline, /\$\{env\.RELEASE_(?:COMMIT|VERSION)\}/);
  assert.doesNotMatch(releasePipeline, /sh '''\s+set -euo pipefail/);
  assert.equal(
    [...releasePipeline.matchAll(/^NODE$/gm)].length,
    1,
    "the release metadata heredoc must close at the shell's first column",
  );
});

test("ForgeAI is a feature-first advisory review for PR merges and main", () => {
  const snapshots = multibranchPipeline.indexOf("stage('Unsigned snapshots')");
  const forgeAi = multibranchPipeline.indexOf(
    "stage('ForgeAI advisory review')",
  );
  const forgeAiStage = multibranchPipeline.slice(forgeAi);

  assert.notEqual(forgeAi, -1);
  assert.ok(snapshots < forgeAi);
  assert.match(
    multibranchPipeline,
    /def isForgeAIReviewBranch\(\)[\s\S]*env\.BRANCH_NAME == 'main'/,
  );
  assert.match(multibranchPipeline, /env\.CHANGE_ID/);
  assert.match(multibranchPipeline, /!env\.CHANGE_FORK/);
  assert.doesNotMatch(
    multibranchPipeline,
    /env\.BRANCH_NAME\.endsWith\('-merge'\)/,
  );
  assert.match(forgeAiStage, /node\('linux'\)/);
  assert.match(forgeAiStage, /timeout\(time: 45, unit: 'MINUTES'\)/);
  assert.match(
    forgeAiStage,
    /buildResult: 'SUCCESS',[\s\S]*stageResult: 'UNSTABLE'/,
  );
  assert.match(forgeAiStage, /prepare-forgeai-review\.mjs/);
  assert.match(
    forgeAiStage,
    /sourceGlob: '\.jenkins\/forgeai-input\/change-review\.txt'/,
  );
  assert.match(forgeAiStage, /failOnCritical: false/);
  assert.match(forgeAiStage, /def expectedAnalyzerCount = 7/);
  assert.match(forgeAiStage, /report\.analyzerCount != expectedAnalyzerCount/);
  assert.match(forgeAiStage, /find forgeai-reports -type f -print -quit/);
  assert.match(
    forgeAiStage,
    /ForgeAI completed every analyzer but produced no report artifact\./,
  );
  assert.match(forgeAiStage, /grep -R -F -q/);
  assert.match(forgeAiStage, /'JSON parsing failed'/);
  assert.match(
    forgeAiStage,
    /ForgeAI returned malformed structured output for one or more analyzers\./,
  );
  assert.match(forgeAiStage, /allowEmptyArchive: !completeForgeAIReport/);

  const analyzerOrder = [
    "'code-review'",
    "'architecture-drift'",
    "'test-gaps'",
    "'commit-intel'",
    "'pipeline-advisor'",
    "'vulnerability'",
    "'dependency-risk'",
  ].map((analyzer) => forgeAiStage.indexOf(analyzer));
  assert.ok(analyzerOrder.every((index) => index >= 0));
  assert.deepEqual(
    analyzerOrder,
    [...analyzerOrder].sort((a, b) => a - b),
  );
  assert.doesNotMatch(forgeAiStage, /'release-readiness'/);
  assert.match(forgeAiStage, /archiveArtifacts\(/);
  assert.match(forgeAiStage, /artifacts: 'forgeai-reports\/\*\*'/);
});

test("trusted release pipeline validates exact tags before credential use", () => {
  const tagValidation = releasePipeline.indexOf(
    "RELEASE_TAG must be a supported vX.Y.Z or prerelease tag.",
  );
  const versionValidation = releasePipeline.indexOf(
    "node scripts/verify-release-version.mjs",
  );
  const ancestryValidation = releasePipeline.indexOf(
    "git merge-base --is-ancestor",
  );
  const credentialBinding = releasePipeline.indexOf("withCredentials");

  assert.notEqual(tagValidation, -1);
  assert.notEqual(versionValidation, -1);
  assert.notEqual(ancestryValidation, -1);
  assert.notEqual(credentialBinding, -1);
  assert.ok(tagValidation < credentialBinding);
  assert.ok(versionValidation < credentialBinding);
  assert.ok(ancestryValidation < credentialBinding);
  assert.match(releasePipeline, /EXCEPTION_REASON.*20/);
  assert.match(
    releasePipeline,
    /Published release \$RELEASE_TAG cannot be replaced/,
  );
  assert.match(releasePipeline, /release-query\.err/);
  assert.match(releasePipeline, /elif ! grep -q 'HTTP 404'/);
  assert.match(releasePipeline, /credentialsId: 'wyrmgrid-github-release'/);
});

test("release publication remains an approved draft prerelease", () => {
  assert.equal([...releasePipeline.matchAll(/npm run ci:prepare/g)].length, 2);
  assert.match(releasePipeline, /stage\('Approve draft publication'\)/);
  assert.match(releasePipeline, /input\(/);
  assert.match(releasePipeline, /gh release create/);
  assert.match(releasePipeline, /gh release edit/);
  assert.match(releasePipeline, /--draft/);
  assert.match(releasePipeline, /--prerelease/);
  assert.match(releasePipeline, /--paginate --slurp/);
  assert.match(releasePipeline, /asset\.name\.endsWith\('-setup\.exe'\)/);
  assert.match(releasePipeline, /SHA256SUMS\.txt/);
  assert.match(releasePipeline, /provenance: 'checksums-only'/);
  assert.match(
    releasePipeline,
    /platforms: \['linux_x86_64', 'windows_x86_64'\]/,
  );
  assert.doesNotMatch(releasePipeline, /--bundles [^\n]*(?:dmg|app,)/i);
  assert.doesNotMatch(
    releasePipeline,
    /hoardmind|ollama|openai-compatible|optional-ai|api\/chat|chat\/completions|model api/i,
  );
});

test("application packaging supports Windows and Linux without macOS bundles", () => {
  assert.deepEqual(tauriConfig.bundle.targets, ["nsis", "appimage", "deb"]);
  assert.equal(tauriConfig.bundle.windows.nsis.installMode, "currentUser");
  assert.equal(tauriConfig.productName, "OnAir WyrmGrid");
  assert.equal(tauriConfig.identifier, "io.github.phobosdthorga.onairwyrmgrid");
});

test("package scripts are the shared CI command contract", () => {
  assert.equal(
    packageMetadata.scripts["ci:python"],
    "python -m unittest discover -s sdk/python/tests -v && python -m unittest discover -s plugins/tests -v",
  );
  assert.match(packageMetadata.scripts["ci:frontend"], /test:tooling/);
  assert.match(packageMetadata.scripts["ci:frontend"], /audit:boundaries/);
  assert.equal(
    packageMetadata.scripts["ci:prepare"],
    "npm run provider:prepare && npm run audio-codec:prepare",
  );
  assert.equal(
    packageMetadata.scripts["audio-codec:prepare"],
    "node scripts/prepare-audio-codec.mjs --release",
  );
  assert.match(packageMetadata.scripts["ci:rust"], /--locked/);
  assert.match(packageMetadata.scripts["ci:rust"], /-D warnings/);
  assert.match(
    packageMetadata.scripts["ci:rust"],
    /cargo test --locked --workspace -- --test-threads=1$/,
  );
  assert.match(packageMetadata.scripts["ci:dependencies"], /cargo deny check/);
  assert.match(
    packageMetadata.scripts["ci:dependencies"],
    /npm audit --audit-level=high/,
  );
});
