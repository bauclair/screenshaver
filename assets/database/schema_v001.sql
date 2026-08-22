-- Screenshaver SQLite schema
-- Schema Version: 1
--
-- Design goals:
--   * Physical shaders and shader policies are separate objects.
--   * One physical shader may be referenced by many policies.
--   * Internal IDs are immutable and never intentionally reused.
--   * Policy Name is the user-facing identifier.
--   * Policy configuration is decomposed into typed semantic fields.
--   * NULL means "inherit the applicable global value" only where documented.
--   * Physical shader files remain authoritative source assets.
--   * Derived runtime data is regenerable from those physical source assets.
--
-- IMPORTANT:
-- This file creates the database structure only.
-- Runtime-derived records such as default.glsl registration, source hashes,
-- runtime shader packages, and the final schema_metadata row are inserted by
-- Rust initialization code after the required runtime values are known.
--
-- Likewise, curated_palette rows should be seeded from the developer-maintained
-- palette catalog, not invented here.

PRAGMA journal_mode = DELETE;
PRAGMA synchronous = FULL;
PRAGMA foreign_keys = ON;

-- Must be established on a new database before tables are created so that
-- incremental vacuuming is available for transparent future maintenance.
PRAGMA auto_vacuum = INCREMENTAL;

BEGIN IMMEDIATE;

-- ---------------------------------------------------------------------------
-- schema_metadata
-- ---------------------------------------------------------------------------
-- Exactly one row is permitted: metadata_id = 1.
-- The row is intentionally inserted by runtime initialization AFTER the rest
-- of Schema Version 1 and required seed data have been created successfully.
-- Application version and schema version are intentionally independent.
CREATE TABLE schema_metadata (
    metadata_id              INTEGER NOT NULL
                             PRIMARY KEY
                             CHECK (metadata_id = 1),

    schema_version           INTEGER NOT NULL
                             CHECK (schema_version >= 1),

    created_by_version       TEXT NOT NULL,

    last_migrated_by_version TEXT NOT NULL
);

-- ---------------------------------------------------------------------------
-- shaders
-- ---------------------------------------------------------------------------
-- One row represents one logical physical shader asset.
CREATE TABLE shaders (
    shader_id              INTEGER NOT NULL
                           PRIMARY KEY AUTOINCREMENT,

    -- Complete Linux filename, including extension.
    -- BINARY collation preserves case-sensitive filesystem semantics.
    filename               TEXT NOT NULL
                           COLLATE BINARY
                           CHECK (length(filename) > 0),

    -- Directory containing the source file.
    -- Retained as first-class physical identity and for diagnostics even though
    -- managed shaders currently reside in ~/.config/screenshaver/shaders.
    source_path            TEXT NOT NULL
                           COLLATE BINARY
                           CHECK (length(source_path) > 0),

    -- Extensible semantic type, validated by Screenshaver rather than by an
    -- SQL enum-like CHECK so future types do not force a schema redesign.
    shader_type            TEXT NOT NULL
                           CHECK (length(shader_type) > 0),

    -- Fingerprint of the last known physical source contents.
    source_hash            TEXT
                           CHECK (
                               source_hash IS NULL
                               OR length(source_hash) > 0
                           ),

    -- Filesystem condition only; renderability is tracked separately.
    file_status            TEXT NOT NULL
                           DEFAULT 'present'
                           CHECK (
                               file_status IN (
                                   'present',
                                   'missing',
                                   'unreadable'
                               )
                           ),

    -- Rendering usability of the physical shader.
    validation_status      TEXT NOT NULL
                           DEFAULT 'unknown'
                           CHECK (
                               validation_status IN (
                                   'unknown',
                                   'valid',
                                   'rejected'
                               )
                           ),

    -- Stable machine-readable rejection/validation category.
    validation_reason      TEXT
                           CHECK (
                               validation_reason IS NULL
                               OR (
                                   length(trim(validation_reason)) > 0
                                   AND validation_reason = lower(validation_reason)
                               )
                           ),

    -- Concise user-facing reason. Detailed diagnostics belong in
    -- screenshaver.log.
    validation_message     TEXT,

    -- Runtime GLSL consumed by the OpenGL compiler. Native GLSL is copied here
    -- unchanged; ShaderToy and ISF source is stored here after preprocessing.
    preprocessed_source    BLOB,

    -- Version of the runtime-source preparation contract that produced the
    -- runtime package.
    preprocessor_version   INTEGER
                           CHECK (
                               preprocessor_version IS NULL
                               OR preprocessor_version >= 1
                           ),

    -- Runtime channel-usage bit mask.
    -- Bit 0 = iChannel0, bit 1 = iChannel1, bit 2 = iChannel2, bit 3 = iChannel3.
    -- Bit 4 = channel sampling requires mipmaps.
    -- This preserves the complete ShaderChannelUsage runtime state in one
    -- compact integer. Valid values therefore range from 0 through 31.
    channel_usage_mask     INTEGER
                           CHECK (
                               channel_usage_mask IS NULL
                               OR channel_usage_mask BETWEEN 0 AND 31
                           ),

    -- Serialized runtime ShaderInput list.
    -- Rust owns the serialization contract. Non-ISF shaders store an empty
    -- serialized list rather than NULL when a complete runtime package exists.
    -- NULL means that no complete runtime package is currently stored.
    shader_inputs_json     TEXT
                           CHECK (
                               shader_inputs_json IS NULL
                               OR length(trim(shader_inputs_json)) > 0
                           ),

    -- The physical pathname must not be registered twice.
    UNIQUE (source_path, filename),

    -- Runtime GLSL, preparation version, channel metadata, and input metadata
    -- form one atomic derived runtime package. Either all four are absent or
    -- all four are present.
    CHECK (
        (
            preprocessed_source IS NULL
            AND preprocessor_version IS NULL
            AND channel_usage_mask IS NULL
            AND shader_inputs_json IS NULL
        )
        OR
        (
            preprocessed_source IS NOT NULL
            AND preprocessor_version IS NOT NULL
            AND channel_usage_mask IS NOT NULL
            AND shader_inputs_json IS NOT NULL
        )
    )
);

