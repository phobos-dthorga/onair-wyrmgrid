use rusqlite::{OptionalExtension, params};

use crate::{StorageError, Store};

const MAX_AUDIO_EVENT_FRAMES: u64 = 48_000 * 60;
const MAX_ABSOLUTE_AUDIO_DRIFT_PPM: i32 = 100_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioRecordingPreferencesRecord {
    pub enabled: bool,
    pub capture_manual: bool,
    pub capture_automatic: bool,
    pub retention_days: u32,
    pub storage_budget_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSourceSelectionRecord {
    pub provider_id: String,
    pub source_id: String,
    pub profile_id: String,
    pub codec_provider_id: String,
    pub enabled: bool,
    pub playback_muted: bool,
    pub playback_solo: bool,
    pub playback_volume_percent: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSessionRecord {
    pub id: String,
    pub simulator_session_id: Option<String>,
    pub provider_id: String,
    pub capture_mode: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub host_start_monotonic_ns: Option<u64>,
    pub status: String,
    pub media_availability: String,
    pub total_media_bytes: u64,
    pub deletion_requested_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioTrackRecord {
    pub id: String,
    pub session_id: String,
    pub source_id: String,
    pub profile_id: String,
    pub codec_provider_id: String,
    pub codec_provider_version: String,
    pub codec_id: String,
    pub codec_media_type: String,
    pub source_role: String,
    pub source_truth: String,
    pub channel_count: u8,
    pub sample_rate_hz: u32,
    pub provider_start_monotonic_ns: u64,
    pub packet_count: u64,
    pub frame_count: u64,
    pub last_packet_sequence: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSegmentRecord {
    pub track_id: String,
    pub segment_index: u32,
    pub storage_key: String,
    pub first_frame: u64,
    pub frame_count: u64,
    pub packet_count: u64,
    pub encrypted_bytes: u64,
    pub envelope_sha256: String,
    pub envelope_version: u16,
    pub key_version: u16,
    pub state: String,
    pub created_at: String,
    pub deletion_requested_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioCaptureEventRecord {
    pub session_id: String,
    pub track_id: Option<String>,
    pub provider_monotonic_ns: u64,
    pub event_kind: String,
    pub code: String,
    pub affected_frames: Option<u64>,
    pub drift_parts_per_million: Option<i32>,
    pub observed_at: String,
}

impl Store {
    pub fn interrupt_active_audio_session_records(
        &self,
        ended_at: &str,
    ) -> Result<usize, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "UPDATE audio_recording_sessions
                 SET status = 'interrupted', ended_at = ?1
                 WHERE status = 'active'",
                [ended_at],
            )
            .map_err(StorageError::from)
    }

    pub fn load_audio_recording_preferences_record(
        &self,
    ) -> Result<Option<AudioRecordingPreferencesRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let values = connection
            .query_row(
                "SELECT enabled, capture_manual, capture_automatic,
                        retention_days, storage_budget_bytes
                 FROM audio_recording_preferences WHERE singleton_id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                },
            )
            .optional()?;
        values.map(audio_preferences_from_values).transpose()
    }

    pub fn save_audio_recording_preferences_record(
        &self,
        record: &AudioRecordingPreferencesRecord,
    ) -> Result<(), StorageError> {
        validate_audio_preferences(record)?;
        let storage_budget_bytes = stored_i64(record.storage_budget_bytes)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection.execute(
            "INSERT INTO audio_recording_preferences
                (singleton_id, enabled, capture_manual, capture_automatic,
                 retention_days, storage_budget_bytes)
             VALUES (1, ?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(singleton_id) DO UPDATE SET
                enabled = excluded.enabled,
                capture_manual = excluded.capture_manual,
                capture_automatic = excluded.capture_automatic,
                retention_days = excluded.retention_days,
                storage_budget_bytes = excluded.storage_budget_bytes,
                updated_at = CURRENT_TIMESTAMP",
            params![
                record.enabled,
                record.capture_manual,
                record.capture_automatic,
                record.retention_days,
                storage_budget_bytes,
            ],
        )?;
        Ok(())
    }

    pub fn list_audio_source_selection_records(
        &self,
    ) -> Result<Vec<AudioSourceSelectionRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT provider_id, source_id, profile_id, codec_provider_id, enabled,
                    playback_muted, playback_solo, playback_volume_percent
             FROM audio_source_selections ORDER BY provider_id, source_id",
        )?;
        let records = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            })?
            .map(|row| {
                row.map_err(StorageError::from)
                    .and_then(audio_selection_from_values)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(records)
    }

    pub fn save_audio_source_selection_record(
        &self,
        record: &AudioSourceSelectionRecord,
    ) -> Result<(), StorageError> {
        validate_machine_id(&record.provider_id, 255)?;
        validate_machine_id(&record.source_id, 128)?;
        validate_profile_id(&record.profile_id)?;
        validate_machine_id(&record.codec_provider_id, 255)?;
        if record.playback_volume_percent > 200 {
            return Err(StorageError::InvalidRecord);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection.execute(
            "INSERT INTO audio_source_selections
                (provider_id, source_id, profile_id, codec_provider_id, enabled, playback_muted,
                 playback_solo, playback_volume_percent)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(provider_id, source_id) DO UPDATE SET
                profile_id = excluded.profile_id,
                codec_provider_id = excluded.codec_provider_id,
                enabled = excluded.enabled,
                playback_muted = excluded.playback_muted,
                playback_solo = excluded.playback_solo,
                playback_volume_percent = excluded.playback_volume_percent,
                updated_at = CURRENT_TIMESTAMP",
            params![
                record.provider_id,
                record.source_id,
                record.profile_id,
                record.codec_provider_id,
                record.enabled,
                record.playback_muted,
                record.playback_solo,
                record.playback_volume_percent,
            ],
        )?;
        Ok(())
    }

    pub fn create_audio_session_record(
        &self,
        session: &AudioSessionRecord,
        tracks: &[AudioTrackRecord],
    ) -> Result<(), StorageError> {
        validate_audio_session(session)?;
        if tracks.is_empty() || tracks.len() > 8 {
            return Err(StorageError::InvalidRecord);
        }
        let host_start_monotonic_ns = session
            .host_start_monotonic_ns
            .map(stored_i64)
            .transpose()?;
        let total_media_bytes = stored_i64(session.total_media_bytes)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO audio_recording_sessions
                (id, simulator_session_id, provider_id, capture_mode, started_at,
                 ended_at, host_start_monotonic_ns, status, media_availability,
                 total_media_bytes, deletion_requested_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                session.id,
                session.simulator_session_id,
                session.provider_id,
                session.capture_mode,
                session.started_at,
                session.ended_at,
                host_start_monotonic_ns,
                session.status,
                session.media_availability,
                total_media_bytes,
                session.deletion_requested_at,
            ],
        )?;
        for track in tracks {
            validate_audio_track(track, &session.id)?;
            let provider_start_monotonic_ns = stored_i64(track.provider_start_monotonic_ns)?;
            let packet_count = stored_i64(track.packet_count)?;
            let frame_count = stored_i64(track.frame_count)?;
            let last_packet_sequence = track.last_packet_sequence.map(stored_i64).transpose()?;
            transaction.execute(
                "INSERT INTO audio_tracks
                    (id, session_id, source_id, profile_id, codec_provider_id,
                     codec_provider_version, codec_id, codec_media_type, source_role, source_truth,
                     channel_count, sample_rate_hz, provider_start_monotonic_ns,
                     packet_count, frame_count, last_packet_sequence)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    track.id,
                    track.session_id,
                    track.source_id,
                    track.profile_id,
                    track.codec_provider_id,
                    track.codec_provider_version,
                    track.codec_id,
                    track.codec_media_type,
                    track.source_role,
                    track.source_truth,
                    track.channel_count,
                    track.sample_rate_hz,
                    provider_start_monotonic_ns,
                    packet_count,
                    frame_count,
                    last_packet_sequence,
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn complete_audio_segment_record(
        &self,
        segment: &AudioSegmentRecord,
        last_packet_sequence: u64,
    ) -> Result<(), StorageError> {
        validate_audio_segment(segment)?;
        if last_packet_sequence == 0 {
            return Err(StorageError::InvalidRecord);
        }
        let first_frame = stored_i64(segment.first_frame)?;
        let frame_count = stored_i64(segment.frame_count)?;
        let packet_count = stored_i64(segment.packet_count)?;
        let encrypted_bytes = stored_i64(segment.encrypted_bytes)?;
        let last_packet_sequence = stored_i64(last_packet_sequence)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO audio_track_segments
                (track_id, segment_index, storage_key, first_frame, frame_count,
                 packet_count, encrypted_bytes, envelope_sha256, envelope_version,
                 key_version, state, created_at, deletion_requested_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                segment.track_id,
                segment.segment_index,
                segment.storage_key,
                first_frame,
                frame_count,
                packet_count,
                encrypted_bytes,
                segment.envelope_sha256,
                segment.envelope_version,
                segment.key_version,
                segment.state,
                segment.created_at,
                segment.deletion_requested_at,
            ],
        )?;
        let changed = transaction.execute(
            "UPDATE audio_tracks SET
                packet_count = packet_count + ?1,
                frame_count = frame_count + ?2,
                last_packet_sequence = ?3
             WHERE id = ?4
               AND (last_packet_sequence IS NULL OR last_packet_sequence < ?3)",
            params![
                packet_count,
                frame_count,
                last_packet_sequence,
                segment.track_id,
            ],
        )?;
        if changed != 1 {
            return Err(StorageError::InvalidRecord);
        }
        transaction.execute(
            "UPDATE audio_recording_sessions
             SET total_media_bytes = total_media_bytes + ?1
             WHERE id = (SELECT session_id FROM audio_tracks WHERE id = ?2)",
            params![encrypted_bytes, segment.track_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn save_audio_capture_event_record(
        &self,
        event: &AudioCaptureEventRecord,
    ) -> Result<(), StorageError> {
        validate_machine_id(&event.session_id, 128)?;
        if let Some(track_id) = &event.track_id {
            validate_machine_id(track_id, 128)?;
        }
        validate_machine_id(&event.event_kind, 64)?;
        validate_machine_id(&event.code, 96)?;
        validate_event_measurements(event)?;
        let provider_monotonic_ns = stored_i64(event.provider_monotonic_ns)?;
        let affected_frames = event.affected_frames.map(stored_i64).transpose()?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection.execute(
            "INSERT INTO audio_capture_events
                (session_id, track_id, provider_monotonic_ns, event_kind, code,
                 affected_frames, drift_parts_per_million, observed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                event.session_id,
                event.track_id,
                provider_monotonic_ns,
                event.event_kind,
                event.code,
                affected_frames,
                event.drift_parts_per_million,
                event.observed_at,
            ],
        )?;
        Ok(())
    }

    pub fn finish_audio_session_record(
        &self,
        session_id: &str,
        ended_at: &str,
        status: &str,
    ) -> Result<(), StorageError> {
        if !matches!(status, "completed" | "interrupted") {
            return Err(StorageError::InvalidRecord);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let changed = connection.execute(
            "UPDATE audio_recording_sessions
             SET ended_at = ?1, status = ?2
             WHERE id = ?3 AND status = 'active'",
            params![ended_at, status, session_id],
        )?;
        if changed != 1 {
            return Err(StorageError::InvalidRecord);
        }
        Ok(())
    }

    pub fn list_audio_session_records(&self) -> Result<Vec<AudioSessionRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT id, simulator_session_id, provider_id, capture_mode, started_at,
                    ended_at, host_start_monotonic_ns, status, media_availability,
                    total_media_bytes, deletion_requested_at
             FROM audio_recording_sessions ORDER BY started_at DESC, id DESC",
        )?;
        statement
            .query_map([], audio_session_from_row)?
            .map(|row| {
                row.map_err(StorageError::from)
                    .and_then(validate_loaded_audio_session)
            })
            .collect()
    }

    pub fn list_audio_deletion_candidate_records(
        &self,
    ) -> Result<Vec<AudioSessionRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT audio.id, audio.simulator_session_id, audio.provider_id,
                    audio.capture_mode, audio.started_at, audio.ended_at,
                    audio.host_start_monotonic_ns, audio.status,
                    audio.media_availability, audio.total_media_bytes,
                    audio.deletion_requested_at
             FROM audio_recording_sessions audio
             LEFT JOIN simulator_session_metadata simulator
                ON simulator.session_id = audio.simulator_session_id
             WHERE audio.status <> 'active'
               AND COALESCE(simulator.pinned, 0) = 0
             ORDER BY audio.started_at ASC, audio.id ASC",
        )?;
        statement
            .query_map([], audio_session_from_row)?
            .map(|row| {
                row.map_err(StorageError::from)
                    .and_then(validate_loaded_audio_session)
            })
            .collect()
    }

    pub fn list_audio_track_records(
        &self,
        session_id: &str,
    ) -> Result<Vec<AudioTrackRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT id, session_id, source_id, profile_id, codec_provider_id,
                    codec_provider_version, codec_id, codec_media_type, source_role, source_truth,
                    channel_count, sample_rate_hz, provider_start_monotonic_ns,
                    packet_count, frame_count, last_packet_sequence
             FROM audio_tracks WHERE session_id = ?1 ORDER BY id",
        )?;
        statement
            .query_map([session_id], audio_track_from_row)?
            .map(|row| {
                row.map_err(StorageError::from)
                    .and_then(validate_loaded_audio_track)
            })
            .collect()
    }

    pub fn list_audio_segment_records(
        &self,
        track_id: &str,
    ) -> Result<Vec<AudioSegmentRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT track_id, segment_index, storage_key, first_frame, frame_count,
                    packet_count, encrypted_bytes, envelope_sha256, envelope_version,
                    key_version, state, created_at, deletion_requested_at
             FROM audio_track_segments WHERE track_id = ?1 ORDER BY segment_index",
        )?;
        statement
            .query_map([track_id], audio_segment_from_row)?
            .map(|row| {
                row.map_err(StorageError::from)
                    .and_then(validate_loaded_audio_segment)
            })
            .collect()
    }

    pub fn mark_audio_session_tombstoned(
        &self,
        session_id: &str,
        requested_at: &str,
    ) -> Result<(), StorageError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "UPDATE audio_recording_sessions
             SET media_availability = 'tombstoned', deletion_requested_at = ?1
             WHERE id = ?2 AND status <> 'active'",
            params![requested_at, session_id],
        )?;
        transaction.execute(
            "UPDATE audio_track_segments
             SET state = 'tombstoned', deletion_requested_at = ?1
             WHERE track_id IN (SELECT id FROM audio_tracks WHERE session_id = ?2)",
            params![requested_at, session_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn delete_audio_session_metadata(&self, session_id: &str) -> Result<bool, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        Ok(connection.execute(
            "DELETE FROM audio_recording_sessions WHERE id = ?1 AND status <> 'active'",
            [session_id],
        )? > 0)
    }
}

