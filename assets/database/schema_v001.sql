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
--   * Derived data (hashes, preprocessed source, future thumbnails) is regenerable.
--
-- IMPORTANT:
-- This file creates the database structure only.
-- Runtime-derived records such as default.glsl registration, source hashes,
-- preprocessed BLOBs, and the final schema_metadata row are inserted by Rust
-- initialization code after the required runtime values are known.
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
    -- Retained for compatibility/diagnostics even after consolidation into
    -- the canonical ~/.config/screenshaver/shaders directory.
    source_path            TEXT NOT NULL
                           COLLATE BINARY
                           CHECK (length(source_path) > 0),

    -- Extensible semantic type, validated by Screenshaver rather than by an
    -- SQL enum-like CHECK so future types (for example "shaver") do not force
    -- a fundamental schema redesign.
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

    -- Concise user-facing reason. Detailed diagnostics belong in
    -- screenshaver.log.
    validation_reason      TEXT
                           CHECK (
                               validation_reason IS NULL
                               OR (
                                   length(trim(validation_reason)) > 0
                                   AND validation_reason = lower(validation_reason)
                               )
                           ),

    validation_message     TEXT,

    -- Current derived preprocessed representation.
    preprocessed_source    BLOB,

    -- Version of the preprocessing contract that produced the BLOB.
    preprocessor_version   INTEGER
                           CHECK (
                               preprocessor_version IS NULL
                               OR preprocessor_version >= 1
                           ),

    -- The physical pathname must not be registered twice.
    UNIQUE (source_path, filename),

    -- A preprocessed representation and its version marker travel together.
    CHECK (
        (preprocessed_source IS NULL AND preprocessor_version IS NULL)
        OR
        (preprocessed_source IS NOT NULL AND preprocessor_version IS NOT NULL)
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
    -- SQLite's built-in NOCASE collation is ASCII-only, so it is NOT
    -- sufficient for the agreed requirement that Policy Names support Unicode
    -- while remaining truly case-insensitively unique.
    --
    -- Rust must derive policy_name_key from policy_name using one canonical
    -- normalization/case-folding algorithm before every insert/update.
    policy_name_key        TEXT NOT NULL
                           UNIQUE
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
    -- NULL means inherit the applicable global value from screenshaver.toml.
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
--   1. Seed curated_palette from the developer-maintained curated catalog.
--
--   2. Ensure ~/.config/screenshaver/shaders/default.glsl exists.
--
--   3. Hash, preprocess, and validate default.glsl.
--
--   4. Insert one shaders row for default.glsl using its actual runtime:
--        filename
--        source_path
--        shader_type
--        source_hash
--        file_status
--        validation_status
--        validation_message
--        preprocessed_source
--        preprocessor_version
--
--   5. Create TWO policies referencing the same default.glsl shader_id:
--        Default Screensaver -> policy_target = 'screensaver'
--        Default Wallpaper   -> policy_target = 'wallpaper'
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
--   6. Insert schema_metadata LAST:
--        metadata_id              = 1
--        schema_version           = 1
--        created_by_version       = current Screenshaver version
--        last_migrated_by_version = current Screenshaver version
--
--   7. Run initialization validation / foreign-key checks.
--
-- If first-time initialization fails before user data exists, the incomplete
-- database may be discarded and rebuilt on the next database-dependent launch.
