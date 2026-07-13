# Security policy

## Supported versions

Until the first stable release, only the latest commit on `main` is supported.

## Reporting a vulnerability

Please use GitHub private vulnerability reporting. Do not open a public issue
for credential exposure, plugin sandbox escapes, unsafe process launching,
path traversal, arbitrary file access, remote code execution, updater or
signature bypasses, or sensitive company-data disclosure.

Please include the affected version, impact, reproduction steps, and any known
mitigation. Do not include a real OnAir API key.

## Security boundaries

- The core owns OnAir credentials; plugins receive capability-scoped data.
- Plugin manifests are untrusted input.
- Plugin processes and their output are untrusted input.
- Imported files, URLs, map styles, and API responses are untrusted input.
- The operating-system credential store is the planned persistent secret store.
- The official platform does not automate unsupported OnAir actions through
  browser scraping or simulated clicks.

See [Threat model](docs/security/threat-model.md) for the evolving design.
