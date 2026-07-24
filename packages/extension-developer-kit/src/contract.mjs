import { isAbsolute } from "node:path";

import { extensionDefinition } from "./catalog.mjs";

export const ID_PATTERN =
  /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?){2,}$/;
export const VERSION_PATTERN =
  /^(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)$/;
export const MACHINE_ID_PATTERN = /^[a-z0-9][a-z0-9_.-]*$/;
export const PACKAGE_PATH_PATTERN = /^[A-Za-z0-9._/-]+$/;
export const SHA256_PATTERN = /^[0-9a-f]{64}$/;

export const PACKAGE_LIMITS = Object.freeze({
  archiveBytes: 32 * 1024 * 1024,
  expandedBytes: 64 * 1024 * 1024,
  fileBytes: 16 * 1024 * 1024,
  files: 512,
  pathBytes: 240,
  pathDepth: 8,
  componentBytes: 80,
});

const PLATFORMS = new Set([
  "windows_x86_64",
  "linux_x86_64",
  "macos_aarch64",
  "macos_x86_64",
]);
const PLUGIN_PERMISSIONS = new Set([
  "on_air_company_read",
  "on_air_fleet_read",
  "on_air_jobs_read",
  "on_air_finance_read",
  "map_layers_publish",
  "charts_publish",
  "notifications_create",
  "plugin_storage",
  "simulator_telemetry_read",
  "external_network",
  "weather_data_publish",
]);
const WEATHER_CAPABILITIES = new Set([
  "airport_reports",
  "forecast_grid",
  "radar_tiles",
]);
const BRIDGE_CAPABILITIES = new Set([
  "telemetry_read",
  "active_plan_read",
  "flight_plan_load",
  "command_execute",
]);
const AUDIO_PROVIDER_CAPABILITIES = new Set([
  "source_enumeration",
  "permission_requests",
  "pcm_s16le_capture",
  "level_metering",
  "hot_plug_notifications",
  "clock_synchronization",
]);
const CODEC_PROFILE_SPECS = Object.freeze({
  pilot_microphone_v1: Object.freeze({ channels: 1 }),
  isolated_voice_v1: Object.freeze({ channels: 1 }),
  mixed_stereo_v1: Object.freeze({ channels: 2 }),
});
const PACKET_DURATIONS = new Set([120, 240, 480, 960, 1920, 2880]);

export function isPlainObject(value) {
  return (
    typeof value === "object" &&
    value !== null &&
    !Array.isArray(value) &&
    Object.getPrototypeOf(value) === Object.prototype
  );
}

export function issue(code, path, message) {
  return { code, path, message };
}

function reservedWindowsComponent(component) {
  const stem = component.split(".", 1)[0].toUpperCase();
  return (
    ["CON", "PRN", "AUX", "NUL"].includes(stem) ||
    /^(?:COM|LPT)[1-9]$/.test(stem)
  );
}

export function validatePackagePath(path) {
  const components = typeof path === "string" ? path.split("/") : [];
  if (
    typeof path !== "string" ||
    path.length === 0 ||
    Buffer.byteLength(path, "ascii") !== path.length ||
    path.length > PACKAGE_LIMITS.pathBytes ||
    path.startsWith("/") ||
    path.endsWith("/") ||
    path.includes("//") ||
    path.includes("\\") ||
    !PACKAGE_PATH_PATTERN.test(path) ||
    components.length > PACKAGE_LIMITS.pathDepth ||
    components.some(
      (component) =>
        component.length === 0 ||
        component === "." ||
        component === ".." ||
        component.length > PACKAGE_LIMITS.componentBytes ||
        reservedWindowsComponent(component),
    )
  )
    throw new Error(`Unsafe package path: ${path}`);
}

function add(issues, condition, code, path, message) {
  if (!condition) issues.push(issue(code, path, message));
}

