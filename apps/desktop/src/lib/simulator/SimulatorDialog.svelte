<script lang="ts">
  import { translation } from "$lib/i18n/runtime";
  import WyrmChart from "$lib/charts/WyrmChart.svelte";
  import type { DisplayPreferences } from "$lib/settings/types";
  import {
    presentAltitude,
    presentFuel,
    presentSpeed,
    presentWeight,
    type PresentedMeasurement,
  } from "$lib/settings/units";
  import "./simulator.css";
  import { altitudeRecordingChart, speedRecordingChart } from "./recordingCharts";
  import type {
    ProviderConnectionState,
    SimulatorBridgeView,
    SimulatorProviderView,
    SimulatorRecordingView,
    SimulatorSessionView,
  } from "./types";

  let {
    open,
    status,
    busy = false,
    errorMessage = "",
    displayPreferences,
    recordingStatus,
    recordingSession,
    recordingBusy = false,
    onrefresh,
    onstart,
    onstop,
    onrecordstart,
    onrecordstop,
    onsessionselect,
    onsessiondelete,
    ondeleteall,
    onclose,
  }: {
    open: boolean;
    status: SimulatorBridgeView;
    busy?: boolean;
    errorMessage?: string;
    displayPreferences: DisplayPreferences;
    recordingStatus: SimulatorRecordingView;
    recordingSession?: SimulatorSessionView;
    recordingBusy?: boolean;
    onrefresh: () => void;
    onstart: (providerId: string) => void;
    onstop: (providerId: string) => void;
    onrecordstart: () => void;
    onrecordstop: () => void;
    onsessionselect: (sessionId: string) => void;
    onsessiondelete: (sessionId: string) => void;
    ondeleteall: () => void;
    onclose: () => void;
  } = $props();

  const snapshot = $derived(status.latest_snapshot);
  const recordingActive = $derived(Boolean(recordingStatus.active_session_id));

  function stateLabel(state: ProviderConnectionState): string {
    return $translation(`simulator-state-${state.replaceAll("_", "-")}`);
  }

  const providerDetailKeys: Record<string, string> = {
    "provider.executable_unavailable":
      "error-simulator-provider-executable-unavailable",
    "provider.handshake_failed": "error-simulator-provider-handshake",
    "provider.protocol_violation": "error-simulator-provider-protocol",
    "provider.stream_closed": "error-simulator-provider-connection",
    "provider.write_failed": "error-simulator-provider-connection",
    "provider.starting": "simulator-detail-starting",
    "provider.stopped": "simulator-detail-stopped",
    "provider.unsupported_platform": "simulator-detail-unsupported-platform",
    "simconnect.client_unavailable": "error-simconnect-client-unavailable",
    "simconnect.client_load_failed": "error-simconnect-client-unavailable",
    "simconnect.waiting_for_simulator": "simulator-detail-waiting",
    "simconnect.connected": "simulator-detail-connected",
    "simconnect.disconnected": "simulator-detail-disconnected",
    "simconnect.setup_failed": "error-simulator-provider-protocol",
    "simconnect.protocol_error": "error-simulator-provider-protocol",
  };

  function providerDetail(code: string): string {
    return $translation(
      providerDetailKeys[code] ?? "simulator-detail-status-update",
    );
  }

  function processActive(provider: SimulatorProviderView): boolean {
    return ["starting", "running", "stopping"].includes(provider.process_state);
  }

  function bridgeRitualStage(provider: SimulatorProviderView): number {
    if (provider.process_state === "starting") return 1;
    if (provider.connection_state === "connected" && !provider.telemetry_stale)
      return 5;
    if (
      ["waiting_for_simulator", "disconnected"].includes(
        provider.connection_state,
      )
    )
      return 4;
    if (
      provider.process_state === "running" ||
      provider.connection_state === "starting"
    )
      return 3;
    return 0;
  }

  const ritualSteps = [
    "simulator-ritual-wake",
    "simulator-ritual-identity",
    "telemetry-simulator-ritual-capability",
    "simulator-ritual-link",
  ] as const;

  function formatNumber(value: number | undefined, digits = 0): string {
    return value === undefined || !Number.isFinite(value)
      ? $translation("simulator-value-unavailable")
      : value.toLocaleString(undefined, {
          minimumFractionDigits: digits,
          maximumFractionDigits: digits,
        });
  }

  function formatMeasurement(
    value: number | undefined,
    unit: string,
    digits = 0,
  ): string {
    if (value === undefined || !Number.isFinite(value)) {
      return $translation("simulator-value-unavailable");
    }
    const separator = unit === "°" ? "" : " ";
    return `${formatNumber(value, digits)}${separator}${unit}`;
  }

  function formatPresented(measurement: PresentedMeasurement): string {
    return formatMeasurement(
      measurement.value,
      measurement.unit,
      measurement.digits,
    );
  }

  function formatTime(value: string | undefined): string {
    if (!value) return $translation("simulator-value-unavailable");
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape" && !busy && !recordingBusy) onclose();
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

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="simulator-backdrop">
    <div
      class="simulator-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="simulator-title"
    >
      <header>
        <div>
          <span class="eyebrow">{$translation("simulator-eyebrow")}</span>
          <h2 id="simulator-title">{$translation("simulator-title")}</h2>
          <p>{$translation("telemetry-simulator-introduction")}</p>
        </div>
        <button
          class="simulator-close"
          type="button"
          aria-label={$translation("simulator-close")}
          disabled={busy}
          onclick={onclose}>×</button
        >
      </header>

      <div class="simulator-boundary">
        <strong>{$translation("telemetry-simulator-boundary-title")}</strong>
        <span>{$translation("telemetry-simulator-boundary-detail")}</span>
      </div>

      {#if errorMessage}<p class="simulator-error" role="alert">
          {errorMessage}
        </p>{/if}

      <section
        class="provider-list"
        aria-label={$translation("simulator-provider-list")}
      >
        {#each status.providers as provider}
          <article
            class:connected={provider.connection_state === "connected" &&
              !provider.telemetry_stale}
            class:stale={provider.telemetry_stale}
          >
            <div>
              <span class="provider-kind"
                >{provider.simulators.join(" · ")}</span
              >
              <h3>{provider.name}</h3>
              <small
                >{$translation("simulator-provider-version", {
                  version: provider.version,
                })}</small
              >
            </div>
            <div class="provider-state">
              <span
                >{provider.telemetry_stale
                  ? $translation("simulator-state-stale")
                  : stateLabel(provider.connection_state)}</span
              >
              {#if provider.telemetry_stale}
                <small class="provider-detail">
                  {$translation("simulator-detail-stale", {
                    seconds:
                      provider.latest_snapshot_age_seconds ??
                      provider.connected_age_seconds ??
                      0,
                  })}
                </small>
              {:else if provider.last_code}<small class="provider-detail">
                  {providerDetail(provider.last_code)}
                </small>{/if}
            </div>
            {#if bridgeRitualStage(provider) > 0}
              <div class="bridge-ritual">
                <span class="ritual-title"
                  >{$translation("simulator-ritual-title")}</span
                >
                <ol aria-label={$translation("simulator-ritual-title")}>
                  {#each ritualSteps as step, index}
                    <li
                      class:complete={bridgeRitualStage(provider) > index + 1}
                      class:current={bridgeRitualStage(provider) === index + 1}
                    >
                      <span aria-hidden="true"></span>
                      {$translation(step)}
                    </li>
                  {/each}
                </ol>
              </div>
            {/if}
            <footer>
              <span>
                {$translation("simulator-capability-summary", {
                  count: provider.capabilities.length,
                })}
              </span>
              {#if processActive(provider)}
                <button
                  class="stop"
                  type="button"
                  disabled={busy || provider.process_state === "stopping"}
                  onclick={() => onstop(provider.id)}
                >
                  {provider.process_state === "stopping"
                    ? $translation("simulator-stopping")
                    : $translation("simulator-stop")}
                </button>
              {:else}
                <button
                  type="button"
                  disabled={busy ||
                    provider.process_state === "unavailable" ||
                    Boolean(status.active_provider_id)}
                  onclick={() => onstart(provider.id)}
                >
                  {provider.process_state === "starting"
                    ? $translation("simulator-starting")
                    : $translation("simulator-start")}
                </button>
              {/if}
            </footer>
          </article>
        {:else}
          <div class="simulator-empty">
            <strong>{$translation("simulator-no-providers")}</strong>
            <span>{$translation("simulator-no-providers-detail")}</span>
          </div>
        {/each}
      </section>

      <section class="telemetry-panel" aria-live="polite">
        <div class="telemetry-heading">
          <div>
            <span class="eyebrow"
              >{$translation("telemetry-simulator-live-eyebrow")}</span
            >
            <h3>
              {snapshot?.aircraft.registration ??
                snapshot?.aircraft.title ??
                $translation("simulator-awaiting-aircraft")}
            </h3>
            {#if snapshot?.aircraft.registration}
              <small>{snapshot.aircraft.title}</small>
            {/if}
          </div>
          <span class:live={Boolean(snapshot)} class="live-badge">
            {snapshot
              ? $translation("simulator-live")
              : $translation("simulator-awaiting-telemetry")}
          </span>
        </div>

        <dl>
          <div>
            <dt>{$translation("simulator-position")}</dt>
            <dd>
              {snapshot
                ? `${snapshot.position.latitude.toFixed(4)}, ${snapshot.position.longitude.toFixed(4)}`
                : $translation("simulator-value-unavailable")}
            </dd>
          </div>
          <div>
            <dt>{$translation("simulator-altitude")}</dt>
            <dd>
              {formatPresented(
                presentAltitude(
                  snapshot?.altitude_feet,
                  displayPreferences.altitude_unit,
                ),
              )}
            </dd>
          </div>
          <div>
            <dt>{$translation("simulator-heading")}</dt>
            <dd>{formatMeasurement(snapshot?.true_heading_degrees, "°")}</dd>
          </div>
          <div>
            <dt>{$translation("simulator-ground-speed")}</dt>
            <dd>
              {formatPresented(
                presentSpeed(
                  snapshot?.ground_speed_knots,
                  displayPreferences.speed_unit,
                ),
              )}
            </dd>
          </div>
          <div>
            <dt>{$translation("simulator-indicated-speed")}</dt>
            <dd>
              {formatPresented(
                presentSpeed(
                  snapshot?.indicated_airspeed_knots,
                  displayPreferences.speed_unit,
                ),
              )}
            </dd>
          </div>
          <div>
            <dt>{$translation("simulator-fuel-weight")}</dt>
            <dd>
              {formatPresented(
                presentFuel(
                  snapshot?.fuel_total_weight_pounds,
                  snapshot?.fuel_total_gallons,
                  displayPreferences.fuel_unit,
                ),
              )}
            </dd>
          </div>
          <div>
            <dt>{$translation("simulator-gross-weight")}</dt>
            <dd>
              {formatPresented(
                presentWeight(
                  snapshot?.gross_weight_pounds,
                  displayPreferences.weight_unit,
                ),
              )}
            </dd>
          </div>
          <div>
            <dt>{$translation("simulator-observed-at")}</dt>
            <dd>{formatTime(snapshot?.provenance.retrieved_at)}</dd>
          </div>
        </dl>
      </section>

      <section class="recording-panel" aria-live="polite">
        <div class="recording-heading">
          <div>
            <span class="eyebrow">{$translation("simulator-recording-eyebrow")}</span>
            <h3>{$translation("simulator-recording-title")}</h3>
            <p>{$translation("simulator-recording-detail", {
              days: recordingStatus.preferences.retention_days,
            })}</p>
          </div>
          {#if recordingActive}
            <button
              class="recording-stop"
              type="button"
              disabled={recordingBusy}
              onclick={onrecordstop}
            >{$translation("simulator-recording-stop")}</button>
          {:else}
            <button
              type="button"
              disabled={recordingBusy || !snapshot}
              onclick={onrecordstart}
            >{$translation("simulator-recording-start")}</button>
          {/if}
        </div>

        {#if recordingStatus.last_code}
          <p class="recording-notice">
            {$translation(
              recordingStatus.last_code === "recording.aircraft_changed"
                ? "simulator-recording-aircraft-changed"
                : "simulator-recording-storage-failed",
            )}
          </p>
        {/if}

        <div class="recording-history-heading">
          <strong>{$translation("simulator-recording-history")}</strong>
          {#if recordingStatus.sessions.length > 0}
            <button
              class="recording-delete-all"
              type="button"
              disabled={recordingBusy || recordingActive}
              onclick={confirmDeleteAll}
            >{$translation("simulator-recording-delete-all")}</button>
          {/if}
        </div>

        {#if recordingStatus.sessions.length === 0}
          <p class="recording-empty">{$translation("simulator-recording-empty")}</p>
        {:else}
          <div class="recording-sessions">
            {#each recordingStatus.sessions as session}
              <article class:active={session.id === recordingStatus.active_session_id}>
                <button
                  class="recording-select"
                  type="button"
                  disabled={recordingBusy}
                  onclick={() => onsessionselect(session.id)}
                >
                  <strong>{session.aircraft_registration ?? session.aircraft_title}</strong>
                  <span>{formatTime(session.started_at)} · {session.sample_count.toLocaleString()} {$translation("simulator-recording-samples")}</span>
                  <small>{$translation(`simulator-recording-status-${session.status}`)}</small>
                </button>
                <button
                  class="recording-delete"
                  type="button"
                  aria-label={$translation("simulator-recording-delete")}
                  disabled={recordingBusy || session.id === recordingStatus.active_session_id}
                  onclick={() => confirmDelete(session.id)}
                >×</button>
              </article>
            {/each}
          </div>
        {/if}

        {#if recordingSession && recordingSession.samples.length > 0}
          <div class="recording-charts">
            <WyrmChart
              spec={altitudeRecordingChart(recordingSession, displayPreferences)}
              eyebrow="WyrmChart telemetry"
              height="210px"
            />
            <WyrmChart
              spec={speedRecordingChart(recordingSession, displayPreferences)}
              eyebrow="WyrmChart telemetry"
              height="210px"
            />
          </div>
        {/if}
      </section>

      <footer class="simulator-footer">
        <span>
          {$translation("simulator-contracts", {
            bridge: status.bridge_protocol_version,
            telemetry: status.telemetry_schema_version,
          })}
        </span>
        <button type="button" disabled={busy} onclick={onrefresh}>
          {$translation("simulator-refresh")}
        </button>
      </footer>
    </div>
  </div>
{/if}
