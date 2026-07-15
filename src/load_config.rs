use serde::Deserialize;
use std::fs;
use std::path::Path;


//
// ------------------------------------------------------------
// Default values
// ------------------------------------------------------------
//

fn default_show_splash() -> bool {
    true
}


//
// ------------------------------------------------------------
// Structures that exactly match screenshaver.toml
// ------------------------------------------------------------
//

#[derive(Debug, Deserialize)]
struct AppearanceSection {

    subtitles: bool,

    #[serde(default = "default_show_splash")]
    show_splash: bool,
}


#[derive(Debug, Deserialize)]
struct OperationSection {

    mode: String,

    idle_timeout: String,

    #[serde(default)]
    global_texture: Option<String>,

    #[serde(default)]
    global_palette: Option<String>,
}


#[derive(Debug, Deserialize)]
struct LockingSection {

    screen_lock: bool,
}


#[derive(Debug, Deserialize)]
struct DebugSection {

    debug_log: bool,
}


#[derive(Debug, Deserialize)]
struct RawToml {

    appearance: AppearanceSection,

    operation: OperationSection,
    locking: LockingSection,

    debug: DebugSection,
}


//
// ------------------------------------------------------------
// Public structure returned to main.rs
// ------------------------------------------------------------
//

#[derive(Debug)]
pub struct Config {

    pub subtitles: bool,

    pub show_splash: bool,

    pub mode: String,

    pub idle_timeout: String,

    pub global_texture:
        Option<
            crate::generate_textures::TextureFamily
        >,

    pub global_palette:
        Option<
            crate::palettes::Palette
        >,

    pub screen_lock: bool,

    pub debug_log: bool,
}


//
// ------------------------------------------------------------
// Diagnostic messages
// ------------------------------------------------------------
//

#[derive(Debug)]
pub struct ConfigResult {

    pub config: Config,

    pub diagnostics: Vec<String>,
}


//
// ------------------------------------------------------------
// Load configuration file
// ------------------------------------------------------------
//

pub fn load_config(
    path: &Path,
) -> Result<ConfigResult, String> {

    //---------------------------------------------------------
    // Read file
    //---------------------------------------------------------

    let text =
        fs::read_to_string(path)
            .map_err(|err| {

                format!(
                    "Unable to read configuration file {} ({})",
                    path.display(),
                    err,
                )
            })?;


    //---------------------------------------------------------
    // Parse TOML
    //---------------------------------------------------------

    let raw: RawToml =
        toml::from_str(&text)
            .map_err(|err| {

                format!(
                    "Invalid TOML in {} ({})",
                    path.display(),
                    err,
                )
            })?;


    //---------------------------------------------------------
    // Parse global texture and palette policy
    //---------------------------------------------------------

    let global_texture =
        parse_global_texture(
            raw.operation
                .global_texture
                .as_deref()
        )?;


    let global_palette =
        parse_global_palette(
            raw.operation
                .global_palette
                .as_deref()
        )?;


    //---------------------------------------------------------
    // Flatten configuration
    //---------------------------------------------------------

    let config =
        Config {

            subtitles:
                raw.appearance.subtitles,

            show_splash:
                raw.appearance.show_splash,

            mode:
                raw.operation.mode,

            idle_timeout:
                raw.operation.idle_timeout,

            global_texture,

            global_palette,

            screen_lock:
                raw.locking.screen_lock,

            debug_log:
                raw.debug.debug_log,
        };


    //---------------------------------------------------------
    // Build diagnostics
    //---------------------------------------------------------

    let diagnostics =
        vec![

            format!(
                "[CONFIG] subtitles = {}",
                config.subtitles,
            ),

            format!(
                "[CONFIG] show_splash = {}",
                config.show_splash,
            ),

            format!(
                "[CONFIG] mode = {}",
                config.mode,
            ),

            format!(
                "[CONFIG] idle_timeout = {}",
                config.idle_timeout,
            ),

            format!(
                "[CONFIG] global_texture = {}",
                config
                    .global_texture
                    .map(
                        |family| {
                            family.name()
                        }
                    )
                    .unwrap_or(
                        "random"
                    ),
            ),

            format!(
                "[CONFIG] global_palette = {}",
                config
                    .global_palette
                    .map(
                        |palette| {
                            palette.name()
                        }
                    )
                    .unwrap_or(
                        "random"
                    ),
            ),

            format!(
                "[CONFIG] screen_lock = {}",
                config.screen_lock,
            ),

            format!(
                "[CONFIG] debug_log = {}",
                config.debug_log,
            ),
        ];


    Ok(
        ConfigResult {

            config,

            diagnostics,
        }
    )
}

//
// ------------------------------------------------------------
// Global texture and palette parsing
// ------------------------------------------------------------
//

fn parse_global_texture(
    value: Option<&str>,
) -> Result<
    Option<
        crate::generate_textures::TextureFamily
    >,
    String,
> {

    let Some(value) =
        value
    else {
        return Ok(
            None
        );
    };


    let normalized =
        value
            .trim()
            .to_ascii_lowercase();


    if normalized.is_empty()
        || normalized
            == "random"
    {
        return Ok(
            None
        );
    }


    let family =
        crate::generate_textures::TextureFamily::from_name(
            &normalized
        )
        .map_err(
            |error| {
                format!(
                    "Invalid global_texture value '{}': {}",
                    value,
                    error,
                )
            }
        )?;


    if family
        == crate::generate_textures::TextureFamily::Julia
    {
        return Err(
            "global_texture = \"julia\" is recognized, but Julia texture generation is not yet implemented"
                .to_string()
        );
    }


    Ok(
        Some(
            family
        )
    )
}


fn parse_global_palette(
    value: Option<&str>,
) -> Result<
    Option<
        crate::palettes::Palette
    >,
    String,
> {

    let Some(value) =
        value
    else {
        return Ok(
            None
        );
    };


    let normalized =
        value
            .trim()
            .to_ascii_lowercase();


    if normalized.is_empty()
        || normalized
            == "random"
    {
        return Ok(
            None
        );
    }


    crate::palettes::Palette::from_name(
        &normalized
    )
    .map(
        Some
    )
    .map_err(
        |error| {
            format!(
                "Invalid global_palette value '{}': {}",
                value,
                error,
            )
        }
    )
}

