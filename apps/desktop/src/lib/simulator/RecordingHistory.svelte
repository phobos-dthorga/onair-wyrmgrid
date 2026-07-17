<script lang="ts">
  import WyrmChart from "$lib/charts/WyrmChart.svelte";
  import ExplorationSummary from "$lib/exploration/ExplorationSummary.svelte";
  import ExplorationTabs from "$lib/exploration/ExplorationTabs.svelte";
  import { translation } from "$lib/i18n/runtime";
  import { formatLocalDateTime } from "$lib/presentation/dateTime";
  import type { DisplayPreferences } from "$lib/settings/types";
  import {
    presentAltitude,
    presentFuel,
    type PresentedMeasurement,
  } from "$lib/settings/units";
  import { altitudeRecordingChart, speedRecordingChart } from "./recordingCharts";
  import {
    activeRecordingFilterCount,
    defaultRecordingFilters,
    filterAndSortRecordings,
    recordingFilterOptions,
    type RecordingFilters,
  } from "./recordingPresentation";
  import type { SimulatorRecordingView, SimulatorSessionView } from "./types";

  let {
    status,
    session,
    displayPreferences,
    busy = false,
    errorMessage = "",
    captureControls = false,
    canStart = false,
    onstart = () => {},
    onstop = () => {},
    onsessionselect,
    onsessiondelete,
    ondeleteall,
    onpin = () => {},
    onpage = () => {},
    onexport = () => {},
  }: {
    status: SimulatorRecordingView;
    session?: SimulatorSessionView;
    displayPreferences: DisplayPreferences;
    busy?: boolean;
    errorMessage?: string;
    captureControls?: boolean;
    canStart?: boolean;
    onstart?: () => void;
    onstop?: () => void;
    onsessionselect: (sessionId: string) => void;
    onsessiondelete: (sessionId: string) => void;
    ondeleteall: () => void;
    onpin?: (sessionId: string, pinned: boolean) => void;
    onpage?: (sessionId: string, sampleOffset: number) => void;
    onexport?: (sessionId: string, format: "json" | "csv") => void;
  } = $props();

  const recordingActive = $derived(Boolean(status.active_session_id));
  let filters = $state<RecordingFilters>({ ...defaultRecordingFilters });
  let detailSection = $state("graphs");
  const visibleSessions = $derived(
    filterAndSortRecordings(status.sessions, filters),
  );
  const filterOptions = $derived(recordingFilterOptions(status.sessions));
  const activeFilterCount = $derived(activeRecordingFilterCount(filters));
  const detailTabs = [
    { id: "graphs", label: "Graphs" },
    { id: "comparison", label: "Plan comparison" },
    { id: "events", label: "Events & exports" },
  ] as const;

  function resetFilters(): void {
    filters = { ...defaultRecordingFilters };
  }

  function formatTime(value: string | undefined): string {
    return formatLocalDateTime(
      value,
      value ?? $translation("simulator-value-unavailable"),
    );
  }

  function confirmDelete(sessionId: string): void {
    if (window.confirm($translation("simulator-recording-delete-confirm"))) {
      onsessiondelete(sessionId);
    }
  }

  function confirmDeleteAll(): void {
    if (window.confirm($translation("simulator-recording-delete-all-confirm"))) {
      ondeleteall();
    }
  }

  function comparisonValue(value: number | undefined, suffix: string): string {
    return value === undefined
      ? $translation("simulator-value-unavailable")
      : `${value.toLocaleString(undefined, { maximumFractionDigits: 1 })}${suffix}`;
  }

  function presentedValue(measurement: PresentedMeasurement): string {
    return measurement.value === undefined
      ? $translation("simulator-value-unavailable")
      : `${measurement.value.toLocaleString(undefined, { maximumFractionDigits: measurement.digits })} ${measurement.unit}`;
  }

  function altitudeValue(value: number | undefined): string {
    return presentedValue(
      presentAltitude(value, displayPreferences.altitude_unit),
    );
  }

  function fuelValue(value: number | undefined): string {
    return presentedValue(
      presentFuel(value, undefined, displayPreferences.fuel_unit),
    );
  }

  function matchValue(value: boolean | undefined): string {
    return value === undefined
      ? $translation("simulator-value-unavailable")
      : $translation(
          value
            ? "simulator-recording-registration-match"
            : "simulator-recording-registration-difference",
        );
  }

  const eventLabels: Record<string, string> = {
    recording_started_manually: "simulator-recording-event-manual-start",
    takeoff_confirmed: "simulator-recording-event-takeoff",
    telemetry_gap: "simulator-recording-event-gap",
    landing_settled: "simulator-recording-event-landing",
    flight_plan_associated: "simulator-recording-event-plan",
    recording_stopped_manually: "simulator-recording-event-manual-stop",
  };

  function eventLabel(eventKind: string): string {
    const key = eventLabels[eventKind];
    return key ? $translation(key) : eventKind.replaceAll("_", " ");
  }
