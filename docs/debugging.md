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

## Inspect a Jenkins AI Agent run

Use the archived `ci-artifacts/ai-agent/phases/` records before adding a new
probe. Every local invocation has its own prompt, system prompt, compact
checkpoint where applicable, redacted response, event stream, model identity,
reasoning mode, token high-water mark, and compaction count. Compare these with
`diff-summary.json`, `formatter-summary.json`, and `tests/summary.json` to locate
the first boundary that failed.

The useful distinction is:

- conversation failure: inspect the matching phase card and phase artifacts;
- prompt-transport failure: inspect the fixed wrapper invocation and
  `prompt.md`; prompt text must never appear as shell words or commands in the
  console;
- test-profile preflight failure: inspect `test-preflight.json` for a missing
  immutable path or npm script and `toolchain.json` for a missing executable;
  this is configuration evidence and must not consume a model repair;
- narrative-only failure: inspect `agent-output.json` and its Jenkins fallback;
- scope or secret rejection: use sanitized console metadata only, because no
  patch artifact is retained;
- dependency-bootstrap reconciliation: inspect `bootstrap-side-effects.json`
  for path/status metadata without file contents;
- formatter failure: inspect `formatter-summary.json` and
  `deterministic-change-rejection.json`; a previously validated
  `proposed.patch` may remain available even though the contaminated worktree
  was rejected;
- test failure: inspect only the latest bounded `tests/output.txt`, with full
  earlier output retained in prior build/phase artifacts.
- publication failure: inspect the **Publish draft PR** console section. The
  worker home is intentionally read-only; current jobs use
  `.jenkins-ai-runtime/github_git_askpass.sh` with terminal prompting disabled
  and never run `gh auth setup-git`. A `.gitconfig` write error identifies a
  stale job definition. The earlier `.permissions.push` repository-metadata
  check was not authoritative because that endpoint requires only Metadata
  read. Current jobs instead enumerate the installation repositories and
  negotiate a unique namespaced `git push --dry-run`. If that preflight or the
  actual publication returns `403`, verify the App installation includes
  WyrmGrid, accept pending Contents/Pull requests/Workflows permission updates,
  and select **All permissions available to the App installation** in Jenkins.
  Do not make the Jenkins home writable or rotate a valid key for either
  failure.

Pipeline/runtime contract tests intentionally run from the job's SCM revision
before the immutable target checkout. Registered product tests run only against
the resolved `SOURCE_REVISION`. Do not add a branch-only runtime test to a
target-revision test profile merely to validate a commissioning Jenkinsfile.

Do not enable arbitrary shell, web access, verbose provider bodies, or secret
logging to diagnose a model run. Do not infer a Qwen3-Coder reasoning failure:
the coding model intentionally receives no reasoning option.
