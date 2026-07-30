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


fn default_screensaver_enabled() -> bool {
    true
}


fn default_wallpaper_enabled() -> bool {
    false
}


fn default_global_rendered_fps() -> u32 {
    crate::define_constants::DEFAULT_RENDER_FPS
}


fn default_screensaver_global_speed() -> f32 {
    crate::define_constants::SCREENSAVER_SPEED_DEFAULT
}


fn default_wallpaper_global_speed() -> f32 {
    crate::define_constants::WALLPAPER_SPEED_DEFAULT
}


fn default_subtitle_placement() -> String {
    "bottom:left".to_string()
}


fn default_log_level() -> u8 {
    5
}


//
// ------------------------------------------------------------
// Structures that exactly match screenshaver.toml
// ------------------------------------------------------------
//

#[derive(Debug, Deserialize)]
struct AppearanceSection {

    #[serde(default = "default_show_splash")]
    show_splash: bool,
}


#[derive(Debug, Deserialize)]
struct ScreensaverSection {

    #[serde(default = "default_screensaver_enabled")]
    enabled: bool,

    subtitles: bool,

    #[serde(default = "default_subtitle_placement")]
    subtitle_placement: String,

    mode: String,

    idle_timeout: String,

    #[serde(default)]
    global_texture: Option<String>,

    #[serde(default)]
    global_palette: Option<String>,

    #[serde(default = "default_screensaver_global_speed")]
    global_speed: f32,
}


#[derive(Debug, Deserialize)]
struct WallpaperSection {

    #[serde(default = "default_wallpaper_enabled")]
    enabled: bool,

    mode: String,

    #[serde(default)]
    global_texture: Option<String>,

    #[serde(default)]
    global_palette: Option<String>,

    monitor_mode: String,

    notifications: bool,

    #[serde(default = "default_wallpaper_global_speed")]
    global_speed: f32,
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
struct RawToml {

    appearance: AppearanceSection,

    screensaver: ScreensaverSection,

    wallpaper: WallpaperSection,

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
pub struct ShaderOverride {

    pub shader:
        String,

    pub shader_texture:
        Option<
            crate::parse_texture_specification::TextureSpecification
        >,

    pub shader_palette:
        Option<
            crate::palettes::Palette
        >,

    pub rendered_fps:
        Option<u32>,

    pub animation_speed:
        Option<f32>,
}


#[derive(
    Debug,
    Clone,
)]
pub struct AnimationSpeedPolicy {

    pub global_speed: f32,

    pub shader_overrides:
        Vec<
            ShaderOverride
        >,
}


impl AnimationSpeedPolicy {

    pub fn animation_speed_for_shader(
        &self,
        shader_name: &str,
        command_line_speed: Option<f32>,
    ) -> f32 {

        if let Some(speed) =
            command_line_speed
        {
            return speed;
        }


        self.shader_overrides
            .iter()
            .find(
                |shader_override| {
                    shader_override
                        .shader
                        .eq_ignore_ascii_case(
                            shader_name
                        )
                }
            )
            .and_then(
                |shader_override| {
                    shader_override.animation_speed
                }
            )
            .unwrap_or(
                self.global_speed
            )
    }
}


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
pub struct FpsSelectionPolicy {

    pub global_rendered_fps: u32,

    pub fps_overrides:
        Vec<
            FpsOverride
        >,
}


impl FpsSelectionPolicy {

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

    pub screensaver_enabled: bool,

    pub wallpaper_enabled: bool,

    pub subtitles: bool,

    pub subtitle_placement:
        crate::parse_subtitle_placement::SubtitlePlacement,

    pub show_splash: bool,

    pub mode: String,

    pub idle_timeout: String,

    pub texture_policy:
        TextureSelectionPolicy,

    pub wallpaper:
        crate::define_wallpaper::WallpaperSettings,

    pub wallpaper_mode: String,

    pub wallpaper_texture_policy:
        TextureSelectionPolicy,

    pub screensaver_global_speed: f32,

    pub wallpaper_global_speed: f32,