function exactKeys(issues, value, allowed, required, path = "manifest") {
  if (!isPlainObject(value)) {
    issues.push(issue("invalid_type", path, "must be a JSON object"));
    return false;
  }
  for (const key of required)
    add(
      issues,
      Object.hasOwn(value, key),
      "missing_field",
      `${path}.${key}`,
      "is required",
    );
  for (const key of Object.keys(value))
    add(
      issues,
      allowed.has(key),
      "unknown_field",
      `${path}.${key}`,
      "is not part of this compatibility contract",
    );
  return true;
}

function text(value, maximum) {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= maximum &&
    value.trim() === value
  );
}

function uniqueKnownArray(
  issues,
  value,
  known,
  path,
  { minimum = 0, maximum = Number.MAX_SAFE_INTEGER } = {},
) {
  add(issues, Array.isArray(value), "invalid_type", path, "must be an array");
  if (!Array.isArray(value)) return;
  add(
    issues,
    value.length >= minimum && value.length <= maximum,
    "invalid_count",
    path,
    `must contain ${minimum}-${maximum} entries`,
  );
  add(
    issues,
    new Set(value).size === value.length,
    "duplicate_value",
    path,
    "must not contain duplicates",
  );
  value.forEach((entry, index) =>
    add(
      issues,
      typeof entry === "string" && known.has(entry),
      "unsupported_value",
      `${path}[${index}]`,
      "is not supported by EDK v1",
    ),
  );
}

function validateCommon(issues, manifest) {
  add(
    issues,
    typeof manifest.id === "string" &&
      manifest.id.length <= 255 &&
      ID_PATTERN.test(manifest.id),
    "invalid_id",
    "manifest.id",
    "must be a lowercase reverse-domain identifier",
  );
  add(
    issues,
    text(manifest.name, 120),
    "invalid_text",
    "manifest.name",
    "must be 1-120 trimmed characters",
  );
  add(
    issues,
    VERSION_PATTERN.test(manifest.version ?? ""),
    "invalid_version",
    "manifest.version",
    "must be an X.Y.Z semantic version",
  );
  add(
    issues,
    text(manifest.author, 120),
    "invalid_text",
    "manifest.author",
    "must be 1-120 trimmed characters",
  );
  let safeEntryPoint = true;
  try {
    validatePackagePath(manifest.entry_point);
    safeEntryPoint =
      typeof manifest.entry_point === "string" &&
      !isAbsolute(manifest.entry_point);
  } catch {
    safeEntryPoint = false;
  }
  add(
    issues,
    safeEntryPoint,
    "unsafe_entry_point",
    "manifest.entry_point",
    "must be a safe relative package path",
  );
}

