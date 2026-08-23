//! Graphical layout and input handling for the Screenshaver Control Center.
//!
//! This module owns the egui window, controls, layout, styling, and SDL-to-egui
//! input translation. Rendering and shader-session behavior remain in
//! `edit_shader`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{
    Duration,
    Instant,
};

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


// First Textures-tab thumbnail sizing experiment.  The Control Center itself
// remains fixed-size; this is only an upper bound on the preview image.
const TEXTURE_THUMBNAIL_MAX_SIZE: f32 =
    240.0;

const TEXTURE_THUMBNAIL_SEED: u64 =
    0x5343_5245_454E_5348;


const CONTROL_CENTER_BRANDING_IMAGE: &[u8] =
    include_bytes!(
        "../assets/screenshaver-splash.png"
    );


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
    Unassigned,
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
enum PolicyNavigation {
    First,
    Last,
    Previous,
    Next,
    PagePrevious,
    PageNext,
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
    Skulls,
    Scales,
    Eyes,
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
            crate::generate_textures::TextureFamily::Skulls => Self::Skulls,
            crate::generate_textures::TextureFamily::Scales => Self::Scales,
            crate::generate_textures::TextureFamily::Eyes => Self::Eyes,
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
            Self::Skulls => crate::generate_textures::TextureFamily::Skulls,
            Self::Scales => crate::generate_textures::TextureFamily::Scales,
            Self::Eyes => crate::generate_textures::TextureFamily::Eyes,
        }
    }


    pub fn name(
        self,
    ) -> &'static str {
        self.family().name()
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaletteSelection(
    crate::palettes::PaletteColor
);


impl PaletteSelection {
    pub const fn from_palette(
        palette: crate::palettes::PaletteColor,
    ) -> Self {

        Self(
            palette
        )
    }


    pub const fn palette(
        self,
    ) -> crate::palettes::PaletteColor {

        self.0
    }


