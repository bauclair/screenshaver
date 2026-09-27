//! Storage-independent semantic representation used during database migration.
//!
//! Historical schema readers translate their database representation into
//! these types.  The current-schema writer consumes these types to construct
//! a new current database.
//!
//! This module deliberately contains no SQLite or rusqlite dependencies.
//! Historical database IDs are used only to establish migration-local object
//! references; they are not destination database identities.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MigrationShaderId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MigrationPolicyId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MigrationPlaylistId(pub u64);


#[derive(Debug, Clone, PartialEq)]
pub struct MigrationData {
    pub source: MigrationSource,

    pub shaders: Vec<MigrationShader>,
    pub policies: Vec<MigrationPolicy>,
    pub playlists: Vec<MigrationPlaylist>,

    pub runtime_configuration: MigrationRuntimeConfiguration,
    pub application_defaults: MigrationApplicationDefaults,
    pub target_defaults: MigrationTargetDefaults,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationSource {
    pub schema_version: i64,
    pub created_by_version: String,
    pub last_migrated_by_version: String,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationShader {
    /// Migration-local identity only.
    pub migration_id: MigrationShaderId,

    /// Displayed physical shader filename.
    pub filename: String,

    /// Source directory/path retained as durable physical-source information.
    pub source_path: String,

    /// Historical addition timestamp when the source schema provides one.
    ///
    /// This is useful metadata, not semantic object identity.
    pub added_at: Option<String>,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationPolicyTarget {
    Unassigned,
    Screensaver,
    Wallpaper,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationTexture {
    Inherit,

    Random,

    Specific {
        family: String,
        primitives: u32,
    },
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationPalette {
    Inherit,

    Random,

    Specific {
        /// Canonical hexadecimal color representation, e.g. "#a1b2c3".
        color: String,
    },
}


#[derive(Debug, Clone, PartialEq)]
pub struct MigrationPolicy {
    /// Migration-local identity only.
    pub migration_id: MigrationPolicyId,

    /// Migration-local reference to the physical shader used by this policy.
    pub shader: MigrationShaderId,

    pub policy_name: String,

    /// Historical timestamps when the source schema provides them.
    ///
    /// They are metadata rather than semantic identity.
    pub created_at: Option<String>,
    pub modified_at: Option<String>,

    pub target: MigrationPolicyTarget,

    pub texture: MigrationTexture,
    pub palette: MigrationPalette,

    /// None means inherit the applicable default.
    pub rendered_fps: Option<u32>,

    /// None means inherit the applicable default.
    pub animation_speed: Option<f64>,

    pub starting_offset_seconds: f64,

    /// None means inherit the applicable default.
    pub anti_aliasing: Option<String>,

    /// None means inherit the applicable default.
    pub dithering: Option<String>,

    /// None means inherit the applicable default.
    pub color_precision: Option<String>,

    /// None means inherit the applicable default.
    pub render_scale: Option<f64>,

    /// Stable internal effect identifier, not a localized display string.
    pub audiovisual_effect: String,

    /// Stable internal effect identifier, not a localized display string.
    pub audio_motion_effect: String,

    pub bloom_intensity: f64,
    pub bloom_saturation: f64,
    pub bloom_threshold: f64,
    pub bloom_frequency_rotation: f64,
    pub bloom_frequency_invert: bool,

    pub invert_colors: bool,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    pub hue_rotation: f64,
}


#[derive(Debug, Clone, PartialEq)]
pub struct MigrationPlaylist {
    /// Migration-local identity only.
    pub migration_id: MigrationPlaylistId,

    pub playlist_name: String,
    pub description: Option<String>,

    /// Historical timestamps when the source schema provides them.
    pub created_at: Option<String>,
    pub modified_at: Option<String>,

    /// Authoritative playlist order.
    ///
    /// A Vec expresses the durable ordering directly, so the canonical
    /// migration representation does not need a relational "position" field.
    pub members: Vec<MigrationPolicyId>,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationDisplayMode {
    Single {
        /// None preserves a recoverable historical Single configuration whose
        /// selected policy was deleted or otherwise absent.
        policy: Option<MigrationPolicyId>,
    },

    Ordered {
        interval_seconds: u64,
    },

    Random {
        interval_seconds: u64,
    },

    Playlist {
        /// None preserves a recoverable historical Playlist configuration
        /// whose selected playlist was deleted or otherwise absent.
        playlist: Option<MigrationPlaylistId>,
        interval_seconds: u64,
    },
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRuntimeConfiguration {
    pub screensaver: MigrationDisplayMode,
    pub wallpaper: MigrationDisplayMode,
}


#[derive(Debug, Clone, PartialEq)]
pub struct MigrationApplicationDefaults {
    pub show_splash: bool,

    pub screensaver_subtitles: bool,
    pub subtitle_placement: String,

    pub wallpaper_notifications: bool,

    pub lyrics_enabled: bool,
    pub wallpaper_display_format: String,

    pub rendered_fps: u32,
    pub anti_aliasing: String,
    pub dithering: String,
    pub color_precision: String,
    pub render_scale: f64,

    pub automatic_backups: bool,
    pub backup_interval_days: u32,

    /// Historical last-backup timestamp/value when present.
    ///
    /// The canonical representation preserves the source value without
    /// imposing a database-specific timestamp encoding.
    pub last_backup: Option<String>,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationIdleTimeout {
    NotApplicable,

    Value {
        value: u64,

        /// Stable internal unit identifier from the historical semantic
        /// contract, e.g. "seconds" or "minutes".
        unit: String,
    },
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationDefaultTexture {
    Random,

    Specific {
        family: String,
        primitives: u32,
    },
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationDefaultPalette {
    Random,

    Specific {
        color: String,
    },
}


#[derive(Debug, Clone, PartialEq)]
pub struct MigrationTargetDefault {
    pub idle_timeout: MigrationIdleTimeout,

    pub animation_speed: f64,

    pub texture: MigrationDefaultTexture,

    /// Primitive count remains independently durable for target defaults.
    ///
    /// Schema 1 stores this value even when texture selection is Random.
    pub texture_primitives: u32,

    pub palette: MigrationDefaultPalette,
}


#[derive(Debug, Clone, PartialEq)]
pub struct MigrationTargetDefaults {
    pub screensaver: MigrationTargetDefault,
    pub wallpaper: MigrationTargetDefault,
}
