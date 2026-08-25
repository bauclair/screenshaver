use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};


//
// ------------------------------------------------------------
// TOML startup/recovery configuration
// ------------------------------------------------------------
//

fn default_screensaver_enabled() -> bool {
    true
}


fn default_wallpaper_enabled() -> bool {
    true
}


fn default_log_level() -> u8 {
    5
}

fn default_screen_lock_enabled() -> bool {
    false
}

#[derive(Debug, Deserialize)]
struct ScreensaverSection {

    #[serde(default = "default_screensaver_enabled")]
    enabled: bool,
}


#[derive(Debug, Deserialize)]
struct WallpaperSection {

    #[serde(default = "default_wallpaper_enabled")]
    enabled: bool,

    monitor_mode: String,
}


#[derive(Debug, Deserialize)]
struct LockingSection {

    #[serde(default = "default_screen_lock_enabled")]
    screen_lock_enabled: bool,
}


#[derive(Debug, Deserialize)]
struct DebugSection {

    debug_log: bool,

    #[serde(default = "default_log_level")]
    log_level: u8,
}


#[derive(Debug, Deserialize)]
struct RawToml {

    screensaver: ScreensaverSection,

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
pub struct ShaderPolicy {

    pub policy_id:
        i64,

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

        self.animation_speed_for_policy(
            0,
            shader_name,
            source_path,
            command_line_speed,
        )
    }


