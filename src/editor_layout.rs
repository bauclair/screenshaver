//! Graphical layout and input handling for the Screenshaver Control Center.
//!
//! This module owns the egui window, controls, layout, styling, and SDL-to-egui
//! input translation. Rendering and shader-session behavior remain in
//! `edit_shader`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::editor_theme::{self, EditorMetrics};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;

// The multi-tabbed editor is intentionally compact so that as much of the
// live shader as possible remains visible around it.  These reference
// dimensions come directly from the approved Qt Designer mock-up.
const EDIT_WINDOW_REFERENCE_WIDTH: f32 =
    645.0;

const EDIT_WINDOW_REFERENCE_HEIGHT_PIXELS: f32 =
    658.0;

const EDIT_WINDOW_REFERENCE_DISPLAY_HEIGHT: f32 =
    1080.0;

const EDIT_WINDOW_SCALE_MIN: f32 =
    0.80;

const EDIT_WINDOW_SCALE_MAX: f32 =
    1.80;

const EDIT_TABBED_CONTENT_HEIGHT: f32 =
    338.0;


#[derive(Clone, Copy)]
struct SliderDragState {
    anchor_value: f32,
    anchor_pointer_x: f32,
    shift_held: bool,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyTarget {
    Screensaver,
    Wallpaper,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorTab {
    Policies,
    Rendering,
    Textures,
    PostProcessing,
    Config,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PolicySortColumn {
    Filename,
    Status,
    Texture,
    PolicyType,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureSelection {
    Marble,
    Clouds,
    Cellular,
    Mesh,
    Radial,
    Noise,
    Bricks,
    Hexagons,
    Facets,
}


impl TextureSelection {
    pub fn from_family(
        family: crate::generate_textures::TextureFamily,
    ) -> Self {
        match family {
            crate::generate_textures::TextureFamily::Marble => Self::Marble,
            crate::generate_textures::TextureFamily::Clouds => Self::Clouds,
            crate::generate_textures::TextureFamily::Cellular => Self::Cellular,
            crate::generate_textures::TextureFamily::Mesh => Self::Mesh,
            crate::generate_textures::TextureFamily::Radial => Self::Radial,
            crate::generate_textures::TextureFamily::Noise => Self::Noise,
            crate::generate_textures::TextureFamily::Bricks => Self::Bricks,
            crate::generate_textures::TextureFamily::Hexagons => Self::Hexagons,
            crate::generate_textures::TextureFamily::Facets => Self::Facets,
        }
    }


    pub fn family(
        self,
    ) -> crate::generate_textures::TextureFamily {
        match self {
            Self::Marble => crate::generate_textures::TextureFamily::Marble,
            Self::Clouds => crate::generate_textures::TextureFamily::Clouds,
            Self::Cellular => crate::generate_textures::TextureFamily::Cellular,
            Self::Mesh => crate::generate_textures::TextureFamily::Mesh,
            Self::Radial => crate::generate_textures::TextureFamily::Radial,
            Self::Noise => crate::generate_textures::TextureFamily::Noise,
            Self::Bricks => crate::generate_textures::TextureFamily::Bricks,
            Self::Hexagons => crate::generate_textures::TextureFamily::Hexagons,
            Self::Facets => crate::generate_textures::TextureFamily::Facets,
        }
    }


    pub fn name(
        self,
    ) -> &'static str {
        self.family().name()
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteSelection {
    Slate,
    Sandstone,
    Lichen,
    Mist,
    Bronze,
    Brick,
}


impl PaletteSelection {
    pub fn from_palette(
        palette: crate::palettes::Palette,
    ) -> Self {
        match palette {
            crate::palettes::Palette::Slate => Self::Slate,
            crate::palettes::Palette::Sandstone => Self::Sandstone,
            crate::palettes::Palette::Lichen => Self::Lichen,
            crate::palettes::Palette::Mist => Self::Mist,
            crate::palettes::Palette::Bronze => Self::Bronze,
            crate::palettes::Palette::Brick => Self::Brick,
        }
    }


    pub fn palette(
        self,
    ) -> crate::palettes::Palette {
        match self {
            Self::Slate => crate::palettes::Palette::Slate,
            Self::Sandstone => crate::palettes::Palette::Sandstone,
            Self::Lichen => crate::palettes::Palette::Lichen,
            Self::Mist => crate::palettes::Palette::Mist,
            Self::Bronze => crate::palettes::Palette::Bronze,
            Self::Brick => crate::palettes::Palette::Brick,
        }
    }


    pub fn name(
        self,
    ) -> &'static str {
        self.palette().name()
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AntiAliasingSelection {
    Off,
    Fxaa,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DitheringSelection {
    Off,
    Subtle,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorPrecisionSelection {
    Automatic,
    High,
    Standard,
}


impl ColorPrecisionSelection {
    pub fn from_policy(
        policy: crate::select_render_precision::ColorPrecisionPolicy,
    ) -> Self {
        match policy {
            crate::select_render_precision::ColorPrecisionPolicy::Auto => {
                Self::Automatic
            }

            crate::select_render_precision::ColorPrecisionPolicy::High => {
                Self::High
            }

            crate::select_render_precision::ColorPrecisionPolicy::Standard => {
                Self::Standard
            }
        }
    }


    pub fn policy(
        self,
    ) -> crate::select_render_precision::ColorPrecisionPolicy {
        match self {
            Self::Automatic => {
                crate::select_render_precision::ColorPrecisionPolicy::Auto
            }

            Self::High => {
                crate::select_render_precision::ColorPrecisionPolicy::High
            }

            Self::Standard => {
                crate::select_render_precision::ColorPrecisionPolicy::Standard
            }
        }
    }


    pub fn display_name(
        self,
    ) -> &'static str {
        match self {
            Self::Automatic => "Automatic",
            Self::High => "High Precision",
            Self::Standard => "Standard Precision",
        }
    }
}


#[derive(Clone, Debug)]
pub struct ShaderInformation {
    pub filename: String,
    pub folder: String,
    pub shader_type: String,
    pub policies: String,
    pub texture_usage: String,
    pub status: String,
}




#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlConfiguration {
    pub screensaver_enabled: bool,
    pub subtitles: bool,
    pub screensaver_display: String,
    pub screensaver_interval_seconds: u64,
    pub screensaver_single_filename: String,
    pub idle_timeout: String,
    pub screensaver_global_texture: String,
    pub screensaver_global_palette: String,
    pub wallpaper_enabled: bool,
    pub notifications: bool,
    pub wallpaper_display: String,
    pub wallpaper_interval_seconds: u64,
    pub wallpaper_single_filename: String,
    pub wallpaper_global_texture: String,
    pub wallpaper_global_palette: String,
}

impl ControlConfiguration {
    pub fn from_config(
        config: &crate::load_config::Config,
    ) -> Self {
        let (
            screensaver_display,
            screensaver_interval_seconds,
            screensaver_single_filename,
        ) =
            split_display_mode(&config.mode);

        let (
            wallpaper_display,
            wallpaper_interval_seconds,
            wallpaper_single_filename,
        ) =
            split_display_mode(&config.wallpaper_mode);

        Self {
            screensaver_enabled: config.screensaver_enabled,
            subtitles: config.subtitles,
            screensaver_display,
            screensaver_interval_seconds,
            screensaver_single_filename,
            idle_timeout: config.idle_timeout.clone(),
            screensaver_global_texture:
                config.texture_policy.global_texture.as_ref()
                    .map(format_texture_specification)
                    .unwrap_or_else(|| "random".to_string()),
            screensaver_global_palette:
                config.texture_policy.global_palette
                    .map(|palette| palette.name().to_string())
                    .unwrap_or_else(|| "random".to_string()),
            wallpaper_enabled: config.wallpaper_enabled,
            notifications: config.wallpaper.notifications,
            wallpaper_display,
            wallpaper_interval_seconds,
            wallpaper_single_filename,
            wallpaper_global_texture:
                config.wallpaper_texture_policy.global_texture.as_ref()
                    .map(format_texture_specification)
                    .unwrap_or_else(|| "random".to_string()),
            wallpaper_global_palette:
                config.wallpaper_texture_policy.global_palette
                    .map(|palette| palette.name().to_string())
                    .unwrap_or_else(|| "random".to_string()),
        }
    }
}

fn split_display_mode(
    mode: &str,
) -> (String, u64, String) {

    const DEFAULT_INTERVAL_SECONDS: u64 =
        600;

    let mut parts =
        mode.splitn(
            2,
            ':'
        );

    let name =
        parts
            .next()
            .unwrap_or(
                "random"
            )
            .trim()
            .to_ascii_lowercase();

    let argument =
        parts
            .next()
            .unwrap_or("")
            .trim();

    match name.as_str() {
        "ordered"
        | "random" => (
            name,
            argument
                .parse::<u64>()
                .ok()
                .filter(
                    |value| {
                        *value > 0
                    }
                )
                .unwrap_or(
                    DEFAULT_INTERVAL_SECONDS
                ),
            String::new(),
        ),

        "single" => (
            "single".to_string(),
            DEFAULT_INTERVAL_SECONDS,
            argument.to_string(),
        ),

        _ => (
            "random".to_string(),
            DEFAULT_INTERVAL_SECONDS,
            String::new(),
        ),
    }
}

fn format_texture_specification(
    texture: &crate::parse_texture_specification::TextureSpecification,
) -> String {
    if texture.count_was_explicit {
        format!(
            "{}:{}",
            texture.family.name(),
            texture.requested_primitive_count,
        )
    } else {
        texture.family.name().to_string()
    }
}

#[derive(Clone, Debug)]
pub struct PolicyDisplayRow {
    pub filename: String,
    pub full_path: String,
    pub accessible: bool,
    pub texture: bool,
    pub policy_target: PolicyTarget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyRowReference {
    pub filename: String,
    pub policy_target: PolicyTarget,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyRowCommand {
    Edit,
    RefreshShader,
    DeletePolicy,
    DeleteShader,
}


#[derive(Clone, Debug)]
struct PendingConfirmation {
    row: PolicyRowReference,
    command: PolicyRowCommand,
}


#[derive(Clone, Debug)]
pub struct EditorOutput {
    pub fps: u32,
    pub animation_speed: f32,
    pub render_scale: f32,
    pub policy_target: Option<PolicyTarget>,
    pub texture: TextureSelection,
    pub palette: PaletteSelection,
    pub primitive_count: u32,
    pub anti_aliasing: AntiAliasingSelection,
    pub dithering: DitheringSelection,
    pub color_precision: ColorPrecisionSelection,
    pub policy_target_change_requested: Option<PolicyTarget>,
    pub save_requested: bool,
    pub cancel_requested: bool,
    pub delete_requested: bool,
    pub browse_shader_requested: bool,
    pub recent_shader_requested: Option<usize>,
    pub clear_recent_files_requested: bool,
    pub refresh_shader_requested: bool,
    pub policy_row_command_requested:
        Option<(PolicyRowReference, PolicyRowCommand)>,
    pub control_configuration: Option<ControlConfiguration>,
    pub control_configuration_dirty: bool,
    pub control_configuration_save_requested: bool,
    pub control_single_browse_requested: Option<PolicyTarget>,
    pub control_single_recent_requested: Option<(PolicyTarget, usize)>,
    pub window_open: bool,
}


#[derive(Clone, Copy)]
struct EditorConfiguration {
    fps: u32,
    animation_speed: f32,
    render_scale: f32,
    policy_target: Option<PolicyTarget>,
    texture: TextureSelection,
    palette: PaletteSelection,
    primitive_count: u32,
    anti_aliasing: AntiAliasingSelection,
    dithering: DitheringSelection,
    color_precision: ColorPrecisionSelection,
}


impl EditorConfiguration {
    fn new(
        fps: u32,
        animation_speed: f32,
        render_scale: f32,
        policy_target: Option<PolicyTarget>,
        texture: TextureSelection,
        palette: PaletteSelection,
        primitive_count: u32,
        anti_aliasing: AntiAliasingSelection,
        dithering: DitheringSelection,
        color_precision: ColorPrecisionSelection,
    ) -> Self {

        Self {
            fps,
            animation_speed:
                normalize_editor_float(
                    animation_speed
                ),
            render_scale:
                normalize_editor_float(
                    render_scale
                ),
            policy_target,
            texture,
            palette,
            primitive_count,
            anti_aliasing,
            dithering,
            color_precision,
        }
    }


    fn differs_from(
        self,
        other: Self,
    ) -> bool {

        self.fps
            != other.fps
            || (
                self.animation_speed
                    - other.animation_speed
            )
                .abs()
                > 0.0001
            || (
                self.render_scale
                    - other.render_scale
            )
                .abs()
                > 0.0001
            || self.policy_target
                != other.policy_target
            || self.texture
                != other.texture
            || self.palette
                != other.palette
            || self.primitive_count
                != other.primitive_count
            || self.anti_aliasing
                != other.anti_aliasing
            || self.dithering
                != other.dithering
            || self.color_precision
                != other.color_precision
    }
}


/// Minimal egui integration for the first graphical-window checkpoint.
///
/// This owns only the egui context, OpenGL painter, pointer events, and the
/// empty movable/resizable window. Shader controls are intentionally omitted.
pub struct EditWindowOverlay {
    context:
        egui::Context,

    painter:
        egui_glow::Painter,

    pending_events:
        Vec<egui::Event>,

    pointer_position:
        egui::Pos2,

    opened_at:
        Instant,

    window_open:
        bool,

    pixels_per_point:
        f32,

    displayed_fps:
        Option<u32>,

    displayed_animation_speed:
        Option<f32>,

    displayed_render_scale:
        Option<f32>,

    initial_configuration:
        Option<EditorConfiguration>,

    policy_target:
        Option<PolicyTarget>,

    status_message:
        String,

    texture:
        TextureSelection,

    palette:
        PaletteSelection,

    primitive_count:
        u32,

    anti_aliasing:
        AntiAliasingSelection,

    dithering:
        DitheringSelection,

    color_precision:
        ColorPrecisionSelection,

    active_tab:
        EditorTab,

    policy_sort_column:
        PolicySortColumn,

    policy_sort_ascending:
        bool,

    selected_policy_row:
        Option<PolicyRowReference>,

    pending_confirmation:
        Option<PendingConfirmation>,

    control_configuration:
        Option<ControlConfiguration>,

    control_configuration_baseline:
        Option<ControlConfiguration>,

    shift_held:
        bool,

    fps_drag_state:
        Option<SliderDragState>,

    animation_speed_drag_state:
        Option<SliderDragState>,

    render_scale_drag_state:
        Option<SliderDragState>,
}


// ============================================================
// MAIN WINDOW
// ============================================================
// Copy/paste replacement boundary for this editor section.

impl EditWindowOverlay {
    pub fn new(
        video: &sdl2::VideoSubsystem,
    ) -> Result<Self, String> {

        let glow_context =
            unsafe {
                glow::Context::from_loader_function(
                    |symbol| {
                        video.gl_get_proc_address(
                            symbol
                        ) as *const _
                    }
                )
            };


        let painter =
            egui_glow::Painter::new(
                Arc::new(
                    glow_context
                ),
                "",
                None,
                false,
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to create egui OpenGL painter: {}",
                        error,
                    )
                }
            )?;


        Ok(
            Self {
                context:
                    egui::Context::default(),

                painter,

                pending_events:
                    Vec::new(),

                pointer_position:
                    egui::Pos2::ZERO,

                opened_at:
                    Instant::now(),

                window_open:
                    true,

                pixels_per_point:
                    1.0,

                displayed_fps:
                    None,

                displayed_animation_speed:
                    None,

                displayed_render_scale:
                    None,

                initial_configuration:
                    None,

                policy_target:
                    None,

                status_message:
                    "Ready".to_string(),

                texture:
                    TextureSelection::Marble,

                palette:
                    PaletteSelection::Slate,

                primitive_count:
                    32,

                anti_aliasing:
                    AntiAliasingSelection::Off,

                dithering:
                    DitheringSelection::Off,

                color_precision:
                    ColorPrecisionSelection::Automatic,

                active_tab:
                    EditorTab::Policies,

                policy_sort_column:
                    PolicySortColumn::Filename,

                policy_sort_ascending:
                    true,

                selected_policy_row:
                    None,

                pending_confirmation:
                    None,

                control_configuration:
                    None,

                control_configuration_baseline:
                    None,

                shift_held:
                    false,

                fps_drag_state:
                    None,

                animation_speed_drag_state:
                    None,

                render_scale_drag_state:
                    None,
            }
        )
    }


    pub fn handle_event(
        &mut self,
        event: &Event,
    ) {

        match *event {
            Event::MouseMotion {
                x,
                y,
                ..
            } => {
                self.pointer_position =
                    egui::pos2(
                        x as f32
                            / self.pixels_per_point,
                        y as f32
                            / self.pixels_per_point,
                    );

                self.pending_events.push(
                    egui::Event::PointerMoved(
                        self.pointer_position
                    )
                );
            }

            Event::MouseButtonDown {
                mouse_btn,
                x,
                y,
                ..
            } => {
                self.push_pointer_button(
                    mouse_btn,
                    x,
                    y,
                    true,
                );
            }

            Event::MouseButtonUp {
                mouse_btn,
                x,
                y,
                ..
            } => {
                self.push_pointer_button(
                    mouse_btn,
                    x,
                    y,
                    false,
                );
            }

            Event::KeyDown {
                keycode:
                    Some(
                        Keycode::LShift
                        | Keycode::RShift
                    ),
                repeat: false,
                ..
            } => {
                self.shift_held =
                    true;
            }

            Event::KeyUp {
                keycode:
                    Some(
                        Keycode::LShift
                        | Keycode::RShift
                    ),
                ..
            } => {
                self.shift_held =
                    false;
            }

            _ => {}
        }
    }


    fn push_pointer_button(
        &mut self,
        mouse_button: MouseButton,
        x: i32,
        y: i32,
        pressed: bool,
    ) {

        let Some(button) =
            egui_pointer_button(
                mouse_button
            )
        else {
            return;
        };


        self.pointer_position =
            egui::pos2(
                x as f32
                    / self.pixels_per_point,
                y as f32
                    / self.pixels_per_point,
            );


        self.pending_events.push(
            egui::Event::PointerButton {
                pos:
                    self.pointer_position,

                button,

                pressed,

                modifiers:
                    egui::Modifiers {
                        shift:
                            self.shift_held,

                        ..Default::default()
                    },
            }
        );
    }


    pub fn display(
        &mut self,
        window: &sdl2::video::Window,
        resolved_fps: u32,
        resolved_animation_speed: f32,
        resolved_render_scale: f32,
        resolved_anti_aliasing: AntiAliasingSelection,
        resolved_dithering: DitheringSelection,
        resolved_color_precision: ColorPrecisionSelection,
        active_texture_selection: Option<(
            crate::parse_texture_specification::TextureSpecification,
            crate::palettes::Palette,
        )>,
        shader_loaded: bool,
        texture_required: bool,
        screensaver_target_available: bool,
        wallpaper_target_available: bool,
        screensaver_target_session_restricted: bool,
        wallpaper_target_session_restricted: bool,
        recent_shader_paths: &[PathBuf],
        shader_information: Option<&ShaderInformation>,
        policy_rows: &[PolicyDisplayRow],
        loaded_configuration: Option<&crate::load_config::Config>,
    ) -> EditorOutput {

        let (
            window_width,
            window_height,
        ) =
            window.size();


        let (
            drawable_width,
            drawable_height,
        ) =
            window.drawable_size();


        let pixels_per_point =
            if window_width == 0 {
                1.0
            } else {
                (
                    drawable_width as f32
                        / window_width as f32
                )
                    .max(1.0)
            };


        let screen_size_points =
            egui::vec2(
                drawable_width as f32
                    / pixels_per_point,
                drawable_height as f32
                    / pixels_per_point,
            );


        self.pixels_per_point =
            pixels_per_point;


        let mut raw_input =
            egui::RawInput::default();

        raw_input.modifiers =
            egui::Modifiers {
                shift:
                    self.shift_held,

                ..Default::default()
            };

        let screen_rect =
            egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                screen_size_points,
            );


        raw_input.screen_rect =
            Some(
                screen_rect
            );


        raw_input.viewports.insert(
            egui::ViewportId::ROOT,
            egui::ViewportInfo {
                native_pixels_per_point:
                    Some(
                        pixels_per_point
                    ),

                inner_rect:
                    Some(
                        screen_rect
                    ),

                outer_rect:
                    Some(
                        screen_rect
                    ),

                ..Default::default()
            },
        );


        raw_input.max_texture_side =
            Some(
                self.painter.max_texture_side()
            );

        raw_input.time =
            Some(
                self.opened_at.elapsed()
                    .as_secs_f64()
            );

        raw_input.events =
            std::mem::take(
                &mut self.pending_events
            );


        let resolution_scale =
            (screen_size_points.y
                / EDIT_WINDOW_REFERENCE_DISPLAY_HEIGHT)
                .clamp(
                    EDIT_WINDOW_SCALE_MIN,
                    EDIT_WINDOW_SCALE_MAX,
                );


        editor_theme::configure_editor_style(
            &self.context,
            resolution_scale,
        );

        let metrics =
            EditorMetrics::new(
                resolution_scale
            );


        let requested_size =
            egui::vec2(
                EDIT_WINDOW_REFERENCE_WIDTH
                    * resolution_scale,
                EDIT_WINDOW_REFERENCE_HEIGHT_PIXELS
                    * resolution_scale,
            );

        let available_size =
            egui::vec2(
                (screen_size_points.x
                    - 24.0 * resolution_scale)
                    .max(320.0),
                (screen_size_points.y
                    - 24.0 * resolution_scale)
                    .max(320.0),
            );

        let initial_size =
            requested_size.min(
                available_size
            );

        // Keep this checkpoint faithful to the Qt mock-up.  Tabs are being
        // introduced specifically to reduce the editor footprint, so the
        // window is deliberately non-resizable for this first format test.
        let minimum_size =
            initial_size;

        let maximum_size =
            initial_size;

        let initial_position =
            egui::pos2(
                ((screen_size_points.x - initial_size.x) * 0.5)
                    .max(0.0),
                ((screen_size_points.y - initial_size.y) * 0.5)
                    .max(0.0),
            );

        let mut window_open =
            self.window_open;

        let mut displayed_fps =
            self.displayed_fps
                .unwrap_or(
                    resolved_fps.clamp(
                        crate::define_constants::MIN_RENDER_FPS,
                        crate::define_constants::MAX_RENDER_FPS,
                    )
                );

        let mut displayed_animation_speed =
            self.displayed_animation_speed
                .unwrap_or(
                    resolved_animation_speed.clamp(
                        crate::define_constants::SCREENSAVER_SPEED_MIN,
                        crate::define_constants::SCREENSAVER_SPEED_MAX,
                    )
                );

        let mut displayed_render_scale =
            self.displayed_render_scale
                .unwrap_or(
                    resolved_render_scale.clamp(
                        crate::define_constants::RENDER_SCALE_MIN,
                        crate::define_constants::RENDER_SCALE_MAX,
                    )
                );


        let mut fps_drag_state =
            self.fps_drag_state;

        let mut animation_speed_drag_state =
            self.animation_speed_drag_state;

        let mut policy_target =
            self.policy_target;

        let mut status_message =
            self.status_message.clone();

        let mut texture =
            self.texture;

        let mut palette =
            self.palette;

        let mut primitive_count =
            self.primitive_count;

        let mut anti_aliasing =
            self.anti_aliasing;

        let mut dithering =
            self.dithering;

        let mut color_precision =
            self.color_precision;


        if self.initial_configuration.is_none() {
            anti_aliasing =
                resolved_anti_aliasing;

            dithering =
                resolved_dithering;

            color_precision =
                resolved_color_precision;

            if let Some((
                specification,
                active_palette,
            )) = active_texture_selection
            {
                texture =
                    TextureSelection::from_family(
                        specification.family
                    );

                palette =
                    PaletteSelection::from_palette(
                        active_palette
                    );

                primitive_count =
                    specification
                        .requested_primitive_count
                        .clamp(
                            2,
                            1024,
                        ) as u32;
            }
        }


        let mut policy_target_change_requested =
            None;

        let mut save_requested =
            false;

        let mut cancel_requested =
            false;

        let mut delete_requested =
            false;

        let mut browse_shader_requested =
            false;

        let mut recent_shader_requested =
            None;

        let mut clear_recent_files_requested =
            false;

        let mut refresh_shader_requested =
            false;


        let baseline_configuration =
            *self.initial_configuration
                .get_or_insert_with(
                    || {
                        EditorConfiguration::new(
                            displayed_fps,
                            displayed_animation_speed,
                            displayed_render_scale,
                            policy_target,
                            texture,
                            palette,
                            primitive_count,
                            anti_aliasing,
                            dithering,
                            color_precision,
                        )
                    }
                );


        let mut render_scale_drag_state =
            self.render_scale_drag_state;

        let shift_held =
            self.shift_held;

        let mut active_tab =
            self.active_tab;

        let mut policy_sort_column =
            self.policy_sort_column;

        let mut policy_sort_ascending =
            self.policy_sort_ascending;

        let mut selected_policy_row =
            self.selected_policy_row.clone();

        let mut pending_confirmation =
            self.pending_confirmation.clone();

        let mut policy_row_command_requested:
            Option<(PolicyRowReference, PolicyRowCommand)> =
            None;

        let mut control_configuration_save_requested =
            false;

        let mut control_single_browse_requested =
            None;

        let mut control_single_recent_requested =
            None;

        if self.control_configuration.is_none() {
            if let Some(config) = loaded_configuration {
                let snapshot = ControlConfiguration::from_config(config);
                self.control_configuration = Some(snapshot.clone());
                self.control_configuration_baseline = Some(snapshot);
            }
        }

        let mut control_configuration =
            self.control_configuration.clone();

        let control_configuration_baseline =
            self.control_configuration_baseline.clone();


        let editor_title =
            "Screenshaver Control Center (ESC or Q to exit)";


        let full_output =
            self.context.run(
                raw_input,
                |context| {
                    let initial_rect =
                        egui::Rect::from_min_size(
                            initial_position,
                            initial_size,
                        );

                    egui::Window::new(
                        editor_title
                    )
                    .open(
                        &mut window_open
                    )
                    .default_open(
                        true
                    )
                    .collapsible(
                        false
                    )
                    .default_rect(
                        initial_rect
                    )
                    .min_size(
                        minimum_size
                    )
                    .max_size(
                        maximum_size
                    )
                    .constrain_to(
                        screen_rect
                    )
                    .resizable(
                        false
                    )
                    .show(
                        context,
                        |ui| {
                            ui.set_enabled(
                                pending_confirmation
                                    .is_none()
                            );

                            let mut hover_help_message:
                                Option<&'static str> =
                                None;

                            let current_configuration =
                                EditorConfiguration::new(
                                    displayed_fps,
                                    displayed_animation_speed,
                                    displayed_render_scale,
                                    policy_target,
                                    texture,
                                    palette,
                                    primitive_count,
                                    anti_aliasing,
                                    dithering,
                                    color_precision,
                                );

                            let configuration_changed =
                                current_configuration
                                    .differs_from(
                                        baseline_configuration
                                    );

                            let mandatory_information_complete =
                                shader_loaded
                                    && policy_target.is_some();

                            let can_save =
                                configuration_changed
                                    && mandatory_information_complete;

                            let can_cancel =
                                configuration_changed;

                            // -----------------------------------------------------------------
                            // Permanent header: Load Shader + Policy Target + Shader Information
                            // -----------------------------------------------------------------
                            draw_compact_header(
                                ui,
                                metrics,
                                shader_information,
                                configuration_changed,
                                recent_shader_paths,
                                policy_target,
                                screensaver_target_available,
                                wallpaper_target_available,
                                screensaver_target_session_restricted,
                                wallpaper_target_session_restricted,
                                &mut browse_shader_requested,
                                &mut recent_shader_requested,
                                &mut clear_recent_files_requested,
                                &mut policy_target_change_requested,
                                &mut status_message,
                                &mut hover_help_message,
                            );

                            ui.add_space(
                                6.0 * metrics.scale
                            );

                            let policy_controls_enabled =
                                policy_target.is_some();

                            // -------------------------------------------------------------
                            // Embedded notebook corresponding to the Qt Designer QTabWidget.
                            // -------------------------------------------------------------
                            draw_editor_tab_bar(
                                ui,
                                metrics,
                                &mut active_tab,
                            );

                            editor_theme::panel_frame(
                                ui,
                                metrics,
                            )
                            .show(
                                ui,
                                |ui| {
                                    ui.set_min_height(
                                        EDIT_TABBED_CONTENT_HEIGHT
                                            * resolution_scale
                                    );

                                    match active_tab {
                                        EditorTab::Policies => {
                                            draw_policies_tab(
                                                ui,
                                                metrics,
                                                policy_rows,
                                                &mut policy_sort_column,
                                                &mut policy_sort_ascending,
                                                &mut selected_policy_row,
                                                &mut pending_confirmation,
                                                &mut policy_row_command_requested,
                                            );
                                        }

                                        EditorTab::Rendering => {
                                            ui.add_enabled_ui(
                                                policy_controls_enabled,
                                                |ui| {
                                                    draw_render_panel(
                                                        ui,
                                                        metrics,
                                                        shift_held,
                                                        &mut displayed_fps,
                                                        &mut displayed_animation_speed,
                                                        &mut displayed_render_scale,
                                                        &mut fps_drag_state,
                                                        &mut animation_speed_drag_state,
                                                        &mut render_scale_drag_state,
                                                        &mut hover_help_message,
                                                    );
                                                },
                                            );
                                        }

                                        EditorTab::Textures => {
                                            ui.add_enabled_ui(
                                                policy_controls_enabled,
                                                |ui| {
                                                    draw_texture_panel(
                                                        ui,
                                                        metrics,
                                                        texture_required,
                                                        &mut texture,
                                                        &mut palette,
                                                        &mut primitive_count,
                                                        &mut hover_help_message,
                                                    );

                                                    ui.add_space(
                                                        8.0 * metrics.scale
                                                    );

                                                    draw_color_picker_placeholder(
                                                        ui,
                                                        metrics,
                                                        palette,
                                                    );
                                                },
                                            );
                                        }

                                        EditorTab::PostProcessing => {
                                            ui.add_enabled_ui(
                                                policy_controls_enabled,
                                                |ui| {
                                                    draw_post_processing_tab(
                                                        ui,
                                                        metrics,
                                                        &mut anti_aliasing,
                                                        &mut dithering,
                                                        &mut color_precision,
                                                        &mut hover_help_message,
                                                    );
                                                },
                                            );
                                        }

                                        EditorTab::Config => {
                                            draw_config_tab(
                                                ui,
                                                metrics,
                                                &mut control_configuration,
                                                control_configuration_baseline.as_ref(),
                                                recent_shader_paths,
                                                &mut clear_recent_files_requested,
                                                &mut control_configuration_save_requested,
                                                &mut control_single_browse_requested,
                                                &mut control_single_recent_requested,
                                                &mut status_message,
                                                &mut hover_help_message,
                                            );
                                        }
                                    }

                                    if !policy_controls_enabled
                                        && active_tab
                                            != EditorTab::Policies
                                        && active_tab
                                            != EditorTab::Config
                                        && hover_help_message.is_none()
                                    {
                                        hover_help_message =
                                            Some(
                                                "Select a Policy Target before changing shader settings."
                                            );
                                    }
                                },
                            );

                            displayed_animation_speed =
                                normalize_editor_float(
                                    displayed_animation_speed
                                );

                            displayed_render_scale =
                                normalize_editor_float(
                                    displayed_render_scale
                                );

                            if !can_save
                                && configuration_changed
                                && policy_target.is_none()
                            {
                                status_message =
                                    "Select a policy target before saving"
                                        .to_string();
                            }

                            ui.add_space(
                                7.0 * metrics.scale
                            );

                            // --------------------------------------------------------
                            // Permanent command row from the approved Qt mock-up.
                            // --------------------------------------------------------
                            draw_compact_action_row(
                                ui,
                                metrics,
                                can_save,
                                can_cancel,
                                &mut save_requested,
                                &mut cancel_requested,
                                &mut displayed_fps,
                                &mut displayed_animation_speed,
                                &mut displayed_render_scale,
                                &mut policy_target,
                                &mut texture,
                                &mut palette,
                                &mut primitive_count,
                                &mut anti_aliasing,
                                &mut dithering,
                                &mut color_precision,
                                baseline_configuration,
                                &mut fps_drag_state,
                                &mut animation_speed_drag_state,
                                &mut render_scale_drag_state,
                                &mut status_message,
                                &mut hover_help_message,
                            );

                            ui.add_space(
                                5.0 * metrics.scale
                            );

                            let displayed_status: &str =
                                match hover_help_message {
                                    Some(message) =>
                                        message,
                                    None =>
                                        status_message.as_str(),
                                };

                            let control_configuration_dirty =
                                control_configuration.as_ref()
                                    .zip(control_configuration_baseline.as_ref())
                                    .map(|(current, baseline)| current != baseline)
                                    .unwrap_or(false);

                            draw_compact_status_row(
                                ui,
                                metrics,
                                shader_information,
                                displayed_status,
                                configuration_changed,
                                control_configuration_dirty,
                            );
                        }
                    );

                    draw_policy_confirmation_modal(
                        context,
                        &mut pending_confirmation,
                        &mut policy_row_command_requested,
                    );
                }
            );


        self.window_open =
            window_open;

        self.active_tab =
            active_tab;

        self.policy_sort_column =
            policy_sort_column;

        self.policy_sort_ascending =
            policy_sort_ascending;

        self.selected_policy_row =
            selected_policy_row;

        self.pending_confirmation =
            pending_confirmation;

        self.control_configuration =
            control_configuration.clone();

        self.displayed_fps =
            Some(
                displayed_fps
            );

        self.displayed_animation_speed =
            Some(
                displayed_animation_speed
            );

        self.displayed_render_scale =
            Some(
                displayed_render_scale
            );

        self.fps_drag_state =
            fps_drag_state;

        self.animation_speed_drag_state =
            animation_speed_drag_state;

        self.render_scale_drag_state =
            render_scale_drag_state;

        self.policy_target =
            policy_target;

        self.status_message =
            status_message;

        self.texture =
            texture;

        self.palette =
            palette;

        self.primitive_count =
            primitive_count;

        self.anti_aliasing =
            anti_aliasing;

        self.dithering =
            dithering;

        self.color_precision =
            color_precision;

        let clipped_primitives =
            self.context.tessellate(
                full_output.shapes,
                full_output.pixels_per_point,
            );


        self.painter.paint_and_update_textures(
            [
                drawable_width,
                drawable_height,
            ],
            full_output.pixels_per_point,
            &clipped_primitives,
            &full_output.textures_delta,
        );


        EditorOutput {
            fps:
                displayed_fps,

            animation_speed:
                displayed_animation_speed,

            render_scale:
                displayed_render_scale,

            policy_target,

            texture,

            palette,

            primitive_count,

            anti_aliasing,

            dithering,

            color_precision,

            policy_target_change_requested,

            save_requested,

            cancel_requested,

            delete_requested,

            browse_shader_requested,

            recent_shader_requested,

            clear_recent_files_requested,

            refresh_shader_requested,

            policy_row_command_requested,

            control_configuration_dirty:
                control_configuration.as_ref()
                    .zip(control_configuration_baseline.as_ref())
                    .map(|(current, baseline)| current != baseline)
                    .unwrap_or(false),

            control_configuration,

            control_configuration_save_requested,

            control_single_browse_requested,

            control_single_recent_requested,

            window_open:
                self.window_open,
        }
    }


    pub fn set_status_message(
        &mut self,
        message: impl Into<String>,
    ) {
        self.status_message =
            message.into();
    }


    pub fn initialize_configuration(
        &mut self,
        fps: u32,
        animation_speed: f32,
        render_scale: f32,
        policy_target: Option<PolicyTarget>,
        anti_aliasing: AntiAliasingSelection,
        dithering: DitheringSelection,
        color_precision: ColorPrecisionSelection,
        active_texture_selection: Option<(
            crate::parse_texture_specification::TextureSpecification,
            crate::palettes::Palette,
        )>,
        status_message: impl Into<String>,
    ) {
        self.displayed_fps =
            Some(
                fps.clamp(
                    crate::define_constants::MIN_RENDER_FPS,
                    crate::define_constants::MAX_RENDER_FPS,
                )
            );

        self.displayed_animation_speed =
            Some(
                normalize_editor_float(
                    animation_speed
                )
            );

        self.displayed_render_scale =
            Some(
                normalize_editor_float(
                    render_scale
                )
            );

        self.policy_target =
            policy_target;

        self.anti_aliasing =
            anti_aliasing;

        self.dithering =
            dithering;

        self.color_precision =
            color_precision;

        if let Some((
            specification,
            palette,
        )) = active_texture_selection
        {
            self.texture =
                TextureSelection::from_family(
                    specification.family
                );

            self.palette =
                PaletteSelection::from_palette(
                    palette
                );

            self.primitive_count =
                specification
                    .requested_primitive_count
                    .clamp(
                        2,
                        1024,
                    ) as u32;
        }

        self.fps_drag_state =
            None;

        self.animation_speed_drag_state =
            None;

        self.render_scale_drag_state =
            None;

        self.status_message =
            status_message.into();

        self.initial_configuration =
            Some(
                EditorConfiguration::new(
                    self.displayed_fps
                        .unwrap_or(fps),
                    self.displayed_animation_speed
                        .unwrap_or(animation_speed),
                    self.displayed_render_scale
                        .unwrap_or(render_scale),
                    self.policy_target,
                    self.texture,
                    self.palette,
                    self.primitive_count,
                    self.anti_aliasing,
                    self.dithering,
                    self.color_precision,
                )
            );
    }


    pub fn accept_current_configuration(
        &mut self,
    ) {
        if let (
            Some(fps),
            Some(animation_speed),
            Some(render_scale),
        ) = (
            self.displayed_fps,
            self.displayed_animation_speed,
            self.displayed_render_scale,
        ) {
            self.initial_configuration =
                Some(
                    EditorConfiguration::new(
                        fps,
                        animation_speed,
                        render_scale,
                        self.policy_target,
                        self.texture,
                        self.palette,
                        self.primitive_count,
                        self.anti_aliasing,
                        self.dithering,
                        self.color_precision,
                    )
                );
        }
    }

    pub fn accept_control_configuration(
        &mut self,
    ) {
        self.control_configuration_baseline =
            self.control_configuration.clone();
    }


    pub fn set_control_single_filename(
        &mut self,
        target: PolicyTarget,
        filename: impl Into<String>,
    ) {
        let Some(configuration) =
            self.control_configuration.as_mut()
        else {
            return;
        };

        let filename =
            filename.into();

        match target {
            PolicyTarget::Screensaver => {
                configuration.screensaver_single_filename =
                    filename;
            }

            PolicyTarget::Wallpaper => {
                configuration.wallpaper_single_filename =
                    filename;
            }
        }
    }



    pub fn destroy(
        &mut self,
    ) {
        self.painter.destroy();
    }
}


fn normalize_editor_float(
    value: f32,
) -> f32 {

    (value * 100.0)
        .round()
        / 100.0
}


fn update_hover_help(
    response: &egui::Response,
    hover_help_message: &mut Option<&'static str>,
    message: &'static str,
) {
    if response.hovered() {
        *hover_help_message =
            Some(message);
    }
}



fn draw_editor_tab_bar(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    active_tab: &mut EditorTab,
) {
    ui.horizontal(
        |ui| {
            for (
                tab,
                label,
            ) in [
                (
                    EditorTab::Policies,
                    "Policies",
                ),
                (
                    EditorTab::Rendering,
                    "Rendering",
                ),
                (
                    EditorTab::Textures,
                    "Textures",
                ),
                (
                    EditorTab::PostProcessing,
                    "Post-Processing",
                ),
                (
                    EditorTab::Config,
                    "Configuration",
                ),
            ] {
                let selected =
                    *active_tab == tab;

                let response =
                    ui.selectable_label(
                        selected,
                        egui::RichText::new(
                            label
                        )
                        .strong(),
                    );

                if response.clicked() {
                    *active_tab =
                        tab;
                }

                ui.add_space(
                    3.0 * metrics.scale
                );
            }
        },
    );
}


#[allow(clippy::too_many_arguments)]
fn draw_compact_header(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    shader_information: Option<&ShaderInformation>,
    configuration_changed: bool,
    recent_shader_paths: &[PathBuf],
    policy_target: Option<PolicyTarget>,
    screensaver_target_available: bool,
    wallpaper_target_available: bool,
    screensaver_target_session_restricted: bool,
    wallpaper_target_session_restricted: bool,
    browse_shader_requested: &mut bool,
    recent_shader_requested: &mut Option<usize>,
    clear_recent_files_requested: &mut bool,
    policy_target_change_requested: &mut Option<PolicyTarget>,
    status_message: &mut String,
    hover_help_message: &mut Option<&'static str>,
) {
    ui.horizontal(
        |ui| {
            ui.scope(
                |ui| {
                    let button_blue =
                        egui::Color32::from_rgb(
                            45,
                            92,
                            155,
                        );

                    {
                        let visuals =
                            ui.visuals_mut();

                        let borderless =
                            egui::Stroke::new(
                                0.0,
                                egui::Color32::TRANSPARENT,
                            );

                        visuals.widgets.inactive.bg_fill =
                            button_blue;
                        visuals.widgets.inactive.bg_stroke =
                            borderless;

                        visuals.widgets.hovered.bg_fill =
                            button_blue;
                        visuals.widgets.hovered.bg_stroke =
                            borderless;

                        visuals.widgets.active.bg_fill =
                            button_blue;
                        visuals.widgets.active.bg_stroke =
                            borderless;

                        visuals.widgets.open.bg_fill =
                            button_blue;
                        visuals.widgets.open.bg_stroke =
                            borderless;

                        visuals.widgets.inactive.fg_stroke.color =
                            egui::Color32::WHITE;

                        visuals.widgets.hovered.fg_stroke.color =
                            egui::Color32::WHITE;

                        visuals.widgets.active.fg_stroke.color =
                            egui::Color32::WHITE;

                        visuals.widgets.open.fg_stroke.color =
                            egui::Color32::WHITE;
                    }

                ui.menu_button(
                    egui::RichText::new(
                        "  Load Shader  "
                    )
                    .strong()
                    .color(
                        egui::Color32::WHITE
                    ),
                    |ui| {
                        if recent_shader_paths.is_empty() {
                            ui.add_enabled(
                                false,
                                egui::Button::new(
                                    "Recent Files (empty)"
                                ),
                            );
                        } else {
                            for (
                                index,
                                path,
                            ) in recent_shader_paths
                                .iter()
                                .enumerate()
                            {
                                let display_name =
                                    path.file_name()
                                        .and_then(
                                            |name| {
                                                name.to_str()
                                            }
                                        )
                                        .unwrap_or(
                                            "Unnamed shader"
                                        );
    
                                let response =
                                    ui.button(
                                        display_name
                                    );
    
                                response
                                    .clone()
                                    .on_hover_text(
                                        path.display()
                                            .to_string()
                                    );
    
                                if response.clicked() {
                                    if configuration_changed {
                                        *status_message =
                                            "Save or cancel the current changes before loading another shader."
                                                .to_string();
                                    } else {
                                        *recent_shader_requested =
                                            Some(index);
                                    }
    
                                    ui.close();
                                }
                            }
                        }
    
                        ui.separator();
    
                        if ui.button(
                            "Browse..."
                        )
                        .clicked()
                        {
                            if configuration_changed {
                                *status_message =
                                    "Save or cancel the current changes before loading another shader."
                                        .to_string();
                            } else {
                                *browse_shader_requested =
                                    true;
                            }
    
                            ui.close();
                        }
    
                        if ui.add_enabled(
                            !recent_shader_paths.is_empty(),
                            egui::Button::new(
                                "Clear Recent Files"
                            ),
                        )
                        .clicked()
                        {
                            *clear_recent_files_requested =
                                true;
    
                            ui.close();
                        }
                    },
                );
                },
            );

            ui.with_layout(
                egui::Layout::right_to_left(
                    egui::Align::Center
                ),
                |ui| {
                    let selected_text =
                        match policy_target {
                            Some(
                                PolicyTarget::Screensaver
                            ) =>
                                "Screensaver",
                            Some(
                                PolicyTarget::Wallpaper
                            ) =>
                                "Wallpaper",
                            None =>
                                "Select...",
                        };

                    egui::ComboBox::from_id_source(
                        "editor_compact_policy_target"
                    )
                    .selected_text(
                        selected_text
                    )
                    .width(
                        130.0 * metrics.scale
                    )
                    .show_ui(
                        ui,
                        |ui| {
                            let screensaver_response =
                                ui.add_enabled(
                                    screensaver_target_available,
                                    egui::SelectableLabel::new(
                                        policy_target
                                            == Some(
                                                PolicyTarget::Screensaver
                                            ),
                                        "Screensaver",
                                    ),
                                );

                            update_hover_help(
                                &screensaver_response,
                                hover_help_message,
                                if screensaver_target_available {
                                    "Load or create the policy used for screensaver rendering."
                                } else if screensaver_target_session_restricted {
                                    "This editing session was opened for the active wallpaper. Only the Wallpaper policy can be edited."
                                } else {
                                    "This shader is unavailable for Screensaver use because it does not exist in the screensavers folder."
                                },
                            );

                            if screensaver_response.clicked()
                                && policy_target
                                    != Some(
                                        PolicyTarget::Screensaver
                                    )
                            {
                                if configuration_changed
                                    && policy_target.is_some()
                                {
                                    *status_message =
                                        "Save or cancel the current changes before switching policy targets."
                                            .to_string();
                                } else {
                                    *policy_target_change_requested =
                                        Some(
                                            PolicyTarget::Screensaver
                                        );
                                }

                                ui.close();
                            }

                            let wallpaper_response =
                                ui.add_enabled(
                                    wallpaper_target_available,
                                    egui::SelectableLabel::new(
                                        policy_target
                                            == Some(
                                                PolicyTarget::Wallpaper
                                            ),
                                        "Wallpaper",
                                    ),
                                );

                            update_hover_help(
                                &wallpaper_response,
                                hover_help_message,
                                if wallpaper_target_available {
                                    "Load or create the policy used for wallpaper rendering."
                                } else if wallpaper_target_session_restricted {
                                    "This editing session was opened for the active screensaver. Only the Screensaver policy can be edited."
                                } else {
                                    "This shader is unavailable for Wallpaper use because it does not exist in the wallpapers folder."
                                },
                            );

                            if wallpaper_response.clicked()
                                && policy_target
                                    != Some(
                                        PolicyTarget::Wallpaper
                                    )
                            {
                                if configuration_changed
                                    && policy_target.is_some()
                                {
                                    *status_message =
                                        "Save or cancel the current changes before switching policy targets."
                                            .to_string();
                                } else {
                                    *policy_target_change_requested =
                                        Some(
                                            PolicyTarget::Wallpaper
                                        );
                                }

                                ui.close();
                            }
                        },
                    );

                    ui.label(
                        egui::RichText::new(
                            "Policy Target:"
                        )
                        .strong(),
                    );
                },
            );
        },
    );

    ui.add_space(
        4.0 * metrics.scale
    );

    let (
        filename,
        folder,
        shader_type,
        policies,
    ) =
        if let Some(information) =
            shader_information
        {
            (
                information.filename.as_str(),
                information.folder.as_str(),
                information.shader_type.as_str(),
                information.policies.as_str(),
            )
        } else {
            (
                "No shader loaded",
                "—",
                "—",
                "None",
            )
        };

    egui::Grid::new(
        "editor_compact_shader_information"
    )
    .num_columns(2)
    .min_col_width(
        82.0 * metrics.scale
    )
    .spacing(
        egui::vec2(
            8.0 * metrics.scale,
            1.0 * metrics.scale,
        )
    )
    .show(
        ui,
        |ui| {
            ui.label("Filename:");
            ui.label(filename);
            ui.end_row();

            ui.label("Folder:");
            let folder_response =
                ui.label(
                    truncate_middle(
                        folder,
                        66,
                    )
                );
            folder_response.on_hover_text(
                folder
            );
            ui.end_row();

            ui.label("Type:");
            ui.label(shader_type);
            ui.end_row();

            ui.label("Policies:");
            ui.label(policies);
            ui.end_row();
        },
    );
}


// ------------------------------------------------------------
// MAIN WINDOW SUPPORT
// ------------------------------------------------------------

fn draw_compact_action_row(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    can_save: bool,
    can_cancel: bool,
    save_requested: &mut bool,
    cancel_requested: &mut bool,
    displayed_fps: &mut u32,
    displayed_animation_speed: &mut f32,
    displayed_render_scale: &mut f32,
    policy_target: &mut Option<PolicyTarget>,
    texture: &mut TextureSelection,
    palette: &mut PaletteSelection,
    primitive_count: &mut u32,
    anti_aliasing: &mut AntiAliasingSelection,
    dithering: &mut DitheringSelection,
    color_precision: &mut ColorPrecisionSelection,
    baseline_configuration: EditorConfiguration,
    fps_drag_state: &mut Option<SliderDragState>,
    animation_speed_drag_state: &mut Option<SliderDragState>,
    render_scale_drag_state: &mut Option<SliderDragState>,
    status_message: &mut String,
    hover_help_message: &mut Option<&'static str>,
) {
    let button_size =
        egui::vec2(
            103.0 * metrics.scale,
            28.0 * metrics.scale,
        );

    ui.horizontal(
        |ui| {
            let save_response =
                ui.add_enabled(
                    can_save,
                    egui::Button::new(
                        "Save Policy"
                    )
                    .min_size(
                        button_size
                    )
                    .fill(
                        egui::Color32::from_rgb(
                            44,
                            126,
                            72,
                        )
                    ),
                );

            update_hover_help(
                &save_response,
                hover_help_message,
                "Save the current per-shader policy.",
            );

            if save_response.clicked() {
                *save_requested =
                    true;

                *status_message =
                    "Saving policy..."
                        .to_string();
            }

            let cancel_response =
                ui.add_enabled(
                    can_cancel,
                    egui::Button::new(
                        "Cancel"
                    )
                    .min_size(
                        egui::vec2(
                            72.0 * metrics.scale,
                            button_size.y,
                        )
                    ),
                );

            update_hover_help(
                &cancel_response,
                hover_help_message,
                "Discard changes made during this editor session.",
            );

            if cancel_response.clicked() {
                *cancel_requested =
                    true;

                *displayed_fps =
                    baseline_configuration.fps;
                *displayed_animation_speed =
                    baseline_configuration.animation_speed;
                *displayed_render_scale =
                    baseline_configuration.render_scale;
                *policy_target =
                    baseline_configuration.policy_target;
                *texture =
                    baseline_configuration.texture;
                *palette =
                    baseline_configuration.palette;
                *primitive_count =
                    baseline_configuration.primitive_count;
                *anti_aliasing =
                    baseline_configuration.anti_aliasing;
                *dithering =
                    baseline_configuration.dithering;
                *color_precision =
                    baseline_configuration.color_precision;
                *fps_drag_state =
                    None;
                *animation_speed_drag_state =
                    None;
                *render_scale_drag_state =
                    None;
                *status_message =
                    "Changes canceled"
                        .to_string();
            }

        },
    );
}


fn draw_compact_status_row(
    ui: &mut egui::Ui,
    _metrics: EditorMetrics,
    shader_information: Option<&ShaderInformation>,
    displayed_status: &str,
    policy_changed: bool,
    control_configuration_dirty: bool,
) {
    ui.separator();

    ui.horizontal(
        |ui| {
            // Left side is reserved exclusively for transient
            // Information / Warning / Error messages.  Suppress steady-state
            // "ready / loaded and rendering" text so it does not compete with
            // the policy-modification status on the right.
            let informational_status =
                if displayed_status
                    .eq_ignore_ascii_case(
                        "Ready"
                    )
                    || displayed_status
                        .to_ascii_lowercase()
                        .contains(
                            "loaded and rendering"
                        )
                {
                    ""
                } else {
                    displayed_status
                };

            ui.label(
                truncate_middle(
                    informational_status,
                    72,
                )
            );

            ui.with_layout(
                egui::Layout::right_to_left(
                    egui::Align::Center
                ),
                |ui| {
                    let policy_text =
                        if shader_information.is_none() {
                            "Policy: --"
                        } else if policy_changed {
                            "Policy: Modified"
                        } else {
                            "Policy: Unchanged"
                        };

                    let config_text =
                        if control_configuration_dirty {
                            "Config: Modified"
                        } else {
                            "Config: Unchanged"
                        };

                    ui.label(
                        egui::RichText::new(
                            format!(
                                "{}   {}",
                                policy_text,
                                config_text,
                            )
                        )
                        .strong(),
                    );
                },
            );
        },
    );
}


fn truncate_middle(
    value: &str,
    maximum_characters: usize,
) -> String {
    let character_count =
        value.chars().count();

    if character_count
        <= maximum_characters
        || maximum_characters < 7
    {
        return value.to_string();
    }

    let retained_characters =
        maximum_characters - 3;

    let leading_characters =
        retained_characters / 2;

    let trailing_characters =
        retained_characters
            - leading_characters;

    let leading =
        value.chars()
            .take(
                leading_characters
            )
            .collect::<String>();

    let trailing =
        value.chars()
            .rev()
            .take(
                trailing_characters
            )
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();

    format!(
        "{}...{}",
        leading,
        trailing,
    )
}


// ======================== END MAIN WINDOW ===================

// ============================================================
// POLICIES TAB
// ============================================================
// Copy/paste replacement boundary for this editor section.
//
// One egui::Grid owns BOTH headers and data.  Every grid cell allocates an
// explicitly left-to-right child UI so neither the header nor data text can
// be centered by add_sized().
//
// Row interaction:
//   single left-click  = select row
//   double left-click  = request Edit Policy
//   right-click        = context menu
//
// Destructive context-menu choices never execute directly.  They open a
// modal confirmation dialog containing the row-specific filename and target.

fn draw_policies_tab(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    policy_rows: &[PolicyDisplayRow],
    sort_column: &mut PolicySortColumn,
    sort_ascending: &mut bool,
    selected_row: &mut Option<PolicyRowReference>,
    pending_confirmation: &mut Option<PendingConfirmation>,
    command_requested:
        &mut Option<(PolicyRowReference, PolicyRowCommand)>,
) {
    fn header_text(
        label: &str,
        column: PolicySortColumn,
        sort_column: PolicySortColumn,
        sort_ascending: bool,
    ) -> String {
        if column == sort_column {
            format!(
                "{} {}",
                label,
                if sort_ascending {
                    "▲"
                } else {
                    "▼"
                },
            )
        } else {
            label.to_string()
        }
    }

    fn apply_sort_request(
        requested_column: PolicySortColumn,
        sort_column: &mut PolicySortColumn,
        sort_ascending: &mut bool,
    ) {
        if *sort_column == requested_column {
            *sort_ascending =
                !*sort_ascending;
        } else {
            *sort_column =
                requested_column;

            *sort_ascending =
                true;
        }
    }

    fn left_aligned_cell(
        ui: &mut egui::Ui,
        width: f32,
        height: f32,
        text: impl Into<egui::WidgetText>,
        sense: egui::Sense,
        selected: bool,
    ) -> egui::Response {
        ui.allocate_ui_with_layout(
            egui::vec2(
                width,
                height,
            ),
            egui::Layout::left_to_right(
                egui::Align::Center
            ),
            |ui| {
                let response =
                    ui.selectable_label(
                        selected,
                        text,
                    );

                if sense
                    == egui::Sense::hover()
                {
                    response
                        .interact(
                            egui::Sense::hover()
                        )
                } else {
                    response
                }
            },
        )
        .inner
    }

    let mut rows =
        policy_rows.to_vec();

    rows.sort_by(
        |left, right| {
            let ordering =
                match *sort_column {
                    PolicySortColumn::Filename =>
                        left.filename
                            .to_ascii_lowercase()
                            .cmp(
                                &right.filename
                                    .to_ascii_lowercase()
                            ),

                    PolicySortColumn::Status =>
                        left.accessible.cmp(
                            &right.accessible
                        ),

                    PolicySortColumn::Texture =>
                        left.texture.cmp(
                            &right.texture
                        ),

                    PolicySortColumn::PolicyType =>
                        policy_target_name(
                            left.policy_target
                        )
                        .cmp(
                            policy_target_name(
                                right.policy_target
                            )
                        ),
                };

            if *sort_ascending {
                ordering
            } else {
                ordering.reverse()
            }
        },
    );

    let spacing =
        10.0 * metrics.scale;

    let usable_width =
        (
            ui.available_width()
                - 18.0 * metrics.scale
                - spacing * 3.0
        )
        .max(
            320.0 * metrics.scale
        );

    let filename_width =
        usable_width * 0.48;

    let status_width =
        usable_width * 0.12;

    let texture_width =
        usable_width * 0.15;

    let policy_width =
        usable_width
            - filename_width
            - status_width
            - texture_width;

    let row_height =
        ui.spacing()
            .interact_size
            .y;

    egui::ScrollArea::vertical()
        .auto_shrink(
            [
                false,
                false,
            ]
        )
        .max_height(
            292.0 * metrics.scale
        )
        .show(
            ui,
            |ui| {
                egui::Grid::new(
                    "editor_policy_table_grid"
                )
                .num_columns(4)
                .spacing(
                    egui::vec2(
                        spacing,
                        3.0 * metrics.scale,
                    )
                )
                .min_col_width(0.0)
                .striped(true)
                .show(
                    ui,
                    |ui| {
                        let filename_header =
                            left_aligned_cell(
                                ui,
                                filename_width,
                                row_height,
                                egui::RichText::new(
                                    header_text(
                                        "Filename",
                                        PolicySortColumn::Filename,
                                        *sort_column,
                                        *sort_ascending,
                                    )
                                )
                                .strong(),
                                egui::Sense::click(),
                                false,
                            );

                        let status_header =
                            left_aligned_cell(
                                ui,
                                status_width,
                                row_height,
                                egui::RichText::new(
                                    header_text(
                                        "Status",
                                        PolicySortColumn::Status,
                                        *sort_column,
                                        *sort_ascending,
                                    )
                                )
                                .strong(),
                                egui::Sense::click(),
                                false,
                            );

                        let texture_header =
                            left_aligned_cell(
                                ui,
                                texture_width,
                                row_height,
                                egui::RichText::new(
                                    header_text(
                                        "Texture",
                                        PolicySortColumn::Texture,
                                        *sort_column,
                                        *sort_ascending,
                                    )
                                )
                                .strong(),
                                egui::Sense::click(),
                                false,
                            );

                        let policy_header =
                            left_aligned_cell(
                                ui,
                                policy_width,
                                row_height,
                                egui::RichText::new(
                                    header_text(
                                        "Policy Type",
                                        PolicySortColumn::PolicyType,
                                        *sort_column,
                                        *sort_ascending,
                                    )
                                )
                                .strong(),
                                egui::Sense::click(),
                                false,
                            );

                        ui.end_row();

                        if filename_header.clicked() {
                            apply_sort_request(
                                PolicySortColumn::Filename,
                                sort_column,
                                sort_ascending,
                            );
                        }

                        if status_header.clicked() {
                            apply_sort_request(
                                PolicySortColumn::Status,
                                sort_column,
                                sort_ascending,
                            );
                        }

                        if texture_header.clicked() {
                            apply_sort_request(
                                PolicySortColumn::Texture,
                                sort_column,
                                sort_ascending,
                            );
                        }

                        if policy_header.clicked() {
                            apply_sort_request(
                                PolicySortColumn::PolicyType,
                                sort_column,
                                sort_ascending,
                            );
                        }

                        if rows.is_empty() {
                            left_aligned_cell(
                                ui,
                                filename_width,
                                row_height,
                                egui::RichText::new(
                                    "No shader policies are currently defined."
                                )
                                .weak(),
                                egui::Sense::hover(),
                                false,
                            );

                            left_aligned_cell(
                                ui,
                                status_width,
                                row_height,
                                "",
                                egui::Sense::hover(),
                                false,
                            );

                            left_aligned_cell(
                                ui,
                                texture_width,
                                row_height,
                                "",
                                egui::Sense::hover(),
                                false,
                            );

                            left_aligned_cell(
                                ui,
                                policy_width,
                                row_height,
                                "",
                                egui::Sense::hover(),
                                false,
                            );

                            ui.end_row();
                            return;
                        }

                        for row in rows {
                            let row_reference =
                                PolicyRowReference {
                                    filename:
                                        row.filename.clone(),

                                    policy_target:
                                        row.policy_target,
                                };

                            let row_selected =
                                selected_row
                                    .as_ref()
                                    .is_some_and(
                                        |selected| {
                                            selected.filename
                                                .eq_ignore_ascii_case(
                                                    &row_reference.filename
                                                )
                                                && selected.policy_target
                                                    == row_reference.policy_target
                                        }
                                    );

                            let filename_response =
                                left_aligned_cell(
                                    ui,
                                    filename_width,
                                    row_height,
                                    row.filename,
                                    egui::Sense::click(),
                                    row_selected,
                                )
                                .on_hover_text(
                                    &row.full_path
                                );

                            let status_response =
                                left_aligned_cell(
                                    ui,
                                    status_width,
                                    row_height,
                                    if row.accessible {
                                        "✅"
                                    } else {
                                        "❌"
                                    },
                                    egui::Sense::click(),
                                    row_selected,
                                )
                                .on_hover_text(
                                    if row.accessible {
                                        format!(
                                            "Shader is accessible:\n{}",
                                            row.full_path,
                                        )
                                    } else {
                                        format!(
                                            "Shader file cannot be accessed:\n{}",
                                            row.full_path,
                                        )
                                    }
                                );

                            let texture_response =
                                left_aligned_cell(
                                    ui,
                                    texture_width,
                                    row_height,
                                    if row.texture {
                                        "Yes"
                                    } else {
                                        "No"
                                    },
                                    egui::Sense::click(),
                                    row_selected,
                                );

                            let policy_response =
                                left_aligned_cell(
                                    ui,
                                    policy_width,
                                    row_height,
                                    policy_target_name(
                                        row.policy_target
                                    ),
                                    egui::Sense::click(),
                                    row_selected,
                                );

                            ui.end_row();

                            let row_clicked =
                                filename_response.clicked()
                                    || status_response.clicked()
                                    || texture_response.clicked()
                                    || policy_response.clicked();

                            let row_double_clicked =
                                filename_response.double_clicked()
                                    || status_response.double_clicked()
                                    || texture_response.double_clicked()
                                    || policy_response.double_clicked();

                            if row_clicked {
                                *selected_row =
                                    Some(
                                        row_reference.clone()
                                    );
                            }

                            if row_double_clicked {
                                *selected_row =
                                    Some(
                                        row_reference.clone()
                                    );

                                *command_requested =
                                    Some(
                                        (
                                            row_reference.clone(),
                                            PolicyRowCommand::Edit,
                                        )
                                    );
                            }

                            // Any cell may open the same row context menu.
                            let mut show_context_menu =
                                |response: &egui::Response| {
                                    response.context_menu(
                                        |ui| {
                                            if ui.button(
                                                "Edit Policy..."
                                            )
                                            .clicked()
                                            {
                                                *selected_row =
                                                    Some(
                                                        row_reference.clone()
                                                    );

                                                *command_requested =
                                                    Some(
                                                        (
                                                            row_reference.clone(),
                                                            PolicyRowCommand::Edit,
                                                        )
                                                    );

                                                ui.close();
                                            }

                                            if ui.button(
                                                "Refresh Shader"
                                            )
                                            .clicked()
                                            {
                                                *selected_row =
                                                    Some(
                                                        row_reference.clone()
                                                    );

                                                *command_requested =
                                                    Some(
                                                        (
                                                            row_reference.clone(),
                                                            PolicyRowCommand::RefreshShader,
                                                        )
                                                    );

                                                ui.close();
                                            }

                                            ui.separator();

                                            if ui.button(
                                                "Delete Policy..."
                                            )
                                            .clicked()
                                            {
                                                *selected_row =
                                                    Some(
                                                        row_reference.clone()
                                                    );

                                                *pending_confirmation =
                                                    Some(
                                                        PendingConfirmation {
                                                            row:
                                                                row_reference.clone(),

                                                            command:
                                                                PolicyRowCommand::DeletePolicy,
                                                        }
                                                    );

                                                ui.close();
                                            }

                                            if ui.button(
                                                "Delete Shader..."
                                            )
                                            .clicked()
                                            {
                                                *selected_row =
                                                    Some(
                                                        row_reference.clone()
                                                    );

                                                *pending_confirmation =
                                                    Some(
                                                        PendingConfirmation {
                                                            row:
                                                                row_reference.clone(),

                                                            command:
                                                                PolicyRowCommand::DeleteShader,
                                                        }
                                                    );

                                                ui.close();
                                            }
                                        },
                                    );
                                };

                            show_context_menu(
                                &filename_response
                            );
                            show_context_menu(
                                &status_response
                            );
                            show_context_menu(
                                &texture_response
                            );
                            show_context_menu(
                                &policy_response
                            );
                        }
                    },
                );
            },
        );
}


fn draw_policy_confirmation_modal(
    context: &egui::Context,
    pending_confirmation: &mut Option<PendingConfirmation>,
    command_requested:
        &mut Option<(PolicyRowReference, PolicyRowCommand)>,
) {
    let Some(confirmation) =
        pending_confirmation.clone()
    else {
        return;
    };

    let target_name =
        policy_target_name(
            confirmation.row.policy_target
        );

    let title =
        match confirmation.command {
            PolicyRowCommand::DeletePolicy =>
                "Delete Policy?",

            PolicyRowCommand::DeleteShader =>
                "Delete Shader?",

            _ =>
                return,
        };

    let mut keep_open =
        true;

    egui::Window::new(
        title
    )
    .id(
        egui::Id::new(
            "editor_destructive_confirmation"
        )
    )
    .collapsible(false)
    .resizable(false)
    .movable(false)
    .anchor(
        egui::Align2::CENTER_CENTER,
        egui::Vec2::ZERO,
    )
    .open(
        &mut keep_open
    )
    .show(
        context,
        |ui| {
            match confirmation.command {
                PolicyRowCommand::DeletePolicy => {
                    ui.label(
                        format!(
                            "Delete the {} policy for:",
                            target_name,
                        )
                    );

                    ui.add_space(6.0);

                    ui.strong(
                        &confirmation.row.filename
                    );

                    ui.add_space(8.0);

                    ui.label(
                        "The shader file will not be deleted."
                    );
                }

                PolicyRowCommand::DeleteShader => {
                    ui.label(
                        format!(
                            "Permanently delete this {} shader:",
                            target_name,
                        )
                    );

                    ui.add_space(6.0);

                    ui.strong(
                        &confirmation.row.filename
                    );

                    ui.add_space(8.0);

                    ui.label(
                        format!(
                            "The associated {} policy will also be deleted.",
                            target_name,
                        )
                    );

                    ui.label(
                        match confirmation.row.policy_target {
                            PolicyTarget::Screensaver =>
                                "Any Wallpaper shader or Wallpaper policy with the same filename will not be changed.",

                            PolicyTarget::Wallpaper =>
                                "Any Screensaver shader or Screensaver policy with the same filename will not be changed.",
                        }
                    );
                }

                _ => {}
            }

            ui.add_space(14.0);

            ui.horizontal(
                |ui| {
                    if ui.button(
                        "Yes"
                    )
                    .clicked()
                    {
                        *command_requested =
                            Some(
                                (
                                    confirmation.row.clone(),
                                    confirmation.command,
                                )
                            );

                        *pending_confirmation =
                            None;
                    }

                    if ui.button(
                        "Cancel"
                    )
                    .clicked()
                    {
                        *pending_confirmation =
                            None;
                    }
                },
            );
        },
    );

    if !keep_open {
        *pending_confirmation =
            None;
    }
}


fn policy_target_name(
    target: PolicyTarget,
) -> &'static str {
    match target {
        PolicyTarget::Screensaver =>
            "Screensaver",

        PolicyTarget::Wallpaper =>
            "Wallpaper",
    }
}


// ======================== END POLICIES TAB ==================

// ============================================================
// RENDERING TAB
// ============================================================
// Copy/paste replacement boundary for this editor section.
//
// Rendering uses one common three-column Grid:
//     description | slider | value
//
// Animation Speed uses a split logarithmic scale.  The midpoint is exactly
// 1.0x.  The left half covers minimum speed through 1.0x; the right half
// covers 1.0x through maximum speed (currently 10.0x).

fn draw_render_panel(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    shift_held: bool,
    displayed_fps: &mut u32,
    displayed_animation_speed: &mut f32,
    displayed_render_scale: &mut f32,
    fps_drag_state: &mut Option<SliderDragState>,
    animation_speed_drag_state: &mut Option<SliderDragState>,
    render_scale_drag_state: &mut Option<SliderDragState>,
    hover_help_message: &mut Option<&'static str>,
) {
    editor_theme::section_heading(
        ui,
        "Render Controls",
    );

    ui.add_space(
        metrics.row_gap
    );

    let horizontal_spacing =
        10.0 * metrics.scale;

    let available_width =
        ui.available_width();

    let label_width =
        128.0 * metrics.scale;

    let value_width =
        74.0 * metrics.scale;

    let slider_width =
        (
            available_width
                - label_width
                - value_width
                - horizontal_spacing * 2.0
        )
        .max(
            180.0 * metrics.scale
        );

    egui::Grid::new(
        "editor_render_controls_grid"
    )
    .num_columns(3)
    .spacing(
        egui::vec2(
            horizontal_spacing,
            metrics.row_gap,
        )
    )
    .min_col_width(0.0)
    .show(
        ui,
        |ui| {
            let mut fps_value =
                *displayed_fps as f32;

            let fps_response =
                draw_aligned_slider_grid_row(
                    ui,
                    "FPS (Max)",
                    &format!(
                        "{} FPS",
                        *displayed_fps,
                    ),
                    &mut fps_value,
                    crate::define_constants::MIN_RENDER_FPS as f32,
                    crate::define_constants::MAX_RENDER_FPS as f32,
                    shift_held,
                    metrics,
                    label_width,
                    slider_width,
                    value_width,
                    fps_drag_state,
                );

            update_hover_help(
                &fps_response,
                hover_help_message,
                "Set the maximum rendering frame rate. Hold Shift for fine adjustment.",
            );

            *displayed_fps =
                fps_value.round()
                    .clamp(
                        crate::define_constants::MIN_RENDER_FPS as f32,
                        crate::define_constants::MAX_RENDER_FPS as f32,
                    ) as u32;

            let speed_response =
                draw_aligned_log_speed_grid_row(
                    ui,
                    "Animation Speed",
                    &format!(
                        "{:.2}x",
                        *displayed_animation_speed,
                    ),
                    displayed_animation_speed,
                    crate::define_constants::SCREENSAVER_SPEED_MIN,
                    crate::define_constants::SCREENSAVER_SPEED_MAX,
                    shift_held,
                    metrics,
                    label_width,
                    slider_width,
                    value_width,
                    animation_speed_drag_state,
                );

            update_hover_help(
                &speed_response,
                hover_help_message,
                "Adjust animation speed on a logarithmic scale. The slider midpoint is 1.0x. Hold Shift for fine adjustment.",
            );

            let scale_response =
                draw_aligned_slider_grid_row(
                    ui,
                    "Render Scale",
                    &format!(
                        "{:.2}x",
                        *displayed_render_scale,
                    ),
                    displayed_render_scale,
                    crate::define_constants::RENDER_SCALE_MIN,
                    crate::define_constants::RENDER_SCALE_MAX,
                    shift_held,
                    metrics,
                    label_width,
                    slider_width,
                    value_width,
                    render_scale_drag_state,
                );

            update_hover_help(
                &scale_response,
                hover_help_message,
                "Change internal rendering resolution. Lower values improve performance; higher values improve quality.",
            );
        },
    );
}


fn draw_aligned_slider_grid_row(
    ui: &mut egui::Ui,
    label: &str,
    displayed_value: &str,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    shift_held: bool,
    metrics: EditorMetrics,
    label_width: f32,
    slider_width: f32,
    value_width: f32,
    drag_state: &mut Option<SliderDragState>,
) -> egui::Response {
    draw_slider_label_cell(
        ui,
        label,
        label_width,
    );

    let response =
        ui.allocate_ui_with_layout(
            egui::vec2(
                slider_width,
                ui.spacing().interact_size.y,
            ),
            egui::Layout::left_to_right(
                egui::Align::Center
            ),
            |ui| {
                ui.set_width(
                    slider_width
                );

                draw_fine_slider(
                    ui,
                    value,
                    minimum,
                    maximum,
                    shift_held,
                    metrics.scale,
                    drag_state,
                )
            },
        )
        .inner;

    draw_slider_value_cell(
        ui,
        displayed_value,
        value_width,
    );

    ui.end_row();

    response
}


fn draw_aligned_log_speed_grid_row(
    ui: &mut egui::Ui,
    label: &str,
    displayed_value: &str,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    shift_held: bool,
    metrics: EditorMetrics,
    label_width: f32,
    slider_width: f32,
    value_width: f32,
    drag_state: &mut Option<SliderDragState>,
) -> egui::Response {
    draw_slider_label_cell(
        ui,
        label,
        label_width,
    );

    let response =
        ui.allocate_ui_with_layout(
            egui::vec2(
                slider_width,
                ui.spacing().interact_size.y,
            ),
            egui::Layout::left_to_right(
                egui::Align::Center
            ),
            |ui| {
                ui.set_width(
                    slider_width
                );

                draw_log_animation_speed_slider(
                    ui,
                    value,
                    minimum,
                    maximum,
                    shift_held,
                    metrics.scale,
                    drag_state,
                )
            },
        )
        .inner;

    draw_slider_value_cell(
        ui,
        displayed_value,
        value_width,
    );

    ui.end_row();

    response
}


fn draw_slider_label_cell(
    ui: &mut egui::Ui,
    label: &str,
    width: f32,
) {
    ui.allocate_ui_with_layout(
        egui::vec2(
            width,
            ui.spacing().interact_size.y,
        ),
        egui::Layout::left_to_right(
            egui::Align::Center
        ),
        |ui| {
            ui.label(label);
        },
    );
}


fn draw_slider_value_cell(
    ui: &mut egui::Ui,
    displayed_value: &str,
    width: f32,
) {
    ui.allocate_ui_with_layout(
        egui::vec2(
            width,
            ui.spacing().interact_size.y,
        ),
        egui::Layout::left_to_right(
            egui::Align::Center
        ),
        |ui| {
            ui.label(
                displayed_value
            );
        },
    );
}


// Standard linear slider used by FPS and Render Scale.

fn draw_fine_slider(
    ui: &mut egui::Ui,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    shift_held: bool,
    resolution_scale: f32,
    drag_state: &mut Option<SliderDragState>,
) -> egui::Response {
    let desired_size =
        egui::vec2(
            ui.available_width(),
            ui.spacing().interact_size.y,
        );

    let (rect, response) =
        ui.allocate_exact_size(
            desired_size,
            egui::Sense::click_and_drag(),
        );

    let pointer_position =
        response.interact_pointer_pos();

    if response.drag_started() {
        if let Some(pointer_position) =
            pointer_position
        {
            *drag_state =
                Some(
                    SliderDragState {
                        anchor_value:
                            *value,

                        anchor_pointer_x:
                            pointer_position.x,

                        shift_held,
                    }
                );
        }
    }

    if response.dragged() {
        if let (
            Some(pointer_position),
            Some(state),
        ) = (
            pointer_position,
            drag_state.as_mut(),
        ) {
            if state.shift_held
                != shift_held
            {
                state.anchor_value =
                    *value;

                state.anchor_pointer_x =
                    pointer_position.x;

                state.shift_held =
                    shift_held;
            }

            let sensitivity =
                if shift_held {
                    0.1
                } else {
                    1.0
                };

            let value_delta =
                (pointer_position.x
                    - state.anchor_pointer_x)
                    / rect.width().max(1.0)
                    * (maximum - minimum)
                    * sensitivity;

            *value =
                (state.anchor_value
                    + value_delta)
                    .clamp(
                        minimum,
                        maximum,
                    );
        }
    } else if response.clicked() {
        if let Some(pointer_position) =
            pointer_position
        {
            let fraction =
                ((pointer_position.x - rect.left())
                    / rect.width().max(1.0))
                    .clamp(
                        0.0,
                        1.0,
                    );

            *value =
                minimum
                    + fraction
                        * (maximum - minimum);
        }
    } else {
        *drag_state =
            None;
    }

    paint_slider(
        ui,
        rect,
        response.clone(),
        ((*value - minimum)
            / (maximum - minimum).max(f32::EPSILON))
            .clamp(
                0.0,
                1.0,
            ),
        resolution_scale,
    );

    response
}


// Animation-speed slider.  Position is piecewise logarithmic:
//   0.00 .. 0.50 = minimum .. 1.0x
//   0.50 .. 1.00 = 1.0x .. maximum

fn draw_log_animation_speed_slider(
    ui: &mut egui::Ui,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    shift_held: bool,
    resolution_scale: f32,
    drag_state: &mut Option<SliderDragState>,
) -> egui::Response {
    let desired_size =
        egui::vec2(
            ui.available_width(),
            ui.spacing().interact_size.y,
        );

    let (rect, response) =
        ui.allocate_exact_size(
            desired_size,
            egui::Sense::click_and_drag(),
        );

    let pointer_position =
        response.interact_pointer_pos();

    let current_fraction =
        animation_speed_to_slider_fraction(
            *value,
            minimum,
            maximum,
        );

    if response.drag_started() {
        if let Some(pointer_position) =
            pointer_position
        {
            *drag_state =
                Some(
                    SliderDragState {
                        // For this slider, anchor_value stores the normalized
                        // slider position rather than the animation speed.
                        anchor_value:
                            current_fraction,

                        anchor_pointer_x:
                            pointer_position.x,

                        shift_held,
                    }
                );
        }
    }

    if response.dragged() {
        if let (
            Some(pointer_position),
            Some(state),
        ) = (
            pointer_position,
            drag_state.as_mut(),
        ) {
            if state.shift_held
                != shift_held
            {
                state.anchor_value =
                    animation_speed_to_slider_fraction(
                        *value,
                        minimum,
                        maximum,
                    );

                state.anchor_pointer_x =
                    pointer_position.x;

                state.shift_held =
                    shift_held;
            }

            let sensitivity =
                if shift_held {
                    0.1
                } else {
                    1.0
                };

            let fraction_delta =
                (pointer_position.x
                    - state.anchor_pointer_x)
                    / rect.width().max(1.0)
                    * sensitivity;

            let new_fraction =
                (state.anchor_value
                    + fraction_delta)
                    .clamp(
                        0.0,
                        1.0,
                    );

            *value =
                animation_speed_from_slider_fraction(
                    new_fraction,
                    minimum,
                    maximum,
                );
        }
    } else if response.clicked() {
        if let Some(pointer_position) =
            pointer_position
        {
            let fraction =
                ((pointer_position.x - rect.left())
                    / rect.width().max(1.0))
                    .clamp(
                        0.0,
                        1.0,
                    );

            *value =
                animation_speed_from_slider_fraction(
                    fraction,
                    minimum,
                    maximum,
                );
        }
    } else {
        *drag_state =
            None;
    }

    paint_slider(
        ui,
        rect,
        response.clone(),
        animation_speed_to_slider_fraction(
            *value,
            minimum,
            maximum,
        ),
        resolution_scale,
    );

    response
}


fn animation_speed_to_slider_fraction(
    value: f32,
    minimum: f32,
    maximum: f32,
) -> f32 {
    let minimum =
        minimum.max(
            f32::MIN_POSITIVE
        );

    let maximum =
        maximum.max(
            1.0
        );

    let value =
        value.clamp(
            minimum,
            maximum,
        );

    if value <= 1.0 {
        if minimum >= 1.0 {
            return 0.5;
        }

        let ratio =
            (value / minimum)
                .ln()
                / (1.0 / minimum)
                    .ln();

        (ratio * 0.5)
            .clamp(
                0.0,
                0.5,
            )
    } else {
        if maximum <= 1.0 {
            return 0.5;
        }

        let ratio =
            value.ln()
                / maximum.ln();

        (0.5 + ratio * 0.5)
            .clamp(
                0.5,
                1.0,
            )
    }
}


fn animation_speed_from_slider_fraction(
    fraction: f32,
    minimum: f32,
    maximum: f32,
) -> f32 {
    let fraction =
        fraction.clamp(
            0.0,
            1.0,
        );

    let minimum =
        minimum.max(
            f32::MIN_POSITIVE
        );

    let maximum =
        maximum.max(
            1.0
        );

    if fraction <= 0.5 {
        if minimum >= 1.0 {
            return 1.0;
        }

        let local =
            fraction / 0.5;

        minimum
            * (1.0 / minimum)
                .powf(
                    local
                )
    } else {
        if maximum <= 1.0 {
            return 1.0;
        }

        let local =
            (fraction - 0.5)
                / 0.5;

        maximum.powf(
            local
        )
    }
}


fn paint_slider(
    ui: &egui::Ui,
    rect: egui::Rect,
    response: egui::Response,
    fraction: f32,
    resolution_scale: f32,
) {
    let track_rect =
        egui::Rect::from_center_size(
            rect.center(),
            egui::vec2(
                rect.width(),
                4.0 * resolution_scale,
            ),
        );

    let widget_visuals =
        if response.dragged() {
            ui.visuals().widgets.active
        } else if response.hovered() {
            ui.visuals().widgets.hovered
        } else {
            ui.visuals().widgets.inactive
        };

    ui.painter().rect_filled(
        track_rect,
        track_rect.height() * 0.5,
        widget_visuals.bg_fill,
    );

    let knob_center =
        egui::pos2(
            egui::lerp(
                rect.left()..=rect.right(),
                fraction.clamp(
                    0.0,
                    1.0,
                ),
            ),
            rect.center().y,
        );

    ui.painter().circle_filled(
        knob_center,
        7.0 * resolution_scale,
        widget_visuals.fg_stroke.color,
    );
}


// ======================== END RENDERING TAB =================

// ============================================================
// TEXTURES TAB
// ============================================================
// Copy/paste replacement boundary for this editor section.
//
// Texture and palette selectors remain compact.  Primitives uses nearly the
// full tab width.  egui's slider_width setting is explicitly overridden here
// because add_sized() alone does not reliably lengthen an egui::Slider.

fn draw_texture_panel(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    texture_required: bool,
    texture: &mut TextureSelection,
    palette: &mut PaletteSelection,
    primitive_count: &mut u32,
    hover_help_message: &mut Option<&'static str>,
) {
    editor_theme::section_heading(
        ui,
        "Texture Settings",
    );

    ui.add_space(
        metrics.row_gap
    );

    ui.add_enabled_ui(
        texture_required,
        |ui| {
            let label_width =
                88.0 * metrics.scale;

            let value_width =
                54.0 * metrics.scale;

            let horizontal_spacing =
                10.0 * metrics.scale;

            // Texture row.
            ui.horizontal(
                |ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(
                            label_width,
                            ui.spacing()
                                .interact_size
                                .y,
                        ),
                        egui::Layout::left_to_right(
                            egui::Align::Center
                        ),
                        |ui| {
                            ui.label(
                                "Texture"
                            );
                        },
                    );

                    let texture_response =
                        egui::ComboBox::from_id_source(
                            "editor_texture_selection"
                        )
                        .selected_text(
                            texture.name()
                        )
                        .width(
                            metrics.dropdown_width
                        )
                        .show_ui(
                            ui,
                            |ui| {
                                // Resource lists are presented
                                // alphanumerically.
                                for selection in [
                                    TextureSelection::Bricks,
                                    TextureSelection::Cellular,
                                    TextureSelection::Clouds,
                                    TextureSelection::Facets,
                                    TextureSelection::Hexagons,
                                    TextureSelection::Marble,
                                    TextureSelection::Mesh,
                                    TextureSelection::Noise,
                                    TextureSelection::Radial,
                                ] {
                                    ui.selectable_value(
                                        texture,
                                        selection,
                                        selection.name(),
                                    );
                                }
                            },
                        )
                        .response;

                    update_hover_help(
                        &texture_response,
                        hover_help_message,
                        "Select the procedural texture family generated in memory for this shader.",
                    );
                },
            );

            ui.add_space(
                metrics.row_gap
            );

            // Palette row.
            ui.horizontal(
                |ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(
                            label_width,
                            ui.spacing()
                                .interact_size
                                .y,
                        ),
                        egui::Layout::left_to_right(
                            egui::Align::Center
                        ),
                        |ui| {
                            ui.label(
                                "Palette"
                            );
                        },
                    );

                    let palette_response =
                        egui::ComboBox::from_id_source(
                            "editor_palette_selection"
                        )
                        .selected_text(
                            palette.name()
                        )
                        .width(
                            metrics.dropdown_width
                        )
                        .show_ui(
                            ui,
                            |ui| {
                                // Resource lists are presented
                                // alphanumerically.
                                for selection in [
                                    PaletteSelection::Brick,
                                    PaletteSelection::Bronze,
                                    PaletteSelection::Lichen,
                                    PaletteSelection::Mist,
                                    PaletteSelection::Sandstone,
                                    PaletteSelection::Slate,
                                ] {
                                    ui.selectable_value(
                                        palette,
                                        selection,
                                        selection.name(),
                                    );
                                }
                            },
                        )
                        .response;

                    update_hover_help(
                        &palette_response,
                        hover_help_message,
                        "Select the built-in palette applied to the generated procedural texture.",
                    );
                },
            );

            ui.add_space(
                metrics.row_gap
            );

            // Primitives row.
            const PRIMITIVE_VALUES: [u32; 10] =
                [
                    2,
                    4,
                    8,
                    16,
                    32,
                    64,
                    128,
                    256,
                    512,
                    1024,
                ];

            let current_index =
                PRIMITIVE_VALUES
                    .iter()
                    .position(
                        |value| {
                            value
                                == primitive_count
                        }
                    )
                    .unwrap_or(0);

            let mut primitive_index =
                current_index as f32;

            let primitive_slider_width =
                (
                    ui.available_width()
                        - label_width
                        - value_width
                        - horizontal_spacing * 2.0
                )
                .max(
                    260.0 * metrics.scale
                );

            let mut primitive_response =
                None;

            ui.horizontal(
                |ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(
                            label_width,
                            ui.spacing()
                                .interact_size
                                .y,
                        ),
                        egui::Layout::left_to_right(
                            egui::Align::Center
                        ),
                        |ui| {
                            ui.label(
                                "Primitives"
                            );
                        },
                    );

                    ui.allocate_ui_with_layout(
                        egui::vec2(
                            primitive_slider_width,
                            ui.spacing()
                                .interact_size
                                .y,
                        ),
                        egui::Layout::left_to_right(
                            egui::Align::Center
                        ),
                        |ui| {
                            ui.spacing_mut()
                                .slider_width =
                                primitive_slider_width;

                            primitive_response =
                                Some(
                                    ui.add(
                                        egui::Slider::new(
                                            &mut primitive_index,
                                            0.0..=(
                                                PRIMITIVE_VALUES.len()
                                                    - 1
                                            ) as f32,
                                        )
                                        .show_value(false),
                                    )
                                );
                        },
                    );

                    let selected_index =
                        primitive_index
                            .round()
                            .clamp(
                                0.0,
                                (
                                    PRIMITIVE_VALUES.len()
                                        - 1
                                ) as f32,
                            ) as usize;

                    *primitive_count =
                        PRIMITIVE_VALUES[
                            selected_index
                        ];

                    ui.allocate_ui_with_layout(
                        egui::vec2(
                            value_width,
                            ui.spacing()
                                .interact_size
                                .y,
                        ),
                        egui::Layout::left_to_right(
                            egui::Align::Center
                        ),
                        |ui| {
                            ui.label(
                                primitive_count
                                    .to_string()
                            );
                        },
                    );
                },
            );

