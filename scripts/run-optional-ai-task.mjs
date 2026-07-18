import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const PROFILE_SCHEMA_VERSION = 1;
export const METRICS_SCHEMA_VERSION = 1;
export const SUPPORTED_PROVIDERS = new Set([
  "ollama-chat",
  "openai-compatible-chat",
]);

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const optionalAiDocsRoot = resolve(repositoryRoot, "docs", "optional-ai");

export const OPTIONAL_AI_TASKS = Object.freeze({
  "release-curation-v1": Object.freeze({
    id: "release-curation-v1",
    display_name: "Release curation",
    prompt_path: resolve(
      optionalAiDocsRoot,
      "tasks",
      "release-curation-v1-system-prompt.md",
    ),
    template_path: resolve(
      optionalAiDocsRoot,
      "templates",
      "release-curation-v1.md",
    ),
    required_packet_headings: [
      "Outcome",
      "Release boundary",
      "Commit subjects",
      "File-level change summary",
      "Compatibility decisions",
      "Current Unreleased entry",
    ],
    required_output_headings: [
      "Release boundary",
      "Omissions and uncertainty",
      "New features",
      "Changes",
      "Removed",
      "🚨 Breaking changes",
      "Verification required",
    ],
  }),
  "change-impact-v1": Object.freeze({
    id: "change-impact-v1",
    display_name: "Change-impact dossier",
    prompt_path: resolve(
      optionalAiDocsRoot,
      "tasks",
      "change-impact-v1-system-prompt.md",
    ),
    template_path: resolve(
      optionalAiDocsRoot,
      "templates",
      "change-impact-v1.md",
    ),
    required_packet_headings: [
      "Task boundary",
      "Intended change",
      "Diff summary",
      "Selected diff evidence",
      "Tests and validation",
      "Known compatibility decisions",
      "Current documentation and changelog",
      "Exclusions",
    ],
    required_output_headings: [
      "Interpreted scope",
      "User and developer impact",
      "Affected components",
      "Test implications",
      "Documentation implications",
      "Compatibility flags for review",
      "Changelog candidates",
      "Uncertainty and verification",
    ],
  }),
  "test-matrix-v1": Object.freeze({
    id: "test-matrix-v1",
    display_name: "Test-matrix drafting",
    prompt_path: resolve(
      optionalAiDocsRoot,
      "tasks",
      "test-matrix-v1-system-prompt.md",
    ),
    template_path: resolve(
      optionalAiDocsRoot,
      "templates",
      "test-matrix-v1.md",
    ),
    required_packet_headings: [
      "Task boundary",
      "Approved behaviour",
      "Target layer and public surface",
      "Existing test conventions",
      "Selected production contract",
      "Existing fixtures and helpers",
      "Required case categories",
      "Validation command",
      "Exclusions",
    ],
    required_output_headings: [
      "Approved behaviour interpreted",
      "Test matrix",
      "Fixtures and helpers",
      "Execution and assertions",
      "Gaps and non-delegable decisions",
    ],
  }),
  "docs-sync-v1": Object.freeze({
    id: "docs-sync-v1",
    display_name: "Documentation synchronization",
    prompt_path: resolve(
      optionalAiDocsRoot,
      "tasks",
      "docs-sync-v1-system-prompt.md",
    ),
    template_path: resolve(optionalAiDocsRoot, "templates", "docs-sync-v1.md"),
    required_packet_headings: [
      "Task boundary",
      "Confirmed change facts",
      "Documentation inventory",
      "Selected current text",
      "Terminology and version markers",
      "Protected boundaries",
      "Existing validation evidence",
      "Exclusions",
    ],
    required_output_headings: [
      "Change facts interpreted",
      "Documents requiring review",
      "Proposed documentation edits",
      "Cross-document consistency",
      "Verification required",
    ],
  }),
  "fixture-variants-v1": Object.freeze({
    id: "fixture-variants-v1",
    display_name: "Synthetic fixture variants",
    prompt_path: resolve(
      optionalAiDocsRoot,
      "tasks",
      "fixture-variants-v1-system-prompt.md",
    ),
    template_path: resolve(
      optionalAiDocsRoot,
      "templates",
      "fixture-variants-v1.md",
    ),
    required_packet_headings: [
      "Task boundary",
      "Approved schema contract",
      "Sanitized base fixture",
      "Required variant categories",
      "Validator and stable errors",
      "Size and range boundaries",
      "Sanitization constraints",
      "Exclusions",
    ],
    required_output_headings: [
      "Contract interpreted",
      "Valid synthetic variants",
      "Invalid synthetic variants",
      "Validation matrix",
      "Sanitization review",
      "Uncertainty and verification",
    ],
  }),
  "implementation-patch-v1": Object.freeze({
    id: "implementation-patch-v1",
    display_name: "Bounded implementation patch",
    prompt_path: resolve(
      optionalAiDocsRoot,
      "tasks",
      "implementation-patch-v1-system-prompt.md",
    ),
    template_path: resolve(
      optionalAiDocsRoot,
      "templates",
      "implementation-patch-v1.md",
    ),
    required_packet_headings: [
      "Task boundary",
      "Approved behaviour",
      "Allowed paths",
      "Selected source evidence",
      "Existing test conventions",
      "Required validation",
      "Compatibility decisions",
      "Exclusions",
    ],
    required_output_headings: [
      "Scope interpreted",
      "Proposed patch",
      "Validation plan",
      "Risks and uncertainty",
    ],
  }),
  "failure-triage-v1": Object.freeze({
    id: "failure-triage-v1",
    display_name: "Sanitized failure triage",
    prompt_path: resolve(
      optionalAiDocsRoot,
      "tasks",
      "failure-triage-v1-system-prompt.md",
    ),
    template_path: resolve(
      optionalAiDocsRoot,
      "templates",
      "failure-triage-v1.md",
    ),
    required_packet_headings: [
      "Task boundary",
      "Command and environment",
      "Exit status",
      "Sanitized failure output",
      "Recent relevant changes",
      "Already attempted",
      "Security and privacy exclusions",
    ],
    required_output_headings: [
      "Execution boundary",
      "Failure clusters",
      "Likely causes",
      "Recommended local checks",
      "Escalation boundaries",
      "Uncertainty and missing evidence",
    ],
  }),
});

