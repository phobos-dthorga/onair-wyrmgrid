<script lang="ts">
  import { responsiveSurface } from "$lib/accessibility/responsiveSurface";
  import ExplorationSummary from "$lib/exploration/ExplorationSummary.svelte";
  import {
    activeAtlasFilterCount,
    atlasSearchItems,
    defaultAtlasFilters,
    filterAtlasItems,
    type AtlasFilters,
    type AtlasSearchItem,
  } from "./presentation";
  import type { AircraftSummary, FboSummary } from "./types";

  let {
    aircraft,
    fbos,
    selectedAircraftId,
    selectedFboId,
    responsiveSurfaces = true,
    onselectaircraft,
    onselectfbo,
  }: {
    aircraft: AircraftSummary[];
    fbos: FboSummary[];
    selectedAircraftId: string | null;
    selectedFboId: string | null;
    responsiveSurfaces?: boolean;
    onselectaircraft: (id: string) => void;
    onselectfbo: (id: string) => void;
  } = $props();

  let filters = $state<AtlasFilters>({ ...defaultAtlasFilters });
  const items = $derived(atlasSearchItems(aircraft, fbos));
  const visibleItems = $derived(filterAtlasItems(items, filters));
  const activeFilterCount = $derived(activeAtlasFilterCount(filters));

  function resetFilters(): void {
    filters = { ...defaultAtlasFilters };
  }

  function select(item: AtlasSearchItem): void {
    if (item.kind === "aircraft") onselectaircraft(item.id);
    else onselectfbo(item.id);
  }
</script>

{#if items.length > 0}
  <section class="atlas-search" aria-label="Find Atlas operations">
    <label>
      <span>Find aircraft, FBOs, or airports</span>
      <input type="search" bind:value={filters.query} />
    </label>
    <details>
      <summary>
        <span>Filter and sort</span>
        {#if activeFilterCount > 0}<strong>{activeFilterCount} active</strong>{/if}
      </summary>
      <div class="atlas-filter-grid">
        <label>
          <span>Operation type</span>
          <select bind:value={filters.kind}>
            <option value="all">Aircraft and FBOs</option>
            <option value="aircraft">Aircraft</option>
            <option value="fbo">FBOs</option>
          </select>
        </label>
        <label>
          <span>Map position</span>
          <select bind:value={filters.mapping}>
            <option value="all">Mapped and unmapped</option>
            <option value="mapped">Mapped only</option>
            <option value="unmapped">Position unavailable</option>
          </select>
        </label>
        <label>
          <span>Order by</span>
          <select bind:value={filters.sort}>
            <option value="label">Name or registration</option>
            <option value="kind">Operation type</option>
            <option value="airport">Airport code</option>
          </select>
        </label>
      </div>
    </details>
    <ExplorationSummary
      shown={visibleItems.length}
      total={items.length}
      label="Atlas operations"
      activeFilters={activeFilterCount}
      onclear={resetFilters}
    />
    <div class="atlas-results" aria-label="Atlas search results">
      {#each visibleItems as item (item.kind + item.id)}
        <button
          class="responsive-surface"
          class:selected={item.kind === "aircraft"
            ? item.id === selectedAircraftId
            : item.id === selectedFboId}
          type="button"
          use:responsiveSurface={{ enabled: responsiveSurfaces }}
          onclick={() => select(item)}
        >
          <span>{item.kind === "aircraft" ? "Aircraft" : "FBO"}</span>
          <strong>{item.label}</strong>
          <small>
            {item.airportCode ?? item.secondary ?? "Airport not reported"} ·
            {item.mapped ? "mapped" : "position unavailable"}
          </small>
        </button>
      {:else}
        <p>No Atlas operations match these controls.</p>
      {/each}
    </div>
  </section>
{/if}

<style>
  .atlas-search {
    display: grid;
    gap: 9px;
    padding: 12px;
    border: 1px solid var(--color-line-faint);
    background: var(--color-surface-soft);
  }
  label {
    display: grid;
    gap: 5px;
    color: var(--color-text-muted);
    font-size: 9px;
  }
  input,
  select {
    min-width: 0;
    border: 1px solid var(--color-line-soft);
    border-radius: 3px;
    padding: 7px 8px;
    color: var(--color-text);
    background: var(--color-surface);
    font: inherit;
  }
  details {
    color: var(--color-text-muted);
    font-size: 9px;
  }
  summary {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    cursor: pointer;
  }
  summary strong {
    color: var(--color-accent);
    text-transform: uppercase;
  }
  .atlas-filter-grid {
    display: grid;
    gap: 7px;
    margin-top: 8px;
  }
  .atlas-results {
    display: grid;
    gap: 5px;
    max-height: 220px;
    overflow: auto;
  }
  .atlas-results button {
    display: grid;
    gap: 3px;
    width: 100%;
    border: 1px solid var(--color-line-faint);
    padding: 8px 9px;
    color: var(--color-text);
    background: var(--color-surface);
    text-align: left;
    cursor: pointer;
  }
  .atlas-results button.selected {
    border-color: var(--color-accent-border);
    box-shadow: inset 3px 0 0 var(--color-accent);
  }
  .atlas-results button > span {
    color: var(--color-highlight);
    font-size: 8px;
    font-weight: 700;
    letter-spacing: 0.09em;
    text-transform: uppercase;
  }
  .atlas-results button small,
  .atlas-results p {
    color: var(--color-text-muted);
    font-size: 9px;
  }
  .atlas-results p {
    margin: 0;
    padding: 12px;
    text-align: center;
  }
</style>