-- ---------------------------------------------------------------------------
-- shader_policies
-- ---------------------------------------------------------------------------
-- One row represents one named rendition/configuration of a physical shader.
CREATE TABLE shader_policies (
    policy_id              INTEGER NOT NULL
                           PRIMARY KEY AUTOINCREMENT,

    -- User-visible Policy Name. The displayed spelling/capitalization is
    -- preserved exactly (after Rust trims leading/trailing whitespace).
    policy_name            TEXT NOT NULL
                           CHECK (
                               length(trim(policy_name)) BETWEEN 1 AND 128
                           ),

    -- Unicode-normalized, case-folded comparison key generated by Rust.
    --
    -- SQLite's built-in NOCASE collation is ASCII-only, so Rust keeps a
    -- canonical normalized/case-folded key for case-insensitive search, sort,
    -- and comparison. This field is NOT policy identity and is intentionally
    -- non-unique.
    --
    -- Rust must derive policy_name_key from policy_name using one canonical
    -- normalization/case-folding algorithm before every insert/update.
    policy_name_key        TEXT NOT NULL
                           CHECK (length(policy_name_key) > 0),

    shader_id              INTEGER NOT NULL,

    policy_target          TEXT NOT NULL
                           DEFAULT 'unassigned'
                           CHECK (
                               policy_target IN (
                                   'unassigned',
                                   'screensaver',
                                   'wallpaper'
                               )
                           ),

    -- Texture selection:
    --   NULL      = inherit applicable global texture behavior
    --   specific  = use texture_family + texture_primitives
    --   random    = choose texture family at render time; primitive count
    --               comes from the applicable global default
    texture_mode           TEXT
                           CHECK (
                               texture_mode IS NULL
                               OR texture_mode IN ('specific', 'random')
                           ),

    texture_family         TEXT,

    texture_primitives     INTEGER
                           CHECK (
                               texture_primitives IS NULL
                               OR texture_primitives > 0
                           ),

    -- Palette selection:
    --   NULL      = inherit applicable global palette behavior
    --   specific  = use palette_color
    --   random    = choose palette at render time
    palette_mode           TEXT
                           CHECK (
                               palette_mode IS NULL
                               OR palette_mode IN ('specific', 'random')
                           ),

    -- Canonical lowercase #rrggbb when palette_mode = 'specific'.
    palette_color          TEXT,

    -- Operational / rendering-quality overrides.
    -- NULL means inherit from app_defaults and/or the applicable target_defaults row.
    rendered_fps           INTEGER
                           CHECK (
                               rendered_fps IS NULL
                               OR rendered_fps > 0
                           ),

    animation_speed        REAL,

    anti_aliasing          TEXT,

    dithering              TEXT,

    color_precision        TEXT,

    render_scale           REAL
                           CHECK (
                               render_scale IS NULL
                               OR render_scale > 0.0
                           ),

    -- Explicit per-policy visual effects. These do NOT inherit from global
    -- configuration.
    bloom_mode             TEXT NOT NULL
                           DEFAULT 'off'
                           CHECK (
                               bloom_mode IN (
                                   'off',
                                   'highlight',
                                   'audio'
                               )
                           ),

    bloom_intensity        REAL NOT NULL
                           DEFAULT 1.0,

    bloom_threshold        REAL NOT NULL
                           DEFAULT 0.80,

    invert_colors          INTEGER NOT NULL
                           DEFAULT 0
                           CHECK (invert_colors IN (0, 1)),

    flip_horizontal        INTEGER NOT NULL
                           DEFAULT 0
                           CHECK (flip_horizontal IN (0, 1)),

    flip_vertical          INTEGER NOT NULL
                           DEFAULT 0
                           CHECK (flip_vertical IN (0, 1)),

    hue_rotation           REAL NOT NULL
                           DEFAULT 0.0,

    FOREIGN KEY (shader_id)
        REFERENCES shaders(shader_id)
        ON DELETE CASCADE,

    -- texture_mode semantics.
    CHECK (
        (
            texture_mode IS NULL
            AND texture_family IS NULL
            AND texture_primitives IS NULL
        )
        OR
        (
            texture_mode = 'random'
            AND texture_family IS NULL
            AND texture_primitives IS NULL
        )
        OR
        (
            texture_mode = 'specific'
            AND texture_family IS NOT NULL
            AND length(trim(texture_family)) > 0
            AND texture_primitives IS NOT NULL
            AND texture_primitives > 0
        )
    ),

    -- palette_mode semantics and canonical lowercase #rrggbb storage.
    CHECK (
        (
            palette_mode IS NULL
            AND palette_color IS NULL
        )
        OR
        (
            palette_mode = 'random'
            AND palette_color IS NULL
        )
        OR
        (
            palette_mode = 'specific'
            AND palette_color IS NOT NULL
            AND length(palette_color) = 7
            AND substr(palette_color, 1, 1) = '#'
            AND palette_color = lower(palette_color)
            AND substr(palette_color, 2) NOT GLOB '*[^0-9a-f]*'
        )
    )
);

