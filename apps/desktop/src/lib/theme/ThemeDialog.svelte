<script lang="ts">
  import { translation } from "$lib/i18n/runtime";
  import type { ThemeStatus } from "./types";

  let {
    open,
    status,
    desktopRuntime,
    busy,
    errorMessage,
    onselect,
    onimport,
    onclose,
  }: {
    open: boolean;
    status: ThemeStatus;
    desktopRuntime: boolean;
    busy: boolean;
    errorMessage: string;
    onselect: (themeId: string) => void;
    onimport: (manifestJson: string) => void;
    onclose: () => void;
  } = $props();

  let fileInput = $state<HTMLInputElement>();
  const MAX_MANIFEST_READ_BYTES = 32 * 1024 + 1;

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape" && !busy) onclose();
  }

  async function handleFile(event: Event): Promise<void> {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (file) onimport(await file.slice(0, MAX_MANIFEST_READ_BYTES).text());
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="dialog-backdrop">
    <div
      class="theme-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="theme-title"
    >
      <header>
        <div>
          <span class="eyebrow">{$translation("theme-eyebrow")}</span>
          <h2 id="theme-title">{$translation("theme-title")}</h2>
        </div>
        <button
          class="close-button"
          type="button"
          aria-label={$translation("theme-close")}
          disabled={busy}
          onclick={onclose}>×</button
        >
      </header>

      <p class="introduction">
        {$translation("theme-introduction")}
      </p>

      <div class="theme-list" aria-label={$translation("theme-list-label")}>
        {#each status.themes as theme}
          <button
            class="theme-card"
            class:selected={theme.manifest.id === status.selected_theme_id}
            type="button"
            disabled={busy}
            aria-pressed={theme.manifest.id === status.selected_theme_id}
            onclick={() => onselect(theme.manifest.id)}
          >
            <span class="swatches" aria-hidden="true">
              {#each [theme.manifest.colors.canvas, theme.manifest.colors.surface, theme.manifest.colors.accent, theme.manifest.colors.highlight, theme.manifest.colors.text] as colour}
                <i style:background={colour}></i>
              {/each}
            </span>
            <span class="theme-copy">
              <strong>{theme.manifest.name}</strong>
              <small>
                {theme.built_in
                  ? $translation("theme-built-in")
                  : $translation("theme-community", {
                      author:
                        theme.manifest.author ??
                        $translation("theme-unknown-author"),
                    })}
              </small>
            </span>
            {#if theme.manifest.id === status.selected_theme_id}
              <span class="selected-label">{$translation("theme-active")}</span>
            {/if}
          </button>
        {/each}
      </div>

      {#if errorMessage}<p class="error-message" role="alert">
          {errorMessage}
        </p>{/if}

      <footer>
        <div>
          <strong>{$translation("theme-community-files")}</strong>
          <span>{$translation("theme-community-files-detail")}</span>
        </div>
        <input
          bind:this={fileInput}
          class="file-input"
          type="file"
          accept="application/json,.json"
          onchange={(event) => void handleFile(event)}
        />
        <button
          class="import-button"
          type="button"
          disabled={busy || !desktopRuntime}
          title={desktopRuntime
            ? $translation("theme-import-title")
            : $translation("theme-import-desktop-only")}
          onclick={() => fileInput?.click()}
        >
          {busy
            ? $translation("theme-applying")
            : $translation("theme-import")}
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: 24px;
    background: var(--color-overlay);
    backdrop-filter: blur(9px);
  }
  .theme-dialog {
    width: min(680px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    overflow: auto;
    border: 1px solid var(--color-line-soft);
    border-radius: 8px;
    color: var(--color-text);
    background: var(--color-surface-elevated);
    box-shadow: 0 28px 90px var(--color-shadow);
  }
  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    padding: 20px 22px;
  }
  header {
    border-bottom: 1px solid var(--color-line-faint);
  }
  h2 {
    margin: 5px 0 0;
    font-family: Georgia, serif;
    font-size: 24px;
    font-weight: 500;
  }
  .close-button {
    border: 0;
    color: var(--color-text-muted);
    background: transparent;
    font-size: 26px;
    cursor: pointer;
  }
  .introduction {
    margin: 0;
    padding: 17px 22px 5px;
    color: var(--color-text-muted);
    font-size: 12px;
    line-height: 1.55;
  }
  .theme-list {
    display: grid;
    gap: 9px;
    padding: 14px 22px 20px;
  }
  .theme-card {
    display: grid;
    grid-template-columns: 104px 1fr auto;
    align-items: center;
    gap: 14px;
    width: 100%;
    padding: 13px;
    border: 1px solid var(--color-line-faint);
    border-radius: 6px;
    color: var(--color-text);
    text-align: left;
    background: var(--color-surface);
    cursor: pointer;
  }
  .theme-card:hover,
  .theme-card.selected {
    border-color: var(--color-accent-border);
    background: var(--color-accent-soft);
  }
  .swatches {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    height: 38px;
    overflow: hidden;
    border: 1px solid var(--color-line-soft);
    border-radius: 4px;
  }
  .swatches i {
    display: block;
  }
  .theme-copy {
    display: grid;
    gap: 4px;
  }
  .theme-copy strong,
  footer strong {
    font-size: 12px;
  }
  .theme-copy small,
  footer span {
    color: var(--color-text-muted);
    font-size: 10px;
  }
  .selected-label {
    color: var(--color-accent);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .error-message {
    margin: 0 22px 16px;
    padding: 10px 12px;
    border: 1px solid var(--color-danger-border);
    border-radius: 4px;
    color: var(--color-danger);
    background: var(--color-danger-soft);
    font-size: 11px;
  }
  footer {
    border-top: 1px solid var(--color-line-faint);
    background: var(--color-surface-soft);
  }
  footer div {
    display: grid;
    gap: 3px;
  }
  .file-input {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
  }
  .import-button {
    padding: 9px 13px;
    border: 1px solid var(--color-accent-border);
    border-radius: 4px;
    color: var(--color-accent);
    background: var(--color-accent-soft);
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
</style>