    pub fn animation_speed_for_policy(
        &self,
        policy_id: i64,
        shader_name: &str,
        source_path: Option<&Path>,
        command_line_speed: Option<f32>,
    ) -> f32 {

        if let Some(speed) =
            command_line_speed
        {
            return speed;
        }


        matching_shader_policy_by_id(
            &self.shader_policies,
            policy_id,
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


fn matching_shader_policy_by_id<'a>(
    policies: &'a [ShaderPolicy],
    policy_id: i64,
    shader_name: &str,
    source_path: Option<&Path>,
) -> Option<&'a ShaderPolicy> {

    if policy_id > 0 {
        if let Some(policy) =
            policies
                .iter()
                .find(
                    |policy| {
                        policy.policy_id
                            == policy_id
                    }
                )
        {
            return Some(
                policy
            );
        }
    }


    matching_shader_policy(
        policies,
        shader_name,
        source_path,
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


fn matching_fps_policy_by_id<'a>(
    policies: &'a [FpsPolicyEntry],
    policy_id: i64,
    shader_name: &str,
    source_path: Option<&Path>,
) -> Option<&'a FpsPolicyEntry> {

    if policy_id > 0 {
        if let Some(policy) =
            policies
                .iter()
                .find(
                    |policy| {
                        policy.policy_id
                            == policy_id
                    }
                )
        {
            return Some(
                policy
            );
        }
    }


    matching_fps_policy(
        policies,
        shader_name,
        source_path,
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

    pub policy_id:
        i64,

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

    pub policy_id:
        i64,

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

        self.rendered_fps_for_policy(
            0,
            shader_name,
            source_path,
            command_line_fps,
        )
    }


    pub fn rendered_fps_for_policy(
        &self,
        policy_id: i64,
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


        matching_fps_policy_by_id(
            &self.fps_policy_entries,
            policy_id,
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

        self.profile_for_policy(
            0,
            shader_name,
            source_path,
        )
    }


    pub(crate) fn profile_for_policy(
        &self,
        policy_id: i64,
        shader_name: &str,
        source_path: Option<&Path>,
    ) -> PostprocessProfile {

        let shader_policy =
            matching_shader_policy_by_id(
                &self.shader_policies,
                policy_id,
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

    // Control Center/editor staging policies. These are intentionally excluded
    // from all runtime screensaver/wallpaper policy resolution.
    pub unassigned_policies:
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

    pub screen_lock_enabled: bool,

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
    // Load database-backed application / target defaults
    //---------------------------------------------------------

    let app_defaults =
        crate::manage_configuration::load_app_defaults()?;


    let screensaver_defaults =
        crate::manage_configuration::load_target_defaults(
            "screensaver"
        )?;


    let wallpaper_defaults =
        crate::manage_configuration::load_target_defaults(
            "wallpaper"
        )?;


    let built_in_postprocess_profile =
        PostprocessProfile::default();


    let (
        global_anti_aliasing,
        anti_aliasing_warning,
    ) =
        parse_global_anti_aliasing(
            Some(
                &app_defaults.anti_aliasing
            ),
            built_in_postprocess_profile
                .anti_aliasing,
        );


    let (
        global_dithering,
        dithering_warning,
    ) =
        parse_global_dithering(
            Some(
                &app_defaults.dithering
            ),
            built_in_postprocess_profile
                .dithering,
        );


    let (
        global_color_precision,
        color_precision_warning,
    ) =
        parse_global_color_precision(
            Some(
                &app_defaults.color_precision
            ),
            built_in_postprocess_profile
                .color_precision,
        );


    let (
        global_render_scale,
        render_scale_warning,
    ) =
        parse_global_render_scale(
            Some(
                app_defaults.render_scale as f32
            ),
            built_in_postprocess_profile
                .render_scale,
        );


    // Bloom and transform effects remain per-policy. Their inherited
    // fallback is the built-in profile rather than screenshaver.toml.
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
            ..built_in_postprocess_profile
        };


    let parsed_subtitle_placement =
        crate::parse_subtitle_placement::parse(
            Some(
                &app_defaults.subtitle_placement
            )
        );


    let screensaver_global_texture =
        parse_database_global_texture(
            &screensaver_defaults
        )?;


    let screensaver_global_palette =
        parse_database_global_palette(
            &screensaver_defaults
        )?;


    let wallpaper_global_texture =
        parse_database_global_texture(
            &wallpaper_defaults
        )?;


    let wallpaper_global_palette =
        parse_database_global_palette(
            &wallpaper_defaults
        )?;


    // Per-shader policy authority now lives in screenshaver.db.
    // The legacy TOML policy/path tables are intentionally ignored.
    let screensaver_policies =
        load_database_policy_table(
            PolicyTarget::Screensaver
        )?;


    let wallpaper_policies =
        load_database_policy_table(
            PolicyTarget::Wallpaper
        )?;


    let unassigned_policies =
        load_database_policy_table(
            PolicyTarget::Unassigned
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
                app_defaults.wallpaper_notifications,
        };


    let (
        screensaver_global_speed,
        screensaver_global_speed_warning,
    ) =
        validate_animation_speed(
            screensaver_defaults.animation_speed as f32,
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
            wallpaper_defaults.animation_speed as f32,
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
            u32::try_from(
                app_defaults.rendered_fps
            )
            .unwrap_or(0)
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


    let screensaver_mode =
        crate::manage_configuration::load_runtime_mode(
            "screensaver"
        )?;


    let wallpaper_mode =
        crate::manage_configuration::load_runtime_mode(
            "wallpaper"
        )?;


    if raw.locking.screen_lock_enabled {
        validate_screen_lock_screensaver_timeout(
            &screensaver_defaults
        )?;
    }


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
                app_defaults.screensaver_subtitles,

            subtitle_placement:
                parsed_subtitle_placement.placement,

            show_splash:
                app_defaults.show_splash,

            mode:
                screensaver_mode,

            idle_timeout:
                match (
                    screensaver_defaults.idle_timeout_value,
                    screensaver_defaults.idle_timeout_unit.as_deref(),
                ) {
                    (
                        Some(value),
                        Some(unit),
                    ) => {
                        let suffix =
                            match unit {
                                "seconds" => "s",
                                "minutes" => "m",
                                "hours" => "h",
                                _ => "s",
                            };

                        format!(
                            "{}{}",
                            value,
                            suffix,
                        )
                    }

                    _ => {
                        "10m".to_string()
                    }
                },

            texture_policy,

            wallpaper,

            wallpaper_mode:
                wallpaper_mode,

            wallpaper_texture_policy,

            screensaver_global_speed,

            wallpaper_global_speed,

            screensaver_policies,

            wallpaper_policies,

            unassigned_policies,

            screensaver_speed_policy,

            wallpaper_speed_policy,

            global_rendered_fps,

            screensaver_fps_policy_entries,

            wallpaper_fps_policy,

            screensaver_postprocess_policy,

            wallpaper_postprocess_policy,

            screen_lock_enabled:
                raw.locking.screen_lock_enabled,

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
                "[CONFIG] unassigned policy count = {}",
                config.unassigned_policies.len(),
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
                "[CONFIG] screen_lock_enabled = {}",
                config.screen_lock_enabled,
            ),

            format!(
                "[CONFIG] screen_lock uses screensaver idle_timeout = {}",
                config.idle_timeout,
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


    diagnostics.push(
        format!(
            "[CONFIG] unassigned_policies count = {}",
            config.unassigned_policies.len(),
        )
    );


    for shader_policy in
        &config.unassigned_policies
    {
        diagnostics.push(
            format_shader_policy_diagnostic(
                "unassigned_policies",
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
    Unassigned,
}


impl PolicyTarget {

    fn table_name(
        self,
    ) -> &'static str {

        match self {
            Self::Screensaver => "screensaver_policies",
            Self::Wallpaper => "wallpaper_policies",
            Self::Unassigned => "unassigned_policies",
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
            Self::Unassigned => (
                crate::define_constants::SCREENSAVER_SPEED_MIN
                    .min(
                        crate::define_constants::WALLPAPER_SPEED_MIN
                    ),
                crate::define_constants::SCREENSAVER_SPEED_MAX
                    .max(
                        crate::define_constants::WALLPAPER_SPEED_MAX
                    ),
            ),
        }
    }
}



fn load_database_policy_table(
    target: PolicyTarget,
) -> Result<Vec<ShaderPolicy>, String> {

    #[derive(Debug)]
    struct DatabasePolicyRow {
        policy_id: i64,
        policy_name: String,
        filename: String,
        source_path: String,
        texture_mode: Option<String>,
        texture_family: Option<String>,
        texture_primitives: Option<i64>,
        palette_mode: Option<String>,
        palette_color: Option<String>,
        rendered_fps: Option<i64>,
        animation_speed: Option<f64>,
        anti_aliasing: Option<String>,
        dithering: Option<String>,
        color_precision: Option<String>,
        render_scale: Option<f64>,
        bloom_mode: String,
        bloom_intensity: f64,
        bloom_threshold: f64,
        invert_colors: i64,
        flip_horizontal: i64,
        flip_vertical: i64,
        hue_rotation: f64,
    }


    let target_name =
        match target {
            PolicyTarget::Screensaver => "screensaver",
            PolicyTarget::Wallpaper => "wallpaper",
            PolicyTarget::Unassigned => "unassigned",
        };


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open Screenshaver database while loading {} policies: {}",
                        target_name,
                        error,
                    )
                }
            )?;


    let mut statement =
        connection
            .prepare(
                "SELECT
                     p.policy_id,
                     p.policy_name,
                     s.filename,
                     s.source_path,
                     p.texture_mode,
                     p.texture_family,
                     p.texture_primitives,
                     p.palette_mode,
                     p.palette_color,
                     p.rendered_fps,
                     p.animation_speed,
                     p.anti_aliasing,
                     p.dithering,
                     p.color_precision,
                     p.render_scale,
                     p.bloom_mode,
                     p.bloom_intensity,
                     p.bloom_threshold,
                     p.invert_colors,
                     p.flip_horizontal,
                     p.flip_vertical,
                     p.hue_rotation
                 FROM shader_policies AS p
                 JOIN shaders AS s
                   ON s.shader_id = p.shader_id
                 WHERE p.policy_target = ?1
                 ORDER BY p.policy_name_key,
                          p.policy_id"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare {} policy query: {}",
                        target_name,
                        error,
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                rusqlite::params![
                    target_name
                ],
                |row| {
                    Ok(
                        DatabasePolicyRow {
                            policy_id: row.get(0)?,
                            policy_name: row.get(1)?,
                            filename: row.get(2)?,
                            source_path: row.get(3)?,
                            texture_mode: row.get(4)?,
                            texture_family: row.get(5)?,
                            texture_primitives: row.get(6)?,
                            palette_mode: row.get(7)?,
                            palette_color: row.get(8)?,
                            rendered_fps: row.get(9)?,
                            animation_speed: row.get(10)?,
                            anti_aliasing: row.get(11)?,
                            dithering: row.get(12)?,
                            color_precision: row.get(13)?,
                            render_scale: row.get(14)?,
                            bloom_mode: row.get(15)?,
                            bloom_intensity: row.get(16)?,
                            bloom_threshold: row.get(17)?,
                            invert_colors: row.get(18)?,
                            flip_horizontal: row.get(19)?,
                            flip_vertical: row.get(20)?,
                            hue_rotation: row.get(21)?,
                        }
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query {} policies from database: {}",
                        target_name,
                        error,
                    )
                }
            )?;


    let managed_directory =
        crate::locate_paths::shader_dir();


    let mut policies =
        Vec::new();


    for row in rows {

        let row =
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode {} policy row from database: {}",
                        target_name,
                        error,
                    )
                }
            )?;


        let mut tokens =
            Vec::<String>::new();


        match row.texture_mode
            .as_deref()
        {
            None => {}

            Some("random") => {
                tokens.push(
                    "texture:random".to_string()
                );
            }

            Some("specific") => {
                let family =
                    row.texture_family
                        .as_deref()
                        .ok_or_else(
                            || {
                                format!(
                                    "Database policy '{}' specifies texture_mode=specific without texture_family",
                                    row.policy_name,
                                )
                            }
                        )?;

                let primitives =
                    row.texture_primitives
                        .ok_or_else(
                            || {
                                format!(
                                    "Database policy '{}' specifies texture_mode=specific without texture_primitives",
                                    row.policy_name,
                                )
                            }
                        )?;

                tokens.push(
                    format!(
                        "texture:{}:{}",
                        family,
                        primitives,
                    )
                );
            }

            Some(other) => {
                return Err(
                    format!(
                        "Database policy '{}' has unsupported texture_mode '{}'",
                        row.policy_name,
                        other,
                    )
                );
            }
        }


        match row.palette_mode
            .as_deref()
        {
            None => {}

            Some("random") => {
                tokens.push(
                    "palette:random".to_string()
                );
            }

            Some("specific") => {
                let color =
                    row.palette_color
                        .as_deref()
                        .ok_or_else(
                            || {
                                format!(
                                    "Database policy '{}' specifies palette_mode=specific without palette_color",
                                    row.policy_name,
                                )
                            }
                        )?;

                tokens.push(
                    format!(
                        "palette:{}",
                        color,
                    )
                );
            }

            Some(other) => {
                return Err(
                    format!(
                        "Database policy '{}' has unsupported palette_mode '{}'",
                        row.policy_name,
                        other,
                    )
                );
            }
        }


        if let Some(value) =
            row.rendered_fps
        {
            tokens.push(
                format!(
                    "fps:{}",
                    value,
                )
            );
        }


        if let Some(value) =
            row.animation_speed
        {
            tokens.push(
                format!(
                    "speed:{}",
                    value,
                )
            );
        }


        if let Some(value) =
            row.anti_aliasing.as_deref()
        {
            tokens.push(
                format!(
                    "anti_aliasing:{}",
                    value,
                )
            );
        }


        if let Some(value) =
            row.dithering.as_deref()
        {
            tokens.push(
                format!(
                    "dithering:{}",
                    value,
                )
            );
        }


        if let Some(value) =
            row.color_precision.as_deref()
        {
            tokens.push(
                format!(
                    "color_precision:{}",
                    value,
                )
            );
        }


        if let Some(value) =
            row.render_scale
        {
            tokens.push(
                format!(
                    "render_scale:{}",
                    value,
                )
            );
        }


        tokens.push(
            format!(
                "bloom:{}",
                row.bloom_mode,
            )
        );

        tokens.push(
            format!(
                "bloom_intensity:{}",
                row.bloom_intensity,
            )
        );

        tokens.push(
            format!(
                "bloom_threshold:{}",
                row.bloom_threshold,
            )
        );

        tokens.push(
            format!(
                "invert_colors:{}",
                row.invert_colors != 0,
            )
        );

        tokens.push(
            format!(
                "flip_horizontal:{}",
                row.flip_horizontal != 0,
            )
        );

        tokens.push(
            format!(
                "flip_vertical:{}",
                row.flip_vertical != 0,
            )
        );

        tokens.push(
            format!(
                "hue_rotation:{}",
                row.hue_rotation,
            )
        );


        let registered_directory =
            PathBuf::from(
                &row.source_path
            );


        let source_path =
            if registered_directory
                == managed_directory
            {
                None
            } else {
                Some(
                    registered_directory
                        .join(
                            &row.filename
                        )
                )
            };


        policies.push(
            parse_policy_specification(
                row.policy_id,
                row.policy_name,
                row.filename,
                source_path,
                &tokens.join(" "),
                target,
            )?
        );
    }