-- Frequent relational lookup:
--   "Which policies reference this physical shader?"
CREATE INDEX idx_shader_policies_shader_id
    ON shader_policies(shader_id);

-- Frequent operational / Policy List filter:
--   screensaver | wallpaper | unassigned
CREATE INDEX idx_shader_policies_target
    ON shader_policies(policy_target);


-- Frequent Policy List search/sort by normalized Policy Name and target.
-- This index is intentionally non-unique: policy_id is the sole policy identity.
CREATE INDEX idx_shader_policies_name_target
    ON shader_policies(
        policy_name_key,
        policy_target
    );

-- ---------------------------------------------------------------------------
-- runtime_targets
-- ---------------------------------------------------------------------------
-- Application-level runtime selection state for Screensaver and Wallpaper.
-- Policy definitions remain in shader_policies; this table records only how
-- each runtime target selects among those policies.
CREATE TABLE runtime_targets (
    target                  TEXT NOT NULL
                            PRIMARY KEY
                            CHECK (
                                target IN (
                                    'screensaver',
                                    'wallpaper'
                                )
                            ),

    display_mode            TEXT NOT NULL
                            CHECK (
                                display_mode IN (
                                    'single',
                                    'ordered',
                                    'random'
                                )
                            ),

    -- Rotation interval for ordered/random modes.
    -- NULL when display_mode = 'single'.
    interval_seconds        INTEGER
                            CHECK (
                                interval_seconds IS NULL
                                OR interval_seconds > 0
                            ),

    -- Exact policy selected for Single mode.
    -- NULL for ordered/random modes. ON DELETE SET NULL deliberately leaves
    -- Single mode recoverable if its selected policy is deleted.
    single_policy_id        INTEGER,

    FOREIGN KEY (single_policy_id)
        REFERENCES shader_policies(policy_id)
        ON DELETE SET NULL,

    CHECK (
        (
            display_mode = 'single'
            AND interval_seconds IS NULL
        )
        OR
        (
            display_mode IN ('ordered', 'random')
            AND interval_seconds IS NOT NULL
            AND single_policy_id IS NULL
        )
    )
);


