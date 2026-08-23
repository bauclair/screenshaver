//! Application-level configuration persistence for the Screenshaver Control Center.
//!
//! Runtime display-mode state and inherited rendering defaults are stored in
//! screenshaver.db. The TOML file retains only startup/recovery settings that must
//! remain manually editable even when database-backed configuration is unavailable.

use std::fs;
use std::path::Path;

use toml_edit::{
    value,
    DocumentMut,
    Item,
    Table,
};


//
// ------------------------------------------------------------
// Public configuration structure
// ------------------------------------------------------------
//

#[derive(Debug, Clone)]
pub struct ConfigurationUpdates {

    // Screensaver settings.
    pub screensaver_enabled: bool,

    pub subtitles: bool,

    /// Complete Screenshaver mode string as stored in screenshaver.toml.
    ///
    /// Examples:
    ///     random:60
    ///     ordered:10
    ///     single:<policy_id> (internal runtime representation only)
    pub screensaver_mode:
        String,

    /// Screensaver idle timeout represented as a duration string for the UI.
    /// Persistence authority is target_defaults.idle_timeout_value + idle_timeout_unit.
    pub idle_timeout:
        String,

    /// None means "random".
    pub screensaver_global_texture:
        Option<
            crate::parse_texture_specification::TextureSpecification
        >,

    /// None means "random".
    pub screensaver_global_palette:
        Option<
            crate::palettes::PaletteColor
        >,


    // Wallpaper settings.
    pub wallpaper_enabled: bool,

    pub notifications: bool,

    /// Canonical Wallpaper runtime-mode string backed by runtime_targets.
    pub wallpaper_mode:
        String,

    /// None means "random".
    pub wallpaper_global_texture:
        Option<
            crate::parse_texture_specification::TextureSpecification
        >,

    /// None means "random".
    pub wallpaper_global_palette:
        Option<
            crate::palettes::PaletteColor
        >,
}


impl ConfigurationUpdates {

    /// Build the legacy configuration-save snapshot from the already-loaded
    /// runtime configuration.
    ///
    /// Database-backed application/target defaults are persisted through
    /// AppDefaults and TargetDefaults. This structure remains responsible for
    /// the retained TOML enable flags and runtime-target mode strings used by
    /// save_configuration().
    pub fn from_config(
        config: &crate::load_config::Config,
    ) -> Self {

        Self {
            screensaver_enabled:
                config.screensaver_enabled,

            subtitles:
                config.subtitles,

            screensaver_mode:
                config.mode.clone(),

            idle_timeout:
                config.idle_timeout.clone(),

            screensaver_global_texture:
                config
                    .texture_policy
                    .global_texture
                    .clone(),

            screensaver_global_palette:
                config
                    .texture_policy
                    .global_palette
                    .clone(),

            wallpaper_enabled:
                config.wallpaper_enabled,

            notifications:
                config.wallpaper.notifications,

            wallpaper_mode:
                config.wallpaper_mode.clone(),

            wallpaper_global_texture:
                config
                    .wallpaper_texture_policy
                    .global_texture
                    .clone(),

            wallpaper_global_palette:
                config
                    .wallpaper_texture_policy
                    .global_palette
                    .clone(),
        }
    }
}


//
// ------------------------------------------------------------
// Database-backed application / target defaults
// ------------------------------------------------------------
//

#[derive(Debug, Clone)]
pub struct AppDefaults {
    pub show_splash: bool,
    pub screensaver_subtitles: bool,
    pub subtitle_placement: String,
    pub wallpaper_notifications: bool,
    pub rendered_fps: i64,
    pub anti_aliasing: String,
    pub dithering: String,
    pub color_precision: String,
    pub render_scale: f64,
}


#[derive(Debug, Clone)]
pub struct CuratedPaletteChoice {
    pub color_hex: String,
    pub description: String,
}


#[derive(Debug, Clone)]
pub struct TargetDefaults {
    pub target: String,
    pub idle_timeout_value: Option<i64>,
    pub idle_timeout_unit: Option<String>,
    pub animation_speed: f64,
    pub texture_mode: String,
    pub texture_family: Option<String>,
    pub texture_primitives: i64,
    pub palette_mode: String,
    pub palette_color: Option<String>,
}


