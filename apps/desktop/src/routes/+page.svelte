<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { Map } from "maplibre-gl";
  import "maplibre-gl/dist/maplibre-gl.css";
  import { onMount } from "svelte";
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

  let mapContainer: HTMLDivElement;
  let map: Map | undefined;
  let status = $state<PlatformStatus>({
    application: "OnAir WyrmGrid",
    version: "0.1.0",
    plugin_api_version: 1,
    mode: "browser preview",
  });
  let connection = $state<OnAirConnectionStatus>(disconnectedStatus);
  let showConnectionDialog = $state(false);

  const layers = [
    { name: "Fleet", count: 0, active: true },
    { name: "FBO network", count: 0, active: true },
    { name: "Jobs", count: 0, active: false },
    { name: "Maintenance", count: 0, active: false },
  ];

  onMount(() => {
    let cancelled = false;

    void import("maplibre-gl").then((module) => {
      if (cancelled) return;

      const maplibregl = module.default;
      map = new maplibregl.Map({
        container: mapContainer,
        style: "https://demotiles.maplibre.org/globe.json",
        center: [18, 22],
        zoom: 1.25,
        attributionControl: false,
      });

      map.addControl(new maplibregl.NavigationControl({ visualizePitch: true }), "top-right");
      map.addControl(new maplibregl.AttributionControl({ compact: true }), "bottom-right");
    });

    invoke<PlatformStatus>("platform_status")
      .then((value) => (status = value))
      .catch(() => {
        // Browser previews do not expose the Tauri command bridge.
      });

    invoke<OnAirConnectionStatus>("onair_connection_status")
      .then((value) => (connection = value))
      .catch(() => {
        // Browser previews do not expose the Tauri command bridge.
      });

    return () => {
      cancelled = true;
      map?.remove();
    };
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
        <button class="icon-button" aria-label="Add layer">+</button>
      </div>

      <div class="layer-list">
        {#each layers as layer}
          <button class:muted={!layer.active} class="layer-row">
            <span class="layer-indicator"></span>
            <span>{layer.name}</span>
            <strong>{layer.count}</strong>
          </button>
        {/each}
      </div>

      <div class="sidebar-note">
        <span class="note-icon">i</span>
        <p>
          {connection.connected
            ? "OnAir is authenticated for this session. Fleet retrieval is the next vertical slice."
            : "Connect an OnAir company to begin. Credentials remain only in memory for this session."}
        </p>
      </div>
    </aside>

    <section class="map-stage" aria-label="Universal operations map">
      <div bind:this={mapContainer} class="map"></div>
      <div class="map-wash"></div>
      <div class="map-title">
        <span class="eyebrow">Universal operations map</span>
        <strong>See the network. Command the skies.</strong>
      </div>
      <div class="readiness-card">
        <span class="eyebrow">Platform readiness</span>
        <div class="readiness-value">Foundation</div>
        <dl>
          <div><dt>Core</dt><dd>Rust</dd></div>
          <div><dt>Plugin API</dt><dd>v{status.plugin_api_version}</dd></div>
          <div><dt>Build</dt><dd>{status.version}</dd></div>
        </dl>
        <WyrmChart spec={foundationChart} />
      </div>
    </section>

    <aside class="inspector" aria-label="Selection inspector">
      <span class="eyebrow">Inspector</span>
      <h2>Nothing selected</h2>
      <p>Select an aircraft, airport, FBO, job, or route to inspect its operational context.</p>

      <div class="empty-radar" aria-hidden="true">
        <span></span><span></span><span></span>
        <i></i>
      </div>

      <div class="status-grid">
        <article><span>OnAir</span><strong>{connection.connected ? "Connected" : "Not connected"}</strong></article>
        <article><span>Storage</span><strong>SQLite ready</strong></article>
        <article><span>Plugins</span><strong>Protocol v1</strong></article>
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
  onstatuschange={(value) => {
    connection = value;
    if (value.connected) showConnectionDialog = false;
  }}
/>
