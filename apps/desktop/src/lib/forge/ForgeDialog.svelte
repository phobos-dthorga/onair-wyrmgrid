<script lang="ts">
  import type { PluginHostView, PluginPermission, PluginView } from "./types";

  let {
    open,
    status,
    busy,
    errorMessage,
    onapprove,
    onrevoke,
    onstart,
    onstop,
    onclose,
  }: {
    open: boolean;
    status: PluginHostView;
    busy: boolean;
    errorMessage: string;
    onapprove: (pluginId: string) => void;
    onrevoke: (pluginId: string) => void;
    onstart: (pluginId: string) => void;
    onstop: (pluginId: string) => void;
    onclose: () => void;
  } = $props();

  const permissionLabels: Record<PluginPermission, string> = {
    on_air_company_read: "Read company identity",
    on_air_fleet_read: "Read sanitized fleet facts",
    on_air_jobs_read: "Read sanitized job facts",
    on_air_finance_read: "Read sanitized finance facts",
    map_layers_publish: "Publish Atlas map layers",
    charts_publish: "Publish host-rendered charts",
    notifications_create: "Create notifications",
    plugin_storage: "Use private plugin storage",
    simulator_telemetry_read: "Read simulator telemetry",
    external_network: "Request external network access",
  };

  function allRequestedGranted(plugin: PluginView): boolean {
    return plugin.requested_permissions.every((permission) =>
      plugin.granted_permissions.includes(permission),
    );
  }

  function stateLabel(plugin: PluginView): string {
    if (plugin.state === "running") return "Running";
    if (plugin.state === "starting") return "Starting";
    if (plugin.state === "stopping") return "Stopping";
    if (plugin.state === "failed") return "Needs attention";
    return allRequestedGranted(plugin) ? "Ready" : "Permission review";
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (open && event.key === "Escape" && !busy) onclose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="dialog-backdrop">
    <div
      class="forge-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="forge-title"
    >
      <header>
        <div>
          <span class="eyebrow">WyrmGrid Forge</span>
          <h2 id="forge-title">Plugin workshop</h2>
          <p>Review capabilities, then supervise separate plugin processes.</p>
        </div>
        <button
          class="close-button"
          type="button"
          aria-label="Close Forge"
          disabled={busy}
          onclick={onclose}>×</button
        >
      </header>

      <div class="safety-note">
        <strong>Developer preview</strong>
        <span>
          Capabilities control WyrmGrid data and actions. This early runtime is
          not an operating-system sandbox, so only run plugin code you trust.
        </span>
      </div>

      {#if status.notice}<p class="notice">{status.notice}</p>{/if}
      {#if !status.available}
        <div class="empty-state">
          <strong>Forge is unavailable</strong>
          <span
            >{status.notice ??
              "The local plugin workshop could not open."}</span
          >
        </div>
      {:else if status.plugins.length === 0}
        <div class="empty-state">
          <strong>No plugins installed</strong>
          <span>Installed plugin folders will appear here for review.</span>
        </div>
      {:else}
        <div class="plugin-list" aria-label="Installed plugins">
          {#each status.plugins as plugin}
            <article class:running={plugin.state === "running"}>
              <div class="plugin-heading">
                <div>
                  <span class="plugin-runtime"
                    >{plugin.runtime ?? "Metadata only"}</span
                  >
                  <h3>{plugin.name}</h3>
                  <small>{plugin.author} · v{plugin.version}</small>
                </div>
                <span
                  class:failed={plugin.state === "failed"}
                  class="state-badge"
                >
                  {stateLabel(plugin)}
                </span>
              </div>

              <div class="capability-panel">
                <span class="panel-label">Requested capabilities</span>
                <ul>
                  {#each plugin.requested_permissions as permission}
                    <li
                      class:granted={plugin.granted_permissions.includes(
                        permission,
                      )}
                    >
                      <i aria-hidden="true"></i>
                      <span>{permissionLabels[permission]}</span>
                      <small>
                        {plugin.granted_permissions.includes(permission)
                          ? "Approved"
                          : "Denied"}
                      </small>
                    </li>
                  {/each}
                </ul>
              </div>

              {#if plugin.last_error}<p class="plugin-error" role="alert">
                  {plugin.last_error}
                </p>{/if}

              <footer>
                <span>
                  {plugin.state === "running"
                    ? `${plugin.published_layer_count} Atlas ${plugin.published_layer_count === 1 ? "layer" : "layers"} published`
                    : "Python 3 runtime · framed protocol v1"}
                </span>
                <div>
                  {#if allRequestedGranted(plugin)}
                    <button
                      class="secondary"
                      type="button"
                      disabled={busy ||
                        plugin.state === "starting" ||
                        plugin.state === "stopping"}
                      onclick={() => onrevoke(plugin.id)}>Revoke access</button
                    >
                  {:else}
                    <button
                      class="secondary"
                      type="button"
                      disabled={busy || plugin.state !== "stopped"}
                      onclick={() => onapprove(plugin.id)}
                      >Approve access</button
                    >
                  {/if}
                  {#if plugin.state === "running"}
                    <button
                      class="primary stop"
                      type="button"
                      disabled={busy}
                      onclick={() => onstop(plugin.id)}>Stop plugin</button
                    >
                  {:else}
                    <button
                      class="primary"
                      type="button"
                      disabled={busy ||
                        !allRequestedGranted(plugin) ||
                        plugin.state !== "stopped"}
                      onclick={() => onstart(plugin.id)}
                    >
                      {plugin.state === "starting"
                        ? "Starting…"
                        : "Start plugin"}
                    </button>
                  {/if}
                </div>
              </footer>
            </article>
          {/each}
        </div>
      {/if}

      {#if errorMessage}<p class="error-message" role="alert">
          {errorMessage}
        </p>{/if}
    </div>
  </div>
{/if}

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    z-index: 42;
    display: grid;
    place-items: center;
    padding: 24px;
    background: var(--color-overlay);
    backdrop-filter: blur(10px);
  }
  .forge-dialog {
    width: min(820px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    overflow: auto;
    border: 1px solid var(--color-line-soft);
    border-radius: 8px;
    color: var(--color-text);
    background: var(--color-surface-elevated);
    box-shadow: 0 28px 90px var(--color-shadow);
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 20px;
    padding: 22px 24px;
    border-bottom: 1px solid var(--color-line-faint);
  }
  h2,
  h3,
  p {
    margin: 0;
  }
  h2 {
    margin-top: 5px;
    font-family: Georgia, serif;
    font-size: 28px;
    font-weight: 500;
  }
  header p {
    margin-top: 7px;
    color: var(--color-text-muted);
    font-size: 11px;
  }
  .close-button {
    border: 0;
    color: var(--color-text-muted);
    background: transparent;
    font-size: 27px;
    cursor: pointer;
  }
  .safety-note {
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
  .safety-note strong {
    color: var(--color-highlight);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 9px;
  }
  .notice,
  .error-message {
    margin: 14px 24px 0;
    padding: 10px 12px;
    border: 1px solid var(--color-line-faint);
    color: var(--color-text-muted);
    background: var(--color-surface-soft);
    font-size: 11px;
  }
  .error-message,
  .plugin-error {
    border-color: var(--color-danger-border);
    color: var(--color-danger);
    background: var(--color-danger-soft);
  }
  .plugin-list {
    display: grid;
    gap: 12px;
    padding: 18px 24px 24px;
  }
  article {
    border: 1px solid var(--color-line-faint);
    border-radius: 6px;
    background: var(--color-surface);
  }
  article.running {
    border-color: var(--color-accent-border);
    box-shadow: inset 3px 0 0 var(--color-accent);
  }
  .plugin-heading,
  article footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    padding: 16px 18px;
  }
  .plugin-runtime,
  .panel-label {
    color: var(--color-highlight);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  h3 {
    margin-top: 4px;
    font-family: Georgia, serif;
    font-size: 21px;
    font-weight: 500;
  }
  .plugin-heading small,
  article footer > span {
    color: var(--color-text-muted);
    font-size: 10px;
  }
  .state-badge {
    border: 1px solid var(--color-accent-border);
    border-radius: 99px;
    padding: 5px 9px;
    color: var(--color-accent);
    background: var(--color-accent-soft);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .state-badge.failed {
    border-color: var(--color-danger-border);
    color: var(--color-danger);
    background: var(--color-danger-soft);
  }
  .capability-panel {
    margin: 0 18px;
    padding: 13px;
    border: 1px solid var(--color-line-faint);
    background: var(--color-surface-soft);
  }
  ul {
    display: grid;
    gap: 7px;
    margin: 11px 0 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: grid;
    grid-template-columns: 8px 1fr auto;
    align-items: center;
    gap: 9px;
    color: var(--color-text-muted);
    font-size: 11px;
  }
  li i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--color-line);
  }
  li.granted i {
    background: var(--color-accent);
    box-shadow: 0 0 8px var(--color-accent-glow);
  }
  li small {
    color: var(--color-text-muted);
    font-size: 9px;
    text-transform: uppercase;
  }
  li.granted small {
    color: var(--color-accent);
  }
  .plugin-error {
    margin: 12px 18px 0;
    padding: 9px 11px;
    border: 1px solid var(--color-danger-border);
    font-size: 10px;
  }
  article footer {
    margin-top: 15px;
    border-top: 1px solid var(--color-line-faint);
    background: var(--color-surface-soft-translucent);
  }
  article footer div {
    display: flex;
    gap: 8px;
  }
  article footer button {
    min-width: 112px;
    padding: 8px 11px;
    border-radius: 4px;
    font: inherit;
    font-size: 10px;
    font-weight: 700;
    cursor: pointer;
  }
  .secondary {
    border: 1px solid var(--color-line-soft);
    color: var(--color-text-muted);
    background: transparent;
  }
  .primary {
    border: 1px solid var(--color-accent);
    color: var(--color-canvas);
    background: var(--color-accent);
  }
  .primary.stop {
    border-color: var(--color-danger-border);
    color: var(--color-danger);
    background: var(--color-danger-soft);
  }
  button:disabled {
    opacity: 0.48;
    cursor: not-allowed;
  }
  .empty-state {
    display: grid;
    gap: 6px;
    margin: 18px 24px 24px;
    padding: 32px;
    border: 1px dashed var(--color-line-soft);
    color: var(--color-text-muted);
    text-align: center;
    font-size: 11px;
  }
  .empty-state strong {
    color: var(--color-text);
    font-family: Georgia, serif;
    font-size: 18px;
    font-weight: 500;
  }
  @media (max-width: 700px) {
    .dialog-backdrop {
      padding: 12px;
    }
    .forge-dialog {
      width: calc(100vw - 24px);
      max-height: calc(100vh - 24px);
    }
    .safety-note,
    article footer {
      grid-template-columns: 1fr;
    }
    article footer {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