fn audio_preferences_from_values(
    values: (i64, i64, i64, i64, i64),
) -> Result<AudioRecordingPreferencesRecord, StorageError> {
    let record = AudioRecordingPreferencesRecord {
        enabled: stored_bool(values.0)?,
        capture_manual: stored_bool(values.1)?,
        capture_automatic: stored_bool(values.2)?,
        retention_days: u32::try_from(values.3).map_err(|_| StorageError::InvalidRecord)?,
        storage_budget_bytes: u64::try_from(values.4).map_err(|_| StorageError::InvalidRecord)?,
    };
    validate_audio_preferences(&record)?;
    Ok(record)
}

fn audio_selection_from_values(
    values: (String, String, String, String, i64, i64, i64, i64),
) -> Result<AudioSourceSelectionRecord, StorageError> {
    let record = AudioSourceSelectionRecord {
        provider_id: values.0,
        source_id: values.1,
        profile_id: values.2,
        codec_provider_id: values.3,
        enabled: stored_bool(values.4)?,
        playback_muted: stored_bool(values.5)?,
        playback_solo: stored_bool(values.6)?,
        playback_volume_percent: u16::try_from(values.7)
            .map_err(|_| StorageError::InvalidRecord)?,
    };
    validate_machine_id(&record.provider_id, 255)?;
    validate_machine_id(&record.source_id, 128)?;
    validate_profile_id(&record.profile_id)?;
    validate_machine_id(&record.codec_provider_id, 255)?;
    if record.playback_volume_percent > 200 {
        return Err(StorageError::InvalidRecord);
    }
    Ok(record)
}

