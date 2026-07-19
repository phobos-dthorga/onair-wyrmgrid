<script lang="ts">
  import { onMount, tick } from "svelte";
  import { isDesktopRuntime } from "$lib/desktop/client";
  import { loadStartupOptions } from "$lib/launch/startup";
  import type { WeatherRenderingProfile } from "$lib/settings/types";
  import type {
    WeatherRenderer,
    WeatherRendererBackend,
  } from "$lib/weather/renderer/types";
  import type { WeatherVolumeAppearanceOverrides } from "$lib/weather/renderer/volumeAppearance";
  import {
    buildWeatherGalleryUpdate,
    projectWeatherGalleryPoint,
    type WeatherGalleryEffect,
  } from "$lib/weather/renderer/weatherGallery";
  import { weatherGalleryAccessEnabled } from "$lib/weather/renderer/weatherGalleryAccess";

  const developerBuild = import.meta.env.DEV;
  type GalleryAccess = "checking" | "enabled" | "disabled";
  const effects: readonly WeatherGalleryEffect[] = [
    "all",
    "cloud",
    "rain",
    "convective",
    "snow",
    "obscuration",
    "dust",
  ];

  let canvas = $state<HTMLCanvasElement>();
  let renderer: WeatherRenderer | undefined;
  let animationFrame: number | undefined;
  let generation = 0;
  let profile = $state<Exclude<WeatherRenderingProfile, "compatibility">>(
    "cinematic",
  );
  let effect = $state<WeatherGalleryEffect>("all");
  let animation = $state(false);
  let thresholdOffset = $state(0);
  let opticalThicknessScale = $state(1);
  let transitionRangeScale = $state(1);
  let status = $state("Waiting for the renderer");
  let backend = $state<WeatherRendererBackend | undefined>();
  let access = $state<GalleryAccess>(
    developerBuild ? "enabled" : "checking",
  );

  function appearanceOverrides(): WeatherVolumeAppearanceOverrides {
    return {
      thresholdOffset,
      opticalThicknessScale,
      transitionRangeScale,
    };
  }

  function updateRenderer(): void {
    renderer?.update(buildWeatherGalleryUpdate(profile, effect, animation));
  }

  function render(timeMs: number): void {
    if (
      renderer &&
      canvas &&
      canvas.clientWidth > 0 &&
      canvas.clientHeight > 0
    ) {
      const width = canvas.clientWidth;
      const height = canvas.clientHeight;
      renderer.render({
        width,
        height,
        pixelRatio: window.devicePixelRatio || 1,
        zoom: 4,
        bearing: 0,
        projectionKey: `gallery:${width}:${height}`,
        timeMs: animation ? timeMs : 0,
        project: (longitude, latitude) =>
          projectWeatherGalleryPoint(
            longitude,
            latitude,
            width,
            height,
          ),
        surfaceVisibilityAt: () => 1,
      });
    }
    animationFrame = window.requestAnimationFrame(render);
  }

  async function rebuildRenderer(): Promise<void> {
    if (access !== "enabled" || !canvas) return;
    const requestedGeneration = ++generation;
    renderer?.dispose();
    renderer = undefined;
    backend = undefined;
    status = "Building the reference volumes…";
    try {
      const { createThreeWeatherRenderer } = await import(
        "$lib/weather/renderer/threeWeatherRenderer"
      );
      const next = await createThreeWeatherRenderer(
        canvas,
        buildWeatherGalleryUpdate(profile, effect, animation),
        (lostBackend, reason) => {
          backend = lostBackend;
          status = `Graphics device lost: ${reason}`;
        },
        (quality) => {
          status = backend
            ? `Ready · ${backend.toUpperCase()} · automatic quality ${quality}`
            : `Ready · automatic quality ${quality}`;
        },
        appearanceOverrides(),
      );
      if (requestedGeneration !== generation) {
        next.dispose();
        return;
      }
      renderer = next;
      backend = next.backend;
      status = `Ready · ${next.backend.toUpperCase()} · automatic quality ${next.quality}`;
    } catch (error) {
      if (requestedGeneration !== generation) return;
      status =
        error instanceof Error
          ? error.message
          : "The reference renderer could not start.";
    }
  }

  function resetCalibration(): void {
    thresholdOffset = 0;
    opticalThicknessScale = 1;
    transitionRangeScale = 1;
    void rebuildRenderer();
  }

  onMount(() => {
    let cancelled = false;
    const initialize = async () => {
      let startupOptions:
        | Awaited<ReturnType<typeof loadStartupOptions>>
        | undefined;
      if (!developerBuild && isDesktopRuntime()) {
        try {
          startupOptions = await loadStartupOptions();
        } catch {
          startupOptions = undefined;
        }
      }
      if (cancelled) return;
      access = weatherGalleryAccessEnabled(
        developerBuild,
        isDesktopRuntime(),
        startupOptions,
      )
        ? "enabled"
        : "disabled";
      if (access !== "enabled") return;
      await tick();
      if (cancelled) return;
      await rebuildRenderer();
      if (!cancelled) {
        animationFrame = window.requestAnimationFrame(render);
      }
    };
    void initialize();
    return () => {
      cancelled = true;
      generation += 1;
      if (animationFrame !== undefined) {
        window.cancelAnimationFrame(animationFrame);
      }
      renderer?.dispose();
      renderer = undefined;
    };
  });