function validatePlugin(issues, manifest) {
  const required = [
    "id",
    "name",
    "version",
    "api_version",
    "author",
    "entry_point",
  ];
  const allowed = new Set([
    ...required,
    "runtime",
    "permissions",
    "weather_capabilities",
    "network_origins",
  ]);
  if (!exactKeys(issues, manifest, allowed, required)) return;
  validateCommon(issues, manifest);
  add(
    issues,
    manifest.api_version === 1,
    "unsupported_api",
    "manifest.api_version",
    "EDK v1 supports plugin API version 1",
  );
  add(
    issues,
    manifest.runtime === undefined || manifest.runtime === "python",
    "unsupported_runtime",
    "manifest.runtime",
    'must be omitted or equal "python"',
  );
  const permissions = manifest.permissions ?? [];
  const weather = manifest.weather_capabilities ?? [];
  const origins = manifest.network_origins ?? [];
  uniqueKnownArray(
    issues,
    permissions,
    PLUGIN_PERMISSIONS,
    "manifest.permissions",
  );
  uniqueKnownArray(
    issues,
    weather,
    WEATHER_CAPABILITIES,
    "manifest.weather_capabilities",
  );
  add(
    issues,
    Array.isArray(origins) && origins.length <= 8,
    "invalid_network_origins",
    "manifest.network_origins",
    "must contain at most eight HTTPS origins",
  );
  if (Array.isArray(origins)) {
    const normalized = [];
    origins.forEach((origin, index) => {
      let accepted;
      try {
        const parsed = new URL(origin);
        accepted =
          parsed.protocol === "https:" &&
          parsed.hostname.length > 0 &&
          parsed.username === "" &&
          parsed.password === "" &&
          (parsed.pathname === "" || parsed.pathname === "/") &&
          parsed.search === "" &&
          parsed.hash === "" &&
          origin.length <= 200 &&
          origin.trim() === origin;
        normalized.push(parsed.origin);
      } catch {
        accepted = false;
      }
      add(
        issues,
        accepted,
        "invalid_network_origin",
        `manifest.network_origins[${index}]`,
        "must be a bounded HTTPS origin without credentials, path, query, or fragment",
      );
    });
    add(
      issues,
      new Set(normalized).size === normalized.length,
      "duplicate_network_origin",
      "manifest.network_origins",
      "must not normalize to duplicate origins",
    );
  }
  const publishesWeather =
    Array.isArray(permissions) && permissions.includes("weather_data_publish");
  const usesNetwork =
    Array.isArray(permissions) && permissions.includes("external_network");
  add(
    issues,
    publishesWeather === (Array.isArray(weather) && weather.length > 0),
    "inconsistent_weather_capability",
    "manifest.weather_capabilities",
    "must be present exactly when weather_data_publish is requested",
  );
  add(
    issues,
    usesNetwork === (Array.isArray(origins) && origins.length > 0) &&
      (!usesNetwork || publishesWeather),
    "inconsistent_network_authority",
    "manifest.network_origins",
    "must be present exactly with external_network and weather_data_publish",
  );
}

function validateNativeBase(
  issues,
  manifest,
  { schemaVersion, protocolField, protocolVersion, capabilities, maximum },
) {
  validateCommon(issues, manifest);
  add(
    issues,
    manifest.schema_version === schemaVersion,
    "unsupported_schema",
    "manifest.schema_version",
    `EDK v1 supports manifest schema ${schemaVersion}`,
  );
  add(
    issues,
    manifest[protocolField] === protocolVersion,
    "unsupported_protocol",
    `manifest.${protocolField}`,
    `EDK v1 supports protocol version ${protocolVersion}`,
  );
  if (manifest.$schema !== undefined)
    add(
      issues,
      text(manifest.$schema, 256),
      "invalid_schema_reference",
      "manifest.$schema",
      "must be a bounded non-empty string",
    );
  uniqueKnownArray(
    issues,
    manifest.platforms,
    PLATFORMS,
    "manifest.platforms",
    {
      minimum: 1,
    },
  );
  uniqueKnownArray(
    issues,
    manifest.capabilities,
    capabilities,
    "manifest.capabilities",
    { minimum: 1, maximum },
  );
}

function validateSimulatorProvider(issues, manifest) {
  const required = [
    "schema_version",
    "id",
    "name",
    "version",
    "bridge_protocol_version",
    "author",
    "entry_point",
    "platforms",
    "simulators",
    "capabilities",
  ];
  if (!exactKeys(issues, manifest, new Set(["$schema", ...required]), required))
    return;
  validateNativeBase(issues, manifest, {
    schemaVersion: 1,
    protocolField: "bridge_protocol_version",
    protocolVersion: 1,
    capabilities: BRIDGE_CAPABILITIES,
    maximum: 4,
  });
  add(
    issues,
    Array.isArray(manifest.simulators) && manifest.simulators.length > 0,
    "invalid_simulators",
    "manifest.simulators",
    "must contain at least one simulator identifier",
  );
  if (Array.isArray(manifest.simulators)) {
    add(
      issues,
      new Set(manifest.simulators).size === manifest.simulators.length,
      "duplicate_value",
      "manifest.simulators",
      "must not contain duplicates",
    );
    manifest.simulators.forEach((value, index) =>
      add(
        issues,
        typeof value === "string" &&
          value.length <= 96 &&
          MACHINE_ID_PATTERN.test(value),
        "invalid_simulator",
        `manifest.simulators[${index}]`,
        "must be a bounded machine identifier",
      ),
    );
  }
}