pub fn load_app_defaults() -> Result<AppDefaults, String> {
    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while loading application defaults: {}",
                        error,
                    )
                }
            )?;

    connection
        .query_row(
            "SELECT
                 show_splash,
                 screensaver_subtitles,
                 subtitle_placement,
                 wallpaper_notifications,
                 rendered_fps,
                 anti_aliasing,
                 dithering,
                 color_precision,
                 render_scale
             FROM app_defaults
             WHERE defaults_id = 1",
            [],
            |row| {
                Ok(
                    AppDefaults {
                        show_splash:
                            row.get::<_, i64>(0)? != 0,
                        screensaver_subtitles:
                            row.get::<_, i64>(1)? != 0,
                        subtitle_placement:
                            row.get(2)?,
                        wallpaper_notifications:
                            row.get::<_, i64>(3)? != 0,
                        rendered_fps:
                            row.get(4)?,
                        anti_aliasing:
                            row.get(5)?,
                        dithering:
                            row.get(6)?,
                        color_precision:
                            row.get(7)?,
                        render_scale:
                            row.get(8)?,
                    }
                )
            },
        )
        .map_err(
            |error| {
                format!(
                    "Unable to load application defaults: {}",
                    error,
                )
            }
        )
}


pub fn load_texture_choices(
) -> Result<Vec<String>, String> {

    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while loading texture choices: {}",
                        error,
                    )
                }
            )?;


    let mut statement =
        connection
            .prepare(
                "SELECT texture_name
                 FROM textures
                 ORDER BY lower(texture_name),
                          texture_name"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare texture-catalog query: {}",
                        error,
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                [],
                |row| row.get::<_, String>(0),
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query texture choices: {}",
                        error,
                    )
                }
            )?;


    let mut choices = Vec::new();


    for row in rows {
        choices.push(
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode texture-catalog row: {}",
                        error,
                    )
                }
            )?
        );
    }


    Ok(choices)
}


pub fn load_curated_palette_choices(
) -> Result<Vec<CuratedPaletteChoice>, String> {
    let connection = crate::open_database::open()
        .map_err(|error| format!("Unable to open database while loading curated palette choices: {}", error))?;

    let mut statement = connection.prepare(
        "SELECT color_hex, description FROM curated_palette ORDER BY lower(description), color_hex"
    ).map_err(|error| format!("Unable to prepare curated palette query: {}", error))?;

    let rows = statement.query_map([], |row| {
        Ok(CuratedPaletteChoice {
            color_hex: row.get(0)?,
            description: row.get(1)?,
        })
    }).map_err(|error| format!("Unable to query curated palette choices: {}", error))?;

    let mut choices = Vec::new();
    for row in rows {
        choices.push(row.map_err(|error| format!("Unable to decode curated palette row: {}", error))?);
    }

    Ok(choices)
}


pub fn load_target_defaults(
    target: &str,
) -> Result<TargetDefaults, String> {
    validate_target_name(
        target
    )?;

    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while loading {} defaults: {}",
                        target,
                        error,
                    )
                }
            )?;

    connection
        .query_row(
            "SELECT
                 target,
                 idle_timeout_value,
                 idle_timeout_unit,
                 animation_speed,
                 texture_mode,
                 texture_family,
                 texture_primitives,
                 palette_mode,
                 palette_color
             FROM target_defaults
             WHERE target = ?1",
            [target],
            |row| {
                Ok(
                    TargetDefaults {
                        target:
                            row.get(0)?,
                        idle_timeout_value:
                            row.get(1)?,
                        idle_timeout_unit:
                            row.get(2)?,
                        animation_speed:
                            row.get(3)?,
                        texture_mode:
                            row.get(4)?,
                        texture_family:
                            row.get(5)?,
                        texture_primitives:
                            row.get(6)?,
                        palette_mode:
                            row.get(7)?,
                        palette_color:
                            row.get(8)?,
                    }
                )
            },
        )
        .map_err(
            |error| {
                format!(
                    "Unable to load {} defaults: {}",
                    target,
                    error,
                )
            }
        )
}