            if let Some(primitive_response) =
                primitive_response
            {
                update_hover_help(
                    &primitive_response,
                    hover_help_message,
                    "Set the number of graphical elements used to generate the procedural texture.",
                );
            }
        },
    );

    if !texture_required {
        ui.label(
            egui::RichText::new(
                "Not required by this shader"
            )
            .weak(),
        );
    }
}


// Texture-tab custom palette placeholder.

fn draw_color_picker_placeholder(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    palette: PaletteSelection,
) {
    ui.separator();

    ui.horizontal(
        |ui| {
            ui.label(
                "Palette Color:"
            );

            egui::ComboBox::from_id_source(
                "editor_palette_named_color_placeholder"
            )
            .selected_text(
                palette.name()
            )
            .width(
                130.0 * metrics.scale
            )
            .show_ui(
                ui,
                |ui| {
                    ui.add_enabled(
                        false,
                        egui::Button::new(
                            "Named color presets"
                        ),
                    );
                },
            );

            let mut placeholder_hex =
                "#bc4a3c".to_string();

            ui.add_enabled(
                false,
                egui::TextEdit::singleline(
                    &mut placeholder_hex
                )
                .desired_width(
                    90.0 * metrics.scale
                ),
            );
        },
    );

    ui.add_space(
        6.0 * metrics.scale
    );

    ui.allocate_ui_with_layout(
        egui::vec2(
            230.0 * metrics.scale,
            165.0 * metrics.scale,
        ),
        egui::Layout::centered_and_justified(
            egui::Direction::LeftToRight
        ),
        |ui| {
            ui.label(
                egui::RichText::new(
                    "[ Color Picker / Color Wheel ]"
                )
                .weak(),
            );
        },
    );
}


