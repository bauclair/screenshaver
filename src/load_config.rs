use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};


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




#[derive(Debug, Deserialize, Default)]
struct PostprocessSection {

    #[serde(default)]
    anti_aliasing: Option<String>,

    #[serde(default)]
    dithering: Option<String>,

    #[serde(default)]
    color_precision: Option<String>,

    #[serde(default)]
    render_scale: Option<f32>,

    #[serde(default)]
    bloom: Option<String>,

    #[serde(default)]
    bloom_intensity: Option<f32>,

    #[serde(default)]
    bloom_threshold: Option<f32>,

    #[serde(default)]
    invert_colors: Option<bool>,

    #[serde(default)]
    flip_horizontal: Option<bool>,

    #[serde(default)]
    flip_vertical: Option<bool>,

    #[serde(default)]
    hue_rotation: Option<f32>,
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
    screensaver_external_paths:
        BTreeMap<String, String>,

    #[serde(default)]
    wallpaper_external_paths:
        BTreeMap<String, String>,

    // Transitional compatibility for the first external-path checkpoint.
    // New writes use *_external_paths exclusively.
    #[serde(default)]
    screensaver_shader_paths:
        BTreeMap<String, String>,

    #[serde(default)]
    wallpaper_shader_paths:
        BTreeMap<String, String>,

    #[serde(default)]
    postprocess: PostprocessSection,

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

    pub policy_key:
        String,

    pub shader:
        String,

    // None means the shader uses the normal managed directory for
    // its policy target. Some(path) identifies an explicitly
    // configured shader stored outside the managed directory.
    pub source_path:
        Option<PathBuf>,

    pub shader_texture:
        Option<
            crate::parse_texture_specification::TextureSpecification
        >,

    pub shader_palette:
        Option<
            crate::palettes::PaletteColor
        >,

    pub rendered_fps:
        Option<u32>,

    pub animation_speed:
        Option<f32>,

    pub anti_aliasing:
        Option<
            crate::render_fxaa::AntiAliasingMethod
        >,

    pub dithering:
        Option<
            crate::render_dithering::DitheringLevel
        >,

    pub color_precision:
        Option<
            crate::select_render_precision::ColorPrecisionPolicy
        >,

    pub render_scale:
        Option<f32>,

    pub bloom:
        Option<
            crate::render_bloom::BloomMode
        >,

    pub bloom_intensity:
        Option<f32>,

    pub bloom_threshold:
        Option<f32>,

    pub invert_colors:
        Option<bool>,

    pub flip_horizontal:
        Option<bool>,

    pub flip_vertical:
        Option<bool>,

    pub hue_rotation:
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
        source_path: Option<&Path>,
        command_line_speed: Option<f32>,
    ) -> f32 {

        if let Some(speed) =
            command_line_speed
        {
            return speed;
        }


        matching_shader_policy(
            &self.shader_policies,
            shader_name,
            source_path,
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


fn paths_refer_to_same_file(
    left: &Path,
    right: &Path,
) -> bool {

    match (
        std::fs::canonicalize(left),
        std::fs::canonicalize(right),
    ) {
        (
            Ok(left),
            Ok(right),
        ) => {
            left == right
        }

        _ => {
            left == right
        }
    }
}


fn managed_policy_name_matches(
    policy_shader: &str,
    shader_name: &str,
    source_path: Option<&Path>,
) -> bool {

    if policy_shader.eq_ignore_ascii_case(
        shader_name
    ) {
        return true;
    }


    source_path
        .and_then(
            |path| {
                path.file_name()
            }
        )
        .and_then(
            |name| {
                name.to_str()
            }
        )
        .is_some_and(
            |filename| {
                policy_shader
                    .eq_ignore_ascii_case(
                        filename
                    )
            }
        )
}


fn matching_shader_policy<'a>(
    policies: &'a [ShaderPolicy],
    shader_name: &str,
    source_path: Option<&Path>,
) -> Option<&'a ShaderPolicy> {

    if let Some(source_path) =
        source_path
    {
        if let Some(policy) =
            policies
                .iter()
                .find(
                    |policy| {
                        policy.source_path
                            .as_deref()
                            .is_some_and(
                                |policy_path| {
                                    paths_refer_to_same_file(
                                        policy_path,
                                        source_path,
                                    )
                                }
                            )
                    }
                )
        {
            return Some(
                policy
            );
        }
    }


    policies
        .iter()
        .find(
            |policy| {
                policy.source_path.is_none()
                    && managed_policy_name_matches(
                        &policy.shader,
                        shader_name,
                        source_path,
                    )
            }
        )
}


fn matching_fps_policy<'a>(
    policies: &'a [FpsPolicyEntry],
    shader_name: &str,
    source_path: Option<&Path>,
) -> Option<&'a FpsPolicyEntry> {

    if let Some(source_path) =
        source_path
    {
        if let Some(policy) =
            policies
                .iter()
                .find(
                    |policy| {
                        policy.source_path
                            .as_deref()
                            .is_some_and(
                                |policy_path| {
                                    paths_refer_to_same_file(
                                        policy_path,
                                        source_path,
                                    )
                                }
                            )
                    }
                )
        {
            return Some(
                policy
            );
        }
    }


    policies
        .iter()
        .find(
            |policy| {
                policy.source_path.is_none()
                    && managed_policy_name_matches(
                        &policy.shader,
                        shader_name,
                        source_path,
                    )
            }
        )
}


