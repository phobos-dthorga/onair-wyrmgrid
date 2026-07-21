ALTER TABLE audio_source_selections
    ADD COLUMN codec_provider_id TEXT NOT NULL DEFAULT 'dev.wyrmgrid.opus'
    CHECK (
        length(codec_provider_id) BETWEEN 3 AND 255
        AND codec_provider_id NOT GLOB '*[^A-Za-z0-9._:-]*'
    );

ALTER TABLE audio_tracks
    ADD COLUMN codec_provider_id TEXT NOT NULL DEFAULT 'dev.wyrmgrid.opus'
    CHECK (
        length(codec_provider_id) BETWEEN 3 AND 255
        AND codec_provider_id NOT GLOB '*[^A-Za-z0-9._:-]*'
    );

ALTER TABLE audio_tracks
    ADD COLUMN codec_provider_version TEXT NOT NULL DEFAULT 'legacy-unversioned'
    CHECK (
        length(codec_provider_version) BETWEEN 1 AND 64
        AND codec_provider_version NOT GLOB '*[^A-Za-z0-9._:-]*'
    );

ALTER TABLE audio_tracks
    ADD COLUMN codec_id TEXT NOT NULL DEFAULT 'opus'
    CHECK (
        length(codec_id) BETWEEN 1 AND 96
        AND codec_id NOT GLOB '*[^A-Za-z0-9._:-]*'
    );

ALTER TABLE audio_tracks
    ADD COLUMN codec_media_type TEXT NOT NULL DEFAULT 'audio/opus'
    CHECK (
        length(codec_media_type) BETWEEN 3 AND 120
        AND instr(codec_media_type, '/') BETWEEN 2 AND length(codec_media_type) - 1
        AND codec_media_type NOT LIKE '% %'
    );

INSERT OR IGNORE INTO schema_migrations (version) VALUES (20);

PRAGMA user_version = 20;
