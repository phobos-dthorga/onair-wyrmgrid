import { resolve } from "node:path";
import { defineConfig, searchForWorkspaceRoot } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { sentrySvelteKit } from "@sentry/sveltekit";
import packageMetadata from "./package.json" with { type: "json" };

const host = process.env.TAURI_DEV_HOST;
const workspaceRoot = searchForWorkspaceRoot(import.meta.dirname);
const brandAssets = resolve(import.meta.dirname, "../../assets/brand");
const sentryRelease =
  process.env.SENTRY_RELEASE || `onair-wyrmgrid@${packageMetadata.version}`;
const uploadSentrySourceMaps =
  process.env.SENTRY_UPLOAD_SOURCEMAPS === "true" &&
  Boolean(
    process.env.SENTRY_AUTH_TOKEN &&
    process.env.SENTRY_ORG &&
    process.env.SENTRY_UI_PROJECT,
  );

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    ...(await sentrySvelteKit({
      autoInstrument: false,
      autoUploadSourceMaps: uploadSentrySourceMaps,
      adapter: "other",
      authToken: uploadSentrySourceMaps
        ? process.env.SENTRY_AUTH_TOKEN
        : undefined,
      org: uploadSentrySourceMaps ? process.env.SENTRY_ORG : undefined,
      project: uploadSentrySourceMaps
        ? process.env.SENTRY_UI_PROJECT
        : undefined,
      release: { name: sentryRelease },
      telemetry: false,
    })),
    sveltekit(),
  ],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    fs: {
      allow: [workspaceRoot, brandAssets],
    },
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
