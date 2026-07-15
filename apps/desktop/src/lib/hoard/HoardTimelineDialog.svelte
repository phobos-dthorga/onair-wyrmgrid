<script lang="ts">
  import WyrmChart from "$lib/charts/WyrmChart.svelte";
  import type { AtlasFlightRoute } from "$lib/atlas/types";
  import type { ChartSpec } from "$lib/charts/types";
  import type { DisplayPreferences } from "$lib/settings/types";
  import RecordingHistory from "$lib/simulator/RecordingHistory.svelte";
  import type {
    SimulatorRecordingView,
    SimulatorSessionDebrief,
    SimulatorSessionView,
  } from "$lib/simulator/types";
  import type { HoardTimelineIndex, TimelineMode } from "./types";

  let {
    open,
    mode,
    timeline,
    cursor,
    growthChart,
    fboGrowthChart,
    compositionChart,
    displayPreferences,
    recordingStatus,
    recordingSession,
    recordingDebrief,
    recordingBusy,
    recordingError,
    busy,
    errorMessage,
    oncursorchange,
    onview,
    onreturn,
    onrecordingselect,
    onrecordingdelete,
    onrecordingdeleteall,
    onrecordingpin,
    onrecordingpage,
    onrecordingexport,
    onrecordingviewatlas,
    onclose,
  }: {
    open: boolean;
    mode: TimelineMode;
    timeline: HoardTimelineIndex;
    cursor: number;
    growthChart: ChartSpec | null;
    fboGrowthChart: ChartSpec | null;
    compositionChart: ChartSpec | null;
    displayPreferences: DisplayPreferences;
    recordingStatus: SimulatorRecordingView;
    recordingSession?: SimulatorSessionView;
    recordingDebrief?: SimulatorSessionDebrief;
    recordingBusy: boolean;
    recordingError: string;
    busy: boolean;
    errorMessage: string;
    oncursorchange: (cursor: number) => void;
    onview: () => void;
    onreturn: () => void;
    onrecordingselect: (sessionId: string) => void;
    onrecordingdelete: (sessionId: string) => void;
    onrecordingdeleteall: () => void;
    onrecordingpin: (sessionId: string, pinned: boolean) => void;
    onrecordingpage: (sessionId: string, sampleOffset: number) => void;
    onrecordingexport: (sessionId: string, format: "json" | "csv") => void;
    onrecordingviewatlas: (route: AtlasFlightRoute) => void;
    onclose: () => void;
  } = $props();

  let activeSection = $state<"company" | "recordings">("company");
  const selectedAt = $derived(timeline.observation_times[cursor]);

  function formatTime(value: string | undefined): string {
    if (!value) return "No retained observation";
    const parsed = new Date(value);
    return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString();
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape" && !busy) onclose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="dialog-backdrop">
    <div
      class="timeline-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="timeline-title"
    >
      <header>
        <div>
          <span class="eyebrow">WyrmGrid Hoard</span>
          <h2 id="timeline-title">History library</h2>
          <p>
            {activeSection === "company"
              ? (timeline.company?.name ?? "No retained company")
              : "Local simulator telemetry recordings"}
          </p>
        </div>
        <div class="header-actions">
          <span class:historical={mode === "historical"} class="mode-badge">
            {mode === "historical" ? "Historical" : "Live"}
          </span>
          <button
            class="close-button"
            type="button"
            aria-label="Close Hoard Timeline"
            disabled={busy}
            onclick={onclose}>×</button
          >
        </div>
      </header>

      <nav class="history-sections" aria-label="Hoard history sections">
        <button
          type="button"
          aria-pressed={activeSection === "company"}
          onclick={() => (activeSection = "company")}>Company timeline</button
        >
        <button
          type="button"
          aria-pressed={activeSection === "recordings"}
          onclick={() => (activeSection = "recordings")}
          >Flight recordings ({recordingStatus.sessions.length})</button
        >
      </nav>

      {#if activeSection === "company"}
        <div class="timeline-content">
          <section
            class="time-control"
            aria-label="Historical observation selection"
          >
            <div class="control-heading">
              <div>
                <span>Selected observation</span>
                <strong>{formatTime(selectedAt)}</strong>
              </div>
              <small>{timeline.observation_times.length} retained moments</small
              >
            </div>

            {#if timeline.observation_times.length > 0}
              <input
                type="range"
                min="0"
                max={timeline.observation_times.length - 1}
                value={cursor}
                aria-label="Choose a retained company observation"
                oninput={(event) =>
                  oncursorchange(Number(event.currentTarget.value))}
              />
              <div class="range-labels">
                <span>{formatTime(timeline.observation_times[0])}</span>
                <span>{formatTime(timeline.observation_times.at(-1))}</span>
              </div>
              <div class="timeline-actions">
                <button
                  class="view-button"
                  type="button"
                  disabled={busy || !selectedAt}
                  onclick={onview}
                >
                  {busy ? "Opening…" : "View this moment"}
                </button>
                {#if mode === "historical"}
                  <button
                    class="return-button"
                    type="button"
                    disabled={busy}
                    onclick={onreturn}
                  >
                    Return to present
                  </button>
                {/if}
              </div>
            {:else}
              <div class="empty-state">
                <strong>The Hoard is ready.</strong>
                <span
                  >Successful fleet and FBO synchronizations will appear here
                  over time.</span
                >
              </div>
            {/if}

            {#if errorMessage}<p class="error-message" role="alert">
                {errorMessage}
              </p>{/if}
            <p class="retention-note">
              Recent observations are retained hourly for seven days, then
              daily. Historical mode changes only what Atlas displays;
              background synchronization keeps the live view ready.
            </p>
          </section>

          <section class="charts" aria-label="Hoard history charts">
            <div class="chart-slot">
              {#if growthChart}
                <WyrmChart spec={growthChart} height="180px" />
              {:else}
                <div class="chart-placeholder">
                  Fleet growth appears after the first retained fleet
                  observation.
                </div>
              {/if}
            </div>
            <div class="chart-slot">
              {#if fboGrowthChart}
                <WyrmChart spec={fboGrowthChart} height="180px" />
              {:else}
                <div class="chart-placeholder">
                  FBO growth appears after the first retained network
                  observation.
                </div>
              {/if}
            </div>
            <div class="chart-slot composition-slot">
              {#if compositionChart}
                <WyrmChart spec={compositionChart} />
              {:else}
                <div class="chart-placeholder">
                  Fleet composition is unavailable for this moment.
                </div>
              {/if}
            </div>
          </section>
        </div>
      {:else}
        <div class="recording-content">
          <RecordingHistory
            status={recordingStatus}
            session={recordingSession}
            debrief={recordingDebrief}
            {displayPreferences}
            busy={recordingBusy}
            errorMessage={recordingError}
            onsessionselect={onrecordingselect}
            onsessiondelete={onrecordingdelete}
            ondeleteall={onrecordingdeleteall}
            onpin={onrecordingpin}
            onpage={onrecordingpage}
            onexport={onrecordingexport}
            onviewatlas={onrecordingviewatlas}
          />
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: 24px;
    background: var(--color-overlay);
    backdrop-filter: blur(9px);
  }
  .timeline-dialog {
    width: min(1180px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    overflow: auto;
    border: 1px solid var(--color-line-soft);
    border-radius: 8px;
    color: var(--color-text);
    background:
      radial-gradient(
        circle at 14% 0%,
        var(--color-highlight-soft),
        transparent 30%
      ),
      var(--color-surface-elevated);
    box-shadow: 0 28px 90px var(--color-shadow);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    padding: 20px 22px;
    border-bottom: 1px solid var(--color-line-faint);
  }
  header h2 {
    margin: 5px 0 0;
    font-family: Georgia, serif;
    font-size: 25px;
    font-weight: 500;
  }
  header p {
    margin: 4px 0 0;
    color: var(--color-text-muted);
    font-size: 11px;
  }
  .header-actions,
  .timeline-actions,
  .control-heading,
  .range-labels {
    display: flex;
    align-items: center;
  }
  .header-actions {
    gap: 12px;
  }
  .mode-badge {
    padding: 5px 9px;
    border: 1px solid var(--color-accent-border);
    border-radius: 99px;
    color: var(--color-accent);
    font-size: 9px;
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .mode-badge.historical {
    border-color: var(--color-highlight-border);
    color: var(--color-highlight);
  }
  .close-button {
    border: 0;
    color: var(--color-text-muted);
    background: transparent;
    font-size: 26px;
    cursor: pointer;
  }
  .timeline-content {
    display: grid;
    grid-template-columns: minmax(280px, 0.72fr) minmax(560px, 1.65fr);
    gap: 22px;
    padding: 22px;
  }
  .history-sections {
    display: flex;
    gap: 8px;
    padding: 12px 22px 0;
  }
  .history-sections button {
    min-height: 34px;
    border: 1px solid var(--color-line-soft);
    border-radius: 4px 4px 0 0;
    padding: 0 14px;
    color: var(--color-text-muted);
    background: var(--color-surface-translucent);
    font-size: 10px;
    font-weight: 700;
    cursor: pointer;
  }
  .history-sections button[aria-pressed="true"] {
    border-color: var(--color-accent-border);
    color: var(--color-accent);
    background: var(--color-accent-soft);
  }
  .recording-content {
    padding: 22px;
  }
  .time-control {
    padding: 18px;
    border: 1px solid var(--color-line-faint);
    border-radius: 6px;
    background: var(--color-surface-translucent);
  }
  .control-heading {
    justify-content: space-between;
    gap: 12px;
  }
  .control-heading div {
    display: grid;
    gap: 5px;
  }
  .control-heading span,
  .control-heading small,
  .range-labels,
  .retention-note {
    color: var(--color-text-muted);
    font-size: 9px;
  }
  .control-heading strong {
    font-size: 13px;
  }
  input[type="range"] {
    width: 100%;
    margin: 30px 0 8px;
    accent-color: var(--color-highlight);
  }
  .range-labels {
    justify-content: space-between;
    gap: 18px;
  }
  .range-labels span:last-child {
    text-align: right;
  }
  .timeline-actions {
    gap: 9px;
    margin-top: 24px;
  }
  .view-button,
  .return-button {
    min-height: 36px;
    border-radius: 3px;
    padding: 0 13px;
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
  }
  .view-button {
    border: 1px solid var(--color-highlight-border);
    color: var(--color-canvas);
    background: var(--color-highlight);
  }
  .return-button {
    border: 1px solid var(--color-accent-border);
    color: var(--color-accent);
    background: transparent;
  }
  button:disabled {
    cursor: not-allowed;
    opacity: 0.48;
  }
  .retention-note {
    margin: 22px 0 0;
    line-height: 1.55;
  }
  .error-message {
    margin: 16px 0 0;
    padding: 9px;
    border: 1px solid var(--color-danger-border);
    color: var(--color-danger);
    background: var(--color-danger-soft);
    font-size: 10px;
  }
  .empty-state {
    display: grid;
    gap: 7px;
    margin-top: 24px;
    padding: 18px;
    border: 1px dashed var(--color-line-soft);
    color: var(--color-text-muted);
    font-size: 11px;
    line-height: 1.5;
  }
  .empty-state strong {
    color: var(--color-text);
  }
  .charts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 18px;
    align-items: start;
  }
  .chart-slot {
    min-width: 0;
  }
  .composition-slot {
    grid-column: 1 / -1;
  }
  .charts :global(.chart-card) {
    margin-top: 0;
    padding: 14px;
    border: 1px solid var(--color-line-faint);
    border-radius: 6px;
    background: var(--color-surface-translucent);
  }
  .chart-placeholder {
    display: grid;
    place-items: center;
    min-height: 260px;
    border: 1px dashed var(--color-line-soft);
    border-radius: 6px;
    padding: 22px;
    color: var(--color-text-muted);
    text-align: center;
    font-size: 11px;
    line-height: 1.5;
  }
  @media (max-width: 1150px) {
    .timeline-content {
      grid-template-columns: 1fr;
    }
  }
  @media (max-width: 720px) {
    .dialog-backdrop {
      padding: 12px;
    }
    .timeline-dialog {
      width: calc(100vw - 24px);
      max-height: calc(100vh - 24px);
    }
    .charts {
      grid-template-columns: 1fr;
    }
    .composition-slot {
      grid-column: auto;
    }
  }
</style>