#[derive(
    Debug,
    Clone,
)]
pub struct TexturePolicyEntry {

    pub shader:
        String,

    pub source_path:
        Option<PathBuf>,

    pub shader_texture:
        Option<
            crate::parse_texture_specification::TextureSpecification
        >,

    pub shader_palette:
        Option<
            crate::palettes::PaletteColor
        >,
}


#[derive(
    Debug,
    Clone,
)]
pub struct FpsPolicyEntry {

    pub shader:
        String,

    pub source_path:
        Option<PathBuf>,

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
        source_path: Option<&Path>,
        command_line_fps: Option<u32>,
    ) -> u32 {

        if let Some(fps) =
            command_line_fps
        {
            return fps.max(
                1
            );
        }


        matching_fps_policy(
            &self.fps_policy_entries,
            shader_name,
            source_path,
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
            crate::palettes::PaletteColor
        >,

    pub texture_policy_entries:
        Vec<
            TexturePolicyEntry
        >,
}


#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
)]
pub(crate) struct PostprocessProfile {

    pub anti_aliasing:
        crate::render_fxaa::AntiAliasingMethod,

    pub dithering:
        crate::render_dithering::DitheringLevel,

    pub color_precision:
        crate::select_render_precision::ColorPrecisionPolicy,

    pub render_scale:
        f32,

    pub bloom:
        crate::render_bloom::BloomMode,

    pub bloom_intensity:
        f32,

    pub bloom_threshold:
        f32,

    pub invert_colors:
        bool,

    pub flip_horizontal:
        bool,

    pub flip_vertical:
        bool,

    pub hue_rotation:
        f32,
}


impl Default for PostprocessProfile {

    fn default() -> Self {

        Self {

            anti_aliasing:
                crate::render_fxaa::AntiAliasingMethod::Fxaa,

            dithering:
                crate::render_dithering::DitheringLevel::Subtle,

            color_precision:
                crate::select_render_precision::ColorPrecisionPolicy::Auto,

            render_scale:
                crate::define_constants::RENDER_SCALE_DEFAULT,

            bloom:
                crate::render_bloom::BloomMode::Off,

            bloom_intensity:
                crate::render_bloom::BLOOM_INTENSITY_DEFAULT,

            bloom_threshold:
                crate::render_bloom::BLOOM_THRESHOLD_DEFAULT,

            invert_colors:
                false,

            flip_horizontal:
                false,

            flip_vertical:
                false,

            hue_rotation:
                crate::postprocess_shader::HUE_ROTATION_DEFAULT,
        }
    }
}


#[derive(
    Debug,
    Clone,
)]
pub(crate) struct PostprocessPolicy {

    pub global_profile:
        PostprocessProfile,

    pub shader_policies:
        Vec<
            ShaderPolicy
        >,
}


impl Default for PostprocessPolicy {

    fn default() -> Self {

        Self {

            global_profile:
                PostprocessProfile::default(),

            shader_policies:
                Vec::new(),
        }
    }
}


impl PostprocessPolicy {

    pub(crate) fn profile_for_shader(
        &self,
        shader_name: &str,
        source_path: Option<&Path>,
    ) -> PostprocessProfile {

        let shader_policy =
            matching_shader_policy(
                &self.shader_policies,
                shader_name,
                source_path,
            );


        PostprocessProfile {

            anti_aliasing:
                shader_policy
                    .and_then(
                        |shader_policy| {
                            shader_policy.anti_aliasing
                        }
                    )
                    .unwrap_or(
                        self.global_profile.anti_aliasing
                    ),

            dithering:
                shader_policy
                    .and_then(
                        |shader_policy| {
                            shader_policy.dithering
                        }
                    )
                    .unwrap_or(
                        self.global_profile.dithering
                    ),

            color_precision:
                shader_policy
                    .and_then(
                        |shader_policy| {
                            shader_policy.color_precision
                        }
                    )
                    .unwrap_or(
                        self.global_profile.color_precision
                    ),

            render_scale:
                shader_policy
                    .and_then(
                        |shader_policy| {
                            shader_policy.render_scale
                        }
                    )
                    .unwrap_or(
                        self.global_profile.render_scale
                    ),

            bloom:
                shader_policy
                    .and_then(
                        |shader_policy| {
                            shader_policy.bloom
                        }
                    )
                    .unwrap_or(
                        self.global_profile.bloom
                    ),

            bloom_intensity:
                shader_policy
                    .and_then(
                        |shader_policy| {
                            shader_policy.bloom_intensity
                        }
                    )
                    .unwrap_or(
                        self.global_profile.bloom_intensity
                    ),

            bloom_threshold:
                shader_policy
                    .and_then(
                        |shader_policy| {
                            shader_policy.bloom_threshold
                        }
                    )
                    .unwrap_or(
                        self.global_profile.bloom_threshold
                    ),

            invert_colors:
                shader_policy
                    .and_then(|p| p.invert_colors)
                    .unwrap_or(self.global_profile.invert_colors),

            flip_horizontal:
                shader_policy
                    .and_then(|p| p.flip_horizontal)
                    .unwrap_or(self.global_profile.flip_horizontal),

            flip_vertical:
                shader_policy
                    .and_then(|p| p.flip_vertical)
                    .unwrap_or(self.global_profile.flip_vertical),

            hue_rotation:
                shader_policy
                    .and_then(|p| p.hue_rotation)
                    .unwrap_or(self.global_profile.hue_rotation),
        }
    }
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

