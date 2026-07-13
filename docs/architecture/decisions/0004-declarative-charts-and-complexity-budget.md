# ADR-0004: Declarative charts and a single-maintainer complexity budget

Status: Accepted

## Context

WyrmGrid needs polished operational charts and eventually permits plugins to
contribute visual analysis. Allowing arbitrary chart-library configuration or
custom interface code would expose the desktop process to unstable APIs,
untrusted content, inconsistent presentation, and a support burden that is not
appropriate while the project has one maintainer.

The project must remain useful and maintainable even if community contributors
never materialise. Extensibility is an architectural boundary, not a forecast
of future labour.

## Decision

- Apache ECharts is the initial chart renderer.
- `WyrmChart.svelte` is the only application component that talks directly to
  ECharts.
- Rust owns a small, versioned `ChartSpec` contract. Plugins submit data and
  presentation intent; they cannot submit JavaScript, callbacks, HTML, themes,
  or native ECharts options.
- Version one supports line, area, and bar charts. New chart families are added
  only in response to a real first-party or plugin use case.
- The host owns colours, accessibility, resizing, tooltips, empty states,
  provenance, and resource limits.
- The `charts_publish` capability is required before an external plugin may
  publish a chart.

Adding the chart schema and permission is backward-compatible within plugin API
version 1. Existing manifests and messages remain valid. Incompatible changes
require a new chart schema version and an explicit migration path.

## Complexity budget

Before adding another library, process, protocol layer, code generator, or SDK,
the change must demonstrate all of the following:

1. a current WyrmGrid use case rather than a hypothetical contributor need;
2. less total maintenance than a small local implementation or existing layer;
3. a clear owner and replacement boundary;
4. tests or validation proportional to its failure impact;
5. safe degradation when the optional capability is unavailable.

Code used once stays local. A shared abstraction is introduced only after its
boundary is clear, but duplication and magic protocol strings are removed as
soon as that boundary is established.

## Consequences

The first chart types are intentionally limited, but their behaviour is
consistent and plugins remain independent of Svelte and ECharts. Replacing the
renderer requires changing one interface component rather than the Rust domain
or plugin protocol.