    pub fn name(
        self,
    ) -> String {

        self.0.to_hex()
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AntiAliasingSelection {
    Off,
    Fxaa,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BloomSelection {
    Off,
    Highlight,
    Audio,
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
    pub policy_name: String,
    pub filename: String,
    pub folder: String,
    pub shader_type: String,
    pub texture_usage: String,
    pub status: String,
}




#[derive(Clone, Debug, PartialEq)]
pub struct ControlConfiguration {
    pub show_splash: bool,
    pub screensaver_enabled: bool,
    pub subtitles: bool,
    pub subtitle_placement: String,
    pub screensaver_display: String,
    pub screensaver_interval_seconds: u64,
    pub screensaver_single_policy_id: Option<i64>,
    pub screensaver_single_policy_name: String,
    pub screensaver_idle_timeout_seconds: i64,
    pub screensaver_animation_speed: f64,
    pub screensaver_global_texture: String,
    pub screensaver_texture_primitives: i64,
    pub screensaver_global_palette: String,
    pub wallpaper_enabled: bool,
    pub notifications: bool,
    pub wallpaper_display: String,
    pub wallpaper_interval_seconds: u64,
    pub wallpaper_single_policy_id: Option<i64>,
    pub wallpaper_single_policy_name: String,
    pub wallpaper_animation_speed: f64,
    pub wallpaper_global_texture: String,
    pub wallpaper_texture_primitives: i64,
    pub wallpaper_global_palette: String,
    pub rendered_fps: i64,
    pub anti_aliasing: String,
    pub dithering: String,
    pub color_precision: String,
    pub render_scale: f64,
}

impl ControlConfiguration {
    pub fn from_config(config: &crate::load_config::Config) -> Self {
        let (screensaver_display, screensaver_interval_seconds, screensaver_single_policy_id) =
            split_display_mode(&config.mode);
        let (wallpaper_display, wallpaper_interval_seconds, wallpaper_single_policy_id) =
            split_display_mode(&config.wallpaper_mode);

        let screensaver_single_policy_name =
            policy_name_for_id(&config.screensaver_policies, screensaver_single_policy_id);
        let wallpaper_single_policy_name =
            policy_name_for_id(&config.wallpaper_policies, wallpaper_single_policy_id);

        let app_defaults = crate::manage_configuration::load_app_defaults().ok();
        let screensaver_defaults =
            crate::manage_configuration::load_target_defaults("screensaver").ok();
        let wallpaper_defaults =
            crate::manage_configuration::load_target_defaults("wallpaper").ok();

        let target_texture = |defaults: Option<&crate::manage_configuration::TargetDefaults>,
                              legacy: String| {
            defaults
                .map(|defaults| {
                    if defaults.texture_mode == "specific" {
                        defaults.texture_family.clone().unwrap_or_else(|| "random".to_string())
                    } else {
                        "random".to_string()
                    }
                })
                .unwrap_or(legacy)
        };

        let target_palette = |defaults: Option<&crate::manage_configuration::TargetDefaults>,
                              legacy: String| {
            defaults
                .map(|defaults| {
                    if defaults.palette_mode == "specific" {
                        defaults.palette_color.clone().unwrap_or_else(|| "random".to_string())
                    } else {
                        "random".to_string()
                    }
                })
                .unwrap_or(legacy)
        };

        Self {
            show_splash: app_defaults.as_ref().map(|d| d.show_splash).unwrap_or(true),
            screensaver_enabled: config.screensaver_enabled,
            subtitles: app_defaults.as_ref().map(|d| d.screensaver_subtitles).unwrap_or(config.subtitles),
            subtitle_placement: app_defaults.as_ref().map(|d| d.subtitle_placement.clone()).unwrap_or_else(|| "bottom:center".to_string()),
            screensaver_display,
            screensaver_interval_seconds,
            screensaver_single_policy_id,
            screensaver_single_policy_name,
            screensaver_idle_timeout_seconds: screensaver_defaults.as_ref().and_then(|d| d.idle_timeout_seconds).unwrap_or(600),
            screensaver_animation_speed: screensaver_defaults.as_ref().map(|d| d.animation_speed).unwrap_or(1.0),
            screensaver_global_texture: target_texture(
                screensaver_defaults.as_ref(),
                config.texture_policy.global_texture.as_ref().map(format_texture_specification).unwrap_or_else(|| "random".to_string()),
            ),
            screensaver_texture_primitives: screensaver_defaults.as_ref().map(|d| d.texture_primitives).unwrap_or(64),
            screensaver_global_palette: target_palette(
                screensaver_defaults.as_ref(),
                config.texture_policy.global_palette.map(|palette| palette.to_hex()).unwrap_or_else(|| "random".to_string()),
            ),
            wallpaper_enabled: config.wallpaper_enabled,
            notifications: app_defaults.as_ref().map(|d| d.wallpaper_notifications).unwrap_or(config.wallpaper.notifications),
            wallpaper_display,
            wallpaper_interval_seconds,
            wallpaper_single_policy_id,
            wallpaper_single_policy_name,
            wallpaper_animation_speed: wallpaper_defaults.as_ref().map(|d| d.animation_speed).unwrap_or(0.03),
            wallpaper_global_texture: target_texture(
                wallpaper_defaults.as_ref(),
                config.wallpaper_texture_policy.global_texture.as_ref().map(format_texture_specification).unwrap_or_else(|| "random".to_string()),
            ),
            wallpaper_texture_primitives: wallpaper_defaults.as_ref().map(|d| d.texture_primitives).unwrap_or(64),
            wallpaper_global_palette: target_palette(
                wallpaper_defaults.as_ref(),
                config.wallpaper_texture_policy.global_palette.map(|palette| palette.to_hex()).unwrap_or_else(|| "random".to_string()),
            ),
            rendered_fps: app_defaults.as_ref().map(|d| d.rendered_fps).unwrap_or(30),
            anti_aliasing: app_defaults.as_ref().map(|d| d.anti_aliasing.clone()).unwrap_or_else(|| "fxaa".to_string()),
            dithering: app_defaults.as_ref().map(|d| d.dithering.clone()).unwrap_or_else(|| "subtle".to_string()),
            color_precision: app_defaults.as_ref().map(|d| d.color_precision.clone()).unwrap_or_else(|| "auto".to_string()),
            render_scale: app_defaults.as_ref().map(|d| d.render_scale).unwrap_or(1.0),
        }
    }
}

fn policy_name_for_id(
    policies: &[crate::load_config::ShaderPolicy],
    policy_id: Option<i64>,
) -> String {

    let Some(policy_id) =
        policy_id
    else {
        return String::new();
    };

    policies
        .iter()
        .find(
            |policy| {
                policy.policy_id == policy_id
            }
        )
        .map(
            |policy| {
                policy.policy_key.clone()
            }
        )
        .unwrap_or_default()
}


fn split_display_mode(
    mode: &str,
) -> (String, u64, Option<i64>) {

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
            None,
        ),

        "single" => (
            "single".to_string(),
            DEFAULT_INTERVAL_SECONDS,
            argument
                .parse::<i64>()
                .ok()
                .filter(
                    |value| {
                        *value > 0
                    }
                ),
        ),

        _ => (
            "random".to_string(),
            DEFAULT_INTERVAL_SECONDS,
            None,
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
    pub policy_id: i64,
    pub policy_key: String,
    pub filename: String,
    pub full_path: String,
    pub accessible: bool,
    pub texture: bool,
    pub policy_target: PolicyTarget,
    pub unassigned: bool,
}

#[derive(Clone, Debug)]
pub struct BulkCreateCandidate {
    pub path: PathBuf,
    pub forced_target: Option<PolicyTarget>,
    pub texture_required: bool,
}


#[derive(Clone, Debug)]
pub struct BulkCreateRequest {
    pub candidates: Vec<BulkCreateCandidate>,
    pub external_target: Option<PolicyTarget>,
    pub rejected_count: usize,
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyRowReference {
    pub policy_id: i64,
    pub policy_key: String,
    pub filename: String,
    pub full_path: String,
    pub policy_target: PolicyTarget,
    pub unassigned: bool,
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyListStateSnapshot {
    pub sort_column: String,
    pub sort_ascending: bool,
    pub selected_policy_row: Option<PolicyRowReference>,
    pub window_x: Option<i32>,
    pub window_y: Option<i32>,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyRowCommand {
    Edit,
    ClonePolicy,
    RenamePolicy,
    RefreshShader,
    MoveToScreensavers,
    MoveToWallpapers,
    DeletePolicy,
    DeleteShader,
}


#[derive(Clone, Debug)]
struct PendingConfirmation {
    row: PolicyRowReference,
    command: PolicyRowCommand,
}


#[derive(Clone, Debug)]
struct PendingPolicyClone {
    row: PolicyRowReference,
    policy_name: String,
    validation_message: String,
}


#[derive(Clone, Debug)]
struct PendingPolicyRename {
    row: PolicyRowReference,
    policy_name: String,
    validation_message: String,
}


#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BulkEditChanges {
    pub policy_target: bool,
    pub fps: bool,
    pub animation_speed: bool,
    pub render_scale: bool,
    pub texture: bool,
    pub palette: bool,
    pub primitive_count: bool,
    pub anti_aliasing: bool,
    pub dithering: bool,
    pub color_precision: bool,
    pub bloom: bool,
    pub bloom_intensity: bool,
    pub bloom_threshold: bool,
    pub invert_colors: bool,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    pub hue_rotation: bool,
}

impl BulkEditChanges {
    pub fn any(self) -> bool {
        self.policy_target
            || self.fps
            || self.animation_speed
            || self.render_scale
            || self.texture
            || self.palette
            || self.primitive_count
            || self.anti_aliasing
            || self.dithering
            || self.color_precision
            || self.bloom
            || self.bloom_intensity
            || self.bloom_threshold
            || self.invert_colors
            || self.flip_horizontal
            || self.flip_vertical
            || self.hue_rotation
    }
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
    pub bloom: BloomSelection,
    pub bloom_intensity: f32,
    pub bloom_threshold: f32,
    pub invert_colors: bool,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    pub hue_rotation: f32,
    pub policy_target_change_requested: Option<PolicyTarget>,
    pub save_requested: bool,
    pub bulk_save_requested: bool,
    pub bulk_edit_changes: BulkEditChanges,
    pub bulk_selected_policy_rows:
        Vec<PolicyRowReference>,
    pub cancel_requested: bool,
    pub delete_requested: bool,
    pub browse_shader_requested: bool,
    pub bulk_create_browse_requested: bool,
    pub bulk_create_requested:
        Option<BulkCreateRequest>,
    pub recent_shader_requested: Option<usize>,
    pub clear_recent_files_requested: bool,
    pub refresh_shader_requested: bool,
    pub policy_row_command_requested:
        Option<(PolicyRowReference, PolicyRowCommand)>,
    pub clone_policy_requested:
        Option<(PolicyRowReference, String)>,
    pub rename_policy_requested:
        Option<(PolicyRowReference, String)>,
    pub control_configuration: Option<ControlConfiguration>,
    pub policy_dirty: bool,
    pub control_configuration_dirty: bool,
    pub control_configuration_save_requested: bool,
    pub exit_after_save_requested: bool,
    pub exit_discard_requested: bool,
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
    bloom: BloomSelection,
    bloom_intensity: f32,
    bloom_threshold: f32,
    invert_colors: bool,
    flip_horizontal: bool,
    flip_vertical: bool,
    hue_rotation: f32,
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
        bloom: BloomSelection,
        bloom_intensity: f32,
        bloom_threshold: f32,
        invert_colors: bool,
        flip_horizontal: bool,
        flip_vertical: bool,
        hue_rotation: f32,
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
            bloom,
            bloom_intensity:
                normalize_editor_float(
                    bloom_intensity
                ),
            bloom_threshold:
                normalize_editor_float(
                    bloom_threshold
                ),
            invert_colors,
            flip_horizontal,
            flip_vertical,
            hue_rotation:
                normalize_editor_float(hue_rotation),
        }
    }


    fn bulk_changes_from(
        self,
        baseline: Self,
    ) -> BulkEditChanges {
        BulkEditChanges {
            policy_target:
                self.policy_target != baseline.policy_target,
            fps:
                self.fps != baseline.fps,
            animation_speed:
                (self.animation_speed - baseline.animation_speed).abs() > 0.0001,
            render_scale:
                (self.render_scale - baseline.render_scale).abs() > 0.0001,
            texture:
                self.texture != baseline.texture,
            palette:
                self.palette != baseline.palette,
            primitive_count:
                self.primitive_count != baseline.primitive_count,
            anti_aliasing:
                self.anti_aliasing != baseline.anti_aliasing,
            dithering:
                self.dithering != baseline.dithering,
            color_precision:
                self.color_precision != baseline.color_precision,
            bloom:
                self.bloom != baseline.bloom,
            bloom_intensity:
                (self.bloom_intensity - baseline.bloom_intensity).abs() > 0.0001,
            bloom_threshold:
                (self.bloom_threshold - baseline.bloom_threshold).abs() > 0.0001,
            invert_colors:
                self.invert_colors != baseline.invert_colors,
            flip_horizontal:
                self.flip_horizontal != baseline.flip_horizontal,
            flip_vertical:
                self.flip_vertical != baseline.flip_vertical,
            hue_rotation:
                (self.hue_rotation - baseline.hue_rotation).abs() > 0.0001,
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
            || self.bloom
                != other.bloom
            || (self.bloom_intensity
                - other.bloom_intensity)
                .abs()
                > 0.0001
            || (self.bloom_threshold
                - other.bloom_threshold)
                .abs()
                > 0.0001
            || self.invert_colors != other.invert_colors
            || self.flip_horizontal != other.flip_horizontal
            || self.flip_vertical != other.flip_vertical
            || (self.hue_rotation - other.hue_rotation).abs() > 0.0001
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

    branding_texture:
        egui::TextureHandle,

    branding_aspect_ratio:
        f32,

    texture_thumbnail:
        Option<egui::TextureHandle>,

    texture_thumbnail_key:
        Option<(TextureSelection, PaletteSelection, u32)>,

    pending_events:
        Vec<egui::Event>,

    pointer_position:
        egui::Pos2,

    opened_at:
        Instant,

    window_open:
        bool,

    window_position:
        Option<egui::Pos2>,

    persisted_window_position:
        Option<(i32, i32)>,

    window_position_changed_at:
        Option<Instant>,

    close_requested:
        bool,

    pending_exit_confirmation:
        bool,

    pending_bulk_save_confirmation:
        bool,

    pending_bulk_create_candidates:
        Option<Vec<BulkCreateCandidate>>,

    pending_bulk_create_rejected_count:
        usize,

    pending_bulk_create_external_target:
        Option<PolicyTarget>,

    policy_creation_pending:
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

    palette_hex_input:
        String,

    color_picker_preview:
        egui::Color32,

    primitive_count:
        u32,

    anti_aliasing:
        AntiAliasingSelection,

    dithering:
        DitheringSelection,

    color_precision:
        ColorPrecisionSelection,

    bloom:
        BloomSelection,

    bloom_intensity:
        f32,

    bloom_threshold:
        f32,

    invert_colors:
        bool,

    flip_horizontal:
        bool,

    flip_vertical:
        bool,

    hue_rotation:
        f32,

    active_tab:
        EditorTab,

    policy_sort_column:
        PolicySortColumn,

    policy_sort_ascending:
        bool,

    selected_policy_row:
        Option<PolicyRowReference>,

    // When a clone is created, Control Center temporarily focuses the new
    // policy without changing the persisted last-edited policy in state.json.
    // Outer Some means transient focus is active; the inner Option preserves
    // the selection that was persistent before cloning.
    transient_policy_selection_persisted_row:
        Option<Option<PolicyRowReference>>,

    restore_selected_policy_scroll:
        bool,

    bulk_selected_policy_rows:
        Vec<PolicyRowReference>,

    bulk_edit_baseline:
        Option<EditorConfiguration>,

    pending_policy_navigation:
        Option<PolicyNavigation>,

    pending_confirmation:
        Option<PendingConfirmation>,

    pending_policy_clone:
        Option<PendingPolicyClone>,

    pending_policy_rename:
        Option<PendingPolicyRename>,

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

    bloom_intensity_drag_state:
        Option<SliderDragState>,

    bloom_threshold_drag_state:
        Option<SliderDragState>,

    hue_rotation_drag_state:
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


        let context =
            egui::Context::default();


        #[cfg(debug_assertions)]
        {
            context.style_mut(
                |style| {
                    style.debug.show_unaligned =
                        false;
                }
            );
        }


        let branding_image =
            image::load_from_memory(
                CONTROL_CENTER_BRANDING_IMAGE
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to decode embedded Control Center branding image: {}",
                        error,
                    )
                }
            )?
            .to_rgba8();


        let branding_width =
            branding_image.width();

        let branding_height =
            branding_image.height();


        if branding_width == 0
            || branding_height == 0
        {
            return Err(
                "Embedded Control Center branding image has invalid dimensions."
                    .to_string()
            );
        }


        let branding_aspect_ratio =
            branding_width as f32
                / branding_height as f32;


        let branding_color_image =
            egui::ColorImage::from_rgba_unmultiplied(
                [
                    branding_width as usize,
                    branding_height as usize,
                ],
                branding_image.as_raw(),
            );


        let branding_texture =
            context.load_texture(
                "screenshaver_control_center_branding",
                branding_color_image,
                egui::TextureOptions::LINEAR,
            );


        Ok(
            Self {
                context,

                painter,

                branding_texture,

                branding_aspect_ratio,

                texture_thumbnail:
                    None,

                texture_thumbnail_key:
                    None,

                pending_events:
                    Vec::new(),

                pointer_position:
                    egui::Pos2::ZERO,

                opened_at:
                    Instant::now(),

                window_open:
                    true,

                window_position:
                    None,

                persisted_window_position:
                    None,

                window_position_changed_at:
                    None,

                close_requested:
                    false,

                pending_exit_confirmation:
                    false,

                pending_bulk_save_confirmation:
                    false,

                pending_bulk_create_candidates:
                    None,

                pending_bulk_create_rejected_count:
                    0,

                pending_bulk_create_external_target:
                    None,

                policy_creation_pending:
                    false,

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
                    PaletteSelection::from_palette(
                        crate::palettes::PaletteColor::new(
                            99,
                            119,
                            134,
                        )
                    ),

                palette_hex_input:
                    crate::palettes::PaletteColor::new(
                        99,
                        119,
                        134,
                    )
                    .to_hex(),

                color_picker_preview:
                    egui::Color32::from_rgb(
                        99,
                        119,
                        134,
                    ),

                primitive_count:
                    32,

                anti_aliasing:
                    AntiAliasingSelection::Off,

                dithering:
                    DitheringSelection::Off,

                color_precision:
                    ColorPrecisionSelection::Automatic,

                bloom:
                    BloomSelection::Off,

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

                active_tab:
                    EditorTab::Policies,

                policy_sort_column:
                    PolicySortColumn::Filename,

                policy_sort_ascending:
                    true,

                selected_policy_row:
                    None,

                transient_policy_selection_persisted_row:
                    None,

                restore_selected_policy_scroll:
                    false,

                bulk_selected_policy_rows:
                    Vec::new(),

                bulk_edit_baseline:
                    None,

                pending_policy_navigation:
                    None,

                pending_confirmation:
                    None,

                pending_policy_clone:
                    None,

                pending_policy_rename:
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

                bloom_intensity_drag_state:
                    None,
                bloom_threshold_drag_state:
                    None,

                hue_rotation_drag_state:
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

            Event::MouseWheel {
                y,
                ..
            } => {
                self.pending_events.push(
                    egui::Event::MouseWheel {
                        unit:
                            egui::MouseWheelUnit::Line,

                        delta:
                            egui::vec2(
                                0.0,
                                y as f32,
                            ),

                        modifiers:
                            egui::Modifiers {
                                shift:
                                    self.shift_held,

                                ..Default::default()
                            },
                    }
                );
            }

            Event::KeyDown {
                keycode:
                    Some(keycode),
                repeat,
                ..
            } if self.active_tab == EditorTab::Policies
                && matches!(
                    keycode,
                    Keycode::Home
                        | Keycode::End
                        | Keycode::Up
                        | Keycode::Down
                        | Keycode::PageUp
                        | Keycode::PageDown
                ) =>
            {
                let navigation =
                    match keycode {
                        Keycode::Home =>
                            PolicyNavigation::First,

                        Keycode::End =>
                            PolicyNavigation::Last,

                        Keycode::Up =>
                            PolicyNavigation::Previous,

                        Keycode::Down =>
                            PolicyNavigation::Next,

                        Keycode::PageUp =>
                            PolicyNavigation::PagePrevious,

                        Keycode::PageDown =>
                            PolicyNavigation::PageNext,

                        _ =>
                            unreachable!(),
                    };

                // SDL key-repeat is useful here: holding an arrow key should
                // continue moving through the policy list.
                let _ =
                    repeat;

                self.pending_policy_navigation =
                    Some(
                        navigation
                    );
            }

            Event::TextInput {
                ref text,
                ..
            } => {
                self.pending_events.push(
                    egui::Event::Text(
                        text.clone()
                    )
                );
            }

            Event::KeyDown {
                keycode:
                    Some(keycode),
                repeat,
                ..
            } if matches!(
                keycode,
                Keycode::Backspace
                    | Keycode::Delete
                    | Keycode::Left
                    | Keycode::Right
                    | Keycode::Return
                    | Keycode::KpEnter
                    | Keycode::Escape
                    | Keycode::Tab
            ) =>
            {
                let key =
                    match keycode {
                        Keycode::Backspace =>
                            egui::Key::Backspace,

                        Keycode::Delete =>
                            egui::Key::Delete,

                        Keycode::Left =>
                            egui::Key::ArrowLeft,

                        Keycode::Right =>
                            egui::Key::ArrowRight,

                        Keycode::Return
                        | Keycode::KpEnter =>
                            egui::Key::Enter,

                        Keycode::Escape =>
                            egui::Key::Escape,

                        Keycode::Tab =>
                            egui::Key::Tab,

                        _ =>
                            unreachable!(),
                    };

                self.pending_events.push(
                    egui::Event::Key {
                        key,
                        physical_key:
                            None,
                        pressed:
                            true,
                        repeat,
                        modifiers:
                            egui::Modifiers {
                                shift:
                                    self.shift_held,

                                ..Default::default()
                            },
                    }
                );
            }

            Event::KeyUp {
                keycode:
                    Some(keycode),
                ..
            } if matches!(
                keycode,
                Keycode::Backspace
                    | Keycode::Delete
                    | Keycode::Left
                    | Keycode::Right
                    | Keycode::Return
                    | Keycode::KpEnter
                    | Keycode::Escape
                    | Keycode::Tab
            ) =>
            {
                let key =
                    match keycode {
                        Keycode::Backspace =>
                            egui::Key::Backspace,

                        Keycode::Delete =>
                            egui::Key::Delete,

                        Keycode::Left =>
                            egui::Key::ArrowLeft,

                        Keycode::Right =>
                            egui::Key::ArrowRight,

                        Keycode::Return
                        | Keycode::KpEnter =>
                            egui::Key::Enter,

                        Keycode::Escape =>
                            egui::Key::Escape,

                        Keycode::Tab =>
                            egui::Key::Tab,

                        _ =>
                            unreachable!(),
                    };

                self.pending_events.push(
                    egui::Event::Key {
                        key,
                        physical_key:
                            None,
                        pressed:
                            false,
                        repeat:
                            false,
                        modifiers:
                            egui::Modifiers {
                                shift:
                                    self.shift_held,

                                ..Default::default()
                            },
                    }
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
        resolved_bloom: BloomSelection,
        resolved_bloom_intensity: f32,
        resolved_bloom_threshold: f32,
        resolved_invert_colors: bool,
        resolved_flip_horizontal: bool,
        resolved_flip_vertical: bool,
        resolved_hue_rotation: f32,
        active_texture_selection: Option<(
            crate::parse_texture_specification::TextureSpecification,
            crate::palettes::PaletteColor,
        )>,
        shader_loaded: bool,
        texture_required: bool,
        screensaver_policy_exists: bool,
        wallpaper_policy_exists: bool,
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

        let centered_position =
            egui::pos2(
                ((screen_size_points.x - initial_size.x) * 0.5)
                    .max(0.0),
                ((screen_size_points.y - initial_size.y) * 0.5)
                    .max(0.0),
            );

        let maximum_window_x =
            (screen_size_points.x - initial_size.x)
                .max(0.0);

        let maximum_window_y =
            (screen_size_points.y - initial_size.y)
                .max(0.0);

        let initial_position =
            self.window_position
                .unwrap_or(
                    centered_position
                );

        let initial_position =
            egui::pos2(
                initial_position.x.clamp(
                    0.0,
                    maximum_window_x,
                ),
                initial_position.y.clamp(
                    0.0,
                    maximum_window_y,
                ),
            );

        let mut observed_window_position:
            Option<egui::Pos2> =
            None;

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

        let mut bloom_intensity_drag_state =
            self.bloom_intensity_drag_state;

        let mut bloom_threshold_drag_state =
            self.bloom_threshold_drag_state;

        let mut policy_target =
            self.policy_target;

        let mut status_message =
            self.status_message.clone();

        let mut texture =
            self.texture;

        let mut palette =
            self.palette;

        let mut palette_hex_input =
            self.palette_hex_input.clone();

        let mut color_picker_preview =
            self.color_picker_preview;

        let mut primitive_count =
            self.primitive_count;

        let mut anti_aliasing =
            self.anti_aliasing;

        let mut dithering =
            self.dithering;

        let mut color_precision =
            self.color_precision;

        let mut bloom =
            self.bloom;

        let mut bloom_intensity =
            self.bloom_intensity;

        let mut bloom_threshold =
            self.bloom_threshold;

        let mut invert_colors =
            self.invert_colors;

        let mut flip_horizontal =
            self.flip_horizontal;

        let mut flip_vertical =
            self.flip_vertical;

        let mut hue_rotation =
            self.hue_rotation;

        let mut hue_rotation_drag_state =
            self.hue_rotation_drag_state;


        // A shader physically located in one of Screenshaver's managed
        // runtime folders has exactly one available policy target.  Enforce
        // that target here as well as in edit_shader.rs so the Control Center
        // cannot display "Select..." or retain a stale opposite target.
        let forced_policy_target =
            match (
                screensaver_target_available,
                wallpaper_target_available,
            ) {
                (
                    true,
                    false,
                ) => {
                    Some(
                        PolicyTarget::Screensaver
                    )
                }

                (
                    false,
                    true,
                ) => {
                    Some(
                        PolicyTarget::Wallpaper
                    )
                }

                _ => {
                    None
                }
            };


        if let Some(
            forced_policy_target
        ) =
            forced_policy_target
        {
            policy_target =
                Some(
                    forced_policy_target
                );

            self.policy_target =
                Some(
                    forced_policy_target
                );


            let forced_policy_exists =
                match forced_policy_target {
                    PolicyTarget::Screensaver => {
                        screensaver_policy_exists
                    }

                    PolicyTarget::Wallpaper => {
                        wallpaper_policy_exists
                    }

                    PolicyTarget::Unassigned => {
                        self.selected_policy_row
                            .as_ref()
                            .is_some_and(
                                |row| row.unassigned
                            )
                    }
                };


            self.policy_creation_pending =
                !forced_policy_exists;
        }


        if self.initial_configuration.is_none() {
            anti_aliasing =
                resolved_anti_aliasing;

            dithering =
                resolved_dithering;

            color_precision =
                resolved_color_precision;

            bloom =
                resolved_bloom;

            bloom_intensity =
                resolved_bloom_intensity;

            bloom_threshold =
                resolved_bloom_threshold;

            invert_colors =
                resolved_invert_colors;

            flip_horizontal =
                resolved_flip_horizontal;

            flip_vertical =
                resolved_flip_vertical;

            hue_rotation =
                resolved_hue_rotation;

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

                palette_hex_input =
                    palette.name();

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

        let mut bulk_save_requested =
            false;

        let mut cancel_requested =
            false;

        let mut delete_requested =
            false;

        let mut browse_shader_requested =
            false;

        let mut bulk_create_browse_requested =
            false;

        let mut bulk_create_requested:
            Option<BulkCreateRequest> =
            None;

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
                            bloom,
                            bloom_intensity,
                            bloom_threshold,
                            invert_colors,
                            flip_horizontal,
                            flip_vertical,
                            hue_rotation,
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

        let mut restore_selected_policy_scroll =
            self.restore_selected_policy_scroll;

        let mut bulk_selected_policy_rows =
            self.bulk_selected_policy_rows.clone();

        let mut pending_policy_navigation =
            self.pending_policy_navigation.take();

        let mut pending_confirmation =
            self.pending_confirmation.clone();

        let mut pending_policy_clone =
            self.pending_policy_clone.clone();

        let mut pending_policy_rename =
            self.pending_policy_rename.clone();

        let mut policy_row_command_requested:
            Option<(PolicyRowReference, PolicyRowCommand)> =
            None;

        let mut clone_policy_requested:
            Option<(PolicyRowReference, String)> =
            None;

        let mut rename_policy_requested:
            Option<(PolicyRowReference, String)> =
            None;

        let mut control_configuration_save_requested =
            false;

        let mut exit_after_save_requested =
            false;

        let mut exit_discard_requested =
            false;

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

        let mut texture_thumbnail =
            self.texture_thumbnail.clone();

        let mut texture_thumbnail_key =
            self.texture_thumbnail_key;


        let editor_title =
            "Screenshaver Control Center (ESC or Q to exit)";


        let bulk_edit_mode =
            bulk_selected_policy_rows.len() > 1;


        if bulk_edit_mode {
            if self.bulk_edit_baseline.is_none() {
                self.bulk_edit_baseline =
                    Some(
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
                            bloom,
                            bloom_intensity,
                            bloom_threshold,
                            invert_colors,
                            flip_horizontal,
                            flip_vertical,
                            hue_rotation,
                        )
                    );

                // Policy Target has special Bulk Edit semantics.  A blank
                // target means "leave every checked policy's target unchanged."
                // The original loaded-policy target remains in the baseline
                // and is restored when Bulk Edit ends.
                policy_target =
                    None;

                policy_target_change_requested =
                    None;

                status_message =
                    "Bulk Edit Mode active-- click Cancel to return to Single Edit mode."
                        .to_string();
            }
        } else {
            if let Some(
                suspended_baseline
            ) =
                self.bulk_edit_baseline
                    .take()
            {
                // Bulk Edit ended because fewer than two policy rows remain
                // checked.  Discard unsaved Bulk Edit working values and
                // restore the exact single-policy editor state that existed
                // before Bulk Edit began.
                displayed_fps =
                    suspended_baseline.fps;

                displayed_animation_speed =
                    suspended_baseline.animation_speed;

                displayed_render_scale =
                    suspended_baseline.render_scale;

                policy_target =
                    suspended_baseline.policy_target;

                texture =
                    suspended_baseline.texture;

                palette =
                    suspended_baseline.palette;

                primitive_count =
                    suspended_baseline.primitive_count;

                anti_aliasing =
                    suspended_baseline.anti_aliasing;

                dithering =
                    suspended_baseline.dithering;

                color_precision =
                    suspended_baseline.color_precision;

                bloom =
                    suspended_baseline.bloom;

                bloom_intensity =
                    suspended_baseline.bloom_intensity;

                bloom_threshold =
                    suspended_baseline.bloom_threshold;

                invert_colors =
                    suspended_baseline.invert_colors;

                flip_horizontal =
                    suspended_baseline.flip_horizontal;

                flip_vertical =
                    suspended_baseline.flip_vertical;

                hue_rotation =
                    suspended_baseline.hue_rotation;

                palette_hex_input =
                    palette
                        .palette()
                        .to_hex();

                let palette_color =
                    palette
                        .palette();

                color_picker_preview =
                    egui::Color32::from_rgb(
                        palette_color.red(),
                        palette_color.green(),
                        palette_color.blue(),
                    );

                fps_drag_state =
                    None;

                animation_speed_drag_state =
                    None;

                render_scale_drag_state =
                    None;

                bloom_intensity_drag_state =
                    None;

                bloom_threshold_drag_state =
                    None;

                hue_rotation_drag_state =
                    None;
            }
        }

        let bulk_edit_baseline =
            self.bulk_edit_baseline;


        let bulk_texture_required =
            if bulk_edit_mode {
                bulk_selected_policy_rows
                    .iter()
                    .any(
                        |selected| {
                            policy_rows
                                .iter()
                                .any(
                                    |row| {
                                        row.policy_id == selected.policy_id
                                            && row.texture
                                    }
                                )
                        }
                    )
            } else {
                texture_required
            };


        let full_output =
            self.context.run(
                raw_input,
                |context| {
                    let initial_rect =
                        egui::Rect::from_min_size(
                            initial_position,
                            initial_size,
                        );

                    let main_window_response =
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
                    .current_pos(
                        initial_position
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
                                    && !self.pending_exit_confirmation
                                    && !self.pending_bulk_save_confirmation
                                    && self.pending_bulk_create_candidates.is_none()
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
                                    bloom,
                                    bloom_intensity,
                                    bloom_threshold,
                                    invert_colors,
                                    flip_horizontal,
                                    flip_vertical,
                                    hue_rotation,
                                );

                            let configuration_changed =
                                current_configuration
                                    .differs_from(
                                        baseline_configuration
                                    );

                            let mandatory_information_complete =
                                shader_loaded
                                    && policy_target.is_some();

                            let current_policy_exists =
                                match policy_target {
                                    Some(PolicyTarget::Screensaver) => {
                                        screensaver_policy_exists
                                    }

                                    Some(PolicyTarget::Wallpaper) => {
                                        wallpaper_policy_exists
                                    }

                                    Some(PolicyTarget::Unassigned) => {
                                        selected_policy_row
                                            .as_ref()
                                            .is_some_and(
                                                |row| {
                                                    row.unassigned
                                                }
                                            )
                                    }

                                    None => {
                                        false
                                    }
                                };

                            let mut pending_bulk_changes =
                                bulk_edit_baseline
                                    .map(
                                        |baseline| {
                                            current_configuration
                                                .bulk_changes_from(
                                                    baseline
                                                )
                                        }
                                    )
                                    .unwrap_or_default();

                            // Blank is the Bulk Edit "no Policy Target change"
                            // sentinel.  Any explicit target choice is dirty.
                            pending_bulk_changes.policy_target =
                                bulk_edit_mode
                                    && policy_target.is_some();


                            let policy_dirty =
                                if bulk_edit_mode {
                                    pending_bulk_changes.any()
                                } else {
                                    configuration_changed
                                        || self.policy_creation_pending
                                };


                            let can_save =
                                if bulk_edit_mode {
                                    pending_bulk_changes.any()
                                } else {
                                    mandatory_information_complete
                                        && (
                                            configuration_changed
                                                || !current_policy_exists
                                        )
                                };


                            let can_cancel =
                                if bulk_edit_mode {
                                    true
                                } else {
                                    configuration_changed
                                };

                            // -----------------------------------------------------------------
                            // Permanent header: Load Shader + Policy Target + Shader Information
                            // -----------------------------------------------------------------
                            draw_compact_header(
                                ui,
                                metrics,
                                shader_information,
                                selected_policy_row.as_ref(),
                                &self.branding_texture,
                                self.branding_aspect_ratio,
                                configuration_changed,
                                recent_shader_paths,
                                &mut policy_target,
                                bulk_edit_mode,
                                bulk_edit_baseline,
                                screensaver_target_available,
                                wallpaper_target_available,
                                screensaver_target_session_restricted,
                                wallpaper_target_session_restricted,
                                &mut browse_shader_requested,
                                &mut bulk_create_browse_requested,
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
                                policy_target.is_some()
                                    || bulk_edit_mode;

                            // -------------------------------------------------------------
                            // Embedded notebook corresponding to the Qt Designer QTabWidget.
                            // -------------------------------------------------------------
                            draw_editor_tab_bar(
                                ui,
                                metrics,
                                &mut active_tab,
                                bulk_edit_mode,
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
                                                &mut restore_selected_policy_scroll,
                                                &mut bulk_selected_policy_rows,
                                                &mut pending_policy_navigation,
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
                                                        bulk_edit_baseline,
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
                                                        bulk_texture_required,
                                                        &mut texture,
                                                        &mut palette,
                                                        &mut primitive_count,
                                                        bulk_edit_baseline,
                                                        &mut hover_help_message,
                                                    );

                                                    ui.add_space(
                                                        metrics.row_gap
                                                    );


                                                    ui.add_enabled_ui(
                                                        bulk_texture_required,
                                                        |ui| {
                                                            if palette
                                                                != self.palette
                                                            {
                                                                palette_hex_input =
                                                                    palette.name();
                                                            }


                                                            ensure_texture_thumbnail(
                                                                context,
                                                                texture,
                                                                palette,
                                                                primitive_count,
                                                                &mut texture_thumbnail,
                                                                &mut texture_thumbnail_key,
                                                                &mut status_message,
                                                            );


                                                            let palette_scope =
                                                                ui.scope(
                                                                    |ui| {
                                                                        draw_color_picker_placeholder(
                                                                            ui,
                                                                            metrics,
                                                                            &mut palette,
                                                                            &mut palette_hex_input,
                                                                            &mut color_picker_preview,
                                                                            texture_thumbnail.as_ref(),
                                                                            &mut hover_help_message,
                                                                        );
                                                                    },
                                                                );

                                                            if bulk_edit_baseline
                                                                .is_some_and(
                                                                    |baseline| {
                                                                        palette != baseline.palette
                                                                    }
                                                                )
                                                            {
                                                                editor_theme::paint_bulk_edit_border(
                                                                    ui,
                                                                    palette_scope.response.rect,
                                                                    metrics.scale,
                                                                );
                                                            }
                                                        },
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
                                                        shift_held,
                                                        &mut anti_aliasing,
                                                        &mut dithering,
                                                        &mut color_precision,
                                                        &mut bloom,
                                                        &mut bloom_intensity,
                                                        &mut bloom_intensity_drag_state,
                                                        &mut bloom_threshold,
                                                        &mut bloom_threshold_drag_state,
                                                        &mut invert_colors,
                                                        &mut flip_horizontal,
                                                        &mut flip_vertical,
                                                        &mut hue_rotation,
                                                        &mut hue_rotation_drag_state,
                                                        bulk_edit_baseline,
                                                        &mut hover_help_message,
                                                    );
                                                },
                                            );
                                        }

                                        EditorTab::Config => {
                                            ui.add_enabled_ui(
                                                !bulk_edit_mode,
                                                |ui| {
                                                    crate::nested_tabs::draw_configuration(
                                                        ui,
                                                        &mut control_configuration,
                                                        control_configuration_baseline.as_ref(),
                                                        policy_rows,
                                                        &mut control_configuration_save_requested,
                                                        &mut status_message,
                                                    );
                                                },
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

                            if !bulk_edit_mode
                                && !can_save
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

                            let control_configuration_dirty =
                                control_configuration.as_ref()
                                    .zip(control_configuration_baseline.as_ref())
                                    .map(|(current, baseline)| current != baseline)
                                    .unwrap_or(false);


                            // --------------------------------------------------------
                            // Permanent command row from the approved Qt mock-up.
                            // Policy/Config state is shown in a compact text box on
                            // the right side of this same row.
                            // --------------------------------------------------------
                            let bulk_edit_mode =
                                bulk_selected_policy_rows.len() > 1;


                            draw_compact_action_row(
                                ui,
                                metrics,
                                can_save,
                                can_cancel,
                                bulk_edit_mode,
                                &mut bulk_selected_policy_rows,
                                &mut self.pending_bulk_save_confirmation,
                                &mut save_requested,
                                &mut cancel_requested,
                                &mut displayed_fps,
                                &mut displayed_animation_speed,
                                &mut displayed_render_scale,
                                &mut policy_target,
                                &mut policy_target_change_requested,
                                &mut texture,
                                &mut palette,
                                &mut primitive_count,
                                &mut anti_aliasing,
                                &mut dithering,
                                &mut color_precision,
                                &mut bloom,
                                &mut bloom_intensity,
                                &mut bloom_threshold,
                                &mut invert_colors,
                                &mut flip_horizontal,
                                &mut flip_vertical,
                                &mut hue_rotation,
                                baseline_configuration,
                                &mut fps_drag_state,
                                &mut animation_speed_drag_state,
                                &mut render_scale_drag_state,
                                &mut bloom_threshold_drag_state,
                                &mut status_message,
                                &mut hover_help_message,
                                shader_information,
                                policy_dirty,
                                control_configuration_dirty,
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

                            draw_compact_status_row(
                                ui,
                                metrics,
                                displayed_status,
                            );
                        }
                    );

                    if let Some(main_window_response) =
                        main_window_response
                    {
                        observed_window_position =
                            Some(
                                main_window_response
                                    .response
                                    .rect
                                    .min
                            );
                    }

                    draw_policy_confirmation_modal(
                        context,
                        &mut pending_confirmation,
                        &mut policy_row_command_requested,
                    );


                    draw_policy_clone_modal(
                        context,
                        &mut pending_policy_clone,
                        &mut clone_policy_requested,
                    );


                    draw_policy_rename_modal(
                        context,
                        &mut pending_policy_rename,
                        &mut rename_policy_requested,
                    );


                    draw_bulk_save_confirmation_modal(
                        context,
                        &mut self.pending_bulk_save_confirmation,
                        &mut bulk_selected_policy_rows,
                        policy_rows,
                        &mut bulk_save_requested,
                    );


                    draw_bulk_create_confirmation_modal(
                        context,
                        &mut self.pending_bulk_create_candidates,
                        &mut self.pending_bulk_create_external_target,
                        self.pending_bulk_create_rejected_count,
                        &mut bulk_create_requested,
                    );


                    let policy_dirty =
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
                            bloom,
                            bloom_intensity,
                            bloom_threshold,
                            invert_colors,
                            flip_horizontal,
                            flip_vertical,
                            hue_rotation,
                        )
                        .differs_from(
                            baseline_configuration
                        )
                        || self.policy_creation_pending;


                    let control_configuration_dirty =
                        control_configuration.as_ref()
                            .zip(control_configuration_baseline.as_ref())
                            .map(
                                |(
                                    current,
                                    baseline,
                                )| {
                                    current != baseline
                                }
                            )
                            .unwrap_or(false);


                    if self.close_requested
                        || !window_open
                    {
                        self.close_requested =
                            false;


                        if policy_dirty
                            || control_configuration_dirty
                        {
                            window_open =
                                true;

                            self.pending_exit_confirmation =
                                true;
                        } else {
                            window_open =
                                false;
                        }
                    }


                    draw_exit_confirmation_modal(
                        context,
                        &mut self.pending_exit_confirmation,
                        policy_dirty,
                        control_configuration_dirty,
                        policy_target.is_some(),
                        bulk_edit_mode,
                        &mut save_requested,
                        &mut bulk_save_requested,
                        &mut control_configuration_save_requested,
                        &mut exit_after_save_requested,
                        &mut exit_discard_requested,
                    );
                }
            );


        if let Some(position) =
            observed_window_position
        {
            let normalized_position =
                egui::pos2(
                    position.x.round(),
                    position.y.round(),
                );

            let position_changed =
                self.window_position
                    .map(
                        |previous| {
                            (previous.x
                                - normalized_position.x)
                                .abs()
                                >= 0.5
                                || (previous.y
                                    - normalized_position.y)
                                    .abs()
                                    >= 0.5
                        }
                    )
                    .unwrap_or(true);

            if position_changed {
                self.window_position =
                    Some(
                        normalized_position
                    );

                self.window_position_changed_at =
                    Some(
                        Instant::now()
                    );
            }
        }


        if let (
            Some(position),
            Some(changed_at),
        ) = (
            self.window_position,
            self.window_position_changed_at,
        ) {
            if Instant::now()
                .saturating_duration_since(
                    changed_at
                )
                >= Duration::from_millis(
                    250
                )
            {
                self.persisted_window_position =
                    Some(
                        (
                            position.x.round()
                                as i32,
                            position.y.round()
                                as i32,
                        )
                    );

                self.window_position_changed_at =
                    None;
            }
        }


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

        self.restore_selected_policy_scroll =
            restore_selected_policy_scroll;

        let bulk_selected_policy_rows_for_output =
            bulk_selected_policy_rows.clone();


        let current_editor_configuration =
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
                bloom,
                bloom_intensity,
                bloom_threshold,
                invert_colors,
                flip_horizontal,
                flip_vertical,
                hue_rotation,
            );

        let mut bulk_edit_changes =
            bulk_edit_baseline
                .map(
                    |baseline| {
                        current_editor_configuration
                            .bulk_changes_from(baseline)
                    }
                )
                .unwrap_or_default();

        // Policy Target uses a deliberate blank sentinel in Bulk Edit.
        // Blank means "no target change"; any explicit target is pending.
        bulk_edit_changes.policy_target =
            bulk_edit_mode
                && policy_target.is_some();


        self.bulk_selected_policy_rows =
            bulk_selected_policy_rows;

        self.pending_confirmation =
            pending_confirmation;

        self.pending_policy_clone =
            pending_policy_clone;

        self.pending_policy_rename =
            pending_policy_rename;

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

        self.bloom_intensity_drag_state =
            bloom_intensity_drag_state;

        self.bloom_threshold_drag_state =
            bloom_threshold_drag_state;

        self.policy_target =
            policy_target;

        self.status_message =
            status_message;

        self.texture =
            texture;

        self.texture_thumbnail =
            texture_thumbnail;

        self.texture_thumbnail_key =
            texture_thumbnail_key;

        self.palette =
            palette;

        self.palette_hex_input =
            palette_hex_input;

        self.color_picker_preview =
            color_picker_preview;

        self.primitive_count =
            primitive_count;

        self.anti_aliasing =
            anti_aliasing;

        self.dithering =
            dithering;

        self.color_precision =
            color_precision;

        self.bloom =
            bloom;

        self.bloom_intensity =
            bloom_intensity;

        self.bloom_threshold =
            bloom_threshold;
        self.invert_colors =
            invert_colors;

        self.flip_horizontal =
            flip_horizontal;

        self.flip_vertical =
            flip_vertical;

        self.hue_rotation =
            hue_rotation;

        self.hue_rotation_drag_state =
            hue_rotation_drag_state;

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

            bloom,

            bloom_intensity,

            bloom_threshold,

            invert_colors,

            flip_horizontal,

            flip_vertical,

            hue_rotation,

            policy_target_change_requested,

            save_requested,

            bulk_save_requested,

            bulk_edit_changes,

            bulk_selected_policy_rows:
                bulk_selected_policy_rows_for_output,

            cancel_requested,

            delete_requested,

            browse_shader_requested,

            bulk_create_browse_requested,

            bulk_create_requested,

            recent_shader_requested,

            clear_recent_files_requested,

            refresh_shader_requested,

            policy_row_command_requested,

            clone_policy_requested,

            rename_policy_requested,

            policy_dirty:
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
                    bloom,
                    bloom_intensity,
                    bloom_threshold,
                    invert_colors,
                    flip_horizontal,
                    flip_vertical,
                    hue_rotation,
                )
                .differs_from(
                    baseline_configuration
                )
                || self.policy_creation_pending,

            control_configuration_dirty:
                control_configuration.as_ref()
                    .zip(control_configuration_baseline.as_ref())
                    .map(|(current, baseline)| current != baseline)
                    .unwrap_or(false),

            control_configuration,

            control_configuration_save_requested,

            exit_after_save_requested,

            exit_discard_requested,

            window_open:
                self.window_open,
        }
    }


    pub fn restore_policy_list_state(
        &mut self,
        sort_column: &str,
        sort_ascending: bool,
        selected_policy_row: Option<PolicyRowReference>,
        window_x: Option<i32>,
        window_y: Option<i32>,
    ) {
        self.policy_sort_column =
            match sort_column {
                "status" => {
                    PolicySortColumn::Status
                }

                "texture" => {
                    PolicySortColumn::Texture
                }

                "policy_type" => {
                    PolicySortColumn::PolicyType
                }

                _ => {
                    PolicySortColumn::Filename
                }
            };

        self.policy_sort_ascending =
            sort_ascending;

        self.selected_policy_row =
            selected_policy_row;

        self.transient_policy_selection_persisted_row =
            None;

        self.restore_selected_policy_scroll =
            self.selected_policy_row.is_some();

        self.persisted_window_position =
            window_x.zip(
                window_y
            );

        self.window_position =
            self.persisted_window_position
                .map(
                    |(
                        x,
                        y,
                    )| {
                        egui::pos2(
                            x as f32,
                            y as f32,
                        )
                    }
                );

        self.window_position_changed_at =
            None;
    }


    pub fn select_policy_row_transiently(
        &mut self,
        row: PolicyRowReference,
    ) {
        // Clone focus is now treated as an ordinary selection so state.json
        // follows the newly-created policy instead of preserving the original.
        self.transient_policy_selection_persisted_row =
            None;

        self.selected_policy_row =
            Some(
                row
            );

        self.restore_selected_policy_scroll =
            true;
    }


    pub fn active_selected_policy_row(
        &self,
    ) -> Option<PolicyRowReference> {
        self.selected_policy_row.clone()
    }


    pub fn policy_list_state_snapshot(
        &self,
    ) -> PolicyListStateSnapshot {
        PolicyListStateSnapshot {
            sort_column:
                match self.policy_sort_column {
                    PolicySortColumn::Filename =>
                        "filename",

                    PolicySortColumn::Status =>
                        "status",

                    PolicySortColumn::Texture =>
                        "texture",

                    PolicySortColumn::PolicyType =>
                        "policy_type",
                }
                .to_string(),

            sort_ascending:
                self.policy_sort_ascending,

            selected_policy_row:
                self.selected_policy_row.clone(),

            window_x:
                self.persisted_window_position
                    .map(
                        |(
                            x,
                            _,
                        )| {
                            x
                        }
                    ),

            window_y:
                self.persisted_window_position
                    .map(
                        |(
                            _,
                            y,
                        )| {
                            y
                        }
                    ),
        }
    }


    pub fn request_close(
        &mut self,
    ) {

        self.commit_window_position();

        self.close_requested =
            true;
    }


    fn commit_window_position(
        &mut self,
    ) {

        if let Some(position) =
            self.window_position
        {
            self.persisted_window_position =
                Some(
                    (
                        position.x.round()
                            as i32,
                        position.y.round()
                            as i32,
                    )
                );

            self.window_position_changed_at =
                None;
        }
    }


    pub fn begin_policy_clone(
        &mut self,
        row: PolicyRowReference,
        suggested_name: String,
    ) {
        self.pending_policy_clone =
            Some(
                PendingPolicyClone {
                    row,
                    policy_name:
                        suggested_name,
                    validation_message:
                        String::new(),
                }
            );
    }


    pub fn complete_policy_clone(
        &mut self,
    ) {
        self.pending_policy_clone =
            None;
    }


    pub fn set_policy_clone_validation_message(
        &mut self,
        message: impl Into<String>,
    ) {
        if let Some(
            pending
        ) =
            self.pending_policy_clone
                .as_mut()
        {
            pending.validation_message =
                message.into();
        }
    }


    pub fn begin_policy_rename(
        &mut self,
        row: PolicyRowReference,
    ) {
        self.pending_policy_rename =
            Some(
                PendingPolicyRename {
                    policy_name:
                        row.policy_key.clone(),
                    row,
                    validation_message:
                        String::new(),
                }
            );
    }


    pub fn complete_policy_rename(
        &mut self,
    ) {
        self.pending_policy_rename =
            None;
    }


    pub fn set_policy_rename_validation_message(
        &mut self,
        message: impl Into<String>,
    ) {
        if let Some(
            pending
        ) =
            self.pending_policy_rename
                .as_mut()
        {
            pending.validation_message =
                message.into();
        }
    }


    pub fn select_policy_row_persistently(
        &mut self,
        row: PolicyRowReference,
    ) {
        self.transient_policy_selection_persisted_row =
            None;

        self.selected_policy_row =
            Some(
                row
            );

        self.restore_selected_policy_scroll =
            true;
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
        bloom: BloomSelection,
        bloom_intensity: f32,
        bloom_threshold: f32,
        invert_colors: bool,
        flip_horizontal: bool,
        flip_vertical: bool,
        hue_rotation: f32,
        active_texture_selection: Option<(
            crate::parse_texture_specification::TextureSpecification,
            crate::palettes::PaletteColor,
        )>,
        policy_exists: bool,
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

        self.bloom =
            bloom;

        self.bloom_intensity =
            bloom_intensity.clamp(
                crate::render_bloom::BLOOM_INTENSITY_MIN,
                crate::render_bloom::BLOOM_INTENSITY_MAX,
            );

        self.invert_colors = invert_colors;

        self.flip_horizontal = flip_horizontal;

        self.flip_vertical = flip_vertical;

        self.hue_rotation =
            crate::postprocess_shader::validate_hue_rotation(hue_rotation)
                .unwrap_or(crate::postprocess_shader::HUE_ROTATION_DEFAULT);

        self.bloom_threshold =
            bloom_threshold.clamp(
                crate::render_bloom::BLOOM_THRESHOLD_MIN,
                crate::render_bloom::BLOOM_THRESHOLD_MAX,
            );

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

            self.palette_hex_input =
                self.palette.name();

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

        self.bloom_intensity_drag_state =
            None;

        self.bloom_threshold_drag_state =
            None;

        self.status_message =
            status_message.into();

        self.policy_creation_pending =
            self.policy_target.is_some()
                && !policy_exists;

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
                    self.bloom,
                    self.bloom_intensity,
                    self.bloom_threshold,
                    self.invert_colors,
                    self.flip_horizontal,
                    self.flip_vertical,
                    self.hue_rotation,
                )
            );
    }


    pub fn begin_bulk_policy_creation(
        &mut self,
        candidates: Vec<BulkCreateCandidate>,
        rejected_count: usize,
    ) {
        self.pending_bulk_create_candidates =
            Some(
                candidates
            );

        self.pending_bulk_create_rejected_count =
            rejected_count;

        self.pending_bulk_create_external_target =
            None;
    }


    pub fn complete_bulk_policy_creation(
        &mut self,
    ) {
        self.pending_bulk_create_candidates =
            None;

        self.pending_bulk_create_rejected_count =
            0;

        self.pending_bulk_create_external_target =
            None;
    }


    pub fn complete_bulk_save(
        &mut self,
        active_policy_was_selected: bool,
    ) {
        // Preserve the exact editor state that existed when Bulk Edit began.
        // This may contain unsaved single-policy changes and therefore must
        // not be replaced with initial_configuration after a successful
        // Bulk Edit operation.
        let suspended_editor_state =
            self.bulk_edit_baseline;


        self.bulk_selected_policy_rows.clear();

        self.pending_bulk_save_confirmation =
            false;

        self.bulk_edit_baseline =
            None;


        if active_policy_was_selected {
            self.accept_current_configuration();
            return;
        }


        let Some(
            baseline
        ) =
            suspended_editor_state
        else {
            return;
        };


        self.displayed_fps =
            Some(
                baseline.fps
            );

        self.displayed_animation_speed =
            Some(
                baseline.animation_speed
            );

        self.displayed_render_scale =
            Some(
                baseline.render_scale
            );

        self.policy_target =
            baseline.policy_target;

        self.texture =
            baseline.texture;

        self.palette =
            baseline.palette;

        self.palette_hex_input =
            baseline.palette
                .palette()
                .to_hex();

        self.color_picker_preview =
            egui::Color32::from_rgb(
                baseline.palette
                    .palette()
                    .red(),
                baseline.palette
                    .palette()
                    .green(),
                baseline.palette
                    .palette()
                    .blue(),
            );

        self.primitive_count =
            baseline.primitive_count;

        self.anti_aliasing =
            baseline.anti_aliasing;

        self.dithering =
            baseline.dithering;

        self.color_precision =
            baseline.color_precision;

        self.bloom =
            baseline.bloom;

        self.bloom_intensity =
            baseline.bloom_intensity;

        self.bloom_threshold =
            baseline.bloom_threshold;

        self.invert_colors =
            baseline.invert_colors;

        self.flip_horizontal =
            baseline.flip_horizontal;

        self.flip_vertical =
            baseline.flip_vertical;

        self.hue_rotation =
            baseline.hue_rotation;

        self.fps_drag_state =
            None;

        self.animation_speed_drag_state =
            None;

        self.render_scale_drag_state =
            None;

        self.bloom_intensity_drag_state =
            None;

        self.bloom_threshold_drag_state =
            None;

        self.hue_rotation_drag_state =
            None;
    }


    pub fn accept_current_configuration(
        &mut self,
    ) {
        self.policy_creation_pending =
            false;


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
                        self.bloom,
                        self.bloom_intensity,
                        self.bloom_threshold,
                        self.invert_colors,
                        self.flip_horizontal,
                        self.flip_vertical,
                        self.hue_rotation,
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
    bulk_edit_mode: bool,
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

                let tab_enabled =
                    !bulk_edit_mode
                        || tab != EditorTab::Config;


                let response =
                    ui.add_enabled(
                        tab_enabled,
                        egui::SelectableLabel::new(
                            selected,
                            egui::RichText::new(
                                label
                            )
                            .strong(),
                        ),
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
    selected_policy_row: Option<&PolicyRowReference>,
    branding_texture: &egui::TextureHandle,
    branding_aspect_ratio: f32,
    configuration_changed: bool,
    recent_shader_paths: &[PathBuf],
    policy_target: &mut Option<PolicyTarget>,
    bulk_edit_mode: bool,
    bulk_edit_baseline: Option<EditorConfiguration>,
    screensaver_target_available: bool,
    wallpaper_target_available: bool,
    screensaver_target_session_restricted: bool,
    wallpaper_target_session_restricted: bool,
    browse_shader_requested: &mut bool,
    bulk_create_browse_requested: &mut bool,
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
    
                        if ui.button(
                            "Create Policies from Multiple Shaders..."
                        )
                        .clicked()
                        {
                            if configuration_changed {
                                *status_message =
                                    "Save or cancel the current changes before creating policies."
                                        .to_string();
                            } else {
                                *bulk_create_browse_requested =
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
                        match *policy_target {
                            Some(
                                PolicyTarget::Screensaver
                            ) =>
                                "Screensaver",
                            Some(
                                PolicyTarget::Wallpaper
                            ) =>
                                "Wallpaper",
                            Some(
                                PolicyTarget::Unassigned
                            ) =>
                                "Unassigned",
                            None if bulk_edit_mode =>
                                "",

                            None =>
                                "Select...",
                        };

                    // The canonical default.glsl policies are fallbacks.
                    // Their rendering settings remain editable, but their
                    // Policy Target assignments are immutable. Bulk Edit is
                    // intentionally not disabled here because other checked
                    // policies may still be retargeted.
                    let policy_target_locked =
                        !bulk_edit_mode
                            && selected_policy_row
                                .map(
                                    |row| {
                                        crate::manage_policies::is_protected_default_policy(
                                            row.policy_id
                                        )
                                        .unwrap_or(false)
                                    }
                                )
                                .unwrap_or(false);


                    let target_response =
                        ui.add_enabled_ui(
                            !policy_target_locked,
                            |ui| {
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
                                                        if bulk_edit_mode {
                                                            let no_change_response =
                                                                ui.selectable_label(
                                                                    policy_target.is_none(),
                                                                    "No Change",
                                                                );

                                                            update_hover_help(
                                                                &no_change_response,
                                                                hover_help_message,
                                                                "Leave Policy Target unchanged for every checked policy.",
                                                            );

                                                            if no_change_response.clicked() {
                                                                *policy_target =
                                                                    None;

                                                                *policy_target_change_requested =
                                                                    None;

                                                                ui.close();
                                                            }

                                                            ui.separator();
                                                        }

                                                        let screensaver_response =
                                                            ui.add_enabled(
                                                                bulk_edit_mode
                                                                    || screensaver_target_available,
                                                                egui::SelectableLabel::new(
                                                                    *policy_target
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
                                                            && *policy_target
                                                                != Some(
                                                                    PolicyTarget::Screensaver
                                                                )
                                                        {
                                                            if bulk_edit_mode {
                                                                *policy_target =
                                                                    Some(
                                                                        PolicyTarget::Screensaver
                                                                    );

                                                                // Bulk Edit target dirty-state is derived from
                                                                // the persistent target versus the Bulk baseline.
                                                                // Do not emit a single-policy target-switch event.
                                                                *policy_target_change_requested =
                                                                    None;
                                                            } else if configuration_changed
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
                                                                bulk_edit_mode
                                                                    || wallpaper_target_available,
                                                                egui::SelectableLabel::new(
                                                                    *policy_target
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
                                                            && *policy_target
                                                                != Some(
                                                                    PolicyTarget::Wallpaper
                                                                )
                                                        {
                                                            if bulk_edit_mode {
                                                                *policy_target =
                                                                    Some(
                                                                        PolicyTarget::Wallpaper
                                                                    );

                                                                // Bulk Edit target dirty-state is derived from
                                                                // the persistent target versus the Bulk baseline.
                                                                // Do not emit a single-policy target-switch event.
                                                                *policy_target_change_requested =
                                                                    None;
                                                            } else if configuration_changed
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

                                                        let unassigned_response =
                                                            ui.selectable_label(
                                                                *policy_target
                                                                    == Some(
                                                                        PolicyTarget::Unassigned
                                                                    ),
                                                                "Unassigned",
                                                            );

                                                        update_hover_help(
                                                            &unassigned_response,
                                                            hover_help_message,
                                                            "Keep this policy and all of its settings, but exclude it from screensaver and wallpaper rendering until it is reassigned.",
                                                        );

                                                        if unassigned_response.clicked()
                                                            && *policy_target
                                                                != Some(
                                                                    PolicyTarget::Unassigned
                                                                )
                                                        {
                                                            if bulk_edit_mode {
                                                                *policy_target =
                                                                    Some(
                                                                        PolicyTarget::Unassigned
                                                                    );

                                                                // Bulk Edit target dirty-state is derived from
                                                                // the persistent target versus the Bulk baseline.
                                                                // Do not emit a single-policy target-switch event.
                                                                *policy_target_change_requested =
                                                                    None;
                                                            } else if configuration_changed
                                                                && policy_target.is_some()
                                                            {
                                                                *status_message =
                                                                    "Save or cancel the current changes before switching policy targets."
                                                                        .to_string();
                                                            } else {
                                                                *policy_target_change_requested =
                                                                    Some(
                                                                        PolicyTarget::Unassigned
                                                                    );
                                                            }

                                                            ui.close();
                                                        }
                                                    },
                                                );

                            },
                        );


                    let bulk_policy_target_changed =
                        bulk_edit_mode
                            && policy_target.is_some();


                    if bulk_policy_target_changed {
                        editor_theme::paint_bulk_edit_border(
                            ui,
                            target_response.response.rect,
                            metrics.scale,
                        );
                    }


                    if policy_target_locked {
                        update_hover_help(
                            &target_response.response,
                            hover_help_message,
                            "default.glsl provides Screenshaver's guaranteed fallback policy. Its Policy Target cannot be changed.",
                        );
                    }


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
        policy_name,
        filename,
        folder,
        shader_type,
    ) =
        if let Some(information) =
            shader_information
        {
            (
                information.policy_name.as_str(),
                information.filename.as_str(),
                information.folder.as_str(),
                information.shader_type.as_str(),
            )
        } else {
            (
                "—",
                "No shader loaded",
                "—",
                "—",
            )
        };

    ui.horizontal_top(
        |ui| {
            ui.vertical(
                |ui| {
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
                            ui.label("Policy Name:");
                            ui.label(policy_name);
                            ui.end_row();

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
                        },
                    );
                },
            );


            ui.with_layout(
                egui::Layout::right_to_left(
                    egui::Align::TOP
                ),
                |ui| {
                    let branding_width =
                        130.0 * metrics.scale;

                    let branding_height =
                        branding_width
                            / branding_aspect_ratio
                                .max(0.001);


                    ui.add(
                        egui::Image::new(
                            branding_texture
                        )
                        .fit_to_exact_size(
                            egui::vec2(
                                branding_width,
                                branding_height,
                            )
                        ),
                    );
                },
            );
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
    bulk_edit_mode: bool,
    bulk_selected_policy_rows: &mut Vec<PolicyRowReference>,
    pending_bulk_save_confirmation: &mut bool,
    save_requested: &mut bool,
    cancel_requested: &mut bool,
    displayed_fps: &mut u32,
    displayed_animation_speed: &mut f32,
    displayed_render_scale: &mut f32,
    policy_target: &mut Option<PolicyTarget>,
    policy_target_change_requested: &mut Option<PolicyTarget>,
    texture: &mut TextureSelection,
    palette: &mut PaletteSelection,
    primitive_count: &mut u32,
    anti_aliasing: &mut AntiAliasingSelection,
    dithering: &mut DitheringSelection,
    color_precision: &mut ColorPrecisionSelection,
    bloom: &mut BloomSelection,
    bloom_intensity: &mut f32,
    bloom_threshold: &mut f32,
    invert_colors: &mut bool,
    flip_horizontal: &mut bool,
    flip_vertical: &mut bool,
    hue_rotation: &mut f32,
    baseline_configuration: EditorConfiguration,
    fps_drag_state: &mut Option<SliderDragState>,
    animation_speed_drag_state: &mut Option<SliderDragState>,
    render_scale_drag_state: &mut Option<SliderDragState>,
    bloom_threshold_drag_state: &mut Option<SliderDragState>,
    status_message: &mut String,
    hover_help_message: &mut Option<&'static str>,
    shader_information: Option<&ShaderInformation>,
    policy_changed: bool,
    control_configuration_dirty: bool,
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
                    if bulk_edit_mode {
                        true
                    } else {
                        can_save
                    },
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
                if bulk_edit_mode {
                    *pending_bulk_save_confirmation =
                        true;
                } else {
                    *save_requested =
                        true;

                    *status_message =
                        "Saving policy..."
                            .to_string();
                }
            }

            let cancel_response =
                ui.add_enabled(
                    if bulk_edit_mode {
                        true
                    } else {
                        can_cancel
                    },
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
                *bloom =
                    baseline_configuration.bloom;
                *bloom_intensity =
                    baseline_configuration.bloom_intensity;

                *bloom_threshold =
                    baseline_configuration.bloom_threshold;
                *invert_colors =
                    baseline_configuration.invert_colors;

                *flip_horizontal =
                    baseline_configuration.flip_horizontal;

                *flip_vertical =
                    baseline_configuration.flip_vertical;

                *hue_rotation =
                    baseline_configuration.hue_rotation;
                *fps_drag_state =
                    None;
                *animation_speed_drag_state =
                    None;
                *render_scale_drag_state =
                    None;
                *bloom_threshold_drag_state =
                    None;
                if bulk_edit_mode {
                    *pending_bulk_save_confirmation =
                        false;

                    bulk_selected_policy_rows.clear();

                    *policy_target_change_requested =
                        None;

                    *status_message =
                        "Bulk Edit Mode canceled"
                            .to_string();
                } else {
                    *status_message =
                        "Changes canceled"
                            .to_string();
                }
            }



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


            ui.with_layout(
                egui::Layout::right_to_left(
                    egui::Align::Center
                ),
                |ui| {
                    egui::Frame::group(
                        ui.style()
                    )
                    .inner_margin(
                        egui::Margin::symmetric(
                            (8.0 * metrics.scale)
                                .round()
                                .clamp(
                                    i8::MIN as f32,
                                    i8::MAX as f32,
                                ) as i8,
                            (3.0 * metrics.scale)
                                .round()
                                .clamp(
                                    i8::MIN as f32,
                                    i8::MAX as f32,
                                ) as i8,
                        )
                    )
                    .show(
                        ui,
                        |ui| {
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
        },
    );
}


fn draw_compact_status_row(
    ui: &mut egui::Ui,
    _metrics: EditorMetrics,
    displayed_status: &str,
) {
    ui.separator();

    // The status bar is now reserved exclusively for transient
    // Information / Warning / Error messages.  Persistent Policy and Config
    // dirty-state indicators are drawn in a dedicated text box beside
    // the Save/Cancel controls.
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
            96,
        )
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
    restore_selected_policy_scroll: &mut bool,
    bulk_selected_rows: &mut Vec<PolicyRowReference>,
    pending_navigation: &mut Option<PolicyNavigation>,
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

    fn row_is_bulk_selected(
        selected_rows: &[PolicyRowReference],
        row: &PolicyRowReference,
    ) -> bool {
        selected_rows.iter()
            .any(
                |selected| {
                    selected.policy_id == row.policy_id
                }
            )
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
                        left.policy_key
                            .to_ascii_lowercase()
                            .cmp(
                                &right.policy_key
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

                    PolicySortColumn::PolicyType => {
                        let left_name =
                            if left.unassigned {
                                "Unassigned"
                            } else {
                                policy_target_name(
                                    left.policy_target
                                )
                            };

                        let right_name =
                            if right.unassigned {
                                "Unassigned"
                            } else {
                                policy_target_name(
                                    right.policy_target
                                )
                            };

                        left_name.cmp(
                            right_name
                        )
                    },
                };

            let ordering =
                ordering.then_with(
                    || {
                        left.policy_id.cmp(
                            &right.policy_id
                        )
                    }
                );

            if *sort_ascending {
                ordering
            } else {
                ordering.reverse()
            }
        },
    );


    let mut keyboard_selected_row:
        Option<PolicyRowReference> =
        None;

    if bulk_selected_rows.len() <= 1 {
        if let Some(navigation) =
            pending_navigation.take()
        {
        if !rows.is_empty() {
            const PAGE_STEP: usize =
                10;

            let current_index =
                selected_row
                    .as_ref()
                    .and_then(
                        |selected| {
                            rows.iter()
                                .position(
                                    |row| {
                                        row.policy_id == selected.policy_id
                                    }
                                )
                        }
                    );

            let new_index =
                match navigation {
                    PolicyNavigation::First =>
                        0,

                    PolicyNavigation::Last =>
                        rows.len() - 1,

                    PolicyNavigation::Previous =>
                        current_index
                            .unwrap_or(0)
                            .saturating_sub(1),

                    PolicyNavigation::Next =>
                        current_index
                            .map(
                                |index| {
                                    (index + 1)
                                        .min(
                                            rows.len() - 1
                                        )
                                }
                            )
                            .unwrap_or(0),

                    PolicyNavigation::PagePrevious =>
                        current_index
                            .unwrap_or(0)
                            .saturating_sub(
                                PAGE_STEP
                            ),

                    PolicyNavigation::PageNext =>
                        current_index
                            .map(
                                |index| {
                                    (index + PAGE_STEP)
                                        .min(
                                            rows.len() - 1
                                        )
                                }
                            )
                            .unwrap_or(0),
                };

            let row =
                &rows[new_index];

            let row_reference =
                PolicyRowReference {
                    policy_id:
                        row.policy_id,

                    policy_key:
                        row.policy_key.clone(),

                    filename:
                        row.filename.clone(),

                    full_path:
                        row.full_path.clone(),

                    policy_target:
                        row.policy_target,

                    unassigned:
                        row.unassigned,
                };

            *selected_row =
                Some(
                    row_reference.clone()
                );

            keyboard_selected_row =
                Some(
                    row_reference
                );
            }
        }
    } else {
        pending_navigation.take();
    }

    let spacing =
        10.0 * metrics.scale;

    let checkbox_width =
        28.0 * metrics.scale;


    let usable_width =
        (
            ui.available_width()
                - 18.0 * metrics.scale
                - checkbox_width
                - spacing * 4.0
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

    // Curated color rows do not need the full standard interactive-control
    // height.  Use a compact row so more swatches fit in the popup without
    // crowding the Textures-tab action buttons below it.
    let row_height =
        (
            ui.spacing()
                .interact_size
                .y
            - 5.0 * metrics.scale
        )
        .max(
            18.0 * metrics.scale
        );

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
                .num_columns(5)
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
                        let all_rows_checked =
                            !rows.is_empty()
                                && rows.iter()
                                    .all(
                                        |row| {
                                            row_is_bulk_selected(
                                                bulk_selected_rows,
                                                &PolicyRowReference {
                                                    policy_id:
                                                        row.policy_id,

                                                    policy_key:
                                                        row.policy_key.clone(),
                                                    filename:
                                                        row.filename.clone(),
                                                    full_path:
                                                        row.full_path.clone(),
                                                    policy_target:
                                                        row.policy_target,

                                                    unassigned:
                                                        row.unassigned,
                                                },
                                            )
                                        }
                                    );


                        let mut header_checked =
                            all_rows_checked;


                        let header_checkbox =
                            ui.allocate_ui_with_layout(
                                egui::vec2(
                                    checkbox_width,
                                    row_height,
                                ),
                                egui::Layout::left_to_right(
                                    egui::Align::Center
                                ),
                                |ui| {
                                    ui.checkbox(
                                        &mut header_checked,
                                        ""
                                    )
                                },
                            )
                            .inner
                            .on_hover_text(
                                if all_rows_checked {
                                    "Clear all policy selections"
                                } else {
                                    "Select all policies"
                                }
                            );


                        if header_checkbox.changed() {
                            if header_checked {
                                bulk_selected_rows.clear();

                                bulk_selected_rows.extend(
                                    rows.iter()
                                        .map(
                                            |row| {
                                                PolicyRowReference {
                                                    policy_id:
                                                        row.policy_id,

                                                    policy_key:
                                                        row.policy_key.clone(),

                                                    filename:
                                                        row.filename.clone(),

                                                    full_path:
                                                        row.full_path.clone(),

                                                    policy_target:
                                                        row.policy_target,

                                                    unassigned:
                                                        row.unassigned,
                                                }
                                            }
                                        )
                                );
                            } else {
                                bulk_selected_rows.clear();
                            }
                        }


                        let filename_header =
                            left_aligned_cell(
                                ui,
                                filename_width,
                                row_height,
                                egui::RichText::new(
                                    header_text(
                                        "Policy Name",
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

                        if bulk_selected_rows.len() <= 1
                            && filename_header.clicked()
                        {
                            apply_sort_request(
                                PolicySortColumn::Filename,
                                sort_column,
                                sort_ascending,
                            );
                        }

                        if bulk_selected_rows.len() <= 1
                            && status_header.clicked()
                        {
                            apply_sort_request(
                                PolicySortColumn::Status,
                                sort_column,
                                sort_ascending,
                            );
                        }

                        if bulk_selected_rows.len() <= 1
                            && texture_header.clicked()
                        {
                            apply_sort_request(
                                PolicySortColumn::Texture,
                                sort_column,
                                sort_ascending,
                            );
                        }

                        if bulk_selected_rows.len() <= 1
                            && policy_header.clicked()
                        {
                            apply_sort_request(
                                PolicySortColumn::PolicyType,
                                sort_column,
                                sort_ascending,
                            );
                        }

                        if rows.is_empty() {
                            ui.allocate_ui_with_layout(
                                egui::vec2(
                                    checkbox_width,
                                    row_height,
                                ),
                                egui::Layout::left_to_right(
                                    egui::Align::Center
                                ),
                                |_ui| {},
                            );

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
                                    policy_id:
                                        row.policy_id,

                                    policy_key:
                                        row.policy_key.clone(),

                                    filename:
                                        row.filename.clone(),

                                    full_path:
                                        row.full_path.clone(),

                                    policy_target:
                                        row.policy_target,

                                    unassigned:
                                        row.unassigned,
                                };

                            let mut bulk_checked =
                                row_is_bulk_selected(
                                    bulk_selected_rows,
                                    &row_reference,
                                );


                            let checkbox_response =
                                ui.allocate_ui_with_layout(
                                    egui::vec2(
                                        checkbox_width,
                                        row_height,
                                    ),
                                    egui::Layout::left_to_right(
                                        egui::Align::Center
                                    ),
                                    |ui| {
                                        ui.checkbox(
                                            &mut bulk_checked,
                                            ""
                                        )
                                    },
                                )
                                .inner
                                .on_hover_text(
                                    "Include this policy in Bulk Edit mode"
                                );


                            if checkbox_response.changed() {
                                if bulk_checked {
                                    if !row_is_bulk_selected(
                                        bulk_selected_rows,
                                        &row_reference,
                                    ) {
                                        bulk_selected_rows.push(
                                            row_reference.clone()
                                        );
                                    }
                                } else {
                                    bulk_selected_rows.retain(
                                        |selected| {
                                            !(
                                                selected.policy_id == row_reference.policy_id
                                            )
                                        }
                                    );
                                }
                            }


                            let row_selected =
                                selected_row
                                    .as_ref()
                                    .is_some_and(
                                        |selected| {
                                            selected.policy_id == row_reference.policy_id
                                        }
                                    );

                            let filename_response =
                                left_aligned_cell(
                                    ui,
                                    filename_width,
                                    row_height,
                                    &row.policy_key,
                                    egui::Sense::click(),
                                    row_selected,
                                )
                                .on_hover_text(
                                    format!(
                                        "Policy Name: {}\nShader: {}\nPath: {}",
                                        row.policy_key,
                                        row.filename,
                                        row.full_path,
                                    )
                                );

                            if *restore_selected_policy_scroll
                                && row_selected
                            {
                                filename_response.scroll_to_me(
                                    Some(
                                        egui::Align::Center
                                    )
                                );

                                *restore_selected_policy_scroll =
                                    false;
                            }


                            if keyboard_selected_row
                                .as_ref()
                                .is_some_and(
                                    |selected| {
                                        selected.policy_id == row_reference.policy_id
                                    }
                                )
                            {
                                filename_response.scroll_to_me(
                                    Some(
                                        egui::Align::Center
                                    )
                                );
                            }

                            let status_response =
                                left_aligned_cell(
                                    ui,
                                    status_width,
                                    row_height,
                                    if !row.accessible {
                                        "❌"
                                    } else if row.unassigned {
                                        "X"
                                    } else {
                                        "✅"
                                    },
                                    egui::Sense::click(),
                                    row_selected,
                                )
                                .on_hover_text(
                                    if !row.accessible {
                                        format!(
                                            "Shader file cannot be accessed:\n{}",
                                            row.full_path,
                                        )
                                    } else if row.unassigned {
                                        "Unassigned policy — this shader cannot be rendered until its Policy Target is changed to Screensaver or Wallpaper."
                                            .to_string()
                                    } else {
                                        format!(
                                            "Shader is accessible:\n{}",
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
                                    if row.unassigned {
                                        "Unassigned"
                                    } else {
                                        policy_target_name(
                                            row.policy_target
                                        )
                                    },
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

                            if bulk_selected_rows.len() <= 1
                                && row_clicked
                            {
                                *selected_row =
                                    Some(
                                        row_reference.clone()
                                    );
                            }

                            if bulk_selected_rows.len() <= 1
                                && row_double_clicked
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
                            }

                            // Any cell may open the same row context menu in
                            // Single Edit mode.  Bulk Edit keeps only the
                            // selection checkboxes interactive.
                            let mut show_context_menu =
                                |response: &egui::Response,
                                 allow_clone: bool| {
                                    if bulk_selected_rows.len() > 1 {
                                        return;
                                    }

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

                                            if allow_clone {
                                                if ui.button(
                                                    "Clone Policy..."
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
                                                                PolicyRowCommand::ClonePolicy,
                                                            )
                                                        );

                                                    ui.close();
                                                }

                                                if ui.button(
                                                    "Rename Policy..."
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
                                                                PolicyRowCommand::RenamePolicy,
                                                            )
                                                        );

                                                    ui.close();
                                                }
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

                                            // Managed shaders now live in one canonical
                                            // /shaders directory. Policy target changes
                                            // are policy operations, not filesystem moves.
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
                                &filename_response,
                                true,
                            );
                            show_context_menu(
                                &status_response,
                                false,
                            );
                            show_context_menu(
                                &texture_response,
                                false,
                            );
                            show_context_menu(
                                &policy_response,
                                false,
                            );
                        }
                    },
                );
            },
        );
}


fn draw_bulk_create_confirmation_modal(
    context: &egui::Context,
    pending_candidates: &mut Option<Vec<BulkCreateCandidate>>,
    external_target: &mut Option<PolicyTarget>,
    rejected_count: usize,
    bulk_create_requested: &mut Option<BulkCreateRequest>,
) {
    let Some(candidates) =
        pending_candidates.as_ref()
    else {
        return;
    };


    let total_count =
        candidates.len();

    let screensaver_count =
        candidates
            .iter()
            .filter(
                |candidate| {
                    candidate.forced_target
                        == Some(
                            PolicyTarget::Screensaver
                        )
                }
            )
            .count();

    let wallpaper_count =
        candidates
            .iter()
            .filter(
                |candidate| {
                    candidate.forced_target
                        == Some(
                            PolicyTarget::Wallpaper
                        )
                }
            )
            .count();

    let external_count =
        candidates
            .iter()
            .filter(
                |candidate| {
                    candidate.forced_target
                        .is_none()
                }
            )
            .count();

    let texture_count =
        candidates
            .iter()
            .filter(
                |candidate| {
                    candidate.texture_required
                }
            )
            .count();


    let mut keep_open =
        true;

    let mut create_clicked =
        false;

    let mut cancel_clicked =
        false;


    egui::Window::new(
        "Create Multiple Policies"
    )
    .id(
        egui::Id::new(
            "editor_bulk_create_confirmation"
        )
    )
    .order(
        egui::Order::Foreground
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
            ui.label(
                format!(
                    "{} usable shader{} selected.",
                    total_count,
                    if total_count == 1 { "" } else { "s" },
                )
            );

            ui.label(
                format!(
                    "{} texture-enabled shader{} detected.",
                    texture_count,
                    if texture_count == 1 { "" } else { "s" },
                )
            );

            ui.add_space(
                6.0
            );

            ui.label(
                format!(
                    "Managed targets: {} Screensaver, {} Wallpaper.",
                    screensaver_count,
                    wallpaper_count,
                )
            );

            ui.label(
                format!(
                    "External shaders requiring a target: {}.",
                    external_count,
                )
            );


            if rejected_count > 0 {
                ui.label(
                    format!(
                        "{} selected shader{} could not be analyzed and will not be included.",
                        rejected_count,
                        if rejected_count == 1 { "" } else { "s" },
                    )
                );
            }


            if external_count > 0 {
                ui.add_space(
                    10.0
                );

                ui.label(
                    "Policy target for all external shaders:"
                );

                ui.horizontal(
                    |ui| {
                        ui.selectable_value(
                            external_target,
                            Some(
                                PolicyTarget::Screensaver
                            ),
                            "Screensaver",
                        );

                        ui.selectable_value(
                            external_target,
                            Some(
                                PolicyTarget::Wallpaper
                            ),
                            "Wallpaper",
                        );
                    },
                );
            }


            ui.add_space(
                12.0
            );


            let target_complete =
                external_count == 0
                    || external_target.is_some();


            ui.horizontal(
                |ui| {
                    if ui.add_enabled(
                        target_complete
                            && total_count > 0,
                        egui::Button::new(
                            "Create Policies"
                        ),
                    )
                    .clicked()
                    {
                        create_clicked =
                            true;
                    }


                    if ui.button(
                        "Cancel"
                    )
                    .clicked()
                    {
                        cancel_clicked =
                            true;
                    }
                },
            );
        },
    );


    if create_clicked {
        *bulk_create_requested =
            Some(
                BulkCreateRequest {
                    candidates:
                        candidates.clone(),

                    external_target:
                        *external_target,

                    rejected_count,
                }
            );

        *pending_candidates =
            None;

        *external_target =
            None;

        return;
    }


    if cancel_clicked
        || !keep_open
    {
        *pending_candidates =
            None;

        *external_target =
            None;
    }
}


fn draw_bulk_save_confirmation_modal(
    context: &egui::Context,
    pending_bulk_save_confirmation: &mut bool,
    selected_rows: &mut Vec<PolicyRowReference>,
    policy_rows: &[PolicyDisplayRow],
    bulk_save_requested: &mut bool,
) {
    if !*pending_bulk_save_confirmation {
        return;
    }


    let selected_count =
        selected_rows.len();


    let row_is_accessible =
        |selected: &PolicyRowReference| {
            policy_rows
                .iter()
                .find(
                    |row| {
                        row.policy_id == selected.policy_id
                    }
                )
                .map(
                    |row| {
                        row.accessible
                    }
                )
                .unwrap_or(false)
        };


    let eligible_count =
        selected_rows
            .iter()
            .filter(
                |selected| {
                    row_is_accessible(
                        selected
                    )
                }
            )
            .count();


    let excluded_rows =
        selected_rows
            .iter()
            .filter(
                |selected| {
                    !row_is_accessible(
                        selected
                    )
                }
            )
            .cloned()
            .collect::<Vec<_>>();


    let excluded_count =
        excluded_rows.len();


    let texture_enabled_count =
        selected_rows
            .iter()
            .filter(
                |selected| {
                    row_is_accessible(
                        selected
                    )
                }
            )
            .filter(
                |selected| {
                    policy_rows
                        .iter()
                        .find(
                            |row| {
                                row.policy_id == selected.policy_id
                            }
                        )
                        .map(
                            |row| {
                                row.texture
                            }
                        )
                        .unwrap_or(false)
                }
            )
            .count();


    let mut keep_open =
        true;


    egui::Window::new(
        "Confirm Bulk Policy Changes"
    )
    .id(
        egui::Id::new(
            "editor_bulk_save_confirmation"
        )
    )
    .order(
        egui::Order::Foreground
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
            if excluded_count == 0 {
                ui.label(
                    format!(
                        "Changes will be applied to {} policies. Click OK to continue or Cancel to abort.",
                        selected_count,
                    )
                );
            } else {
                ui.label(
                    format!(
                        "{} policies were selected. {} will be updated and {} will be skipped because the shader file is unavailable.",
                        selected_count,
                        eligible_count,
                        excluded_count,
                    )
                );


                ui.add_space(
                    6.0
                );


                ui.label(
                    if excluded_count == 1 {
                        "Excluded policy:"
                    } else {
                        "Excluded policies:"
                    }
                );


                for selected in
                    &excluded_rows
                {
                    ui.label(
                        format!(
                            "  {} ({})",
                            selected.filename,
                            policy_target_name(
                                selected.policy_target
                            ),
                        )
                    );
                }
            }


            ui.add_space(
                6.0
            );


            if eligible_count > 0 {
                ui.label(
                    format!(
                        "Texture and Palette settings will apply to {} texture-enabled shader{}.",
                        texture_enabled_count,
                        if texture_enabled_count == 1 {
                            ""
                        } else {
                            "s"
                        },
                    )
                );
            } else {
                ui.label(
                    "No selected policies are eligible for Bulk Edit because their shader files are unavailable."
                );
            }


            ui.add_space(
                12.0
            );


            ui.horizontal(
                |ui| {
                    if ui.add_enabled(
                        eligible_count > 0,
                        egui::Button::new(
                            "OK"
                        ),
                    )
                    .clicked()
                    {
                        selected_rows.retain(
                            |selected| {
                                row_is_accessible(
                                    selected
                                )
                            }
                        );

                        *bulk_save_requested =
                            true;

                        *pending_bulk_save_confirmation =
                            false;
                    }


                    if ui.button(
                        "Cancel"
                    )
                    .clicked()
                    {
                        *pending_bulk_save_confirmation =
                            false;
                    }
                },
            );
        },
    );


    if !keep_open {
        *pending_bulk_save_confirmation =
            false;
    }
}