    pub(crate) screensaver_postprocess_policy:
        PostprocessPolicy,

    pub(crate) wallpaper_postprocess_policy:
        PostprocessPolicy,

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
    // Parse global post-processing policy
    //---------------------------------------------------------

    let built_in_postprocess_profile =
        PostprocessProfile::default();


    let (
        global_anti_aliasing,
        anti_aliasing_warning,
    ) =
        parse_global_anti_aliasing(
            raw.postprocess
                .anti_aliasing
                .as_deref(),
            built_in_postprocess_profile
                .anti_aliasing,
        );


    let (
        global_dithering,
        dithering_warning,
    ) =
        parse_global_dithering(
            raw.postprocess
                .dithering
                .as_deref(),
            built_in_postprocess_profile
                .dithering,
        );


    let (
        global_color_precision,
        color_precision_warning,
    ) =
        parse_global_color_precision(
            raw.postprocess
                .color_precision
                .as_deref(),
            built_in_postprocess_profile
                .color_precision,
        );


    let (
        global_render_scale,
        render_scale_warning,
    ) =
        parse_global_render_scale(
            raw.postprocess
                .render_scale,
            built_in_postprocess_profile
                .render_scale,
        );


    let (
        global_bloom,
        bloom_warning,
    ) =
        parse_global_bloom(
            raw.postprocess
                .bloom
                .as_deref(),
            built_in_postprocess_profile
                .bloom,
        );


    let (
        global_bloom_intensity,
        bloom_intensity_warning,
    ) =
        parse_global_bloom_intensity(
            raw.postprocess
                .bloom_intensity,
            built_in_postprocess_profile
                .bloom_intensity,
        );

    let (
        global_bloom_threshold,
        bloom_threshold_warning,
    ) =
        parse_global_bloom_threshold(
            raw.postprocess
                .bloom_threshold,
            built_in_postprocess_profile
                .bloom_threshold,
        );


    let global_invert_colors =
        raw.postprocess.invert_colors.unwrap_or(false);

    let global_flip_horizontal =
        raw.postprocess.flip_horizontal.unwrap_or(false);

    let global_flip_vertical =
        raw.postprocess.flip_vertical.unwrap_or(false);

    let (
        global_hue_rotation,
        hue_rotation_warning,
    ) =
        parse_global_hue_rotation(
            raw.postprocess.hue_rotation,
            built_in_postprocess_profile.hue_rotation,
        );


    let global_postprocess_profile =
        PostprocessProfile {
            anti_aliasing:
                global_anti_aliasing,
            dithering:
                global_dithering,
            color_precision:
                global_color_precision,
            render_scale:
                global_render_scale,
            bloom:
                global_bloom,
            bloom_intensity:
                global_bloom_intensity,
            bloom_threshold:
                global_bloom_threshold,
            invert_colors:
                global_invert_colors,
            flip_horizontal:
                global_flip_horizontal,
            flip_vertical:
                global_flip_vertical,
            hue_rotation:
                global_hue_rotation,
        };


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


    let screensaver_external_paths =
        merge_external_path_tables(
            raw.screensaver_shader_paths,
            raw.screensaver_external_paths,
        );


    let wallpaper_external_paths =
        merge_external_path_tables(
            raw.wallpaper_shader_paths,
            raw.wallpaper_external_paths,
        );


    let screensaver_policies =
        parse_policy_table(
            raw.screensaver_policies,
            screensaver_external_paths,
            PolicyTarget::Screensaver,
        )?;


