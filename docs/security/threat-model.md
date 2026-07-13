# Threat model

## Protected assets

- OnAir API credentials and company identifiers;
- fleet, employee, finance, job, and flight history;
- local files and operating-system access;
- plugin trust decisions and signatures;
- update integrity and release artifacts.

## Primary threats

- credential disclosure through logs, errors, telemetry, storage, or plugins;
- malicious plugin manifests, executables, dependencies, and messages;
- path traversal and unsafe process arguments;
- unbounded messages, event storms, hangs, and resource exhaustion;
- hostile API payloads, imported files, map styles, and URLs;
- dependency or release-pipeline compromise;
- stale data presented as current fact;
- recommendations mistaken for OnAir-provided facts.

## Initial controls

- secrets wrapped and redacted at the adapter boundary;
- read-only API design;
- explicit provenance and observation timestamps;
- deny-by-default plugin capabilities;
- relative plugin entry-point validation;
- content security policy for the desktop webview;
- locked dependencies, dependency updates, audit jobs, and CI-built releases;
- no plugin runtime until framing, lifecycle, limits, and permission review are
  specified and tested.
- chart contributions are data-only; the host rejects executable callbacks,
  arbitrary ECharts options, HTML tooltips, non-finite values, oversized series,
  and charts published without `charts_publish`.

Before stable release, the project needs operating-system credential storage,
signed updates, hardened plugin supervision, abuse-case tests, and a formal
security review of every external input boundary.
