<script lang="ts">
  import { translation } from "$lib/i18n/runtime";
  import type { TranslationKey } from "$lib/i18n/catalog";
  import { formatLocalDateTime } from "$lib/presentation/dateTime";
  import type {
    AudioPlaybackView,
    AudioProviderPackageInspection,
    AudioRecordingPreferences,
    AudioRecordingView,
    AudioSessionSummary,
    AudioSourceSelection,
    AudioSourceView,
    ManagedAudioProviderPackage,
  } from "./types";

  let {
    status,
    playback,
    managedProviders,
    pendingProviderPackage,
    busy = false,
    onpreferences,
    onchooseproviderpackage,
    oncancelproviderpackage,
    oninstallproviderpackage,
    onproviderselect,
    onproviderenable,
    onproviderrollback,
    onproviderremove,
    onrefresh,
    onpermission,
    onsource,
    onplayback,
    onexport,
    ondelete,
  }: {
    status: AudioRecordingView;
    playback?: AudioPlaybackView;
    managedProviders: ManagedAudioProviderPackage[];
    pendingProviderPackage?: AudioProviderPackageInspection;
    busy?: boolean;
    onpreferences: (preferences: AudioRecordingPreferences) => void;
    onchooseproviderpackage: () => void;
    oncancelproviderpackage: () => void;
    oninstallproviderpackage: () => void;
    onproviderselect: (providerId: string) => void;
    onproviderenable: (providerId: string, enabled: boolean) => void;
    onproviderrollback: (providerId: string) => void;
    onproviderremove: (providerId: string) => void;
    onrefresh: () => void;
    onpermission: (sourceId: string) => void;
    onsource: (selection: AudioSourceSelection) => void;
    onplayback: (sessionId: string) => void;
    onexport: (sessionId: string, trackId: string) => void;
    ondelete: (sessionId: string) => void;
  } = $props();

  let removalCandidate = $state<string>();

  const availabilityLabels = {
    available: "audio-source-available",
    unavailable: "audio-source-unavailable",
  } satisfies Record<AudioSourceView["availability"], TranslationKey>;
  const permissionLabels = {
    not_required: "audio-permission-not-required",
    prompt_required: "audio-permission-prompt-required",
    granted: "audio-permission-granted",
    denied: "audio-permission-denied",
  } satisfies Record<AudioSourceView["permission"], TranslationKey>;
  const captureModeLabels = {
    manual: "audio-mode-manual",
    automatic: "audio-mode-automatic",
  } satisfies Record<AudioSessionSummary["capture_mode"], TranslationKey>;
  const sessionStatusLabels = {
    active: "audio-session-active",
    completed: "audio-session-completed",
    interrupted: "audio-session-interrupted",
  } satisfies Record<AudioSessionSummary["status"], TranslationKey>;
  const mediaAvailabilityLabels = {
    available: "audio-media-available",
    not_in_backup: "audio-media-not-in-backup",
    missing: "audio-media-missing",
    tombstoned: "audio-media-deletion-pending",
  } satisfies Record<AudioSessionSummary["media_availability"], TranslationKey>;

  function updatePreferences(patch: Partial<AudioRecordingPreferences>): void {
    onpreferences({ ...status.preferences, ...patch });
  }

  function updateSource(
    source: AudioSourceView,
    patch: Partial<AudioSourceSelection>,
  ): void {
    const profile = source.supported_profiles[0];
    if (!status.provider_id || !profile) return;
    onsource({
      provider_id: status.provider_id,
      source_id: source.id,
      profile_id: profile,
      enabled: source.enabled,
      playback_muted: source.playback_muted,
      playback_solo: source.playback_solo,
      playback_volume_percent: source.playback_volume_percent,
      ...patch,
    });
  }

  function mediaSize(bytes: number): string {
    return `${(bytes / 1024).toFixed(1)} KiB`;
  }

  function compactBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
  }
</script>