pub fn save_app_defaults(
    defaults: &AppDefaults,
) -> Result<(), String> {
    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while saving application defaults: {}",
                        error,
                    )
                }
            )?;

    let changed =
        connection
            .execute(
                "UPDATE app_defaults
                 SET show_splash = ?1,
                     screensaver_subtitles = ?2,
                     subtitle_placement = ?3,
                     wallpaper_notifications = ?4,
                     rendered_fps = ?5,
                     anti_aliasing = ?6,
                     dithering = ?7,
                     color_precision = ?8,
                     render_scale = ?9
                 WHERE defaults_id = 1",
                rusqlite::params![
                    defaults.show_splash,
                    defaults.screensaver_subtitles,
                    defaults.subtitle_placement,
                    defaults.wallpaper_notifications,
                    defaults.rendered_fps,
                    defaults.anti_aliasing,
                    defaults.dithering,
                    defaults.color_precision,
                    defaults.render_scale,
                ],
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to save application defaults: {}",
                        error,
                    )
                }
            )?;

    if changed != 1 {
        return Err(
            format!(
                "Unable to save application defaults: expected one row, updated {}",
                changed,
            )
        );
    }

    Ok(())
}


pub fn save_target_defaults(
    defaults: &TargetDefaults,
) -> Result<(), String> {
    validate_target_name(
        &defaults.target
    )?;

    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while saving {} defaults: {}",
                        defaults.target,
                        error,
                    )
                }
            )?;

    let changed =
        connection
            .execute(
                "UPDATE target_defaults
                 SET idle_timeout_value = ?1,
                     idle_timeout_unit = ?2,
                     animation_speed = ?3,
                     texture_mode = ?4,
                     texture_family = ?5,
                     texture_primitives = ?6,
                     palette_mode = ?7,
                     palette_color = ?8
                 WHERE target = ?9",
                rusqlite::params![
                    defaults.idle_timeout_value,
                    defaults.idle_timeout_unit,
                    defaults.animation_speed,
                    defaults.texture_mode,
                    defaults.texture_family,
                    defaults.texture_primitives,
                    defaults.palette_mode,
                    defaults.palette_color,
                    defaults.target,
                ],
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to save {} defaults: {}",
                        defaults.target,
                        error,
                    )
                }
            )?;

    if changed != 1 {
        return Err(
            format!(
                "Unable to save {} defaults: expected one row, updated {}",
                defaults.target,
                changed,
            )
        );
    }

    Ok(())
}


fn validate_target_name(
    target: &str,
) -> Result<(), String> {
    if matches!(
        target,
        "screensaver" | "wallpaper"
    ) {
        Ok(())
    } else {
        Err(
            format!(
                "Unsupported configuration target '{}'",
                target,
            )
        )
    }
}


//
// ------------------------------------------------------------
// Runtime-target database helpers
// ------------------------------------------------------------
//

pub fn load_runtime_mode(
    target: &str,
) -> Result<String, String> {

    validate_target_name(
        target
    )?;


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while loading {} runtime mode: {}",
                        target,
                        error,
                    )
                }
            )?;


    let (
        display_mode,
        interval_seconds,
        single_policy_id,
    ): (
        String,
        Option<i64>,
        Option<i64>,
    ) =
        connection
            .query_row(
                "SELECT
                     display_mode,
                     interval_seconds,
                     single_policy_id
                 FROM runtime_targets
                 WHERE target = ?1",
                [target],
                |row| {
                    Ok(
                        (
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                        )
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to load {} runtime target configuration: {}",
                        target,
                        error,
                    )
                }
            )?;


    let interval_seconds =
        interval_seconds
            .map(
                |value| {
                    u64::try_from(
                        value
                    )
                    .map_err(
                        |_| {
                            format!(
                                "Invalid negative {} runtime interval_seconds value {}",
                                target,
                                value,
                            )
                        }
                    )
                }
            )
            .transpose()?;


    match display_mode.as_str() {
        "single" => {
            let policy_id =
                single_policy_id
                    .ok_or_else(
                        || {
                            format!(
                                "{} runtime target is Single but has no selected policy",
                                target,
                            )
                        }
                    )?;

            Ok(
                format!(
                    "single:{}",
                    policy_id,
                )
            )
        }

        "random"
        | "ordered" => {
            let interval =
                interval_seconds
                    .filter(
                        |value| {
                            *value > 0
                        }
                    )
                    .ok_or_else(
                        || {
                            format!(
                                "{} runtime target '{}' mode has no valid interval",
                                target,
                                display_mode,
                            )
                        }
                    )?;

            Ok(
                format!(
                    "{}:{}",
                    display_mode,
                    interval,
                )
            )
        }

        other => {
            Err(
                format!(
                    "{} runtime target contains unsupported display mode '{}'",
                    target,
                    other,
                )
            )
        }
    }
}


