<script lang="ts">
  import "./dispatch.css";
  import type {
    DispatchStatus,
    Mass,
    SimBriefReferenceKind,
  } from "$lib/dispatch/types";

  let {
    status,
    busy = false,
    errorMessage = "",
    onimport,
    onclear,
  }: {
    status: DispatchStatus;
    busy?: boolean;
    errorMessage?: string;
    onimport: (kind: SimBriefReferenceKind, reference: string) => void;
    onclear: () => void;
  } = $props();

  let referenceKind = $state<SimBriefReferenceKind>("pilot_id");
  let reference = $state("");

  const plan = $derived(status.snapshot);
  const airports = $derived(plan?.airports.value);
  const schedule = $derived(plan?.schedule?.value);
  const aircraft = $derived(plan?.aircraft?.value);
  const route = $derived(plan?.route?.value);
  const weights = $derived(plan?.weights?.value);
  const fuel = $derived(plan?.fuel?.value);

  function formatDate(value: string | undefined): string {
    if (!value) return "Not supplied";
    const date = new Date(value);
    return Number.isNaN(date.getTime())
      ? "Not supplied"
      : date.toLocaleString([], { dateStyle: "medium", timeStyle: "short" });
  }

  function formatTime(value: string | undefined): string {
    if (!value) return "—";
    const date = new Date(value);
    return Number.isNaN(date.getTime())
      ? "—"
      : date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }

  function formatDuration(seconds: number | undefined): string {
    if (seconds === undefined) return "—";
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.round((seconds % 3600) / 60);
    return `${hours}h ${minutes.toString().padStart(2, "0")}m`;
  }

  function formatMass(value: Mass | undefined): string {
    if (!value) return "—";
    const unit = value.unit === "kilograms" ? "kg" : "lb";
    return `${new Intl.NumberFormat().format(Math.round(value.value))} ${unit}`;
  }
</script>

