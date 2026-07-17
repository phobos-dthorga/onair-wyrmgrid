<script lang="ts">
  import type {
    FlightOperationContextChange,
    FlightOperationView,
    ManifestUnavailableField,
  } from "./types";

  let {
    operation,
    operationChange,
    busy = false,
    onoperation,
  }: {
    operation?: FlightOperationView;
    operationChange: FlightOperationContextChange;
    busy?: boolean;
    onoperation: (action: "start" | "revise") => void;
  } = $props();

  function unavailableLabel(value: ManifestUnavailableField): string {
    return value === "passenger_count"
      ? "Passenger count not reported"
      : "Freight weight not reported";
  }

  function contextChangeLabel(value: FlightOperationContextChange): string {
    if (value === "plan_and_job") return "plan and job";
    return value;
  }
</script>

<article id="dispatch-operation" class="dispatch-card dispatch-operation-card">
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
          <li>
            <span>{leg.sequence + 1}</span>
            <div>
              <strong
                >{leg.departure?.icao ?? "Origin unavailable"} →
                {leg.destination?.icao ?? "Destination unavailable"}</strong
              >
              <small>Read-only OnAir job evidence</small>
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
        <strong>No job manifest is attached.</strong>
        <span
          >Select a verified pending job, then create a revision. WyrmGrid will
          not invent passengers or freight.</span
        >
      </div>
    {/if}
  {:else}
    <p class="dispatch-card-intro">
      Preserve this plan and the currently selected read-only job as operation
      revision 1. Passenger and freight facts remain exactly as OnAir supplied
      them, including gaps.
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
