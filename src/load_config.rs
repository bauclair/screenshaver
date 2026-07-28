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


fn default_global_rendered_fps() -> u32 {
    crate::define_constants::DEFAULT_RENDER_FPS
}


fn default_subtitle_placement() -> String {
    "bottom:left".to_string()
}


fn default_log_level() -> u8 {
    5
}


fn default_wallpaper_monitor_mode() -> String {
    "mirror".to_string()
}


fn default_wallpaper_notifications() -> bool {
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
struct RawFpsOverride {

    shader: String,

    rendered_fps: u32,
}


#[derive(Debug, Deserialize)]
struct PerformanceSection {

    #[serde(default = "default_global_rendered_fps")]
    global_rendered_fps: u32,
}


impl Default for PerformanceSection {

    fn default() -> Self {

        Self {
            global_rendered_fps:
                default_global_rendered_fps(),
        }
    }
}


#[derive(Debug, Deserialize)]
struct LockingSection {

    screen_lock: bool,
}


#[derive(Debug, Deserialize)]
struct DebugSection {

    debug_log: bool,

    #[serde(default = "default_log_level")]
    log_level: u8,
}


#[derive(Debug, Deserialize)]
struct WallpaperSection {

    #[serde(default = "default_wallpaper_monitor_mode")]
    monitor_mode: String,

    #[serde(default = "default_wallpaper_notifications")]
    notifications: bool,
}


impl Default for WallpaperSection {

    fn default() -> Self {

        Self {
            monitor_mode:
                default_wallpaper_monitor_mode(),

            notifications:
                default_wallpaper_notifications(),
        }
    }
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

    #[serde(default)]
    performance: PerformanceSection,

    #[serde(default)]
    fps_override:
        Vec<
            RawFpsOverride
        >,

    #[serde(default)]
    wallpaper: WallpaperSection,

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
        crate::parse_texture_specification::TextureSpecification,

    pub shader_palette:
        crate::palettes::Palette,
}


#[derive(
    Debug,
    Clone,
)]
pub struct FpsOverride {

    pub shader:
        String,

    pub rendered_fps:
        u32,
}


#[derive(
    Debug,
    Clone,
)]
pub struct TextureSelectionPolicy {

    pub global_texture:
        Option<
            crate::parse_texture_specification::TextureSpecification
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

    pub global_rendered_fps: u32,

    pub fps_overrides:
        Vec<
            FpsOverride
        >,

    pub wallpaper:
        crate::define_wallpaper::WallpaperSettings,

    pub screen_lock: bool,

    pub debug_log: bool,

    pub log_level: u8,
}


impl Config {

    pub fn rendered_fps_for_shader(
        &self,
        shader_name: &str,
        command_line_fps: Option<u32>,
    ) -> u32 {

        if let Some(fps) =
            command_line_fps
        {
            return fps.max(
                1
            );
        }


        self.fps_overrides
            .iter()
            .find(
                |fps_override| {
                    fps_override
                        .shader
                        .eq_ignore_ascii_case(
                            shader_name
                        )
                }
            )
            .map(
                |fps_override| {
                    fps_override.rendered_fps
                }
            )
            .unwrap_or(
                self.global_rendered_fps
            )
            .max(
                1
            )
    }
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


    let (
        global_rendered_fps,
        global_rendered_fps_warning,
    ) =
        validate_rendered_fps(
            raw.performance.global_rendered_fps
        );


    let fps_overrides =
        parse_fps_overrides(
            raw.fps_override
        )?;


    let (
        log_level,
        log_level_warning,
    ) =
        validate_log_level(
            raw.debug.log_level
        );


    let wallpaper =
        crate::configure_wallpaper::resolve(
            &raw.wallpaper.monitor_mode,
            raw.wallpaper.notifications,
        )
        .map_err(
            |error| {
                format!(
                    "Invalid [wallpaper] configuration: {}",
                    error,
                )
            }
        )?;


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

            global_rendered_fps,

            fps_overrides,

            wallpaper,

            screen_lock:
                raw.locking.screen_lock,

            debug_log:
                raw.debug.debug_log,

            log_level,
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
                    .as_ref()
                    .map(
                        |texture| {
                            format_texture_specification(
                                texture
                            )
                        }
                    )
                    .unwrap_or_else(
                        || {
                            "random".to_string()
                        }
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
                "[CONFIG] global_rendered_fps = {}",
                config.global_rendered_fps,
            ),

