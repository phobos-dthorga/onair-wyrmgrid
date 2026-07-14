# Debugging WyrmGrid

WyrmGrid includes checked-in VS Code launch configurations for deliberate local
debugging. Debuggers complement tests and the privacy-bounded local diagnostic
log; they are not required for ordinary development sessions.

## Prerequisites

Install the workspace-recommended extensions when VS Code offers them:

- Rust Analyzer (`rust-lang.rust-analyzer`)
- CodeLLDB (`vadimcn.vscode-lldb`)

The normal Windows development prerequisites in [development.md](development.md)
are still required. The debugger uses the repository's pinned Rust toolchain and
normal Cargo debug profile. Do not add machine-specific debugger paths or Visual
Studio installation paths to the checked-in configuration.

## Debug the desktop application

1. Open the repository root in VS Code.
2. Open **Run and Debug** and select **WyrmGrid: debug desktop**.
3. Place breakpoints in a Rust application service, adapter, or thin Tauri
   command boundary and press **F5**.

The launch builds the desktop binary directly with Cargo and starts the Vite
development server as a background task. Because this path does not use the
Tauri CLI, its `beforeDevCommand` hook is represented by the checked-in VS Code
task instead.

Use **WyrmGrid: attach to desktop** when WyrmGrid is already running through
`npm run dev`. Select `wyrmgrid-desktop.exe` from the process list. Attach mode
is useful when the problem depends on Tauri CLI file watching or startup order.

Stopping a debug session stops the desktop process, but VS Code may retain the
shared frontend task for the next launch. Terminate **wyrmgrid: frontend dev**
from the Tasks menu when it is no longer needed.

## Debug a Rust test

Select **WyrmGrid: debug Rust tests**, choose the crate, and enter an optional
test-name filter. Leave the filter empty to run the selected test target. Test
threads are restricted to one so breakpoint ordering remains understandable.
Use **WyrmGrid: debug desktop Rust tests** for the Tauri crate; its explicit
library target avoids ambiguity with the desktop executable's empty test target.

Prefer a focused regression test over reproducing a remote provider response.
Captured OnAir data must be sanitized before it becomes a fixture, following
[the API boundary](onair/api-boundary.md).

## Inspect the Svelte WebView

In a Tauri development build, press **Ctrl+Shift+I** inside the WyrmGrid window,
or right-click the WebView and choose **Inspect**. The WebView inspector provides
Svelte/JavaScript breakpoints, the console, rendered layout inspection, and
network timing. Frontend source maps are already enabled by the desktop
TypeScript configuration.

The Rust and WebView debuggers are independent and may be used together: use
CodeLLDB for the command/application/adapter path and the WebView inspector for
the presentational client path.

## Credential and privacy rules

Debugger state is more privileged than the local diagnostic log. While a live
OnAir session is connected, assume the debugger can see inherited environment
variables and process memory:

- never expand, evaluate, print, or screenshot credential-bearing client or
  request-header objects;
- never copy raw provider responses into an issue, fixture, diagnostic entry,
  chat, or committed file;
- do not add API keys or Sentry credentials to `launch.json`, `tasks.json`,
  workspace settings, `.env` files, or debugger environment blocks;
- prefer breakpoints after the adapter has converted raw JSON into stable
  WyrmGrid domain models;
- clear copied values and close the debug session before sharing screenshots.

Pausing around a network request can produce timeouts that do not occur at full
speed. When investigating OnAir downloads, first use the English diagnostic code
to choose the boundary, then break after response receipt or reproduce the
decoder behavior with a sanitized fixture.

`RUST_BACKTRACE=1` is enabled for the checked-in launch configurations. More
verbose logging must be enabled only for a specific investigation and must
continue to obey the same credential and raw-response restrictions.
