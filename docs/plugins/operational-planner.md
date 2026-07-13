# Operational Planner plugin concept

Status: product concept; not scheduled for the initial plugin-host proof.

The Operational Planner is intended to become a first-party flagship plugin and
a practical demonstration of WyrmGrid's public extension system. It will help a
player explore a plan, understand its assumptions, and compare alternatives. It
will not execute purchases, accept work, or create routes in OnAir.

The working package identity is `org.wyrmgrid.operational-planner`, presented as
one plugin with two related workspaces:

- **Charter Desk** plans individual missions, short chains, positioning, and
  aircraft acquisition for charter and ad-hoc operators.
- **Airline Network** plans recurring routes, frequencies, schedules, fleet
  assignment, capacity, and longer-term network development.

These should begin as two modes of one plugin, not two independently maintained
plugins. They share most facts, calculations, settings, and interface
contributions. A split is justified only if their release cadence or dependency
needs genuinely diverge.

## Product promise

A useful recommendation should answer four questions:

1. What is being proposed?
2. Which current facts and external specifications support it?
3. Which assumptions and calculations were introduced by WyrmGrid?
4. Why did this alternative outrank the others?

The planner therefore produces explainable scenarios rather than a single
opaque score. Every result retains provenance and observation time, and stale
or unavailable inputs remain visible.

## Shared planning foundation

Both workspaces can share a deliberately small planning kernel:

- aircraft suitability: payload, seats, range, runway/airport constraints, and
  crew requirements;
- operating economics: acquisition or rental cost, fuel, maintenance reserve,
  positioning, airport charges when known, and time value;
- availability windows and provisional aircraft assignment;
- scenario assumptions, constraints, alternatives, and sensitivity ranges;
- saved plans in isolated plugin storage;
- explainable ranking with independent financial, operational, and strategic
  components;
- declarative Atlas layers, tables, forms, and charts published through public
  plugin contracts.

The first implementation should use straightforward filtering and weighted
scoring. A linear, constraint, or mixed-integer solver should be introduced only
after a real planning case demonstrates that simpler methods cannot produce a
useful result.

## Charter Desk

An initial charter plan can combine:

- origin, destination, date window, passengers, cargo, and service preference;
- owned, rentable, leased, or purchasable aircraft candidates;
- empty positioning before and after the charter;
- payload-range and airport suitability;
- current aircraft location, condition, price, and availability when exposed;
- likely direct cost, contribution margin, break-even price, and risk buffer;
- follow-on OnAir work and strategic repositioning value;
- user preferences such as personal flying, aircraft type, region, realism, or
  avoidance of excessive AI operation.

The smallest useful Charter Desk release should solve one charter or short
multi-leg chain using the player's existing fleet. Acquisition and market
discovery can follow after the core suitability and economics are trusted.

## Airline Network

An airline scenario can combine:

- candidate origin and destination airports;
- frequencies, time windows, turnaround assumptions, and weekly patterns;
- passenger classes, cargo allocation, aircraft capacity, and utilization;
- fleet assignment, spare capacity, maintenance allowance, and positioning;
- estimated revenue, operating cost, break-even load factor, and capital need;
- connections, network coverage, cannibalization, and expansion value;
- best/base/worst demand bands when OnAir does not expose authoritative demand;
- comparison between using the current fleet and acquiring another aircraft.

Until OnAir exposes a supported route-query or route-management endpoint, an
"available" or "creatable" route is a WyrmGrid scenario, not an OnAir fact. The
user must perform any supported route creation inside OnAir itself.

## Current OnAir feasibility

The public Swagger contract was reviewed on 2026-07-14. It currently supports
useful read paths for company fleet, jobs, FBOs, flights, finances, individual
aircraft economic and maintenance details, aircraft types, airports, and the
aircraft located at a specified airport.

Aircraft and aircraft-type models include potentially useful facts such as
base or sale price, rental price, sell/rent/lease flags, seats, cargo capacity,
range, fuel capacity and flow, airport-size requirement, current airport,
condition, hours, and maintenance state. Each field still requires a sanitized
authenticated fixture before the planner treats it as a supported WyrmGrid
domain fact.

Important limitations:

- airport aircraft can be queried for a known ICAO, but the public contract does
  not expose a global aircraft-market listing;
- route-shaped models exist in the Swagger definitions, but no public route
  query, creation, or update path is published;
