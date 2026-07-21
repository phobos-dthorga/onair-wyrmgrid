<script lang="ts">
  import { translation } from "$lib/i18n/runtime";
  import type { TranslationKey } from "$lib/i18n/catalog";
  import { formatLocalDateTime } from "$lib/presentation/dateTime";
  import type {
    AudioPlaybackView,
    AudioRecordingPreferences,
    AudioRecordingView,
    AudioSessionSummary,
    AudioSourceSelection,
    AudioSourceView,
  } from "./types";

  let {
    status,
    playback,
    busy = false,
    onpreferences,
    onrefresh,
    onpermission,
    onsource,
    onplayback,
    onexport,
    ondelete,
  }: {
    status: AudioRecordingView;
    playback?: AudioPlaybackView;
    busy?: boolean;
    onpreferences: (preferences: AudioRecordingPreferences) => void;
    onrefresh: () => void;
    onpermission: (sourceId: string) => void;
    onsource: (selection: AudioSourceSelection) => void;
    onplayback: (sessionId: string) => void;
    onexport: (sessionId: string, trackId: string) => void;
    ondelete: (sessionId: string) => void;
  } = $props();

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
    const codecProviderId =
      source.codec_provider_id ??
      status.codecs.find((codec) => codec.supported_profiles.includes(profile))
        ?.id;
    if (!codecProviderId) return;
    onsource({
      provider_id: status.provider_id,
      source_id: source.id,
      profile_id: profile,
      codec_provider_id: codecProviderId,
      enabled: source.enabled,
      playback_muted: source.playback_muted,
      playback_solo: source.playback_solo,
      playback_volume_percent: source.playback_volume_percent,
      ...patch,
    });
  }

  function compatibleCodecs(source: AudioSourceView) {
    return status.codecs.filter((codec) =>
      source.supported_profiles.some((profile) =>
        codec.supported_profiles.includes(profile),
      ),
    );
  }

  function selectedCodecAvailable(source: AudioSourceView): boolean {
    return (
      source.codec_provider_id === undefined ||
      compatibleCodecs(source).some(
        (codec) => codec.id === source.codec_provider_id,
      )
    );
  }

  function mediaSize(bytes: number): string {
    return `${(bytes / 1024).toFixed(1)} KiB`;
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
            <label class="audio-codec-choice">
              <span>{$translation("audio-codec-label")}</span>
              <select
                value={source.codec_provider_id ?? compatibleCodecs(source)[0]?.id}
                disabled={busy || compatibleCodecs(source).length === 0}
                onchange={(event) =>
                  updateSource(source, {
                    codec_provider_id: event.currentTarget.value,
                  })}
              >
                {#if source.codec_provider_id && !selectedCodecAvailable(source)}
                  <option value={source.codec_provider_id} disabled>
                    {$translation("audio-codec-unavailable")}
                  </option>
                {/if}
                {#each compatibleCodecs(source) as codec}
                  <option value={codec.id}>{codec.name}</option>
                {/each}
              </select>
            </label>
            {#if compatibleCodecs(source).length === 0 || !selectedCodecAvailable(source)}
              <small>{$translation("audio-codec-unavailable")}</small>
            {/if}
            <label>
              <input
                type="checkbox"
                checked={source.enabled}
                disabled={busy ||
                  source.availability !== "available" ||
                  compatibleCodecs(source).length === 0 ||
                  !selectedCodecAvailable(source)}
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
