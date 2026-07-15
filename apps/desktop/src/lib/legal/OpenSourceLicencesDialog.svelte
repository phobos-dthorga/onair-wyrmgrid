<script lang="ts">
  import { translation } from "$lib/i18n/runtime";
  import sqlCipherLicence from "./sqlcipher-license.txt?raw";

  let {
    open,
    onclose,
  }: {
    open: boolean;
    onclose: () => void;
  } = $props();

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="licences-backdrop">
    <div
      class="licences-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="licences-title"
    >
      <header>
        <div>
          <span class="eyebrow">{$translation("data-protection-licences-eyebrow")}</span>
          <h2 id="licences-title">{$translation("data-protection-licences-title")}</h2>
          <p>{$translation("data-protection-licences-detail")}</p>
        </div>
        <button
          type="button"
          aria-label={$translation("data-protection-licences-close")}
          onclick={onclose}>×</button
        >
      </header>

      <section>
        <h3>SQLCipher Community Edition</h3>
        <p>{$translation("data-protection-sqlcipher-notice")}</p>
        <pre>{sqlCipherLicence}</pre>
      </section>

      <section>
        <h3>OpenSSL</h3>
        <p>{$translation("data-protection-openssl-notice")}</p>
        <a
          href="https://www.openssl.org/source/license.html"
          target="_blank"
          rel="noreferrer"
          >openssl.org/source/license.html</a
        >
      </section>

      <footer>
        <button type="button" onclick={onclose}
          >{$translation("data-protection-done")}</button
        >
      </footer>
    </div>
  </div>
{/if}

<style>
  .licences-backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    padding: 24px;
    background: var(--color-overlay);
    backdrop-filter: blur(10px);
  }
  .licences-dialog {
    width: min(780px, calc(100vw - 48px));
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
    padding: 21px 24px;
  }
  header {
    border-bottom: 1px solid var(--color-line-faint);
  }
  header button {
    border: 0;
    color: var(--color-text-muted);
    background: transparent;
    font-size: 27px;
    cursor: pointer;
  }
  h2,
  h3,
  p {
    margin: 0;
  }
  h2,
  h3 {
    margin-top: 5px;
    font-family: Georgia, serif;
    font-weight: 500;
  }
  h2 {
    font-size: 28px;
  }
  h3 {
    font-size: 20px;
  }
  header p,
  section p {
    margin-top: 7px;
    color: var(--color-text-muted);
    line-height: 1.5;
  }
  section {
    padding: 20px 24px;
    border-bottom: 1px solid var(--color-line-faint);
  }
  pre {
    overflow: auto;
    max-height: 290px;
    margin: 14px 0 0;
    border: 1px solid var(--color-line-faint);
    padding: 14px;
    color: var(--color-text-muted);
    background: var(--color-surface);
    font-size: 10px;
    line-height: 1.55;
    white-space: pre-wrap;
  }
  a {
    display: inline-block;
    margin-top: 10px;
    color: var(--color-accent);
  }
  footer {
    justify-content: flex-end;
  }
  footer button {
    border: 1px solid var(--color-accent);
    border-radius: 4px;
    padding: 9px 12px;
    color: var(--color-canvas);
    background: var(--color-accent);
    font: inherit;
    cursor: pointer;
  }
  @media (max-width: 700px) {
    .licences-backdrop {
      padding: 12px;
    }
    .licences-dialog {
      width: calc(100vw - 24px);
      max-height: calc(100vh - 24px);
    }
  }
</style>
