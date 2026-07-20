<script lang="ts">
  import { translation } from "$lib/i18n/runtime";
  import ThemeAuthoringTool from "./ThemeAuthoringTool.svelte";
  import type { AvailableTheme, ThemeManifest, ThemeStatus } from "./types";

  let {
    open,
    status,
    desktopRuntime,
    busy,
    errorMessage,
    onselect,
    onimport,
    onexport,
    ondownload,
    ondelete,
    onclose,
  }: {
    open: boolean;
    status: ThemeStatus;
    desktopRuntime: boolean;
    busy: boolean;
    errorMessage: string;
    onselect: (themeId: string) => void;
    onimport: (manifestJson: string) => Promise<boolean>;
    onexport: (themeId: string) => Promise<void>;
    ondownload: (manifestJson: string, filename: string) => Promise<void>;
    ondelete: (themeId: string) => void;
    onclose: () => void;
  } = $props();

  let fileInput = $state<HTMLInputElement>();
  let authoringSource = $state<ThemeManifest>();
  let authoringCopy = $state(true);
  const MAX_MANIFEST_READ_BYTES = 32 * 1024 + 1;

  $effect(() => {
    if (!open) authoringSource = undefined;
  });

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape" && !busy) {
      if (authoringSource) authoringSource = undefined;
      else onclose();
    }
  }

  async function handleFile(event: Event): Promise<void> {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (file) {
      await onimport(await file.slice(0, MAX_MANIFEST_READ_BYTES).text());
    }
  }

  function beginAuthoring(theme: ThemeManifest, copy: boolean): void {
    authoringCopy = copy;
    authoringSource = theme;
  }

  async function saveAuthoredTheme(manifestJson: string): Promise<void> {
    if (await onimport(manifestJson)) authoringSource = undefined;
  }

  function confirmDelete(theme: ThemeManifest): void {
    if (
      window.confirm(
        $translation("destructive-theme-delete-confirm", { name: theme.name }),
      )
    ) {
      ondelete(theme.id);
    }
  }

  function activeAvailableTheme(): AvailableTheme {
    return (
      status.themes.find(
        (theme) => theme.manifest.id === status.selected_theme_id,
      ) ?? status.themes[0]
    );
  }

  function localDate(value: string | undefined): string {
    if (!value) return $translation("security-theme-provenance-time-unknown");
    const date = new Date(value);
    return Number.isNaN(date.valueOf())
      ? $translation("security-theme-provenance-time-unknown")
      : new Intl.DateTimeFormat(undefined, {
          dateStyle: "medium",
          timeStyle: "short",
        }).format(date);
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="dialog-backdrop">
    <div
      class="theme-dialog"
      class:authoring-dialog={authoringSource}
      role="dialog"
      aria-modal="true"
      aria-labelledby="theme-title"
    >
      <header class="dialog-header">
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

      {#if authoringSource}
        {#key `${authoringSource.id}:${authoringCopy}`}
          <ThemeAuthoringTool
            source={authoringSource}
            copy={authoringCopy}
            themes={status.themes}
            {desktopRuntime}
            {busy}
            onsave={saveAuthoredTheme}
            {ondownload}
            onclose={() => (authoringSource = undefined)}
          />
        {/key}
      {:else}
        <div class="introduction-row">
          <p class="introduction">
            {$translation("theme-introduction")}
          </p>
          <button
            class="author-button"
            type="button"
            disabled={busy}
            onclick={() =>
              beginAuthoring(activeAvailableTheme().manifest, true)}
          >
            {$translation("theme-authoring-open")}
          </button>
        </div>

        <div class="theme-list" aria-label={$translation("theme-list-label")}>
          {#each status.themes as theme}
            <article
              class="theme-card"
              class:selected={theme.manifest.id === status.selected_theme_id}
            >
              <button
                class="theme-choice"
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
                    {theme.provenance.source === "bundled"
                      ? $translation("security-theme-provenance-bundled")
                      : $translation("security-theme-provenance-imported", {
                          date: localDate(theme.provenance.imported_at),
                        })}
                  </small>
                  {#if theme.provenance.source === "local_import" && theme.provenance.updated_at !== theme.provenance.imported_at}
                    <small>
                      {$translation("security-theme-provenance-updated", {
                        date: localDate(theme.provenance.updated_at),
                      })}
                    </small>
                  {/if}
                  <small>
                    {$translation("security-theme-author-claim", {
                      author:
                        theme.manifest.author ??
                        $translation("theme-unknown-author"),
                    })}
                  </small>
                </span>
                {#if theme.manifest.id === status.selected_theme_id}
                  <span class="selected-label"
                    >{$translation("theme-active")}</span
                  >
                {/if}
              </button>
              <div class="theme-actions">
                <button
                  type="button"
                  disabled={busy}
                  onclick={() =>
                    beginAuthoring(
                      theme.manifest,
                      theme.provenance.source === "bundled",
                    )}
                >
                  {$translation(
                    theme.provenance.source === "bundled"
                      ? "theme-create-copy"
                      : "theme-edit",
                  )}
                </button>
                <button
                  type="button"
                  disabled={busy || !desktopRuntime}
                  onclick={() => void onexport(theme.manifest.id)}
                >
                  {$translation("theme-export")}
                </button>
                {#if theme.provenance.source === "local_import"}
                  <button
                    class="delete-button"
                    type="button"
                    disabled={busy || !desktopRuntime}
                    onclick={() => confirmDelete(theme.manifest)}
                  >
                    {$translation("destructive-theme-delete")}
                  </button>
                {/if}
              </div>
            </article>
          {/each}
        </div>
      {/if}

      {#if errorMessage}<p class="error-message" role="alert">
          {errorMessage}
        </p>{/if}

      {#if !authoringSource}
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
      {/if}
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
    width: min(760px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    overflow: auto;
    border: 1px solid var(--color-line-soft);
    border-radius: 8px;
    color: var(--color-text);
    background: var(--color-surface-elevated);
    box-shadow: 0 28px 90px var(--color-shadow);
  }
  .theme-dialog.authoring-dialog {
    width: min(1080px, calc(100vw - 48px));
  }
  .dialog-header,
  footer,
  .introduction-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    padding: 20px 22px;
  }
  .dialog-header {
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
  .introduction-row {
    align-items: start;
    padding-bottom: 5px;
  }
  .introduction {
    max-width: 520px;
    margin: 0;
    color: var(--color-text-muted);
    font-size: 12px;
    line-height: 1.55;
  }
  .author-button,
  .import-button {
    flex: none;
    padding: 9px 13px;
    border: 1px solid var(--color-accent-border);
    border-radius: 4px;
    color: var(--color-accent);
    background: var(--color-accent-soft);
    cursor: pointer;
  }
  .theme-list {
    display: grid;
    gap: 9px;
    padding: 14px 22px 20px;
  }
  .theme-card {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: stretch;
    overflow: hidden;
    border: 1px solid var(--color-line-faint);
    border-radius: 6px;
    background: var(--color-surface);
  }
  .theme-card:hover,
  .theme-card.selected {
    border-color: var(--color-accent-border);
    background: var(--color-accent-soft);
  }
  .theme-choice {
    display: grid;
    grid-template-columns: 104px minmax(0, 1fr) auto;
    align-items: center;
    gap: 14px;
    min-width: 0;
    padding: 13px;
    border: 0;
    color: var(--color-text);
    text-align: left;
    background: transparent;
    cursor: pointer;
  }
  .swatches {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    height: 42px;
    overflow: hidden;
    border: 1px solid var(--color-line-soft);
    border-radius: 4px;
  }
  .swatches i {
    display: block;
  }
  .theme-copy {
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .theme-copy strong,
  footer strong {
    font-size: 12px;
  }
  .theme-copy small,
  footer span {
    overflow: hidden;
    color: var(--color-text-muted);
    font-size: 9px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .selected-label {
    color: var(--color-accent);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .theme-actions {
    display: grid;
    align-content: center;
    min-width: 88px;
    border-left: 1px solid var(--color-line-faint);
  }
  .theme-actions button {
    padding: 6px 10px;
    border: 0;
    color: var(--color-text-muted);
    background: transparent;
    font-size: 9px;
    cursor: pointer;
  }
  .theme-actions button:hover {
    color: var(--color-accent);
    background: var(--color-accent-soft);
  }
  .theme-actions .delete-button:hover {
    color: var(--color-danger);
    background: var(--color-danger-soft);
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
  button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  @media (max-width: 650px) {
    .theme-card {
      grid-template-columns: 1fr;
    }
    .theme-choice {
      grid-template-columns: 76px minmax(0, 1fr);
    }
    .selected-label {
      display: none;
    }
    .theme-actions {
      grid-template-columns: repeat(3, 1fr);
      border-top: 1px solid var(--color-line-faint);
      border-left: 0;
    }
  }
</style>
