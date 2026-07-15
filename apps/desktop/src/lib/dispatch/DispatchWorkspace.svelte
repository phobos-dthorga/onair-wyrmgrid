<script lang="ts">
  import "./dispatch.css";
  import { translation } from "$lib/i18n/runtime";
  import type {
    DispatchStatus,
    Mass,
    SimBriefAccountPreference,
    SimBriefReferenceKind,
  } from "$lib/dispatch/types";

  let {
    status,
    busy = false,
    errorMessage = "",
    accountPreference,
    onimport,
    onweather,
    onclear,
    onviewatlas,
  }: {
    status: DispatchStatus;
    busy?: boolean;
    errorMessage?: string;
    accountPreference?: SimBriefAccountPreference;
    onimport: (
      kind: SimBriefReferenceKind,
      reference: string,
      rememberReference: boolean,
    ) => void;
    onweather: () => void;
    onclear: () => void;
    onviewatlas: (pointId?: string) => void;
  } = $props();

  let referenceKind = $state<SimBriefReferenceKind>("pilot_id");
  let reference = $state("");
  let rememberReference = $state(false);

  $effect(() => {
    if (accountPreference) {
      referenceKind = accountPreference.reference_kind;
      reference = accountPreference.reference;
      rememberReference = true;
    }
  });

  const plan = $derived(status.snapshot);
  const selectedJob = $derived(status.selected_job?.job);
  const airports = $derived(plan?.airports.value);
  const schedule = $derived(plan?.schedule?.value);
  const aircraft = $derived(plan?.aircraft?.value);
  const route = $derived(plan?.route?.value);
  const weights = $derived(plan?.weights?.value);
  const fuel = $derived(plan?.fuel?.value);
  const comparison = $derived(status.comparison);
  const weather = $derived(status.weather.snapshot);
  const atlasPlan = $derived(status.atlas_plan);
  const routeMapPoints = $derived(
    atlasPlan?.points.filter((point) => point.kind === "route_leg") ?? [],
  );

  function atlasPointId(kind: "origin" | "destination"): string | undefined {
    return atlasPlan?.points.find((point) => point.kind === kind)?.id;
  }

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

  function formatFindingCategory(value: string): string {
    return value.replaceAll("_", " ");
  }

  function selectedJobRoute(): string {
    const first = selectedJob?.legs[0]?.departure?.icao;
    const last = selectedJob?.legs.at(-1)?.destination?.icao;
    return first && last ? `${first} → ${last}` : "Route unavailable";
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

    {#if selectedJob}
      <article class="dispatch-selected-job">
        <span class="eyebrow">Selected OnAir job</span>
        <strong>{selectedJob.mission_type ?? "Pending job"}</strong>
        <span>{selectedJobRoute()}</span>
        <small>Read-only Hoard observation</small>
      </article>
    {/if}

    <form
      class="dispatch-import-form"
      onsubmit={(event) => {
        event.preventDefault();
        onimport(referenceKind, reference.trim(), rememberReference);
      }}
    >
      <label>
        <span>Account reference</span>
        <select bind:value={referenceKind} disabled={busy}>
          <option value="pilot_id">Pilot ID</option>
          <option value="username">Username</option>
        </select>
      </label>
      <label class="dispatch-remember-reference">
        <input
          type="checkbox"
          bind:checked={rememberReference}
          disabled={busy}
        />
        <span>
          <strong>Remember this account reference</strong>
          <small>
            Saves only the Pilot ID or username in WyrmGrid's encrypted local
            database—never a SimBrief or Navigraph password.
          </small>
        </span>
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
        {busy ? "Working…" : "Import latest OFP"}
      </button>
    </form>

    {#if errorMessage}
      <div class="dispatch-notice dispatch-error" role="alert">
        <strong>Dispatch action not completed</strong>
        <span>{errorMessage}</span>
      </div>
    {:else}
      <div class="dispatch-notice">
        <strong
          >{rememberReference
            ? "Account reference remembered"
            : "Session-only by choice"}</strong
        >
        <span>
          {rememberReference
            ? "The reference is retained locally; imported plans remain session-only in this preview."
            : "The account reference and imported plan are cleared when WyrmGrid closes."}
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
        <button
          type="button"
          class="dispatch-airport dispatch-map-link"
          title="Open the origin on Atlas"
          onclick={() => onviewatlas(atlasPointId("origin"))}
        >
          <span>Origin</span>
          <strong>{airports.origin.icao}</strong>
          <small>{airports.origin.name ?? "Name not supplied"}</small>
          <i>{airports.origin.planned_runway ?? "RWY —"}</i>
        </button>
        <div class="dispatch-route-line" aria-hidden="true">
          <span></span><b>✦</b><span></span>
          <small>{route?.distance_nm?.toFixed(0) ?? "—"} NM</small>
        </div>
        <button
          type="button"
          class="dispatch-airport dispatch-airport-arrival dispatch-map-link"
          title="Open the destination on Atlas"
          onclick={() => onviewatlas(atlasPointId("destination"))}
        >
          <span>Destination</span>
          <strong>{airports.destination.icao}</strong>
          <small>{airports.destination.name ?? "Name not supplied"}</small>
          <i>{airports.destination.planned_runway ?? "RWY —"}</i>
        </button>
      </header>

      <div class="dispatch-summary-strip">
        <article>
          <span>Aircraft</span><strong>{aircraft?.icao_type ?? "—"}</strong
          ><small
            >{aircraft?.registration ??
              aircraft?.model ??
              "Not supplied"}</small
          >
        </article>
        <article>
          <span>Block out</span><strong
            >{formatTime(schedule?.scheduled_out)}</strong
          ><small>{formatDate(schedule?.scheduled_out)}</small>
        </article>
        <article>
          <span>Enroute</span><strong
            >{formatDuration(schedule?.estimated_enroute_seconds)}</strong
          ><small>Planned duration</small>
        </article>
        <article>
          <span>Initial level</span><strong
            >{route?.initial_altitude_ft
              ? `FL${Math.round(route.initial_altitude_ft / 100)}`
              : "—"}</strong
          ><small>SimBrief calculation</small>
        </article>
        <article>
          <span>AIRAC</span><strong>{plan.identity.value.airac ?? "—"}</strong
          ><small>Provider revision</small>
        </article>
      </div>

      <div class="dispatch-plan-grid">
        <article class="dispatch-card dispatch-route-card">
          <div class="dispatch-card-heading">
            <div>
              <span class="eyebrow">Route spine</span>
              <h3>Filed route</h3>
            </div>
            <div class="dispatch-route-actions">
              <strong>{route?.legs.length ?? 0} fixes</strong>
              <button type="button" onclick={() => onviewatlas()}>
                View full route in Atlas
              </button>
            </div>
          </div>
          <p class="dispatch-route-text">
            {route?.source_text ?? "Route text was not supplied."}
          </p>
          {#if routeMapPoints.length}
            <ol class="dispatch-fix-list">
              {#each routeMapPoints.slice(0, 12) as point}
                <li>
                  <button
                    type="button"
                    class:dispatch-map-unavailable={!point.location}
                    title={point.location
                      ? `Open ${point.label} on Atlas`
                      : `Open ${point.label} in Atlas; its location was not supplied`}
                    onclick={() => onviewatlas(point.id)}
                  >
                    <span>{(point.sequence ?? 0) + 1}</span><strong
                      >{point.label}</strong
                    ><small>{point.airway ?? "DCT"}</small>
                    {#if !point.location}<i>Location unavailable</i>{/if}
                  </button>
                </li>
              {/each}
            </ol>
            {#if routeMapPoints.length > 12}<small class="dispatch-muted"
                >+ {routeMapPoints.length - 12} additional fixes retained in the snapshot</small
              >{/if}
          {/if}
        </article>

        <article class="dispatch-card">
          <div class="dispatch-card-heading">
            <div>
              <span class="eyebrow">Mass plan</span>
              <h3>Weights</h3>
            </div>
          </div>
          <dl class="dispatch-metric-list">
            <div>
              <dt>Payload</dt>
              <dd>{formatMass(weights?.payload)}</dd>
            </div>
            <div>
              <dt>Zero fuel</dt>
              <dd>{formatMass(weights?.zero_fuel)}</dd>
            </div>
            <div>
              <dt>Takeoff</dt>
              <dd>{formatMass(weights?.takeoff)}</dd>
            </div>
            <div>
              <dt>Landing</dt>
              <dd>{formatMass(weights?.landing)}</dd>
            </div>
          </dl>
        </article>

        <article class="dispatch-card">
          <div class="dispatch-card-heading">
            <div>
              <span class="eyebrow">Fuel plan</span>
              <h3>Release fuel</h3>
            </div>
          </div>
          <dl class="dispatch-metric-list">
            <div>
              <dt>Ramp</dt>
              <dd>{formatMass(fuel?.ramp)}</dd>
            </div>
            <div>
              <dt>Enroute</dt>
              <dd>{formatMass(fuel?.enroute)}</dd>
            </div>
            <div>
              <dt>Reserve</dt>
              <dd>{formatMass(fuel?.reserve)}</dd>
            </div>
            <div>
              <dt>Alternate</dt>
              <dd>{formatMass(fuel?.alternate)}</dd>
            </div>
          </dl>
        </article>
      </div>

      <div class="dispatch-intelligence-grid">
        <article class="dispatch-card dispatch-intelligence-card">
          <div class="dispatch-card-heading">
            <div>
              <span class="eyebrow">Explainable reconciliation</span>
              <h3>Plan cross-check</h3>
            </div>
            <strong
              >{comparison?.fleet_available ? "ON AIR" : "NO FLEET"}</strong
            >
          </div>
          <p class="dispatch-card-intro">
            WyrmGrid keeps SimBrief calculations and OnAir observations
            separate, then shows the evidence behind every match, difference, or
            gap.
          </p>
          {#if comparison}
            <ol class="dispatch-finding-list">
              {#each comparison.findings as finding}
                <li class={`dispatch-finding-${finding.status}`}>
                  <div class="dispatch-finding-heading">
                    <span>{formatFindingCategory(finding.category)}</span>
                    <b>{finding.status}</b>
                  </div>
                  <strong
                    >{$translation(
                      `${finding.message_key}-title`,
                      {},
                      finding.title,
                    )}</strong
                  >
                  <p>
                    {$translation(
                      `${finding.message_key}-explanation`,
                      {},
                      finding.explanation,
                    )}
                  </p>
                  {#if finding.plan_value || finding.onair_value}
                    <dl>
                      <div>
                        <dt>Plan</dt>
                        <dd>{finding.plan_value ?? "Not supplied"}</dd>
                      </div>
                      <div>
                        <dt>OnAir</dt>
                        <dd>{finding.onair_value ?? "Not observed"}</dd>
                      </div>
                    </dl>
                  {/if}
                </li>
              {/each}
            </ol>
          {/if}
        </article>

        <article
          class="dispatch-card dispatch-intelligence-card dispatch-weather-card"
        >
          <div class="dispatch-card-heading">
            <div>
              <span class="eyebrow">External airport facts</span>
              <h3>METAR + TAF context</h3>
            </div>
            <strong>{status.weather.cache}</strong>
          </div>
          <p class="dispatch-card-intro">
            Public AviationWeather.gov reports for the origin, destination, and
            alternates. Raw coded text remains visible; no hidden safety score
            is applied.
          </p>
          <button
            class="dispatch-inline-action"
            type="button"
            disabled={busy || !status.weather.provider_available}
            onclick={onweather}
          >
            {busy
              ? "Working…"
              : weather
                ? "Refresh when due"
                : "Fetch airport weather"}
          </button>

          {#if weather}
            <div class="dispatch-weather-grid">
              {#each weather.airports as airport}
                <section class="dispatch-weather-station">
                  <header>
                    <strong>{airport.station_icao}</strong>
                    <span
                      class={`dispatch-flight-category dispatch-flight-category-${airport.metar?.value.flight_category ?? "unknown"}`}
                    >
                      {airport.metar?.value.flight_category?.toUpperCase() ??
                        "NO METAR"}
                    </span>
                  </header>
                  {#if airport.metar}
                    <div class="dispatch-weather-metrics">
                      <span
                        >Wind <b
                          >{airport.metar.value.wind_direction?.kind ===
                          "degrees"
                            ? `${airport.metar.value.wind_direction.value.toString().padStart(3, "0")}°`
                            : airport.metar.value.wind_direction?.kind ===
                                "variable"
                              ? "VRB"
                              : "—"} / {airport.metar.value.wind_speed_kt ??
                            "—"} kt{airport.metar.value.wind_gust_kt
                            ? ` G${airport.metar.value.wind_gust_kt}`
                            : ""}</b
                        ></span
                      >
                      <span
                        >Visibility <b
                          >{airport.metar.value.visibility_sm
                            ? `${airport.metar.value.visibility_sm} sm`
                            : "—"}</b
                        ></span
                      >
                      <span
                        >Temp <b>{airport.metar.value.temperature_c ?? "—"}°C</b
                        ></span
                      >
                      <span
                        >Observed <b
                          >{formatDate(airport.metar.value.observed_at)}</b
                        ></span
                      >
                    </div>
                    <p class="dispatch-coded-weather">
                      {airport.metar.value.raw_text}
                    </p>
                  {:else}
                    <p class="dispatch-weather-empty">
                      No recent METAR was returned for this station.
                    </p>
                  {/if}
                  {#if airport.taf}
                    <div class="dispatch-taf-heading">
                      <span>TAF valid to</span><strong
                        >{formatDate(airport.taf.value.valid_to)}</strong
                      >
                    </div>
                    <p class="dispatch-coded-weather dispatch-coded-taf">
                      {airport.taf.value.raw_text}
                    </p>
                  {:else}
                    <p class="dispatch-weather-empty">
                      No current TAF was returned for this station.
                    </p>
                  {/if}
                </section>
              {/each}
            </div>
          {:else}
            <div class="dispatch-weather-prompt">
              <strong>Weather has not been requested.</strong>
              <span
                >The first explicit fetch remains in memory and is reused for
                ten minutes.</span
              >
            </div>
          {/if}
        </article>
      </div>
    {:else}
      <div class="dispatch-empty-state">
        <div class="dispatch-runway" aria-hidden="true">
          <span></span><i>34L</i>
        </div>
        <span class="eyebrow">Awaiting operational flight plan</span>
        <h2>Dispatch begins with a known plan.</h2>
        <p>
          Import the latest SimBrief OFP to inspect its route, schedule,
          weights, fuel, aircraft, alternates, and source metadata without
          changing it.
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
        <article>
          <span>Provider</span><strong>SimBrief</strong><small
            >Read-only latest OFP</small
          >
        </article>
        <article>
          <span>Generated</span><strong
            >{formatDate(plan.identity.provenance.generated_at)}</strong
          >
        </article>
        <article>
          <span>Retrieved</span><strong
            >{formatDate(plan.identity.provenance.retrieved_at)}</strong
          >
        </article>
        <article>
          <span>Alternates</span>
          {#if atlasPlan?.points.some((point) => point.kind === "alternate")}
            <div class="dispatch-alternate-links">
              {#each atlasPlan.points.filter((point) => point.kind === "alternate") as alternate}
                <button
                  type="button"
                  class:dispatch-map-unavailable={!alternate.location}
                  onclick={() => onviewatlas(alternate.id)}
                  >{alternate.label}</button
                >
              {/each}
            </div>
          {:else}
            <strong>None supplied</strong>
          {/if}
        </article>
        <article>
          <span>OnAir aircraft</span><strong
            >{comparison?.matched_aircraft?.registration ??
              "No deterministic match"}</strong
          ><small
            >{comparison?.matched_aircraft?.basis?.replaceAll("_", " ") ??
              "Comparison is evidence-bound"}</small
          >
        </article>
        <article>
          <span>Airport weather</span><strong
            >{status.weather.availability === "ready"
              ? `${weather?.airports.length ?? 0} stations`
              : "Not requested"}</strong
          ><small>{status.weather.cache} session cache</small>
        </article>
      </div>

      <div class="dispatch-clearance">
        <span>Interpretation</span>
        <strong>Planning assistance only</strong>
        <p>
          Imported values remain attributed to SimBrief. WyrmGrid has not
          certified this plan for real-world navigation or regulatory dispatch.
        </p>
      </div>

      <button
        class="dispatch-secondary-action"
        type="button"
        disabled={busy}
        onclick={onclear}>Clear session plan</button
      >
    {:else}
      <h2>No plan selected</h2>
      <p>Import a plan to inspect its source and freshness.</p>
      <div class="dispatch-radar" aria-hidden="true">
        <span></span><span></span><i></i>
      </div>
    {/if}
  </aside>
</section>
