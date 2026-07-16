<script lang="ts">
  import { translation } from "$lib/i18n/runtime";
  import type { SecurityCentreStatus, SecurityGrantView } from "./types";

  let {
    open,
    loaded,
    status,
    busy = false,
    errorMessage = "",
    onrefresh,
    onrevoke,
    onprivacy,
    onclose,
  }: {
    open: boolean;
    loaded: boolean;
    status: SecurityCentreStatus;
    busy?: boolean;
    errorMessage?: string;
    onrefresh: () => void;
    onrevoke: (subjectId: string) => void;
    onprivacy: () => void;
    onclose: () => void;
  } = $props();

  const capabilityKeys: Record<string, string> = {
    on_air_company_read: "security-capability-company-read",
    on_air_fleet_read: "security-capability-fleet-read",
    on_air_jobs_read: "security-capability-jobs-read",
    on_air_finance_read: "security-capability-finance-read",
    map_layers_publish: "security-capability-map-publish",
    charts_publish: "security-capability-charts-publish",
    notifications_create: "security-capability-notifications-create",
    plugin_storage: "security-capability-plugin-storage",
    simulator_telemetry_read: "security-capability-simulator-read",
    external_network: "security-capability-external-network",
  };

  function capabilityLabel(capability: string): string {
    const key = capabilityKeys[capability];
    return key ? $translation(key) : capability;
  }

  function subjectLabel(grant: SecurityGrantView): string {
    return grant.subject_kind === "plugin"
      ? $translation("security-subject-plugin")
      : grant.subject_kind;
  }

  function localTime(value: string): string {
    const normalized = value.includes("T") ? value : `${value.replace(" ", "T")}Z`;
    const parsed = new Date(normalized);
    return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString();
  }

  function lifetimeLabel(lifetime: "once" | "session" | "standing"): string {
    return $translation(`security-lifetime-${lifetime}`);
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape" && !busy) onclose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="security-backdrop">
    <div
      class="security-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="security-title"
    >
      <header>
        <div>
          <span class="eyebrow">{$translation("security-eyebrow")}</span>
          <h2 id="security-title">{$translation("security-title")}</h2>
          <p>{$translation("security-introduction")}</p>
        </div>
        <button
          class="close-button"
          type="button"
          aria-label={$translation("security-close")}
          disabled={busy}
          onclick={onclose}>×</button
        >
      </header>

      <div class="authority-note">
        <strong>{$translation("security-boundary-title")}</strong>
        <span>{$translation("security-boundary-detail")}</span>
      </div>

      {#if !loaded}
        <section class="loading-state" aria-live="polite">
          <strong
            >{$translation(
              busy ? "security-loading" : "security-status-unavailable",
            )}</strong
          >
          {#if !busy}
            <button type="button" onclick={onrefresh}
              >{$translation("security-refresh")}</button
            >
          {/if}
        </section>
      {:else}
        <section
          class="summary-grid"
          aria-label={$translation("security-summary")}
        >
        <article>
          <span>{$translation("security-active-actors")}</span>
          <strong>{status.active_grants.length}</strong>
          <small>{$translation("security-active-actors-detail")}</small>
        </article>
        <article>
          <span>{$translation("security-legal-status")}</span>
          <strong
            >{status.legal.acknowledged
              ? $translation("security-current")
              : $translation("security-review-required")}</strong
          >
          <small>{$translation("security-legal-detail")}</small>
        </article>
        <article>
          <span>{$translation("security-diagnostics")}</span>
          <strong
            >{status.legal.telemetry_enabled
              ? $translation("security-enabled")
              : $translation("security-disabled")}</strong
          >
          <small>{$translation("security-diagnostics-detail")}</small>
        </article>
        </section>

        <section class="security-section">
        <div class="section-heading">
          <div>
            <span class="eyebrow">{$translation("security-access-eyebrow")}</span>
            <h3>{$translation("security-access-title")}</h3>
          </div>
          <button type="button" disabled={busy} onclick={onrefresh}
            >{$translation("security-refresh")}</button
          >
        </div>

        {#if status.active_grants.length === 0}
          <div class="empty-state">
            <strong>{$translation("security-no-active-access")}</strong>
            <span>{$translation("security-no-active-access-detail")}</span>
          </div>
        {:else}
          <div class="grant-list">
            {#each status.active_grants as grant}
              <article class="grant-card">
                <div class="grant-heading">
                  <div>
                    <span class="subject-kind">{subjectLabel(grant)}</span>
                    <h4>{grant.subject_id}</h4>
                    <small
                      >{$translation("security-granted-at", {
                        time: localTime(grant.granted_at),
                      })} · {lifetimeLabel(grant.lifetime)}</small
                    >
                  </div>
                  <button
                    class="revoke-button"
                    type="button"
                    disabled={busy || grant.subject_kind !== "plugin"}
                    onclick={() => onrevoke(grant.subject_id)}
                    >{$translation("security-revoke")}</button
                  >
                </div>
                <ul class="capability-list">
                  {#each grant.capabilities as capability}
                    <li>{capabilityLabel(capability)}</li>
                  {/each}
                </ul>
                <details>
                  <summary>{$translation("security-scope-revision")}</summary>
                  <code>{grant.scope_revision}</code>
                </details>
              </article>
            {/each}
          </div>
        {/if}
        </section>

        <section class="security-section">
        <div class="section-heading">
          <div>
            <span class="eyebrow">{$translation("security-history-eyebrow")}</span>
            <h3>{$translation("security-history-title")}</h3>
          </div>
          <small
            >{$translation("security-history-retention", {
              count: status.decision_retention_limit,
            })}</small
          >
        </div>
        {#if status.recent_decisions.length === 0}
          <div class="empty-state compact">
            <span>{$translation("security-no-history")}</span>
          </div>
        {:else}
          <ol class="decision-list">
            {#each status.recent_decisions as decision}
              <li>
                <i class:revoked={decision.decision === "revoke"}></i>
                <div>
                  <strong>{decision.subject_id}</strong>
                  <span>
                    {decision.decision === "grant"
                      ? $translation("security-decision-granted", {
                          count: decision.capability_count,
                        }) +
                        (decision.lifetime
                          ? ` · ${lifetimeLabel(decision.lifetime)}`
                          : "")
                      : $translation("security-decision-revoked")}
                  </span>
                </div>
                <time>{localTime(decision.decided_at)}</time>
              </li>
            {/each}
          </ol>
        {/if}
        </section>
      {/if}

      {#if errorMessage}<p class="error-message" role="alert">
          {errorMessage}
        </p>{/if}

      <footer>
        <button type="button" disabled={busy} onclick={onprivacy}
          >{$translation("security-review-privacy")}</button
        >
        <button type="button" disabled={busy} onclick={onclose}
          >{$translation("security-done")}</button
        >
      </footer>
    </div>
  </div>
{/if}

<style>
  .security-backdrop {
    position: fixed;
    inset: 0;
    z-index: 47;
    display: grid;
    place-items: center;
    padding: 24px;
    background: var(--color-overlay);
    backdrop-filter: blur(10px);
  }
  .security-dialog {
    width: min(900px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    overflow: auto;
    border: 1px solid var(--color-line-soft);
    border-radius: 8px;
    color: var(--color-text);
    background: var(--color-surface-elevated);
    box-shadow: 0 28px 90px var(--color-shadow);
  }
  header,
  footer,
  .section-heading,
  .grant-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
  }
  header,
  footer {
    padding: 21px 24px;
  }
  header {
    border-bottom: 1px solid var(--color-line-faint);
  }
  header p {
    margin: 7px 0 0;
    color: var(--color-text-muted);
    line-height: 1.5;
  }
  h2,
  h3,
  h4,
  p {
    margin: 0;
  }
  h2 {
    margin-top: 5px;
    font-family: Georgia, serif;
    font-size: 28px;
    font-weight: 500;
  }
  h3 {
    margin-top: 4px;
    font-family: Georgia, serif;
    font-size: 21px;
    font-weight: 500;
  }
  h4 {
    margin-top: 4px;
    font-size: 14px;
  }
  .close-button {
    border: 0;
    color: var(--color-text-muted);
    background: transparent;
    font-size: 27px;
    cursor: pointer;
  }
  .authority-note {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 12px;
    margin: 18px 24px 0;
    padding: 12px 14px;
    border: 1px solid var(--color-highlight-border);
    color: var(--color-text-muted);
    background: var(--color-highlight-soft);
    font-size: 11px;
    line-height: 1.5;
  }
  .authority-note strong,
  .subject-kind {
    color: var(--color-highlight);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    font-size: 9px;
  }
  .summary-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
    padding: 18px 24px;
  }
  .loading-state {
    display: grid;
    place-items: center;
    gap: 12px;
    min-height: 220px;
    margin: 18px 24px;
    border: 1px dashed var(--color-line-soft);
    color: var(--color-text-muted);
  }
  .loading-state button {
    border: 1px solid var(--color-line-soft);
    border-radius: 4px;
    padding: 8px 11px;
    color: var(--color-text-muted);
    background: transparent;
    font: inherit;
    cursor: pointer;
  }
  .summary-grid article {
    display: grid;
    gap: 6px;
    padding: 14px;
    border: 1px solid var(--color-line-faint);
    background: var(--color-surface);
  }
  .summary-grid span,
  .summary-grid small,
  .section-heading small,
  .grant-heading small {
    color: var(--color-text-muted);
    font-size: 10px;
  }
  .summary-grid strong {
    color: var(--color-accent);
    font-family: Georgia, serif;
    font-size: 22px;
    font-weight: 500;
  }
  .security-section {
    padding: 20px 24px;
    border-top: 1px solid var(--color-line-faint);
  }
  .section-heading > button,
  footer button,
  .revoke-button {
    border: 1px solid var(--color-line-soft);
    border-radius: 4px;
    padding: 8px 11px;
    color: var(--color-text-muted);
    background: transparent;
    font: inherit;
    cursor: pointer;
  }
  .grant-list {
    display: grid;
    gap: 10px;
    margin-top: 14px;
  }
  .grant-card {
    border: 1px solid var(--color-accent-border);
    border-radius: 5px;
    padding: 15px;
    background: var(--color-surface);
    box-shadow: inset 3px 0 0 var(--color-accent);
  }
  .revoke-button {
    border-color: var(--color-danger-border);
    color: var(--color-danger);
    background: var(--color-danger-soft);
  }
  .capability-list {
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
    margin: 14px 0;
    padding: 0;
    list-style: none;
  }
  .capability-list li {
    border: 1px solid var(--color-line-faint);
    border-radius: 99px;
    padding: 5px 9px;
    color: var(--color-text-muted);
    background: var(--color-surface-soft);
    font-size: 10px;
  }
  details {
    color: var(--color-text-muted);
    font-size: 10px;
  }
  details code {
    display: block;
    overflow-wrap: anywhere;
    margin-top: 7px;
    padding: 8px;
    background: var(--color-surface-soft);
  }
  .decision-list {
    display: grid;
    gap: 2px;
    margin: 14px 0 0;
    padding: 0;
    list-style: none;
  }
  .decision-list li {
    display: grid;
    grid-template-columns: 8px minmax(0, 1fr) auto;
    align-items: center;
    gap: 11px;
    padding: 10px 12px;
    background: var(--color-surface);
    font-size: 10px;
  }
  .decision-list i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--color-accent);
  }
  .decision-list i.revoked {
    background: var(--color-danger);
  }
  .decision-list div {
    display: grid;
    gap: 3px;
  }
  .decision-list span,
  .decision-list time {
    color: var(--color-text-muted);
  }
  .empty-state {
    display: grid;
    gap: 6px;
    margin-top: 14px;
    padding: 24px;
    border: 1px dashed var(--color-line-soft);
    color: var(--color-text-muted);
    text-align: center;
    font-size: 11px;
  }
  .empty-state strong {
    color: var(--color-text);
    font-family: Georgia, serif;
    font-size: 17px;
    font-weight: 500;
  }
  .empty-state.compact {
    padding: 15px;
  }
  .error-message {
    margin: 0 24px 18px;
    padding: 10px 12px;
    border: 1px solid var(--color-danger-border);
    color: var(--color-danger);
    background: var(--color-danger-soft);
    font-size: 11px;
  }
  footer {
    justify-content: flex-end;
    border-top: 1px solid var(--color-line-faint);
  }
  footer button:last-child {
    border-color: var(--color-accent);
    color: var(--color-canvas);
    background: var(--color-accent);
  }
  button:disabled {
    cursor: not-allowed;
    opacity: 0.48;
  }
  @media (max-width: 700px) {
    .security-backdrop {
      padding: 12px;
    }
    .security-dialog {
      width: calc(100vw - 24px);
      max-height: calc(100vh - 24px);
    }
    .summary-grid {
      grid-template-columns: 1fr;
    }
    .authority-note,
    .grant-heading,
    .section-heading,
    .decision-list li {
      grid-template-columns: 1fr;
    }
    .grant-heading,
    .section-heading {
      align-items: stretch;
      flex-direction: column;
    }
    .decision-list time {
      padding-left: 19px;
    }
  }
</style>
