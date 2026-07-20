CREATE TABLE IF NOT EXISTS audio_recording_preferences (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    capture_manual INTEGER NOT NULL DEFAULT 0 CHECK (capture_manual IN (0, 1)),
    capture_automatic INTEGER NOT NULL DEFAULT 0 CHECK (capture_automatic IN (0, 1)),
    retention_days INTEGER NOT NULL DEFAULT 30 CHECK (retention_days BETWEEN 1 AND 3650),
    storage_budget_bytes INTEGER NOT NULL DEFAULT 5368709120
        CHECK (storage_budget_bytes BETWEEN 16777216 AND 1099511627776),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS audio_source_selections (
    provider_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    profile_id TEXT NOT NULL CHECK (
        profile_id IN ('pilot_microphone_v1', 'isolated_voice_v1', 'mixed_stereo_v1')
    ),
    enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    playback_muted INTEGER NOT NULL DEFAULT 0 CHECK (playback_muted IN (0, 1)),
    playback_solo INTEGER NOT NULL DEFAULT 0 CHECK (playback_solo IN (0, 1)),
    playback_volume_percent INTEGER NOT NULL DEFAULT 100
        CHECK (playback_volume_percent BETWEEN 0 AND 200),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (provider_id, source_id)
);

CREATE TABLE IF NOT EXISTS audio_recording_sessions (
    id TEXT PRIMARY KEY,
    simulator_session_id TEXT REFERENCES simulator_sessions(id) ON DELETE SET NULL,
    provider_id TEXT NOT NULL,
    capture_mode TEXT NOT NULL CHECK (capture_mode IN ('manual', 'automatic')),
    started_at TEXT NOT NULL,
    ended_at TEXT,
    host_start_monotonic_ns INTEGER CHECK (host_start_monotonic_ns >= 0),
    status TEXT NOT NULL CHECK (status IN ('active', 'completed', 'interrupted')),
    media_availability TEXT NOT NULL DEFAULT 'available'
        CHECK (media_availability IN ('available', 'not_in_backup', 'missing', 'tombstoned')),
    total_media_bytes INTEGER NOT NULL DEFAULT 0 CHECK (total_media_bytes >= 0),
    deletion_requested_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_audio_recording_sessions_started
    ON audio_recording_sessions(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_audio_recording_sessions_simulator
    ON audio_recording_sessions(simulator_session_id);

CREATE TABLE IF NOT EXISTS audio_tracks (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES audio_recording_sessions(id) ON DELETE CASCADE,
    source_id TEXT NOT NULL,
    profile_id TEXT NOT NULL CHECK (
        profile_id IN ('pilot_microphone_v1', 'isolated_voice_v1', 'mixed_stereo_v1')
    ),
    source_role TEXT NOT NULL CHECK (
        source_role IN (
            'microphone_input', 'application_output', 'output_endpoint',
            'simulator_master_mix', 'isolated_com1', 'isolated_com2',
            'pilot_radio', 'copilot_radio'
        )
    ),
    source_truth TEXT NOT NULL CHECK (source_truth IN ('isolated', 'mixed_output', 'metadata_only')),
    channel_count INTEGER NOT NULL CHECK (channel_count BETWEEN 1 AND 8),
    sample_rate_hz INTEGER NOT NULL CHECK (sample_rate_hz = 48000),
    provider_start_monotonic_ns INTEGER NOT NULL CHECK (provider_start_monotonic_ns >= 0),
    packet_count INTEGER NOT NULL DEFAULT 0 CHECK (packet_count >= 0),
    frame_count INTEGER NOT NULL DEFAULT 0 CHECK (frame_count >= 0),
    last_packet_sequence INTEGER CHECK (last_packet_sequence > 0),
    UNIQUE(session_id, source_id)
);

CREATE INDEX IF NOT EXISTS idx_audio_tracks_session
    ON audio_tracks(session_id);

CREATE TABLE IF NOT EXISTS audio_track_segments (
    track_id TEXT NOT NULL REFERENCES audio_tracks(id) ON DELETE CASCADE,
    segment_index INTEGER NOT NULL CHECK (segment_index >= 0),
    storage_key TEXT NOT NULL UNIQUE CHECK (
        length(storage_key) = 32
        AND storage_key NOT GLOB '*[^0-9a-f]*'
    ),
    first_frame INTEGER NOT NULL CHECK (first_frame >= 0),
    frame_count INTEGER NOT NULL CHECK (frame_count >= 0),
    packet_count INTEGER NOT NULL CHECK (packet_count > 0),
    encrypted_bytes INTEGER NOT NULL CHECK (encrypted_bytes > 0),
    envelope_sha256 TEXT NOT NULL CHECK (
        length(envelope_sha256) = 64
        AND envelope_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    envelope_version INTEGER NOT NULL CHECK (envelope_version > 0),
    key_version INTEGER NOT NULL CHECK (key_version > 0),
    state TEXT NOT NULL CHECK (state IN ('pending', 'complete', 'unavailable', 'tombstoned')),
    created_at TEXT NOT NULL,
    deletion_requested_at TEXT,
    PRIMARY KEY (track_id, segment_index)
);

CREATE INDEX IF NOT EXISTS idx_audio_segments_state
    ON audio_track_segments(state, created_at);

CREATE TABLE IF NOT EXISTS audio_capture_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES audio_recording_sessions(id) ON DELETE CASCADE,
    track_id TEXT REFERENCES audio_tracks(id) ON DELETE CASCADE,
    provider_monotonic_ns INTEGER NOT NULL CHECK (provider_monotonic_ns >= 0),
    event_kind TEXT NOT NULL CHECK (
        event_kind IN (
            'permission_required', 'permission_denied', 'source_unavailable',
            'source_changed', 'gap', 'dropout', 'drift', 'backpressure',
            'encoder_failure'
        )
    ),
    code TEXT NOT NULL,
    affected_frames INTEGER CHECK (affected_frames BETWEEN 1 AND 2880000),
    drift_parts_per_million INTEGER CHECK (
        drift_parts_per_million BETWEEN -100000 AND 100000
    ),
    observed_at TEXT NOT NULL,
    CHECK (
        (event_kind IN ('gap', 'dropout', 'backpressure')
            AND affected_frames IS NOT NULL
            AND drift_parts_per_million IS NULL)
        OR (event_kind = 'drift'
            AND affected_frames IS NULL
            AND drift_parts_per_million IS NOT NULL)
        OR (event_kind NOT IN ('gap', 'dropout', 'backpressure', 'drift')
            AND affected_frames IS NULL
            AND drift_parts_per_million IS NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_audio_events_session
    ON audio_capture_events(session_id, id);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (18);
