<script lang="ts">
  import type { DispatchStatus } from "$lib/dispatch/types";
  import {
    formatLocalDateTime,
    mediumDateShortTime,
  } from "$lib/presentation/dateTime";
  import { manifestHandoffState } from "./presentation";
  import type {
    FlightOperationContextChange,
    FlightOperationView,
    ManifestUnavailableField,
  } from "./types";

  let {
    operation,
    operationChange,
    jobSelection,
    busy = false,
    onoperation,
  }: {
    operation?: FlightOperationView;
    operationChange: FlightOperationContextChange;
    jobSelection?: DispatchStatus["selected_job"];
    busy?: boolean;
    onoperation: (action: "start" | "revise") => void;
  } = $props();

  const selectedJob = $derived(jobSelection?.job);
  const handoffState = $derived(
    manifestHandoffState(operation, operationChange, selectedJob?.id),
  );

  function unavailableLabel(value: ManifestUnavailableField): string {
    return value === "passenger_count"
      ? "Passenger count not reported"
      : "Freight weight not reported";
  }

  function contextChangeLabel(value: FlightOperationContextChange): string {
    if (value === "plan_and_job") return "plan and job";
    return value;
  }

  function selectedJobRoute(): string {
    const first = selectedJob?.legs[0]?.departure?.icao;
    const last = selectedJob?.legs.at(-1)?.destination?.icao;
    return first && last ? `${first} → ${last}` : "Route unavailable";
  }

  function handoffTitle(): string {
    if (handoffState === "staged_initial") return "Ready for revision 1";
    if (handoffState === "staged_revision") {
      return `Pending reviewed revision ${operation ? operation.revision + 1 : 1}`;
    }
    if (handoffState === "attached") {
      return `Attached to revision ${operation?.revision ?? 1}`;
    }
    return `Retained in revision ${operation?.revision ?? 1}`;
  }

  function handoffDetail(): string {
    if (handoffState === "staged_initial") {
      return "The selected OnAir job is staged. Begin explicitly to create an encrypted operation and derive its aggregate manifest.";
    }
    if (handoffState === "staged_revision") {
      return `Revision ${operation?.revision ?? 1} is unchanged. Create a reviewed revision to attach the current job evidence and its derived manifest.`;
    }
    if (handoffState === "attached") {
      return "The accepted manifest matches the currently selected read-only OnAir job facts and remains retained in this revision.";
    }
    return "The accepted manifest retains attributed OnAir job evidence from an earlier session.";
  }

  function selectionAvailability(): string {
    if (jobSelection?.availability === "live") return "Live snapshot";
    if (jobSelection?.availability === "cached") return "Cached snapshot";
    if (jobSelection?.availability === "offline") return "Offline snapshot";
    return "Retained revision";
  }
</script>

<article
  id="dispatch-operation"
  class="dispatch-card dispatch-operation-card"
  tabindex="-1"
>
  <div class="dispatch-card-heading">
    <div>
      <span class="eyebrow">Encrypted local operation</span>
      <h3>
        {operation
          ? `${operation.origin} → ${operation.destination}`
          : "Begin an operation"}
      </h3>
    </div>
    {#if operation}
      <strong>REVISION {operation.revision}</strong>
    {:else}
      <strong>NOT STARTED</strong>
    {/if}
  </div>

  {#if handoffState !== "empty"}
    <section
      class={`dispatch-manifest-handoff dispatch-manifest-handoff-${handoffState}`}
    >
      <div>
        <span class="eyebrow">Job-to-manifest handoff</span>
        <strong>{handoffTitle()}</strong>
      </div>
      <p>{handoffDetail()}</p>
      <dl>
        <div>
          <dt>Source</dt>
          <dd>OnAir fact</dd>
        </div>
        <div>
          <dt>Route</dt>
          <dd>
            {selectedJob
              ? selectedJobRoute()
              : `${operation?.origin} → ${operation?.destination}`}
          </dd>
        </div>
        <div>
          <dt>Evidence state</dt>
          <dd>{selectionAvailability()}</dd>
        </div>
        <div>
          <dt>{jobSelection ? "Current observation" : "Accepted evidence"}</dt>
          <dd>
            {jobSelection
              ? formatLocalDateTime(
                  jobSelection.observed_at,
                  "Observation time unavailable",
                  mediumDateShortTime,
                )
              : "Retained with revision"}
          </dd>
        </div>
      </dl>
    </section>
  {/if}

  {#if operation}
    <div class="dispatch-operation-summary">
      <span><b>{operation.manifest.legs.length}</b> manifest legs</span>
      <span
        ><b>{operation.selected_job_id ? "Attached" : "None"}</b> job evidence</span
      >
      <span><b>{operation.reason.replaceAll("_", " ")}</b> revision reason</span
      >
    </div>

    {#if operationChange !== "none"}
      <div class="dispatch-operation-revision">
        <div>
          <strong>Dispatch has changed</strong>
          <span>
            The current {contextChangeLabel(operationChange)} differs from the accepted
            revision. Nothing changes until you create a new revision.
          </span>
        </div>
        <button
          type="button"
          disabled={busy}
          onclick={() => onoperation("revise")}
        >
          Create reviewed revision
        </button>
      </div>
    {/if}

    {#if operation.manifest.legs.length}
      <ol class="dispatch-manifest-list">
        {#each operation.manifest.legs as leg}
          <li class="responsive-surface">
            <span>{leg.sequence + 1}</span>
            <div>
              <strong
                >{leg.departure?.icao ?? "Origin unavailable"} →
                {leg.destination?.icao ?? "Destination unavailable"}</strong
              >
              <small
                >Derived from retained OnAir job evidence · revision
                {operation.revision}</small
              >
            </div>
            <dl>
              <div>
                <dt>Passengers</dt>
                <dd>{leg.passengers?.count ?? "Unavailable"}</dd>
              </div>
              <div>
                <dt>Freight</dt>
                <dd>
                  {leg.freight
                    ? `${new Intl.NumberFormat().format(Math.round(leg.freight.weight_lb))} lb`
                    : "Unavailable"}
                </dd>
              </div>
            </dl>
            {#if leg.unavailable_fields.length}
              <ul
                class="dispatch-manifest-gaps"
                aria-label="Unavailable manifest facts"
              >
                {#each leg.unavailable_fields as field}
                  <li>{unavailableLabel(field)}</li>
                {/each}
              </ul>
            {/if}
          </li>
        {/each}
      </ol>
    {:else}
      <div class="dispatch-weather-prompt">
        <strong>
          {selectedJob
            ? "Selected job is awaiting a reviewed revision."
            : "No job manifest is attached."}
        </strong>
        <span>
          {selectedJob
            ? "Create the reviewed revision to attach its attributed manifest. The accepted revision remains unchanged until then."
            : "Select a verified pending job, then create a revision. WyrmGrid will not invent passengers or freight."}
        </span>
      </div>
    {/if}
  {:else}
    <p class="dispatch-card-intro">
      {selectedJob
        ? "Preserve this plan and the staged read-only OnAir job as operation revision 1. Passenger and freight facts remain exactly as supplied, including gaps."
        : "Preserve this plan as operation revision 1 without an OnAir manifest. A job can be attached later through an explicit reviewed revision."}
    </p>
    <button
      class="dispatch-inline-action"
      type="button"
      disabled={busy}
      onclick={() => onoperation("start")}
    >
      Begin flight operation
    </button>
  {/if}
</article>