    pub shader_overrides:
        Vec<
            ShaderOverride
        >,

    pub screensaver_speed_policy:
        AnimationSpeedPolicy,

    pub wallpaper_speed_policy:
        AnimationSpeedPolicy,

    pub global_rendered_fps: u32,

    pub fps_overrides:
        Vec<
            FpsOverride
        >,

    pub fps_policy:
        FpsSelectionPolicy,

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
                &raw.screensaver.subtitle_placement
            )
        );


    //---------------------------------------------------------
    // Parse global texture and palette policy
    //---------------------------------------------------------

    let screensaver_global_texture =
        parse_global_texture(
            raw.screensaver
                .global_texture
                .as_deref()
        )?;


    let screensaver_global_palette =
        parse_global_palette(
            raw.screensaver
                .global_palette
                .as_deref()
        )?;


    let wallpaper_global_texture =
        parse_global_texture(
            raw.wallpaper
                .global_texture
                .as_deref()
        )?;


    let wallpaper_global_palette =
        parse_global_palette(
            raw.wallpaper
                .global_palette
                .as_deref()
        )?;


    let texture_overrides =
        parse_texture_overrides(
            raw.texture_override
        )?;


    let texture_policy =
        TextureSelectionPolicy {
            global_texture:
                screensaver_global_texture,
            global_palette:
                screensaver_global_palette,
            texture_overrides:
                texture_overrides.clone(),
        };


    let wallpaper_texture_policy =
        TextureSelectionPolicy {
            global_texture:
                wallpaper_global_texture,
            global_palette:
                wallpaper_global_palette,
            texture_overrides,
        };


    let wallpaper_monitor_mode =
        crate::define_wallpaper::WallpaperMonitorMode::parse(
            &raw.wallpaper.monitor_mode
        )?;


    let wallpaper =
        crate::define_wallpaper::WallpaperSettings {
            monitor_mode:
                wallpaper_monitor_mode,

            notifications:
                raw.wallpaper.notifications,
        };


    let (
        screensaver_global_speed,
        screensaver_global_speed_warning,
    ) =
        validate_animation_speed(
            raw.screensaver.global_speed,
            crate::define_constants::SCREENSAVER_SPEED_MIN,
            crate::define_constants::SCREENSAVER_SPEED_MAX,
            crate::define_constants::SCREENSAVER_SPEED_DEFAULT,
            "screensaver.global_speed",
        );


    let (
        wallpaper_global_speed,
        wallpaper_global_speed_warning,
    ) =
        validate_animation_speed(
            raw.wallpaper.global_speed,
            crate::define_constants::WALLPAPER_SPEED_MIN,
            crate::define_constants::WALLPAPER_SPEED_MAX,
            crate::define_constants::WALLPAPER_SPEED_DEFAULT,
            "wallpaper.global_speed",
        );


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


    let fps_policy =
        FpsSelectionPolicy {
            global_rendered_fps,
            fps_overrides:
                fps_overrides.clone(),
        };


    let shader_overrides =
        merge_legacy_shader_overrides(
            &texture_policy.texture_overrides,
            &fps_overrides,
        );


    let screensaver_speed_policy =
        AnimationSpeedPolicy {
            global_speed:
                screensaver_global_speed,
            shader_overrides:
                shader_overrides.clone(),
        };


    let wallpaper_speed_policy =
        AnimationSpeedPolicy {
            global_speed:
                wallpaper_global_speed,
            shader_overrides:
                shader_overrides.clone(),
        };


    let (
        log_level,
        log_level_warning,
    ) =
        validate_log_level(
            raw.debug.log_level
        );


    //---------------------------------------------------------
    // Flatten configuration
    //---------------------------------------------------------

    let config =
        Config {

            screensaver_enabled:
                raw.screensaver.enabled,

            wallpaper_enabled:
                raw.wallpaper.enabled,

            subtitles:
                raw.screensaver.subtitles,

            subtitle_placement:
                parsed_subtitle_placement.placement,

            show_splash:
                raw.appearance.show_splash,

            mode:
                raw.screensaver.mode,

            idle_timeout:
                raw.screensaver.idle_timeout,

            texture_policy,

            wallpaper,

            wallpaper_mode:
                raw.wallpaper.mode,

            wallpaper_texture_policy,

            screensaver_global_speed,

            wallpaper_global_speed,

            shader_overrides,

            screensaver_speed_policy,

            wallpaper_speed_policy,

            global_rendered_fps,

            fps_overrides,

            fps_policy,

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
                "[CONFIG] screensaver.enabled = {}",
                config.screensaver_enabled,
            ),

            format!(
                "[CONFIG] wallpaper.enabled = {}",
                config.wallpaper_enabled,
            ),

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
                "[CONFIG] screensaver.mode = {}",
                config.mode,
            ),

            format!(
                "[CONFIG] screensaver.idle_timeout = {}",
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
                "[CONFIG] wallpaper.mode = {}",
                config.wallpaper_mode,
            ),

            format!(
                "[CONFIG] wallpaper.global_texture = {}",
                config
                    .wallpaper_texture_policy
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
                "[CONFIG] wallpaper.global_palette = {}",
                config
                    .wallpaper_texture_policy
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
                "[CONFIG] wallpaper.monitor_mode = {}",
                config.wallpaper.monitor_mode.name(),
            ),

            format!(
                "[CONFIG] wallpaper.notifications = {}",
                config.wallpaper.notifications,
            ),

            format!(
                "[CONFIG] screensaver.global_speed = {}",
                config.screensaver_global_speed,
            ),

            format!(
                "[CONFIG] wallpaper.global_speed = {}",
                config.wallpaper_global_speed,
            ),

            format!(
                "[CONFIG] unified shader_override count = {}",
                config.shader_overrides.len(),
            ),

            format!(
                "[CONFIG] global_rendered_fps = {}",
                config.global_rendered_fps,
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
        screensaver_global_speed_warning
    {
        diagnostics.push(
            warning
        );
    }


    if let Some(warning) =
        wallpaper_global_speed_warning
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
// Legacy override consolidation
// ------------------------------------------------------------
//

fn merge_legacy_shader_overrides(
    texture_overrides: &[TextureOverride],
    fps_overrides: &[FpsOverride],
) -> Vec<ShaderOverride> {

    let mut merged:
        Vec<ShaderOverride> =
            Vec::new();


    for texture_override in
        texture_overrides
    {
        merged.push(
            ShaderOverride {
                shader:
                    texture_override.shader.clone(),
                shader_texture:
                    Some(
                        texture_override.shader_texture.clone()
                    ),
                shader_palette:
                    Some(
                        texture_override.shader_palette
                    ),
                rendered_fps:
                    None,
                animation_speed:
                    None,
            }
        );
    }


    for fps_override in
        fps_overrides
    {
        if let Some(existing) =
            merged
                .iter_mut()
                .find(
                    |shader_override| {
                        shader_override
                            .shader
                            .eq_ignore_ascii_case(
                                &fps_override.shader
                            )
                    }
                )
        {
            existing.rendered_fps =
                Some(
                    fps_override.rendered_fps
                );
        } else {
            merged.push(
                ShaderOverride {
                    shader:
                        fps_override.shader.clone(),
                    shader_texture:
                        None,
                    shader_palette:
                        None,
                    rendered_fps:
                        Some(
                            fps_override.rendered_fps
                        ),
                    animation_speed:
                        None,
                }
            );
        }
    }


    merged
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
// Animation-speed validation
// ------------------------------------------------------------
//

fn validate_animation_speed(
    value: f32,
    minimum: f32,
    maximum: f32,
    fallback: f32,
    setting_name: &str,
) -> (
    f32,
    Option<String>,
) {

    if value.is_finite()
        && (minimum..=maximum).contains(
            &value
        )
    {
        return (
            value,
            None,
        );
    }


    (
        fallback,
        Some(
            format!(
                "[CONFIG] WARNING: {} = {} is outside the supported range {}-{}; using {}",
                setting_name,
                value,
                minimum,
                maximum,
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

