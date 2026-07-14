<script lang="ts">
  import { translation } from "./runtime";
  import type { LanguageStatus } from "./types";
  import "./language.css";

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
    status: LanguageStatus;
    desktopRuntime: boolean;
    busy: boolean;
    errorMessage: string;
    onselect: (packId: string) => void;
    onimport: (manifestJson: string) => void;
    onclose: () => void;
  } = $props();

  let fileInput = $state<HTMLInputElement>();
  const MAX_MANIFEST_READ_BYTES = 256 * 1024 + 1;

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape" && !busy) onclose();
  }

  async function handleFile(event: Event): Promise<void> {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (file) onimport(await file.slice(0, MAX_MANIFEST_READ_BYTES).text());
  }

  function trustLabel(trust: string): string {
    if (trust === "built_in") return $translation("language-built-in");
    if (trust === "reviewed") return $translation("language-reviewed");
    return $translation("language-community");
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="language-backdrop">
    <div
      class="language-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="language-title"
    >
      <header>
        <div>
          <span class="eyebrow">{$translation("language-eyebrow")}</span>
          <h2 id="language-title">{$translation("language-title")}</h2>
        </div>
        <button
          class="language-close"
          type="button"
          aria-label={$translation("language-close")}
          disabled={busy}
          onclick={onclose}>×</button
        >
      </header>

      <p class="language-introduction">
        {$translation("language-introduction")}
      </p>

      <div class="language-list" aria-label={$translation("language-list-label")}>
        {#each status.packs as pack}
          <button
            class="language-card"
            class:selected={pack.manifest.id === status.selected_language_pack_id}
            type="button"
            disabled={busy}
            aria-pressed={pack.manifest.id === status.selected_language_pack_id}
            onclick={() => onselect(pack.manifest.id)}
          >
            <span class="language-locale" lang={pack.manifest.locale}
              >{pack.manifest.locale}</span
            >
            <span class="language-copy">
              <strong>{pack.manifest.name}</strong>
              <small>
                {trustLabel(pack.trust)}
                {#if pack.manifest.author}
                  · {$translation("language-by-author", {
                    author: pack.manifest.author,
                  })}
                {/if}
              </small>
              <small>
                {$translation("language-coverage", {
                  translated: pack.translated_messages,
                  total: pack.eligible_messages,
                })}
                · {$translation(
                  pack.manifest.direction === "right_to_left"
                    ? "language-direction-rtl"
                    : "language-direction-ltr",
                )}
              </small>
            </span>
            {#if pack.manifest.id === status.selected_language_pack_id}
              <span class="language-active">{$translation("language-active")}</span>
            {/if}
          </button>
        {/each}
      </div>

      <aside class="language-boundary">
        <strong>{$translation("language-source-version", {
          version: status.active_pack.source_catalog_version,
        })}</strong>
        <span>{$translation("security-language-boundary")}</span>
      </aside>

      {#if errorMessage}<p class="language-error" role="alert">
          {errorMessage}
        </p>{/if}

      <footer>
        <div>
          <strong>{$translation("language-community-files")}</strong>
          <span>{$translation("language-community-files-detail")}</span>
        </div>
        <input
          bind:this={fileInput}
          class="language-file-input"
          type="file"
          accept="application/json,.json"
          onchange={(event) => void handleFile(event)}
        />
        <button
          class="language-import"
          type="button"
          disabled={busy || !desktopRuntime}
          title={desktopRuntime
            ? $translation("language-import-title")
            : $translation("language-import-desktop-only")}
          onclick={() => fileInput?.click()}
        >
          {busy
            ? $translation("language-applying")
            : $translation("language-import")}
        </button>
      </footer>
    </div>
  </div>
{/if}
