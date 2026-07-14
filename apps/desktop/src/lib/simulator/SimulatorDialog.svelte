<script lang="ts">
  import { translation } from "$lib/i18n/runtime";
  import "./simulator.css";
  import type {
    ProviderConnectionState,
    SimulatorBridgeView,
    SimulatorProviderView,
  } from "./types";

  let {
    open,
    status,
    busy = false,
    errorMessage = "",
    onrefresh,
    onstart,
    onstop,
    onclose,
  }: {
    open: boolean;
    status: SimulatorBridgeView;
    busy?: boolean;
    errorMessage?: string;
    onrefresh: () => void;
    onstart: (providerId: string) => void;
    onstop: (providerId: string) => void;
    onclose: () => void;
  } = $props();

  const snapshot = $derived(status.latest_snapshot);

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
    if (provider.connection_state === "connected") return 5;
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

  function formatTime(value: string | undefined): string {
    if (!value) return $translation("simulator-value-unavailable");
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape" && !busy) onclose();
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
          <article class:connected={provider.connection_state === "connected"}>
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
              <span>{stateLabel(provider.connection_state)}</span>
              {#if provider.last_code}<small class="provider-detail">
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
            <dd>{formatMeasurement(snapshot?.altitude_feet, "ft")}</dd>
          </div>
          <div>
            <dt>{$translation("simulator-heading")}</dt>
            <dd>{formatMeasurement(snapshot?.true_heading_degrees, "°")}</dd>
          </div>
          <div>
            <dt>{$translation("simulator-ground-speed")}</dt>
            <dd>{formatMeasurement(snapshot?.ground_speed_knots, "kt")}</dd>
          </div>
          <div>
            <dt>{$translation("simulator-indicated-speed")}</dt>
            <dd>
              {formatMeasurement(snapshot?.indicated_airspeed_knots, "kt")}
            </dd>
          </div>
          <div>
            <dt>{$translation("simulator-fuel-weight")}</dt>
            <dd>
              {formatMeasurement(snapshot?.fuel_total_weight_pounds, "lb")}
            </dd>
          </div>
          <div>
            <dt>{$translation("simulator-gross-weight")}</dt>
            <dd>{formatMeasurement(snapshot?.gross_weight_pounds, "lb")}</dd>
          </div>
          <div>
            <dt>{$translation("simulator-observed-at")}</dt>
            <dd>{formatTime(snapshot?.provenance.retrieved_at)}</dd>
          </div>
        </dl>
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
