# Reusable presentation and data exploration

WyrmGrid treats collection exploration as a shared interface capability, not a
copy-and-paste feature owned independently by Staff, Jobs, Hoard, Atlas, Forge,
Diagnostics, or Security Centre. Search, data-derived filters, sorting, result
counts, filter clearing, selection reconciliation, and dossier tabs should feel
consistent while each module retains its own factual vocabulary.

This boundary is presentation-only. Filtering a Security Centre list does not
revoke authority, hiding a job does not change it, and selecting an Atlas item
does not mutate an OnAir record. Provider facts are still translated and
validated by Rust before the interface receives them.

## Shared implementation

The reusable pieces live under focused, non-domain directories:

- `apps/desktop/src/lib/exploration/collection.ts` owns query normalization,
  matching over explicit facts, unique reported values, active-filter counts,
  stable optional-value comparisons, and visible-selection reconciliation.
- `apps/desktop/src/lib/exploration/ExplorationSummary.svelte` owns the
  accessible shown/total/active-filter summary and non-destructive clear action.
- `apps/desktop/src/lib/exploration/ExplorationTabs.svelte` owns accessible
  dossier tab semantics.
- `apps/desktop/src/lib/presentation/dateTime.ts` owns tolerant parsing of ISO
  and device-database timestamps plus locale-aware display.
- `apps/desktop/src/lib/authorization/presentation.ts` owns the shared mapping
  from capability and grant-lifetime identifiers to localization keys.
- `apps/desktop/src/lib/accessibility/responsiveSurface.ts` owns the optional,
  pointer-reactive surface effect. The setting, reduced-motion behavior, and
  CSS are shared rather than reimplemented by each workspace.

Domain modules own adapters such as `staff/presentation.ts` and
`jobs/presentation.ts`. These name the fields that may be searched and define
domain-specific predicates. Keeping those adapters local is intentional: a
generic filter engine must never guess that two provider fields have equivalent
meaning.

Every shared rule has a physically separate unit test. Each domain adapter also
tests successful matches, unavailable or omitted facts, data-derived options,
sorting, and active-filter accounting.

## Current application audit

| Area                     | Current exploration behavior                                                                                                                            | Decision                                                                                                                          |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Staff                    | Search; filters derived from reported provider codes and qualifications; sorting; counts; clear; overview, qualification, and evidence tabs             | Implemented on the shared foundation                                                                                              |
| Jobs                     | Search; reported mission and payload filters; expiry-field filter; sorting; counts; clear; overview, route, payload, and evidence tabs                  | Implemented on the shared foundation                                                                                              |
| Hoard flight recordings  | Search; recorded status/capture-mode filters; plan/pin filters; sorting; counts; clear; graph, plan-comparison, and event/export tabs                   | Implemented on the shared foundation                                                                                              |
| Security Centre          | Actor/capability/decision search; data-derived lifetime and capability filters; decision filter; counts; clear; current-access and audit-history tabs   | Implemented; the interface explicitly states that filtering never changes authority                                               |
| Forge                    | Plugin, author, capability, state, and error search; installed-state and requested-capability filters; permission-review filter; sorting; counts; clear | Implemented; full plugin cards already provide the present drill-down depth                                                       |
| Diagnostics              | Code, operation, level, and message search; recorded level/operation filters; sorting; counts; presentation-only clear                                  | Implemented; destructive **Clear log** remains visually and behaviorally separate                                                 |
| Atlas                    | Aircraft, FBO, and airport search; operation/mapping filters; sorting; counts; clear; selection opens the existing inspector                            | Implemented only for received fleet and FBO facts; regional geography remains a map-data concern                                  |
| Dispatch                 | One active plan plus reconciliation sections, rather than a large peer collection                                                                       | Do not add ornamental collection filters. Reuse date presentation now; coordinate future section navigation with the journey rail |
| Fleet                    | No dedicated roster workspace exists yet; fleet facts currently feed Atlas and Hoard                                                                    | Apply this foundation when a roster is built, after the required OnAir fields are verified                                        |
| Hoard company timeline   | A chronological cursor over retained observations                                                                                                       | Keep the time navigator. Adding search would duplicate a control without a distinct user problem                                  |
| Settings and legal views | Forms and required acknowledgement flows                                                                                                                | Keep local. Their close/back and mandatory-consent behavior differs materially from collection exploration                        |

## Extraction rule

Extract a repeated rule when at least two current consumers have the same
semantics, or earlier when security, accessibility, provenance, or destructive
action safety requires one authoritative implementation. Reuse the smallest
cohesive primitive; do not build a universal workspace component with dozens of
conditional properties.

The following are candidates, not automatic abstractions:

- dialog frames, after required legal flows and ordinary dismissible dialogs
  share a proven focus/escape/back contract;
- empty/error/loading cards, after their action and announcement semantics are
  reconciled;
- fact-card grids, after compact, inspector, and dossier layouts have a stable
  common shape;
- monetary presentation, after provider currency is carried as a fact rather
  than assumed;
- provider status labels, only after authoritative catalogues are verified.

Do not centralize raw OnAir field access, domain-specific search-field lists,
permission decisions, destructive actions, or unavailable-data inference in a
Svelte component. Those belong in provider, domain, or application services.

## Adding another collection

1. Verify that every proposed field is present in a captured provider response
   or an application-owned domain contract.
2. Add a small domain `presentation.ts` adapter using the shared exploration
   primitives. Define explicit defaults and derive option lists from received
   records.
3. Add boundary tests for missing values, zero results, sort ties, and stale
   selection.
4. Compose the shared summary and tabs where they improve navigation. Keep
   destructive actions separate from filter clearing.
5. Apply responsive surfaces only through the shared accessibility action and
   honor both the user setting and operating-system reduced-motion preference.
6. Keep the Svelte workspace presentational; any rule that changes data or
   authority belongs in Rust.

This policy deliberately allows visual vocabulary to remain distinctive while
preventing invisible behavior from drifting between modules.