    let wallpaper_policies =
        parse_policy_table(
            raw.wallpaper_policies,
            wallpaper_external_paths,
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


    let screensaver_postprocess_policy =
        PostprocessPolicy {
            global_profile:
                global_postprocess_profile,
            shader_policies:
                screensaver_policies.clone(),
        };


    let wallpaper_postprocess_policy =
        PostprocessPolicy {
            global_profile:
                global_postprocess_profile,
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

            screensaver_postprocess_policy,

            wallpaper_postprocess_policy,

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
                            palette.to_hex()
                        }
                    )
                    .unwrap_or_else(
                        || {
                            "random".to_string()
                        }
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
                            palette.to_hex()
                        }
                    )
                    .unwrap_or_else(
                        || {
                            "random".to_string()
                        }
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
                "[CONFIG] postprocess.anti_aliasing = {}",
                config
                    .screensaver_postprocess_policy
                    .global_profile
                    .anti_aliasing
                    .name(),
            ),

            format!(
                "[CONFIG] postprocess.dithering = {}",
                config
                    .screensaver_postprocess_policy
                    .global_profile
                    .dithering
                    .name(),
            ),

            format!(
                "[CONFIG] postprocess.color_precision = {}",
                config
                    .screensaver_postprocess_policy
                    .global_profile
                    .color_precision
                    .name(),
            ),

            format!(
                "[CONFIG] postprocess.render_scale = {:.3}",
                config
                    .screensaver_postprocess_policy
                    .global_profile
                    .render_scale,
            ),

            format!(
                "[CONFIG] postprocess.bloom = {}",
                config
                    .screensaver_postprocess_policy
                    .global_profile
                    .bloom
                    .name(),
            ),

            format!(
                "[CONFIG] postprocess.bloom_intensity = {:.3}",
                config
                    .screensaver_postprocess_policy
                    .global_profile
                    .bloom_intensity,
            ),

            format!(
                "[CONFIG] postprocess.bloom_threshold = {:.3}",
                config
                    .screensaver_postprocess_policy
                    .global_profile
                    .bloom_threshold,
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
        anti_aliasing_warning
    {
        diagnostics.push(
            warning
        );
    }


    if let Some(warning) =
        dithering_warning
    {
        diagnostics.push(
            warning
        );
    }


    if let Some(warning) =
        color_precision_warning
    {
        diagnostics.push(
            warning
        );
    }


    if let Some(warning) =
        render_scale_warning
    {
        diagnostics.push(
            warning
        );
    }


    if let Some(warning) =
        bloom_warning
    {
        diagnostics.push(
            warning
        );
    }


    if let Some(warning) =
        bloom_intensity_warning
    {
        diagnostics.push(
            warning
        );
    }


    if let Some(warning) =
        bloom_threshold_warning
    {
        diagnostics.push(
            warning
        );
    }


    if let Some(warning) =
        hue_rotation_warning
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


    fn path_table_name(
        self,
    ) -> &'static str {

        match self {
            Self::Screensaver => "screensaver_external_paths",
            Self::Wallpaper => "wallpaper_external_paths",
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



fn merge_external_path_tables(
    legacy_paths: BTreeMap<String, String>,
    external_paths: BTreeMap<String, String>,
) -> BTreeMap<String, String> {

    let mut merged =
        legacy_paths;


    for (
        shader,
        path,
    ) in external_paths
    {
        let existing_key =
            merged
                .keys()
                .find(
                    |key| {
                        key.eq_ignore_ascii_case(
                            &shader
                        )
                    }
                )
                .cloned();


        if let Some(existing_key) =
            existing_key
        {
            merged.remove(
                &existing_key
            );
        }


        merged.insert(
            shader,
            path,
        );
    }


    merged
}

fn parse_policy_table(
    raw_policies: BTreeMap<String, String>,
    mut raw_shader_paths: BTreeMap<String, String>,
    target: PolicyTarget,
) -> Result<Vec<ShaderPolicy>, String> {

    let mut policies =
        Vec::with_capacity(
            raw_policies.len()
        );


    for (policy_key, specification) in
        raw_policies
    {
        let policy_key =
            policy_key.trim().to_string();


        if policy_key.is_empty() {
            return Err(
                format!(
                    "[{}] contains an empty shader policy key",
                    target.table_name(),
                )
            );
        }


        let source_path =
            take_policy_source_path(
                &mut raw_shader_paths,
                &policy_key,
                target,
            )?;


        let shader =
            source_path
                .as_ref()
                .and_then(
                    |path| {
                        path.file_name()
                            .and_then(|name| name.to_str())
                    }
                )
                .map(str::to_string)
                .unwrap_or_else(
                    || {
                        crate::manage_policies::policy_display_name_from_key(
                            &policy_key
                        )
                        .to_string()
                    }
                );


        policies.push(
            parse_policy_specification(
                policy_key,
                shader,
                source_path,
                &specification,
                target,
            )?
        );
    }


    // Path entries without matching policies are intentionally inert.
    // This keeps configuration loading tolerant of stale hand-edited
    // metadata while preserving the rule that an external shader cannot
    // enter normal operation without an actual policy.
    Ok(policies)
}


fn take_policy_source_path(
    raw_shader_paths: &mut BTreeMap<String, String>,
    shader: &str,
    target: PolicyTarget,
) -> Result<Option<PathBuf>, String> {

    let matching_key =
        raw_shader_paths
            .keys()
            .find(
                |key| {
                    key.eq_ignore_ascii_case(
                        shader
                    )
                }
            )
            .cloned();


    let Some(matching_key) =
        matching_key
    else {
        return Ok(
            None
        );
    };


    let raw_path =
        raw_shader_paths
            .remove(
                &matching_key
            )
            .unwrap_or_default();


    let raw_path =
        raw_path.trim();


    if raw_path.is_empty() {
        return Err(
            format!(
                "[{}] path for '{}' may not be empty",
                target.path_table_name(),
                shader,
            )
        );
    }


    let path =
        PathBuf::from(
            raw_path
        );


    if !path.is_absolute() {
        return Err(
            format!(
                "[{}] path for '{}' must be absolute: {}",
                target.path_table_name(),
                shader,
                raw_path,
            )
        );
    }


    // Do not require the file to exist while loading configuration.
    // A missing external file must leave its policy visible so the
    // Control Center can report and eventually repair the reference.
    Ok(
        Some(
            path
        )
    )
}

fn parse_policy_specification(
    policy_key: String,
    shader: String,
    source_path: Option<PathBuf>,
    specification: &str,
    target: PolicyTarget,
) -> Result<ShaderPolicy, String> {

    let mut shader_texture = None;
    let mut shader_palette = None;
    let mut rendered_fps = None;
    let mut animation_speed = None;
    let mut anti_aliasing = None;
    let mut dithering = None;
    let mut color_precision = None;
    let mut render_scale = None;
    let mut bloom = None;
    let mut bloom_intensity = None;
    let mut bloom_threshold = None;
    let mut invert_colors = None;
    let mut flip_horizontal = None;
    let mut flip_vertical = None;
    let mut hue_rotation = None;


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

            "anti_aliasing" => {
                if anti_aliasing.is_some() {
                    return Err(duplicate_policy_property(
                        &shader,
                        target,
                        "anti_aliasing",
                    ));
                }

                anti_aliasing =
                    Some(
                        parse_policy_anti_aliasing(
                            &shader,
                            value,
                            target.table_name(),
                        )?
                    );
            }

            "dithering" => {
                if dithering.is_some() {
                    return Err(duplicate_policy_property(
                        &shader,
                        target,
                        "dithering",
                    ));
                }

                dithering =
                    Some(
                        parse_policy_dithering(
                            &shader,
                            value,
                            target.table_name(),
                        )?
                    );
            }

            "color_precision" => {
                if color_precision.is_some() {
                    return Err(duplicate_policy_property(
                        &shader,
                        target,
                        "color_precision",
                    ));
                }

                color_precision =
                    Some(
                        parse_policy_color_precision(
                            &shader,
                            value,
                            target.table_name(),
                        )?
                    );
            }

            "render_scale" => {
                if render_scale.is_some() {
                    return Err(duplicate_policy_property(
                        &shader,
                        target,
                        "render_scale",
                    ));
                }

                render_scale =
                    Some(
                        parse_policy_render_scale(
                            &shader,
                            value,
                            target.table_name(),
                        )?
                    );
            }

            "bloom" => {
                if bloom.is_some() {
                    return Err(duplicate_policy_property(
                        &shader,
                        target,
                        "bloom",
                    ));
                }

                bloom =
                    Some(
                        parse_policy_bloom(
                            &shader,
                            value,
                            target.table_name(),
                        )?
                    );
            }

            "bloom_intensity" => {
                if bloom_intensity.is_some() {
                    return Err(duplicate_policy_property(
                        &shader,
                        target,
                        "bloom_intensity",
                    ));
                }

                bloom_intensity =
                    Some(
                        parse_policy_bloom_intensity(
                            &shader,
                            value,
                            target.table_name(),
                        )?
                    );
            }

            "bloom_threshold" => {
                if bloom_threshold.is_some() {
                    return Err(duplicate_policy_property(
                        &shader,
                        target,
                        "bloom_threshold",
                    ));
                }

                bloom_threshold =
                    Some(
                        parse_policy_bloom_threshold(
                            &shader,
                            value,
                            target.table_name(),
                        )?
                    );
            }
            "invert_colors" => {
                if invert_colors.is_some() {
                    return Err(duplicate_policy_property(&shader, target, "invert_colors"));
                }
                invert_colors = Some(match value.trim().to_ascii_lowercase().as_str() {
                    "true" => true,
                    "false" => false,
                    other => return Err(format!(
                        "Invalid invert_colors '{}' for '{}' in [{}]; expected true or false",
                        other, shader, target.table_name()
                    )),
                });
            }
            "flip_horizontal" => {
                if flip_horizontal.is_some() {
                    return Err(duplicate_policy_property(&shader, target, "flip_horizontal"));
                }
                flip_horizontal = Some(match value.trim().to_ascii_lowercase().as_str() {
                    "true" => true,
                    "false" => false,
                    other => return Err(format!(
                        "Invalid flip_horizontal '{}' for '{}' in [{}]; expected true or false",
                        other, shader, target.table_name()
                    )),
                });
            }
            "flip_vertical" => {
                if flip_vertical.is_some() {
                    return Err(duplicate_policy_property(&shader, target, "flip_vertical"));
                }
                flip_vertical = Some(match value.trim().to_ascii_lowercase().as_str() {
                    "true" => true,
                    "false" => false,
                    other => return Err(format!(
                        "Invalid flip_vertical '{}' for '{}' in [{}]; expected true or false",
                        other, shader, target.table_name()
                    )),
                });
            }
            "hue_rotation" => {
                if hue_rotation.is_some() {
                    return Err(
                        duplicate_policy_property(
                            &shader,
                            target,
                            "hue_rotation",
                        )
                    );
                }

                hue_rotation =
                    Some(
                        parse_policy_hue_rotation(
                            &shader,
                            value,
                            target.table_name(),
                        )?
                    );
            }

            other => {
                return Err(
                    format!(
                        "Unknown policy property '{}' for '{}' in [{}]; supported properties: texture, palette, fps, speed, anti_aliasing, dithering, color_precision, render_scale, bloom, bloom_intensity, bloom_threshold, invert_colors, flip_horizontal, flip_vertical, hue_rotation",
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
        && anti_aliasing.is_none()
        && dithering.is_none()
        && color_precision.is_none()
        && render_scale.is_none()
        && bloom.is_none()
        && bloom_intensity.is_none()
        && bloom_threshold.is_none()
        && invert_colors.is_none()
        && flip_horizontal.is_none()
        && flip_vertical.is_none()
        && hue_rotation.is_none()
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
            policy_key,
            shader,
            source_path,
            shader_texture,
            shader_palette,
            rendered_fps,
            animation_speed,
            anti_aliasing,
            dithering,
            color_precision,
            render_scale,
            bloom,
            bloom_intensity,
            bloom_threshold,
            invert_colors,
            flip_horizontal,
            flip_vertical,
            hue_rotation,
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
                    source_path:
                        shader_policy.source_path.clone(),
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
                                source_path:
                                    shader_policy.source_path.clone(),
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

    let source_path =
        shader_policy
            .source_path
            .as_ref()
            .map(
                |path| {
                    path.display()
                        .to_string()
                }
            )
            .unwrap_or_else(
                || {
                    "<managed>".to_string()
                }
            );

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

    let anti_aliasing = shader_policy.anti_aliasing
        .map(|method| method.name())
        .unwrap_or("<global>");

    let dithering = shader_policy.dithering
        .map(|level| level.name())
        .unwrap_or("<global>");

    let color_precision = shader_policy.color_precision
        .map(|precision| precision.name())
        .unwrap_or("<global>");

    let render_scale = shader_policy.render_scale
        .map(
            |render_scale| {
                format!(
                    "{:.3}",
                    render_scale,
                )
            }
        )
        .unwrap_or_else(
            || {
                "<global>".to_string()
            }
        );

    let bloom = shader_policy.bloom
        .map(
            |mode| {
                mode.name().to_string()
            }
        )
        .unwrap_or_else(
            || {
                "<global>".to_string()
            }
        );

    let bloom_intensity = shader_policy.bloom_intensity
        .map(
            |intensity| {
                format!(
                    "{:.3}",
                    intensity,
                )
            }
        )
        .unwrap_or_else(
            || {
                "<global>".to_string()
            }
        );

    let bloom_threshold = shader_policy.bloom_threshold
        .map(
            |threshold| {
                format!(
                    "{:.3}",
                    threshold,
                )
            }
        )
        .unwrap_or_else(
            || {
                "<global>".to_string()
            }
        );

    format!(
        "[CONFIG] {} shader={} source={} texture={} palette={} fps={} speed={} anti_aliasing={} dithering={} color_precision={} render_scale={} bloom={} bloom_intensity={} bloom_threshold={}",
        table_name,
        shader_policy.shader,
        source_path,
        texture,
        palette,
        fps,
        speed,
        anti_aliasing,
        dithering,
        color_precision,
        render_scale,
        bloom,
        bloom_intensity,
        bloom_threshold,
    )
}


// ------------------------------------------------------------
// Per-shader post-processing policy parsing
// ------------------------------------------------------------
//

fn parse_policy_anti_aliasing(
    shader: &str,
    value: &str,
    table_name: &str,
) -> Result<
    crate::render_fxaa::AntiAliasingMethod,
    String,
> {

    match value
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "off" => {
            Ok(
                crate::render_fxaa::AntiAliasingMethod::Off
            )
        }

        "fxaa" => {
            Ok(
                crate::render_fxaa::AntiAliasingMethod::Fxaa
            )
        }

        _ => {
            Err(
                format!(
                    "Invalid anti_aliasing value '{}' for '{}' in [{}]; supported values: off, fxaa",
                    value,
                    shader,
                    table_name,
                )
            )
        }
    }
}


fn parse_policy_dithering(
    shader: &str,
    value: &str,
    table_name: &str,
) -> Result<
    crate::render_dithering::DitheringLevel,
    String,
> {

    match value
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "off" => {
            Ok(
                crate::render_dithering::DitheringLevel::Off
            )
        }

        "subtle" => {
            Ok(
                crate::render_dithering::DitheringLevel::Subtle
            )
        }

        _ => {
            Err(
                format!(
                    "Invalid dithering value '{}' for '{}' in [{}]; supported values: off, subtle",
                    value,
                    shader,
                    table_name,
                )
            )
        }
    }
}


fn parse_policy_color_precision(
    shader: &str,
    value: &str,
    table_name: &str,
) -> Result<
    crate::select_render_precision::ColorPrecisionPolicy,
    String,
> {

    match value
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "auto" => {
            Ok(
                crate::select_render_precision::ColorPrecisionPolicy::Auto
            )
        }

        "standard" => {
            Ok(
                crate::select_render_precision::ColorPrecisionPolicy::Standard
            )
        }

        "high" => {
            Ok(
                crate::select_render_precision::ColorPrecisionPolicy::High
            )
        }

        _ => {
            Err(
                format!(
                    "Invalid color_precision value '{}' for '{}' in [{}]; supported values: auto, standard, high",
                    value,
                    shader,
                    table_name,
                )
            )
        }
    }
}


fn parse_policy_bloom(
    shader: &str,
    value: &str,
    table_name: &str,
) -> Result<
    crate::render_bloom::BloomMode,
    String,
> {

    crate::render_bloom::BloomMode::parse(
        value
    )
    .map_err(
        |error| {
            format!(
                "Invalid bloom value '{}' for '{}' in [{}]: {}",
                value,
                shader,
                table_name,
                error,
            )
        }
    )
}


fn parse_policy_bloom_intensity(
    shader: &str,
    value: &str,
    table_name: &str,
) -> Result<f32, String> {

    let intensity =
        value.parse::<f32>()
            .map_err(
                |_| {
                    format!(
                        "Invalid bloom_intensity '{}' for '{}' in [{}]; expected a number from {:.2} through {:.2}",
                        value,
                        shader,
                        table_name,
                        crate::render_bloom::BLOOM_INTENSITY_MIN,
                        crate::render_bloom::BLOOM_INTENSITY_MAX,
                    )
                }
            )?;


    crate::render_bloom::validate_bloom_intensity(
        intensity
    )
    .map_err(
        |_| {
            format!(
                "bloom_intensity {} for '{}' in [{}] is outside the supported range {:.2}-{:.2}",
                value,
                shader,
                table_name,
                crate::render_bloom::BLOOM_INTENSITY_MIN,
                crate::render_bloom::BLOOM_INTENSITY_MAX,
            )
        }
    )
}


fn parse_policy_render_scale(
    shader: &str,
    value: &str,
    table_name: &str,
) -> Result<f32, String> {

    let render_scale =
        value.parse::<f32>()
            .map_err(
                |_| {
                    format!(
                        "Invalid render_scale '{}' for '{}' in [{}]; expected a number from {:.2} through {:.2}",
                        value,
                        shader,
                        table_name,
                        crate::define_constants::RENDER_SCALE_MIN,
                        crate::define_constants::RENDER_SCALE_MAX,
                    )
                }
            )?;


    if !render_scale.is_finite()
        || !(crate::define_constants::RENDER_SCALE_MIN
            ..=crate::define_constants::RENDER_SCALE_MAX)
            .contains(
                &render_scale
            )
    {
        return Err(
            format!(
                "render_scale {} for '{}' in [{}] is outside the supported range {:.2}-{:.2}",
                value,
                shader,
                table_name,
                crate::define_constants::RENDER_SCALE_MIN,
                crate::define_constants::RENDER_SCALE_MAX,
            )
        );
    }


    Ok(
        render_scale
    )
}


// ------------------------------------------------------------
// Global post-processing policy parsing
// ------------------------------------------------------------
//

fn parse_global_anti_aliasing(
    value: Option<&str>,
    fallback: crate::render_fxaa::AntiAliasingMethod,
) -> (
    crate::render_fxaa::AntiAliasingMethod,
    Option<String>,
) {

    let Some(value) = value
    else {
        return (
            fallback,
            None,
        );
    };


    match value
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "off" => (
            crate::render_fxaa::AntiAliasingMethod::Off,
            None,
        ),

        "fxaa" => (
            crate::render_fxaa::AntiAliasingMethod::Fxaa,
            None,
        ),

        _ => (
            fallback,
            Some(
                format!(
                    "[CONFIG] WARNING: postprocess.anti_aliasing = '{}' is unsupported; using '{}'",
                    value,
                    fallback.name(),
                )
            ),
        ),
    }
}


fn parse_global_dithering(
    value: Option<&str>,
    fallback: crate::render_dithering::DitheringLevel,
) -> (
    crate::render_dithering::DitheringLevel,
    Option<String>,
) {

    let Some(value) = value
    else {
        return (
            fallback,
            None,
        );
    };


    match value
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "off" => (
            crate::render_dithering::DitheringLevel::Off,
            None,
        ),

        "subtle" => (
            crate::render_dithering::DitheringLevel::Subtle,
            None,
        ),

        _ => (
            fallback,
            Some(
                format!(
                    "[CONFIG] WARNING: postprocess.dithering = '{}' is unsupported; using '{}'",
                    value,
                    fallback.name(),
                )
            ),
        ),
    }
}


