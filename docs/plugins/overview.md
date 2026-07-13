# Plugin platform

WyrmGrid plugins are separate processes. The initial transport will use framed
JSON messages over standard input and output; JSON-RPC semantics may be adopted
where they improve tooling, but transport framing and lifecycle messages must be
specified before plugins execute.

`plugin.json` declares identity, compatibility, entry point, and requested
permissions. The host validates it before launch. A manifest is not a sandbox:
process isolation, operating-system controls, message validation, timeouts, and
user trust decisions remain necessary.

The version-one manifest groundwork is in
`schemas/plugin-manifest.schema.json`, mirrored by Rust types in
`wyrmgrid-plugin-protocol`. The example is intentionally non-executable until
the lifecycle and framing contract are accepted.
