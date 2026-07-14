<script lang="ts">
  import WyrmChart from "$lib/charts/WyrmChart.svelte";
  import { translation } from "$lib/i18n/runtime";
  import type { DisplayPreferences } from "$lib/settings/types";
  import { altitudeRecordingChart, speedRecordingChart } from "./recordingCharts";
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
  } = $props();

  const recordingActive = $derived(Boolean(status.active_session_id));

  function formatTime(value: string | undefined): string {
    if (!value) return $translation("simulator-value-unavailable");
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
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

  {#if status.sessions.length === 0}
    <p class="recording-empty">
      {$translation(
        captureControls ? "simulator-recording-empty" : "hoard-recording-empty",
      )}
    </p>
  {:else}
    <div class="recording-sessions">
      {#each status.sessions as recording}
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
          </button>
          <button
            class="recording-delete"
            type="button"
            aria-label={$translation("simulator-recording-delete")}
            disabled={busy || recording.id === status.active_session_id}
            onclick={() => confirmDelete(recording.id)}
          >×</button>
        </article>
      {/each}
    </div>
  {/if}

  {#if session && session.samples.length > 0}
    <div class="recording-charts">
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
  .recording-charts {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 18px;
    margin-top: 8px;
  }
  @media (max-width: 760px) {
    .recording-sessions,
    .recording-charts {
      grid-template-columns: 1fr;
    }
  }
</style>
