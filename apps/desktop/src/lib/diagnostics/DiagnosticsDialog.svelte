<script lang="ts">
  import ExplorationSummary from "$lib/exploration/ExplorationSummary.svelte";
  import { formatLocalDateTime } from "$lib/presentation/dateTime";
  import "./diagnostics.css";
  import {
    activeDiagnosticFilterCount,
    defaultDiagnosticFilters,
    diagnosticFilterOptions,
    filterDiagnosticEntries,
    type DiagnosticFilters,
  } from "./presentation";
  import type { DiagnosticLogView } from "./types";

  let {
    open,
    log,
    busy = false,
    errorMessage = "",
    onrefresh,
    onclear,
    onclose,
  }: {
    open: boolean;
    log: DiagnosticLogView;
    busy?: boolean;
    errorMessage?: string;
    onrefresh: () => void;
    onclear: () => void;
    onclose: () => void;
  } = $props();

  let filters = $state<DiagnosticFilters>({ ...defaultDiagnosticFilters });
  const entries = $derived(filterDiagnosticEntries(log.entries, filters));
  const filterOptions = $derived(diagnosticFilterOptions(log.entries));
  const activeFilterCount = $derived(activeDiagnosticFilterCount(filters));

  function resetFilters(): void {
    filters = { ...defaultDiagnosticFilters };
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape" && !busy) onclose();
  }

  function formatTime(value: string): string {
    return formatLocalDateTime(value, value);
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="diagnostics-backdrop">
    <div
      class="diagnostics-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="diagnostics-title"
    >
      <header>
        <div>
          <span class="eyebrow">Local support record</span>
          <h2 id="diagnostics-title">Diagnostics</h2>
        </div>
        <button
          class="diagnostics-close"
          type="button"
          aria-label="Close diagnostics"
          disabled={busy}
          onclick={onclose}>×</button
        >
      </header>

      <div class="diagnostics-boundary">
        <strong>English-only by design</strong>
        <span>
          These entries use stable diagnostic codes and are not processed by
          language packs. WyrmGrid does not include OnAir credentials, company
          identifiers, or raw provider responses in this log.
        </span>
      </div>

      {#if errorMessage}<p class="diagnostics-error" role="alert">
          {errorMessage}
        </p>{/if}

      {#if log.entries.length > 0}
        <section class="diagnostics-explorer" aria-label="Diagnostic log exploration">
          <label class="diagnostics-search">
            <span>Find a code, operation, level, or message</span>
            <input type="search" bind:value={filters.query} />
          </label>
          <details class="diagnostics-filter-panel">
            <summary>
              <span>Filter and sort</span>
              {#if activeFilterCount > 0}<strong>{activeFilterCount} active</strong>{/if}
            </summary>
            <div class="diagnostics-filter-grid">
              <label>
                <span>Recorded level</span>
                <select
                  value={filters.level ?? ""}
                  onchange={(event) =>
                    (filters.level = event.currentTarget.value || null)}
                >
                  <option value="">Any recorded level</option>
                  {#each filterOptions.levels as level}
                    <option value={level}>{level}</option>
                  {/each}
                </select>
              </label>
              <label>
                <span>Operation</span>
                <select
                  value={filters.operation ?? ""}
                  onchange={(event) =>
                    (filters.operation = event.currentTarget.value || null)}
                >
                  <option value="">Any recorded operation</option>
                  {#each filterOptions.operations as operation}
                    <option value={operation}>{operation}</option>
                  {/each}
                </select>
              </label>
              <label>
                <span>Order entries by</span>
                <select bind:value={filters.sort}>
                  <option value="newest">Newest first</option>
                  <option value="oldest">Oldest first</option>
                  <option value="code">Diagnostic code</option>
                </select>
              </label>
            </div>
          </details>
          <ExplorationSummary
            shown={entries.length}
            total={log.entries.length}
            label="diagnostic entries"
            activeFilters={activeFilterCount}
            clearLabel="Clear presentation filters"
            onclear={resetFilters}
          />
        </section>
      {/if}

      <div class="diagnostics-list" aria-label="Diagnostic entries">
        {#each entries as entry, index (`${entry.occurred_at}-${entry.code}-${entry.operation}-${index}`)}
          <article class:error={entry.level === "error"}>
            <div class="diagnostics-entry-heading">
              <code>{entry.code}</code>
              <time datetime={entry.occurred_at}
                >{formatTime(entry.occurred_at)}</time
              >
            </div>
            <strong>{entry.operation}</strong>
            <p>{entry.message}</p>
          </article>
        {:else}
          <div class="diagnostics-empty">
            {#if log.entries.length === 0}
              <strong>No diagnostic entries</strong>
              <span>WyrmGrid has not recorded a failure in this local log.</span>
            {:else}
              <strong>No entries match these controls</strong>
              <span>Clear the presentation filters to review the retained log.</span>
            {/if}
          </div>
        {/each}
      </div>

      <footer>
        <span>
          {log.entries.length} of 200 entries retained · {log.storage ===
          "local_file"
            ? "stored locally"
            : "memory only"}
        </span>
        <div>
          <button type="button" disabled={busy} onclick={onrefresh}
            >Refresh</button
          >
          <button
            class="diagnostics-clear"
            type="button"
            disabled={busy || log.entries.length === 0}
            onclick={onclear}>Clear log</button
          >
        </div>
      </footer>
    </div>
  </div>
{/if}