fn parse_global_color_precision(
    value: Option<&str>,
    fallback:
        crate::select_render_precision::ColorPrecisionPolicy,
) -> (
    crate::select_render_precision::ColorPrecisionPolicy,
    Option<String>,
) {

    let Some(value) = value
    else {
        return (
            fallback,
            None,
        );
    };


    match value
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "auto" => (
            crate::select_render_precision::ColorPrecisionPolicy::Auto,
            None,
        ),

        "standard" => (
            crate::select_render_precision::ColorPrecisionPolicy::Standard,
            None,
        ),

        "high" => (
            crate::select_render_precision::ColorPrecisionPolicy::High,
            None,
        ),

        _ => (
            fallback,
            Some(
                format!(
                    "[CONFIG] WARNING: postprocess.color_precision = '{}' is unsupported; using '{}'",
                    value,
                    fallback.name(),
                )
            ),
        ),
    }
}


fn parse_policy_hue_rotation(
    shader: &str,
    value: &str,
    table_name: &str,
) -> Result<f32, String> {
    let parsed =
        value.parse::<f32>()
            .map_err(
                |_| {
                    format!(
                        "Invalid hue_rotation '{}' for '{}' in [{}]; expected a number from {:.1} through {:.1}",
                        value,
                        shader,
                        table_name,
                        crate::postprocess_shader::HUE_ROTATION_MIN,
                        crate::postprocess_shader::HUE_ROTATION_MAX,
                    )
                }
            )?;

    crate::postprocess_shader::validate_hue_rotation(parsed)
}


