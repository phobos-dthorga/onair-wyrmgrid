<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { OnAirConnectionStatus } from "./types";

  let {
    open,
    status,
    onclose,
    onstatuschange,
  }: {
    open: boolean;
    status: OnAirConnectionStatus;
    onclose: () => void;
    onstatuschange: (status: OnAirConnectionStatus) => void;
  } = $props();

  let companyId = $state("");
  let apiKey = $state("");
  let revealApiKey = $state(false);
  let submitting = $state(false);
  let errorMessage = $state("");

  function safeError(error: unknown): string {
    return typeof error === "string" && error.length > 0
      ? error
      : "WyrmGrid could not complete the connection request.";
  }

  async function connect(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    submitting = true;
    errorMessage = "";

    try {
      const connected = await invoke<OnAirConnectionStatus>("connect_onair", {
        companyId: companyId.trim(),
        apiKey,
      });
      apiKey = "";
      companyId = "";
      revealApiKey = false;
      onstatuschange(connected);
    } catch (error) {
      errorMessage = safeError(error);
    } finally {
      submitting = false;
    }
  }

  async function disconnect(): Promise<void> {
    submitting = true;
    errorMessage = "";
    try {
      const disconnected = await invoke<OnAirConnectionStatus>("disconnect_onair");
      apiKey = "";
      companyId = "";
      onstatuschange(disconnected);
    } catch (error) {
      errorMessage = safeError(error);
    } finally {
      submitting = false;
    }
  }

  function close(): void {
    apiKey = "";
    errorMessage = "";
    revealApiKey = false;
    onclose();
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape" && !submitting) close();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="dialog-backdrop">
    <div class="connection-dialog" role="dialog" aria-modal="true" aria-labelledby="connection-title">
      <header>
        <div>
          <span class="eyebrow">Secure data link</span>
          <h2 id="connection-title">OnAir connection</h2>
        </div>
        <button class="close-button" type="button" aria-label="Close connection dialog" onclick={close}>×</button>
      </header>

      {#if status.connected && status.company}
        <div class="connected-company">
          <span class="status-light" aria-hidden="true"></span>
          <div>
            <span>Connected for this session</span>
            <strong>{status.company.name}</strong>
            {#if status.company.airline_code}<small>{status.company.airline_code}</small>{/if}
          </div>
        </div>

        <p class="explanation">
          The API key is held only in Rust memory and will be forgotten when WyrmGrid closes.
        </p>

        {#if errorMessage}<p class="error" role="alert">{errorMessage}</p>{/if}

        <div class="dialog-actions">
          <button class="secondary" type="button" onclick={close}>Done</button>
          <button class="danger" type="button" disabled={submitting} onclick={disconnect}>
            {submitting ? "Disconnecting…" : "Disconnect"}
          </button>
        </div>
      {:else}
        <p class="explanation">
          Copy the Company ID and API Key from OnAir Client → Options → Global Settings. WyrmGrid uses them only with OnAir's read-only public API.
        </p>

        <div class="credential-source-warning" role="note">
          <strong>Use OnAir Client—not OnAir Companion</strong>
          <span>Companion may display similar API details, but they are not valid for this connection.</span>
        </div>

        <form onsubmit={connect}>
          <label for="company-id">Company ID</label>
          <input
            id="company-id"
            bind:value={companyId}
            type="text"
            inputmode="text"
            autocomplete="off"
            autocapitalize="none"
            spellcheck="false"
            placeholder="00000000-0000-0000-0000-000000000000"
            required
            disabled={submitting}
          />

          <label for="api-key">API Key</label>
          <div class="secret-field">
            <input
              id="api-key"
              bind:value={apiKey}
              type={revealApiKey ? "text" : "password"}
              autocomplete="off"
              autocapitalize="none"
              spellcheck="false"
              required
              disabled={submitting}
            />
            <button
              type="button"
              aria-label={revealApiKey ? "Hide API key" : "Show API key"}
              onclick={() => (revealApiKey = !revealApiKey)}
            >{revealApiKey ? "Hide" : "Show"}</button>
          </div>

          <div class="session-notice">
            <strong>Session only</strong>
            <span>No credential is written to SQLite, browser storage, logs, or plugins.</span>
          </div>

          {#if errorMessage}<p class="error" role="alert">{errorMessage}</p>{/if}

          <div class="dialog-actions">
            <button class="secondary" type="button" disabled={submitting} onclick={close}>Cancel</button>
            <button class="primary" type="submit" disabled={submitting}>
              {submitting ? "Contacting OnAir…" : "Connect to OnAir"}
            </button>
          </div>
        </form>
      {/if}
    </div>
  </div>
{/if}

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(1, 7, 6, 0.78);
    backdrop-filter: blur(7px);
  }
  .connection-dialog {
    width: min(480px, 100%);
    border: 1px solid var(--line);
    padding: 24px;
    background: #0a1916;
    box-shadow: 0 28px 90px rgba(0, 0, 0, 0.48);
  }
  header,
  .dialog-actions,
  .connected-company {
    display: flex;
    align-items: center;
  }
  header {
    justify-content: space-between;
    margin-bottom: 18px;
  }
  h2 {
    margin-top: 5px;
    font-family: Georgia, serif;
    font-size: 25px;
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
  .explanation {
    color: var(--muted);
    font-size: 13px;
    line-height: 1.55;
  }
  form {
    display: grid;
    margin-top: 20px;
  }
  label {
    margin: 14px 0 7px;
    color: #cfddd8;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  input {
    width: 100%;
    min-width: 0;
    border: 1px solid var(--line);
    border-radius: 3px;
    padding: 11px 12px;
    outline: none;
    color: #edf5f2;
    background: #07110f;
    font: inherit;
  }
  input:focus {
    border-color: rgba(115, 214, 173, 0.58);
    box-shadow: 0 0 0 2px rgba(115, 214, 173, 0.09);
  }
  input:disabled {
    opacity: 0.65;
  }
  .secret-field {
    display: grid;
    grid-template-columns: 1fr auto;
  }
  .secret-field input {
    border-radius: 3px 0 0 3px;
  }
  .secret-field button {
    border: 1px solid var(--line);
    border-left: 0;
    padding: 0 12px;
    color: var(--mint);
    background: #10231f;
    cursor: pointer;
  }
  .session-notice {
    display: grid;
    gap: 3px;
    margin-top: 18px;
    border-left: 2px solid var(--gold);
    padding: 10px 12px;
    background: rgba(213, 174, 95, 0.06);
    font-size: 11px;
  }
  .session-notice strong {
    color: var(--gold);
  }
  .session-notice span {
    color: var(--muted);
  }
  .credential-source-warning {
    display: grid;
    gap: 3px;
    margin-top: 14px;
    border: 1px solid rgba(207, 126, 101, 0.32);
    padding: 10px 12px;
    background: rgba(207, 126, 101, 0.08);
    font-size: 11px;
    line-height: 1.4;
  }
  .credential-source-warning strong {
    color: #efb19e;
  }
  .credential-source-warning span {
    color: var(--muted);
  }
  .error {
    margin-top: 14px;
    border: 1px solid rgba(207, 126, 101, 0.32);
    padding: 10px 12px;
    color: #efb19e;
    background: rgba(207, 126, 101, 0.08);
    font-size: 12px;
    line-height: 1.4;
  }
  .dialog-actions {
    justify-content: flex-end;
    gap: 9px;
    margin-top: 22px;
  }
  .dialog-actions button {
    border: 1px solid var(--line);
    border-radius: 3px;
    padding: 9px 14px;
    cursor: pointer;
  }
  .dialog-actions button:disabled {
    cursor: wait;
    opacity: 0.65;
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
  .danger {
    border-color: rgba(207, 126, 101, 0.36) !important;
    color: #efb19e;
    background: rgba(207, 126, 101, 0.08);
  }
  .connected-company {
    gap: 12px;
    margin: 8px 0 16px;
    border: 1px solid rgba(115, 214, 173, 0.24);
    padding: 14px;
    background: rgba(115, 214, 173, 0.05);
  }
  .connected-company div {
    display: grid;
    gap: 3px;
  }
  .connected-company span,
  .connected-company small {
    color: var(--muted);
    font-size: 10px;
  }
  .connected-company strong {
    font-family: Georgia, serif;
    font-size: 19px;
    font-weight: 500;
  }
  .status-light {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--mint);
    box-shadow: 0 0 12px rgba(115, 214, 173, 0.7);
  }
</style>