</script>

<section class:hoard={!captureControls} class="recording-panel" aria-live="polite">
  <div class="recording-heading">
    <div>
      <span class="eyebrow">
        {$translation(
          captureControls ? "simulator-recording-eyebrow" : "hoard-recording-eyebrow",
        )}
      </span>
      <h3>
        {$translation(
          captureControls ? "simulator-recording-title" : "hoard-recording-title",
        )}
      </h3>
      <p>
        {$translation(
          captureControls ? "simulator-recording-detail" : "hoard-recording-detail",
          { days: status.preferences.retention_days },
        )}
      </p>
    </div>
    {#if captureControls}
      {#if recordingActive}
        <button
          class="recording-stop"
          type="button"
          disabled={busy}
          onclick={onstop}
        >{$translation("simulator-recording-stop")}</button>
      {:else}
        <button type="button" disabled={busy || !canStart} onclick={onstart}
          >{$translation("simulator-recording-start")}</button
        >
      {/if}
    {/if}
  </div>

  {#if status.last_code}
    <p class="recording-notice">
      {$translation(
        status.last_code === "recording.aircraft_changed"
          ? "simulator-recording-aircraft-changed"
          : "simulator-recording-storage-failed",
      )}
    </p>
  {/if}
  {#if errorMessage}
    <p class="recording-notice" role="alert">{errorMessage}</p>
  {/if}

  <div class="recording-history-heading">
    <strong>{$translation("simulator-recording-history")}</strong>
    {#if status.sessions.length > 0}
      <button
        class="recording-delete-all"
        type="button"
        disabled={busy || recordingActive}
        onclick={confirmDeleteAll}
      >{$translation("simulator-recording-delete-all")}</button>
    {/if}
  </div>

  {#if status.sessions.length > 0}
    <div class="recording-explorer">
      <label class="recording-search">
        <span>{$translation("simulator-recording-search")}</span>
        <input
          type="search"
          bind:value={filters.query}
          placeholder={$translation("simulator-recording-search-placeholder")}
        />
      </label>
      <details class="recording-filter-panel">
        <summary>
          <span>Filter and sort</span>
          {#if activeFilterCount > 0}<strong>{activeFilterCount} active</strong>{/if}
        </summary>
        <div class="recording-filter-grid">
          <label>
            <span>Recording status</span>
            <select bind:value={filters.status}>
              <option value="all">Any reported status</option>
              {#each filterOptions.statuses as recordingStatus}
                <option value={recordingStatus}>{recordingStatus}</option>
              {/each}
            </select>
          </label>
          <label>
            <span>Capture mode</span>
            <select bind:value={filters.captureMode}>
              <option value="all">Either capture mode</option>
              {#each filterOptions.captureModes as captureMode}
                <option value={captureMode}>{captureMode}</option>
              {/each}
            </select>
          </label>
          <label>
            <span>Flight plan</span>
            <select bind:value={filters.plan}>
              <option value="all">Either plan state</option>
              <option value="linked">Plan linked</option>
              <option value="unlinked">No linked plan</option>
            </select>
          </label>
          <label>
            <span>Pinned</span>
            <select bind:value={filters.pinned}>
              <option value="all">Either pin state</option>
              <option value="pinned">Pinned</option>
              <option value="unpinned">Not pinned</option>
            </select>
          </label>
          <label>
            <span>Order recordings by</span>
            <select bind:value={filters.sort}>
              <option value="newest">Newest first</option>
              <option value="oldest">Oldest first</option>
              <option value="samples">Sample count</option>
            </select>
          </label>
        </div>
      </details>
      <ExplorationSummary
        shown={visibleSessions.length}
        total={status.sessions.length}
        label="recordings"
        activeFilters={activeFilterCount}
        onclear={resetFilters}
      />
    </div>
  {/if}

  {#if status.sessions.length === 0}
    <p class="recording-empty">
      {$translation(
        captureControls ? "simulator-recording-empty" : "hoard-recording-empty",
      )}
    </p>
  {:else}
    <div class="recording-sessions">
      {#each visibleSessions as recording}
        <article class:active={recording.id === status.active_session_id}>
          <button
            class="recording-select"
            type="button"
            disabled={busy}
            aria-pressed={session?.session.id === recording.id}
            onclick={() => onsessionselect(recording.id)}
          >
            <strong>{recording.aircraft_registration ?? recording.aircraft_title}</strong>
            <span>
              {formatTime(recording.started_at)} · {recording.sample_count.toLocaleString()}
              {$translation("simulator-recording-samples")}
            </span>
            <small>{$translation(`simulator-recording-status-${recording.status}`)}</small>
            <small>
              {$translation(
                recording.capture_mode === "automatic"
                  ? "simulator-recording-mode-automatic"
                  : "simulator-recording-mode-manual",
              )}
              {recording.plan_associated
                ? ` · ${$translation("simulator-recording-plan-linked")}`
                : ""}
            </small>
          </button>
          <div class="recording-row-actions">
            <button
              class="recording-pin"
              class:pinned={recording.pinned}
              type="button"
              aria-label={$translation(
                recording.pinned
                  ? "simulator-recording-unpin"
                  : "simulator-recording-pin",
              )}
              disabled={busy}
              onclick={() => onpin(recording.id, !recording.pinned)}
            >{$translation(
                recording.pinned
                  ? "simulator-recording-pinned"
                  : "simulator-recording-pin",
              )}</button>
            <button
              class="recording-delete"
              type="button"
              aria-label={$translation("simulator-recording-delete")}
              disabled={busy || recording.id === status.active_session_id}
              onclick={() => confirmDelete(recording.id)}
            >×</button>
          </div>
        </article>
      {/each}
      {#if visibleSessions.length === 0}
        <p class="recording-empty">No recordings match the current filters.</p>
      {/if}
    </div>
  {/if}

  {#if session && session.samples.length > 0}
    <div class="recording-window-actions">
      <button
        type="button"
        disabled={busy || !session.has_older_samples}
        onclick={() => onpage(session.session.id, session.sample_window_offset + session.sample_window_limit)}
      >{$translation("simulator-recording-older-samples")}</button>
      <span>
        {$translation("simulator-recording-window", {
          first: (session.sample_window_offset + 1).toLocaleString(),
          last: (
            session.sample_window_offset + session.samples.length
          ).toLocaleString(),
        })}
      </span>
      <button
        type="button"
        disabled={busy || !session.has_newer_samples}
        onclick={() => onpage(session.session.id, Math.max(0, session.sample_window_offset - session.sample_window_limit))}
      >{$translation("simulator-recording-newer-samples")}</button>
    </div>
    <div class="recording-detail-tabs">
      <ExplorationTabs
        tabs={detailTabs}
        bind:selected={detailSection}
        label="Recording detail sections"
        idPrefix="recording"
      />
    </div>

    {#if detailSection === "graphs"}
      <div id="recording-panel-graphs" class="recording-charts" role="tabpanel">
        <WyrmChart
          spec={altitudeRecordingChart(session, displayPreferences)}
          eyebrow="WyrmChart telemetry"
          height="210px"
        />
        <WyrmChart
          spec={speedRecordingChart(session, displayPreferences)}
          eyebrow="WyrmChart telemetry"
          height="210px"
        />
      </div>
    {:else if detailSection === "comparison"}
      {#if session.comparison}
      <section id="recording-panel-comparison" class="plan-comparison" role="tabpanel">
        <div class="comparison-heading">
          <div>
            <span class="eyebrow">{$translation("simulator-recording-comparison-version", { version: session.comparison.association.correlation_version })}</span>
            <h4>{session.comparison.association.origin_icao} → {session.comparison.association.destination_icao}</h4>
          </div>
          <small>{$translation("simulator-recording-comparison-boundary")}</small>
        </div>
        <div class="comparison-grid">
          <article><span>{$translation("simulator-recording-comparison-time")}</span><strong>{comparisonValue(session.comparison.planned_enroute_seconds, " s")} / {comparisonValue(session.comparison.recorded_seconds, " s")}</strong></article>
          <article><span>{$translation("simulator-recording-comparison-distance")}</span><strong>{comparisonValue(session.comparison.planned_distance_nm, " nm")} / {comparisonValue(session.comparison.recorded_track_distance_nm, " nm")}</strong></article>
          <article><span>{$translation("simulator-recording-comparison-altitude")}</span><strong>{altitudeValue(session.comparison.planned_initial_altitude_ft)} / {altitudeValue(session.comparison.recorded_peak_altitude_ft)}</strong></article>
          <article><span>{$translation("simulator-recording-comparison-fuel")}</span><strong>{fuelValue(session.comparison.planned_takeoff_fuel_pounds)} / {fuelValue(session.comparison.recorded_fuel_used_pounds)}</strong></article>
          <article><span>{$translation("simulator-recording-comparison-origin")}</span><strong>{comparisonValue(session.comparison.origin_proximity_nm, " nm")}</strong></article>
          <article><span>{$translation("simulator-recording-comparison-destination")}</span><strong>{comparisonValue(session.comparison.destination_proximity_nm, " nm")}</strong></article>
          <article><span>{$translation("simulator-recording-comparison-registration")}</span><strong>{matchValue(session.comparison.registration_matches)}</strong></article>
        </div>
        {#if !session.comparison.analysis_complete}
          <p>{$translation("simulator-recording-analysis-withheld")}</p>
        {/if}
      </section>
      {:else}
        <div
          id="recording-panel-comparison"
          class="recording-empty"
          role="tabpanel"
        >
          No SimBrief plan is associated with this recording.
        </div>
      {/if}
    {:else}
    <section id="recording-panel-events" class="recording-evidence" role="tabpanel">
      <div class="comparison-heading">
        <div>
          <span class="eyebrow">{$translation("simulator-recording-evidence-eyebrow")}</span>
          <h4>{$translation("simulator-recording-events-title")}</h4>
        </div>
        <div class="export-actions">
          <button type="button" disabled={busy} onclick={() => onexport(session.session.id, "json")}>{$translation("simulator-recording-export-json")}</button>
          <button type="button" disabled={busy} onclick={() => onexport(session.session.id, "csv")}>{$translation("simulator-recording-export-csv")}</button>
        </div>
      </div>
      {#if session.events.length === 0}
        <p>{$translation("simulator-recording-events-empty")}</p>
      {:else}
        <ol>
          {#each session.events as event}
            <li><strong>{eventLabel(event.event_kind)}</strong><time>{formatTime(event.observed_at)}</time></li>
          {/each}
        </ol>
      {/if}
    </section>
    {/if}
  {/if}
</section>

<style>
  .recording-panel {
    margin: 0 24px 22px;
    padding: 18px;
    border: 1px solid var(--color-line-faint);
    border-radius: 6px;
    background: var(--color-surface-soft);
  }
  .recording-panel.hoard {
    margin: 0;
  }
  .recording-heading,
  .recording-history-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 18px;
  }
  .recording-heading h3 {
    margin: 4px 0;
  }
  .recording-heading p,
  .recording-empty,
  .recording-notice {
    margin: 5px 0 0;
    color: var(--color-text-muted);
    font-size: 11px;
    line-height: 1.5;
  }
  button {
    flex: 0 0 auto;
    border: 1px solid var(--color-accent-border);
    border-radius: 4px;
    padding: 8px 12px;
    color: var(--color-canvas);
    background: var(--color-accent);
    cursor: pointer;
  }
  button:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
  button.recording-stop,
  button.recording-delete-all,
  button.recording-delete {
    border-color: var(--color-danger-border);
    color: var(--color-danger);
    background: var(--color-danger-soft);
  }
  .recording-notice {
    padding: 10px;
    border: 1px solid var(--color-danger-border);
    color: var(--color-danger);
    background: var(--color-danger-soft);
  }
  .recording-history-heading {
    align-items: center;
    margin-top: 18px;
    padding-top: 14px;
    border-top: 1px solid var(--color-line-faint);
    font-size: 12px;
  }
  .recording-history-heading button {
    padding: 6px 9px;
    font-size: 10px;
  }
  .recording-sessions {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    margin-top: 10px;
  }
  .recording-search {
    display: grid;
    gap: 5px;
    margin-top: 10px;
    color: var(--color-text-muted);
    font-size: 9px;
  }
  .recording-search input {
    border: 1px solid var(--color-line-faint);
    border-radius: 4px;
    padding: 8px 10px;
    color: var(--color-text);
    background: var(--color-surface);
  }
  .recording-explorer {
    display: grid;
    gap: 9px;
  }
  .recording-filter-panel {
    border: 1px solid var(--color-line-faint);
    background: var(--color-surface);
  }
  .recording-filter-panel summary {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 10px;
    color: var(--color-text-muted);
    font-size: 9px;
    cursor: pointer;
  }
  .recording-filter-panel summary strong {
    color: var(--color-accent);
  }
  .recording-filter-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
    padding: 0 10px 10px;
  }
  .recording-filter-grid label {
    display: grid;
    gap: 4px;
    color: var(--color-text-muted);
    font-size: 9px;
  }
  .recording-filter-grid select {
    min-width: 0;
    border: 1px solid var(--color-line-faint);
    padding: 7px;
    color: var(--color-text);
    background: var(--color-surface-soft);
  }
  .recording-sessions article {
    display: grid;
    grid-template-columns: 1fr auto;
    min-width: 0;
    border: 1px solid var(--color-line-faint);
    background: var(--color-surface);
  }
  .recording-sessions article.active,
  .recording-sessions article:has(.recording-select[aria-pressed="true"]) {
    border-color: var(--color-accent-border);
  }
  button.recording-select {
    display: grid;
    min-width: 0;
    gap: 4px;
    border: 0;
    padding: 10px;
    color: var(--color-text);
    background: transparent;
    text-align: left;
  }
  .recording-select span,
  .recording-select small {
    overflow: hidden;
    color: var(--color-text-muted);
    font-size: 9px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  button.recording-delete {
    align-self: center;
    margin-right: 8px;
    padding: 3px 7px;
  }
  .recording-row-actions {
    display: grid;
    align-content: center;
    gap: 5px;
    padding-right: 8px;
  }
  button.recording-pin,
  .recording-window-actions button,
  .export-actions button {
    border-color: var(--color-line-soft);
    padding: 5px 7px;
    color: var(--color-text-muted);
    background: transparent;
    font-size: 9px;
  }
  button.recording-pin.pinned {
    border-color: var(--color-highlight-border);
    color: var(--color-highlight);
    background: var(--color-highlight-soft);
  }
  .recording-window-actions,
  .comparison-heading,
  .export-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .recording-window-actions {
    margin-top: 14px;
    color: var(--color-text-muted);
    font-size: 9px;
  }
  .recording-detail-tabs {
    margin-top: 12px;
  }
  .recording-charts {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 18px;
    margin-top: 8px;
  }
  .plan-comparison,
  .recording-evidence {
    margin-top: 14px;
    padding: 14px;
    border: 1px solid var(--color-line-faint);
    background: var(--color-surface);
  }
  .comparison-heading h4 {
    margin: 4px 0 0;
  }
  .comparison-heading small,
  .plan-comparison p,
  .recording-evidence p {
    color: var(--color-text-muted);
    font-size: 9px;
  }
  .comparison-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 7px;
    margin-top: 10px;
  }
  .comparison-grid article {
    display: grid;
    gap: 4px;
    padding: 9px;
    background: var(--color-surface-soft);
  }
  .comparison-grid span,
  .recording-evidence time {
    color: var(--color-text-muted);
    font-size: 9px;
  }
  .recording-evidence ol {
    display: grid;
    gap: 4px;
    margin: 10px 0 0;
    padding: 0;
    list-style: none;
  }
  .recording-evidence li {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    padding: 7px 9px;
    background: var(--color-surface-soft);
    font-size: 9px;
    text-transform: capitalize;
  }
  @media (max-width: 760px) {
    .recording-sessions,
    .recording-charts,
    .recording-filter-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
