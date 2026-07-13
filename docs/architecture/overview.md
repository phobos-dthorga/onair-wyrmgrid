# Architecture overview

```text
OnAir API                Simulator sidecars
    |                           |
    v                           v
OnAir adapter            WyrmGrid Bridge
    |                           |
    +--------> application <----+
                    |
          +---------+---------+
          |                   |
       SQLite             plugin broker
     (Hoard)                   |
          |              external plugins
          v
      Tauri commands
          |
          v
  Svelte presentation
     |          |
 MapLibre    WyrmChart
  (Atlas)    (ECharts)
```

The dependency direction points inward. Interface and infrastructure adapters
depend on application-owned domain contracts; domain code does not depend on
Tauri, SQLite, HTTP, MapLibre, or a plugin language.

## Data categories

Every user-facing value should be traceable to one of four categories:

1. OnAir fact;
2. external fact, such as simulator telemetry or weather;
3. calculated value;
4. recommendation.

Provenance records the source and observation time. Recommendations should also
explain their contributing factors rather than presenting an opaque score.

## Extension boundary

Community plugins never link into the desktop process. The host launches a
declared entry point, grants approved capabilities, validates messages, applies
timeouts and size limits, and owns all privileged actions. Declarative map,
table, form, chart, notification, command, and inspector contributions come
before unrestricted custom UI.

## Maintainability boundary

WyrmGrid is designed to remain sustainable for one maintainer. Community-ready
boundaries must not require community-scale infrastructure. Each replaceable
technology has one application-owned adapter: OnAir JSON, SQLite, Tauri,
MapLibre, and ECharts remain outside domain rules.

New abstraction is justified by a current use case, not the possibility of a
future contributor. See
[ADR-0004](decisions/0004-declarative-charts-and-complexity-budget.md).