fn draw_exit_confirmation_modal(
    context: &egui::Context,
    pending_exit_confirmation: &mut bool,
    policy_dirty: bool,
    control_configuration_dirty: bool,
    policy_target_selected: bool,
    bulk_edit_mode: bool,
    save_requested: &mut bool,
    bulk_save_requested: &mut bool,
    control_configuration_save_requested: &mut bool,
    exit_after_save_requested: &mut bool,
    exit_discard_requested: &mut bool,
) {
    if !*pending_exit_confirmation {
        return;
    }


    let mut keep_open =
        true;


    egui::Window::new(
        "Unsaved Changes"
    )
    .id(
        egui::Id::new(
            "editor_exit_confirmation"
        )
    )
    .order(
        egui::Order::Foreground
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
            ui.label(
                "The Screenshaver Control Center has unsaved changes."
            );

            ui.add_space(
                6.0
            );

            ui.label(
                "Would you like to save those changes before exiting?"
            );

            ui.add_space(
                12.0
            );


            let save_exit_enabled =
                bulk_edit_mode
                    || !policy_dirty
                    || policy_target_selected;


            ui.horizontal(
                |ui| {
                    let save_response =
                        ui.add_enabled(
                            save_exit_enabled,
                            egui::Button::new(
                                "Save and Exit"
                            ),
                        );


                    if save_response.clicked() {
                        if policy_dirty {
                            if bulk_edit_mode {
                                *bulk_save_requested =
                                    true;
                            } else {
                                *save_requested =
                                    true;
                            }
                        }

                        if control_configuration_dirty {
                            *control_configuration_save_requested =
                                true;
                        }

                        *exit_after_save_requested =
                            true;

                        *pending_exit_confirmation =
                            false;
                    }


                    if ui.button(
                        "Exit Without Saving"
                    )
                    .clicked()
                    {
                        *exit_discard_requested =
                            true;

                        *pending_exit_confirmation =
                            false;
                    }


                    if ui.button(
                        "Cancel"
                    )
                    .clicked()
                    {
                        *pending_exit_confirmation =
                            false;
                    }
                },
            );


            if !save_exit_enabled {
                ui.add_space(
                    6.0
                );

                ui.label(
                    egui::RichText::new(
                        "Select a policy target before saving the modified policy."
                    )
                    .weak(),
                );
            }
        },
    );


    if !keep_open {
        *pending_exit_confirmation =
            false;
    }
}


