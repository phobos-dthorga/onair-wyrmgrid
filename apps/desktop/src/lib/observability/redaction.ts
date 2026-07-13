import type { Breadcrumb, ErrorEvent, StackFrame } from "@sentry/sveltekit";

const SAFE_FAILURE_MESSAGE =
  "WyrmGrid encountered an unexpected interface failure.";
const SAFE_CODE = /^[a-z][a-z0-9_.-]{2,79}$/;
const SAFE_CONTEXTS = new Set(["app", "browser", "os", "runtime"]);

export function sanitizeEvent(event: ErrorEvent): ErrorEvent {
  delete event.user;
  delete event.request;
  delete event.server_name;
  delete event.transaction;
  delete event.logger;
  delete event.logentry;
  delete event.modules;
  delete event.extra;
  delete event.breadcrumbs;

  event.tags = Object.fromEntries(
    Object.entries(event.tags ?? {}).filter(([key]) => key === "error.code"),
  );
  event.contexts = Object.fromEntries(
    Object.entries(event.contexts ?? {}).filter(([key]) =>
      SAFE_CONTEXTS.has(key),
    ),
  );

  if (event.message) {
    event.message = safeCode(event.tags["error.code"]) ?? SAFE_FAILURE_MESSAGE;
  }

  for (const exception of event.exception?.values ?? []) {
    exception.value = SAFE_FAILURE_MESSAGE;
    delete exception.module;
    sanitizeFrames(exception.stacktrace?.frames);
  }
  for (const thread of event.threads?.values ?? []) {
    delete thread.name;
    sanitizeFrames(thread.stacktrace?.frames);
  }

  return event;
}

export function discardBreadcrumb(_breadcrumb: Breadcrumb): null {
  return null;
}

function sanitizeFrames(frames: StackFrame[] | undefined): void {
  for (const frame of frames ?? []) {
    frame.abs_path = safeApplicationUrl(frame.abs_path);
    frame.filename = safeApplicationUrl(frame.filename);
    delete frame.pre_context;
    delete frame.context_line;
    delete frame.post_context;
    delete frame.vars;
  }
}

function safeApplicationUrl(value: string | undefined): string | undefined {
  if (!value) return undefined;
  const normalized = value.replaceAll("\\", "/");
  if (/^[a-zA-Z]:\//.test(normalized)) return safeFilename(normalized);
  try {
    const url = new URL(value);
    const safeOrigin =
      (url.protocol === "tauri:" && url.hostname === "localhost") ||
      (url.protocol === "http:" && url.hostname === "tauri.localhost");
    return safeOrigin
      ? `${url.protocol}//${url.host}${url.pathname}`
      : undefined;
  } catch {
    return safeFilename(normalized);
  }
}

function safeFilename(value: string): string | undefined {
  const filename = value.split("/").at(-1);
  return filename && /^[a-zA-Z0-9_.-]{1,120}$/.test(filename)
    ? filename
    : undefined;
}

function safeCode(value: unknown): string | undefined {
  return typeof value === "string" && SAFE_CODE.test(value) ? value : undefined;
}