fn write_runtime_modes(
    screensaver_mode: &str,
    wallpaper_mode: &str,
) -> Result<(), String> {

    let screensaver =
        parse_runtime_mode(
            "screensaver",
            screensaver_mode,
        )?;

    let wallpaper =
        parse_runtime_mode(
            "wallpaper",
            wallpaper_mode,
        )?;


    let mut connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while saving runtime target configuration: {}",
                        error,
                    )
                }
            )?;


    let transaction =
        connection
            .transaction()
            .map_err(
                |error| {
                    format!(
                        "Unable to begin runtime-target configuration transaction: {}",
                        error,
                    )
                }
            )?;


    for runtime in [
        screensaver,
        wallpaper,
    ] {
        if let Some(policy_id) =
            runtime.single_policy_id
        {
            let matching_count: i64 =
                transaction
                    .query_row(
                        "SELECT COUNT(*)
                         FROM shader_policies
                         WHERE policy_id = ?1
                           AND policy_target = ?2",
                        rusqlite::params![
                            policy_id,
                            runtime.target,
                        ],
                        |row| {
                            row.get(0)
                        },
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Unable to validate selected {} Single policy ID {}: {}",
                                runtime.target,
                                policy_id,
                                error,
                            )
                        }
                    )?;

            if matching_count != 1 {
                return Err(
                    format!(
                        "Selected {} Single policy ID {} does not exist with the required target",
                        runtime.target,
                        policy_id,
                    )
                );
            }
        }


        let changed =
            transaction
                .execute(
                    "UPDATE runtime_targets
                     SET display_mode = ?1,
                         interval_seconds = ?2,
                         single_policy_id = ?3
                     WHERE target = ?4",
                    rusqlite::params![
                        runtime.display_mode,
                        runtime.interval_seconds
                            .map(
                                |value| {
                                    i64::try_from(
                                        value
                                    )
                                }
                            )
                            .transpose()
                            .map_err(
                                |_| {
                                    format!(
                                        "{} runtime interval_seconds value is too large for SQLite",
                                        runtime.target,
                                    )
                                }
                            )?,
                        runtime.single_policy_id,
                        runtime.target,
                    ],
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to save {} runtime target configuration: {}",
                            runtime.target,
                            error,
                        )
                    }
                )?;

        if changed != 1 {
            return Err(
                format!(
                    "Unable to save {} runtime target configuration: expected one row, updated {}",
                    runtime.target,
                    changed,
                )
            );
        }
    }


    transaction
        .commit()
        .map_err(
            |error| {
                format!(
                    "Unable to commit runtime-target configuration: {}",
                    error,
                )
            }
        )
}


struct ParsedRuntimeMode<'a> {
    target: &'a str,
    display_mode: &'a str,
    interval_seconds: Option<u64>,
    single_policy_id: Option<i64>,
}


fn parse_runtime_mode<'a>(
    target: &'a str,
    mode: &'a str,
) -> Result<ParsedRuntimeMode<'a>, String> {

    let trimmed =
        mode.trim();

    let mut parts =
        trimmed.splitn(
            2,
            ':'
        );

    let display_mode =
        parts
            .next()
            .unwrap_or("")
            .trim();

    let argument =
        parts
            .next()
            .unwrap_or("")
            .trim();


    match display_mode {
        "single" => {
            let policy_id =
                argument
                    .parse::<i64>()
                    .ok()
                    .filter(
                        |value| {
                            *value > 0
                        }
                    )
                    .ok_or_else(
                        || {
                            format!(
                                "{} Single mode requires a valid policy selection",
                                target,
                            )
                        }
                    )?;

            Ok(
                ParsedRuntimeMode {
                    target,
                    display_mode,
                    interval_seconds:
                        None,
                    single_policy_id:
                        Some(policy_id),
                }
            )
        }

        "random"
        | "ordered" => {
            let interval_seconds =
                argument
                    .parse::<u64>()
                    .ok()
                    .filter(
                        |value| {
                            *value > 0
                        }
                    )
                    .ok_or_else(
                        || {
                            format!(
                                "{} {} mode requires a positive interval",
                                target,
                                display_mode,
                            )
                        }
                    )?;

            Ok(
                ParsedRuntimeMode {
                    target,
                    display_mode,
                    interval_seconds:
                        Some(interval_seconds),
                    single_policy_id:
                        None,
                }
            )
        }

        _ => {
            Err(
                format!(
                    "Unsupported {} display mode '{}'",
                    target,
                    display_mode,
                )
            )
        }
    }
}