// ======================== END TEXTURES TAB ==================

// ============================================================
// POST-PROCESSING TAB
// ============================================================
// Copy/paste replacement boundary for this editor section.

fn draw_post_processing_tab(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    anti_aliasing: &mut AntiAliasingSelection,
    dithering: &mut DitheringSelection,
    color_precision: &mut ColorPrecisionSelection,
    hover_help_message: &mut Option<&'static str>,
) {
    draw_post_processing_panel(
        ui,
        metrics,
        anti_aliasing,
        dithering,
        color_precision,
        hover_help_message,
    );

    ui.add_space(
        8.0 * metrics.scale
    );

    ui.horizontal(
        |ui| {
            ui.label(
                "Bloom:"
            );

            ui.add_enabled(
                false,
                egui::Button::new(
                    "Highlight (planned)"
                )
                .min_size(
                    egui::vec2(
                        150.0 * metrics.scale,
                        0.0,
                    )
                ),
            );
        },
    );
}


#[allow(clippy::too_many_arguments)]




fn draw_post_processing_panel(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    anti_aliasing: &mut AntiAliasingSelection,
    dithering: &mut DitheringSelection,
    color_precision: &mut ColorPrecisionSelection,
    hover_help_message: &mut Option<&'static str>,
) {
    editor_theme::panel_frame(
        ui,
        metrics,
    )
    .show(
        ui,
        |ui| {
            editor_theme::section_heading(
                ui,
                "Post-Processing",
            );

            ui.add_space(
                metrics.row_gap
            );

            egui::Grid::new(
                "editor_post_processing_grid"
            )
            .num_columns(2)
            .spacing(
                egui::vec2(
                    8.0 * metrics.scale,
                    metrics.row_gap,
                )
            )
            .show(
                ui,
                |ui| {
                    ui.label("Anti-Aliasing");

                    let selected_text =
                        match *anti_aliasing {
                            AntiAliasingSelection::Off =>
                                "Off",
                            AntiAliasingSelection::Fxaa =>
                                "FXAA",
                        };

                    let response =
                        egui::ComboBox::from_id_source(
                            "editor_anti_aliasing"
                        )
                        .selected_text(selected_text)
                        .width(metrics.dropdown_width)
                        .show_ui(
                            ui,
                            |ui| {
                                ui.selectable_value(
                                    anti_aliasing,
                                    AntiAliasingSelection::Off,
                                    "Off",
                                );

                                ui.selectable_value(
                                    anti_aliasing,
                                    AntiAliasingSelection::Fxaa,
                                    "FXAA",
                                );
                            },
                        )
                        .response;

                    update_hover_help(
                        &response,
                        hover_help_message,
                        "Select the anti-aliasing method used to smooth rendered edges.",
                    );
                    ui.end_row();

                    ui.label("Dithering");

                    let selected_text =
                        match *dithering {
                            DitheringSelection::Off =>
                                "Off",
                            DitheringSelection::Subtle =>
                                "Subtle",
                        };

                    let response =
                        egui::ComboBox::from_id_source(
                            "editor_dithering"
                        )
                        .selected_text(selected_text)
                        .width(metrics.dropdown_width)
                        .show_ui(
                            ui,
                            |ui| {
                                ui.selectable_value(
                                    dithering,
                                    DitheringSelection::Off,
                                    "Off",
                                );

                                ui.selectable_value(
                                    dithering,
                                    DitheringSelection::Subtle,
                                    "Subtle",
                                );
                            },
                        )
                        .response;

                    update_hover_help(
                        &response,
                        hover_help_message,
                        "Reduce visible color banding in smooth gradients.",
                    );
                    ui.end_row();

                    ui.label("Color Precision");

                    let response =
                        egui::ComboBox::from_id_source(
                            "editor_color_precision"
                        )
                        .selected_text(
                            color_precision.display_name()
                        )
                        .width(metrics.dropdown_width)
                        .show_ui(
                            ui,
                            |ui| {
                                ui.selectable_value(
                                    color_precision,
                                    ColorPrecisionSelection::Automatic,
                                    "Automatic",
                                );

                                ui.selectable_value(
                                    color_precision,
                                    ColorPrecisionSelection::High,
                                    "High Precision",
                                );

                                ui.selectable_value(
                                    color_precision,
                                    ColorPrecisionSelection::Standard,
                                    "Standard Precision",
                                );
                            },
                        )
                        .response;

                    update_hover_help(
                        &response,
                        hover_help_message,
                        "Controls off-screen render-target precision. Higher precision can reduce banding but may use more GPU memory.",
                    );
                    ui.end_row();
                },
            );
        },
    );
}




