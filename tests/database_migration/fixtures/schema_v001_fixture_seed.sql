-- Screenshaver Schema-1 permanent historical fixture data.
--
-- Apply only after assets/database/schema_v001.sql has created an empty
-- Schema-1 database. This data is deliberately synthetic and contains no
-- private user paths or irreplaceable shader source.

PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

-- Minimal developer catalogs. Historical readers do not migrate these rows;
-- they exist so the fixture still resembles a normally initialized database.
INSERT INTO textures(texture_name, display_order) VALUES
    ('marble', 0),
    ('noise', 1),
    ('radial', 2);

INSERT INTO curated_palette(color_hex, description) VALUES
    ('#112233', 'Fixture midnight'),
    ('#44aa88', 'Fixture green'),
    ('#cc8844', 'Fixture amber');

-- Three synthetic physical shaders. No real files are required by the
-- historical reader: source_path is durable registration data, while runtime
-- shader packages are derived/regenerable and intentionally mixed here.
INSERT INTO shaders(
    shader_id, shader_added_at, filename, source_path, shader_type,
    source_hash, file_status, validation_status, validation_reason,
    validation_message, preprocessed_source, preprocessor_version,
    channel_usage_mask, shader_inputs_json
) VALUES
    (1, '2026-09-26T12:00:00Z', 'fixture_alpha.glsl', '/fixture/shaders',
     'native', 'fixture-hash-alpha', 'present', 'valid', NULL, NULL,
     CAST('fixture runtime alpha' AS BLOB), 1, 0, '[]'),
    (2, '2026-09-26T12:01:00Z', 'fixture_beta.glsl', '/fixture/shaders',
     'shadertoy', 'fixture-hash-beta', 'present', 'valid', NULL, NULL,
     NULL, NULL, NULL, NULL),
    (3, '2026-09-26T12:02:00Z', 'fixture_gamma.fs', '/fixture/shaders',
     'isf', 'fixture-hash-gamma', 'missing', 'rejected', 'fixture_rejected',
     'Synthetic rejected shader used only by migration tests.',
     NULL, NULL, NULL, NULL);

-- Six policies cover every target plus inheritance/random/specific policy
-- semantics. Alpha deliberately has multiple policies so reconstruction must
-- preserve many-policies-to-one-shader relationships.
INSERT INTO shader_policies(
    policy_id, policy_created_at, policy_modified_at,
    policy_name, policy_name_key, shader_id, policy_target,
    texture_mode, texture_family, texture_primitives,
    palette_mode, palette_color,
    rendered_fps, animation_speed, starting_offset,
    anti_aliasing, dithering, color_precision, render_scale,
    audiovisual_effect, audio_motion_effect,
    bloom_intensity, bloom_saturation, bloom_threshold,
    bloom_frequency_rotation, bloom_frequency_invert,
    invert_colors, flip_horizontal, flip_vertical, hue_rotation
) VALUES
    -- Fully inherited operational/texture/palette settings.
    (1, '2026-09-26T13:00:00Z', '2026-09-26T13:00:00Z',
     'Fixture Alpha Screensaver', 'fixture alpha screensaver', 1, 'screensaver',
     NULL, NULL, NULL, NULL, NULL,
     NULL, NULL, 0.0, NULL, NULL, NULL, NULL,
     'off', 'off', 1.0, 1.0, 0.80, 0.0, 0,
     0, 0, 0, 0.0),

    -- Same physical shader, explicit texture/palette and Audio Bloom/Motion.
    (2, '2026-09-26T13:01:00Z', '2026-09-26T14:01:00Z',
     'Fixture Alpha Wallpaper', 'fixture alpha wallpaper', 1, 'wallpaper',
     'specific', 'marble', 48, 'specific', '#112233',
     60, 1.25, 12.5, 'fxaa', 'subtle', 'high', 0.75,
     'audio_bloom', 'woofer_from_hell', 1.35, 1.20, 0.72, 15.0, 1,
     1, 0, 1, 45.0),

    -- Random texture/palette and Spectral Bloom.
    (3, '2026-09-26T13:02:00Z', '2026-09-26T13:02:00Z',
     'Fixture Beta Screensaver', 'fixture beta screensaver', 2, 'screensaver',
     'random', NULL, NULL, 'random', NULL,
     30, 0.80, 3.0, 'off', 'off', 'standard', 1.25,
     'spectral_bloom', 'fft_mirror_warp', 0.90, 1.50, 0.65, 270.0, 0,
     0, 1, 0, 120.0),

    -- Specific settings and Loudness Bloom.
    (4, '2026-09-26T13:03:00Z', '2026-09-26T13:30:00Z',
     'Fixture Beta Wallpaper', 'fixture beta wallpaper', 2, 'wallpaper',
     'specific', 'noise', 96, 'specific', '#44aa88',
     45, 1.50, 22.0, 'fxaa', 'subtle', 'auto', 1.50,
     'loudness_bloom', 'polar_propeller', 1.75, 2.00, 0.55, 360.0, 1,
     0, 1, 1, 300.0),

    -- Unassigned is durable and must survive reconstruction.
    (5, '2026-09-26T13:04:00Z', '2026-09-26T13:04:00Z',
     'Fixture Gamma Unassigned', 'fixture gamma unassigned', 3, 'unassigned',
     NULL, NULL, NULL, 'specific', '#cc8844',
     NULL, 2.0, 1.0, NULL, NULL, NULL, NULL,
     'off', 'fft_mirror_warp', 1.10, 1.10, 0.85, 90.0, 0,
     1, 1, 0, 15.0),

    -- A third policy on Alpha proves a shader can participate in several
    -- semantically different policies and playlists.
    (6, '2026-09-26T13:05:00Z', '2026-09-26T13:05:00Z',
     'Fixture Alpha Alternate', 'fixture alpha alternate', 1, 'screensaver',
     'specific', 'radial', 32, 'random', NULL,
     24, 0.50, 7.25, 'off', 'subtle', 'standard', 0.50,
     'audio_bloom', 'off', 0.75, 1.30, 0.90, 180.0, 0,
     0, 0, 1, 210.0);

