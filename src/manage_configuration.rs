//! Application-level configuration persistence for the Screenshaver Control Center.
//!
//! This module deliberately edits only the global [screensaver] and [wallpaper]
//! configuration fields exposed by the Control Center's Config tab.  It uses
//! toml_edit so unrelated settings, comments, policy tables, and formatting are
//! preserved as far as toml_edit permits.

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
    ///     single:default.glsl
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
            crate::palettes::Palette
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
            crate::palettes::Palette
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


        screensaver[
            "mode"
        ] =
            value(
                updates
                    .screensaver_mode
                    .trim()
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


        wallpaper[
            "mode"
        ] =
            value(
                updates
                    .wallpaper_mode
                    .trim()
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

            if argument.is_empty() {
                return Err(
                    format!(
                        "{} single mode requires a shader filename",
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
            &crate::palettes::Palette
        >,
) -> String {

    palette
        .map(
            |palette| {
                palette.name()
                    .to_string()
            }
        )
        .unwrap_or_else(
            || {
                "random"
                    .to_string()
            }
        )
}

