<script lang="ts">
  import { translation } from "$lib/i18n/runtime";
  import { displayPresets, type DisplayPreferences } from "./types";
  import type {
    SimulatorPreferences,
    SimulatorProviderView,
    SimulatorRecordingPreferences,
  } from "$lib/simulator/types";
  import "./settings.css";

  let {
    open,
    preferences,
    simulatorPreferences,
    recordingPreferences,
    simulatorProviders,
    busy = false,
    errorMessage = "",
    onsave,
    onappearance,
    onlanguage,
    onprivacy,
    onsecurity,
    onclose,
  }: {
    open: boolean;
    preferences: DisplayPreferences;
    simulatorPreferences: SimulatorPreferences;
    recordingPreferences: SimulatorRecordingPreferences;
    simulatorProviders: SimulatorProviderView[];
    busy?: boolean;
    errorMessage?: string;
    onsave: (
      preferences: DisplayPreferences,
      simulatorPreferences: SimulatorPreferences,
      recordingPreferences: SimulatorRecordingPreferences,
    ) => void;
    onappearance: () => void;
    onlanguage: () => void;
    onprivacy: () => void;
    onsecurity: () => void;
    onclose: () => void;
  } = $props();

  let draft = $state<DisplayPreferences>({ ...displayPresets.aviation });
  let simulatorDraft = $state<SimulatorPreferences>({
    start_with_wyrmgrid: false,
  });
  let recordingDraft = $state<SimulatorRecordingPreferences>({
    retention_days: 30,
  });

  $effect(() => {
    if (open) {
      draft = { ...preferences };
      simulatorDraft = { ...simulatorPreferences };
      recordingDraft = { ...recordingPreferences };
    }
  });

  function applyPreset(preset: keyof typeof displayPresets): void {
    draft = { ...displayPresets[preset] };
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape" && !busy) onclose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="settings-backdrop">
    <div
      class="settings-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-title"
    >
      <header>
        <div>
          <span class="eyebrow">{$translation("settings-eyebrow")}</span>
          <h2 id="settings-title">{$translation("settings-title")}</h2>
          <p>{$translation("settings-introduction")}</p>
        </div>
        <button
          class="settings-close"
          type="button"
          aria-label={$translation("settings-close")}
          disabled={busy}
          onclick={onclose}>×</button
        >
      </header>

      <section class="settings-section">
        <div class="section-copy">
          <span class="eyebrow">{$translation("settings-units-eyebrow")}</span>
          <h3>{$translation("settings-units-title")}</h3>
          <p>{$translation("settings-units-detail")}</p>
        </div>

        <div class="preset-row" aria-label={$translation("settings-presets")}>
          {#each Object.keys(displayPresets) as preset}
            <button
              type="button"
              disabled={busy}
              onclick={() => applyPreset(preset as keyof typeof displayPresets)}
            >
              {$translation(`settings-preset-${preset}`)}
            </button>
          {/each}
        </div>

        <div class="unit-grid">
          <label>
            <span>{$translation("settings-altitude-unit")}</span>
            <select
              disabled={busy}
              bind:value={draft.altitude_unit}
              aria-label={$translation("settings-altitude-unit")}
            >
              <option value="feet">{$translation("unit-feet")}</option>
              <option value="metres">{$translation("unit-metres")}</option>
            </select>
          </label>
          <label>
            <span>{$translation("settings-speed-unit")}</span>
            <select
              disabled={busy}
              bind:value={draft.speed_unit}
              aria-label={$translation("settings-speed-unit")}
            >
              <option value="knots">{$translation("unit-knots")}</option>
              <option value="miles_per_hour"
                >{$translation("unit-miles-per-hour")}</option
              >
              <option value="kilometres_per_hour"
                >{$translation("unit-kilometres-per-hour")}</option
              >
              <option value="metres_per_second"
                >{$translation("unit-metres-per-second")}</option
              >
            </select>
          </label>
          <label>
            <span>{$translation("settings-weight-unit")}</span>
            <select
              disabled={busy}
              bind:value={draft.weight_unit}
              aria-label={$translation("settings-weight-unit")}
            >
              <option value="pounds">{$translation("unit-pounds")}</option>
              <option value="kilograms">{$translation("unit-kilograms")}</option
              >
            </select>
          </label>
          <label>
            <span>{$translation("settings-fuel-unit")}</span>
            <select
              disabled={busy}
              bind:value={draft.fuel_unit}
              aria-label={$translation("settings-fuel-unit")}
            >
              <option value="pounds"
                >{$translation("unit-pounds-weight")}</option
              >
              <option value="kilograms"
                >{$translation("unit-kilograms-weight")}</option
              >
              <option value="us_gallons"
                >{$translation("unit-us-gallons")}</option
              >
              <option value="imperial_gallons"
                >{$translation("unit-imperial-gallons")}</option
              >
              <option value="litres">{$translation("unit-litres")}</option>
            </select>
          </label>
        </div>

        <p class="settings-boundary">
          {$translation("settings-units-boundary")}
        </p>
      </section>

      <section class="settings-section">
        <div class="section-copy">
          <span class="eyebrow">{$translation("settings-simulator-eyebrow")}</span>
          <h3>{$translation("settings-simulator-title")}</h3>
          <p>{$translation("settings-simulator-detail")}</p>
        </div>

        <div class="unit-grid simulator-settings-grid">
          <label>
            <span>{$translation("settings-simulator-provider")}</span>
            <select
              disabled={busy || simulatorProviders.length === 0}
              value={simulatorDraft.selected_provider_id ?? ""}
              onchange={(event) =>
                (simulatorDraft = {
                  ...simulatorDraft,
                  selected_provider_id: event.currentTarget.value || undefined,
                })}
            >
              {#if simulatorProviders.length === 0}
                <option value="">{$translation("simulator-no-providers")}</option>
              {/if}
              {#each simulatorProviders as provider}
                <option value={provider.id}>{provider.name}</option>
              {/each}
            </select>
          </label>

          <label class="settings-toggle">
            <input
              type="checkbox"
              disabled={busy || !simulatorDraft.selected_provider_id}
              bind:checked={simulatorDraft.start_with_wyrmgrid}
            />
            <span>
              <strong>{$translation("settings-simulator-auto-start")}</strong>
              <small>{$translation("settings-simulator-auto-start-detail")}</small>
            </span>
          </label>
          <label>
            <span>{$translation("settings-simulator-retention")}</span>
            <select disabled={busy} bind:value={recordingDraft.retention_days}>
              <option value={7}>{$translation("settings-retention-days", { days: 7 })}</option>
              <option value={30}>{$translation("settings-retention-days", { days: 30 })}</option>
              <option value={90}>{$translation("settings-retention-days", { days: 90 })}</option>
              <option value={365}>{$translation("settings-retention-days", { days: 365 })}</option>
            </select>
          </label>
        </div>

        <p class="settings-boundary">
          {$translation("settings-simulator-boundary")}
        </p>
      </section>

      <section class="settings-section">
        <div class="section-copy">
          <span class="eyebrow">{$translation("settings-more-eyebrow")}</span>
          <h3>{$translation("settings-more-title")}</h3>
        </div>
        <div class="settings-links">
          <button type="button" disabled={busy} onclick={onappearance}>
            <strong>{$translation("settings-theme")}</strong>
            <span>{$translation("settings-theme-detail")}</span>
          </button>
          <button type="button" disabled={busy} onclick={onlanguage}>
            <strong>{$translation("settings-language")}</strong>
            <span>{$translation("settings-language-detail")}</span>
          </button>
          <button type="button" disabled={busy} onclick={onprivacy}>
            <strong>{$translation("settings-privacy-terms")}</strong>
            <span>{$translation("settings-privacy-detail")}</span>
          </button>
          <button type="button" disabled={busy} onclick={onsecurity}>
            <strong>{$translation("security-settings-link-title")}</strong>
            <span>{$translation("security-settings-link-detail")}</span>
          </button>
        </div>
      </section>

      {#if errorMessage}<p class="settings-error" role="alert">
          {errorMessage}
        </p>{/if}

      <footer>
        <button type="button" disabled={busy} onclick={onclose}>
          {$translation("action-cancel")}
        </button>
        <button
          class="settings-save"
          type="button"
          disabled={busy}
          onclick={() =>
            onsave(
              { ...draft },
              { ...simulatorDraft },
              { ...recordingDraft },
            )}
        >
          {busy
            ? $translation("settings-saving")
            : $translation("settings-save")}
        </button>
      </footer>
    </div>
  </div>
{/if}