INSERT INTO playlists(
    playlist_id, playlist_created_at, playlist_modified_at,
    playlist_name, playlist_name_key, description
) VALUES
    (1, '2026-09-26T15:00:00Z', '2026-09-26T15:10:00Z',
     'Fixture Mixed Rotation', 'fixture mixed rotation',
     'Synthetic mixed-target playlist used by Schema-1 migration tests.'),
    (2, '2026-09-26T15:01:00Z', '2026-09-26T15:11:00Z',
     'Fixture Secondary Rotation', 'fixture secondary rotation',
     NULL);

-- Positions are deliberately meaningful and contiguous; the reader converts
-- these relational rows into ordered MigrationPolicyId vectors.
INSERT INTO playlist_members(playlist_id, policy_id, position) VALUES
    (1, 2, 1),
    (1, 1, 2),
    (1, 5, 3),
    (1, 4, 4),
    (2, 6, 1),
    (2, 3, 2),
    (2, 2, 3);

-- One database can represent only one mode per target at a time. The fixture
-- chooses Single + Playlist because they exercise foreign-key references;
-- Ordered was already exercised by the real 0.5.4 reader diagnostic.
INSERT INTO runtime_targets(
    target, display_mode, interval_seconds, single_policy_id, playlist_id
) VALUES
    ('screensaver', 'single', NULL, 6, NULL),
    ('wallpaper', 'playlist', 37, NULL, 1);

-- Deliberately non-default values prove migration preserves user choices rather
-- than merely recreating current application defaults.
INSERT INTO app_defaults(
    defaults_id, show_splash, screensaver_subtitles, subtitle_placement,
    wallpaper_notifications, lyrics_enabled, wallpaper_display_format,
    rendered_fps, anti_aliasing, dithering, color_precision, render_scale,
    automatic_backups, backup_interval_days, last_backup
) VALUES
    (1, 0, 0, 'top:left', 0, 1, 'windowed',
     72, 'off', 'off', 'high', 1.50,
     0, 13, '2026-09-20T09:30:00Z');

INSERT INTO target_defaults(
    target, idle_timeout_value, idle_timeout_unit, animation_speed,
    texture_mode, texture_family, texture_primitives,
    palette_mode, palette_color
) VALUES
    ('screensaver', 7, 'minutes', 1.40,
     'specific', 'marble', 80, 'specific', '#112233'),
    ('wallpaper', NULL, NULL, 0.65,
     'random', NULL, 144, 'specific', '#44aa88');

-- Initialization inserts schema_metadata last. Keep that behavior in the
-- fixture so its provenance is unambiguous.
INSERT INTO schema_metadata(
    metadata_id, schema_version, created_by_version, last_migrated_by_version
) VALUES
    (1, 1, '0.5.4', '0.5.4');

COMMIT;