-- ---------------------------------------------------------------------------
-- app_defaults
-- ---------------------------------------------------------------------------
-- Single-row application defaults used by all runtime targets unless a more
-- specific target default or per-policy override applies.
--
-- Settings deliberately retained in screenshaver.toml for startup/recovery
-- purposes (screensaver/wallpaper enable flags, debug_log, log_level,
-- monitor_mode, screen_lock) are NOT stored here.
CREATE TABLE app_defaults (
    defaults_id             INTEGER NOT NULL
                            PRIMARY KEY
                            CHECK (defaults_id = 1),

    show_splash             INTEGER NOT NULL
                            DEFAULT 1
                            CHECK (show_splash IN (0, 1)),

    screensaver_subtitles   INTEGER NOT NULL
                            DEFAULT 1
                            CHECK (screensaver_subtitles IN (0, 1)),

    subtitle_placement      TEXT NOT NULL
                            DEFAULT 'bottom:center'
                            CHECK (
                                subtitle_placement IN (
                                    'top:left',
                                    'top:center',
                                    'top:right',
                                    'bottom:left',
                                    'bottom:center',
                                    'bottom:right'
                                )
                            ),

    wallpaper_notifications INTEGER NOT NULL
                            DEFAULT 1
                            CHECK (wallpaper_notifications IN (0, 1)),

    rendered_fps            INTEGER NOT NULL
                            DEFAULT 30
                            CHECK (
                                rendered_fps BETWEEN 16 AND 120
                            ),

    anti_aliasing           TEXT NOT NULL
                            DEFAULT 'fxaa'
                            CHECK (
                                anti_aliasing IN (
                                    'off',
                                    'fxaa'
                                )
                            ),

    dithering               TEXT NOT NULL
                            DEFAULT 'subtle'
                            CHECK (
                                dithering IN (
                                    'off',
                                    'subtle'
                                )
                            ),

    color_precision         TEXT NOT NULL
                            DEFAULT 'auto'
                            CHECK (
                                color_precision IN (
                                    'auto',
                                    'standard',
                                    'high'
                                )
                            ),

    render_scale            REAL NOT NULL
                            DEFAULT 1.0
                            CHECK (
                                render_scale BETWEEN 0.25 AND 2.0
                            )
);


-- ---------------------------------------------------------------------------
-- target_defaults
-- ---------------------------------------------------------------------------
-- Target-specific inherited defaults. Exactly one screensaver row and one
-- wallpaper row are created during database initialization.
--
-- Primitive count is always concrete. There is intentionally NO random
-- primitive mode. If texture_mode = 'random', only the texture family is
-- randomized; texture_primitives remains the stored target default.
CREATE TABLE target_defaults (
    target                  TEXT NOT NULL
                            PRIMARY KEY
                            CHECK (
                                target IN (
                                    'screensaver',
                                    'wallpaper'
                                )
                            ),

    -- Screensaver-only idle delay. Wallpaper must store NULL.
    idle_timeout_seconds    INTEGER
                            CHECK (
                                idle_timeout_seconds IS NULL
                                OR idle_timeout_seconds > 0
                            ),

    animation_speed         REAL NOT NULL
                            CHECK (
                                animation_speed > 0.0
                            ),

    texture_mode            TEXT NOT NULL
                            DEFAULT 'random'
                            CHECK (
                                texture_mode IN (
                                    'specific',
                                    'random'
                                )
                            ),

    texture_family          TEXT,

    texture_primitives      INTEGER NOT NULL
                            DEFAULT 64
                            CHECK (
                                texture_primitives BETWEEN 1 AND 1024
                            ),

    palette_mode            TEXT NOT NULL
                            DEFAULT 'random'
                            CHECK (
                                palette_mode IN (
                                    'specific',
                                    'random'
                                )
                            ),

    palette_color           TEXT,

    -- Screensaver has an idle timeout; Wallpaper does not.
    CHECK (
        (
            target = 'screensaver'
            AND idle_timeout_seconds IS NOT NULL
        )
        OR
        (
            target = 'wallpaper'
            AND idle_timeout_seconds IS NULL
        )
    ),

    -- Random texture selection randomizes family only.
    CHECK (
        (
            texture_mode = 'random'
            AND texture_family IS NULL
        )
        OR
        (
            texture_mode = 'specific'
            AND texture_family IS NOT NULL
            AND length(trim(texture_family)) > 0
        )
    ),

    -- Palette semantics and canonical lowercase #rrggbb storage.
    CHECK (
        (
            palette_mode = 'random'
            AND palette_color IS NULL
        )
        OR
        (
            palette_mode = 'specific'
            AND palette_color IS NOT NULL
            AND length(palette_color) = 7
            AND substr(palette_color, 1, 1) = '#'
            AND palette_color = lower(palette_color)
            AND substr(palette_color, 2) NOT GLOB '*[^0-9a-f]*'
        )
    )
);