//
// ------------------------------------------------------------
// Display-mode helpers
// ------------------------------------------------------------
//

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub enum RotationMode {

    Random,

    Ordered,
}


impl RotationMode {

    pub fn name(
        self,
    ) -> &'static str {

        match self {
            Self::Random => {
                "random"
            }

            Self::Ordered => {
                "ordered"
            }
        }
    }
}


/// Build the canonical mode string used by a rotating shader target.
///
/// Examples:
///     format_rotation_mode(RotationMode::Random, 60)  -> "random:60"
///     format_rotation_mode(RotationMode::Ordered, 10) -> "ordered:10"
pub fn format_rotation_mode(
    mode: RotationMode,
    interval_seconds: u64,
) -> Result<String, String> {

    if interval_seconds
        == 0
    {
        return Err(
            "Shader display interval must be greater than zero"
                .to_string()
        );
    }


    Ok(
        format!(
            "{}:{}",
            mode.name(),
            interval_seconds,
        )
    )
}


/// Split an existing rotating mode string into the values needed by the Config
/// tab's "Display" and "Every" controls.
///
/// Single-shader mode intentionally returns None because it does not have a
/// rotation interval.  The caller may preserve the complete mode string until
/// the user explicitly selects Random or Ordered.
pub fn parse_rotation_mode(
    mode: &str,
) -> Result<Option<(RotationMode, u64)>, String> {

    let trimmed =
        mode.trim();


    if trimmed.is_empty() {
        return Err(
            "Shader display mode may not be empty"
                .to_string()
        );
    }


    let mut parts =
        trimmed.splitn(
            2,
            ':'
        );


    let mode_name =
        parts
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();


    let argument =
        parts
            .next()
            .map(
                str::trim
            );


    match mode_name.as_str() {

        "random"
        | "ordered" => {

            let interval_text =
                argument
                    .ok_or_else(
                        || {
                            format!(
                                "Display mode '{}' requires an interval",
                                mode_name,
                            )
                        }
                    )?;


            let interval_seconds =
                interval_text
                    .parse::<u64>()
                    .map_err(
                        |_| {
                            format!(
                                "Invalid interval '{}' in display mode '{}'",
                                interval_text,
                                trimmed,
                            )
                        }
                    )?;


            if interval_seconds
                == 0
            {
                return Err(
                    "Shader display interval must be greater than zero"
                        .to_string()
                );
            }


            let parsed_mode =
                if mode_name
                    == "random"
                {
                    RotationMode::Random
                } else {
                    RotationMode::Ordered
                };


            Ok(
                Some(
                    (
                        parsed_mode,
                        interval_seconds,
                    )
                )
            )
        }


        "single" => {

            let policy_id =
                argument
                    .unwrap_or("")
                    .trim()
                    .parse::<i64>()
                    .ok()
                    .filter(
                        |policy_id| {
                            *policy_id > 0
                        }
                    );


            if policy_id.is_none() {
                return Err(
                    "Single display mode requires a valid policy ID"
                        .to_string()
                );
            }


            Ok(
                None
            )
        }


        _ => {
            Err(
                format!(
                    "Unsupported shader display mode '{}'",
                    mode_name,
                )
            )
        }
    }
}


//
// ------------------------------------------------------------
// Public save operation
// ------------------------------------------------------------
//