fn audio_session_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AudioSessionRecord> {
    Ok(AudioSessionRecord {
        id: row.get(0)?,
        simulator_session_id: row.get(1)?,
        provider_id: row.get(2)?,
        capture_mode: row.get(3)?,
        started_at: row.get(4)?,
        ended_at: row.get(5)?,
        host_start_monotonic_ns: row_optional_u64(row, 6)?,
        status: row.get(7)?,
        media_availability: row.get(8)?,
        total_media_bytes: row_u64(row, 9)?,
        deletion_requested_at: row.get(10)?,
    })
}

fn audio_track_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AudioTrackRecord> {
    Ok(AudioTrackRecord {
        id: row.get(0)?,
        session_id: row.get(1)?,
        source_id: row.get(2)?,
        profile_id: row.get(3)?,
        codec_provider_id: row.get(4)?,
        codec_provider_version: row.get(5)?,
        codec_id: row.get(6)?,
        codec_media_type: row.get(7)?,
        source_role: row.get(8)?,
        source_truth: row.get(9)?,
        channel_count: row.get(10)?,
        sample_rate_hz: row.get(11)?,
        provider_start_monotonic_ns: row_u64(row, 12)?,
        packet_count: row_u64(row, 13)?,
        frame_count: row_u64(row, 14)?,
        last_packet_sequence: row_optional_u64(row, 15)?,
    })
}

