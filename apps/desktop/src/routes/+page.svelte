<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import AtlasMap from "$lib/atlas/AtlasMap.svelte";
  import { atlasPreviewFleet } from "$lib/atlas/sample";
  import type { AircraftSummary, FleetSnapshot } from "$lib/atlas/types";
  import WyrmChart from "$lib/charts/WyrmChart.svelte";
  import { foundationChart } from "$lib/charts/sample";
  import ConnectionDialog from "$lib/onair/ConnectionDialog.svelte";
  import {
    disconnectedStatus,
    type OnAirConnectionStatus,
  } from "$lib/onair/types";

  type PlatformStatus = {
    application: string;
    version: string;
    plugin_api_version: number;
    mode: string;
  };

  type FleetLoadState = "idle" | "loading" | "ready" | "error";

  let status = $state<PlatformStatus>({
    application: "OnAir WyrmGrid",
    version: "0.1.0",
    plugin_api_version: 1,
    mode: "browser preview",
  });
  let connection = $state<OnAirConnectionStatus>(disconnectedStatus);
  let showConnectionDialog = $state(false);
  let fleetSnapshot = $state<FleetSnapshot | null>(null);
  let fleetLoadState = $state<FleetLoadState>("idle");
  let fleetError = $state("");
  let fleetVisible = $state(true);
  let selectedAircraftId = $state<string | null>(null);

  const aircraft = $derived(fleetSnapshot?.value ?? []);
  const plottedAircraftCount = $derived(aircraft.filter((item) => item.location).length);
  const selectedAircraft = $derived(
    aircraft.find((item) => item.id === selectedAircraftId) ?? null,
  );
  const fleetSourceLabel = $derived(
    fleetSnapshot?.provenance.kind === "on_air_fact" ? "OnAir fact" : "Illustrative preview",
  );
  const layers = $derived([
    { id: "fleet", name: "Fleet", count: plottedAircraftCount, active: fleetVisible, available: true },
    { id: "fbos", name: "FBO network", count: 0, active: false, available: false },
    { id: "jobs", name: "Jobs", count: 0, active: false, available: false },
    { id: "maintenance", name: "Maintenance", count: 0, active: false, available: false },
  ]);

  function safeError(error: unknown): string {
    return typeof error === "string" && error.length > 0
      ? error
      : "WyrmGrid could not refresh the fleet.";
  }

  function formatObservedAt(value: string | undefined): string {
    if (!value) return "No fleet observation yet";
    const observed = new Date(value);
    return Number.isNaN(observed.getTime())
      ? "Observation time unavailable"
      : `Observed ${observed.toLocaleString()}`;
  }

  function displayRegistration(item: AircraftSummary): string {
    return item.registration ?? "Unknown registration";
  }

  function formatCoordinates(item: AircraftSummary): string {
    if (!item.location) return "Location unavailable";
    return `${item.location.latitude.toFixed(4)}, ${item.location.longitude.toFixed(4)}`;
  }

  async function refreshFleet(): Promise<void> {
    if (!connection.connected || fleetLoadState === "loading") return;
    fleetLoadState = "loading";
    fleetError = "";

    try {
      fleetSnapshot = await invoke<FleetSnapshot>("refresh_onair_fleet");
      fleetLoadState = "ready";
      if (
        selectedAircraftId &&
        !fleetSnapshot.value.some((item) => item.id === selectedAircraftId)
      ) {
        selectedAircraftId = null;
      }
    } catch (error) {
      fleetLoadState = "error";
      fleetError = safeError(error);
    }
  }

  async function restoreFleetSnapshot(): Promise<void> {
    try {
      const snapshot = await invoke<FleetSnapshot | null>("onair_fleet_snapshot");
      if (snapshot) {
        fleetSnapshot = snapshot;
        fleetLoadState = "ready";
      } else {
        await refreshFleet();
      }
    } catch {
      // Browser previews do not expose the Tauri command bridge.
    }
  }

  function handleConnectionStatus(value: OnAirConnectionStatus): void {
    connection = value;
    if (value.connected) {
      showConnectionDialog = false;
      void refreshFleet();
    } else {
      fleetSnapshot = null;
      fleetLoadState = "idle";
      fleetError = "";
      selectedAircraftId = null;
    }
  }

  onMount(() => {
    invoke<PlatformStatus>("platform_status")
      .then((value) => (status = value))
      .catch(() => {
        fleetSnapshot = atlasPreviewFleet;
        fleetLoadState = "ready";
      });

    invoke<OnAirConnectionStatus>("onair_connection_status")
      .then((value) => {
        connection = value;
        if (value.connected) void restoreFleetSnapshot();
      })
      .catch(() => {
        // Browser previews do not expose the Tauri command bridge.
      });
  });
</script>

<svelte:head>
  <title>OnAir WyrmGrid</title>
</svelte:head>