// ============================================================
// CONFIG TAB
// ============================================================
// Configuration-tab editing state.  Persistence is coordinated by
// edit_shader.rs through manage_configuration.rs.

fn draw_config_tab(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    configuration: &mut Option<ControlConfiguration>,
    baseline: Option<&ControlConfiguration>,
    recent_shader_paths: &[PathBuf],
    clear_recent_files_requested: &mut bool,
    save_requested: &mut bool,
    single_browse_requested: &mut Option<PolicyTarget>,
    single_recent_requested: &mut Option<(PolicyTarget, usize)>,
    status_message: &mut String,
    hover_help_message: &mut Option<&'static str>,
) {
    let Some(configuration) =
        configuration.as_mut()
    else {
        ui.label(
            egui::RichText::new(
                "Configuration is not available."
            )
            .weak(),
        );
        return;
    };

    let texture_choices = [
        "bricks",
        "cells",
        "clouds",
        "facets",
        "hexagons",
        "marble",
        "mesh",
        "noise",
        "radial",
        "random",
    ];

    let palette_choices = [
        "brick",
        "bronze",
        "lichen",
        "mist",
        "random",
        "sandstone",
        "slate",
    ];

    ui.columns(
        2,
        |columns| {
            draw_config_target_column(
                &mut columns[0],
                metrics,
                "Screensavers",
                PolicyTarget::Screensaver,
                &mut configuration.screensaver_enabled,
                Some(
                    &mut configuration.subtitles
                ),
                None,
                &mut configuration.screensaver_display,
                &mut configuration.screensaver_interval_seconds,
                &mut configuration.screensaver_single_filename,
                Some(
                    &mut configuration.idle_timeout
                ),
                &mut configuration.screensaver_global_texture,
                &mut configuration.screensaver_global_palette,
                &texture_choices,
                &palette_choices,
                recent_shader_paths,
                clear_recent_files_requested,
                single_browse_requested,
                single_recent_requested,
                status_message,
                hover_help_message,
            );

            draw_config_target_column(
                &mut columns[1],
                metrics,
                "Wallpapers",
                PolicyTarget::Wallpaper,
                &mut configuration.wallpaper_enabled,
                None,
                Some(
                    &mut configuration.notifications
                ),
                &mut configuration.wallpaper_display,
                &mut configuration.wallpaper_interval_seconds,
                &mut configuration.wallpaper_single_filename,
                None,
                &mut configuration.wallpaper_global_texture,
                &mut configuration.wallpaper_global_palette,
                &texture_choices,
                &palette_choices,
                recent_shader_paths,
                clear_recent_files_requested,
                single_browse_requested,
                single_recent_requested,
                status_message,
                hover_help_message,
            );
        },
    );

    ui.add_space(
        8.0 * metrics.scale
    );

    ui.separator();

    let dirty =
        baseline
            .map(
                |baseline| {
                    &*configuration
                        != baseline
                }
            )
            .unwrap_or(
                false
            );

    let single_filename_missing =
        (
            configuration.screensaver_display
                == "single"
                && configuration
                    .screensaver_single_filename
                    .trim()
                    .is_empty()
        )
        || (
            configuration.wallpaper_display
                == "single"
                && configuration
                    .wallpaper_single_filename
                    .trim()
                    .is_empty()
        );

    ui.horizontal(
        |ui| {
            let save_response =
                ui.add_enabled(
                    dirty
                        && !single_filename_missing,
                    egui::Button::new(
                        "Save Configuration"
                    ),
                );

            update_hover_help(
                &save_response,
                hover_help_message,
                if single_filename_missing {
                    "Select a shader filename before saving Single display mode."
                } else {
                    "Save configuration changes."
                },
            );

            if save_response.clicked() {
                *save_requested =
                    true;

                *status_message =
                    "Saving configuration..."
                        .to_string();
            }

            let cancel_response =
                ui.add_enabled(
                    dirty,
                    egui::Button::new(
                        "Cancel"
                    ),
                );

            update_hover_help(
                &cancel_response,
                hover_help_message,
                "Discard unsaved configuration changes.",
            );

            if cancel_response.clicked() {
                if let Some(baseline) =
                    baseline
                {
                    *configuration =
                        baseline.clone();

                    *status_message =
                        "Configuration changes discarded."
                            .to_string();
                }
            }
        },
    );
}