fn audio_segment_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AudioSegmentRecord> {
    Ok(AudioSegmentRecord {
        track_id: row.get(0)?,
        segment_index: row.get(1)?,
        storage_key: row.get(2)?,
        first_frame: row_u64(row, 3)?,
        frame_count: row_u64(row, 4)?,
        packet_count: row_u64(row, 5)?,
        encrypted_bytes: row_u64(row, 6)?,
        envelope_sha256: row.get(7)?,
        envelope_version: row.get(8)?,
        key_version: row.get(9)?,
        state: row.get(10)?,
        created_at: row.get(11)?,
        deletion_requested_at: row.get(12)?,
    })
}

fn validate_loaded_audio_session(
    record: AudioSessionRecord,
) -> Result<AudioSessionRecord, StorageError> {
    validate_audio_session(&record)?;
    Ok(record)
}

fn validate_loaded_audio_track(record: AudioTrackRecord) -> Result<AudioTrackRecord, StorageError> {
    let session_id = record.session_id.clone();
    validate_audio_track(&record, &session_id)?;
    Ok(record)
}

fn validate_loaded_audio_segment(
    record: AudioSegmentRecord,
) -> Result<AudioSegmentRecord, StorageError> {
    validate_audio_segment(&record)?;
    Ok(record)
}

