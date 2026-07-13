# ADR-0005: Use Tauri's platform webviews

Status: Accepted

## Context

WyrmGrid needs a GPU-capable map and chart interface on Windows, macOS, and
Linux. Tauri uses Microsoft Edge WebView2 on Windows, WKWebView on macOS, and
WebKitGTK on Linux. WebView2 is Chromium-based; it is not Microsoft's legacy
EdgeHTML engine.

Bundling Chromium through Chromium Embedded Framework or an Electron-style
runtime would make the renderer more consistent across platforms, but would
also substantially increase application size, packaging work, security-update
responsibility, and platform-specific integration. That cost conflicts with
the single-maintainer complexity budget.

## Decision

- Retain Tauri's platform webview selected through WRY.
- Use the Evergreen WebView2 Runtime on Windows and let the Tauri installer
  ensure it is present where necessary.
- Treat Chromium/WebKit portability as a tested compatibility requirement.
- Use standards-based HTML, CSS, and TypeScript; avoid engine-specific APIs
  unless guarded by feature detection and a tested fallback.
- Exercise supported Windows, macOS, and Linux webviews in release CI before a
  stable release.
- Reconsider a bundled Chromium runtime only after a demonstrated, material
  incompatibility that cannot be solved reasonably within the interface layer.

## Consequences

Windows uses a recent Chromium engine without WyrmGrid shipping and servicing a
private browser runtime. macOS and Linux retain their native Tauri backends, so
pixel-perfect engine identity is not guaranteed. The application remains
smaller and its renderer lifecycle follows platform security updates, while UI
testing must cover both Chromium and WebKit behaviour.