#[allow(clippy::too_many_arguments)]
fn draw_config_target_column(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    heading: &str,
    target: PolicyTarget,
    enabled: &mut bool,
    subtitles: Option<&mut bool>,
    notifications: Option<&mut bool>,
    display_mode: &mut String,
    interval_seconds: &mut u64,
    single_filename: &mut String,
    idle_timeout: Option<&mut String>,
    global_texture: &mut String,
    global_palette: &mut String,
    texture_choices: &[&str],
    palette_choices: &[&str],
    recent_shader_paths: &[PathBuf],
    clear_recent_files_requested: &mut bool,
    single_browse_requested: &mut Option<PolicyTarget>,
    single_recent_requested: &mut Option<(PolicyTarget, usize)>,
    status_message: &mut String,
    hover_help_message: &mut Option<&'static str>,
) {
    const DEFAULT_INTERVAL_SECONDS: u64 =
        600;

    ui.heading(
        heading
    );

    ui.add_space(
        4.0 * metrics.scale
    );

    ui.checkbox(
        enabled,
        "Enabled",
    );

    if let Some(subtitles) =
        subtitles
    {
        ui.checkbox(
            subtitles,
            "Subtitles",
        );
    }

    if let Some(notifications) =
        notifications
    {
        ui.checkbox(
            notifications,
            "Notifications",
        );
    }

    ui.add_space(
        5.0 * metrics.scale
    );

    egui::Grid::new(
        format!(
            "config_grid_{}",
            heading,
        )
    )
    .num_columns(
        2
    )
    .spacing(
        egui::vec2(
            8.0 * metrics.scale,
            6.0 * metrics.scale,
        )
    )
    .show(
        ui,
        |ui| {
            ui.label(
                "Display"
            );

            let previous_display_mode =
                display_mode.clone();

            egui::ComboBox::from_id_source(
                format!(
                    "config_display_{}",
                    heading,
                )
            )
            .selected_text(
                display_mode.as_str()
            )
            .show_ui(
                ui,
                |ui| {
                    // Alphanumeric order is intentional.
                    for choice in [
                        "ordered",
                        "random",
                        "single",
                    ] {
                        ui.selectable_value(
                            display_mode,
                            choice.to_string(),
                            choice,
                        );
                    }
                },
            );

            if previous_display_mode
                == "single"
                && display_mode.as_str()
                    != "single"
            {
                // A rotating mode always receives a known-good interval
                // when it is selected from Single mode.
                *interval_seconds =
                    DEFAULT_INTERVAL_SECONDS;
            }

            if display_mode.as_str()
                != "single"
                && *interval_seconds == 0
            {
                *interval_seconds =
                    DEFAULT_INTERVAL_SECONDS;
            }

            ui.end_row();

            if display_mode.as_str()
                == "single"
            {
                ui.label(
                    "Filename"
                );

                let displayed_filename =
                    if single_filename
                        .trim()
                        .is_empty()
                    {
                        "<select shader>"
                    } else {
                        single_filename
                            .as_str()
                    };

                ui.menu_button(
                    displayed_filename,
                    |ui| {
                        if recent_shader_paths
                            .is_empty()
                        {
                            ui.add_enabled(
                                false,
                                egui::Button::new(
                                    "Recent Files (empty)"
                                ),
                            );
                        } else {
                            for (
                                index,
                                path,
                            ) in recent_shader_paths
                                .iter()
                                .enumerate()
                            {
                                let display_name =
                                    path
                                        .file_name()
                                        .and_then(
                                            |name| {
                                                name.to_str()
                                            }
                                        )
                                        .unwrap_or(
                                            "Unnamed shader"
                                        );

                                let response =
                                    ui.button(
                                        display_name
                                    );

                                response
                                    .clone()
                                    .on_hover_text(
                                        path.display()
                                            .to_string()
                                    );

                                if response.clicked() {
                                    *single_recent_requested =
                                        Some(
                                            (
                                                target,
                                                index,
                                            )
                                        );

                                    ui.close();
                                }
                            }
                        }

                        ui.separator();

                        if ui.button(
                            "Browse..."
                        )
                        .clicked()
                        {
                            *single_browse_requested =
                                Some(
                                    target
                                );

                            ui.close();
                        }

                        if ui.add_enabled(
                            !recent_shader_paths.is_empty(),
                            egui::Button::new(
                                "Clear Recent Files"
                            ),
                        )
                        .clicked()
                        {
                            *clear_recent_files_requested =
                                true;

                            ui.close();
                        }
                    },
                );

                ui.end_row();
            } else {
                ui.label(
                    "Every"
                );

                ui.horizontal(
                    |ui| {
                        ui.add(
                            egui::DragValue::new(
                                interval_seconds
                            )
                            .clamp_range(
                                1..=86400
                            ),
                        );

                        ui.label(
                            "seconds"
                        );
                    },
                );

                ui.end_row();
            }

            if let Some(idle_timeout) =
                idle_timeout
            {
                ui.label(
                    "After idle"
                );

                ui.text_edit_singleline(
                    idle_timeout
                );

                ui.end_row();
            }

            ui.label(
                "Default texture"
            );

            egui::ComboBox::from_id_source(
                format!(
                    "config_texture_{}",
                    heading,
                )
            )
            .selected_text(
                global_texture.as_str()
            )
            .show_ui(
                ui,
                |ui| {
                    for choice in texture_choices {
                        ui.selectable_value(
                            global_texture,
                            (*choice).to_string(),
                            *choice,
                        );
                    }
                },
            );

            ui.end_row();

            ui.label(
                "Default palette"
            );

            egui::ComboBox::from_id_source(
                format!(
                    "config_palette_{}",
                    heading,
                )
            )
            .selected_text(
                global_palette.as_str()
            )
            .show_ui(
                ui,
                |ui| {
                    for choice in palette_choices {
                        ui.selectable_value(
                            global_palette,
                            (*choice).to_string(),
                            *choice,
                        );
                    }
                },
            );

            ui.end_row();
        },
    );

    if display_mode.as_str()
        == "single"
        && single_filename
            .trim()
            .is_empty()
    {
        *status_message =
            "Select a shader for Single display mode."
                .to_string();
    }
}