fn draw_policy_rename_modal(
    context: &egui::Context,
    pending_rename: &mut Option<PendingPolicyRename>,
    rename_requested:
        &mut Option<(PolicyRowReference, String)>,
) {
    let Some(rename) =
        pending_rename.as_mut()
    else {
        return;
    };

    let mut keep_open = true;
    let mut save_clicked = false;
    let mut cancel_clicked = false;

    egui::Window::new(
        "Rename Policy"
    )
    .id(
        egui::Id::new(
            "editor_rename_policy"
        )
    )
    .order(
        egui::Order::Foreground
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
            ui.label(
                "Change the user-facing Policy Name. The shader and policy settings are unchanged."
            );

            ui.add_space(8.0);

            ui.label(
                "Policy Name:"
            );

            let response =
                ui.add(
                    egui::TextEdit::singleline(
                        &mut rename.policy_name
                    )
                    .desired_width(
                        320.0
                    )
                );

            if response.changed() {
                rename.validation_message.clear();
            }

            if !rename.validation_message.is_empty() {
                ui.add_space(4.0);
                ui.label(
                    &rename.validation_message
                );
            }

            ui.add_space(12.0);

            ui.horizontal(
                |ui| {
                    if ui.button(
                        "Save"
                    )
                    .clicked()
                    {
                        save_clicked = true;
                    }

                    if ui.button(
                        "Cancel"
                    )
                    .clicked()
                    {
                        cancel_clicked = true;
                    }
                },
            );
        },
    );

    if save_clicked {
        let proposed =
            rename.policy_name.trim();

        let length =
            proposed.chars().count();

        if !(1..=128).contains(
            &length
        ) {
            rename.validation_message =
                format!(
                    "Policy Name must contain between 1 and 128 characters; found {}.",
                    length,
                );
        } else {
            *rename_requested =
                Some(
                    (
                        rename.row.clone(),
                        proposed.to_string(),
                    )
                );
        }
    } else if cancel_clicked
        || !keep_open
    {
        *pending_rename = None;
    }
}


