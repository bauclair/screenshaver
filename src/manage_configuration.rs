//! Application-level configuration persistence for the Screenshaver Control Center.
//!
//! This module edits the global [screensaver] and [wallpaper] fields exposed by
//! the Control Center Config tab. Runtime display mode state is stored in
//! screenshaver.db (runtime_targets), while non-policy global settings remain in
//! screenshaver.toml. toml_edit preserves unrelated settings and formatting as
//! far as practical.

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

    /// Complete idle-timeout string as stored in screenshaver.toml.
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

    /// Complete wallpaper mode string as stored in screenshaver.toml.
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

    /// Build an editable Config-tab snapshot from the already-loaded runtime
    /// configuration.
    ///
    /// This is intentionally limited to fields represented by the first Config
    /// tab design.  Other screenshaver.toml settings remain outside this
    /// structure and therefore cannot be accidentally overwritten by
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
// Runtime-target database helpers
// ------------------------------------------------------------
//

pub fn load_runtime_mode(
    target: &str,
) -> Result<String, String> {

    if !matches!(
        target,
        "screensaver" | "wallpaper"
    ) {
        return Err(
            format!(
                "Unsupported runtime target '{}'",
                target,
            )
        );
    }


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

            let shader =
                argument
                    .unwrap_or("")
                    .trim();


            if shader.is_empty() {
                return Err(
                    "Single display mode requires a shader filename"
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

/// Save only the application-level fields represented by the Config tab.
///
/// This function does not rebuild screenshaver.toml from Config.  It loads the
/// existing TOML document, changes only the intended keys, and writes the
/// resulting document back.  Policy tables and unrelated application settings
/// therefore remain untouched.
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


        screensaver[
            "enabled"
        ] =
            value(
                updates.screensaver_enabled
            );


        screensaver[
            "subtitles"
        ] =
            value(
                updates.subtitles
            );


        // Runtime display-mode state belongs to screenshaver.db.
        // Remove legacy mode keys when the Config tab is saved.
        let _ =
            screensaver.remove(
                "mode"
            );


        screensaver[
            "idle_timeout"
        ] =
            value(
                updates
                    .idle_timeout
                    .trim()
            );


        screensaver[
            "global_texture"
        ] =
            value(
                format_global_texture(
                    updates
                        .screensaver_global_texture
                        .as_ref()
                )
            );


        screensaver[
            "global_palette"
        ] =
            value(
                format_global_palette(
                    updates
                        .screensaver_global_palette
                        .as_ref()
                )
            );
    }


    {
        let wallpaper =
            section_table_mut(
                &mut document,
                "wallpaper",
            )?;


        wallpaper[
            "enabled"
        ] =
            value(
                updates.wallpaper_enabled
            );


        wallpaper[
            "notifications"
        ] =
            value(
                updates.notifications
            );


        // Runtime display-mode state belongs to screenshaver.db.
        // Remove legacy mode keys when the Config tab is saved.
        let _ =
            wallpaper.remove(
                "mode"
            );


        wallpaper[
            "global_texture"
        ] =
            value(
                format_global_texture(
                    updates
                        .wallpaper_global_texture
                        .as_ref()
                )
            );


        wallpaper[
            "global_palette"
        ] =
            value(
                format_global_palette(
                    updates
                        .wallpaper_global_palette
                        .as_ref()
                )
            );
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
    )?;


    if updates
        .idle_timeout
        .trim()
        .is_empty()
    {
        return Err(
            "screensaver.idle_timeout may not be empty"
                .to_string()
        );
    }


    Ok(())
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


//
// ------------------------------------------------------------
// Global texture / palette serialization
// ------------------------------------------------------------
//

fn format_global_texture(
    texture:
        Option<
            &crate::parse_texture_specification::TextureSpecification
        >,
) -> String {

    let Some(texture) =
        texture
    else {
        return "random"
            .to_string();
    };


    if texture.count_was_explicit {

        format!(
            "{}:{}",
            texture.family.name(),
            texture.requested_primitive_count,
        )

    } else {

        texture
            .family
            .name()
            .to_string()
    }
}


fn format_global_palette(
    palette:
        Option<
            &crate::palettes::PaletteColor
        >,
) -> String {

    palette
        .map(
            |palette| {
                palette.to_hex()
            }
        )
        .unwrap_or_else(
            || {
                "random"
                    .to_string()
            }
        )
}