fn parse_policy_bloom_threshold(
    shader: &str,
    value: &str,
    table_name: &str,
) -> Result<f32, String> {

    let parsed =
        value.parse::<f32>()
            .map_err(
                |_| {
                    format!(
                        "Invalid bloom_threshold '{}' for '{}' in [{}]; expected a number from {:.2} through {:.2}",
                        value,
                        shader,
                        table_name,
                        crate::render_bloom::BLOOM_THRESHOLD_MIN,
                        crate::render_bloom::BLOOM_THRESHOLD_MAX,
                    )
                }
            )?;

    crate::render_bloom::validate_bloom_threshold(
        parsed
    )
    .map_err(
        |_| {
            format!(
                "bloom_threshold {} for '{}' in [{}] is outside the supported range {:.2}-{:.2}",
                parsed,
                shader,
                table_name,
                crate::render_bloom::BLOOM_THRESHOLD_MIN,
                crate::render_bloom::BLOOM_THRESHOLD_MAX,
            )
        }
    )
}


fn parse_global_bloom(
    value: Option<&str>,
    fallback: crate::render_bloom::BloomMode,
) -> (
    crate::render_bloom::BloomMode,
    Option<String>,
) {

    let Some(value) = value
    else {
        return (
            fallback,
            None,
        );
    };


    match crate::render_bloom::BloomMode::parse(
        value
    ) {
        Ok(mode) => (
            mode,
            None,
        ),

        Err(_) => (
            fallback,
            Some(
                format!(
                    "[CONFIG] WARNING: postprocess.bloom = '{}' is unsupported; using '{}'",
                    value,
                    fallback.name(),
                )
            ),
        ),
    }
}


