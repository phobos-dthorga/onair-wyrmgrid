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
              connects to OnAir only when you provide credentials. Successful
              fleet, FBO, and pending-job observations are retained in the local
              Hoard. Atlas uses MapLibre's public demo map service. Dispatch
              contacts SimBrief only when you explicitly import the latest OFP,
              and contacts AviationWeather.gov only when you request airport
              weather.
            </p>
            <div class="summary-grid">
              <article>
                <strong>Flight recordings</strong>
                <span
                  >Manual recording or an optional default-off automatic setting
                  stores local telemetry. A current sanitized SimBrief plan may
                  be retained with that recording for later comparison.</span
                >
              </article>
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
                  >OnAir, SimBrief, AviationWeather.gov, Sentry, map services,
                  and community plugins operate under their own terms.</span
                >
              </article>
              <article>
                <strong>Local customisation</strong>
                <span
                  >Imported themes and language packs remain in WyrmGrid's
                  local database and are not sent to translation services.</span
                >
              </article>
              <article>
                <strong>Encrypted local data</strong>
                <span
                  >SQLCipher protects WyrmGrid's database at rest. Portable
                  backups are complete encrypted copies protected by a password
                  you choose and WyrmGrid cannot recover.</span
                >
              </article>
              <article>
                <strong>Photosensitivity</strong>
                <span
                  >Future weather and warning effects may include lightning or
                  flashing light. Reduced-flash presentation will remain on by
                  default before stronger effects are offered.</span
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
        var(--color-accent-soft),
        transparent 35%
      ),
      var(--color-overlay-strong);
    backdrop-filter: blur(8px);
  }
  .legal-dialog {
    display: grid;
    grid-template-rows: auto auto minmax(210px, 1fr) auto auto auto;
    width: min(820px, 100%);
    max-height: min(820px, calc(100vh - 48px));
    border: 1px solid var(--color-accent-border);
    padding: 24px;
    overflow: hidden;
    background: var(--color-surface);
    box-shadow: 0 30px 110px var(--color-shadow);
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
    border: 1px solid var(--color-line-faint);
    color: var(--color-text-muted);
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
    border: 1px solid var(--color-line-faint);
    padding: 8px 12px;
    color: var(--color-text-muted);
    background: var(--color-overlay);
    cursor: pointer;
  }
  .document-tabs button.active {
    border-color: var(--color-accent-border);
    color: var(--color-accent);
    background: var(--color-accent-soft);
  }
  .document-panel {
    min-height: 0;
    border: 1px solid var(--color-line-faint);
    overflow: auto;
    background: var(--color-overlay);
  }
  .legal-summary {
    padding: 20px;
  }
  .legal-summary > p {
    color: var(--color-text-muted);
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
    border-left: 2px solid var(--color-highlight);
    padding: 11px 12px;
    background: var(--color-highlight-soft);
  }
  .summary-grid strong {
    color: var(--color-text);
    font-size: 12px;
  }
  .summary-grid span {
    color: var(--color-text-muted);
    font-size: 11px;
    line-height: 1.45;
  }
  .foundation-note {
    color: var(--color-highlight) !important;
  }
  .telemetry-choice {
    gap: 12px;
    margin-top: 14px;
    border: 1px solid var(--color-accent-border);
    padding: 12px;
    background: var(--color-accent-soft);
    cursor: pointer;
  }
  .telemetry-choice input,
  .acknowledgements input {
    width: 17px;
    height: 17px;
    accent-color: var(--color-accent);
  }
  .telemetry-choice span {
    display: grid;
    gap: 4px;
  }
  .telemetry-choice strong {
    color: var(--color-text);
    font-size: 12px;
  }
  .telemetry-choice small {
    color: var(--color-text-muted);
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
    color: var(--color-text-muted);
    font-size: 11px;
  }
  .acceptance-record {
    margin-top: 12px;
    color: var(--color-text-muted);
    font-size: 10px;
  }
  .error {
    margin-top: 12px;
    border: 1px solid var(--color-danger-border);
    padding: 9px 11px;
    color: var(--color-danger);
    background: var(--color-danger-soft);
    font-size: 11px;
  }
  footer {
    justify-content: flex-end;
    gap: 9px;
    margin-top: 17px;
  }
  footer button {
    border: 1px solid var(--color-line-faint);
    border-radius: 3px;
    padding: 9px 14px;
    cursor: pointer;
  }
  footer button:disabled {
    cursor: not-allowed;
    opacity: 0.52;
  }
  .primary {
    border-color: var(--color-accent-border) !important;
    color: var(--color-canvas);
    background: var(--color-accent);
    font-weight: 700;
  }
  .secondary {
    color: var(--color-text-muted);
    background: transparent;
  }
  @media (max-width: 760px) {
    .summary-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