    Ok(
        policies
    )
}


fn parse_policy_specification(
    policy_id: i64,
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
            policy_id,
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
                    policy_id:
                        shader_policy.policy_id,
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
                                policy_id:
                                    shader_policy.policy_id,
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
                "[CONFIG] WARNING: app_defaults.rendered_fps = {} is outside the supported range {}-{}; using {}",
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

fn parse_database_global_texture(
    defaults: &crate::manage_configuration::TargetDefaults,
) -> Result<Option<crate::parse_texture_specification::TextureSpecification>, String> {

    match defaults.texture_mode.as_str() {
        "random" =>
            parse_global_texture(
                Some(
                    "random"
                )
            ),

        "specific" => {
            let family =
                defaults
                    .texture_family
                    .as_deref()
                    .ok_or_else(
                        || {
                            format!(
                                "Database target defaults for '{}' specify texture_mode=specific without texture_family",
                                defaults.target,
                            )
                        }
                    )?;

            let specification =
                format!(
                    "{}:{}",
                    family,
                    defaults.texture_primitives,
                );

            parse_global_texture(
                Some(
                    &specification
                )
            )
        }

        other =>
            Err(
                format!(
                    "Database target defaults for '{}' contain unsupported texture_mode '{}'",
                    defaults.target,
                    other,
                )
            ),
    }
}


fn parse_database_global_palette(
    defaults: &crate::manage_configuration::TargetDefaults,
) -> Result<Option<crate::palettes::PaletteColor>, String> {

    match defaults.palette_mode.as_str() {
        "random" =>
            parse_global_palette(
                Some(
                    "random"
                )
            ),

        "specific" => {
            let color =
                defaults
                    .palette_color
                    .as_deref()
                    .ok_or_else(
                        || {
                            format!(
                                "Database target defaults for '{}' specify palette_mode=specific without palette_color",
                                defaults.target,
                            )
                        }
                    )?;

            parse_global_palette(
                Some(
                    color
                )
            )
        }

        other =>
            Err(
                format!(
                    "Database target defaults for '{}' contain unsupported palette_mode '{}'",
                    defaults.target,
                    other,
                )
            ),
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



//
// ------------------------------------------------------------
// Screen-lock idle-timeout validation
// ------------------------------------------------------------
//

const SCREEN_LOCK_MIN_IDLE_SECONDS: i64 = 60;


fn validate_screen_lock_screensaver_timeout(
    defaults: &crate::manage_configuration::TargetDefaults,
) -> Result<(), String> {

    let value =
        defaults
            .idle_timeout_value
            .unwrap_or(10);


    let unit =
        defaults
            .idle_timeout_unit
            .as_deref()
            .unwrap_or("minutes");


    if value <= 0 {
        return Err(
            format!(
                "Invalid screensaver idle timeout '{} {}'; screen locking requires a minimum idle timeout of 60 seconds",
                value,
                unit,
            )
        );
    }


    let multiplier =
        match unit {
            "seconds" => 1_i64,
            "minutes" => 60_i64,
            "hours" => 3600_i64,

            other => {
                return Err(
                    format!(
                        "Invalid screensaver idle-timeout unit '{}'; screen locking supports seconds, minutes, or hours",
                        other,
                    )
                );
            }
        };


    let seconds =
        value
            .checked_mul(
                multiplier
            )
            .ok_or_else(
                || {
                    format!(
                        "Invalid screensaver idle timeout '{} {}'; duration is too large",
                        value,
                        unit,
                    )
                }
            )?;


    if seconds < SCREEN_LOCK_MIN_IDLE_SECONDS {
        return Err(
            format!(
                "Invalid screensaver idle timeout '{} {}'; screen locking requires a minimum idle timeout of 60 seconds",
                value,
                unit,
            )
        );
    }


    Ok(())
}