fn parse_global_bloom_intensity(
    value: Option<f32>,
    fallback: f32,
) -> (
    f32,
    Option<String>,
) {

    let Some(value) = value
    else {
        return (
            fallback,
            None,
        );
    };


    if crate::render_bloom::validate_bloom_intensity(
        value
    )
    .is_ok()
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
                "[CONFIG] WARNING: postprocess.bloom_intensity = '{}' is outside the supported range {:.2}-{:.2}; using '{:.3}'",
                value,
                crate::render_bloom::BLOOM_INTENSITY_MIN,
                crate::render_bloom::BLOOM_INTENSITY_MAX,
                fallback,
            )
        ),
    )
}


fn parse_global_hue_rotation(
    value: Option<f32>,
    fallback: f32,
) -> (
    f32,
    Option<String>,
) {
    let Some(value) = value
    else {
        return (fallback, None);
    };

    if crate::postprocess_shader::validate_hue_rotation(value).is_ok() {
        return (value, None);
    }

    (
        fallback,
        Some(
            format!(
                "[CONFIG] WARNING: postprocess.hue_rotation = '{}' is outside the supported range {:.1}-{:.1}; using '{:.1}'",
                value,
                crate::postprocess_shader::HUE_ROTATION_MIN,
                crate::postprocess_shader::HUE_ROTATION_MAX,
                fallback,
            )
        ),
    )
}