// ======================== END CONFIG TAB =====================
// ==================== END POST-PROCESSING TAB ===============

// ------------------------------------------------------------
// LEGACY / SHARED SUPPORT
// ------------------------------------------------------------
// Retained for compatibility while the multi-tabbed editor settles.
// These helpers are not part of the tab replacement blocks above.

fn draw_policy_target_panel(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    policy_target: Option<PolicyTarget>,
    configuration_changed: bool,
    screensaver_target_available: bool,
    wallpaper_target_available: bool,
    screensaver_target_session_restricted: bool,
    wallpaper_target_session_restricted: bool,
    policy_target_change_requested: &mut Option<PolicyTarget>,
    status_message: &mut String,
    hover_help_message: &mut Option<&'static str>,
) {
    editor_theme::panel_frame(
        ui,
        metrics,
    )
    .show(
        ui,
        |ui| {
            editor_theme::section_heading(
                ui,
                "Policy Target",
            );

            ui.add_space(
                metrics.row_gap
            );

            let screensaver_response =
                ui.add_enabled(
                    screensaver_target_available,
                    egui::RadioButton::new(
                        policy_target
                            == Some(
                                PolicyTarget::Screensaver
                            ),
                        "Screensaver",
                    ),
                );

            update_hover_help(
                &screensaver_response,
                hover_help_message,
                if screensaver_target_available {
                    "Load or create the policy used for screensaver rendering."
                } else if screensaver_target_session_restricted {
                    "This editing session was opened for the active wallpaper. Only the Wallpaper policy can be edited."
                } else {
                    "This shader is unavailable for Screensaver use because it does not exist in the screensavers folder."
                },
            );

            if screensaver_response.clicked()
                && policy_target
                    != Some(
                        PolicyTarget::Screensaver
                    )
            {
                if configuration_changed
                    && policy_target.is_some()
                {
                    *status_message =
                        "Save or cancel the current changes before switching policy targets."
                            .to_string();
                } else {
                    *policy_target_change_requested =
                        Some(
                            PolicyTarget::Screensaver
                        );
                }
            }

            let wallpaper_response =
                ui.add_enabled(
                    wallpaper_target_available,
                    egui::RadioButton::new(
                        policy_target
                            == Some(
                                PolicyTarget::Wallpaper
                            ),
                        "Wallpaper",
                    ),
                );

            update_hover_help(
                &wallpaper_response,
                hover_help_message,
                if wallpaper_target_available {
                    "Load or create the policy used for wallpaper rendering."
                } else if wallpaper_target_session_restricted {
                    "This editing session was opened for the active screensaver. Only the Screensaver policy can be edited."
                } else {
                    "This shader is unavailable for Wallpaper use because it does not exist in the wallpapers folder."
                },
            );

            if wallpaper_response.clicked()
                && policy_target
                    != Some(
                        PolicyTarget::Wallpaper
                    )
            {
                if configuration_changed
                    && policy_target.is_some()
                {
                    *status_message =
                        "Save or cancel the current changes before switching policy targets."
                            .to_string();
                } else {
                    *policy_target_change_requested =
                        Some(
                            PolicyTarget::Wallpaper
                        );
                }
            }
        },
    );
}