<section class="dispatch-workspace" aria-label="Dispatch flight plan workspace">
  <aside class="dispatch-import-panel">
    <div>
      <span class="eyebrow">WyrmGrid Dispatch</span>
      <h2>Flight-plan intake</h2>
      <p class="dispatch-muted">
        Bring the latest SimBrief plan into this session. WyrmGrid never asks
        for your SimBrief or Navigraph password.
      </p>
    </div>

    <form
      class="dispatch-import-form"
      onsubmit={(event) => {
        event.preventDefault();
        onimport(referenceKind, reference.trim());
      }}
    >
      <label>
        <span>Account reference</span>
        <select bind:value={referenceKind} disabled={busy}>
          <option value="pilot_id">Pilot ID</option>
          <option value="username">Username</option>
        </select>
      </label>
      <label>
        <span>{referenceKind === "pilot_id" ? "Pilot ID" : "Username"}</span>
        <input
          bind:value={reference}
          type="text"
          autocomplete="off"
          spellcheck="false"
          maxlength={64}
          placeholder={referenceKind === "pilot_id" ? "1234567" : "wyrm.pilot"}
          disabled={busy || !status.provider_available}
        />
      </label>
      <button
        class="dispatch-primary-action"
        type="submit"
        disabled={busy || !status.provider_available || !reference.trim()}
      >
        {busy ? "Retrieving latest OFP…" : "Import latest OFP"}
      </button>
    </form>

    {#if errorMessage}
      <div class="dispatch-notice dispatch-error" role="alert">
        <strong>Import not completed</strong>
        <span>{errorMessage}</span>
      </div>
    {:else}
      <div class="dispatch-notice">
        <strong>Session-only by design</strong>
        <span>
          Your account reference and imported plan are not written to Hoard in
          this preview. Closing WyrmGrid clears them.
        </span>
      </div>
    {/if}

    <div class="dispatch-boundary">
      <span class="dispatch-boundary-mark">READ</span>
      <p>
        This imports the newest plan only. It cannot create, edit, file, or
        dispatch a flight through SimBrief.
      </p>
    </div>
  </aside>

  <section class="dispatch-board">
    {#if plan && airports}
      <header class="dispatch-route-header">
        <div class="dispatch-airport">
          <span>Origin</span>
          <strong>{airports.origin.icao}</strong>
          <small>{airports.origin.name ?? "Name not supplied"}</small>
          <i>{airports.origin.planned_runway ?? "RWY —"}</i>
        </div>
        <div class="dispatch-route-line" aria-hidden="true">
          <span></span><b>✦</b><span></span>
          <small>{route?.distance_nm?.toFixed(0) ?? "—"} NM</small>
        </div>
        <div class="dispatch-airport dispatch-airport-arrival">
          <span>Destination</span>
          <strong>{airports.destination.icao}</strong>
          <small>{airports.destination.name ?? "Name not supplied"}</small>
          <i>{airports.destination.planned_runway ?? "RWY —"}</i>
        </div>
      </header>

      <div class="dispatch-summary-strip">
        <article><span>Aircraft</span><strong>{aircraft?.icao_type ?? "—"}</strong><small>{aircraft?.registration ?? aircraft?.model ?? "Not supplied"}</small></article>
        <article><span>Block out</span><strong>{formatTime(schedule?.scheduled_out)}</strong><small>{formatDate(schedule?.scheduled_out)}</small></article>
        <article><span>Enroute</span><strong>{formatDuration(schedule?.estimated_enroute_seconds)}</strong><small>Planned duration</small></article>
        <article><span>Initial level</span><strong>{route?.initial_altitude_ft ? `FL${Math.round(route.initial_altitude_ft / 100)}` : "—"}</strong><small>SimBrief calculation</small></article>
        <article><span>AIRAC</span><strong>{plan.identity.value.airac ?? "—"}</strong><small>Provider revision</small></article>
      </div>

      <div class="dispatch-plan-grid">
        <article class="dispatch-card dispatch-route-card">
          <div class="dispatch-card-heading">
            <div><span class="eyebrow">Route spine</span><h3>Filed route</h3></div>
            <strong>{route?.legs.length ?? 0} fixes</strong>
          </div>
          <p class="dispatch-route-text">{route?.source_text ?? "Route text was not supplied."}</p>
          {#if route?.legs.length}
            <ol class="dispatch-fix-list">
              {#each route.legs.slice(0, 12) as leg}
                <li><span>{leg.sequence + 1}</span><strong>{leg.ident}</strong><small>{leg.airway ?? "DCT"}</small></li>
              {/each}
            </ol>
            {#if route.legs.length > 12}<small class="dispatch-muted">+ {route.legs.length - 12} additional fixes retained in the snapshot</small>{/if}
          {/if}
        </article>

        <article class="dispatch-card">
          <div class="dispatch-card-heading"><div><span class="eyebrow">Mass plan</span><h3>Weights</h3></div></div>
          <dl class="dispatch-metric-list">
            <div><dt>Payload</dt><dd>{formatMass(weights?.payload)}</dd></div>
            <div><dt>Zero fuel</dt><dd>{formatMass(weights?.zero_fuel)}</dd></div>
            <div><dt>Takeoff</dt><dd>{formatMass(weights?.takeoff)}</dd></div>
            <div><dt>Landing</dt><dd>{formatMass(weights?.landing)}</dd></div>
          </dl>
        </article>

        <article class="dispatch-card">
          <div class="dispatch-card-heading"><div><span class="eyebrow">Fuel plan</span><h3>Release fuel</h3></div></div>
          <dl class="dispatch-metric-list">
            <div><dt>Ramp</dt><dd>{formatMass(fuel?.ramp)}</dd></div>
            <div><dt>Enroute</dt><dd>{formatMass(fuel?.enroute)}</dd></div>
            <div><dt>Reserve</dt><dd>{formatMass(fuel?.reserve)}</dd></div>
            <div><dt>Alternate</dt><dd>{formatMass(fuel?.alternate)}</dd></div>
          </dl>
        </article>
      </div>
    {:else}
      <div class="dispatch-empty-state">
        <div class="dispatch-runway" aria-hidden="true"><span></span><i>34L</i></div>
        <span class="eyebrow">Awaiting operational flight plan</span>
        <h2>Dispatch begins with a known plan.</h2>
        <p>
          Import the latest SimBrief OFP to inspect its route, schedule, weights,
          fuel, aircraft, alternates, and source metadata without changing it.
        </p>
      </div>
    {/if}
  </section>

  <aside class="dispatch-inspector">
    <span class="eyebrow">Plan inspector</span>
    {#if plan && airports}
      <h2>{airports.origin.icao} → {airports.destination.icao}</h2>
      <p>External operational calculation</p>

      <div class="dispatch-inspector-stack">
        <article><span>Provider</span><strong>SimBrief</strong><small>Read-only latest OFP</small></article>
        <article><span>Generated</span><strong>{formatDate(plan.identity.provenance.generated_at)}</strong></article>
        <article><span>Retrieved</span><strong>{formatDate(plan.identity.provenance.retrieved_at)}</strong></article>
        <article><span>Alternate</span><strong>{airports.alternates.map((airport) => airport.icao).join(", ") || "None supplied"}</strong></article>
      </div>

      <div class="dispatch-clearance">
        <span>Interpretation</span>
        <strong>Planning assistance only</strong>
        <p>
          Imported values remain attributed to SimBrief. WyrmGrid has not
          certified this plan for real-world navigation or regulatory dispatch.
        </p>
      </div>

      <button class="dispatch-secondary-action" type="button" disabled={busy} onclick={onclear}>Clear session plan</button>
    {:else}
      <h2>No plan selected</h2>
      <p>Import a plan to inspect its source and freshness.</p>
      <div class="dispatch-radar" aria-hidden="true"><span></span><span></span><i></i></div>
    {/if}
  </aside>
</section>
