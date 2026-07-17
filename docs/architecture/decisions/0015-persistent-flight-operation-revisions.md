# ADR-0015: Persistent flight-operation revisions and evidence-derived manifests

**Status:** Accepted

## Context

Dispatch already holds a current SimBrief plan and may attach a read-only OnAir
job observation. Those session views are useful for comparison, but they do not
identify the operation the user actually intends to carry through Weather,
Jobs, Fleet, Staff, Atlas, Bridge, and Hoard. Silently replacing that intent
whenever a provider refreshes would make later debrief evidence ambiguous.

OnAir job legs may provide aggregate passenger counts or freight weight, but
fields can also be absent. The lifecycle foundation must preserve that
distinction without turning a plausible default into a provider fact.

## Decision

WyrmGrid owns a simulator-neutral `FlightOperationId` and immutable numbered
revisions in the core domain.

- Revision 1 is created only by an explicit **Begin flight operation** action
  and requires a validated current flight plan.
- The revision stores the sanitized WyrmGrid plan snapshot, the explicitly
  selected validated OnAir job observation and its originating company identity
  when present, and a deterministic per-leg manifest derived from that job.
- Passenger counts and freight weights are copied only when the selected job
  supplies them. A missing fact is retained as an explicit unavailable field.
- A changed plan, selected job, originating company, or same-identity job fact
  does not alter the accepted revision. The interface offers an explicit
  reviewed revision; the previous row remains append-only.
- The domain validator recomputes the manifest from its retained job evidence.
  A manifest that differs from that source is rejected, even when its shape and
  values would otherwise be valid.
- Migration 13 stores operation identity, immutable revision snapshots, and a
  single active-operation pointer in the existing SQLCipher database. Operation
  schema 1, manifest schema 1, journey schema 1, and database migration 13 are
  independent compatibility markers.
- Provider credentials, raw responses, inferred people, invented consignments,
  aircraft assignments, staff assignments, and readiness guarantees are not
  part of this slice.

The application service derives journey states and context-change notices.
Tauri commands delegate to that service, and Svelte only displays the resulting
view and forwards explicit actions.

## Consequences

An accepted operation survives application restart and remains visible even
when the current session has no imported plan. Portable backup format 1 already
copies the complete encrypted database, so it carries these operations without
a separate export format. It still never carries the device key or OnAir API
key.

The active pointer is not deletion or archival policy. Earlier operations and
revisions remain retained until a later, separately designed management flow
can list, archive, export, or delete them safely. Bridge recording association,
full people and consignment identities, fleet and staff assignment, operational
review, and Hoard debrief linkage remain later revisions of the lifecycle
model rather than assumptions hidden in schema 1.
