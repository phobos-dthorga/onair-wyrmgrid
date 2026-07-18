<script lang="ts">
  import type {
    FlightOperationJourneyView,
    FlightOperationStage,
    FlightOperationStageState,
  } from "./types";

  let {
    journey,
    onstage,
  }: {
    journey: FlightOperationJourneyView;
    onstage: (stage: FlightOperationStage) => void;
  } = $props();

  const labels: Record<FlightOperationStage, string> = {
    plan: "Plan",
    weather: "Weather",
    jobs: "Jobs",
    manifest: "Manifest",
    fleet: "Fleet",
    staff: "Staff",
    review: "Review",
    atlas: "Atlas",
  };

  const stateLabels: Record<FlightOperationStageState, string> = {
    not_started: "Not started",
    available: "Available",
    ready: "Ready",
    needs_attention: "Needs attention",
    stale: "Stale",
    unavailable: "Unavailable",
  };

  function openStage(stage: FlightOperationStage): void {
    onstage(stage);
  }

  function isActionable(state: FlightOperationStageState): boolean {
    return state !== "not_started" && state !== "unavailable";
  }
</script>

<nav class="flight-journey" aria-label="Flight operation journey">
  <div class="flight-journey-heading">
    <div>
      <span class="eyebrow">Flight operation</span>
      <h3>Plan to flight</h3>
    </div>
    <span>Host-verified progress</span>
  </div>
  <ol>
    {#each journey.stages as item, index}
      <li class={`flight-journey-${item.state}`}>
        <button
          class="responsive-surface"
          type="button"
          disabled={!isActionable(item.state)}
          onclick={() => openStage(item.stage)}
          aria-label={`${labels[item.stage]}: ${stateLabels[item.state]}`}
        >
          <span class="flight-journey-index">{index + 1}</span>
          <span class="flight-journey-copy">
            <strong>{labels[item.stage]}</strong>
            <small>{stateLabels[item.state]}</small>
          </span>
        </button>
      </li>
    {/each}
  </ol>
</nav>
