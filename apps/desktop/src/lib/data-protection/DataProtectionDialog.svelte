<script lang="ts">
  import { translation } from "$lib/i18n/runtime";
  import type { DataProtectionStatus } from "./types";
  import "./data-protection.css";

  let {
    open,
    desktopRuntime,
    loaded,
    status,
    busy = false,
    errorMessage = "",
    successMessage = "",
    onrefresh,
    onchoosebackup,
    onchooserestore,
    onbackup,
    onrestore,
    onreset,
    onlicenses,
    onclose,
  }: {
    open: boolean;
    desktopRuntime: boolean;
    loaded: boolean;
    status: DataProtectionStatus;
    busy?: boolean;
    errorMessage?: string;
    successMessage?: string;
    onrefresh: () => void;
    onchoosebackup: () => Promise<string | null>;
    onchooserestore: () => Promise<string | null>;
    onbackup: (
      destination: string,
      password: string,
      passwordConfirmation: string,
    ) => void;
    onrestore: (
      source: string,
      password: string,
      replacementConfirmed: boolean,
    ) => void;
    onreset: (confirmation: string) => void;
    onlicenses: () => void;
    onclose: () => void;
  } = $props();

  let backupDestination = $state("");
  let backupPassword = $state("");
  let backupConfirmation = $state("");
  let restoreSource = $state("");
  let restorePassword = $state("");
  let replacementConfirmed = $state(false);
  let choosingFile = $state(false);
  let resetAcknowledged = $state(false);
  let resetConfirmation = $state("");

  $effect(() => {
    if (open) resetSecrets();
  });

  $effect(() => {
    if (successMessage) {
      backupPassword = "";
      backupConfirmation = "";
      restorePassword = "";
      replacementConfirmed = false;
    }
  });

  function resetSecrets(): void {
    backupDestination = "";
    backupPassword = "";
    backupConfirmation = "";
    restoreSource = "";
    restorePassword = "";
    replacementConfirmed = false;
    resetAcknowledged = false;
    resetConfirmation = "";
  }

  function selectedName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  async function chooseBackup(): Promise<void> {
    choosingFile = true;
    try {
      backupDestination = (await onchoosebackup()) ?? backupDestination;
    } finally {
      choosingFile = false;
    }
  }

  async function chooseRestore(): Promise<void> {
    choosingFile = true;
    try {
      const source = await onchooserestore();
      if (source) {
        restoreSource = source;
        restorePassword = "";
        replacementConfirmed = false;
      }
    } finally {
      choosingFile = false;
    }
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape" && !busy && !choosingFile) onclose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="data-protection-backdrop">
    <div
      class="data-protection-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="data-protection-title"
    >
      <header>
        <div>
          <span class="eyebrow">{$translation("data-protection-eyebrow")}</span>
          <h2 id="data-protection-title">
            {$translation("data-protection-title")}
          </h2>
          <p>{$translation("data-protection-introduction")}</p>
        </div>
        <button
          class="close-button"
          type="button"
          aria-label={$translation("data-protection-close")}
          disabled={busy || choosingFile}
          onclick={onclose}>×</button
        >
      </header>

      {#if !loaded}
        <section class="data-protection-loading" aria-live="polite">
          <strong
            >{$translation(
              busy ? "data-protection-loading" : "data-protection-unavailable",
            )}</strong
          >
          {#if !busy && desktopRuntime}
            <button type="button" onclick={onrefresh}
              >{$translation("data-protection-refresh")}</button
            >
          {/if}
        </section>
      {:else}
        <section
          class="protection-summary"
          aria-label={$translation("data-protection-summary")}
        >
          <article class="responsive-surface">
            <span>{$translation("data-protection-database")}</span>
            <strong
              >{$translation(
                status.database_encrypted
                  ? "data-protection-encrypted"
                  : "data-protection-unavailable",
              )}</strong
            >
            <small>{$translation("data-protection-database-detail")}</small>
          </article>
          <article class="responsive-surface">
            <span>{$translation("data-protection-device-key")}</span>
            <strong
              >{$translation(
                status.device_key_protected
                  ? "data-protection-os-protected"
                  : "data-protection-unavailable",
              )}</strong
            >
            <small>{$translation("data-protection-device-key-detail")}</small>
          </article>
          <article class="responsive-surface">
            <span>{$translation("data-protection-format")}</span>
            <strong>v{status.portable_backup_format_version}</strong>
            <small>{$translation("data-protection-format-detail")}</small>
          </article>
        </section>

        {#if status.pending_restore}
          <div class="pending-restore" role="status">
            <strong>{$translation("data-protection-pending-title")}</strong>
            <span>{$translation("data-protection-pending-detail")}</span>
          </div>
        {/if}

        <div class="protection-workflows">
          <section class="protection-workflow">
            <div>
              <span class="eyebrow"
                >{$translation("data-protection-backup-eyebrow")}</span
              >
              <h3>{$translation("data-protection-backup-title")}</h3>
              <p>{$translation("data-protection-backup-detail")}</p>
            </div>
            <button
              class="file-choice"
              type="button"
              disabled={!desktopRuntime || busy || choosingFile}
              onclick={() => void chooseBackup()}
            >
              {backupDestination
                ? selectedName(backupDestination)
                : $translation("data-protection-choose-destination")}
            </button>
            <label>
              <span>{$translation("data-protection-backup-password")}</span>
              <input
                type="password"
                autocomplete="new-password"
                minlength="12"
                maxlength="1024"
                disabled={busy}
                bind:value={backupPassword}
              />
            </label>
            <label>
              <span>{$translation("data-protection-backup-confirm")}</span>
              <input
                type="password"
                autocomplete="new-password"
                minlength="12"
                maxlength="1024"
                disabled={busy}
                bind:value={backupConfirmation}
              />
            </label>
            <p class="password-warning">
              {$translation("data-protection-password-warning")}
            </p>
            <button
              class="primary-action"
              type="button"
              disabled={!desktopRuntime ||
                busy ||
                !backupDestination ||
                backupPassword.length < 12 ||
                backupConfirmation.length < 12}
              onclick={() =>
                onbackup(backupDestination, backupPassword, backupConfirmation)}
            >
              {$translation("data-protection-create-backup")}
            </button>
          </section>

          <section class="protection-workflow restore-workflow">
            <div>
              <span class="eyebrow"
                >{$translation("data-protection-restore-eyebrow")}</span
              >
              <h3>{$translation("data-protection-restore-title")}</h3>
              <p>{$translation("data-protection-restore-detail")}</p>
            </div>
            <button
              class="file-choice"
              type="button"
              disabled={!desktopRuntime || busy || choosingFile}
              onclick={() => void chooseRestore()}
            >
              {restoreSource
                ? selectedName(restoreSource)
                : $translation("data-protection-choose-source")}
            </button>
            <label>
              <span>{$translation("data-protection-restore-password")}</span>
              <input
                type="password"
                autocomplete="current-password"
                minlength="12"
                maxlength="1024"
                disabled={busy}
                bind:value={restorePassword}
              />
            </label>
            <label class="replacement-confirmation">
              <input
                type="checkbox"
                disabled={busy}
                bind:checked={replacementConfirmed}
              />
              <span>{$translation("data-protection-restore-confirm")}</span>
            </label>
            <button
              class="restore-action"
              type="button"
              disabled={!desktopRuntime ||
                busy ||
                !restoreSource ||
                restorePassword.length < 12 ||
                !replacementConfirmed}
              onclick={() =>
                onrestore(restoreSource, restorePassword, replacementConfirmed)}
            >
              {$translation("data-protection-prepare-restore")}
            </button>
          </section>
        </div>

        <section class="local-data-reset" aria-labelledby="local-data-reset-title">
          <div>
            <span class="eyebrow"
              >{$translation("data-protection-reset-eyebrow")}</span
            >
            <h3 id="local-data-reset-title">
              {$translation("data-protection-reset-title")}
            </h3>
            <p>{$translation("data-protection-reset-detail")}</p>
          </div>
          <div class="reset-losses">
            <strong>{$translation("data-protection-reset-erases-title")}</strong>
            <ul>
              <li>{$translation("data-protection-reset-erases-history")}</li>
              <li>{$translation("data-protection-reset-erases-recordings")}</li>
              <li>{$translation("data-protection-reset-erases-preferences")}</li>
              <li>{$translation("data-protection-reset-erases-access")}</li>
            </ul>
            <strong>{$translation("data-protection-reset-keeps-title")}</strong>
            <p>{$translation("data-protection-reset-keeps-detail")}</p>
          </div>
          <label class="replacement-confirmation reset-acknowledgement">
            <input
              type="checkbox"
              disabled={busy}
              bind:checked={resetAcknowledged}
            />
            <span>{$translation("data-protection-reset-acknowledge")}</span>
          </label>
          <label>
            <span
              >{$translation("data-protection-reset-type", {
                phrase: status.local_data_reset_confirmation,
              })}</span
            >
            <input
              type="text"
              autocomplete="off"
              spellcheck="false"
              disabled={busy || !resetAcknowledged}
              bind:value={resetConfirmation}
            />
          </label>
          <button
            class="reset-action"
            type="button"
            disabled={!desktopRuntime ||
              busy ||
              !resetAcknowledged ||
              resetConfirmation !== status.local_data_reset_confirmation}
            onclick={() => onreset(resetConfirmation)}
          >
            {$translation("data-protection-reset-action")}
          </button>
        </section>
      {/if}

      {#if successMessage}
        <p class="success-message" role="status">{successMessage}</p>
      {/if}
      {#if errorMessage}
        <p class="error-message" role="alert">{errorMessage}</p>
      {/if}

      <footer>
        <button type="button" disabled={busy} onclick={onlicenses}
          >{$translation("data-protection-licenses")}</button
        >
        <button type="button" disabled={busy} onclick={onclose}
          >{$translation("data-protection-done")}</button
        >
      </footer>
    </div>
  </div>
{/if}