const MAX_HANDOFF_BYTES = 64 * 1024;
const REQUIRED_SYSTEM_PROMPT_PATTERNS = [
  /review-only/i,
  /untrusted (?:data|evidence)/i,
  /(?:does not|do not|never)[^\n]{0,80}authori[sz]e/i,
  /(?:not|never)[^\n]{0,40}proof/i,
];
const SENSITIVE_PATTERNS = [
  /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/,
  /\bgh[pousr]_[A-Za-z0-9]{20,}\b/,
  /\bsk-(?:proj-)?[A-Za-z0-9_-]{20,}\b/,
  /\bBearer\s+[A-Za-z0-9._~+/=-]{16,}\b/i,
  /\b(?:api[_-]?key|password|secret|token)\s*[:=]\s*["']?[^\s"']{12,}/i,
];

export function getOptionalAiTask(taskId) {
  const task = OPTIONAL_AI_TASKS[taskId];
  if (!task) {
    throw new Error(
      `Unsupported optional AI task '${taskId}'. Supported tasks: ${Object.keys(OPTIONAL_AI_TASKS).join(", ")}`,
    );
  }
  return task;
}

function levelTwoHeadings(markdown) {
  return [...markdown.matchAll(/^##[ \t]+(.+?)[ \t]*$/gm)].map(
    (match) => match[1],
  );
}

export function assertMarkdownContract({ document, requiredHeadings, label }) {
  const headings = levelTwoHeadings(document);
  let previousIndex = -1;
  for (const requiredHeading of requiredHeadings) {
    const matchingIndexes = headings.flatMap((heading, index) =>
      heading === requiredHeading ? [index] : [],
    );
    if (matchingIndexes.length !== 1) {
      throw new Error(
        `${label} must contain exactly one '## ${requiredHeading}' heading`,
      );
    }
    if (matchingIndexes[0] <= previousIndex) {
      throw new Error(`${label} headings are not in the required order`);
    }
    previousIndex = matchingIndexes[0];
  }
}

function finiteNumber(value, field, source = "AI provider") {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
    throw new Error(
      `${source} response field '${field}' is missing or invalid`,
    );
  }
  return value;
}

function optionalFiniteNumber(value) {
  return typeof value === "number" && Number.isFinite(value) && value >= 0
    ? value
    : undefined;
}

function nanosecondsToMilliseconds(nanoseconds) {
  return Math.round((nanoseconds / 1_000_000) * 100) / 100;
}

function tokensPerSecond(tokens, nanoseconds) {
  if (tokens === 0 || nanoseconds === 0) return 0;
  return Math.round((tokens / (nanoseconds / 1_000_000_000)) * 100) / 100;
}

function tokensPerMillisecond(tokens, milliseconds) {
  if (milliseconds === undefined) return null;
  if (tokens === 0 || milliseconds === 0) return 0;
  return Math.round((tokens / (milliseconds / 1_000)) * 100) / 100;
}

function maximum(samples, field) {
  return samples.reduce(
    (largest, sample) => Math.max(largest, sample[field] ?? 0),
    0,
  );
}

function boundedNumber(value, field, minimum, maximum) {
  if (
    typeof value !== "number" ||
    !Number.isFinite(value) ||
    value < minimum ||
    value > maximum
  ) {
    throw new Error(
      `Optional AI profile field '${field}' must be between ${minimum} and ${maximum}`,
    );
  }
  return value;
}

export function validateLocalAiProfile(profile, profilePath) {
  if (profile?.schema_version !== PROFILE_SCHEMA_VERSION) {
    throw new Error(
      `Optional AI profile must use schema version ${PROFILE_SCHEMA_VERSION}`,
    );
  }
  if (
    typeof profile.id !== "string" ||
    !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(profile.id)
  ) {
    throw new Error("Optional AI profile id must be a lowercase kebab-case id");
  }
  if (
    typeof profile.display_name !== "string" ||
    !profile.display_name.trim() ||
    profile.display_name.length > 80
  ) {
    throw new Error(
      "Optional AI profile display_name must contain 1-80 characters",
    );
  }
  if (!SUPPORTED_PROVIDERS.has(profile.provider)) {
    throw new Error(
      "Optional AI profile provider must be 'ollama-chat' or 'openai-compatible-chat' in schema version 1",
    );
  }

  let endpoint;
  try {
    endpoint = new URL(profile.endpoint);
  } catch (error) {
    throw new Error("Optional AI profile endpoint must be an absolute URL", {
      cause: error,
    });
  }
  const loopbackHosts = new Set(["127.0.0.1", "localhost", "[::1]"]);
  if (
    endpoint.protocol !== "http:" ||
    !loopbackHosts.has(endpoint.hostname) ||
    endpoint.username ||
    endpoint.password ||
    (endpoint.pathname !== "/" && endpoint.pathname !== "") ||
    endpoint.search ||
    endpoint.hash
  ) {
    throw new Error(
      "Optional AI profile endpoint must be an unauthenticated loopback HTTP origin",
    );
  }
  if (
    typeof profile.model !== "string" ||
    !profile.model.trim() ||
    profile.model.length > 200
  ) {
    throw new Error("Optional AI profile model must contain 1-200 characters");
  }
  if (
    typeof profile.system_prompt_file !== "string" ||
    !profile.system_prompt_file.trim()
  ) {
    throw new Error(
      "Optional AI profile system_prompt_file must identify a local file",
    );
  }

  const parameters = profile.parameters ?? {};
  const temperature = boundedNumber(
    parameters.temperature,
    "parameters.temperature",
    0,
    2,
  );
  const seed = boundedNumber(
    parameters.seed,
    "parameters.seed",
    0,
    2_147_483_647,
  );
  if (!Number.isInteger(seed)) {
    throw new Error("Optional AI profile parameters.seed must be an integer");
  }
  const maxOutputTokens = boundedNumber(
    parameters.max_output_tokens,
    "parameters.max_output_tokens",
    1,
    8_192,
  );
  if (!Number.isInteger(maxOutputTokens)) {
    throw new Error("Optional AI max_output_tokens must be an integer");
  }

  let contextLength = null;
  let think = null;
  if (profile.provider === "ollama-chat") {
    contextLength = boundedNumber(
      parameters.context_length,
      "parameters.context_length",
      1_024,
      131_072,
    );
    if (!Number.isInteger(contextLength)) {
      throw new Error("Optional AI context_length must be an integer");
    }
    if (typeof parameters.think !== "boolean") {
      throw new Error("Optional AI profile parameters.think must be boolean");
    }
    think = parameters.think;
  } else if (
    parameters.context_length !== undefined ||
    parameters.think !== undefined
  ) {
    throw new Error(
      "OpenAI-compatible local profiles must not set Ollama-only context_length or think parameters",
    );
  }

  return {
    schema_version: PROFILE_SCHEMA_VERSION,
    id: profile.id,
    display_name: profile.display_name.trim(),
    provider: profile.provider,
    endpoint: endpoint.origin,
    model: profile.model.trim(),
    system_prompt_path: resolve(
      dirname(profilePath),
      profile.system_prompt_file,
    ),
    parameters: {
      context_length: contextLength,
      temperature,
      seed,
      think,
      max_output_tokens: maxOutputTokens,
    },
  };
}

export function assertSafeHandoff({
  packet,
  systemPrompt,
  approveOnce,
  environment = process.env,
}) {
  if (!approveOnce) {
    throw new Error(
      "Optional AI task execution requires the explicit --approve-once flag",
    );
  }
  if (environment.CI === "true" || environment.GITHUB_ACTIONS === "true") {
    throw new Error(
      "Optional AI task execution is local-only and cannot run in CI",
    );
  }

  const packetBytes = Buffer.byteLength(packet, "utf8");
  if (packetBytes === 0 || packetBytes > MAX_HANDOFF_BYTES) {
    throw new Error(
      `The approved handoff packet must contain 1-${MAX_HANDOFF_BYTES} UTF-8 bytes`,
    );
  }
  if (SENSITIVE_PATTERNS.some((pattern) => pattern.test(packet))) {
    throw new Error(
      "The approved handoff packet resembles credential or private-key material",
    );
  }

  for (const pattern of REQUIRED_SYSTEM_PROMPT_PATTERNS) {
    if (!pattern.test(systemPrompt)) {
      throw new Error(
        `The selected optional AI system prompt is missing required boundary text matching ${pattern}`,
      );
    }
  }
}

function sanitizeModelSample(model, expectedModel) {
  if (!model || typeof model !== "object") return undefined;
  const name = model.name ?? model.model;
  if (name !== expectedModel) return undefined;

  return {
    sampled_at: new Date().toISOString(),
    model: name,
    loaded_bytes: optionalFiniteNumber(model.size) ?? 0,
    vram_bytes: optionalFiniteNumber(model.size_vram) ?? 0,
    context_length: optionalFiniteNumber(model.context_length) ?? 0,
    parameter_size:
      typeof model.details?.parameter_size === "string"
        ? model.details.parameter_size
        : undefined,
    quantization_level:
      typeof model.details?.quantization_level === "string"
        ? model.details.quantization_level
        : undefined,
  };
}

async function readLoadedModel(fetchImpl, profile) {
  const response = await fetchImpl(`${profile.endpoint}/api/ps`);
  if (!response.ok) {
    throw new Error(
      `Ollama model-state request failed with ${response.status}`,
    );
  }
  const payload = await response.json();
  if (!Array.isArray(payload.models)) {
    throw new Error("Ollama model-state response did not contain a model list");
  }
  return payload.models
    .map((model) => sanitizeModelSample(model, profile.model))
    .find(Boolean);
}

async function readOllamaVersion(fetchImpl, profile) {
  const response = await fetchImpl(`${profile.endpoint}/api/version`);
  if (!response.ok) {
    throw new Error(`Ollama version request failed with ${response.status}`);
  }
  const payload = await response.json();
  if (typeof payload.version !== "string" || !payload.version) {
    throw new Error("Ollama version response did not contain a version");
  }
  return payload.version;
}

async function readOpenAiCompatibleModel(fetchImpl, profile) {
  const response = await fetchImpl(`${profile.endpoint}/v1/models`);
  if (!response.ok) {
    throw new Error(
      `OpenAI-compatible model-list request failed with ${response.status}`,
    );
  }
  const payload = await response.json();
  if (!Array.isArray(payload.data)) {
    throw new Error(
      "OpenAI-compatible model-list response did not contain a data array",
    );
  }
  const model = payload.data.find(
    (candidate) => candidate?.id === profile.model,
  );
  if (!model) {
    throw new Error(
      `OpenAI-compatible server did not advertise selected model '${profile.model}'`,
    );
  }

  return {
    id: model.id,
    owned_by: typeof model.owned_by === "string" ? model.owned_by : null,
    advertised_model_file_bytes: optionalFiniteNumber(model.meta?.size) ?? null,
    advertised_training_context_length:
      optionalFiniteNumber(model.meta?.n_ctx_train) ?? null,
    advertised_parameter_count:
      optionalFiniteNumber(model.meta?.n_params) ?? null,
  };
}

async function readError(response) {
  try {
    return (await response.text()).slice(0, 1_000);
  } catch {
    return "unreadable response";
  }
}

function buildMetrics({
  task,
  profile,
  packetCharacters,
  systemPromptCharacters,
  responseCharacters,
  runtimeVersion,
  tokens,
  duration,
  resources,
  completion,
  collectedAt,
  modelUnloadRequested,
}) {
  return {
    schema_version: METRICS_SCHEMA_VERSION,
    kind: "optional-ai-task-metrics",
    collected_at: collectedAt,
    task: {
      id: task.id,
      display_name: task.display_name,
    },
    profile: {
      id: profile.id,
      display_name: profile.display_name,
      provider: profile.provider,
    },
    model: profile.model,
    runtime: {
      protocol: profile.provider,
      version: runtimeVersion,
    },
    endpoint: profile.endpoint,
    request: {
      packet_characters: packetCharacters,
      system_prompt_characters: systemPromptCharacters,
      response_characters: responseCharacters,
      context_length: profile.parameters.context_length,
      temperature: profile.parameters.temperature,
      seed: profile.parameters.seed,
      thinking_enabled: profile.parameters.think,
      maximum_output_tokens: profile.parameters.max_output_tokens,
    },
    tokens: {
      ...tokens,
      hosted_tokens_saved: "not-measured",
    },
    duration_ms: duration,
    resources,
    completion,
    privacy: {
      loopback_only: true,
      ai_required_for_wyrmgrid: false,
      wyrmgrid_runtime_integration: false,
      packet_persisted: false,
      system_prompt_persisted: false,
      response_content_in_metrics: false,
      durable_memory_created: false,
      ci_execution: false,
      model_unload_requested: modelUnloadRequested,
    },
  };
}

export function summarizeOllamaResponse({
  task,
  profile,
  response,
  resourceSamples,
  packetCharacters,
  systemPromptCharacters,
  resourceSamplingErrors = 0,
  modelLoadedAfterRun = false,
  ollamaVersion = "unknown",
  clientObservedDurationMs = null,
  collectedAt = new Date().toISOString(),
}) {
  if (
    response?.done !== true ||
    response?.message?.role !== "assistant" ||
    typeof response.message.content !== "string"
  ) {
    throw new Error("Ollama did not return a completed assistant response");
  }
  if (response.model !== profile.model) {
    throw new Error(
      `Ollama returned '${response.model ?? "unknown"}' instead of '${profile.model}'`,
    );
  }

  const totalDuration = finiteNumber(response.total_duration, "total_duration");
  const loadDuration = finiteNumber(response.load_duration, "load_duration");
  const promptTokens = finiteNumber(
    response.prompt_eval_count,
    "prompt_eval_count",
  );
  const promptDuration = finiteNumber(
    response.prompt_eval_duration,
    "prompt_eval_duration",
  );
  const generatedTokens = finiteNumber(response.eval_count, "eval_count");
  const generationDuration = finiteNumber(
    response.eval_duration,
    "eval_duration",
  );
  const peakLoadedBytes = maximum(resourceSamples, "loaded_bytes");
  const peakVramBytes = maximum(resourceSamples, "vram_bytes");

  return buildMetrics({
    task,
    profile,
    packetCharacters,
    systemPromptCharacters,
    responseCharacters: response.message.content.length,
    runtimeVersion: ollamaVersion,
    tokens: {
      prompt: promptTokens,
      generated: generatedTokens,
      local_total: promptTokens + generatedTokens,
      prompt_per_second: tokensPerSecond(promptTokens, promptDuration),
      generated_per_second: tokensPerSecond(
        generatedTokens,
        generationDuration,
      ),
    },
    duration: {
      client_observed_total:
        clientObservedDurationMs === null
          ? null
          : Math.round(
              finiteNumber(
                clientObservedDurationMs,
                "client_observed_duration_ms",
                "Local wrapper",
              ) * 100,
            ) / 100,
      server_reported_total: nanosecondsToMilliseconds(totalDuration),
      model_load: nanosecondsToMilliseconds(loadDuration),
      prompt_evaluation: nanosecondsToMilliseconds(promptDuration),
      generation: nanosecondsToMilliseconds(generationDuration),
    },
    resources: {
      measurement_status: "sampled-through-ollama-api",
      samples: resourceSamples.length,
      sampling_errors: resourceSamplingErrors,
      peak_loaded_model_bytes: peakLoadedBytes,
      peak_vram_bytes: peakVramBytes,
      estimated_peak_system_ram_bytes: Math.max(
        peakLoadedBytes - peakVramBytes,
        0,
      ),
      maximum_context_length: maximum(resourceSamples, "context_length"),
      model_loaded_after_run: modelLoadedAfterRun,
      parameter_size:
        resourceSamples.find((sample) => sample.parameter_size)
          ?.parameter_size ?? null,
      quantization_level:
        resourceSamples.find((sample) => sample.quantization_level)
          ?.quantization_level ?? null,
      advertised_model_file_bytes: null,
      advertised_training_context_length: null,
      advertised_parameter_count: null,
    },
    completion: {
      created_at: response.created_at ?? null,
      done_reason: response.done_reason ?? null,
    },
    collectedAt,
    modelUnloadRequested: true,
  });
}

export function summarizeOpenAiCompatibleResponse({
  task,
  profile,
  response,
  modelMetadata = {},
  packetCharacters,
  systemPromptCharacters,
  clientObservedDurationMs,
  collectedAt = new Date().toISOString(),
}) {
  const choice = response?.choices?.[0];
  if (
    !Array.isArray(response?.choices) ||
    response.choices.length !== 1 ||
    choice?.message?.role !== "assistant" ||
    typeof choice.message.content !== "string"
  ) {
    throw new Error(
      "OpenAI-compatible server did not return one completed assistant response",
    );
  }
  if (response.model !== profile.model) {
    throw new Error(
      `OpenAI-compatible server returned '${response.model ?? "unknown"}' instead of '${profile.model}'`,
    );
  }

  const promptTokens = finiteNumber(
    response.usage?.prompt_tokens,
    "usage.prompt_tokens",
    "OpenAI-compatible server",
  );
  const generatedTokens = finiteNumber(
    response.usage?.completion_tokens,
    "usage.completion_tokens",
    "OpenAI-compatible server",
  );
  const totalTokens = finiteNumber(
    response.usage?.total_tokens,
    "usage.total_tokens",
    "OpenAI-compatible server",
  );
  if (
    !Number.isInteger(promptTokens) ||
    !Number.isInteger(generatedTokens) ||
    !Number.isInteger(totalTokens) ||
    totalTokens !== promptTokens + generatedTokens
  ) {
    throw new Error(
      "OpenAI-compatible server usage must contain internally consistent integer token counts",
    );
  }
  const observedDuration = finiteNumber(
    clientObservedDurationMs,
    "client_observed_duration_ms",
    "Local wrapper",
  );
  const promptDuration = optionalFiniteNumber(response.timings?.prompt_ms);
  const generationDuration = optionalFiniteNumber(
    response.timings?.predicted_ms,
  );
  const serverTotal =
    promptDuration === undefined || generationDuration === undefined
      ? null
      : Math.round((promptDuration + generationDuration) * 100) / 100;
  const reportedPromptRate = optionalFiniteNumber(
    response.timings?.prompt_per_second,
  );
  const reportedGenerationRate = optionalFiniteNumber(
    response.timings?.predicted_per_second,
  );

  return buildMetrics({
    task,
    profile,
    packetCharacters,
    systemPromptCharacters,
    responseCharacters: choice.message.content.length,
    runtimeVersion: null,
    tokens: {
      prompt: promptTokens,
      generated: generatedTokens,
      local_total: totalTokens,
      prompt_per_second:
        reportedPromptRate ??
        tokensPerMillisecond(promptTokens, promptDuration),
      generated_per_second:
        reportedGenerationRate ??
        tokensPerMillisecond(generatedTokens, generationDuration),
    },
    duration: {
      client_observed_total: Math.round(observedDuration * 100) / 100,
      server_reported_total: serverTotal,
      model_load: null,
      prompt_evaluation: promptDuration ?? null,
      generation: generationDuration ?? null,
    },
    resources: {
      measurement_status: "not-exposed-by-openai-compatible-api",
      samples: 0,
      sampling_errors: 0,
      peak_loaded_model_bytes: null,
      peak_vram_bytes: null,
      estimated_peak_system_ram_bytes: null,
      maximum_context_length: null,
      model_loaded_after_run: null,
      parameter_size: null,
      quantization_level: null,
      advertised_model_file_bytes:
        modelMetadata.advertised_model_file_bytes ?? null,
      advertised_training_context_length:
        modelMetadata.advertised_training_context_length ?? null,
      advertised_parameter_count:
        modelMetadata.advertised_parameter_count ?? null,
    },
    completion: {
      created_at:
        typeof response.created === "number"
          ? new Date(response.created * 1_000).toISOString()
          : null,
      done_reason: choice.finish_reason ?? null,
    },
    collectedAt,
    modelUnloadRequested: false,
  });
}

function formatBytes(bytes) {
  if (bytes === null || bytes === undefined) return "Not reported";
  if (bytes === 0) return "0 bytes";
  const gibibytes = bytes / 1024 ** 3;
  return `${gibibytes.toFixed(2)} GiB`;
}

function formatDuration(milliseconds) {
  if (milliseconds === null || milliseconds === undefined) {
    return "Not reported";
  }
  if (milliseconds < 1_000) return `${milliseconds.toFixed(2)} ms`;
  return `${(milliseconds / 1_000).toFixed(2)} s`;
}

function formatRate(rate) {
  return rate === null || rate === undefined
    ? "Not reported"
    : `${rate.toFixed(2)} tokens/s`;
}

function formatLoadedState(state) {
  if (state === null || state === undefined) return "Not observable";
  return state ? "Yes" : "No";
}

function formatInteger(value) {
  return value === null || value === undefined
    ? "Not reported"
    : value.toLocaleString("en-AU");
}

export function renderEfficiencyReport(metrics) {
  const runtimeName =
    metrics.runtime.protocol === "ollama-chat"
      ? "Ollama"
      : "an OpenAI-compatible local server";
  const runtimeVersion = metrics.runtime.version
    ? ` ${metrics.runtime.version}`
    : " (version not reported by the compatibility API)";

  return `# Optional AI task efficiency report

Captured ${metrics.collected_at} for **${metrics.task.display_name}** (${metrics.task.id}) using optional local profile **${metrics.profile.display_name}** (${metrics.model}) through ${runtimeName}${runtimeVersion}.

## Exact local token work

| Measurement | Result |
| --- | ---: |
| Prompt tokens | ${metrics.tokens.prompt.toLocaleString("en-AU")} |
| Generated tokens | ${metrics.tokens.generated.toLocaleString("en-AU")} |
| Total local tokens | ${metrics.tokens.local_total.toLocaleString("en-AU")} |
| Prompt evaluation | ${formatRate(metrics.tokens.prompt_per_second)} |
| Generation | ${formatRate(metrics.tokens.generated_per_second)} |

## Timing

| Measurement | Result |
| --- | ---: |
| Client-observed request | ${formatDuration(metrics.duration_ms.client_observed_total)} |
| Server-reported total | ${formatDuration(metrics.duration_ms.server_reported_total)} |
| Model load | ${formatDuration(metrics.duration_ms.model_load)} |
| Prompt evaluation | ${formatDuration(metrics.duration_ms.prompt_evaluation)} |
| Generation | ${formatDuration(metrics.duration_ms.generation)} |

## Observed model resources

| Measurement | Result |
| --- | ---: |
| Peak loaded model allocation | ${formatBytes(metrics.resources.peak_loaded_model_bytes)} |
| Peak VRAM allocation | ${formatBytes(metrics.resources.peak_vram_bytes)} |
| Estimated non-VRAM allocation | ${formatBytes(metrics.resources.estimated_peak_system_ram_bytes)} |
| Resource samples | ${metrics.resources.samples.toLocaleString("en-AU")} |
| Sampling errors | ${metrics.resources.sampling_errors.toLocaleString("en-AU")} |
| Model loaded after run | ${formatLoadedState(metrics.resources.model_loaded_after_run)} |
| Resource measurement | ${metrics.resources.measurement_status} |
| Advertised model-file size | ${formatBytes(metrics.resources.advertised_model_file_bytes)} |
| Advertised training context | ${formatInteger(metrics.resources.advertised_training_context_length)} |
| Advertised parameter count | ${formatInteger(metrics.resources.advertised_parameter_count)} |

## Interpretation boundary

These are exact tokens reported by the selected local server. They are **not automatically equivalent to hosted frontier-model tokens saved** because the coordinating agent may still prepare or review the packet. Hosted token and monetary savings remain unmeasured unless the hosted service provides separate usage data.

The handoff packet and system prompt were not copied into this report or its metrics file. The request stayed on loopback, requested no tools or durable model memory, and was not executed in CI. Resource sampling and automatic unload are available through the Ollama adapter; the OpenAI-compatible standard exposes neither portably, so those fields remain explicitly unreported and model lifecycle stays under the selected local server's control. WyrmGrid does not require this AI profile, local server, or any other AI for application use, builds, tests, contributions, or release publication.
`;
}

function messages(systemPrompt, packet) {
  return [
    { role: "system", content: systemPrompt },
    { role: "user", content: packet },
  ];
}

function ollamaRequestBody(profile, systemPrompt, packet) {
  return {
    model: profile.model,
    stream: false,
    think: profile.parameters.think,
    keep_alive: 0,
    messages: messages(systemPrompt, packet),
    options: {
      num_ctx: profile.parameters.context_length,
      temperature: profile.parameters.temperature,
      seed: profile.parameters.seed,
      num_predict: profile.parameters.max_output_tokens,
    },
  };
}

function openAiCompatibleRequestBody(profile, systemPrompt, packet) {
  return {
    model: profile.model,
    stream: false,
    messages: messages(systemPrompt, packet),
    temperature: profile.parameters.temperature,
    seed: profile.parameters.seed,
    max_tokens: profile.parameters.max_output_tokens,
  };
}

async function executeOllamaTask({
  profile,
  systemPrompt,
  packet,
  fetchImpl,
  pollIntervalMs,
  timeoutMs,
  unloadTimeoutMs,
  unloadPollIntervalMs,
  monotonicNow,
}) {
  const ollamaVersion = await readOllamaVersion(fetchImpl, profile);
  await readLoadedModel(fetchImpl, profile);
  const resourceSamples = [];
  let resourceSamplingErrors = 0;
  let sampleInProgress = false;
  const sample = async () => {
    if (sampleInProgress) return;
    sampleInProgress = true;
    try {
      const model = await readLoadedModel(fetchImpl, profile);
      if (model) resourceSamples.push(model);
    } catch {
      resourceSamplingErrors += 1;
    } finally {
      sampleInProgress = false;
    }
  };
  const timer = setInterval(() => void sample(), pollIntervalMs);
  const abortController = new AbortController();
  const timeout = setTimeout(() => abortController.abort(), timeoutMs);

  const startedAt = monotonicNow();
  let responsePayload;
  try {
    const response = await fetchImpl(`${profile.endpoint}/api/chat`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(ollamaRequestBody(profile, systemPrompt, packet)),
      signal: abortController.signal,
    });
    if (!response.ok) {
      throw new Error(
        `Ollama chat request failed with ${response.status}: ${await readError(response)}`,
      );
    }
    responsePayload = await response.json();
  } finally {
    clearInterval(timer);
    clearTimeout(timeout);
  }
  const clientObservedDurationMs = monotonicNow() - startedAt;
  while (sampleInProgress) {
    await new Promise((resolveWait) => setTimeout(resolveWait, 10));
  }
  const unloadDeadline = Date.now() + unloadTimeoutMs;
  let finalModel;
  do {
    try {
      finalModel = await readLoadedModel(fetchImpl, profile);
      if (finalModel) resourceSamples.push(finalModel);
    } catch {
      resourceSamplingErrors += 1;
      break;
    }
    if (!finalModel || Date.now() >= unloadDeadline) break;
    await new Promise((resolveWait) =>
      setTimeout(resolveWait, unloadPollIntervalMs),
    );
  } while (true);

  return {
    responsePayload,
    assistantContent: responsePayload?.message?.content,
    clientObservedDurationMs,
    resourceSamples,
    resourceSamplingErrors,
    modelLoadedAfterRun: Boolean(finalModel),
    ollamaVersion,
  };
}