/// Save the retained TOML enable flags and database-backed runtime modes.
///
/// Runtime display modes are written to runtime_targets. The existing TOML
/// document is then loaded and only screensaver.enabled and wallpaper.enabled
/// are changed. All database-backed defaults remain outside this function.
pub fn save_configuration(
    config_path: &Path,
    updates: &ConfigurationUpdates,
) -> Result<(), String> {

    validate_updates(
        updates
    )?;

    write_runtime_modes(
        &updates.screensaver_mode,
        &updates.wallpaper_mode,
    )?;

    let mut document =
        load_document(
            config_path
        )?;

    {
        let screensaver =
            section_table_mut(
                &mut document,
                "screensaver",
            )?;

        screensaver["enabled"] =
            value(
                updates.screensaver_enabled
            );

        // Runtime display-mode state belongs to screenshaver.db.
        let _ = screensaver.remove("mode");

        // Database-backed defaults are deliberately not written to TOML.
    }

    {
        let wallpaper =
            section_table_mut(
                &mut document,
                "wallpaper",
            )?;

        wallpaper["enabled"] =
            value(
                updates.wallpaper_enabled
            );

        // Runtime display-mode state belongs to screenshaver.db.
        let _ = wallpaper.remove("mode");

        // Database-backed defaults are deliberately not written to TOML.
    }

    save_document(
        config_path,
        &document,
    )
}


//
// ------------------------------------------------------------
// Validation
// ------------------------------------------------------------
//

fn validate_updates(
    updates: &ConfigurationUpdates,
) -> Result<(), String> {

    validate_mode_string(
        "screensaver.mode",
        &updates.screensaver_mode,
    )?;

    validate_mode_string(
        "wallpaper.mode",
        &updates.wallpaper_mode,
    )
}


fn validate_mode_string(
    field_name: &str,
    mode: &str,
) -> Result<(), String> {

    let trimmed =
        mode.trim();


    if trimmed.is_empty() {
        return Err(
            format!(
                "{} may not be empty",
                field_name,
            )
        );
    }


    let mut parts =
        trimmed.splitn(
            2,
            ':'
        );


    let mode_name =
        parts
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();


    let argument =
        parts
            .next()
            .unwrap_or("")
            .trim();


    match mode_name.as_str() {

        "random"
        | "ordered" => {

            if argument.is_empty() {
                return Err(
                    format!(
                        "{} '{}' requires an interval",
                        field_name,
                        mode_name,
                    )
                );
            }


            let interval =
                argument
                    .parse::<u64>()
                    .map_err(
                        |_| {
                            format!(
                                "{} interval '{}' is not a positive integer",
                                field_name,
                                argument,
                            )
                        }
                    )?;


            if interval
                == 0
            {
                return Err(
                    format!(
                        "{} interval must be greater than zero",
                        field_name,
                    )
                );
            }
        }


        "single" => {

            let valid_policy_id =
                argument
                    .parse::<i64>()
                    .ok()
                    .is_some_and(
                        |policy_id| {
                            policy_id > 0
                        }
                    );

            if !valid_policy_id {
                return Err(
                    format!(
                        "{} single mode requires a valid policy selection",
                        field_name,
                    )
                );
            }
        }


        _ => {
            return Err(
                format!(
                    "{} contains unsupported mode '{}'; expected single, random, or ordered",
                    field_name,
                    mode_name,
                )
            );
        }
    }


    Ok(())
}


//
// ------------------------------------------------------------
// TOML document handling
// ------------------------------------------------------------
//

fn load_document(
    path: &Path,
) -> Result<DocumentMut, String> {

    let text =
        fs::read_to_string(
            path
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read configuration file {} ({})",
                    path.display(),
                    error,
                )
            }
        )?;


    text.parse::<DocumentMut>()
        .map_err(
            |error| {
                format!(
                    "Unable to parse configuration file {} ({})",
                    path.display(),
                    error,
                )
            }
        )
}


fn save_document(
    path: &Path,
    document: &DocumentMut,
) -> Result<(), String> {

    fs::write(
        path,
        document.to_string(),
    )
    .map_err(
        |error| {
            format!(
                "Unable to write configuration file {} ({})",
                path.display(),
                error,
            )
        }
    )
}


fn section_table_mut<'a>(
    document: &'a mut DocumentMut,
    section_name: &str,
) -> Result<&'a mut Table, String> {

    if !document.contains_key(
        section_name
    ) {
        let mut table =
            Table::new();


        table.set_implicit(
            false
        );


        document[
            section_name
        ] =
            Item::Table(
                table
            );
    }


    document[
        section_name
    ]
        .as_table_mut()
        .ok_or_else(
            || {
                format!(
                    "[{}] exists but is not a TOML table",
                    section_name,
                )
            }
        )
}
