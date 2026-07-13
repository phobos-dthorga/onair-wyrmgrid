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

## Accepted product decisions

The following starting decisions were accepted on 2026-07-14:

- the first case is a user-defined charter request with an optional action to
  import compatible facts from an OnAir job;
- recommendations compare several kinds of value through selectable planning
  profiles rather than reducing every benefit to credits;
- leasing is the first external acquisition path to compare with the existing
  fleet, before short-term rental or outright purchase;
- the planner may be promoted to a first-class visible WyrmGrid workspace when
  its maturity and breadth justify that status, while remaining a plugin behind
  the interface and using the same public contracts as community plugins.

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

## Value model

Only monetary return is labelled **profit**. Other benefits are separately
named value dimensions so the interface never confuses an accounting result
with a preference or recommendation:

- **financial value** — revenue, direct cost, contribution margin, cash flow,
  capital exposure, and risk-adjusted profit;
- **operational value** — utilization, empty positioning avoided, schedule
  resilience, maintenance fit, and spare capacity;
- **network value** — connectivity, coverage, feed, future opportunities, and
  strategic positioning;
- **player value** — personal flying, progression, variety, preferred aircraft,
  region, and realism goals.

Balanced, Profit-focused, and Realism-focused profiles provide understandable
defaults. Advanced users may inspect and adjust their weights. Every result
shows the dimensions independently before presenting a combined ranking, and
no non-financial value is silently converted into OnAir credits.

## Charter Desk

An initial charter plan can combine:

- a user-defined origin, destination, date window, passengers, cargo, and
  service preference, optionally seeded from an OnAir job;
- owned, rentable, leased, or purchasable aircraft candidates;
- empty positioning before and after the charter;
- payload-range and airport suitability;
- current aircraft location, condition, price, and availability when exposed;
- likely direct cost, contribution margin, break-even price, and risk buffer;
- follow-on OnAir work and strategic repositioning value;
- user preferences such as personal flying, aircraft type, region, realism, or
  avoidance of excessive AI operation.

The smallest useful Charter Desk release should solve one charter or short
multi-leg chain using the player's existing fleet. Lease comparison follows
after the core suitability and economics are trusted; short-term rental and
purchase remain later alternatives.

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

The published `AircraftLease` model includes status, start and end dates,
weekly payment and next-payment date, plus starting airframe and average engine
condition. It does not identify real-world categories such as wet, dry,
operating, or finance lease. The first planner version should therefore model
an **OnAir lease** using observed OnAir terms and a user-entered offer when live
candidate terms cannot be queried. Real-world lease classifications may be
added later as external scenarios, never inferred from those fields.

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

## Visible promotion without architectural privilege

The plugin may eventually outgrow the visual status of an optional tool. At
that point WyrmGrid may promote it into primary navigation and present its
Charter Desk and Airline Network workspaces as first-class product areas.

Promotion changes discovery and presentation, not trust boundaries. The
planner remains independently disableable, keeps plugin-owned storage, declares
permissions, communicates through versioned protocol messages, and receives no
private Rust, Tauri, Svelte, MapLibre, database, or credential access.

Promotion should require demonstrated regular use, stable workflows, acceptable
performance, accessibility coverage, migration tests, and a maintenance burden
that remains credible for the active maintainer. It is not triggered merely by
feature count.

## Staged delivery

1. **Feasibility inventory** — identify the exact WyrmGrid facts, external
   sources, user inputs, units, freshness rules, and missing data for one real
   charter case.
2. **Charter suitability** — rank the current fleet for one mission and explain
   payload, range, airport, positioning, and maintenance constraints.
3. **Charter economics and leasing** — add cost, price, margin, sensitivity,
   saved plans, and lease comparisons where observed data permits.
4. **Airline scenario** — compare one proposed recurring route across the
   current fleet using explicit demand and schedule assumptions.
5. **Network planning** — add multiple routes, connections, utilization,
   maintenance allowance, cash-flow effects, and strategic scoring.
6. **Living-market observations** — add safe, conservatively synchronized price
   and availability observations only for supported, bounded OnAir queries.

Each stage must remain useful on its own. None depends on building a general
optimization platform, custom UI runtime, or comprehensive real-world aircraft
database first.

## Resolved starting questions

| Question                                             | Recommended starting answer                                                                                           |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| What is the first concrete planning case?            | A user-defined charter request, optionally seeded from an OnAir job, using the current fleet.                         |
| What planning horizon should be supported first?     | One mission or a short chain; add a seven-day view before longer horizons.                                            |
| Should real-world or OnAir values win?               | OnAir for game-state and economy; official manuals for technical context; always show the selected source.            |
| Which acquisition modes matter first?                | Compare the current fleet with leasing first; add short-term rental and outright purchase later.                      |
| How should unknown demand be handled?                | User-editable best/base/worst bands, never a fabricated live-demand value.                                            |
| What should the default objective be?                | Selectable Balanced, Profit-focused, and Realism-focused profiles with independently visible value dimensions.        |
| Should the plugin perform actions in OnAir?          | No. It prepares a plan; the user executes supported actions in OnAir.                                                 |
| Should charter and airline planning ship separately? | No. Keep one plugin; it may later receive first-class visible status without receiving architectural privilege.       |
| How much realism should be mandatory?                | Safe, credible defaults with optional advanced constraints; never require a full manual data pack for basic planning. |
| When is an optimizer justified?                      | Only after real scenarios expose a repeated failure of deterministic filtering and scoring.                           |

## Questions intentionally left open

Before implementation, the first real charter scenario still needs concrete
passenger, cargo, airport, time-window, and service requirements. We must also
validate the lease fields against authenticated data and determine whether
candidate lease offers are exposed by a bounded public query. The initial
profiles must also be scoped either globally or separately for each company and
planning workspace.
