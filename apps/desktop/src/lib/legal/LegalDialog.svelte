<script lang="ts">
  import termsText from "../../../../../docs/legal/terms-of-use.md?raw";
  import privacyText from "../../../../../docs/legal/privacy-notice.md?raw";
  import LegalDocument from "./LegalDocument.svelte";
  import type { LegalStatus } from "./client";

  let {
    open,
    required,
    status,
    telemetryEnabled,
    busy,
    errorMessage,
    ontelemetrychange,
    onsubmit,
    onclose,
  }: {
    open: boolean;
    required: boolean;
    status: LegalStatus;
    telemetryEnabled: boolean;
    busy: boolean;
    errorMessage: string;
    ontelemetrychange: (enabled: boolean) => void;
    onsubmit: () => void;
    onclose: () => void;
  } = $props();

  let termsAccepted = $state(false);
  let privacyAcknowledged = $state(false);
  let document = $state<"summary" | "terms" | "privacy">("summary");
  let previouslyOpen = false;

  const canSubmit = $derived(
    !busy && (!required || (termsAccepted && privacyAcknowledged)),
  );

  $effect(() => {
    if (open && !previouslyOpen) {
      termsAccepted = false;
      privacyAcknowledged = false;
      document = "summary";
    }
    previouslyOpen = open;
  });

  function close(): void {
    if (required || busy) return;
    document = "summary";
    onclose();
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape") close();
  }

  function formatAcknowledgedAt(value: string): string {
    const date = new Date(value.includes("T") ? value : `${value}Z`);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="dialog-backdrop">
    <div
      class="legal-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="legal-title"
    >
      <header>
        <div>
          <span class="eyebrow">Privacy before connection</span>
          <h2 id="legal-title">
            {required ? "Welcome to WyrmGrid" : "Privacy & Terms"}
          </h2>
        </div>
        {#if !required}
          <button
            class="close-button"
            type="button"
            aria-label="Close Privacy and Terms"
            onclick={close}>×</button
          >
        {/if}
      </header>

      <nav class="document-tabs" aria-label="Legal documents">
        <button
          class:active={document === "summary"}
          type="button"
          onclick={() => (document = "summary")}>Summary</button
        >
        <button
          class:active={document === "terms"}
          type="button"
          onclick={() => (document = "terms")}>Application Terms</button
        >
        <button
          class:active={document === "privacy"}
          type="button"
          onclick={() => (document = "privacy")}>Privacy Notice</button
        >
      </nav>

      <div class="document-panel">
        {#if document === "summary"}
          <div class="legal-summary">
            <p>
              WyrmGrid is a local-first, independent community application. It
              connects to OnAir only when you provide credentials. Atlas uses
              MapLibre's public demo map service.
            </p>
            <div class="summary-grid">
              <article>
                <strong>OnAir credentials</strong>
                <span
                  >Held for the active session and not written to WyrmGrid's
                  database.</span
                >
              </article>
              <article>
                <strong>Map requests</strong>
                <span
                  >The map provider receives ordinary connection metadata such
                  as your IP address.</span
                >
              </article>
              <article>
                <strong>Error diagnostics</strong>
                <span
                  >Privacy-filtered Sentry reports are optional and off by
                  default.</span
                >
              </article>
              <article>
                <strong>Independent providers</strong>
                <span
                  >OnAir, Sentry, map services, and community plugins operate
                  under their own terms.</span
                >
              </article>
            </div>
            <p class="foundation-note">
              This is foundation-stage software, not an OnAir service or a
              source of real-world flight, financial, legal, or safety advice.
            </p>
          </div>
        {:else}
          <LegalDocument
            text={document === "terms" ? termsText : privacyText}
          />
        {/if}
      </div>

      <label class="telemetry-choice">
        <input
          type="checkbox"
          checked={telemetryEnabled}
          disabled={busy}
          onchange={(event) => ontelemetrychange(event.currentTarget.checked)}
        />
        <span>
          <strong>Send privacy-filtered error reports</strong>
          <small
            >Optional. When the official build supports it, unexpected error
            codes and sanitized technical context go to Sentry's US region. No
            OnAir API key, fleet data, typed input, or local paths are intended
            to be sent.</small
          >
        </span>
      </label>

      {#if required}
        <div class="acknowledgements">
          <label>
            <input
              type="checkbox"
              bind:checked={termsAccepted}
              disabled={busy}
            />
            <span
              >I agree to Application Terms version {status.terms_version}.</span
            >
          </label>
          <label>
            <input
              type="checkbox"
              bind:checked={privacyAcknowledged}
              disabled={busy}
            />
            <span
              >I acknowledge Privacy Notice version {status.privacy_notice_version}.</span
            >
          </label>
        </div>
      {:else if status.acknowledged_at}
        <p class="acceptance-record">
          Current documents acknowledged {formatAcknowledgedAt(
            status.acknowledged_at,
          )}.
        </p>
      {/if}

      {#if errorMessage}<p class="error" role="alert">{errorMessage}</p>{/if}

      <footer>
        {#if !required}
          <button
            class="secondary"
            type="button"
            disabled={busy}
            onclick={close}>Cancel</button
          >
        {/if}
        <button
          class="primary"
          type="button"
          disabled={!canSubmit}
          onclick={onsubmit}
        >
          {busy
            ? "Saving…"
            : required
              ? "Save choices and continue"
              : "Save privacy choice"}
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    z-index: 120;
    display: grid;
    place-items: center;
    padding: 24px;
    background:
      radial-gradient(
        circle at 50% 25%,
        rgba(40, 104, 83, 0.24),
        transparent 35%
      ),
      rgba(1, 7, 6, 0.92);
    backdrop-filter: blur(8px);
  }
  .legal-dialog {
    display: grid;
    grid-template-rows: auto auto minmax(210px, 1fr) auto auto auto;
    width: min(820px, 100%);
    max-height: min(820px, calc(100vh - 48px));
    border: 1px solid rgba(115, 214, 173, 0.25);
    padding: 24px;
    overflow: hidden;
    background: #0a1916;
    box-shadow: 0 30px 110px rgba(0, 0, 0, 0.58);
  }
  header,
  footer,
  .telemetry-choice,
  .acknowledgements label {
    display: flex;
    align-items: center;
  }
  header {
    justify-content: space-between;
    margin-bottom: 16px;
  }
  h2 {
    margin-top: 5px;
    font-family: Georgia, serif;
    font-size: 27px;
    font-weight: 500;
  }
  .close-button {
    width: 34px;
    height: 34px;
    border: 1px solid var(--line);
    color: var(--muted);
    background: transparent;
    font-size: 22px;
    cursor: pointer;
  }
  .document-tabs {
    display: flex;
    gap: 6px;
    margin: 0 0 12px;
  }
  .document-tabs button {
    border: 1px solid var(--line);
    padding: 8px 12px;
    color: var(--muted);
    background: rgba(7, 17, 15, 0.54);
    cursor: pointer;
  }
  .document-tabs button.active {
    border-color: rgba(115, 214, 173, 0.4);
    color: var(--mint);
    background: rgba(115, 214, 173, 0.08);
  }
  .document-panel {
    min-height: 0;
    border: 1px solid var(--line);
    overflow: auto;
    background: rgba(4, 12, 10, 0.58);
  }
  .legal-summary {
    padding: 20px;
  }
  .legal-summary > p {
    color: #c9d8d3;
    font-size: 13px;
    line-height: 1.6;
  }
  .summary-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 9px;
    margin: 16px 0;
  }
  .summary-grid article {
    display: grid;
    gap: 5px;
    border-left: 2px solid var(--gold);
    padding: 11px 12px;
    background: rgba(213, 174, 95, 0.05);
  }
  .summary-grid strong {
    color: #e8eee9;
    font-size: 12px;
  }
  .summary-grid span {
    color: var(--muted);
    font-size: 11px;
    line-height: 1.45;
  }
  .foundation-note {
    color: var(--gold) !important;
  }
  .telemetry-choice {
    gap: 12px;
    margin-top: 14px;
    border: 1px solid rgba(115, 214, 173, 0.2);
    padding: 12px;
    background: rgba(115, 214, 173, 0.05);
    cursor: pointer;
  }
  .telemetry-choice input,
  .acknowledgements input {
    width: 17px;
    height: 17px;
    accent-color: var(--mint);
  }
  .telemetry-choice span {
    display: grid;
    gap: 4px;
  }
  .telemetry-choice strong {
    color: #dce9e4;
    font-size: 12px;
  }
  .telemetry-choice small {
    color: var(--muted);
    font-size: 10px;
    line-height: 1.45;
  }
  .acknowledgements {
    display: grid;
    gap: 8px;
    margin-top: 13px;
  }
  .acknowledgements label {
    gap: 9px;
    color: #cddbd6;
    font-size: 11px;
  }
  .acceptance-record {
    margin-top: 12px;
    color: var(--muted);
    font-size: 10px;
  }
  .error {
    margin-top: 12px;
    border: 1px solid rgba(207, 126, 101, 0.32);
    padding: 9px 11px;
    color: #efb19e;
    background: rgba(207, 126, 101, 0.08);
    font-size: 11px;
  }
  footer {
    justify-content: flex-end;
    gap: 9px;
    margin-top: 17px;
  }
  footer button {
    border: 1px solid var(--line);
    border-radius: 3px;
    padding: 9px 14px;
    cursor: pointer;
  }
  footer button:disabled {
    cursor: not-allowed;
    opacity: 0.52;
  }
  .primary {
    border-color: rgba(115, 214, 173, 0.42) !important;
    color: #06100d;
    background: var(--mint);
    font-weight: 700;
  }
  .secondary {
    color: var(--muted);
    background: transparent;
  }
  @media (max-width: 760px) {
    .summary-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