#[allow(clippy::too_many_arguments)]
fn draw_policy_actions_panel(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    can_save: bool,
    can_cancel: bool,
    can_delete_shader: bool,
    save_requested: &mut bool,
    cancel_requested: &mut bool,
    delete_requested: &mut bool,
    displayed_fps: &mut u32,
    displayed_animation_speed: &mut f32,
    displayed_render_scale: &mut f32,
    policy_target: &mut Option<PolicyTarget>,
    texture: &mut TextureSelection,
    palette: &mut PaletteSelection,
    primitive_count: &mut u32,
    anti_aliasing: &mut AntiAliasingSelection,
    dithering: &mut DitheringSelection,
    color_precision: &mut ColorPrecisionSelection,
    baseline_configuration: EditorConfiguration,
    fps_drag_state: &mut Option<SliderDragState>,
    animation_speed_drag_state: &mut Option<SliderDragState>,
    render_scale_drag_state: &mut Option<SliderDragState>,
    status_message: &mut String,
    hover_help_message: &mut Option<&'static str>,
) {
    editor_theme::panel_frame(
        ui,
        metrics,
    )
    .show(
        ui,
        |ui| {
            editor_theme::section_heading(
                ui,
                "Policy Actions",
            );

            ui.add_space(
                metrics.row_gap
            );

            let save_response =
                ui.add_enabled(
                    can_save,
                    egui::Button::new(
                        "Save Policy"
                    )
                    .min_size(
                        egui::vec2(
                            metrics.action_button_width,
                            0.0,
                        )
                    )
                    .fill(
                        egui::Color32::from_rgb(
                            30,
                            100,
                            180,
                        )
                    ),
                );

            update_hover_help(
                &save_response,
                hover_help_message,
                "Save the current per-shader policy after all mandatory information is supplied.",
            );

            if save_response.clicked() {
                *save_requested =
                    true;

                *status_message =
                    "Saving policy..."
                        .to_string();
            }

            let cancel_response =
                ui.add_enabled(
                    can_cancel,
                    egui::Button::new(
                        "Cancel"
                    )
                    .min_size(
                        egui::vec2(
                            metrics.action_button_width,
                            0.0,
                        )
                    ),
                );

            update_hover_help(
                &cancel_response,
                hover_help_message,
                "Discard changes made during this editor session.",
            );

            if cancel_response.clicked() {
                *cancel_requested =
                    true;

                *displayed_fps =
                    baseline_configuration.fps;
                *displayed_animation_speed =
                    baseline_configuration.animation_speed;
                *displayed_render_scale =
                    baseline_configuration.render_scale;
                *policy_target =
                    baseline_configuration.policy_target;
                *texture =
                    baseline_configuration.texture;
                *palette =
                    baseline_configuration.palette;
                *primitive_count =
                    baseline_configuration.primitive_count;
                *anti_aliasing =
                    baseline_configuration.anti_aliasing;
                *dithering =
                    baseline_configuration.dithering;
                *color_precision =
                    baseline_configuration.color_precision;
                *fps_drag_state =
                    None;
                *animation_speed_drag_state =
                    None;
                *render_scale_drag_state =
                    None;
                *status_message =
                    "Changes canceled"
                        .to_string();
            }

            let delete_response =
                ui.add_enabled(
                    can_delete_shader,
                    egui::Button::new(
                        "Delete Shader"
                    )
                    .min_size(
                        egui::vec2(
                            metrics.action_button_width,
                            0.0,
                        )
                    )
                    .fill(
                        egui::Color32::from_rgb(
                            150,
                            40,
                            40,
                        )
                    ),
                );

            update_hover_help(
                &delete_response,
                hover_help_message,
                "Permanently remove the shader file after confirmation.",
            );

            if delete_response.clicked() {
                *delete_requested =
                    true;

                *status_message =
                    "Delete Shader is not implemented"
                        .to_string();
            }
        },
    );
}