async function executeOpenAiCompatibleTask({
  profile,
  systemPrompt,
  packet,
  fetchImpl,
  timeoutMs,
  monotonicNow,
}) {
  const modelMetadata = await readOpenAiCompatibleModel(fetchImpl, profile);
  const abortController = new AbortController();
  const timeout = setTimeout(() => abortController.abort(), timeoutMs);
  const startedAt = monotonicNow();
  let responsePayload;
  try {
    const response = await fetchImpl(
      `${profile.endpoint}/v1/chat/completions`,
      {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(
          openAiCompatibleRequestBody(profile, systemPrompt, packet),
        ),
        signal: abortController.signal,
      },
    );
    if (!response.ok) {
      throw new Error(
        `OpenAI-compatible chat request failed with ${response.status}: ${await readError(response)}`,
      );
    }
    responsePayload = await response.json();
  } finally {
    clearTimeout(timeout);
  }

  return {
    responsePayload,
    assistantContent: responsePayload?.choices?.[0]?.message?.content,
    clientObservedDurationMs: monotonicNow() - startedAt,
    modelMetadata,
  };
}

export async function runOptionalAiTask({
  taskId,
  packetPath,
  profilePath,
  outputDirectory,
  approveOnce,
  environment = process.env,
  fetchImpl = fetch,
  pollIntervalMs = 500,
  timeoutMs = 10 * 60 * 1_000,
  unloadTimeoutMs = 5_000,
  unloadPollIntervalMs = 100,
  now = () => new Date(),
  monotonicNow = () => performance.now(),
}) {
  const task = getOptionalAiTask(taskId);
  const [packet, profileSource, taskPrompt, untouchedTemplate] =
    await Promise.all([
      readFile(packetPath, "utf8"),
      readFile(profilePath, "utf8"),
      readFile(task.prompt_path, "utf8"),
      readFile(task.template_path, "utf8"),
    ]);
  if (packet.trim() === untouchedTemplate.trim()) {
    throw new Error(
      `${task.id} handoff packet still matches the untouched template; replace its placeholders with bounded reviewed evidence`,
    );
  }
  let profileDocument;
  try {
    profileDocument = JSON.parse(profileSource);
  } catch (error) {
    throw new Error("Optional AI profile must contain valid JSON", {
      cause: error,
    });
  }
  const profile = validateLocalAiProfile(profileDocument, profilePath);
  const baseSystemPrompt = await readFile(profile.system_prompt_path, "utf8");
  const systemPrompt = `${baseSystemPrompt.trim()}\n\n${taskPrompt.trim()}\n`;
  assertMarkdownContract({
    document: packet,
    requiredHeadings: task.required_packet_headings,
    label: `${task.id} handoff packet`,
  });
  assertSafeHandoff({ packet, systemPrompt, approveOnce, environment });

  const providerResult =
    profile.provider === "ollama-chat"
      ? await executeOllamaTask({
          profile,
          systemPrompt,
          packet,
          fetchImpl,
          pollIntervalMs,
          timeoutMs,
          unloadTimeoutMs,
          unloadPollIntervalMs,
          monotonicNow,
        })
      : await executeOpenAiCompatibleTask({
          profile,
          systemPrompt,
          packet,
          fetchImpl,
          timeoutMs,
          monotonicNow,
        });
  const collectedAt = now().toISOString();
  assertMarkdownContract({
    document: providerResult.assistantContent ?? "",
    requiredHeadings: task.required_output_headings,
    label: `${task.id} assistant output`,
  });
  const commonSummary = {
    task,
    profile,
    response: providerResult.responsePayload,
    packetCharacters: packet.length,
    systemPromptCharacters: systemPrompt.length,
    clientObservedDurationMs: providerResult.clientObservedDurationMs,
    collectedAt,
  };
  const metrics =
    profile.provider === "ollama-chat"
      ? summarizeOllamaResponse({
          ...commonSummary,
          resourceSamples: providerResult.resourceSamples,
          resourceSamplingErrors: providerResult.resourceSamplingErrors,
          modelLoadedAfterRun: providerResult.modelLoadedAfterRun,
          ollamaVersion: providerResult.ollamaVersion,
        })
      : summarizeOpenAiCompatibleResponse({
          ...commonSummary,
          modelMetadata: providerResult.modelMetadata,
        });
  const report = renderEfficiencyReport(metrics);
  const runId = collectedAt.replaceAll(":", "-").replaceAll(".", "-");
  const prefix = `optional-ai-${task.id}-${runId}`;
  await mkdir(outputDirectory, { recursive: true });
  const paths = {
    draft: resolve(outputDirectory, `${prefix}-draft.md`),
    metrics: resolve(outputDirectory, `${prefix}-metrics.json`),
    report: resolve(outputDirectory, `${prefix}-efficiency.md`),
  };

  await Promise.all([
    writeFile(paths.draft, providerResult.assistantContent, {
      encoding: "utf8",
      flag: "wx",
    }),
    writeFile(paths.metrics, `${JSON.stringify(metrics, null, 2)}\n`, {
      encoding: "utf8",
      flag: "wx",
    }),
    writeFile(paths.report, report, { encoding: "utf8", flag: "wx" }),
  ]);

  return { metrics, paths };
}