fn parse_global_bloom_threshold(
    value: Option<f32>,
    fallback: f32,
) -> (
    f32,
    Option<String>,
) {
    let Some(value) = value
    else {
        return (
            fallback,
            None,
        );
    };

    if crate::render_bloom::validate_bloom_threshold(
        value
    )
    .is_ok()
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
                "[CONFIG] WARNING: postprocess.bloom_threshold = '{}' is outside the supported range {:.2}-{:.2}; using '{:.3}'",
                value,
                crate::render_bloom::BLOOM_THRESHOLD_MIN,
                crate::render_bloom::BLOOM_THRESHOLD_MAX,
                fallback,
            )
        ),
    )
}


fn parse_global_render_scale(
    value: Option<f32>,
    fallback: f32,
) -> (
    f32,
    Option<String>,
) {

    let Some(value) = value
    else {
        return (
            fallback,
            None,
        );
    };


    if value.is_finite()
        && (crate::define_constants::RENDER_SCALE_MIN
            ..=crate::define_constants::RENDER_SCALE_MAX)
            .contains(
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
                "[CONFIG] WARNING: postprocess.render_scale = '{}' is outside the supported range {:.2}-{:.2}; using '{:.3}'",
                value,
                crate::define_constants::RENDER_SCALE_MIN,
                crate::define_constants::RENDER_SCALE_MAX,
                fallback,
            )
        ),
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
        crate::palettes::PaletteColor
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


    crate::palettes::PaletteColor::parse_hex(
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
    crate::palettes::PaletteColor,
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


    crate::palettes::PaletteColor::parse_hex(
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

