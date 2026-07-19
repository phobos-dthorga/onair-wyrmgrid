<script lang="ts">
  import "./dispatch.css";
  import FlightOperationCard from "$lib/flightOperation/FlightOperationCard.svelte";
  import FlightOperationJourney from "$lib/flightOperation/FlightOperationJourney.svelte";
  import { translation } from "$lib/i18n/runtime";
  import {
    formatLocalDateTime,
    mediumDateShortTime,
    shortClockTime,
  } from "$lib/presentation/dateTime";
  import type {
    DispatchStatus,
    Mass,
    RouteWeatherAvailability,
    SimBriefAccountPreference,
    SimBriefReferenceKind,
  } from "$lib/dispatch/types";
  import type { FlightOperationStage } from "$lib/flightOperation/types";
  import type { GlobalWeatherCondition } from "$lib/forge/types";
  import { dispatchFindingMessageKeys } from "./localization";

  let {
    status,
    busy = false,
    errorMessage = "",
    accountPreference,
    onimport,
    onweather,
    onclear,
    onoperation,
    onjourney,
    onviewweatheratlas,
    onviewroute,
    onviewrouteweather,
    onviewfeature,
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
    onoperation: (action: "start" | "revise") => void;
    onjourney: (stage: FlightOperationStage) => void;
    onviewweatheratlas: (stationId?: string) => void;
    onviewroute: () => void;
    onviewrouteweather: () => void;
    onviewfeature: (featureId: string) => void;
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
  const atlasWeather = $derived(status.atlas_weather);
  const routeWeather = $derived(status.route_weather);

  function routeWeatherConditionLabel(
    condition: GlobalWeatherCondition,
  ): string {
    switch (condition) {
      case "clear":
        return $translation("dispatch-route-weather-condition-clear");
      case "cloud":
        return $translation("dispatch-route-weather-condition-cloud");
      case "rain":
        return $translation("dispatch-route-weather-condition-rain");
      case "snow":
        return $translation("dispatch-route-weather-condition-snow");
      case "convective":
        return $translation("dispatch-route-weather-condition-convective");
      case "obscuration":
        return $translation("dispatch-route-weather-condition-obscuration");
      case "unknown":
        return $translation("dispatch-route-weather-condition-unknown");
    }
  }

  function routeWeatherAvailabilityLabel(
    availability: RouteWeatherAvailability,
  ): string {
    switch (availability) {
      case "ready":
        return $translation("dispatch-route-weather-availability-ready");
      case "partial":
        return $translation("dispatch-route-weather-availability-partial");
      case "route_unavailable":
        return $translation(
          "dispatch-route-weather-availability-route-unavailable",
        );
      case "source_unavailable":
        return $translation(
          "dispatch-route-weather-availability-source-unavailable",
        );
    }
  }
  const operation = $derived(status.operation);
  function atlasWeatherStationId(stationIcao: string): string | undefined {
    return atlasWeather?.stations.find(
      (station) => station.station_icao === stationIcao,
    )?.id;
  }

  function scrollWeatherIntoView(): void {
    document
      .getElementById("dispatch-weather")
      ?.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  function openJourneyStage(stage: FlightOperationStage): void {
    if (stage === "weather") {
      scrollWeatherIntoView();
      return;
    }
    if (stage === "manifest" || stage === "review") {
      document
        .getElementById("dispatch-operation")
        ?.scrollIntoView({ behavior: "smooth", block: "start" });
      return;
    }
    onjourney(stage);
  }

  const atlasRoute = $derived(status.atlas_route);
  const originRouteFeature = $derived(
    atlasRoute?.features.find((feature) => feature.kind === "origin"),
  );
  const destinationRouteFeature = $derived(
    atlasRoute?.features.find((feature) => feature.kind === "destination"),
  );
  const alternateRouteFeatures = $derived(
    atlasRoute?.features.filter((feature) => feature.kind === "alternate") ??
      [],
  );

  function formatDate(value: string | undefined): string {
    return formatLocalDateTime(value, "Not supplied", mediumDateShortTime);
  }

  function formatTime(value: string | undefined): string {
    return formatLocalDateTime(value, "—", shortClockTime);
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

  function routeFeatureForSequence(sequence: number) {
    return atlasRoute?.features.find(
      (feature) =>
        feature.kind === "route_fix" && feature.sequence === sequence,
    );
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
      <article class="dispatch-selected-job responsive-surface">
        <span class="eyebrow">Selected OnAir job</span>
        <strong>{selectedJob.mission_type ?? "Pending job"}</strong>
        <span>{selectedJobRoute()}</span>
        <small>
          Read-only Hoard observation · {status.selected_job?.availability ??
            "unavailable"}
        </small>
        <button type="button" onclick={() => onjourney("jobs")}
          >Choose another job</button
        >
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
            ? "The reference is retained locally; a plan is persisted only when accepted into an operation or associated with a recording."
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
          class="dispatch-airport dispatch-atlas-link responsive-surface"
          type="button"
          disabled={!originRouteFeature}
          onclick={() =>
            originRouteFeature && onviewfeature(originRouteFeature.id)}
        >
          <span>Origin</span>
          <strong>{airports.origin.icao}</strong>
          <small>{airports.origin.name ?? "Name not supplied"}</small>
          <i>{airports.origin.planned_runway ?? "RWY —"}</i>
        </button>
        <button
          class="dispatch-route-line dispatch-atlas-link responsive-surface"
          type="button"
          disabled={!atlasRoute}
          aria-label="View the complete Dispatch route in Atlas"
          onclick={onviewroute}
        >
          <span></span><b>✦</b><span></span>
          <small>{route?.distance_nm?.toFixed(0) ?? "—"} NM</small>
        </button>
        <button
          class="dispatch-airport dispatch-airport-arrival dispatch-atlas-link responsive-surface"
          type="button"
          disabled={!destinationRouteFeature}
          onclick={() =>
            destinationRouteFeature &&
            onviewfeature(destinationRouteFeature.id)}
        >
          <span>Destination</span>
          <strong>{airports.destination.icao}</strong>
          <small>{airports.destination.name ?? "Name not supplied"}</small>
          <i>{airports.destination.planned_runway ?? "RWY —"}</i>
        </button>
      </header>

      <div class="dispatch-summary-strip">
        <article class="responsive-surface">
          <span>Aircraft</span><strong>{aircraft?.icao_type ?? "—"}</strong
          ><small
            >{aircraft?.registration ??
              aircraft?.model ??
              "Not supplied"}</small
          >
        </article>
        <article class="responsive-surface">
          <span>Block out</span><strong
            >{formatTime(schedule?.scheduled_out)}</strong
          ><small>{formatDate(schedule?.scheduled_out)}</small>
        </article>
        <article class="responsive-surface">
          <span>Enroute</span><strong
            >{formatDuration(schedule?.estimated_enroute_seconds)}</strong
          ><small>Planned duration</small>
        </article>
        <article class="responsive-surface">
          <span>Initial level</span><strong
            >{route?.initial_altitude_ft
              ? `FL${Math.round(route.initial_altitude_ft / 100)}`
              : "—"}</strong
          ><small>SimBrief calculation</small>
        </article>
        <article class="responsive-surface">
          <span>AIRAC</span><strong>{plan.identity.value.airac ?? "—"}</strong
          ><small>Provider revision</small>
        </article>
      </div>

      {#if routeWeather}
        <article class="dispatch-card dispatch-route-weather-card">
          <div class="dispatch-card-heading">
            <div>
              <span class="eyebrow"
                >{$translation("dispatch-route-weather-eyebrow")}</span
              >
              <h3>{$translation("dispatch-route-weather-title")}</h3>
            </div>
            <strong
              >{routeWeatherAvailabilityLabel(
                routeWeather.availability,
              )}</strong
            >
          </div>
          <p class="dispatch-card-intro">
            {$translation("dispatch-route-weather-intro", {
              interval: routeWeather.sample_interval_nm,
            })}
          </p>
          <div class="dispatch-weather-actions">
            <button
              class="dispatch-inline-action"
              type="button"
              disabled={!status.atlas_route}
              onclick={onviewrouteweather}
            >
              {$translation("dispatch-route-weather-view-atlas")}
            </button>
          </div>

          {#if routeWeather.layers.length > 0}
            {#each routeWeather.layers as layer}
              <section class="dispatch-route-weather-layer responsive-surface">
                <header>
                  <div>
                    <strong>{layer.title}</strong>
                    <span>{layer.provenance.provider}</span>
                  </div>
                  <div>
                    <span
                      >{$translation("dispatch-route-weather-model-time")}</span
                    >
                    <strong
                      >{formatDate(
                        layer.provenance.generated_at ??
                          layer.provenance.retrieved_at,
                      )}</strong
                    >
                  </div>
                </header>
                <ol class="dispatch-route-weather-samples">
                  {#each layer.samples as sample}
                    <li class:unavailable={!sample.source}>
                      <span
                        >{Math.round(sample.distance_from_origin_nm)} nm</span
                      >
                      {#if sample.source}
                        <strong
                          >{routeWeatherConditionLabel(
                            sample.source.condition,
                          )}</strong
                        >
                        <small>
                          {$translation("dispatch-route-weather-metrics", {
                            temperature: sample.source.temperature_c ?? "—",
                            precipitation:
                              sample.source.precipitation_mm ?? "—",
                            wind: sample.source.wind_speed_kt ?? "—",
                          })}
                        </small>
                        <em>
                          {$translation("dispatch-route-weather-support", {
                            distance: Math.round(
                              sample.source.support_distance_nm,
                            ),
                          })}
                        </em>
                      {:else}
                        <strong
                          >{$translation(
                            "dispatch-route-weather-no-nearby-sample",
                          )}</strong
                        >
                        <small>
                          {$translation(
                            "dispatch-route-weather-support-limit",
                            {
                              distance:
                                routeWeather.maximum_support_distance_nm,
                            },
                          )}
                        </small>
                      {/if}
                    </li>
                  {/each}
                </ol>
              </section>
            {/each}
          {:else}
            <div class="dispatch-weather-prompt">
              <strong
                >{$translation("dispatch-route-weather-no-layer-title")}</strong
              >
              <span>
                {$translation("dispatch-route-weather-no-layer-detail")}
              </span>
            </div>
          {/if}
        </article>
      {/if}

      <FlightOperationJourney
        journey={status.journey}
        onstage={openJourneyStage}
      />

      <FlightOperationCard
        {operation}
        operationChange={status.operation_change}
        jobSelection={status.selected_job}
        {busy}
        {onoperation}
      />

      <div class="dispatch-plan-grid">
        <article class="dispatch-card dispatch-route-card responsive-surface">
          <div class="dispatch-card-heading">
            <div>
              <span class="eyebrow">Route spine</span>
              <h3>Filed route</h3>
            </div>
            <div class="dispatch-route-actions">
              <strong>{route?.legs.length ?? 0} fixes</strong>
              <button type="button" disabled={!atlasRoute} onclick={onviewroute}
                >View full route</button
              >
            </div>
          </div>
          <p class="dispatch-route-text">
            {route?.source_text ?? "Route text was not supplied."}
          </p>
          {#if route?.legs.length}
            <ol class="dispatch-fix-list">
              {#each route.legs.slice(0, 12) as leg}
                {@const atlasFeature = routeFeatureForSequence(leg.sequence)}
                <li>
                  <button
                    type="button"
                    disabled={!atlasFeature}
                    aria-label={`${leg.ident}${atlasFeature?.availability === "location_unavailable" ? ", location unavailable" : ", view in Atlas"}`}
                    onclick={() =>
                      atlasFeature && onviewfeature(atlasFeature.id)}
                  >
                    <span>{leg.sequence + 1}</span><strong>{leg.ident}</strong>
                    <small>{leg.airway ?? "DCT"}</small>
                    <em
                      class:unavailable={atlasFeature?.availability ===
                        "location_unavailable"}
                    >
                      {atlasFeature?.availability === "resolved"
                        ? "Atlas"
                        : "Location unavailable"}
                    </em>
                  </button>
                </li>
              {/each}
            </ol>
            {#if route.legs.length > 12}<small class="dispatch-muted"
                >+ {route.legs.length - 12} additional fixes retained in the snapshot</small
              >{/if}
          {/if}
        </article>

        <article class="dispatch-card responsive-surface">
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

        <article class="dispatch-card responsive-surface">
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
                {@const messageKeys =
                  dispatchFindingMessageKeys[finding.message_key]}
                <li
                  class={`dispatch-finding-${finding.status} responsive-surface`}
                >
                  <div class="dispatch-finding-heading">
                    <span>{formatFindingCategory(finding.category)}</span>
                    <b>{finding.status}</b>
                  </div>
                  <strong
                    >{messageKeys
                      ? $translation(messageKeys.title, {}, finding.title)
                      : finding.title}</strong
                  >
                  <p>
                    {messageKeys
                      ? $translation(
                          messageKeys.explanation,
                          {},
                          finding.explanation,
                        )
                      : finding.explanation}
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
          id="dispatch-weather"
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
          <div class="dispatch-weather-actions">
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
            {#if atlasWeather}
              <button
                class="dispatch-inline-action"
                type="button"
                onclick={() => onviewweatheratlas()}
              >
                View weather in Atlas
              </button>
            {/if}
          </div>

          {#if weather}
            <div class="dispatch-weather-grid">
              {#each weather.airports as airport}
                <section class="dispatch-weather-station responsive-surface">
                  <header>
                    <strong>{airport.station_icao}</strong>
                    <div class="dispatch-weather-station-actions">
                      <button
                        type="button"
                        disabled={!atlasWeatherStationId(airport.station_icao)}
                        onclick={() =>
                          onviewweatheratlas(
                            atlasWeatherStationId(airport.station_icao),
                          )}
                      >
                        Atlas
                      </button>
                      <span
                        class={`dispatch-flight-category dispatch-flight-category-${airport.metar?.value.flight_category ?? "unknown"}`}
                      >
                        {airport.metar?.value.flight_category?.toUpperCase() ??
                          "NO METAR"}
                      </span>
                    </div>
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
    {:else if operation}
      <div class="dispatch-persisted-operation">
        <FlightOperationJourney
          journey={status.journey}
          onstage={openJourneyStage}
        />
        <FlightOperationCard
          {operation}
          operationChange={status.operation_change}
          jobSelection={status.selected_job}
          {busy}
          {onoperation}
        />
        <p>
          This accepted revision remains available after restart. Import a
          current plan to compare or revise it; the stored revision is not
          silently replaced.
        </p>
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
        <article class="responsive-surface">
          <span>Provider</span><strong>SimBrief</strong><small
            >Read-only latest OFP</small
          >
        </article>
        <article class="responsive-surface">
          <span>Generated</span><strong
            >{formatDate(plan.identity.provenance.generated_at)}</strong
          >
        </article>
        <article class="responsive-surface">
          <span>Retrieved</span><strong
            >{formatDate(plan.identity.provenance.retrieved_at)}</strong
          >
        </article>
        <article class="responsive-surface">
          <span>Alternates</span>
          {#if alternateRouteFeatures.length}
            <div class="dispatch-alternate-links">
              {#each alternateRouteFeatures as alternate}
                <button
                  type="button"
                  onclick={() => onviewfeature(alternate.id)}
                  >{alternate.ident}</button
                >
              {/each}
            </div>
          {:else}
            <strong>None supplied</strong>
          {/if}
        </article>
        <article class="responsive-surface">
          <span>OnAir aircraft</span><strong
            >{comparison?.matched_aircraft?.registration ??
              "No deterministic match"}</strong
          ><small
            >{comparison?.matched_aircraft?.basis?.replaceAll("_", " ") ??
              "Comparison is evidence-bound"}</small
          >
        </article>
        <article class="responsive-surface">
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