<main class="shell">
  <header class="topbar">
    <div class="brand-mark" aria-hidden="true">WG</div>
    <div class="brand-copy">
      <span class="eyebrow">OnAir</span>
      <h1>WyrmGrid</h1>
    </div>
    <nav aria-label="Primary navigation">
      <button class="nav-item active">Atlas</button>
      <button class="nav-item">Fleet</button>
      <button class="nav-item">Dispatch</button>
      <button class="nav-item">Hoard</button>
      <button class="nav-item">Forge</button>
    </nav>
    <button
      class:connected={connection.connected}
      class="connection-pill"
      type="button"
      onclick={() => (showConnectionDialog = true)}
    >
      <span></span>
      {connection.connected && connection.company ? connection.company.name : "Connect OnAir"}
    </button>
  </header>

  <section class="workspace">
    <aside class="sidebar" aria-label="Map layers">
      <div class="section-heading">
        <div>
          <span class="eyebrow">WyrmGrid Atlas</span>
          <h2>Operations layers</h2>
        </div>
        <button
          class="icon-button"
          class:refreshing={fleetLoadState === "loading"}
          aria-label="Refresh fleet"
          title="Refresh fleet"
          disabled={!connection.connected || fleetLoadState === "loading"}
          onclick={() => void refreshFleet()}
        >↻</button>
      </div>

      <div class="layer-list">
        {#each layers as layer}
          <button
            class:muted={!layer.active}
            class="layer-row"
            aria-pressed={layer.active}
            disabled={!layer.available}
            title={layer.available ? `Toggle ${layer.name}` : `${layer.name} is planned for a later slice`}
            onclick={() => {
              if (layer.id === "fleet") fleetVisible = !fleetVisible;
            }}
          >
            <span class="layer-indicator"></span>
            <span>{layer.name}</span>
            <strong>{layer.count}</strong>
          </button>
        {/each}
      </div>

      <div class="sidebar-note" class:error-note={fleetLoadState === "error"}>
        <span class="note-icon">{fleetLoadState === "error" ? "!" : "i"}</span>
        <p>
          {#if fleetLoadState === "loading"}
            Refreshing fleet from OnAir…
          {:else if fleetLoadState === "error"}
            {fleetError}
          {:else if fleetSnapshot}
            {aircraft.length} aircraft received; {plottedAircraftCount} have a mappable location.
            {formatObservedAt(fleetSnapshot.provenance.observed_at)}.
          {:else if connection.connected}
            OnAir is connected. Refresh the fleet to populate Atlas.
          {:else}
            Connect an OnAir company to begin. Credentials remain only in memory for this session.
          {/if}
        </p>
      </div>
    </aside>

    <section class="map-stage" aria-label="Universal operations map">
      <AtlasMap
        {aircraft}
        fleetVisible={fleetVisible}
        selectedAircraftId={selectedAircraftId}
        onselect={(aircraftId) => (selectedAircraftId = aircraftId)}
      />
      <div class="map-wash"></div>
      <div class="map-title">
        <span class="eyebrow">Universal operations map</span>
        <strong>See the network. Command the skies.</strong>
      </div>
      <div class="readiness-card">
        <span class="eyebrow">Atlas readiness</span>
        <div class="readiness-value">
          {fleetSnapshot ? `${plottedAircraftCount} aircraft mapped` : "Awaiting fleet"}
        </div>
        <dl>
          <div><dt>Source</dt><dd>{fleetSnapshot ? fleetSourceLabel : "Not connected"}</dd></div>
          <div><dt>Plugin API</dt><dd>v{status.plugin_api_version}</dd></div>
          <div><dt>Build</dt><dd>{status.version}</dd></div>
        </dl>
        {#if !fleetSnapshot}<WyrmChart spec={foundationChart} />{/if}
      </div>
    </section>

    <aside class="inspector" aria-label="Selection inspector">
      <span class="eyebrow">Inspector</span>
      {#if selectedAircraft}
        <h2>{displayRegistration(selectedAircraft)}</h2>
        <p>{selectedAircraft.model ?? "Aircraft type unavailable"}</p>

        <div class="selection-details">
          <article>
            <span>Current airport</span>
            <strong>{selectedAircraft.current_airport?.icao || "Not reported"}</strong>
            {#if selectedAircraft.current_airport?.name}
              <small>{selectedAircraft.current_airport.name}</small>
            {/if}
          </article>
          <article>
            <span>Coordinates</span>
            <strong>{formatCoordinates(selectedAircraft)}</strong>
          </article>
          <article>
            <span>Provenance</span>
            <strong>{fleetSourceLabel}</strong>
            <small>{formatObservedAt(fleetSnapshot?.provenance.observed_at)}</small>
          </article>
        </div>
      {:else}
        <h2>Nothing selected</h2>
        <p>Select a mapped aircraft to inspect its current operational context.</p>

        <div class="empty-radar" aria-hidden="true">
          <span></span><span></span><span></span>
          <i></i>
        </div>
      {/if}

      <div class="status-grid">
        <article><span>OnAir</span><strong>{connection.connected ? "Connected" : "Not connected"}</strong></article>
        <article><span>Fleet</span><strong>{fleetLoadState}</strong></article>
        <article><span>Storage</span><strong>Memory snapshot</strong></article>
      </div>
    </aside>
  </section>

  <footer>
    <span>{status.application} · {status.mode}</span>
    <span>Unaffiliated community project</span>
  </footer>
</main>

<ConnectionDialog
  open={showConnectionDialog}
  status={connection}
  onclose={() => (showConnectionDialog = false)}
  onstatuschange={handleConnectionStatus}
/>
