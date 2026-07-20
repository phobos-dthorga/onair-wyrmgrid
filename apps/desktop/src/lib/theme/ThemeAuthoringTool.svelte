<script lang="ts">
  import { translation } from "$lib/i18n/runtime";
  import {
    serialiseThemeDraft,
    themeColourRoles,
    themeContrastChecks,
    themeDraftFrom,
    visualDuplicate,
  } from "./authoring";
  import type { AvailableTheme, ThemeManifest } from "./types";

  let {
    source,
    copy,
    themes,
    desktopRuntime,
    busy,
    onsave,
    onclose,
  }: {
    source: ThemeManifest;
    copy: boolean;
    themes: AvailableTheme[];
    desktopRuntime: boolean;
    busy: boolean;
    onsave: (manifestJson: string) => void;
    onclose: () => void;
  } = $props();

  let draft = $state(initialDraft());
  const contrastChecks = $derived(themeContrastChecks(draft));
  const failedChecks = $derived(
    contrastChecks.filter((check) => !check.passes),
  );
  const duplicate = $derived(visualDuplicate(draft, themes));
  const metadataValid = $derived(
    /^(?=.{3,64}$)[a-z0-9](?:[a-z0-9-]*[a-z0-9])$/.test(draft.id) &&
      !draft.id.startsWith("wyrmgrid-") &&
      draft.name.trim().length > 0 &&
      draft.name.length <= 64 &&
      (draft.author?.length ?? 0) <= 80,
  );
  const readyToSave = $derived(
    desktopRuntime && metadataValid && failedChecks.length === 0 && !duplicate,
  );

  function initialDraft(): ThemeManifest {
    return copy
      ? themeDraftFrom(source)
      : {
          ...source,
          author: source.author ?? "",
          colors: { ...source.colors },
          chart_palette: [...source.chart_palette],
        };
  }

  function updateColour(key: keyof ThemeManifest["colors"], value: string) {
    draft.colors[key] = value.toUpperCase();
  }

  function updatePalette(index: number, value: string): void {
    draft.chart_palette[index] = value.toUpperCase();
  }

  function addPaletteColour(): void {
    if (draft.chart_palette.length < 8) {
      draft.chart_palette.push(draft.colors.accent);
    }
  }

  function removePaletteColour(index: number): void {
    if (draft.chart_palette.length > 3) draft.chart_palette.splice(index, 1);
  }

  function downloadDraft(): void {
    const url = URL.createObjectURL(
      new Blob([serialiseThemeDraft(draft)], { type: "application/json" }),
    );
    const link = document.createElement("a");
    link.href = url;
    link.download = `${draft.id || "wyrmgrid-theme"}.wyrmgrid-theme.json`;
    link.click();
    URL.revokeObjectURL(url);
  }
</script>