-- ---------------------------------------------------------------------------
-- textures
-- ---------------------------------------------------------------------------
-- Developer-maintained catalog of procedural texture families supported by
-- this Screenshaver build. Runtime initialization seeds these rows directly
-- from crate::generate_textures::TextureFamily::ALL.
CREATE TABLE textures (
    texture_name           TEXT NOT NULL
                           PRIMARY KEY
                           CHECK (
                               length(trim(texture_name)) > 0
                               AND texture_name = lower(texture_name)
                           ),

    display_order          INTEGER NOT NULL
                           UNIQUE
                           CHECK (display_order >= 0)
);


-- ---------------------------------------------------------------------------
-- curated_palette
-- ---------------------------------------------------------------------------
-- Developer-maintained reference catalog only.
-- Policies copy the selected hexadecimal color; they do not reference
-- palette_id, so future catalog changes cannot alter existing policies.
CREATE TABLE curated_palette (
    color_hex              TEXT NOT NULL
                           PRIMARY KEY
                           CHECK (
                               length(color_hex) = 7
                               AND substr(color_hex, 1, 1) = '#'
                               AND color_hex = lower(color_hex)
                               AND substr(color_hex, 2) NOT GLOB '*[^0-9a-f]*'
                           ),

    description            TEXT NOT NULL
                           CHECK (length(trim(description)) > 0)
);

COMMIT;

-- ---------------------------------------------------------------------------
-- Runtime initialization responsibilities (NOT static schema SQL)
-- ---------------------------------------------------------------------------
--
-- After the transaction above succeeds, initialization code should:
--
--   1. Seed textures from crate::generate_textures::TextureFamily::ALL.
--
--   2. Seed curated_palette from the developer-maintained curated catalog.
--
--   3. Insert the single app_defaults row:
--        defaults_id             = 1
--        show_splash             = 1
--        screensaver_subtitles   = 1
--        subtitle_placement      = 'bottom:center'
--        wallpaper_notifications = 1
--        rendered_fps            = 30
--        anti_aliasing           = 'fxaa'
--        dithering               = 'subtle'
--        color_precision         = 'auto'
--        render_scale            = 1.0
--
--   4. Insert TWO target_defaults rows:
--        screensaver:
--          idle_timeout_seconds = 600
--          animation_speed      = 1.0
--          texture_mode         = 'random'
--          texture_family       = NULL
--          texture_primitives   = 64
--          palette_mode         = 'random'
--          palette_color        = NULL
--
--        wallpaper:
--          idle_timeout_seconds = NULL
--          animation_speed      = 0.03
--          texture_mode         = 'random'
--          texture_family       = NULL
--          texture_primitives   = 64
--          palette_mode         = 'random'
--          palette_color        = NULL
--
--      Primitive count is NEVER randomized. Random texture mode chooses only
--      the texture family and continues using the stored primitive count.
--
--   5. Ensure ~/.config/screenshaver/shaders/default.glsl exists.
--
--   6. Hash, analyze, and validate default.glsl.
--
--   7. Insert one shaders row for default.glsl using its actual runtime:
--        filename
--        source_path
--        shader_type
--        source_hash
--        file_status
--        validation_status
--        validation_reason
--        validation_message
--        preprocessed_source
--        preprocessor_version
--        channel_usage_mask
--        shader_inputs_json
--
--      The final four fields form the complete derived runtime package.
--      Native GLSL is stored unchanged in preprocessed_source.
--
--   8. Create TWO policies referencing the same default.glsl shader_id:
--        screensaver default -> policy_target = 'screensaver'
--        wallpaper default   -> policy_target = 'wallpaper'
--
--      Their inherited operational fields remain NULL.
--      Their explicit visual defaults are:
--        bloom_mode       = 'off'
--        bloom_intensity  = 1.0
--        bloom_threshold  = 0.80
--        invert_colors    = 0
--        flip_horizontal  = 0
--        flip_vertical    = 0
--        hue_rotation     = 0.0
--
--   9. Insert TWO runtime_targets rows using the exact policy_id values
--      created above:
--        screensaver -> display_mode = 'single', single_policy_id = the
--                       screensaver default policy
--        wallpaper   -> display_mode = 'single', single_policy_id = the
--                       wallpaper default policy
--
--   10. Insert schema_metadata LAST:
--        metadata_id              = 1
--        schema_version           = 1
--        created_by_version       = current Screenshaver version
--        last_migrated_by_version = current Screenshaver version
--
--   11. Run initialization validation / foreign-key checks.
--
-- If first-time initialization fails before user data exists, the incomplete
-- database may be discarded and rebuilt on the next database-dependent launch.