fn validate_audio_preferences(
    record: &AudioRecordingPreferencesRecord,
) -> Result<(), StorageError> {
    if !(1..=3650).contains(&record.retention_days)
        || !(16 * 1024 * 1024..=1024 * 1024 * 1024 * 1024).contains(&record.storage_budget_bytes)
    {
        return Err(StorageError::InvalidRecord);
    }
    Ok(())
}

fn validate_audio_session(record: &AudioSessionRecord) -> Result<(), StorageError> {
    validate_machine_id(&record.id, 128)?;
    validate_machine_id(&record.provider_id, 255)?;
    if !matches!(record.capture_mode.as_str(), "manual" | "automatic")
        || !matches!(
            record.status.as_str(),
            "active" | "completed" | "interrupted"
        )
        || !matches!(
            record.media_availability.as_str(),
            "available" | "not_in_backup" | "missing" | "tombstoned"
        )
    {
        return Err(StorageError::InvalidRecord);
    }
    Ok(())
}

fn validate_audio_track(record: &AudioTrackRecord, session_id: &str) -> Result<(), StorageError> {
    validate_machine_id(&record.id, 128)?;
    validate_machine_id(&record.source_id, 128)?;
    validate_profile_id(&record.profile_id)?;
    validate_machine_id(&record.codec_provider_id, 255)?;
    validate_machine_id(&record.codec_provider_version, 64)?;
    validate_machine_id(&record.codec_id, 96)?;
    validate_media_type(&record.codec_media_type)?;
    if record.session_id != session_id
        || !(1..=8).contains(&record.channel_count)
        || record.sample_rate_hz != 48_000
        || !matches!(
            record.source_role.as_str(),
            "microphone_input"
                | "application_output"
                | "output_endpoint"
                | "simulator_master_mix"
                | "isolated_com1"
                | "isolated_com2"
                | "pilot_radio"
                | "copilot_radio"
        )
        || !matches!(
            record.source_truth.as_str(),
            "isolated" | "mixed_output" | "metadata_only"
        )
        || record.last_packet_sequence == Some(0)
    {
        return Err(StorageError::InvalidRecord);
    }
    Ok(())
}

