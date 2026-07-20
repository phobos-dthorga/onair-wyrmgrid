<script lang="ts">
  import { translation } from "$lib/i18n/runtime";
  import type { AtlasFlightRoute } from "$lib/atlas/types";
  import { formatLocalDateTime } from "$lib/presentation/dateTime";
  import type { DisplayPreferences } from "$lib/settings/types";
  import {
    presentAltitude,
    presentFuel,
    presentSpeed,
    presentWeight,
    type PresentedMeasurement,
  } from "$lib/settings/units";
  import "./simulator.css";
  import RecordingHistory from "./RecordingHistory.svelte";
  import AudioRecordingPanel from "./AudioRecordingPanel.svelte";
  import {
    providerConnectionStateMessageKeys,
    providerDetailFallbackMessageKey,
    providerDetailMessageKeys,
    simulatorRitualMessageKeys,
  } from "./bridgePresentation";
  import type {
    ProviderConnectionState,
    SimulatorBridgeView,
    AudioPlaybackView,
    AudioRecordingPreferences,
    AudioRecordingView,
    AudioSourceSelection,
    SimulatorProviderView,
    SimulatorRecordingView,
    SimulatorSessionDebrief,
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
    recordingDebrief,
    recordingBusy = false,
    audioStatus,
    audioPlayback,
    audioBusy = false,
    onrefresh,
    onstart,
    onstop,
    onrecordstart,
    onrecordstop,
    onsessionselect,
    onsessiondelete,
    ondeleteall,
    onpin,
    onpage,
    onexport,
    onviewatlas,
    onaudiopreferences,
    onaudiorefresh,
    onaudiopermission,
    onaudiosource,
    onaudioplayback,
    onaudioexport,
    onaudiodelete,
    onclose,
  }: {
    open: boolean;
    status: SimulatorBridgeView;
    busy?: boolean;
    errorMessage?: string;
    displayPreferences: DisplayPreferences;
    recordingStatus: SimulatorRecordingView;
    recordingSession?: SimulatorSessionView;
    recordingDebrief?: SimulatorSessionDebrief;
    recordingBusy?: boolean;
    audioStatus: AudioRecordingView;
    audioPlayback?: AudioPlaybackView;
    audioBusy?: boolean;
    onrefresh: () => void;
    onstart: (providerId: string) => void;
    onstop: (providerId: string) => void;
    onrecordstart: () => void;
    onrecordstop: () => void;
    onsessionselect: (sessionId: string) => void;
    onsessiondelete: (sessionId: string) => void;
    ondeleteall: () => void;
    onpin: (sessionId: string, pinned: boolean) => void;
    onpage: (sessionId: string, sampleOffset: number) => void;
    onexport: (sessionId: string, format: "json" | "csv") => void;
    onviewatlas: (route: AtlasFlightRoute) => void;
    onaudiopreferences: (preferences: AudioRecordingPreferences) => void;
    onaudiorefresh: () => void;
    onaudiopermission: (sourceId: string) => void;
    onaudiosource: (selection: AudioSourceSelection) => void;
    onaudioplayback: (sessionId: string) => void;
    onaudioexport: (sessionId: string, trackId: string) => void;
    onaudiodelete: (sessionId: string) => void;
    onclose: () => void;
  } = $props();

  const snapshot = $derived(status.latest_snapshot);

  function stateLabel(state: ProviderConnectionState): string {
    return $translation(providerConnectionStateMessageKeys[state]);
  }

  function providerDetail(code: string): string {
    return $translation(
      providerDetailMessageKeys[code] ?? providerDetailFallbackMessageKey,
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
    return formatLocalDateTime(
      value,
      value ?? $translation("simulator-value-unavailable"),
    );
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape" && !busy && !recordingBusy) onclose();
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
                  {#each simulatorRitualMessageKeys as step, index}
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

      <RecordingHistory
        status={recordingStatus}
        session={recordingSession}
        debrief={recordingDebrief}
        {displayPreferences}
        busy={recordingBusy}
        captureControls
        canStart={Boolean(snapshot)}
        onstart={onrecordstart}
        onstop={onrecordstop}
        {onsessionselect}
        {onsessiondelete}
        {ondeleteall}
        {onpin}
        {onpage}
        {onexport}
        {onviewatlas}
      />

      <AudioRecordingPanel
        status={audioStatus}
        playback={audioPlayback}
        busy={audioBusy}
        onpreferences={onaudiopreferences}
        onrefresh={onaudiorefresh}
        onpermission={onaudiopermission}
        onsource={onaudiosource}
        onplayback={onaudioplayback}
        onexport={onaudioexport}
        ondelete={onaudiodelete}
      />

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