fn draw_about_panel(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
) {
    editor_theme::panel_frame(
        ui,
        metrics,
    )
    .show(
        ui,
        |ui| {
            editor_theme::section_heading(
                ui,
                "About Shader Policies",
            );

            ui.add_space(
                metrics.row_gap
            );

            ui.label(
                "A shader policy determines how the selected shader is rendered as a screensaver or wallpaper."
            );
        },
    );
}


fn draw_status_panel(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    message: &str,
    configuration_changed: bool,
) {
    editor_theme::panel_frame(
        ui,
        metrics,
    )
    .show(
        ui,
        |ui| {
            ui.set_min_height(
                metrics.status_height
            );

            ui.horizontal(
                |ui| {
                    ui.label(
                        message
                    );

                    ui.with_layout(
                        egui::Layout::right_to_left(
                            egui::Align::Center
                        ),
                        |ui| {
                            ui.label(
                                if configuration_changed {
                                    "Policy: Unsaved changes"
                                } else {
                                    "Policy: Unchanged"
                                }
                            );
                        },
                    );
                },
            );
        },
    );
}




fn egui_pointer_button(
    button: MouseButton,
) -> Option<egui::PointerButton> {

    match button {
        MouseButton::Left => {
            Some(
                egui::PointerButton::Primary
            )
        }

        MouseButton::Right => {
            Some(
                egui::PointerButton::Secondary
            )
        }

        MouseButton::Middle => {
            Some(
                egui::PointerButton::Middle
            )
        }

        _ => {
            None
        }
    }
}


