<script lang="ts">
  import {
    capabilityTranslationKey,
    lifetimeTranslationKey,
  } from "$lib/authorization/presentation";
  import ExplorationSummary from "$lib/exploration/ExplorationSummary.svelte";
  import { translation } from "$lib/i18n/runtime";
  import {
    pluginSettingChoiceKey,
    pluginSettingPresentation,
  } from "./configuration";
  import type {
    AuthorizationGrantLifetime,
    ManagedPluginPackage,
    PluginHostView,
    PluginPackageInspection,
    PluginPermission,
    PluginView,
  } from "./types";
  import {
    activeForgeFilterCount,
    allRequestedCapabilitiesGranted,
    defaultForgeFilters,
    filterForgePlugins,
    forgeFilterOptions,
    type ForgeFilters,
  } from "./presentation";

  let {
    open,
    status,
    busy,
    errorMessage,
    managedPackages,
    pendingPackage,
    onchoosepackage,
    oncancelpackage,
    oninstallpackage,
    onpackageenable,
    onpackagerollback,
    onpackageremove,
    onapprove,
    onrevoke,
    onstart,
    onstop,
    onstartupchange,
    onconfigurationchange,
    onclose,
  }: {
    open: boolean;
    status: PluginHostView;
    busy: boolean;
    errorMessage: string;
    managedPackages: ManagedPluginPackage[];
    pendingPackage: PluginPackageInspection | null;
    onchoosepackage: () => void;
    oncancelpackage: () => void;
    oninstallpackage: () => void;
    onpackageenable: (pluginId: string, enabled: boolean) => void;
    onpackagerollback: (pluginId: string) => void;
    onpackageremove: (pluginId: string) => void;
    onapprove: (pluginId: string, lifetime: AuthorizationGrantLifetime) => void;
    onrevoke: (pluginId: string) => void;
    onstart: (pluginId: string) => void;
    onstop: (pluginId: string) => void;
    onstartupchange: (pluginId: string, enabled: boolean) => void;
    onconfigurationchange: (
      pluginId: string,
      settingKey: string,
      value: string,
    ) => void;
    onclose: () => void;
  } = $props();

  let approvalLifetime = $state<AuthorizationGrantLifetime>("session");
  let removalCandidate = $state<string | null>(null);
  let filters = $state<ForgeFilters>({ ...defaultForgeFilters });
  const visiblePlugins = $derived(filterForgePlugins(status.plugins, filters));
  const filterOptions = $derived(forgeFilterOptions(status.plugins));
  const activeFilterCount = $derived(activeForgeFilterCount(filters));

  function resetFilters(): void {
    filters = { ...defaultForgeFilters };
  }

  function permissionLabel(permission: PluginPermission): string {
    const key = capabilityTranslationKey(permission);
    return key ? $translation(key) : permission;
  }

  function lifetimeLabel(lifetime: AuthorizationGrantLifetime): string {
    return $translation(lifetimeTranslationKey(lifetime));
  }

  function allRequestedGranted(plugin: PluginView): boolean {
    return allRequestedCapabilitiesGranted(plugin);
  }

  function stateLabel(plugin: PluginView): string {
    if (plugin.state === "running") return "Running";
    if (plugin.state === "starting") return "Starting";
    if (plugin.state === "stopping") return "Stopping";
    if (plugin.state === "failed") return "Needs attention";
    return allRequestedGranted(plugin) ? "Ready" : "Permission review";
  }

  function packageProcessIsActive(pluginId: string): boolean {
    return status.plugins.some(
      (plugin) =>
        plugin.id === pluginId &&
        ["starting", "running", "stopping"].includes(plugin.state),
    );
  }

  function compactBytes(bytes: number): string {
    if (bytes >= 1024 * 1024)
      return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
    return `${Math.max(1, Math.ceil(bytes / 1024))} KiB`;
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

      <section
        class="package-workshop"
        aria-labelledby="package-workshop-title"
      >
        <div class="package-workshop-heading">
          <div>
            <span class="eyebrow">External packages</span>
            <h3 id="package-workshop-title">Install from a file</h3>
            <p>
              Review a <code>.wyrmplugin</code> package before WyrmGrid copies it
              into managed local storage. Installation never starts it.
            </p>
          </div>
          <button
            class="secondary package-choice"
            type="button"
            disabled={busy || pendingPackage !== null}
            onclick={onchoosepackage}>Choose package…</button
          >
        </div>

        {#if pendingPackage}
          <div class="package-review" role="group" aria-label="Package review">
            <div>
              <strong>{pendingPackage.name}</strong>
              <span>{pendingPackage.author} · v{pendingPackage.version}</span>
            </div>
            <dl>
              <div>
                <dt>Identity</dt>
                <dd>{pendingPackage.id}</dd>
              </div>
              <div>
                <dt>Runtime</dt>
                <dd>{pendingPackage.runtime ?? "Unsupported metadata only"}</dd>
              </div>
              <div>
                <dt>Contents</dt>
                <dd>
                  {pendingPackage.file_count} files · {compactBytes(
                    pendingPackage.expanded_size,
                  )}
                </dd>
              </div>
              <div>
                <dt>SHA-256</dt>
                <dd class="package-digest">{pendingPackage.archive_sha256}</dd>
              </div>
            </dl>
            <p class="publisher-warning">
              Publisher identity is not verified in package format v1. Install
              only if you trust where this file came from.
            </p>
            <div class="package-actions">
              <button
                class="secondary"
                type="button"
                disabled={busy}
                onclick={oncancelpackage}>Cancel</button
              >
              <button
                class="primary"
                type="button"
                disabled={busy}
                onclick={oninstallpackage}>Install package</button
              >
            </div>
          </div>
        {/if}

        {#if managedPackages.length > 0}
          <div class="managed-package-list">
            {#each managedPackages as managed (managed.id)}
              <article class="managed-package">
                <div>
                  <strong>{managed.name}</strong>
                  <span>
                    v{managed.active_version} · {managed.enabled
                      ? "Enabled"
                      : "Disabled"}
                  </span>
                  <small>{managed.id}</small>
                </div>
                {#if removalCandidate === managed.id}
                  <div class="removal-confirmation">
                    <span
                      >Remove every installed version and revoke its saved
                      access?</span
                    >
                    <button
                      class="secondary"
                      type="button"
                      disabled={busy}
                      onclick={() => (removalCandidate = null)}>Keep</button
                    >
                    <button
                      class="danger"
                      type="button"
                      disabled={busy}
                      onclick={() => {
                        removalCandidate = null;
                        onpackageremove(managed.id);
                      }}>Remove</button
                    >
                  </div>
                {:else}
                  <div class="managed-package-actions">
                    <button
                      class="secondary"
                      type="button"
                      disabled={busy || packageProcessIsActive(managed.id)}
                      onclick={() =>
                        onpackageenable(managed.id, !managed.enabled)}
                      >{managed.enabled ? "Disable" : "Enable"}</button
                    >
                    <button
                      class="secondary"
                      type="button"
                      disabled={busy ||
                        !managed.rollback_version ||
                        packageProcessIsActive(managed.id)}
                      onclick={() => onpackagerollback(managed.id)}
                      >Rollback{managed.rollback_version
                        ? ` to v${managed.rollback_version}`
                        : ""}</button
                    >
                    <button
                      class="danger"
                      type="button"
                      disabled={busy || packageProcessIsActive(managed.id)}
                      onclick={() => (removalCandidate = managed.id)}
                      >Remove</button
                    >
                  </div>
                {/if}
              </article>
            {/each}
          </div>
        {/if}
      </section>

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
        <section
          class="forge-explorer"
          aria-label="Installed plugin exploration"
        >
          <label class="forge-search">
            <span>Find a plugin, author, capability, state, or error</span>
            <input type="search" bind:value={filters.query} />
          </label>
          <details class="forge-filter-panel">
            <summary>
              <span>Filter and sort</span>
              {#if activeFilterCount > 0}<strong
                  >{activeFilterCount} active</strong
                >{/if}
            </summary>
            <div class="forge-filter-grid">
              <label>
                <span>Process state</span>
                <select bind:value={filters.state}>
                  <option value="all">Any reported state</option>
                  {#each filterOptions.states as state}
                    <option value={state}>{state}</option>
                  {/each}
                </select>
              </label>
              <label>
                <span>Access review</span>
                <select bind:value={filters.access}>
                  <option value="all">Approved and awaiting review</option>
                  <option value="approved">All requested access approved</option
                  >
                  <option value="review">Permission review required</option>
                </select>
              </label>
              <label>
                <span>Requested capability</span>
                <select
                  value={filters.capability ?? ""}
                  onchange={(event) =>
                    (filters.capability =
                      (event.currentTarget.value as PluginPermission) || null)}
                >
                  <option value="">Any requested capability</option>
                  {#each filterOptions.capabilities as capability}
                    <option value={capability}
                      >{permissionLabel(capability)}</option
                    >
                  {/each}
                </select>
              </label>
              <label>
                <span>Order plugins by</span>
                <select bind:value={filters.sort}>
                  <option value="name">Plugin name</option>
                  <option value="state">Process state</option>
                </select>
              </label>
            </div>
          </details>
          <ExplorationSummary
            shown={visiblePlugins.length}
            total={status.plugins.length}
            label="installed plugins"
            activeFilters={activeFilterCount}
            onclear={resetFilters}
          />
        </section>
        <div class="plugin-list" aria-label="Installed plugins">
          {#each visiblePlugins as plugin}
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
                      <span>{permissionLabel(permission)}</span>
                      <small>
                        {plugin.granted_permissions.includes(permission)
                          ? $translation("security-approved-lifetime", {
                              lifetime: lifetimeLabel(
                                plugin.grant_lifetime ?? "standing",
                              ),
                            })
                          : "Denied"}
                      </small>
                    </li>
                  {/each}
                </ul>
              </div>

              {#if plugin.weather_capabilities.length > 0}
                <div class="provider-scope">
                  <span class="panel-label">Weather provider scope</span>
                  <p>
                    {plugin.weather_capabilities
                      .map((capability) => capability.replaceAll("_", " "))
                      .join(" · ")}
                  </p>
                  {#each plugin.network_origins as origin}
                    <code>{origin}</code>
                  {/each}
                </div>
              {/if}

              {#if plugin.configuration.length > 0}
                <div class="configuration-panel">
                  <span class="panel-label"
                    >{$translation("forge-host-settings")}</span
                  >
                  <p>{$translation("forge-host-settings-detail")}</p>
                  <div class="configuration-grid">
                    {#each plugin.configuration as setting (setting.key)}
                      {@const presentation = pluginSettingPresentation(
                        setting.key,
                      )}
                      <label>
                        <span>{$translation(presentation.label)}</span>
                        <select
                          value={setting.value}
                          disabled={busy}
                          onchange={(event) =>
                            onconfigurationchange(
                              plugin.id,
                              setting.key,
                              event.currentTarget.value,
                            )}
                        >
                          {#each setting.choices as choice (choice.value)}
                            <option value={choice.value}
                              >{$translation(
                                pluginSettingChoiceKey(choice.value),
                              )}</option
                            >
                          {/each}
                        </select>
                        <small>{$translation(presentation.detail)}</small>
                      </label>
                    {/each}
                  </div>
                </div>
              {/if}

              {#if plugin.last_error}<p class="plugin-error" role="alert">
                  {plugin.last_error}
                </p>{/if}

              <footer>
                <span>
                  {plugin.state === "running"
                    ? `${plugin.published_layer_count} Atlas · ${plugin.published_weather_layer_count} weather published`
                    : "Python 3 runtime · framed protocol v1"}
                </span>
                <div>
                  <label class="startup-choice">
                    <input
                      type="checkbox"
                      checked={plugin.start_with_wyrmgrid}
                      disabled={busy || plugin.grant_lifetime !== "standing"}
                      onchange={(event) =>
                        onstartupchange(plugin.id, event.currentTarget.checked)}
                    />
                    <span>
                      <strong
                        >{$translation("forge-start-with-wyrmgrid")}</strong
                      >
                      <small
                        >{$translation(
                          plugin.grant_lifetime === "standing"
                            ? "forge-start-with-wyrmgrid-detail"
                            : "forge-start-with-wyrmgrid-standing-required",
                        )}</small
                      >
                    </span>
                  </label>
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
                    <label class="lifetime-choice">
                      <span>{$translation("security-approval-duration")}</span>
                      <select bind:value={approvalLifetime} disabled={busy}>
                        <option value="once"
                          >{$translation("security-lifetime-once")}</option
                        >
                        <option value="session"
                          >{$translation("security-lifetime-session")}</option
                        >
                        <option value="standing"
                          >{$translation("security-lifetime-standing")}</option
                        >
                      </select>
                    </label>
                    <button
                      class="secondary"
                      type="button"
                      disabled={busy || plugin.state !== "stopped"}
                      onclick={() => onapprove(plugin.id, approvalLifetime)}
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
          {:else}
            <div class="empty-state filtered">
              <strong>No plugins match these controls</strong>
              <span
                >Clear the presentation filters to review every installed
                plugin.</span
              >
            </div>
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
  .package-workshop {
    display: grid;
    gap: 12px;
    margin: 14px 24px 0;
    padding: 15px;
    border: 1px solid var(--color-line-faint);
    background: var(--color-surface-soft);
  }
  .package-workshop-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
  }
  .package-workshop-heading h3 {
    font-size: 18px;
  }
  .package-workshop-heading p,
  .package-review span,
  .managed-package span,
  .managed-package small {
    display: block;
    margin-top: 4px;
    color: var(--color-text-muted);
    font-size: 10px;
    line-height: 1.45;
  }
  .package-workshop code {
    color: var(--color-highlight);
  }
  .package-workshop button {
    padding: 8px 11px;
    border-radius: 4px;
    font: inherit;
    font-size: 10px;
    font-weight: 700;
    cursor: pointer;
  }
  .package-choice {
    flex: 0 0 auto;
  }
  .package-review {
    display: grid;
    gap: 12px;
    padding: 13px;
    border: 1px solid var(--color-highlight-border);
    background: var(--color-highlight-soft);
  }
  .package-review dl {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 9px;
    margin: 0;
  }
  .package-review dl div {
    min-width: 0;
  }
  .package-review dt {
    color: var(--color-text-muted);
    font-size: 8px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .package-review dd {
    margin: 3px 0 0;
    color: var(--color-text);
    font-size: 10px;
  }
  .package-digest {
    overflow-wrap: anywhere;
    font-family: monospace;
  }
  .publisher-warning {
    padding: 9px 10px;
    border-left: 2px solid var(--color-highlight);
    color: var(--color-text-muted);
    background: var(--color-surface-soft-translucent);
    font-size: 10px;
  }
  .package-actions,
  .managed-package-actions,
  .removal-confirmation {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 7px;
  }
  .managed-package-list {
    display: grid;
    gap: 8px;
  }
  .managed-package {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 12px;
  }
  .managed-package > div:first-child {
    min-width: 0;
  }
  .managed-package small {
    overflow-wrap: anywhere;
  }
  .removal-confirmation span {
    max-width: 230px;
    color: var(--color-danger);
    font-size: 9px;
  }
  .danger {
    border: 1px solid var(--color-danger-border);
    color: var(--color-danger);
    background: var(--color-danger-soft);
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
  .forge-explorer {
    display: grid;
    gap: 12px;
    padding: 18px 24px 0;
  }
  .forge-search,
  .forge-filter-grid label {
    display: grid;
    gap: 6px;
    color: var(--color-text-muted);
    font-size: 10px;
  }
  .forge-search input,
  .forge-filter-grid select {
    min-width: 0;
    border: 1px solid var(--color-line-soft);
    border-radius: 4px;
    padding: 9px 10px;
    color: var(--color-text);
    background: var(--color-surface);
    font: inherit;
  }
  .forge-filter-panel {
    border: 1px solid var(--color-line-faint);
    padding: 10px 12px;
    background: var(--color-surface-soft);
  }
  .forge-filter-panel > summary {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    cursor: pointer;
  }
  .forge-filter-panel > summary strong {
    color: var(--color-accent);
    font-size: 9px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .forge-filter-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
    margin-top: 12px;
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
  .provider-scope {
    display: grid;
    gap: 6px;
    margin: 10px 18px 0;
    padding: 11px 13px;
    border-left: 2px solid var(--color-highlight);
    background: var(--color-surface-soft-translucent);
  }
  .provider-scope p {
    margin: 0;
    color: var(--color-text);
    font-size: 10px;
    text-transform: capitalize;
  }
  .provider-scope code {
    color: var(--color-text-muted);
    font-family: inherit;
    font-size: 9px;
    overflow-wrap: anywhere;
  }
  .configuration-panel {
    display: grid;
    gap: 7px;
    margin: 10px 18px 0;
    padding: 12px 13px;
    border: 1px solid var(--color-line-faint);
    background: var(--color-surface-soft);
  }
  .configuration-panel > p {
    margin: 0;
    color: var(--color-text-muted);
    font-size: 10px;
  }
  .configuration-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 10px;
  }
  .configuration-grid label {
    display: grid;
    gap: 5px;
    color: var(--color-text);
    font-size: 10px;
  }
  .configuration-grid select {
    min-width: 0;
    border: 1px solid var(--color-line-soft);
    border-radius: 4px;
    padding: 8px 9px;
    color: var(--color-text);
    background: var(--color-surface);
    font: inherit;
  }
  .configuration-grid small {
    color: var(--color-text-muted);
    line-height: 1.45;
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
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }
  .startup-choice {
    display: grid;
    grid-template-columns: auto minmax(150px, 220px);
    align-items: start;
    gap: 7px;
    color: var(--color-text-muted);
    font-size: 9px;
  }
  .startup-choice input {
    margin-top: 2px;
    accent-color: var(--color-accent);
  }
  .startup-choice span {
    display: grid;
    gap: 2px;
  }
  .startup-choice strong {
    color: var(--color-text);
    font-size: 10px;
  }
  .startup-choice small {
    line-height: 1.35;
  }
  .lifetime-choice {
    display: grid;
    gap: 3px;
    color: var(--color-text-muted);
    font-size: 8px;
  }
  .lifetime-choice select {
    border: 1px solid var(--color-line-soft);
    border-radius: 4px;
    padding: 6px 8px;
    color: var(--color-text);
    background: var(--color-surface);
    font: inherit;
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
  .empty-state.filtered {
    margin: 0;
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
    .package-workshop-heading,
    .managed-package,
    article footer {
      grid-template-columns: 1fr;
    }
    .package-workshop-heading,
    .managed-package {
      align-items: stretch;
      flex-direction: column;
    }
    .package-review dl {
      grid-template-columns: 1fr;
    }
    .forge-filter-grid {
      grid-template-columns: 1fr;
    }
    article footer {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
