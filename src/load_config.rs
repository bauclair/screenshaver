use serde::Deserialize;
use std::collections::BTreeMap;
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
    screensaver_policies:
        BTreeMap<String, String>,

    #[serde(default)]
    wallpaper_policies:
        BTreeMap<String, String>,

    #[serde(default)]
    performance: PerformanceSection,

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
pub struct ShaderPolicy {

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

    pub shader_policies:
        Vec<
            ShaderPolicy
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


        self.shader_policies
            .iter()
            .find(
                |shader_policy| {
                    shader_policy
                        .shader
                        .eq_ignore_ascii_case(
                            shader_name
                        )
                }
            )
            .and_then(
                |shader_policy| {
                    shader_policy.animation_speed
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
pub struct TexturePolicyEntry {

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
}


#[derive(
    Debug,
    Clone,
)]
pub struct FpsPolicyEntry {

    pub shader:
        String,

    pub rendered_fps:
        u32,
}


#[derive(
    Debug,
    Clone,
)]
pub struct FpsPolicy {

    pub global_rendered_fps: u32,

    pub fps_policy_entries:
        Vec<
            FpsPolicyEntry
        >,
}


impl FpsPolicy {

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


        self.fps_policy_entries
            .iter()
            .find(
                |fps_policy_entry| {
                    fps_policy_entry
                        .shader
                        .eq_ignore_ascii_case(
                            shader_name
                        )
                }
            )
            .map(
                |fps_policy_entry| {
                    fps_policy_entry.rendered_fps
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
pub struct TexturePolicy {

    pub global_texture:
        Option<
            crate::parse_texture_specification::TextureSpecification
        >,

    pub global_palette:
        Option<
            crate::palettes::Palette
        >,

    pub texture_policy_entries:
        Vec<
            TexturePolicyEntry
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
        TexturePolicy,

    pub wallpaper:
        crate::define_wallpaper::WallpaperSettings,

    pub wallpaper_mode: String,

    pub wallpaper_texture_policy:
        TexturePolicy,

    pub screensaver_global_speed: f32,

    pub wallpaper_global_speed: f32,

    pub screensaver_policies:
        Vec<
            ShaderPolicy
        >,

    pub wallpaper_policies:
        Vec<
            ShaderPolicy
        >,

    pub screensaver_speed_policy:
        AnimationSpeedPolicy,

    pub wallpaper_speed_policy:
        AnimationSpeedPolicy,

    pub global_rendered_fps: u32,

    pub screensaver_fps_policy_entries:
        Vec<
            FpsPolicyEntry
        >,

    pub wallpaper_fps_policy:
        FpsPolicy,

    pub screen_lock: bool,

    pub debug_log: bool,

    pub log_level: u8,
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


    let screensaver_policies =
        parse_policy_table(
            raw.screensaver_policies,
            PolicyTarget::Screensaver,
        )?;


    let wallpaper_policies =
        parse_policy_table(
            raw.wallpaper_policies,
            PolicyTarget::Wallpaper,
        )?;


    let screensaver_texture_policy_entries =
        texture_policy_entries_from(
            &screensaver_policies
        );


    let wallpaper_texture_policy_entries =
        texture_policy_entries_from(
            &wallpaper_policies
        );


    let texture_policy =
        TexturePolicy {
            global_texture:
                screensaver_global_texture,
            global_palette:
                screensaver_global_palette,
            texture_policy_entries:
                screensaver_texture_policy_entries,
        };


    let wallpaper_texture_policy =
        TexturePolicy {
            global_texture:
                wallpaper_global_texture,
            global_palette:
                wallpaper_global_palette,
            texture_policy_entries:
                wallpaper_texture_policy_entries,
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


    let screensaver_fps_policy_entries =
        fps_policy_entries_from(
            &screensaver_policies
        );


    let wallpaper_fps_policy_entries =
        fps_policy_entries_from(
            &wallpaper_policies
        );


    let wallpaper_fps_policy =
        FpsPolicy {
            global_rendered_fps,
            fps_policy_entries:
                wallpaper_fps_policy_entries,
        };


    let screensaver_speed_policy =
        AnimationSpeedPolicy {
            global_speed:
                screensaver_global_speed,
            shader_policies:
                screensaver_policies.clone(),
        };


    let wallpaper_speed_policy =
        AnimationSpeedPolicy {
            global_speed:
                wallpaper_global_speed,
            shader_policies:
                wallpaper_policies.clone(),
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

            screensaver_policies,

            wallpaper_policies,

            screensaver_speed_policy,

            wallpaper_speed_policy,

            global_rendered_fps,

            screensaver_fps_policy_entries,

            wallpaper_fps_policy,

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
                "[CONFIG] screensaver policy count = {}",
                config.screensaver_policies.len(),
            ),

            format!(
                "[CONFIG] wallpaper policy count = {}",
                config.wallpaper_policies.len(),
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
            "[CONFIG] screensaver_policies count = {}",
            config.screensaver_policies.len(),
        )
    );


    for shader_policy in
        &config.screensaver_policies
    {
        diagnostics.push(
            format_shader_policy_diagnostic(
                "screensaver_policies",
                shader_policy,
            )
        );
    }


    diagnostics.push(
        format!(
            "[CONFIG] wallpaper_policies count = {}",
            config.wallpaper_policies.len(),
        )
    );


    for shader_policy in
        &config.wallpaper_policies
    {
        diagnostics.push(
            format_shader_policy_diagnostic(
                "wallpaper_policies",
                shader_policy,
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
// Per-mode shader policy parsing
// ------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum PolicyTarget {
    Screensaver,
    Wallpaper,
}


impl PolicyTarget {

    fn table_name(
        self,
    ) -> &'static str {

        match self {
            Self::Screensaver => "screensaver_policies",
            Self::Wallpaper => "wallpaper_policies",
        }
    }


    fn speed_range(
        self,
    ) -> (f32, f32) {

        match self {
            Self::Screensaver => (
                crate::define_constants::SCREENSAVER_SPEED_MIN,
                crate::define_constants::SCREENSAVER_SPEED_MAX,
            ),
            Self::Wallpaper => (
                crate::define_constants::WALLPAPER_SPEED_MIN,
                crate::define_constants::WALLPAPER_SPEED_MAX,
            ),
        }
    }
}


fn parse_policy_table(
    raw_policies: BTreeMap<String, String>,
    target: PolicyTarget,
) -> Result<Vec<ShaderPolicy>, String> {

    let mut policies =
        Vec::with_capacity(
            raw_policies.len()
        );


    for (shader, specification) in
        raw_policies
    {
        let shader =
            shader.trim().to_string();


        if shader.is_empty() {
            return Err(
                format!(
                    "[{}] contains an empty shader name",
                    target.table_name(),
                )
            );
        }


        policies.push(
            parse_policy_specification(
                shader,
                &specification,
                target,
            )?
        );
    }


    Ok(policies)
}


fn parse_policy_specification(
    shader: String,
    specification: &str,
    target: PolicyTarget,
) -> Result<ShaderPolicy, String> {

    let mut shader_texture = None;
    let mut shader_palette = None;
    let mut rendered_fps = None;
    let mut animation_speed = None;


    for token in
        specification.split_whitespace()
    {
        let (name, value) =
            token.split_once(':')
                .ok_or_else(
                    || {
                        format!(
                            "Invalid policy token '{}' for '{}' in [{}]; expected name:value",
                            token,
                            shader,
                            target.table_name(),
                        )
                    }
                )?;


        if value.trim().is_empty() {
            return Err(
                format!(
                    "Policy property '{}' for '{}' in [{}] requires a value",
                    name,
                    shader,
                    target.table_name(),
                )
            );
        }


        match name.trim().to_ascii_lowercase().as_str() {
            "texture" => {
                if shader_texture.is_some() {
                    return Err(duplicate_policy_property(
                        &shader,
                        target,
                        "texture",
                    ));
                }

                shader_texture =
                    Some(
                        parse_shader_texture(
                            &shader,
                            value,
                            target.table_name(),
                        )?
                    );
            }

            "palette" => {
                if shader_palette.is_some() {
                    return Err(duplicate_policy_property(
                        &shader,
                        target,
                        "palette",
                    ));
                }

                shader_palette =
                    Some(
                        parse_shader_palette(
                            &shader,
                            value,
                            target.table_name(),
                        )?
                    );
            }

            "fps" => {
                if rendered_fps.is_some() {
                    return Err(duplicate_policy_property(
                        &shader,
                        target,
                        "fps",
                    ));
                }

                let fps =
                    value.parse::<u32>()
                        .map_err(
                            |_| {
                                format!(
                                    "Invalid fps '{}' for '{}' in [{}]; expected an integer from {} through {}",
                                    value,
                                    shader,
                                    target.table_name(),
                                    crate::define_constants::MIN_RENDER_FPS,
                                    crate::define_constants::MAX_RENDER_FPS,
                                )
                            }
                        )?;

                if !(crate::define_constants::MIN_RENDER_FPS
                    ..=crate::define_constants::MAX_RENDER_FPS)
                    .contains(&fps)
                {
                    return Err(
                        format!(
                            "FPS policy {} for '{}' in [{}] is outside the supported range {}-{}",
                            fps,
                            shader,
                            target.table_name(),
                            crate::define_constants::MIN_RENDER_FPS,
                            crate::define_constants::MAX_RENDER_FPS,
                        )
                    );
                }

                rendered_fps = Some(fps);
            }

            "speed" => {
                if animation_speed.is_some() {
                    return Err(duplicate_policy_property(
                        &shader,
                        target,
                        "speed",
                    ));
                }

                let speed =
                    value.parse::<f32>()
                        .map_err(
                            |_| {
                                format!(
                                    "Invalid speed '{}' for '{}' in [{}]",
                                    value,
                                    shader,
                                    target.table_name(),
                                )
                            }
                        )?;

                let (minimum, maximum) =
                    target.speed_range();

                if !speed.is_finite()
                    || !(minimum..=maximum).contains(&speed)
                {
                    return Err(
                        format!(
                            "Speed policy {} for '{}' in [{}] is outside the supported range {}-{}",
                            value,
                            shader,
                            target.table_name(),
                            minimum,
                            maximum,
                        )
                    );
                }

                animation_speed = Some(speed);
            }

            other => {
                return Err(
                    format!(
                        "Unknown policy property '{}' for '{}' in [{}]; supported properties: texture, palette, fps, speed",
                        other,
                        shader,
                        target.table_name(),
                    )
                );
            }
        }
    }


    if shader_texture.is_none()
        && shader_palette.is_none()
        && rendered_fps.is_none()
        && animation_speed.is_none()
    {
        return Err(
            format!(
                "Policy for '{}' in [{}] does not define any properties",
                shader,
                target.table_name(),
            )
        );
    }


    Ok(
        ShaderPolicy {
            shader,
            shader_texture,
            shader_palette,
            rendered_fps,
            animation_speed,
        }
    )
}


fn duplicate_policy_property(
    shader: &str,
    target: PolicyTarget,
    property: &str,
) -> String {

    format!(
        "Policy property '{}' is specified more than once for '{}' in [{}]",
        property,
        shader,
        target.table_name(),
    )
}


fn texture_policy_entries_from(
    shader_policies: &[ShaderPolicy],
) -> Vec<TexturePolicyEntry> {

    shader_policies
        .iter()
        .filter(
            |shader_policy| {
                shader_policy.shader_texture.is_some()
                    || shader_policy.shader_palette.is_some()
            }
        )
        .map(
            |shader_policy| {
                TexturePolicyEntry {
                    shader:
                        shader_policy.shader.clone(),
                    shader_texture:
                        shader_policy.shader_texture.clone(),
                    shader_palette:
                        shader_policy.shader_palette,
                }
            }
        )
        .collect()
}


fn fps_policy_entries_from(
    shader_policies: &[ShaderPolicy],
) -> Vec<FpsPolicyEntry> {

    shader_policies
        .iter()
        .filter_map(
            |shader_policy| {
                shader_policy.rendered_fps
                    .map(
                        |rendered_fps| {
                            FpsPolicyEntry {
                                shader:
                                    shader_policy.shader.clone(),
                                rendered_fps,
                            }
                        }
                    )
            }
        )
        .collect()
}


fn format_shader_policy_diagnostic(
    table_name: &str,
    shader_policy: &ShaderPolicy,
) -> String {

    let texture = shader_policy.shader_texture
        .as_ref()
        .map(format_texture_specification)
        .unwrap_or_else(|| "<global>".to_string());

    let palette = shader_policy.shader_palette
        .map(|palette| palette.to_string())
        .unwrap_or_else(|| "<global>".to_string());

    let fps = shader_policy.rendered_fps
        .map(|fps| fps.to_string())
        .unwrap_or_else(|| "<global>".to_string());

    let speed = shader_policy.animation_speed
        .map(|speed| speed.to_string())
        .unwrap_or_else(|| "<global>".to_string());

    format!(
        "[CONFIG] {} shader={} texture={} palette={} fps={} speed={}",
        table_name,
        shader_policy.shader,
        texture,
        palette,
        fps,
        speed,
    )
}


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
// Per-shader FPS policy parsing
// ------------------------------------------------------------
//


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
// Per-shader texture and palette validation
// ------------------------------------------------------------

fn parse_shader_texture(
    shader: &str,
    value: &str,
    table_name: &str,
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
                "[{}] policy for '{}' requires a specific texture; 'random' is not permitted",
                table_name,
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
                    "Invalid texture '{}' for '{}' in [{}]: {}",
                    value,
                    shader,
                    table_name,
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
    table_name: &str,
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
                "[{}] policy for '{}' requires a specific palette; 'random' is not permitted",
                table_name,
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
                "Invalid palette '{}' for '{}' in [{}]: {}",
                value,
                shader,
                table_name,
                error,
            )
        }
    )
}

