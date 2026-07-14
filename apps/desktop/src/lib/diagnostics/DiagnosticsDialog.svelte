<script lang="ts">
  import "./diagnostics.css";
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

  const entries = $derived([...log.entries].reverse());

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape" && !busy) onclose();
  }

  function formatTime(value: string): string {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
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
            <strong>No diagnostic entries</strong>
            <span>WyrmGrid has not recorded a failure in this local log.</span>
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