export function parseArguments(args) {
  const parsed = { approveOnce: false };
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    if (argument === "--approve-once") {
      parsed.approveOnce = true;
      continue;
    }
    if (!["--task", "--packet", "--profile", "--output"].includes(argument)) {
      throw new Error(`Unsupported option '${argument}'`);
    }
    const value = args[index + 1];
    if (!value) throw new Error(`Option '${argument}' requires a value`);
    index += 1;
    if (argument === "--task") parsed.taskId = value;
    if (argument === "--packet") parsed.packetPath = resolve(value);
    if (argument === "--profile") parsed.profilePath = resolve(value);
    if (argument === "--output") parsed.outputDirectory = resolve(value);
  }

  if (
    !parsed.taskId ||
    !parsed.packetPath ||
    !parsed.profilePath ||
    !parsed.outputDirectory
  ) {
    throw new Error(
      "Usage: node scripts/run-optional-ai-task.mjs --task <task-id> --packet <path> --profile <path> --output <directory> --approve-once",
    );
  }
  return parsed;
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : undefined;
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    const result = await runOptionalAiTask(
      parseArguments(process.argv.slice(2)),
    );
    console.log(
      `${result.metrics.profile.display_name} processed ${result.metrics.tokens.local_total.toLocaleString("en-AU")} local tokens for ${result.metrics.task.id}.`,
    );
    console.log(`Draft: ${result.paths.draft}`);
    console.log(`Metrics: ${result.paths.metrics}`);
    console.log(`Efficiency report: ${result.paths.report}`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
