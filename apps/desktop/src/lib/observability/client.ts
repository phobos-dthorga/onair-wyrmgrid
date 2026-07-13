import * as Sentry from "@sentry/sveltekit";
import packageMetadata from "../../../package.json";
import { discardBreadcrumb, sanitizeEvent } from "./redaction";

const dsn = import.meta.env.VITE_SENTRY_DSN?.trim();
const buildEnabled =
  import.meta.env.VITE_WYRMGRID_SENTRY_ENABLED?.trim().toLowerCase() ===
    "true" && Boolean(dsn);

let active = false;

export async function configureClientTelemetry(
  userEnabled: boolean,
): Promise<void> {
  const shouldBeActive = userEnabled && buildEnabled;
  if (shouldBeActive === active) return;

  if (!shouldBeActive) {
    await Sentry.close(2_000);
    active = false;
    return;
  }

  Sentry.init({
    dsn,
    release:
      import.meta.env.VITE_SENTRY_RELEASE?.trim() ||
      `onair-wyrmgrid@${packageMetadata.version}`,
    environment:
      import.meta.env.VITE_SENTRY_ENVIRONMENT?.trim() || "maintainer",
    sendDefaultPii: false,
    sendClientReports: false,
    attachStacktrace: true,
    tracesSampleRate: 0,
    replaysSessionSampleRate: 0,
    replaysOnErrorSampleRate: 0,
    enableLogs: false,
    maxBreadcrumbs: 0,
    beforeSend: sanitizeEvent,
    beforeBreadcrumb: discardBreadcrumb,
  });
  active = true;

  if (
    import.meta.env.VITE_WYRMGRID_SENTRY_TEST_EVENT?.trim().toLowerCase() ===
    "true"
  ) {
    Sentry.withScope((scope) => {
      scope.setTag("error.code", "diagnostic.synthetic_test");
      Sentry.captureException(new Error("Synthetic WyrmGrid interface test"));
    });
  }
}
