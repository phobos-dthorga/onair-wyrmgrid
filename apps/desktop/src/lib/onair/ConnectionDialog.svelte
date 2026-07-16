<script lang="ts">
  import { invokeDesktop, operationErrorMessage } from "$lib/desktop/client";
  import {
    connectOnAir,
    connectRememberedOnAir,
    forgetOnAirCredentials,
    loadOnAirCredentialProfile,
  } from "./client";
  import {
    emptyCredentialProfile,
    type OnAirConnectionStatus,
    type OnAirCredentialProfileStatus,
  } from "./types";

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
  let remember = $state(false);
  let connectOnStart = $state(false);
  let profile = $state<OnAirCredentialProfileStatus>(emptyCredentialProfile);
  let profileLoaded = $state(false);
  let submitting = $state(false);
  let errorMessage = $state("");

  $effect(() => {
    if (open) void refreshProfile();
  });

  function safeError(error: unknown): string {
    return operationErrorMessage(
      error,
      "WyrmGrid could not complete the connection request.",
    );
  }

  async function connect(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    submitting = true;
    errorMessage = "";

    try {
      const result = await connectOnAir(
        companyId.trim(),
        apiKey,
        remember,
        remember && connectOnStart,
      );
      profile = result.profile;
      apiKey = "";
      companyId = "";
      revealApiKey = false;
      onstatuschange(result.connection);
    } catch (error) {
      errorMessage = safeError(error);
    } finally {
      submitting = false;
    }
  }

  async function refreshProfile(): Promise<void> {
    profileLoaded = false;
    try {
      profile = await loadOnAirCredentialProfile();
      remember = profile.remembered;
      connectOnStart = profile.connect_on_start;
      if (!companyId && profile.company_id) companyId = profile.company_id;
    } catch (error) {
      errorMessage = safeError(error);
    } finally {
      profileLoaded = true;
    }
  }

  async function connectRemembered(): Promise<void> {
    submitting = true;
    errorMessage = "";
    try {
      const result = await connectRememberedOnAir();
      profile = result.profile;
      onstatuschange(result.connection);
    } catch (error) {
      errorMessage = safeError(error);
      await refreshProfile();
    } finally {
      submitting = false;
    }
  }

  async function forgetCredentials(): Promise<void> {
    submitting = true;
    errorMessage = "";
    try {
      profile = await forgetOnAirCredentials();
      remember = false;
      connectOnStart = false;
      companyId = "";
      apiKey = "";
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
      const disconnected =
        await invokeDesktop<OnAirConnectionStatus>("disconnect_onair");
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
    <div
      class="connection-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="connection-title"
    >
      <header>
        <div>
          <span class="eyebrow">Secure data link</span>
          <h2 id="connection-title">OnAir connection</h2>
        </div>
        <button
          class="close-button"
          type="button"
          aria-label="Close connection dialog"
          onclick={close}>×</button
        >
      </header>

      {#if status.connected && status.company}
        <div class="connected-company">
          <span class="status-light" aria-hidden="true"></span>
          <div>
            <span>Connected for this session</span>
            <strong>{status.company.name}</strong>
            {#if status.company.airline_code}<small
                >{status.company.airline_code}</small
              >{/if}
          </div>
        </div>

        <p class="explanation">
          {profile.remembered
            ? "The API key is remembered by Windows Credential Manager. WyrmGrid's encrypted database stores only the Company ID and your startup choice."
            : "This connection is session-only. The API key will be forgotten when WyrmGrid closes."}
        </p>

        {#if errorMessage}<p class="error" role="alert">{errorMessage}</p>{/if}

        <div class="dialog-actions">
          {#if profile.remembered}
            <button
              class="danger"
              type="button"
              disabled={submitting}
              onclick={forgetCredentials}>Forget saved details</button
            >
          {/if}
          <button class="secondary" type="button" onclick={close}>Done</button>
          <button
            class="danger"
            type="button"
            disabled={submitting}
            onclick={disconnect}
          >
            {submitting ? "Disconnecting…" : "Disconnect"}
          </button>
        </div>
      {:else}
        <p class="explanation">
          Copy the Company ID and API Key from OnAir Client → Options → Global
          Settings. WyrmGrid uses them only with OnAir's read-only public API.
        </p>

        <div class="credential-source-warning" role="note">
          <strong>For now, use OnAir Client—not OnAir Companion</strong>
          <span
            >Companion is still approaching feature parity; its current API
            details are not yet valid for this connection.</span
          >
        </div>

        {#if profileLoaded && profile.remembered}
          <div class="remembered-profile">
            <div>
              <span>Remembered OnAir connection</span>
              <strong>{profile.company_id}</strong>
              <small>
                {profile.connect_on_start
                  ? "Automatic connection is enabled."
                  : "Automatic connection is off."}
              </small>
            </div>
            <div class="remembered-actions">
              <button
                class="secondary"
                type="button"
                disabled={submitting || !profile.secret_available}
                onclick={connectRemembered}>Use saved details</button
              >
              <button
                class="danger"
                type="button"
                disabled={submitting || !profile.credential_store_available}
                onclick={forgetCredentials}>Forget</button
              >
            </div>
            {#if !profile.secret_available}
              <small class="profile-warning">
                {profile.credential_store_available
                  ? "Windows no longer has the saved API key. Enter it below to replace this connection."
                  : "Windows Credential Manager is unavailable. Session-only connections can still be attempted."}
              </small>
            {/if}
          </div>
        {/if}

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
              >{revealApiKey ? "Hide" : "Show"}</button
            >
          </div>

          <label class="remember-toggle">
            <input type="checkbox" bind:checked={remember} disabled={submitting} />
            <span>
              <strong>Remember this connection</strong>
              <small>
                Windows stores the API key; WyrmGrid stores only the Company ID
                in its encrypted database. Nothing is shared with plugins.
              </small>
            </span>
          </label>

          <label class="remember-toggle nested-toggle">
            <input
              type="checkbox"
              bind:checked={connectOnStart}
              disabled={submitting || !remember}
            />
            <span>
              <strong>Connect automatically when WyrmGrid starts</strong>
              <small>
                Off by default. When enabled, WyrmGrid contacts OnAir after the
                current privacy notice has been accepted.
              </small>
            </span>
          </label>

          <div class="session-notice">
            <strong>{remember ? "Protected local storage" : "Session only"}</strong>
            <span>
              {remember
                ? "The API key is never written to SQLite, browser storage, logs, backups, or plugins."
                : "No credential is written to SQLite, browser storage, logs, backups, or plugins."}
            </span>
          </div>

          {#if errorMessage}<p class="error" role="alert">
              {errorMessage}
            </p>{/if}

          <div class="dialog-actions">
            <button
              class="secondary"
              type="button"
              disabled={submitting}
              onclick={close}>Cancel</button
            >
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
    background: var(--color-overlay);
    backdrop-filter: blur(7px);
  }
  .connection-dialog {
    width: min(480px, 100%);
    max-height: calc(100vh - 48px);
    border: 1px solid var(--color-line-faint);
    padding: 24px;
    overflow-y: auto;
    background: var(--color-surface);
    box-shadow: 0 28px 90px var(--color-shadow);
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
    border: 1px solid var(--color-line-faint);
    color: var(--color-text-muted);
    background: transparent;
    font-size: 22px;
    cursor: pointer;
  }
  .explanation {
    color: var(--color-text-muted);
    font-size: 13px;
    line-height: 1.55;
  }
  form {
    display: grid;
    margin-top: 20px;
  }
  label {
    margin: 14px 0 7px;
    color: var(--color-text-muted);
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  input {
    width: 100%;
    min-width: 0;
    border: 1px solid var(--color-line-faint);
    border-radius: 3px;
    padding: 11px 12px;
    outline: none;
    color: var(--color-text);
    background: var(--color-canvas);
    font: inherit;
  }
  input:focus {
    border-color: var(--color-accent-border);
    box-shadow: 0 0 0 2px var(--color-accent-soft);
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
    border: 1px solid var(--color-line-faint);
    border-left: 0;
    padding: 0 12px;
    color: var(--color-accent);
    background: var(--color-surface-elevated);
    cursor: pointer;
  }
  .session-notice {
    display: grid;
    gap: 3px;
    margin-top: 18px;
    border-left: 2px solid var(--color-highlight);
    padding: 10px 12px;
    background: var(--color-highlight-soft);
    font-size: 11px;
  }
  .remember-toggle {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: start;
    gap: 9px;
    margin-top: 18px;
    text-transform: none;
    letter-spacing: normal;
  }
  .remember-toggle input {
    width: auto;
    margin-top: 3px;
  }
  .remember-toggle span {
    display: grid;
    gap: 3px;
  }
  .remember-toggle strong {
    color: var(--color-text);
    font-size: 12px;
  }
  .remember-toggle small {
    color: var(--color-text-muted);
    font-weight: 400;
    line-height: 1.45;
  }
  .nested-toggle {
    margin-top: 11px;
    margin-left: 22px;
  }
  .remembered-profile {
    display: grid;
    gap: 12px;
    margin-top: 16px;
    border: 1px solid var(--color-accent-border);
    padding: 13px;
    background: var(--color-accent-soft);
  }
  .remembered-profile > div:first-child {
    display: grid;
    gap: 4px;
  }
  .remembered-profile span,
  .remembered-profile small {
    color: var(--color-text-muted);
    font-size: 10px;
  }
  .remembered-profile strong {
    overflow-wrap: anywhere;
    font-size: 12px;
  }
  .remembered-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .remembered-actions button {
    border: 1px solid var(--color-line-faint);
    border-radius: 3px;
    padding: 8px 11px;
    cursor: pointer;
  }
  .profile-warning {
    color: var(--color-danger) !important;
    line-height: 1.45;
  }
  .session-notice strong {
    color: var(--color-highlight);
  }
  .session-notice span {
    color: var(--color-text-muted);
  }
  .credential-source-warning {
    display: grid;
    gap: 3px;
    margin-top: 14px;
    border: 1px solid var(--color-danger-border);
    padding: 10px 12px;
    background: var(--color-danger-soft);
    font-size: 11px;
    line-height: 1.4;
  }
  .credential-source-warning strong {
    color: var(--color-danger);
  }
  .credential-source-warning span {
    color: var(--color-text-muted);
  }
  .error {
    margin-top: 14px;
    border: 1px solid var(--color-danger-border);
    padding: 10px 12px;
    color: var(--color-danger);
    background: var(--color-danger-soft);
    font-size: 12px;
    line-height: 1.4;
  }
  .dialog-actions {
    justify-content: flex-end;
    gap: 9px;
    margin-top: 22px;
  }
  .dialog-actions button {
    border: 1px solid var(--color-line-faint);
    border-radius: 3px;
    padding: 9px 14px;
    cursor: pointer;
  }
  .dialog-actions button:disabled {
    cursor: wait;
    opacity: 0.65;
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
  .danger {
    border-color: var(--color-danger-border) !important;
    color: var(--color-danger);
    background: var(--color-danger-soft);
  }
  .connected-company {
    gap: 12px;
    margin: 8px 0 16px;
    border: 1px solid var(--color-accent-border);
    padding: 14px;
    background: var(--color-accent-soft);
  }
  .connected-company div {
    display: grid;
    gap: 3px;
  }
  .connected-company span,
  .connected-company small {
    color: var(--color-text-muted);
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
    background: var(--color-accent);
    box-shadow: 0 0 12px var(--color-accent-glow);
  }

  @media (max-height: 720px), (max-width: 640px) {
    .dialog-backdrop {
      padding: 12px;
    }

    .connection-dialog {
      max-height: calc(100vh - 24px);
      padding: 18px;
    }
  }
</style>