- the presence of an apparent write endpoint elsewhere in Swagger does not
  broaden WyrmGrid's read-only boundary;
- live price or availability must never be inferred from stale history without
  an explicit age and estimate label;
- the planner must continue to work with manual scenario inputs when a live fact
  is unavailable.

Endpoint capability is revalidated before each implementation slice. Raw OnAir
responses remain inside `wyrmgrid-onair-api`; the planner receives only stable,
permission-filtered WyrmGrid models and never the API key.

## External technical and product sources

External source adapters may contribute:

- aircraft flight manuals and type-certificate data;
- manufacturer performance and loading specifications;
- airport and runway reference data;
- published fuel, maintenance, or operating-cost references;
- other licensed datasets that materially improve a scenario.

Official manuals and manufacturer publications are preferred for technical
facts. The source registry records title, publisher, edition or revision,
effective date, URL or local reference, retrieval time, units, and any
transformation. WyrmGrid stores normalized facts and citations; it does not
redistribute copyrighted manuals or proprietary datasets unless their licences
permit it.

Conflicts are not silently blended. For example, an OnAir aircraft-type value,
a real-world manual value, and a user override remain separate inputs. The
scenario explicitly selects one and explains the choice.

## Presentation contributions

The plugin is a strong test of several public extension points:

- Atlas markers, candidate route lines, reachability regions, and network gaps;
- selection-inspector sections for suitability and scenario participation;
- tables for aircraft, route, schedule, and acquisition comparisons;
- forms for constraints and user assumptions;
- charts for profit, cash flow, utilization, load factor, break-even points,
  sensitivity, and scenario comparison;
- notifications for stale inputs or a materially changed plan;
- plugin-owned saved scenarios and published, bounded result datasets.

The host renders these contributions. The plugin does not receive the MapLibre,
Svelte, Tauri, or chart-library objects and does not inject executable interface
code.

Likely permissions include company, fleet, jobs, and finance read access;
plugin storage; map-layer and chart publication; and optional external network
access. Every permission remains separately reviewable and deny-by-default.

## Staged delivery

1. **Feasibility inventory** — identify the exact WyrmGrid facts, external
   sources, user inputs, units, freshness rules, and missing data for one real
   charter case.
2. **Charter suitability** — rank the current fleet for one mission and explain
   payload, range, airport, positioning, and maintenance constraints.
3. **Charter economics** — add cost, price, margin, sensitivity, saved plans,
   and acquisition comparisons where observed data permits.
4. **Airline scenario** — compare one proposed recurring route across the
   current fleet using explicit demand and schedule assumptions.
5. **Network planning** — add multiple routes, connections, utilization,
   maintenance allowance, cash-flow effects, and strategic scoring.
6. **Living-market observations** — add safe, conservatively synchronized price
   and availability observations only for supported, bounded OnAir queries.

Each stage must remain useful on its own. None depends on building a general
optimization platform, custom UI runtime, or comprehensive real-world aircraft
database first.

## Product questions and recommended starting answers

| Question                                             | Recommended starting answer                                                                                           |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| What is the first concrete planning case?            | One charter request using the current owned fleet.                                                                    |
| What planning horizon should be supported first?     | One mission or a short chain; add a seven-day view before longer horizons.                                            |
| Should real-world or OnAir values win?               | OnAir for game-state and economy; official manuals for technical context; always show the selected source.            |
| Which acquisition modes matter first?                | Compare owned and rented aircraft first, then purchase and lease when live evidence is available.                     |
| How should unknown demand be handled?                | User-editable best/base/worst bands, never a fabricated live-demand value.                                            |
| What should the default objective be?                | Explainable balance of profit, operational feasibility, positioning, and strategic value.                             |
| Should the plugin perform actions in OnAir?          | No. It prepares a plan; the user executes supported actions in OnAir.                                                 |
| Should charter and airline planning ship separately? | No. Keep one package with two workspaces until maintenance evidence supports a split.                                 |
| How much realism should be mandatory?                | Safe, credible defaults with optional advanced constraints; never require a full manual data pack for basic planning. |
| When is an optimizer justified?                      | Only after real scenarios expose a repeated failure of deterministic filtering and scoring.                           |

Before implementation begins, the most consequential user decisions are the
first real charter scenario, the relative importance of economic versus
realism objectives, and whether rental or purchase comparison should follow the
owned-fleet milestone first.
