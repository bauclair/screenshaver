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


fn default_subtitle_placement() -> String {
    "bottom:left".to_string()
}


//
// ------------------------------------------------------------
// Structures that exactly match screenshaver.toml
// ------------------------------------------------------------
//

#[derive(Debug, Deserialize)]
struct AppearanceSection {

    subtitles: bool,

    #[serde(default = "default_subtitle_placement")]
    subtitle_placement: String,

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
struct RawTextureOverride {

    shader: String,

    shader_texture: String,

    shader_palette: String,
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

    #[serde(default)]
    texture_override:
        Vec<
            RawTextureOverride
        >,

    locking: LockingSection,

    debug: DebugSection,
}


//
// ------------------------------------------------------------
// Public structure returned to main.rs
// ------------------------------------------------------------
//

#[derive(
    Debug,
    Clone,
)]
pub struct TextureOverride {

    pub shader:
        String,

    pub shader_texture:
        crate::generate_textures::TextureFamily,

    pub shader_palette:
        crate::palettes::Palette,
}


#[derive(
    Debug,
    Clone,
)]
pub struct TextureSelectionPolicy {

    pub global_texture:
        Option<
            crate::generate_textures::TextureFamily
        >,

    pub global_palette:
        Option<
            crate::palettes::Palette
        >,

    pub texture_overrides:
        Vec<
            TextureOverride
        >,
}


#[derive(Debug)]
pub struct Config {

    pub subtitles: bool,

    pub subtitle_placement:
        crate::parse_subtitle_placement::SubtitlePlacement,

    pub show_splash: bool,

    pub mode: String,

    pub idle_timeout: String,

    pub texture_policy:
        TextureSelectionPolicy,

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
    // Parse subtitle placement
    //---------------------------------------------------------

    let parsed_subtitle_placement =
        crate::parse_subtitle_placement::parse(
            Some(
                &raw.appearance.subtitle_placement
            )
        );


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


    let texture_overrides =
        parse_texture_overrides(
            raw.texture_override
        )?;


    let texture_policy =
        TextureSelectionPolicy {
            global_texture,
            global_palette,
            texture_overrides,
        };


    //---------------------------------------------------------
    // Flatten configuration
    //---------------------------------------------------------

    let config =
        Config {

            subtitles:
                raw.appearance.subtitles,

            subtitle_placement:
                parsed_subtitle_placement.placement,

            show_splash:
                raw.appearance.show_splash,

            mode:
                raw.operation.mode,

            idle_timeout:
                raw.operation.idle_timeout,

            texture_policy,

            screen_lock:
                raw.locking.screen_lock,

            debug_log:
                raw.debug.debug_log,
        };


    //---------------------------------------------------------
    // Build diagnostics
    //---------------------------------------------------------

    let mut diagnostics =
        vec![

            format!(
                "[CONFIG] subtitles = {}",
                config.subtitles,
            ),

            format!(
                "[CONFIG] subtitle_placement = {}",
                config.subtitle_placement.name(),
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
                    .texture_policy
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
                    .texture_policy
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


    if let Some(warning) =
        parsed_subtitle_placement.warning
    {
        diagnostics.push(
            warning
        );
    }


    diagnostics.push(
        format!(
            "[CONFIG] texture_override count = {}",
            config
                .texture_policy
                .texture_overrides
                .len(),
        )
    );


    for texture_override in
        &config
            .texture_policy
            .texture_overrides
    {
        diagnostics.push(
            format!(
                "[CONFIG] texture_override shader={} shader_texture={} shader_palette={}",
                texture_override.shader,
                texture_override.shader_texture,
                texture_override.shader_palette,
            )
        );
    }


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



//
// ------------------------------------------------------------
// Per-shader texture override parsing
// ------------------------------------------------------------
//

fn parse_texture_overrides(
    raw_overrides:
        Vec<
            RawTextureOverride
        >,
) -> Result<
    Vec<
        TextureOverride
    >,
    String,
> {

    let mut overrides =
        Vec::with_capacity(
            raw_overrides.len()
        );


    for raw_override in
        raw_overrides
    {
        let shader =
            raw_override
                .shader
                .trim()
                .to_string();


        if shader.is_empty() {
            return Err(
                "A [[texture_override]] block contains an empty shader name"
                    .to_string()
            );
        }


        if overrides
            .iter()
            .any(
                |existing: &TextureOverride| {
                    existing
                        .shader
                        .eq_ignore_ascii_case(
                            &shader
                        )
                }
            )
        {
            return Err(
                format!(
                    "Duplicate [[texture_override]] block for shader '{}'",
                    shader,
                )
            );
        }


        let shader_texture =
            parse_shader_texture(
                &shader,
                &raw_override.shader_texture,
            )?;


        let shader_palette =
            parse_shader_palette(
                &shader,
                &raw_override.shader_palette,
            )?;


        overrides.push(
            TextureOverride {
                shader,
                shader_texture,
                shader_palette,
            }
        );
    }


    Ok(
        overrides
    )
}


fn parse_shader_texture(
    shader: &str,
    value: &str,
) -> Result<
    crate::generate_textures::TextureFamily,
    String,
> {

    let normalized =
        value
            .trim()
            .to_ascii_lowercase();


    if normalized.is_empty()
        || normalized
            == "random"
    {
        return Err(
            format!(
                "[[texture_override]] for '{}' requires a specific shader_texture; 'random' is not permitted",
                shader,
            )
        );
    }


    let family =
        crate::generate_textures::TextureFamily::from_name(
            &normalized
        )
        .map_err(
            |error| {
                format!(
                    "Invalid shader_texture '{}' in [[texture_override]] for '{}': {}",
                    value,
                    shader,
                    error,
                )
            }
        )?;


    if family
        == crate::generate_textures::TextureFamily::Julia
    {
        return Err(
            format!(
                "[[texture_override]] for '{}' selects Julia, but Julia texture generation is not yet implemented",
                shader,
            )
        );
    }


    Ok(
        family
    )
}


fn parse_shader_palette(
    shader: &str,
    value: &str,
) -> Result<
    crate::palettes::Palette,
    String,
> {

    let normalized =
        value
            .trim()
            .to_ascii_lowercase();


    if normalized.is_empty()
        || normalized
            == "random"
    {
        return Err(
            format!(
                "[[texture_override]] for '{}' requires a specific shader_palette; 'random' is not permitted",
                shader,
            )
        );
    }


    crate::palettes::Palette::from_name(
        &normalized
    )
    .map_err(
        |error| {
            format!(
                "Invalid shader_palette '{}' in [[texture_override]] for '{}': {}",
                value,
                shader,
                error,
            )
        }
    )
}