<section class="authoring" aria-labelledby="theme-authoring-title">
  <header class="authoring-header">
    <div>
      <span class="eyebrow">{$translation("theme-authoring-eyebrow")}</span>
      <h3 id="theme-authoring-title">
        {$translation(
          copy ? "theme-authoring-copy-title" : "theme-authoring-edit-title",
          { name: source.name },
        )}
      </h3>
    </div>
    <button
      type="button"
      class="quiet-button"
      disabled={busy}
      onclick={onclose}
    >
      {$translation("theme-authoring-back")}
    </button>
  </header>

  <p class="authoring-introduction">
    {$translation("security-theme-authoring-introduction")}
  </p>

  <div class="authoring-layout">
    <div class="editor-column">
      <fieldset class="metadata-fields">
        <legend>{$translation("theme-authoring-identity")}</legend>
        <label>
          <span>{$translation("theme-authoring-id")}</span>
          <input
            type="text"
            maxlength="64"
            value={draft.id}
            aria-invalid={!metadataValid}
            oninput={(event) =>
              (draft.id = event.currentTarget.value.toLowerCase())}
          />
        </label>
        <label>
          <span>{$translation("theme-authoring-name")}</span>
          <input type="text" maxlength="64" bind:value={draft.name} />
        </label>
        <label>
          <span>{$translation("theme-authoring-author")}</span>
          <input
            type="text"
            maxlength="80"
            value={draft.author ?? ""}
            oninput={(event) => (draft.author = event.currentTarget.value)}
          />
        </label>
        <small>{$translation("security-theme-authoring-author-detail")}</small>
      </fieldset>

      <fieldset>
        <legend>{$translation("theme-authoring-colours")}</legend>
        <div class="colour-grid">
          {#each themeColourRoles as role}
            <label class="colour-field">
              <span>{$translation(role.labelKey)}</span>
              <span class="colour-control">
                <input
                  type="color"
                  value={draft.colors[role.key]}
                  oninput={(event) =>
                    updateColour(role.key, event.currentTarget.value)}
                />
                <code>{draft.colors[role.key]}</code>
              </span>
            </label>
          {/each}
        </div>
      </fieldset>

      <fieldset>
        <legend>{$translation("theme-authoring-chart-palette")}</legend>
        <div class="palette-grid">
          {#each draft.chart_palette as colour, index}
            <label class="palette-field">
              <span
                >{$translation("theme-authoring-chart-colour", {
                  index: index + 1,
                })}</span
              >
              <input
                type="color"
                value={colour}
                oninput={(event) =>
                  updatePalette(index, event.currentTarget.value)}
              />
              <button
                type="button"
                class="remove-colour"
                aria-label={$translation(
                  "theme-authoring-remove-chart-colour",
                  {
                    index: index + 1,
                  },
                )}
                disabled={busy || draft.chart_palette.length <= 3}
                onclick={() => removePaletteColour(index)}>×</button
              >
            </label>
          {/each}
          <button
            type="button"
            class="quiet-button add-colour"
            disabled={busy || draft.chart_palette.length >= 8}
            onclick={addPaletteColour}
          >
            {$translation("theme-authoring-add-chart-colour")}
          </button>
        </div>
      </fieldset>
    </div>

    <aside class="preview-column">
      <div
        class="interface-preview"
        style:background={draft.colors.canvas}
        style:color={draft.colors.text}
      >
        <div
          class="preview-panel"
          style:background={draft.colors.surface}
          style:border-color={draft.colors.line}
        >
          <span style:color={draft.colors.text_muted}
            >{$translation("theme-preview-eyebrow")}</span
          >
          <strong>{$translation("theme-preview-title")}</strong>
          <p style:color={draft.colors.text_muted}>
            {$translation("theme-preview-detail")}
          </p>
          <div class="preview-actions">
            <i
              style:background={draft.colors.accent}
              style:color={draft.colors.canvas}
              >{$translation("theme-preview-primary")}</i
            >
            <i
              style:border-color={draft.colors.highlight}
              style:color={draft.colors.highlight}
              >{$translation("theme-preview-secondary")}</i
            >
          </div>
          <div
            class="preview-alert"
            style:border-color={draft.colors.danger}
            style:color={draft.colors.danger}
          >
            {$translation("theme-preview-alert")}
          </div>
          <div class="preview-map" style:background={draft.colors.map_halo}>
            <i style:background={draft.colors.map_aircraft}></i>
            <i style:background={draft.colors.map_fbo}></i>
            <span style:color={draft.colors.map_label}>YSSY</span>
          </div>
          <div class="preview-chart" aria-hidden="true">
            {#each draft.chart_palette as colour, index}
              <i style:background={colour} style:height={`${38 + index * 8}%`}
              ></i>
            {/each}
          </div>
        </div>
      </div>

      <section class="contrast-results" aria-live="polite">
        <header>
          <strong>{$translation("theme-contrast-title")}</strong>
          <span class:passing={failedChecks.length === 0}>
            {failedChecks.length === 0
              ? $translation("theme-contrast-passing")
              : $translation("theme-contrast-failing", {
                  count: failedChecks.length,
                })}
          </span>
        </header>
        <div class="contrast-list">
          {#each contrastChecks as check}
            <div class:failed={!check.passes}>
              <span>{$translation(check.labelKey, check.labelArguments)}</span>
              <strong
                >{check.ratio.toFixed(2)}:1 / {check.minimum.toFixed(
                  1,
                )}:1</strong
              >
            </div>
          {/each}
        </div>
      </section>

      {#if duplicate}
        <p class="duplicate-warning" role="status">
          {$translation("theme-authoring-duplicate", {
            name: duplicate.manifest.name,
          })}
        </p>
      {/if}
      {#if !metadataValid}
        <p class="metadata-warning" role="status">
          {$translation("theme-authoring-invalid-metadata")}
        </p>
      {/if}

      <div class="authoring-actions">
        <button
          type="button"
          class="quiet-button"
          disabled={busy || !metadataValid}
          onclick={downloadDraft}
        >
          {$translation("theme-authoring-download")}
        </button>
        <button
          type="button"
          class="save-button"
          disabled={busy || !readyToSave}
          title={desktopRuntime
            ? $translation("theme-authoring-save-title")
            : $translation("theme-import-desktop-only")}
          onclick={() => onsave(serialiseThemeDraft(draft))}
        >
          {busy
            ? $translation("theme-applying")
            : $translation("theme-authoring-save")}
        </button>
      </div>
      <small class="authority-note">
        {$translation("security-theme-authoring-authority")}
      </small>
    </aside>
  </div>
</section>

<style>
  .authoring {
    padding: 20px 22px 24px;
  }
  .authoring-header,
  .contrast-results header,
  .authoring-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
  }
  h3 {
    margin: 4px 0 0;
    font-family: Georgia, serif;
    font-size: 20px;
    font-weight: 500;
  }
  .authoring-introduction {
    margin: 12px 0 18px;
    color: var(--color-text-muted);
    font-size: 11px;
    line-height: 1.55;
  }
  .authoring-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.25fr) minmax(290px, 0.75fr);
    gap: 18px;
    align-items: start;
  }
  .editor-column {
    display: grid;
    gap: 12px;
  }
  fieldset,
  .contrast-results {
    margin: 0;
    padding: 13px;
    border: 1px solid var(--color-line-faint);
    border-radius: 6px;
    background: var(--color-surface);
  }
  legend {
    padding: 0 5px;
    color: var(--color-text-muted);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }
  .metadata-fields {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }
  .metadata-fields label,
  .colour-field {
    display: grid;
    gap: 5px;
    color: var(--color-text-muted);
    font-size: 9px;
  }
  .metadata-fields input {
    min-width: 0;
    padding: 8px;
    border: 1px solid var(--color-line-soft);
    border-radius: 4px;
    color: var(--color-text);
    background: var(--color-surface-soft);
  }
  .metadata-fields small {
    grid-column: 1 / -1;
    color: var(--color-text-muted);
    font-size: 9px;
  }
  .colour-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 9px;
  }
  .colour-control {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 4px 6px;
    border: 1px solid var(--color-line-faint);
    border-radius: 4px;
    background: var(--color-surface-soft);
  }
  input[type="color"] {
    width: 28px;
    height: 24px;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
  }
  code {
    color: var(--color-text);
    font-size: 9px;
  }
  .palette-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .palette-field {
    display: grid;
    grid-template-columns: auto auto;
    align-items: center;
    gap: 3px;
    color: var(--color-text-muted);
    font-size: 8px;
  }
  .palette-field > span {
    grid-column: 1 / -1;
  }
  .remove-colour {
    border: 0;
    color: var(--color-text-muted);
    background: transparent;
    cursor: pointer;
  }
  .add-colour {
    align-self: end;
  }
  .preview-column {
    position: sticky;
    top: 0;
    display: grid;
    gap: 11px;
  }
  .interface-preview {
    padding: 18px;
    border-radius: 7px;
  }
  .preview-panel {
    display: grid;
    gap: 10px;
    padding: 16px;
    border: 1px solid;
    border-radius: 5px;
  }
  .preview-panel > span {
    font-size: 8px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .preview-panel > strong {
    font-family: Georgia, serif;
    font-size: 18px;
  }
  .preview-panel p {
    margin: 0;
    font-size: 10px;
    line-height: 1.45;
  }
  .preview-actions {
    display: flex;
    gap: 7px;
  }
  .preview-actions i {
    padding: 6px 8px;
    border: 1px solid transparent;
    border-radius: 3px;
    font-size: 8px;
    font-style: normal;
    font-weight: 700;
  }
  .preview-alert {
    padding: 7px;
    border: 1px solid;
    border-radius: 3px;
    font-size: 8px;
  }
  .preview-map {
    position: relative;
    height: 56px;
    overflow: hidden;
    border-radius: 3px;
  }
  .preview-map i {
    position: absolute;
    width: 9px;
    height: 9px;
    border-radius: 50%;
  }
  .preview-map i:first-child {
    top: 14px;
    left: 28px;
  }
  .preview-map i:nth-child(2) {
    right: 38px;
    bottom: 13px;
  }
  .preview-map span {
    position: absolute;
    top: 22px;
    left: 47%;
    font-size: 9px;
    font-weight: 700;
  }
  .preview-chart {
    display: flex;
    align-items: end;
    gap: 4px;
    height: 48px;
  }
  .preview-chart i {
    flex: 1;
    border-radius: 2px 2px 0 0;
  }
  .contrast-results {
    padding: 11px;
  }
  .contrast-results header {
    margin-bottom: 7px;
    font-size: 9px;
  }
  .contrast-results header span {
    color: var(--color-danger);
  }
  .contrast-results header span.passing {
    color: var(--color-success);
  }
  .contrast-list {
    display: grid;
    gap: 2px;
    max-height: 174px;
    overflow: auto;
  }
  .contrast-list div {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 4px 5px;
    color: var(--color-text-muted);
    font-size: 8px;
  }
  .contrast-list div.failed {
    color: var(--color-danger);
    background: var(--color-danger-soft);
  }
  .duplicate-warning,
  .metadata-warning {
    margin: 0;
    padding: 8px 10px;
    border: 1px solid var(--color-highlight-border);
    border-radius: 4px;
    color: var(--color-highlight);
    background: var(--color-highlight-soft);
    font-size: 9px;
    line-height: 1.45;
  }
  .quiet-button,
  .save-button {
    padding: 8px 11px;
    border: 1px solid var(--color-line-soft);
    border-radius: 4px;
    color: var(--color-text-muted);
    background: var(--color-surface-soft);
    cursor: pointer;
  }
  .save-button {
    border-color: var(--color-accent-border);
    color: var(--color-accent);
    background: var(--color-accent-soft);
  }
  .authority-note {
    color: var(--color-text-muted);
    font-size: 8px;
    line-height: 1.45;
  }
  button:disabled {
    opacity: 0.52;
    cursor: not-allowed;
  }
  @media (max-width: 860px) {
    .authoring-layout {
      grid-template-columns: 1fr;
    }
    .preview-column {
      position: static;
    }
  }
  @media (max-width: 620px) {
    .metadata-fields,
    .colour-grid {
      grid-template-columns: 1fr 1fr;
    }
  }
</style>
