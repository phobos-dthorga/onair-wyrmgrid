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
    AudioCodecPackageInspection,
    AudioProviderPackageInspection,
    AudioRecordingPreferences,
    AudioRecordingView,
    AudioSourceSelection,
    ManagedAudioProviderPackage,
    ManagedAudioCodecPackage,
    ManagedSimulatorProviderPackage,
    SimulatorProviderView,
    SimulatorProviderPackageInspection,
    SimulatorRecordingView,
    SimulatorSessionDebrief,
    SimulatorSessionView,
  } from "./types";

  let {
    open,
    status,
    managedProviders,
    pendingProviderPackage,
    busy = false,
    errorMessage = "",
    displayPreferences,
    recordingStatus,
    recordingSession,
    recordingDebrief,
    recordingBusy = false,
    audioStatus,
    audioPlayback,
    managedAudioProviders,
    pendingAudioProviderPackage,
    managedAudioCodecs,
    pendingAudioCodecPackage,
    audioBusy = false,
    onrefresh,
    onstart,
    onstop,
    onchooseproviderpackage,
    oncancelproviderpackage,
    oninstallproviderpackage,
    onproviderpackageenable,
    onproviderpackagerollback,
    onproviderpackageremove,
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
    onaudiochooseproviderpackage,
    onaudiocancelproviderpackage,
    onaudioinstallproviderpackage,
    onaudioproviderselect,
    onaudioproviderenable,
    onaudioproviderrollback,
    onaudioproviderremove,
    onaudiochoosecodecpackage,
    onaudiocancelcodecpackage,
    onaudioinstallcodecpackage,
    onaudiocodecenable,
    onaudiocodecrollback,
    onaudiocodecremove,
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
    managedProviders: ManagedSimulatorProviderPackage[];
    pendingProviderPackage?: SimulatorProviderPackageInspection;
    busy?: boolean;
    errorMessage?: string;
    displayPreferences: DisplayPreferences;
    recordingStatus: SimulatorRecordingView;
    recordingSession?: SimulatorSessionView;
    recordingDebrief?: SimulatorSessionDebrief;
    recordingBusy?: boolean;
    audioStatus: AudioRecordingView;
    audioPlayback?: AudioPlaybackView;
    managedAudioProviders: ManagedAudioProviderPackage[];
    pendingAudioProviderPackage?: AudioProviderPackageInspection;
    managedAudioCodecs: ManagedAudioCodecPackage[];
    pendingAudioCodecPackage?: AudioCodecPackageInspection;
    audioBusy?: boolean;
    onrefresh: () => void;
    onstart: (providerId: string) => void;
    onstop: (providerId: string) => void;
    onchooseproviderpackage: () => void;
    oncancelproviderpackage: () => void;
    oninstallproviderpackage: () => void;
    onproviderpackageenable: (providerId: string, enabled: boolean) => void;
    onproviderpackagerollback: (providerId: string) => void;
    onproviderpackageremove: (providerId: string) => void;
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
    onaudiochooseproviderpackage: () => void;
    onaudiocancelproviderpackage: () => void;
    onaudioinstallproviderpackage: () => void;
    onaudioproviderselect: (providerId: string) => void;
    onaudioproviderenable: (providerId: string, enabled: boolean) => void;
    onaudioproviderrollback: (providerId: string) => void;
    onaudioproviderremove: (providerId: string) => void;
    onaudiochoosecodecpackage: () => void;
    onaudiocancelcodecpackage: () => void;
    onaudioinstallcodecpackage: () => void;
    onaudiocodecenable: (providerId: string, enabled: boolean) => void;
    onaudiocodecrollback: (providerId: string) => void;
    onaudiocodecremove: (providerId: string) => void;
    onaudiorefresh: () => void;
    onaudiopermission: (sourceId: string) => void;
    onaudiosource: (selection: AudioSourceSelection) => void;
    onaudioplayback: (sessionId: string) => void;
    onaudioexport: (sessionId: string, trackId: string) => void;
    onaudiodelete: (sessionId: string) => void;
    onclose: () => void;
  } = $props();

  const snapshot = $derived(status.latest_snapshot);
  let removalCandidate = $state<string>();

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

  function providerPackageProcessActive(providerId: string): boolean {
    return status.providers.some(
      (provider) => provider.id === providerId && processActive(provider),
    );
  }

  function compactBytes(bytes: number): string {
    if (bytes >= 1024 * 1024)
      return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
    return `${Math.max(1, Math.ceil(bytes / 1024))} KiB`;
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
        class="provider-packages"
        aria-labelledby="provider-package-title"
      >
        <div class="provider-package-heading">
          <div>
            <span class="eyebrow"
              >{$translation("simulator-provider-packages-eyebrow")}</span
            >
            <h3 id="provider-package-title">
              {$translation("simulator-provider-packages-title")}
            </h3>
            <p>
              {$translation("simulator-provider-packages-introduction")}
            </p>
          </div>
          <button
            type="button"
            disabled={busy || pendingProviderPackage !== undefined}
            onclick={onchooseproviderpackage}
            >{$translation("simulator-provider-packages-choose")}</button
          >
        </div>

        {#if pendingProviderPackage}
          <div class="provider-package-review">
            <div>
              <strong>{pendingProviderPackage.name}</strong>
              <span>
                {$translation("simulator-provider-package-by-author", {
                  author: pendingProviderPackage.author,
                  version: pendingProviderPackage.version,
                })}
              </span>
            </div>
            <dl>
              <div>
                <dt>{$translation("simulator-provider-package-identity")}</dt>
                <dd>{pendingProviderPackage.id}</dd>
              </div>
              <div>
                <dt>
                  {$translation("simulator-provider-package-compatibility")}
                </dt>
                <dd>
                  {$translation(
                    "simulator-provider-package-compatibility-value",
                    {
                      platforms: pendingProviderPackage.platforms.join(" · "),
                      simulators:
                        pendingProviderPackage.simulators.join(" · "),
                      bridge: pendingProviderPackage.bridge_protocol_version,
                    },
                  )}
                </dd>
              </div>
              <div>
                <dt>{$translation("simulator-provider-package-contents")}</dt>
                <dd>
                  {$translation("simulator-provider-package-contents-value", {
                    count: pendingProviderPackage.file_count,
                    size: compactBytes(pendingProviderPackage.expanded_size),
                  })}
                </dd>
              </div>
              <div>
                <dt>
                  {$translation("simulator-provider-package-capabilities")}
                </dt>
                <dd>{pendingProviderPackage.capabilities.join(" · ")}</dd>
              </div>
              <div>
                <dt>
                  {$translation("simulator-provider-package-archive-digest")}
                </dt>
                <dd class="provider-package-digest">
                  {pendingProviderPackage.archive_sha256}
                </dd>
              </div>
            </dl>
            <p class="provider-package-warning">
              {$translation("security-simulator-provider-package-warning")}
            </p>
            <div class="provider-package-actions">
              <button
                class="secondary"
                type="button"
                disabled={busy}
                onclick={oncancelproviderpackage}
                >{$translation("action-cancel")}</button
              >
              <button
                type="button"
                disabled={busy}
                onclick={oninstallproviderpackage}
                >{$translation("simulator-provider-package-install")}</button
              >
            </div>
          </div>
        {/if}

        {#if managedProviders.length > 0}
          <div class="managed-provider-packages">
            {#each managedProviders as managed (managed.id)}
              <article>
                <div class="managed-provider-summary">
                  <strong>{managed.name}</strong>
                  <span>
                    {managed.enabled
                      ? $translation("simulator-provider-package-enabled", {
                          version: managed.active_version,
                        })
                      : $translation("simulator-provider-package-disabled", {
                          version: managed.active_version,
                        })}
                  </span>
                  <small>{managed.id}</small>
                </div>
                {#if removalCandidate === managed.id}
                  <div class="provider-removal-confirmation">
                    <span
                      >{$translation(
                        "destructive-simulator-provider-package-remove-confirm",
                      )}</span
                    >
                    <button
                      class="secondary"
                      type="button"
                      disabled={busy}
                      onclick={() => (removalCandidate = undefined)}
                      >{$translation("simulator-provider-package-keep")}</button
                    >
                    <button
                      class="stop"
                      type="button"
                      disabled={busy}
                      onclick={() => {
                        removalCandidate = undefined;
                        onproviderpackageremove(managed.id);
                      }}
                      >{$translation("simulator-provider-package-remove")}</button
                    >
                  </div>
                {:else}
                  <div class="provider-package-actions">
                    <button
                      class="secondary"
                      type="button"
                      disabled={busy ||
                        providerPackageProcessActive(managed.id)}
                      onclick={() =>
                        onproviderpackageenable(managed.id, !managed.enabled)}
                    >
                      {managed.enabled
                        ? $translation("simulator-provider-package-disable")
                        : $translation("simulator-provider-package-enable")}
                    </button>
                    <button
                      class="secondary"
                      type="button"
                      disabled={busy ||
                        !managed.rollback_version ||
                        providerPackageProcessActive(managed.id)}
                      onclick={() => onproviderpackagerollback(managed.id)}
                      >{$translation("simulator-provider-package-rollback")}</button
                    >
                    <button
                      class="stop"
                      type="button"
                      disabled={busy ||
                        providerPackageProcessActive(managed.id)}
                      onclick={() => (removalCandidate = managed.id)}
                      >{$translation("simulator-provider-package-remove-menu")}</button
                    >
                  </div>
                {/if}
              </article>
            {/each}
          </div>
        {/if}
      </section>

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
        managedProviders={managedAudioProviders}
        pendingProviderPackage={pendingAudioProviderPackage}
        managedCodecs={managedAudioCodecs}
        pendingCodecPackage={pendingAudioCodecPackage}
        busy={audioBusy}
        onpreferences={onaudiopreferences}
        onchooseproviderpackage={onaudiochooseproviderpackage}
        oncancelproviderpackage={onaudiocancelproviderpackage}
        oninstallproviderpackage={onaudioinstallproviderpackage}
        onproviderselect={onaudioproviderselect}
        onproviderenable={onaudioproviderenable}
        onproviderrollback={onaudioproviderrollback}
        onproviderremove={onaudioproviderremove}
        onchoosecodecpackage={onaudiochoosecodecpackage}
        oncancelcodecpackage={onaudiocancelcodecpackage}
        oninstallcodecpackage={onaudioinstallcodecpackage}
        oncodecenable={onaudiocodecenable}
        oncodecrollback={onaudiocodecrollback}
        oncodecremove={onaudiocodecremove}
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