            format!(
                "[CONFIG] wallpaper.monitor_mode = {}",
                config.wallpaper.monitor_mode.name(),
            ),

            format!(
                "[CONFIG] wallpaper.notifications = {}",
                config.wallpaper.notifications,
            ),

            format!(
                "[CONFIG] screen_lock = {}",
                config.screen_lock,
            ),

            format!(
                "[CONFIG] debug_log = {}",
                config.debug_log,
            ),

            format!(
                "[CONFIG] log_level = {}",
                config.log_level,
            ),
        ];


    if let Some(warning) =
        parsed_subtitle_placement.warning
    {
        diagnostics.push(
            warning
        );
    }


    if let Some(warning) =
        global_rendered_fps_warning
    {
        diagnostics.push(
            warning
        );
    }


    if let Some(warning) =
        log_level_warning
    {
        diagnostics.push(
            warning
        );
    }


    diagnostics.push(
        format!(
            "[CONFIG] fps_override count = {}",
            config.fps_overrides.len(),
        )
    );


    for fps_override in
        &config.fps_overrides
    {
        diagnostics.push(
            format!(
                "[CONFIG] fps_override shader={} rendered_fps={}",
                fps_override.shader,
                fps_override.rendered_fps,
            )
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
            format_texture_specification(
                &texture_override.shader_texture
            ),
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
// Log-level validation
// ------------------------------------------------------------
//

fn validate_log_level(
    value: u8,
) -> (
    u8,
    Option<String>,
) {

    if (1..=6).contains(
        &value
    ) {
        return (
            value,
            None,
        );
    }


    let fallback =
        default_log_level();


    (
        fallback,
        Some(
            format!(
                "[CONFIG] WARNING: log_level = {} is outside the supported range 1-6; using {}",
                value,
                fallback,
            )
        ),
    )
}


//
// ------------------------------------------------------------
// Performance validation
// ------------------------------------------------------------
//

fn validate_rendered_fps(
    value: u32,
) -> (
    u32,
    Option<String>,
) {

    if (
        crate::define_constants::MIN_RENDER_FPS
            ..=
        crate::define_constants::MAX_RENDER_FPS
    )
        .contains(
            &value
        )
    {
        return (
            value,
            None,
        );
    }


    let fallback =
        crate::define_constants::DEFAULT_RENDER_FPS;


    (
        fallback,
        Some(
            format!(
                "[CONFIG] WARNING: global_rendered_fps = {} is outside the supported range {}-{}; using {}",
                value,
                crate::define_constants::MIN_RENDER_FPS,
                crate::define_constants::MAX_RENDER_FPS,
                fallback,
            )
        ),
    )
}



//
// ------------------------------------------------------------
// Per-shader FPS override parsing
// ------------------------------------------------------------
//

fn parse_fps_overrides(
    raw_overrides:
        Vec<
            RawFpsOverride
        >,
) -> Result<
    Vec<
        FpsOverride
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
                "A [[fps_override]] block contains an empty shader name"
                    .to_string()
            );
        }


        if overrides
            .iter()
            .any(
                |existing: &FpsOverride| {
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
                    "Duplicate [[fps_override]] block for shader '{}'",
                    shader,
                )
            );
        }


        let (
            rendered_fps,
            warning,
        ) =
            validate_rendered_fps(
                raw_override.rendered_fps
            );


        if warning.is_some() {
            return Err(
                format!(
                    "rendered_fps = {} in [[fps_override]] for '{}' is outside the supported range {}-{}",
                    raw_override.rendered_fps,
                    shader,
                    crate::define_constants::MIN_RENDER_FPS,
                    crate::define_constants::MAX_RENDER_FPS,
                )
            );
        }


        overrides.push(
            FpsOverride {
                shader,
                rendered_fps,
            }
        );
    }


    Ok(
        overrides
    )
}


//
// ------------------------------------------------------------
// Global texture and palette parsing
// ------------------------------------------------------------
//

fn format_texture_specification(
    texture:
        &crate::parse_texture_specification::TextureSpecification,
) -> String {

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

fn parse_global_texture(
    value: Option<&str>,
) -> Result<
    Option<
        crate::parse_texture_specification::TextureSpecification
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


    let texture =
        crate::parse_texture_specification::parse_texture_specification(
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


    Ok(
        Some(
            texture
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
    crate::parse_texture_specification::TextureSpecification,
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


    let texture =
        crate::parse_texture_specification::parse_texture_specification(
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


    Ok(
        texture
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