<section class="audio-recording-panel" aria-labelledby="audio-recording-title">
  <header>
    <div>
      <span class="eyebrow">{$translation("audio-recording-eyebrow")}</span>
      <h3 id="audio-recording-title">
        {$translation("audio-recording-title")}
      </h3>
    </div>
    {#if status.recording_active}
      <strong class="audio-active-indicator" role="status">
        <span aria-hidden="true">●</span>
        {$translation("audio-recording-active")}
      </strong>
    {/if}
  </header>

  <p class="audio-boundary">{$translation("audio-recording-boundary")}</p>

  <section
    class="provider-packages audio-provider-packages"
    aria-labelledby="audio-provider-package-title"
  >
    <div class="provider-package-heading">
      <div>
        <span class="eyebrow"
          >{$translation("audio-provider-packages-eyebrow")}</span
        >
        <h4 id="audio-provider-package-title">
          {$translation("audio-provider-packages-title")}
        </h4>
        <p>{$translation("audio-provider-packages-introduction")}</p>
      </div>
      <button
        type="button"
        disabled={busy || pendingProviderPackage !== undefined}
        onclick={onchooseproviderpackage}
        >{$translation("audio-provider-packages-choose")}</button
      >
    </div>

    {#if pendingProviderPackage}
      <div class="provider-package-review">
        <div>
          <strong>{pendingProviderPackage.name}</strong>
          <span>
            {$translation("audio-provider-package-by-author", {
              author: pendingProviderPackage.author,
              version: pendingProviderPackage.version,
            })}
          </span>
        </div>
        <dl>
          <div>
            <dt>{$translation("audio-provider-package-identity")}</dt>
            <dd>{pendingProviderPackage.id}</dd>
          </div>
          <div>
            <dt>{$translation("audio-provider-package-compatibility")}</dt>
            <dd>
              {$translation("audio-provider-package-compatibility-value", {
                platforms: pendingProviderPackage.platforms.join(" · "),
                protocol: pendingProviderPackage.audio_protocol_version,
              })}
            </dd>
          </div>
          <div>
            <dt>{$translation("audio-provider-package-contents")}</dt>
            <dd>
              {$translation("audio-provider-package-contents-value", {
                count: pendingProviderPackage.file_count,
                size: compactBytes(pendingProviderPackage.expanded_size),
              })}
            </dd>
          </div>
          <div>
            <dt>{$translation("audio-provider-package-capabilities")}</dt>
            <dd>{pendingProviderPackage.capabilities.join(" · ")}</dd>
          </div>
          <div>
            <dt>{$translation("audio-provider-package-archive-digest")}</dt>
            <dd class="provider-package-digest">
              {pendingProviderPackage.archive_sha256}
            </dd>
          </div>
        </dl>
        <p class="provider-package-warning">
          {$translation("security-audio-provider-package-warning")}
        </p>
        <div class="provider-package-actions">
          <button
            class="secondary"
            type="button"
            disabled={busy}
            onclick={oncancelproviderpackage}
            >{$translation("action-cancel")}</button
          >
          <button
            type="button"
            disabled={busy}
            onclick={oninstallproviderpackage}
            >{$translation("audio-provider-package-install")}</button
          >
        </div>
      </div>
    {/if}

    {#if managedProviders.length === 0}
      <p class="audio-empty-state">
        {$translation("audio-provider-packages-empty")}
      </p>
    {:else}
      <div class="managed-provider-packages">
        {#each managedProviders as managed (managed.id)}
          <article>
            <div class="managed-provider-summary">
              <strong>{managed.name}</strong>
              <span>
                {managed.enabled
                  ? $translation("audio-provider-package-enabled", {
                      version: managed.active_version,
                    })
                  : $translation("audio-provider-package-disabled", {
                      version: managed.active_version,
                    })}
                {#if status.provider_id === managed.id}
                  · {$translation("audio-provider-package-selected")}
                {/if}
              </span>
              <small>{managed.id}</small>
            </div>
            {#if removalCandidate === managed.id}
              <div class="provider-removal-confirmation">
                <span
                  >{$translation(
                    "destructive-audio-provider-package-remove-confirm",
                  )}</span
                >
                <button
                  class="secondary"
                  type="button"
                  disabled={busy}
                  onclick={() => (removalCandidate = undefined)}
                  >{$translation("audio-provider-package-keep")}</button
                >
                <button
                  class="stop"
                  type="button"
                  disabled={busy}
                  onclick={() => {
                    removalCandidate = undefined;
                    onproviderremove(managed.id);
                  }}
                  >{$translation("audio-provider-package-remove")}</button
                >
              </div>
            {:else}
              <div class="provider-package-actions">
                <button
                  type="button"
                  disabled={busy || !managed.enabled || status.recording_active}
                  onclick={() => onproviderselect(managed.id)}
                  >{status.provider_id === managed.id
                    ? $translation("audio-provider-package-selected")
                    : $translation("audio-provider-package-select")}</button
                >
                <button
                  class="secondary"
                  type="button"
                  disabled={busy || status.recording_active}
                  onclick={() =>
                    onproviderenable(managed.id, !managed.enabled)}
                >
                  {managed.enabled
                    ? $translation("audio-provider-package-disable")
                    : $translation("audio-provider-package-enable")}
                </button>
                <button
                  class="secondary"
                  type="button"
                  disabled={busy ||
                    !managed.rollback_version ||
                    status.recording_active}
                  onclick={() => onproviderrollback(managed.id)}
                  >{$translation("audio-provider-package-rollback")}</button
                >
                <button
                  class="stop"
                  type="button"
                  disabled={busy || status.recording_active}
                  onclick={() => (removalCandidate = managed.id)}
                  >{$translation("audio-provider-package-remove-menu")}</button
                >
              </div>
            {/if}
          </article>
        {/each}
      </div>
    {/if}
  </section>

  <div class="audio-preferences">
    <label class:enabled={status.preferences.enabled} class="audio-consent-row">
      <input
        type="checkbox"
        checked={status.preferences.enabled}
        disabled={busy}
        onchange={(event) =>
          updatePreferences({ enabled: event.currentTarget.checked })}
      />
      <span>
        <strong>{$translation("audio-recording-enable")}</strong>
        <small>{$translation("audio-recording-enable-detail")}</small>
      </span>
    </label>

    <div class="audio-mode-grid">
      <label class:disabled={busy || !status.preferences.enabled}>
        <input
          type="checkbox"
          checked={status.preferences.capture_manual}
          disabled={busy || !status.preferences.enabled}
          onchange={(event) =>
            updatePreferences({ capture_manual: event.currentTarget.checked })}
        />
        <span>{$translation("audio-recording-manual")}</span>
      </label>
      <label class:disabled={busy || !status.preferences.enabled}>
        <input
          type="checkbox"
          checked={status.preferences.capture_automatic}
          disabled={busy || !status.preferences.enabled}
          onchange={(event) =>
            updatePreferences({
              capture_automatic: event.currentTarget.checked,
            })}
        />
        <span>{$translation("audio-recording-automatic")}</span>
      </label>
    </div>
  </div>

  {#if !status.provider_available}
    <div class="audio-unavailable" role="status">
      <span class="audio-status-mark" aria-hidden="true">!</span>
      <span>{$translation("audio-provider-unavailable")}</span>
    </div>
  {:else if status.preferences.enabled}
    <div class="audio-source-heading">
      <strong>{$translation("audio-sources-title")}</strong>
      <button type="button" disabled={busy} onclick={onrefresh}>
        {$translation("audio-sources-refresh")}
      </button>
    </div>
    {#if status.sources.length === 0}
      <p>{$translation("audio-sources-empty")}</p>
    {:else}
      <div class="audio-source-list">
        {#each status.sources as source}
          <article>
            <div>
              <strong>{source.display_name}</strong>
              <small>
                {$translation(availabilityLabels[source.availability])} ·
                {$translation(permissionLabels[source.permission])}
                {#if source.peak_millidbfs !== undefined}
                  · {(source.peak_millidbfs / 1000).toFixed(1)} dBFS
                {/if}
                {#if source.clipped}
                  · {$translation("audio-source-clipped")}{/if}
              </small>
            </div>
            <label>
              <input
                type="checkbox"
                checked={source.enabled}
                disabled={busy || source.availability !== "available"}
                onchange={(event) =>
                  updateSource(source, {
                    enabled: event.currentTarget.checked,
                  })}
              />
              {$translation("audio-source-record")}
            </label>
            {#if source.permission === "prompt_required"}
              <button
                type="button"
                disabled={busy}
                onclick={() => onpermission(source.id)}
              >
                {$translation("audio-source-permission")}
              </button>
            {/if}
          </article>
        {/each}
      </div>
    {/if}
  {/if}

  <section class="audio-session-list" aria-labelledby="audio-sessions-title">
    <div class="audio-section-heading">
      <strong id="audio-sessions-title"
        >{$translation("audio-sessions-title")}</strong
      >
    </div>
    {#if status.sessions.length === 0}
      <p class="audio-empty-state">
        <span aria-hidden="true">◇</span>
        <span>{$translation("audio-sessions-empty")}</span>
      </p>
    {:else}
      {#each status.sessions as session}
        <article class:active={session.status === "active"}>
          <div>
            <strong
              >{formatLocalDateTime(
                session.started_at,
                session.started_at,
              )}</strong
            >
            <small>
              {$translation(captureModeLabels[session.capture_mode])} ·
              {$translation(sessionStatusLabels[session.status])} ·
              {$translation(
                mediaAvailabilityLabels[session.media_availability],
              )}
              · {mediaSize(session.total_media_bytes)}
            </small>
          </div>
          <div class="audio-session-actions">
            <button
              type="button"
              disabled={busy ||
                session.status === "active" ||
                session.media_availability !== "available"}
              onclick={() => onplayback(session.id)}
            >
              {$translation("audio-playback-open")}
            </button>
            <button
              type="button"
              class="danger"
              disabled={busy || session.status === "active"}
              onclick={() => ondelete(session.id)}
            >
              {$translation("audio-session-delete")}
            </button>
          </div>
        </article>
      {/each}
    {/if}
  </section>

  {#if playback}
    <div class="audio-playback" aria-live="polite">
      <strong>{$translation("audio-playback-authenticated")}</strong>
      <p>{$translation("audio-playback-packet-boundary")}</p>
      {#each playback.tracks as track}
        <article>
          <div>
            <strong>{track.source_id}</strong>
            <small>
              {track.packets.length}
              {$translation("audio-playback-packets")} ·
              {track.frame_count}
              {$translation("audio-playback-frames")}
            </small>
          </div>
          <button
            type="button"
            disabled={busy}
            onclick={() => onexport(playback.session_id, track.track_id)}
          >
            {$translation("audio-export-track")}
          </button>
        </article>
      {/each}
    </div>
  {/if}
</section>