function validateAudioProvider(issues, manifest) {
  const required = [
    "schema_version",
    "id",
    "name",
    "version",
    "audio_protocol_version",
    "author",
    "entry_point",
    "platforms",
    "capabilities",
  ];
  if (!exactKeys(issues, manifest, new Set(["$schema", ...required]), required))
    return;
  validateNativeBase(issues, manifest, {
    schemaVersion: 2,
    protocolField: "audio_protocol_version",
    protocolVersion: 2,
    capabilities: AUDIO_PROVIDER_CAPABILITIES,
    maximum: 6,
  });
}

function validateCodecProfile(issues, profile, index) {
  const path = `manifest.profiles[${index}]`;
  const required = [
    "id",
    "codec_id",
    "media_type",
    "channels",
    "sample_rate_hz",
    "target_bitrate_bps",
    "packet_duration_48khz_frames",
  ];
  if (!exactKeys(issues, profile, new Set(required), required, path)) return;
  const specification = CODEC_PROFILE_SPECS[profile.id];
  add(
    issues,
    specification !== undefined,
    "unsupported_profile",
    `${path}.id`,
    "is not a WyrmGrid audio profile",
  );
  add(
    issues,
    specification?.channels === profile.channels,
    "invalid_channels",
    `${path}.channels`,
    "does not match the selected WyrmGrid profile",
  );
  add(
    issues,
    profile.sample_rate_hz === 48_000,
    "invalid_sample_rate",
    `${path}.sample_rate_hz`,
    "must equal 48000",
  );
  add(
    issues,
    typeof profile.codec_id === "string" &&
      profile.codec_id.length <= 96 &&
      MACHINE_ID_PATTERN.test(profile.codec_id),
    "invalid_codec_id",
    `${path}.codec_id`,
    "must be a bounded machine identifier",
  );
  add(
    issues,
    typeof profile.media_type === "string" &&
      /^[a-z0-9!#$&^_.+-]+\/[a-z0-9!#$&^_.+-]+$/i.test(profile.media_type) &&
      profile.media_type.length <= 120,
    "invalid_media_type",
    `${path}.media_type`,
    "must be a bounded media type",
  );
  add(
    issues,
    Number.isInteger(profile.target_bitrate_bps) &&
      profile.target_bitrate_bps >= 8_000 &&
      profile.target_bitrate_bps <= 10_000_000,
    "invalid_bitrate",
    `${path}.target_bitrate_bps`,
    "must be between 8000 and 10000000",
  );
  add(
    issues,
    PACKET_DURATIONS.has(profile.packet_duration_48khz_frames),
    "invalid_packet_duration",
    `${path}.packet_duration_48khz_frames`,
    "is not a supported 48 kHz packet duration",
  );
}

function validateAudioCodec(issues, manifest) {
  const required = [
    "schema_version",
    "id",
    "name",
    "version",
    "codec_protocol_version",
    "author",
    "entry_point",
    "platforms",
    "capabilities",
    "profiles",
  ];
  if (!exactKeys(issues, manifest, new Set(["$schema", ...required]), required))
    return;
  validateNativeBase(issues, manifest, {
    schemaVersion: 1,
    protocolField: "codec_protocol_version",
    protocolVersion: 1,
    capabilities: new Set(["encode_pcm_s16le"]),
    maximum: 1,
  });
  add(
    issues,
    Array.isArray(manifest.profiles) &&
      manifest.profiles.length >= 1 &&
      manifest.profiles.length <= 16,
    "invalid_profiles",
    "manifest.profiles",
    "must contain 1-16 codec profiles",
  );
  if (Array.isArray(manifest.profiles)) {
    manifest.profiles.forEach((profile, index) =>
      validateCodecProfile(issues, profile, index),
    );
    const ids = manifest.profiles.map((profile) => profile?.id);
    add(
      issues,
      new Set(ids).size === ids.length,
      "duplicate_profile",
      "manifest.profiles",
      "must not contain duplicate profile IDs",
    );
  }
}

export function validateExtensionManifest(kind, manifest) {
  const issues = [];
  extensionDefinition(kind);
  switch (kind) {
    case "plugin":
      validatePlugin(issues, manifest);
      break;
    case "simulator-provider":
      validateSimulatorProvider(issues, manifest);
      break;
    case "audio-provider":
      validateAudioProvider(issues, manifest);
      break;
    case "audio-codec":
      validateAudioCodec(issues, manifest);
      break;
  }
  return issues;
}

export function validatePackageManifest(definition, manifest) {
  const issues = [];
  const required = [
    "schema_version",
    "kind",
    "id",
    "version",
    "manifest_path",
    "files",
  ];
  if (!exactKeys(issues, manifest, new Set(required), required, "package"))
    return issues;
  add(
    issues,
    manifest.schema_version === 1,
    "unsupported_package_schema",
    "package.schema_version",
    "EDK v1 supports package schema version 1",
  );
  add(
    issues,
    manifest.kind === definition.packageKind,
    "wrong_package_kind",
    "package.kind",
    `must equal ${definition.packageKind}`,
  );
  add(
    issues,
    typeof manifest.id === "string" && ID_PATTERN.test(manifest.id),
    "invalid_id",
    "package.id",
    "must be a lowercase reverse-domain identifier",
  );
  add(
    issues,
    VERSION_PATTERN.test(manifest.version ?? ""),
    "invalid_version",
    "package.version",
    "must be an X.Y.Z semantic version",
  );
  add(
    issues,
    manifest.manifest_path === definition.manifestPath,
    "wrong_manifest_path",
    "package.manifest_path",
    `must equal ${definition.manifestPath}`,
  );
  const minimum = definition.kind === "plugin" ? 1 : 2;
  add(
    issues,
    Array.isArray(manifest.files) &&
      manifest.files.length >= minimum &&
      manifest.files.length <= PACKAGE_LIMITS.files,
    "invalid_inventory",
    "package.files",
    `must contain ${minimum}-${PACKAGE_LIMITS.files} files`,
  );
  if (Array.isArray(manifest.files)) {
    const paths = [];
    for (const [index, file] of manifest.files.entries()) {
      const path = `package.files[${index}]`;
      if (
        !exactKeys(
          issues,
          file,
          new Set(["path", "size", "sha256"]),
          ["path", "size", "sha256"],
          path,
        )
      )
        continue;
      let safe = true;
      try {
        validatePackagePath(file.path);
      } catch {
        safe = false;
      }
      add(
        issues,
        safe,
        "unsafe_package_path",
        `${path}.path`,
        "must be a safe portable package path",
      );
      add(
        issues,
        Number.isInteger(file.size) &&
          file.size >= 1 &&
          file.size <= PACKAGE_LIMITS.fileBytes,
        "invalid_file_size",
        `${path}.size`,
        "is outside the package file bound",
      );
      add(
        issues,
        SHA256_PATTERN.test(file.sha256 ?? ""),
        "invalid_digest",
        `${path}.sha256`,
        "must be a lowercase SHA-256 digest",
      );
      paths.push(file.path);
    }
    add(
      issues,
      new Set(paths).size === paths.length &&
        new Set(paths.map((path) => path?.toLowerCase())).size === paths.length,
      "duplicate_package_path",
      "package.files",
      "must not contain duplicate or case-colliding paths",
    );
    add(
      issues,
      paths.includes(definition.manifestPath),
      "missing_manifest",
      "package.files",
      `must include ${definition.manifestPath}`,
    );
  }
  return issues;
}