fn validate_audio_segment(record: &AudioSegmentRecord) -> Result<(), StorageError> {
    validate_machine_id(&record.track_id, 128)?;
    if !valid_lower_hex(&record.storage_key, 32)
        || !valid_lower_hex(&record.envelope_sha256, 64)
        || record.packet_count == 0
        || record.encrypted_bytes == 0
        || record.envelope_version == 0
        || record.key_version == 0
        || !matches!(
            record.state.as_str(),
            "pending" | "complete" | "unavailable" | "tombstoned"
        )
    {
        return Err(StorageError::InvalidRecord);
    }
    Ok(())
}

fn validate_event_measurements(record: &AudioCaptureEventRecord) -> Result<(), StorageError> {
    let valid = match record.event_kind.as_str() {
        "gap" | "dropout" | "backpressure" => {
            record
                .affected_frames
                .is_some_and(|frames| (1..=MAX_AUDIO_EVENT_FRAMES).contains(&frames))
                && record.drift_parts_per_million.is_none()
        }
        "drift" => {
            record.affected_frames.is_none()
                && record.drift_parts_per_million.is_some_and(|drift| {
                    (-MAX_ABSOLUTE_AUDIO_DRIFT_PPM..=MAX_ABSOLUTE_AUDIO_DRIFT_PPM).contains(&drift)
                })
        }
        "permission_required"
        | "permission_denied"
        | "source_unavailable"
        | "source_changed"
        | "encoder_failure" => {
            record.affected_frames.is_none() && record.drift_parts_per_million.is_none()
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(StorageError::InvalidRecord)
    }
}

fn validate_profile_id(value: &str) -> Result<(), StorageError> {
    if matches!(
        value,
        "pilot_microphone_v1" | "isolated_voice_v1" | "mixed_stereo_v1"
    ) {
        Ok(())
    } else {
        Err(StorageError::InvalidRecord)
    }
}

fn validate_machine_id(value: &str, maximum: usize) -> Result<(), StorageError> {
    if !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        Ok(())
    } else {
        Err(StorageError::InvalidRecord)
    }
}

fn validate_media_type(value: &str) -> Result<(), StorageError> {
    let valid_token = |token: &str| {
        !token.is_empty()
            && token.bytes().all(|byte| {
                byte.is_ascii_alphanumeric()
                    || matches!(
                        byte,
                        b'!' | b'#' | b'$' | b'&' | b'^' | b'_' | b'.' | b'+' | b'-'
                    )
            })
    };
    if value.len() <= 120
        && value
            .split_once('/')
            .is_some_and(|(kind, subtype)| valid_token(kind) && valid_token(subtype))
    {
        Ok(())
    } else {
        Err(StorageError::InvalidRecord)
    }
}

fn valid_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn stored_bool(value: i64) -> Result<bool, StorageError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(StorageError::InvalidRecord),
    }
}

fn stored_i64(value: u64) -> Result<i64, StorageError> {
    i64::try_from(value).map_err(|_| StorageError::InvalidRecord)
}

fn row_u64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value = row.get::<_, i64>(index)?;
    u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, value))
}

fn row_optional_u64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<Option<u64>> {
    row.get::<_, Option<i64>>(index)?
        .map(|value| {
            u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, value))
        })
        .transpose()
}
