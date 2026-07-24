# Fleet Locations example plugin

This small Plugin API version 1 example reads the approved fleet snapshot and
publishes a bounded Atlas point layer. It never receives the OnAir API key or
raw OnAir payloads.

With Extension Developer Kit v1 installed:

```powershell
wyrmgrid-extension validate --source .
wyrmgrid-extension test --source .
wyrmgrid-extension package `
  --source . `
  --output .\dist\fleet-locations.wyrmplugin
```

Runtime conformance requires Python and the `wyrmgrid_sdk` package. Use
`--skip-runtime` only for packaging validation when that runtime is
intentionally unavailable; the resulting compatibility report records the
runtime check as skipped.

The example requests only `on_air_fleet_read` and `map_layers_publish`. A new
plugin should begin with no permissions and add only the capabilities its
implemented behaviour requires.