</script>

<svelte:head>
  <title>Weather gallery · WyrmGrid developer tools</title>
</svelte:head>

{#if access === "enabled"}
  <main class="weather-gallery">
    <header>
      <div>
        <span>WyrmGrid developer tools</span>
        <h1>Deterministic weather gallery</h1>
        <p>
          The same renderer and appearance rules used by Atlas, isolated from
          the geographic camera for repeatable visual calibration.
        </p>
      </div>
      {#if developerBuild}
        <a href="/">Return to WyrmGrid</a>
      {:else}
        <span class="restart-note">
          Restart without the gallery flag to return to WyrmGrid
        </span>
      {/if}
    </header>

    <section class="gallery-controls" aria-label="Weather gallery controls">
      <label>
        <span>Rendering profile</span>
        <select bind:value={profile} onchange={updateRenderer}>
          <option value="enhanced">Enhanced</option>
          <option value="cinematic">Cinematic</option>
        </select>
      </label>
      <label>
        <span>Reference scene</span>
        <select bind:value={effect} onchange={updateRenderer}>
          {#each effects as option}
            <option value={option}>{option}</option>
          {/each}
        </select>
      </label>
      <label class="check-control">
        <input type="checkbox" bind:checked={animation} onchange={updateRenderer} />
        <span>Animate without lightning flashes</span>
      </label>
      <label>
        <span>Density threshold offset: {thresholdOffset.toFixed(2)}</span>
        <input
          type="range"
          min="-0.15"
          max="0.15"
          step="0.01"
          bind:value={thresholdOffset}
          onchange={() => void rebuildRenderer()}
        />
      </label>
      <label>
        <span>Optical thickness: {opticalThicknessScale.toFixed(2)}×</span>
        <input
          type="range"
          min="0.4"
          max="2"
          step="0.05"
          bind:value={opticalThicknessScale}
          onchange={() => void rebuildRenderer()}
        />
      </label>
      <label>
        <span>Edge softness: {transitionRangeScale.toFixed(2)}×</span>
        <input
          type="range"
          min="0.5"
          max="1.8"
          step="0.05"
          bind:value={transitionRangeScale}
          onchange={() => void rebuildRenderer()}
        />
      </label>
      <button type="button" onclick={resetCalibration}>Reset calibration</button>
    </section>

    <section class="gallery-stage" aria-label="Rendered weather references">
      <canvas bind:this={canvas}></canvas>
      <div class="gallery-status" class:error={!backend}>{status}</div>
    </section>

    <footer>
      These controls are deliberately developer-only. End users receive stable
      profiles, effect toggles, Reduced Motion, and flash protection.
    </footer>
  </main>
{:else if access === "checking"}
  <main class="gallery-unavailable">
    <h1>Checking developer access…</h1>
    <p>WyrmGrid is confirming whether the weather gallery was requested.</p>
  </main>
{:else}
  <main class="gallery-unavailable">
    <h1>Developer tool unavailable</h1>
    <p>
      Use a development build or deliberately launch WyrmGrid with
      <code>--weather-gallery</code>.
    </p>
    <a href="/">Return to WyrmGrid</a>
  </main>
{/if}

<style>
  :global(body) {
    margin: 0;
    background: #091525;
    color: #e8f2f7;
    font-family: Inter, system-ui, sans-serif;
  }

  .weather-gallery {
    min-height: 100vh;
    display: grid;
    grid-template-rows: auto auto minmax(420px, 1fr) auto;
    gap: 16px;
    padding: 20px;
    box-sizing: border-box;
    background:
      radial-gradient(circle at 50% 20%, #174d72 0, transparent 48%),
      linear-gradient(#0d2c48, #081522 72%);
  }

  header,
  .gallery-controls,
  footer {
    background: rgb(5 18 31 / 82%);
    border: 1px solid rgb(150 205 231 / 20%);
    border-radius: 12px;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 16px 20px;
  }

  header span,
  label span {
    color: #91bed4;
    font-size: 0.75rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  h1,
  p {
    margin: 4px 0;
  }

  header p {
    color: #b9ced9;
  }

  header .restart-note {
    max-width: 24rem;
    color: #b9ced9;
    line-height: 1.5;
    text-align: right;
  }

  a,
  button {
    color: #d9f3ff;
  }

  .gallery-controls {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
    align-items: end;
    gap: 14px;
    padding: 14px;
  }

  label {
    display: grid;
    gap: 7px;
  }

  select,
  input[type="range"],
  button {
    width: 100%;
  }

  select,
  button {
    min-height: 36px;
    border: 1px solid rgb(150 205 231 / 28%);
    border-radius: 7px;
    background: #102b40;
    color: inherit;
  }

  .check-control {
    grid-template-columns: auto 1fr;
    align-items: center;
    align-self: center;
  }

  .check-control input {
    width: 18px;
    height: 18px;
  }

  .gallery-stage {
    position: relative;
    min-height: 420px;
    overflow: hidden;
    border: 1px solid rgb(177 222 242 / 28%);
    border-radius: 16px;
    background:
      linear-gradient(rgb(255 255 255 / 4%) 1px, transparent 1px),
      linear-gradient(90deg, rgb(255 255 255 / 4%) 1px, transparent 1px),
      linear-gradient(#1a5d86, #708e9d 62%, #45545f);
    background-size: 40px 40px, 40px 40px, auto;
  }

  canvas {
    width: 100%;
    height: 100%;
    position: absolute;
    inset: 0;
  }

  .gallery-status {
    position: absolute;
    left: 12px;
    bottom: 12px;
    padding: 8px 10px;
    border-radius: 6px;
    background: rgb(3 12 20 / 82%);
    color: #bde9d1;
    font-size: 0.82rem;
  }

  .gallery-status.error {
    color: #ffd6c8;
  }

  footer,
  .gallery-unavailable {
    padding: 14px 18px;
    color: #a9c1ce;
  }

  .gallery-unavailable {
    max-width: 540px;
    margin: 12vh auto;
  }

  @media (max-width: 720px) {
    .weather-gallery {
      padding: 10px;
    }

    header {
      align-items: flex-start;
      flex-direction: column;
    }

    header .restart-note {
      text-align: left;
    }
  }
</style>