fn draw_policy_clone_modal(
    context: &egui::Context,
    pending_clone: &mut Option<PendingPolicyClone>,
    clone_requested:
        &mut Option<(PolicyRowReference, String)>,
) {
    let Some(clone) =
        pending_clone.as_mut()
    else {
        return;
    };

    let mut keep_open =
        true;

    let mut save_clicked =
        false;

    let mut cancel_clicked =
        false;

    egui::Window::new(
        "Clone Policy"
    )
    .id(
        egui::Id::new(
            "editor_clone_policy"
        )
    )
    .order(
        egui::Order::Foreground
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
            ui.label(
                "Create a new policy with the same shader, target, and settings."
            );

            ui.add_space(8.0);

            ui.label(
                "Policy Name:"
            );

            let response =
                ui.add(
                    egui::TextEdit::singleline(
                        &mut clone.policy_name
                    )
                    .desired_width(
                        320.0
                    )
                );

            if response.changed() {
                clone.validation_message.clear();
            }

            if !clone.validation_message.is_empty() {
                ui.add_space(4.0);

                ui.label(
                    &clone.validation_message
                );
            }

            ui.add_space(12.0);

            ui.horizontal(
                |ui| {
                    if ui.button(
                        "Save"
                    )
                    .clicked()
                    {
                        save_clicked = true;
                    }

                    if ui.button(
                        "Cancel"
                    )
                    .clicked()
                    {
                        cancel_clicked = true;
                    }
                },
            );
        },
    );

    if save_clicked {
        let proposed =
            clone.policy_name.trim();

        let length =
            proposed.chars().count();

        if !(1..=128).contains(
            &length
        ) {
            clone.validation_message =
                format!(
                    "Policy Name must contain between 1 and 128 characters; found {}.",
                    length,
                );
        } else {
            *clone_requested =
                Some(
                    (
                        clone.row.clone(),
                        proposed.to_string(),
                    )
                );
        }
    } else if cancel_clicked
        || !keep_open
    {
        *pending_clone = None;
    }
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
            PolicyRowCommand::MoveToScreensavers =>
                "Move Shader?",

            PolicyRowCommand::MoveToWallpapers =>
                "Move Shader?",

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
    .order(
        egui::Order::Foreground
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
                PolicyRowCommand::MoveToScreensavers
                | PolicyRowCommand::MoveToWallpapers => {
                    let destination =
                        match confirmation.command {
                            PolicyRowCommand::MoveToScreensavers =>
                                "/screensavers",

                            PolicyRowCommand::MoveToWallpapers =>
                                "/wallpapers",

                            _ =>
                                unreachable!(),
                        };

                    ui.label(
                        format!(
                            "Move this shader to {}:",
                            destination,
                        )
                    );

                    ui.add_space(6.0);

                    ui.strong(
                        &confirmation.row.filename
                    );

                    ui.add_space(8.0);

                    let destination_target =
                        match confirmation.command {
                            PolicyRowCommand::MoveToScreensavers =>
                                PolicyTarget::Screensaver,

                            PolicyRowCommand::MoveToWallpapers =>
                                PolicyTarget::Wallpaper,

                            _ =>
                                unreachable!(),
                        };


                    if confirmation.row.policy_target
                        != destination_target
                    {
                        ui.label(
                            format!(
                                "The existing {} policy will be changed to a {} policy. All policy settings will be preserved.",
                                policy_target_name(
                                    confirmation.row.policy_target
                                ),
                                policy_target_name(
                                    destination_target
                                ),
                            )
                        );
                    } else {
                        ui.label(
                            format!(
                                "The existing {} policy will be retained and its path will be updated automatically.",
                                policy_target_name(
                                    confirmation.row.policy_target
                                ),
                            )
                        );
                    }
                }

                PolicyRowCommand::DeletePolicy => {
                    ui.label(
                        format!(
                            "Delete this {} policy:",
                            target_name,
                        )
                    );

                    ui.add_space(6.0);

                    ui.strong(
                        &confirmation.row.policy_key
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

                            PolicyTarget::Unassigned =>
                                "The shader file will not be changed.",
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

        PolicyTarget::Unassigned =>
            "Unassigned",
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
    bulk_edit_baseline: Option<EditorConfiguration>,
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

            if bulk_edit_baseline.is_some_and(|baseline| *displayed_fps != baseline.fps) {
                editor_theme::paint_bulk_edit_border(ui, fps_response.rect, metrics.scale);
            }

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

            if bulk_edit_baseline.is_some_and(
                |baseline| (*displayed_animation_speed - baseline.animation_speed).abs() > 0.0001
            ) {
                editor_theme::paint_bulk_edit_border(ui, speed_response.rect, metrics.scale);
            }

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

            if bulk_edit_baseline.is_some_and(
                |baseline| (*displayed_render_scale - baseline.render_scale).abs() > 0.0001
            ) {
                editor_theme::paint_bulk_edit_border(ui, scale_response.rect, metrics.scale);
            }
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
    _palette: &mut PaletteSelection,
    primitive_count: &mut u32,
    bulk_edit_baseline: Option<EditorConfiguration>,
    hover_help_message: &mut Option<&'static str>,
) {
    ui.add_enabled_ui(
        texture_required,
        |ui| {
            const PRIMITIVE_VALUES: [u32; 10] = [
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


            let label_width =
                120.0 * metrics.scale;

            let dropdown_width =
                130.0 * metrics.scale;

            let primitives_label_width =
                58.0 * metrics.scale;

            let primitive_value_width =
                42.0 * metrics.scale;

            let inter_control_gap =
                12.0 * metrics.scale;


            ui.set_width(
                ui.available_width()
            );


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
                                "Texture:"
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
                            dropdown_width
                        )
                        .show_ui(
                            ui,
                            |ui| {
                                for selection in [
                                    TextureSelection::Bricks,
                                    TextureSelection::Cellular,
                                    TextureSelection::Clouds,
                                    TextureSelection::Eyes,
                                    TextureSelection::Facets,
                                    TextureSelection::Hexagons,
                                    TextureSelection::Marble,
                                    TextureSelection::Mesh,
                                    TextureSelection::Noise,
                                    TextureSelection::Radial,
                                    TextureSelection::Scales,
                                    TextureSelection::Skulls,
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

                    if bulk_edit_baseline.is_some_and(|baseline| *texture != baseline.texture) {
                        editor_theme::paint_bulk_edit_border(ui, texture_response.rect, metrics.scale);
                    }


                    ui.add_space(
                        inter_control_gap
                    );


                    ui.allocate_ui_with_layout(
                        egui::vec2(
                            primitives_label_width,
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


                    let primitive_slider_width =
                        (
                            ui.available_width()
                                - primitive_value_width
                                - ui.spacing()
                                    .item_spacing
                                    .x
                        )
                        .max(
                            90.0 * metrics.scale
                        );


                    let primitive_response =
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
                            },
                        )
                        .inner;


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
                            primitive_value_width,
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


                    update_hover_help(
                        &primitive_response,
                        hover_help_message,
                        "Set the number of graphical elements used to generate the procedural texture.",
                    );

                    if bulk_edit_baseline.is_some_and(
                        |baseline| *primitive_count != baseline.primitive_count
                    ) {
                        editor_theme::paint_bulk_edit_border(ui, primitive_response.rect, metrics.scale);
                    }
                },
            );
        },
    );
}

fn ensure_texture_thumbnail(
    context: &egui::Context,
    texture: TextureSelection,
    palette: PaletteSelection,
    primitive_count: u32,
    thumbnail: &mut Option<egui::TextureHandle>,
    thumbnail_key: &mut Option<(TextureSelection, PaletteSelection, u32)>,
    status_message: &mut String,
) {

    let key =
        (
            texture,
            palette,
            primitive_count,
        );


    if *thumbnail_key
        == Some(key)
        && thumbnail.is_some()
    {
        return;
    }


    let specification =
        crate::parse_texture_specification::TextureSpecification {
            family:
                texture.family(),

            requested_primitive_count:
                primitive_count as usize,

            count_was_explicit:
                true,
        };


    match crate::preview_texture_thumbnail::generate(
        &specification,
        palette.palette(),
        TEXTURE_THUMBNAIL_SEED,
        TEXTURE_THUMBNAIL_MAX_SIZE as u32,
    ) {
        Ok(generated) => {
            *thumbnail =
                Some(
                    context.load_texture(
                        "screenshaver_texture_thumbnail",
                        generated.color_image,
                        egui::TextureOptions::LINEAR,
                    )
                );

            *thumbnail_key =
                Some(
                    key
                );
        }

        Err(error) => {
            *thumbnail =
                None;

            *thumbnail_key =
                None;

            *status_message =
                format!(
                    "Unable to generate texture thumbnail: {}",
                    error,
                );
        }
    }
}


fn draw_texture_thumbnail(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    texture_thumbnail: Option<&egui::TextureHandle>,
) {

    let available_width =
        ui.available_width();


    // Do not use ui.available_height() here.  This thumbnail lives inside a
    // nested horizontal/vertical layout whose child height is content-driven.
    // At this point egui can legitimately report essentially zero remaining
    // height even though the parent Textures tab still has visible room.  The
    // previous experiment therefore collapsed the requested thumbnail to a
    // nearly invisible 1x1 image.
    //
    // The Control Center and tab dimensions are already fixed.  For this
    // sizing experiment, constrain only by the width of this column and by the
    // explicit thumbnail maximum.  The surrounding fixed tab remains the hard
    // vertical boundary.
    let preview_size =
        (
            TEXTURE_THUMBNAIL_MAX_SIZE
                * metrics.scale
        )
        .min(
            available_width
        )
        .max(
            1.0
        );


    let desired_size =
        egui::vec2(
            preview_size,
            preview_size,
        );


    if let Some(texture_thumbnail) =
        texture_thumbnail
    {
        ui.add(
            egui::Image::new(
                texture_thumbnail
            )
            .fit_to_exact_size(
                desired_size
            ),
        );

    } else {
        ui.allocate_ui_with_layout(
            desired_size,
            egui::Layout::centered_and_justified(
                egui::Direction::TopDown
            ),
            |ui| {
                ui.label(
                    "Texture preview unavailable"
                );
            },
        );
    }
}


fn curated_palette_color_button(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    entry: &crate::palettes::CuratedPaletteColor,
    selected: bool,
) -> egui::Response {

    let row_height =
        ui.spacing()
            .interact_size
            .y;


    let desired_size =
        egui::vec2(
            128.0 * metrics.scale,
            row_height,
        );


    let (
        rect,
        response,
    ) =
        ui.allocate_exact_size(
            desired_size,
            egui::Sense::click(),
        );


    if ui.is_rect_visible(
        rect
    ) {
        let visuals =
            ui.style()
                .interact_selectable(
                    &response,
                    selected,
                );


        ui.painter()
            .rect_filled(
                rect,
                visuals.rounding(),
                visuals.bg_fill,
            );


        ui.painter()
            .rect_stroke(
                rect,
                visuals.rounding(),
                visuals.bg_stroke,
                egui::StrokeKind::Inside,
            );


        let swatch_size =
            14.0 * metrics.scale;


        let swatch_rect =
            egui::Rect::from_min_size(
                egui::pos2(
                    rect.left()
                        + 6.0 * metrics.scale,
                    rect.center().y
                        - swatch_size * 0.5,
                ),
                egui::vec2(
                    swatch_size,
                    swatch_size,
                ),
            );


        let color =
            entry.color;


        ui.painter()
            .rect_filled(
                swatch_rect,
                2.0 * metrics.scale,
                egui::Color32::from_rgb(
                    color.red(),
                    color.green(),
                    color.blue(),
                ),
            );


        ui.painter()
            .rect_stroke(
                swatch_rect,
                2.0 * metrics.scale,
                egui::Stroke::new(
                    1.0,
                    ui.visuals()
                        .widgets
                        .noninteractive
                        .fg_stroke
                        .color,
                ),
                egui::StrokeKind::Inside,
            );


        let mut entry_font =
            egui::TextStyle::Button.resolve(
                ui.style()
            );


        entry_font.size =
            (
                entry_font.size - 1.0
            )
            .max(
                1.0
            );


        ui.painter()
            .text(
                egui::pos2(
                    swatch_rect.right()
                        + 7.0 * metrics.scale,
                    rect.center().y,
                ),
                egui::Align2::LEFT_CENTER,
                entry.name,
                entry_font,
                visuals.text_color(),
            );
    }


    response
}


fn draw_standalone_visual_color_picker(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    color: &mut egui::Color32,
    palette: &mut PaletteSelection,
    palette_hex_input: &mut String,
) {
    let active_palette_color =
        palette.palette();


    let active_picker_color =
        egui::Color32::from_rgb(
            active_palette_color.red(),
            active_palette_color.green(),
            active_palette_color.blue(),
        );


    if active_picker_color
        != *color
    {
        *color =
            active_picker_color;
    }


    let mut hsva =
        egui::ecolor::Hsva::from(
            *color
        );


    let hue_width =
        18.0 * metrics.scale;

    let gap =
        8.0 * metrics.scale;


    // Keep the picker inside a predictable horizontal footprint so the
    // Textures tab cannot widen the Control Center window.
    let picker_total_width =
        220.0 * metrics.scale;


    let field_width =
        (
            picker_total_width
                - hue_width
                - gap
        )
        .max(
            140.0 * metrics.scale
        );

    let field_height =
        field_width;


    ui.horizontal(
        |ui| {
            draw_saturation_value_field(
                ui,
                egui::vec2(
                    field_width,
                    field_height,
                ),
                &mut hsva,
            );


            ui.add_space(
                gap
            );


            draw_hue_strip(
                ui,
                egui::vec2(
                    hue_width,
                    field_height,
                ),
                &mut hsva,
            );
        },
    );


    let selected_color =
        egui::Color32::from(
            hsva
        );


    if selected_color
        != *color
    {
        *color =
            selected_color;


        let palette_color =
            crate::palettes::PaletteColor::new(
                selected_color.r(),
                selected_color.g(),
                selected_color.b(),
            );


        *palette =
            PaletteSelection::from_palette(
                palette_color
            );


        *palette_hex_input =
            palette_color.to_hex();
    }


}


fn draw_palette_color_swatch(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    color: egui::Color32,
) {
    let swatch_size =
        egui::vec2(
            72.0 * metrics.scale,
            24.0 * metrics.scale,
        );


    let (
        swatch_rect,
        _swatch_response,
    ) =
        ui.allocate_exact_size(
            swatch_size,
            egui::Sense::hover(),
        );


    ui.painter()
        .rect_filled(
            swatch_rect,
            3.0 * metrics.scale,
            color,
        );


    ui.painter()
        .rect_stroke(
            swatch_rect,
            3.0 * metrics.scale,
            egui::Stroke::new(
                1.0,
                ui.visuals()
                    .widgets
                    .noninteractive
                    .fg_stroke
                    .color,
            ),
            egui::StrokeKind::Inside,
        );
}


fn draw_saturation_value_field(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    hsva: &mut egui::ecolor::Hsva,
) {
    let (
        rect,
        response,
    ) =
        ui.allocate_exact_size(
            size,
            egui::Sense::click_and_drag(),
        );


    let mesh_resolution =
        24_usize;


    if ui.is_rect_visible(
        rect
    ) {
        let mut mesh =
            egui::Mesh::default();


        for y_index in
            0..=mesh_resolution
        {
            let y_fraction =
                y_index as f32
                    / mesh_resolution as f32;

            let value =
                1.0 - y_fraction;


            for x_index in
                0..=mesh_resolution
            {
                let x_fraction =
                    x_index as f32
                        / mesh_resolution as f32;

                let saturation =
                    x_fraction;


                let position =
                    egui::pos2(
                        egui::lerp(
                            rect.x_range(),
                            x_fraction,
                        ),
                        egui::lerp(
                            rect.y_range(),
                            y_fraction,
                        ),
                    );


                let color =
                    egui::Color32::from(
                        egui::ecolor::Hsva::new(
                            hsva.h,
                            saturation,
                            value,
                            1.0,
                        )
                    );


                mesh.vertices.push(
                    egui::epaint::Vertex {
                        pos:
                            position,
                        uv:
                            egui::Pos2::ZERO,
                        color,
                    }
                );
            }
        }


        for y_index in
            0..mesh_resolution
        {
            for x_index in
                0..mesh_resolution
            {
                let row_width =
                    mesh_resolution + 1;

                let top_left =
                    (
                        y_index * row_width
                            + x_index
                    ) as u32;

                let top_right =
                    top_left + 1;

                let bottom_left =
                    top_left
                        + row_width as u32;

                let bottom_right =
                    bottom_left + 1;


                mesh.add_triangle(
                    top_left,
                    top_right,
                    bottom_right,
                );

                mesh.add_triangle(
                    top_left,
                    bottom_right,
                    bottom_left,
                );
            }
        }


        ui.painter()
            .add(
                egui::Shape::mesh(
                    mesh
                )
            );


        ui.painter()
            .rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(
                    1.0,
                    ui.visuals()
                        .widgets
                        .noninteractive
                        .bg_stroke
                        .color,
                ),
                egui::StrokeKind::Inside,
            );


        let marker_position =
            egui::pos2(
                egui::lerp(
                    rect.x_range(),
                    hsva.s,
                ),
                egui::lerp(
                    rect.y_range(),
                    1.0 - hsva.v,
                ),
            );


        ui.painter()
            .circle_stroke(
                marker_position,
                5.0,
                egui::Stroke::new(
                    2.0,
                    if hsva.v > 0.55 {
                        egui::Color32::BLACK
                    } else {
                        egui::Color32::WHITE
                    },
                ),
            );
    }


    if response.dragged()
        || response.clicked()
    {
        if let Some(pointer) =
            response.interact_pointer_pos()
        {
            hsva.s =
                (
                    (
                        pointer.x
                            - rect.left()
                    )
                        / rect.width()
                )
                .clamp(
                    0.0,
                    1.0,
                );

            hsva.v =
                (
                    1.0
                        - (
                            (
                                pointer.y
                                    - rect.top()
                            )
                                / rect.height()
                        )
                )
                .clamp(
                    0.0,
                    1.0,
                );
        }
    }
}


fn draw_hue_strip(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    hsva: &mut egui::ecolor::Hsva,
) {
    let (
        rect,
        response,
    ) =
        ui.allocate_exact_size(
            size,
            egui::Sense::click_and_drag(),
        );


    if ui.is_rect_visible(
        rect
    ) {
        let segments =
            36_usize;


        for index in
            0..segments
        {
            let top_fraction =
                index as f32
                    / segments as f32;

            let bottom_fraction =
                (
                    index + 1
                ) as f32
                    / segments as f32;


            let segment_rect =
                egui::Rect::from_min_max(
                    egui::pos2(
                        rect.left(),
                        egui::lerp(
                            rect.y_range(),
                            top_fraction,
                        ),
                    ),
                    egui::pos2(
                        rect.right(),
                        egui::lerp(
                            rect.y_range(),
                            bottom_fraction,
                        ),
                    ),
                );


            let color =
                egui::Color32::from(
                    egui::ecolor::Hsva::new(
                        top_fraction,
                        1.0,
                        1.0,
                        1.0,
                    )
                );


            ui.painter()
                .rect_filled(
                    segment_rect,
                    0.0,
                    color,
                );
        }


        ui.painter()
            .rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(
                    1.0,
                    ui.visuals()
                        .widgets
                        .noninteractive
                        .bg_stroke
                        .color,
                ),
                egui::StrokeKind::Inside,
            );


        let marker_y =
            egui::lerp(
                rect.y_range(),
                hsva.h,
            );


        ui.painter()
            .line_segment(
                [
                    egui::pos2(
                        rect.left() - 3.0,
                        marker_y,
                    ),
                    egui::pos2(
                        rect.right() + 3.0,
                        marker_y,
                    ),
                ],
                egui::Stroke::new(
                    2.0,
                    ui.visuals()
                        .widgets
                        .active
                        .fg_stroke
                        .color,
                ),
            );
    }


    if response.dragged()
        || response.clicked()
    {
        if let Some(pointer) =
            response.interact_pointer_pos()
        {
            hsva.h =
                (
                    (
                        pointer.y
                            - rect.top()
                    )
                        / rect.height()
                )
                .clamp(
                    0.0,
                    1.0,
                );
        }
    }
}


// Texture-tab curated palette selector and authoritative hexadecimal field.

fn draw_color_picker_placeholder(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    palette: &mut PaletteSelection,
    palette_hex_input: &mut String,
    color_picker_preview: &mut egui::Color32,
    texture_thumbnail: Option<&egui::TextureHandle>,
    hover_help_message: &mut Option<&'static str>,
) {
    let label_width =
        120.0 * metrics.scale;

    let dropdown_width =
        130.0 * metrics.scale;


    ui.set_width(
        ui.available_width()
    );


    ui.horizontal(
        |ui| {
            ui.vertical(
                |ui| {
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
                                        "Palette Color:"
                                    );
                                },
                            );


            let selected_curated_color =
                crate::palettes::curated_color_for_palette(
                    palette.palette()
                );


            let curated_selected_text =
                selected_curated_color
                    .map(
                        |entry| {
                            entry.name
                        }
                    )
                    .unwrap_or(
                        "Custom Color"
                    );


            let curated_button_response =
                ui.add_sized(
                    [
                        dropdown_width,
                        ui.spacing()
                            .interact_size
                            .y,
                    ],
                    egui::Button::new(
                        curated_selected_text
                    ),
                );


            let popup_id =
                egui::Popup::default_response_id(
                    &curated_button_response
                );


            let _ =
                egui::Popup::menu(
                    &curated_button_response
                )
                .id(
                    popup_id
                )
                .align(
                    egui::RectAlign::BOTTOM_START
                )
                .align_alternatives(
                    &[]
                )
                .width(
                    dropdown_width
                )
                .show(
                    |ui| {
                        ui.spacing_mut()
                            .item_spacing
                            .y =
                            0.0;


                        egui::ScrollArea::vertical()
                            .max_height(
                                235.0 * metrics.scale
                            )
                            .show(
                                ui,
                                |ui| {
                                    for family in [
                                        crate::palettes::CuratedColorFamily::Red,
                                        crate::palettes::CuratedColorFamily::Orange,
                                        crate::palettes::CuratedColorFamily::Yellow,
                                        crate::palettes::CuratedColorFamily::Green,
                                        crate::palettes::CuratedColorFamily::Blue,
                                        crate::palettes::CuratedColorFamily::Indigo,
                                        crate::palettes::CuratedColorFamily::Violet,
                                        crate::palettes::CuratedColorFamily::Grayscale,
                                    ] {
                                        for entry in
                                            crate::palettes::curated_colors_for_family(
                                                family
                                            )
                                        {
                                            let selected =
                                                entry.color
                                                    == palette.palette();


                                            let response =
                                                curated_palette_color_button(
                                                    ui,
                                                    metrics,
                                                    entry,
                                                    selected,
                                                );


                                            if response.clicked() {
                                                *palette =
                                                    PaletteSelection::from_palette(
                                                        entry.color
                                                    );

                                                *palette_hex_input =
                                                    entry.color.to_hex();

                                                egui::Popup::close_id(
                                                    ui.ctx(),
                                                    popup_id,
                                                );
                                            }
                                        }
                                    }
                                },
                            );
                    },
                );


            let curated_response =
                curated_button_response;


            update_hover_help(
                &curated_response,
                hover_help_message,
                "Choose a curated palette color. The selected color is written to the hexadecimal field.",
            );
                        },
                    );


                    ui.add_space(
                        8.0 * metrics.scale
                    );


                    draw_texture_thumbnail(
                        ui,
                        metrics,
                        texture_thumbnail,
                    );
                },
            );

            ui.vertical(
                |ui| {
                    let hex_response =
                        ui.add(
                            egui::TextEdit::singleline(
                                palette_hex_input
                            )
                            .desired_width(
                                74.0 * metrics.scale
                            )
                            .hint_text(
                                "#rrggbb"
                            ),
                        );

                    update_hover_help(
                        &hex_response,
                        hover_help_message,
                        "Enter a palette color using six-digit hexadecimal notation (#rrggbb).",
                    );

                    if hex_response.changed() {
                        // Do not reject intermediate text while the user is typing.
                        // As soon as the field contains a valid #rrggbb value, make
                        // that color authoritative and allow the live shader preview
                        // to regenerate its texture immediately.
                        if let Ok(color) =
                            crate::palettes::PaletteColor::parse_hex(
                                palette_hex_input
                            )
                        {
                            *palette =
                                PaletteSelection::from_palette(
                                    color
                                );
                        }
                    }

                    if hex_response.lost_focus() {
                        match crate::palettes::PaletteColor::parse_hex(
                            palette_hex_input
                        ) {
                            Ok(color) => {
                                *palette =
                                    PaletteSelection::from_palette(
                                        color
                                    );

                                *palette_hex_input =
                                    color.to_hex();
                            }

                            Err(_) => {
                                // Never leave an invalid string displayed as though it
                                // were the active palette.  Revert to the last valid
                                // palette value when editing finishes.
                                *palette_hex_input =
                                    palette.name();
                            }
                        }
                    }


                    ui.add_space(
                        6.0 * metrics.scale
                    );


                    draw_palette_color_swatch(
                        ui,
                        metrics,
                        *color_picker_preview,
                    );
                },
            );


            ui.allocate_ui_with_layout(
                egui::vec2(
                    220.0 * metrics.scale,
                    ui.available_height(),
                ),
                egui::Layout::top_down(
                    egui::Align::LEFT
                ),
                |ui| {
                    draw_standalone_visual_color_picker(
                        ui,
                        metrics,
                        color_picker_preview,
                        palette,
                        palette_hex_input,
                    );
                },
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
    shift_held: bool,
    anti_aliasing: &mut AntiAliasingSelection,
    dithering: &mut DitheringSelection,
    color_precision: &mut ColorPrecisionSelection,
    bloom: &mut BloomSelection,
    bloom_intensity: &mut f32,
    bloom_intensity_drag_state: &mut Option<SliderDragState>,
    bloom_threshold: &mut f32,
    bloom_threshold_drag_state: &mut Option<SliderDragState>,
    invert_colors: &mut bool,
    flip_horizontal: &mut bool,
    flip_vertical: &mut bool,
    hue_rotation: &mut f32,
    hue_rotation_drag_state: &mut Option<SliderDragState>,
    bulk_edit_baseline: Option<EditorConfiguration>,
    hover_help_message: &mut Option<&'static str>,
) {
    draw_post_processing_panel(
        ui,
        metrics,
        anti_aliasing,
        dithering,
        color_precision,
        bulk_edit_baseline,
        hover_help_message,
    );

    ui.add_space(
        8.0 * metrics.scale
    );

    egui::Grid::new(
        "editor_bloom_grid"
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
            ui.label("Bloom");

            let selected_text =
                match *bloom {
                    BloomSelection::Off => "Off",
                    BloomSelection::Highlight => "Highlight",
                    BloomSelection::Audio => "Audio",
                };

            let response =
                egui::ComboBox::from_id_source(
                    "editor_bloom"
                )
                .selected_text(selected_text)
                .width(metrics.dropdown_width)
                .show_ui(
                    ui,
                    |ui| {
                        ui.selectable_value(
                            bloom,
                            BloomSelection::Off,
                            "Off",
                        );

                        ui.selectable_value(
                            bloom,
                            BloomSelection::Highlight,
                            "Highlight",
                        );

                        ui.selectable_value(
                            bloom,
                            BloomSelection::Audio,
                            "Audio",
                        );
                    },
                )
                .response;

            update_hover_help(
                &response,
                hover_help_message,
                "Select the bloom processing mode. Highlight bloom affects bright image regions; Audio bloom targets bass, midrange, and high-frequency color bands.",
            ); if bulk_edit_baseline.is_some_and(|baseline| *bloom != baseline.bloom) {
                editor_theme::paint_bulk_edit_border(ui, response.rect, metrics.scale);
            }
            ui.end_row();

            ui.label("Bloom Intensity");

            let intensity_response =
                ui.add_enabled_ui(
                    *bloom != BloomSelection::Off,
                    |ui| {
                        ui.horizontal(
                            |ui| {
                                ui.set_width(
                                    metrics.dropdown_width
                                );

                                let slider_width =
                                    (metrics.dropdown_width
                                        - 52.0 * metrics.scale)
                                        .max(
                                            80.0 * metrics.scale
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
                                                bloom_intensity,
                                                crate::render_bloom::BLOOM_INTENSITY_MIN,
                                                crate::render_bloom::BLOOM_INTENSITY_MAX,
                                                shift_held,
                                                metrics.scale,
                                                bloom_intensity_drag_state,
                                            )
                                        },
                                    )
                                    .inner;

                                ui.label(
                                    format!(
                                        "{:.2}",
                                        *bloom_intensity,
                                    )
                                );

                                response
                            },
                        )
                        .inner
                    },
                )
                .inner;

            update_hover_help(
                &intensity_response,
                hover_help_message,
                "Set the strength of the bloom effect. Hold Shift while dragging for fine adjustment. Lower values are suitable for subtle wallpaper bloom; higher values produce a stronger effect.",
            ); if bulk_edit_baseline.is_some_and(
                |baseline| (*bloom_intensity - baseline.bloom_intensity).abs() > 0.0001
            ) {
                editor_theme::paint_bulk_edit_border(ui, intensity_response.rect, metrics.scale);
            }
            ui.end_row();

            ui.label("Bloom Threshold");

            let threshold_response =
                ui.add_enabled_ui(
                    *bloom != BloomSelection::Off,
                    |ui| {
                        ui.horizontal(
                            |ui| {
                                ui.set_width(
                                    metrics.dropdown_width
                                );

                                let slider_width =
                                    (metrics.dropdown_width
                                        - 52.0 * metrics.scale)
                                        .max(
                                            80.0 * metrics.scale
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
                                                bloom_threshold,
                                                crate::render_bloom::BLOOM_THRESHOLD_MIN,
                                                crate::render_bloom::BLOOM_THRESHOLD_MAX,
                                                shift_held,
                                                metrics.scale,
                                                bloom_threshold_drag_state,
                                            )
                                        },
                                    )
                                    .inner;

                                ui.label(
                                    format!(
                                        "{:.2}",
                                        *bloom_threshold,
                                    )
                                );

                                response
                            },
                        )
                        .inner
                    },
                )
                .inner;

            update_hover_help(
                &threshold_response,
                hover_help_message,
                "Set the minimum luminance that contributes to Highlight Bloom. Lower values include more of the image; higher values restrict bloom to brighter regions. Hold Shift while dragging for fine adjustment.",
            ); if bulk_edit_baseline.is_some_and(
                |baseline| (*bloom_threshold - baseline.bloom_threshold).abs() > 0.0001
            ) {
                editor_theme::paint_bulk_edit_border(ui, threshold_response.rect, metrics.scale);
            }
            ui.end_row();
            ui.label("Invert Colors");
            let invert_response = ui.checkbox(invert_colors, "Enabled");
            update_hover_help(
                &invert_response,
                hover_help_message,
                "Invert the rendered shader colors before Bloom and the remaining post-processing stages.",
            ); if bulk_edit_baseline.is_some_and(|baseline| *invert_colors != baseline.invert_colors) {
                editor_theme::paint_bulk_edit_border(ui, invert_response.rect, metrics.scale);
            }
            ui.end_row();

            ui.label("Flip Horizontal");
            let flip_horizontal_response = ui.checkbox(flip_horizontal, "Enabled");
            update_hover_help(
                &flip_horizontal_response,
                hover_help_message,
                "Mirror the rendered shader from left to right without changing its aspect ratio.",
            ); if bulk_edit_baseline.is_some_and(|baseline| *flip_horizontal != baseline.flip_horizontal) {
                editor_theme::paint_bulk_edit_border(ui, flip_horizontal_response.rect, metrics.scale);
            }
            ui.end_row();

            ui.label("Flip Vertical");
            let flip_vertical_response = ui.checkbox(flip_vertical, "Enabled");
            update_hover_help(
                &flip_vertical_response,
                hover_help_message,
                "Mirror the rendered shader from top to bottom without changing its aspect ratio.",
            ); if bulk_edit_baseline.is_some_and(|baseline| *flip_vertical != baseline.flip_vertical) {
                editor_theme::paint_bulk_edit_border(ui, flip_vertical_response.rect, metrics.scale);
            }
            ui.end_row();

            ui.label("Hue Rotation");

            let hue_response =
                ui.horizontal(
                    |ui| {
                        ui.set_width(metrics.dropdown_width);

                        let slider_width =
                            (metrics.dropdown_width - 58.0 * metrics.scale)
                                .max(80.0 * metrics.scale);

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
                                    ui.set_width(slider_width);

                                    draw_fine_slider(
                                        ui,
                                        hue_rotation,
                                        crate::postprocess_shader::HUE_ROTATION_MIN,
                                        crate::postprocess_shader::HUE_ROTATION_MAX,
                                        shift_held,
                                        metrics.scale,
                                        hue_rotation_drag_state,
                                    )
                                },
                            )
                            .inner;

                        ui.label(
                            format!(
                                "{:.1}°",
                                *hue_rotation,
                            )
                        );

                        response
                    },
                )
                .inner;

            update_hover_help(
                &hue_response,
                hover_help_message,
                "Rotate shader hue from -180° through +180°. Hold Shift while dragging for 10x finer adjustment.",
            ); if bulk_edit_baseline.is_some_and(
                |baseline| (*hue_rotation - baseline.hue_rotation).abs() > 0.0001
            ) {
                editor_theme::paint_bulk_edit_border(ui, hue_response.rect, metrics.scale);
            }
            ui.end_row();

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
    bulk_edit_baseline: Option<EditorConfiguration>,
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
                    ); if bulk_edit_baseline.is_some_and(|baseline| *anti_aliasing != baseline.anti_aliasing) {
                        editor_theme::paint_bulk_edit_border(ui, response.rect, metrics.scale);
                    }
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
                    ); if bulk_edit_baseline.is_some_and(|baseline| *dithering != baseline.dithering) {
                        editor_theme::paint_bulk_edit_border(ui, response.rect, metrics.scale);
                    }
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
                    ); if bulk_edit_baseline.is_some_and(|baseline| *color_precision != baseline.color_precision) {
                        editor_theme::paint_bulk_edit_border(ui, response.rect, metrics.scale);
                    }
                    ui.end_row();
                },
            );
        },
    );
}




// ============================================================
// CONFIGURATION TAB
// ============================================================
// The live Configuration UI is implemented in nested_tabs.rs and invoked
// directly from the EditorTab::Config branch above.

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


