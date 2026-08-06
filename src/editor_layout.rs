//! Graphical layout and input handling for the Shader Policy Editor.
//!
//! This module owns the egui window, controls, layout, styling, and SDL-to-egui
//! input translation. Rendering and shader-session behavior remain in
//! `edit_shader`.

use std::sync::Arc;
use std::time::Instant;

use crate::editor_theme::{self, EditorMetrics};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;

// Editor-window geometry is expressed as a fraction of the active display.
// Initial and maximum dimensions are bounded against the active display.
// The 0.68 by 0.68 maximum occupies 46.24% of the usable display area.
const EDIT_WINDOW_INITIAL_WIDTH_FRACTION: f32 =
    0.62;

const EDIT_WINDOW_INITIAL_HEIGHT_FRACTION: f32 =
    0.62;

const EDIT_WINDOW_MAXIMUM_WIDTH_FRACTION: f32 =
    0.68;

const EDIT_WINDOW_MAXIMUM_HEIGHT_FRACTION: f32 =
    0.68;

const EDIT_WINDOW_REFERENCE_HEIGHT: f32 =
    1080.0;

const EDIT_WINDOW_SCALE_MIN: f32 =
    0.80;

const EDIT_WINDOW_SCALE_MAX: f32 =
    1.80;


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
pub enum TextureSelection {
    Current,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteSelection {
    Current,
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


#[derive(Clone, Copy, Debug)]
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
    pub save_requested: bool,
    pub cancel_requested: bool,
    pub delete_requested: bool,
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

    shift_held:
        bool,

    fps_drag_state:
        Option<SliderDragState>,

    animation_speed_drag_state:
        Option<SliderDragState>,

    render_scale_drag_state:
        Option<SliderDragState>,
}


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
                    TextureSelection::Current,

                palette:
                    PaletteSelection::Current,

                primitive_count:
                    32,

                anti_aliasing:
                    AntiAliasingSelection::Off,

                dithering:
                    DitheringSelection::Off,

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
        shader_loaded: bool,
        texture_required: bool,
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
                / EDIT_WINDOW_REFERENCE_HEIGHT)
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


        let initial_size =
            egui::vec2(
                screen_size_points.x
                    * EDIT_WINDOW_INITIAL_WIDTH_FRACTION,
                screen_size_points.y
                    * EDIT_WINDOW_INITIAL_HEIGHT_FRACTION,
            );

        let maximum_size =
            egui::vec2(
                screen_size_points.x
                    * EDIT_WINDOW_MAXIMUM_WIDTH_FRACTION,
                screen_size_points.y
                    * EDIT_WINDOW_MAXIMUM_HEIGHT_FRACTION,
            );

        let minimum_size =
            egui::vec2(
                460.0 * resolution_scale,
                500.0 * resolution_scale,
            )
            .min(
                maximum_size
            );

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

        let mut save_requested =
            false;

        let mut cancel_requested =
            false;

        let mut delete_requested =
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
                        )
                    }
                );


        let mut render_scale_drag_state =
            self.render_scale_drag_state;

        let shift_held =
            self.shift_held;


        let current_configuration_before_ui =
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
            );

        let configuration_changed_before_ui =
            current_configuration_before_ui
                .differs_from(
                    baseline_configuration
                );

        let editor_title =
            if configuration_changed_before_ui {
                "* Shader Policy Editor (ESC or Q to exit)"
            } else {
                "Shader Policy Editor (ESC or Q to exit)"
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
                        true
                    )
                    .show(
                        context,
                        |ui| {
                            let mut hover_help_message:
                                Option<&'static str> =
                                None;

                            draw_shader_header_row(
                                ui,
                                metrics,
                                shader_loaded,
                                &mut status_message,
                                &mut hover_help_message,
                            );

                            ui.add_space(
                                metrics.panel_gap
                            );

                            ui.columns(
                                3,
                                |columns| {
                                    draw_render_panel(
                                        &mut columns[0],
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

                                    draw_texture_panel(
                                        &mut columns[1],
                                        metrics,
                                        texture_required,
                                        &mut texture,
                                        &mut palette,
                                        &mut primitive_count,
                                        &mut hover_help_message,
                                    );

                                    draw_post_processing_panel(
                                        &mut columns[2],
                                        metrics,
                                        &mut anti_aliasing,
                                        &mut dithering,
                                        &mut hover_help_message,
                                    );
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

                            ui.add_space(
                                metrics.panel_gap
                            );

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

                            let can_delete_shader =
                                false;

                            ui.columns(
                                3,
                                |columns| {
                                    draw_policy_target_panel(
                                        &mut columns[0],
                                        metrics,
                                        &mut policy_target,
                                        &mut status_message,
                                        &mut hover_help_message,
                                    );

                                    draw_policy_actions_panel(
                                        &mut columns[1],
                                        metrics,
                                        can_save,
                                        can_cancel,
                                        can_delete_shader,
                                        &mut save_requested,
                                        &mut cancel_requested,
                                        &mut delete_requested,
                                        &mut displayed_fps,
                                        &mut displayed_animation_speed,
                                        &mut displayed_render_scale,
                                        &mut policy_target,
                                        &mut texture,
                                        &mut palette,
                                        &mut primitive_count,
                                        &mut anti_aliasing,
                                        &mut dithering,
                                        baseline_configuration,
                                        &mut fps_drag_state,
                                        &mut animation_speed_drag_state,
                                        &mut render_scale_drag_state,
                                        &mut status_message,
                                        &mut hover_help_message,
                                    );

                                    draw_about_panel(
                                        &mut columns[2],
                                        metrics,
                                    );
                                },
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
                                metrics.panel_gap
                            );

                            let displayed_status: &str =
                                match hover_help_message {
                                    Some(message) =>
                                        message,
                                    None =>
                                        status_message.as_str(),
                                };

                            draw_status_panel(
                                ui,
                                metrics,
                                displayed_status,
                                configuration_changed,
                            );
                        }
                    );
                }
            );

        self.window_open =
            window_open;

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

            save_requested,

            cancel_requested,

            delete_requested,

            window_open:
                self.window_open,
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


fn draw_shader_header_row(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    shader_loaded: bool,
    status_message: &mut String,
    hover_help_message: &mut Option<&'static str>,
) {
    ui.columns(
        2,
        |columns| {
            editor_theme::panel_frame(
                &columns[0],
                metrics,
            )
            .show(
                &mut columns[0],
                |ui| {
                    ui.horizontal(
                        |ui| {
                            ui.strong(
                                "Current Shader:"
                            );

                            ui.label(
                                if shader_loaded {
                                    "Loaded shader"
                                } else {
                                    "No shader loaded"
                                }
                            );

                            ui.with_layout(
                                egui::Layout::right_to_left(
                                    egui::Align::Center
                                ),
                                |ui| {
                                    ui.menu_button(
                                        "Load Shader",
                                        |ui| {
                                            ui.add_enabled(
                                                false,
                                                egui::Button::new(
                                                    "Recent shader placeholder"
                                                ),
                                            );

                                            ui.separator();

                                            let refresh_response =
                                                ui.add_enabled(
                                                    shader_loaded,
                                                    egui::Button::new(
                                                        "Refresh Shader"
                                                    ),
                                                );

                                            update_hover_help(
                                                &refresh_response,
                                                hover_help_message,
                                                "Re-read the current shader file from disk and update the preview.",
                                            );

                                            if refresh_response.clicked() {
                                                *status_message =
                                                    "Refresh Shader is not implemented in this checkpoint"
                                                        .to_string();
                                            }

                                            let browse_response =
                                                ui.button(
                                                    "Browse..."
                                                );

                                            update_hover_help(
                                                &browse_response,
                                                hover_help_message,
                                                "Select a shader file from another folder.",
                                            );

                                            if browse_response.clicked() {
                                                *status_message =
                                                    "Shader browsing is not implemented in this checkpoint"
                                                        .to_string();
                                            }

                                            ui.separator();

                                            let clear_response =
                                                ui.add_enabled(
                                                    false,
                                                    egui::Button::new(
                                                        "Clear Recent Files"
                                                    ),
                                                );

                                            update_hover_help(
                                                &clear_response,
                                                hover_help_message,
                                                "Remove saved shader-file history. No shader files will be deleted.",
                                            );
                                        },
                                    );
                                },
                            );
                        },
                    );
                },
            );

            editor_theme::panel_frame(
                &columns[1],
                metrics,
            )
            .show(
                &mut columns[1],
                |ui| {
                    editor_theme::section_heading(
                        ui,
                        "Shader Information",
                    );

                    egui::Grid::new(
                        "editor_shader_information_grid"
                    )
                    .num_columns(2)
                    .spacing(
                        egui::vec2(
                            8.0 * metrics.scale,
                            2.0 * metrics.scale,
                        )
                    )
                    .show(
                        ui,
                        |ui| {
                            ui.label("Path:");
                            ui.label("Provided by editor session");
                            ui.end_row();

                            ui.label("Type:");
                            ui.label("Shader file");
                            ui.end_row();

                            ui.label("Status:");
                            ui.label(
                                if shader_loaded {
                                    "Loaded"
                                } else {
                                    "No shader loaded"
                                }
                            );
                            ui.end_row();
                        },
                    );
                },
            );
        },
    );
}


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
    editor_theme::panel_frame(
        ui,
        metrics,
    )
    .show(
        ui,
        |ui| {
            editor_theme::section_heading(
                ui,
                "Render Controls",
            );

            ui.add_space(
                metrics.row_gap
            );

            let mut fps_value =
                *displayed_fps as f32;

            let fps_response =
                draw_editor_slider_row(
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
                draw_editor_slider_row(
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
                    animation_speed_drag_state,
                );

            update_hover_help(
                &speed_response,
                hover_help_message,
                "Adjust shader animation speed independently of frame rate. Hold Shift for fine adjustment.",
            );

            let scale_response =
                draw_editor_slider_row(
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


fn draw_texture_panel(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    texture_required: bool,
    texture: &mut TextureSelection,
    palette: &mut PaletteSelection,
    primitive_count: &mut u32,
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
                "Texture Settings",
            );

            ui.add_space(
                metrics.row_gap
            );

            ui.add_enabled_ui(
                texture_required,
                |ui| {
                    egui::Grid::new(
                        "editor_texture_settings_grid"
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
                            ui.label("Texture");
                            let response =
                                egui::ComboBox::from_id_source(
                                    "editor_texture_selection"
                                )
                                .selected_text("Current")
                                .width(metrics.dropdown_width)
                                .show_ui(
                                    ui,
                                    |ui| {
                                        ui.selectable_value(
                                            texture,
                                            TextureSelection::Current,
                                            "Current",
                                        );
                                    },
                                )
                                .response;

                            update_hover_help(
                                &response,
                                hover_help_message,
                                "Select the procedural texture generated in memory for this shader.",
                            );
                            ui.end_row();

                            ui.label("Palette");
                            let response =
                                egui::ComboBox::from_id_source(
                                    "editor_palette_selection"
                                )
                                .selected_text("Current")
                                .width(metrics.dropdown_width)
                                .show_ui(
                                    ui,
                                    |ui| {
                                        ui.selectable_value(
                                            palette,
                                            PaletteSelection::Current,
                                            "Current",
                                        );
                                    },
                                )
                                .response;

                            update_hover_help(
                                &response,
                                hover_help_message,
                                "Select the built-in color palette applied to the generated texture.",
                            );
                            ui.end_row();

                            ui.label("Primitives");
                            let response =
                                egui::ComboBox::from_id_source(
                                    "editor_primitive_count"
                                )
                                .selected_text(
                                    primitive_count.to_string()
                                )
                                .width(metrics.dropdown_width)
                                .show_ui(
                                    ui,
                                    |ui| {
                                        for value in [
                                            2_u32,
                                            4,
                                            8,
                                            16,
                                            32,
                                            64,
                                            128,
                                            256,
                                            512,
                                            1024,
                                        ] {
                                            ui.selectable_value(
                                                primitive_count,
                                                value,
                                                value.to_string(),
                                            );
                                        }
                                    },
                                )
                                .response;

                            update_hover_help(
                                &response,
                                hover_help_message,
                                "Set the number of graphical elements used to generate the procedural texture.",
                            );
                            ui.end_row();
                        },
                    );
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
        },
    );
}


fn draw_post_processing_panel(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    anti_aliasing: &mut AntiAliasingSelection,
    dithering: &mut DitheringSelection,
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
                },
            );
        },
    );
}


fn draw_policy_target_panel(
    ui: &mut egui::Ui,
    metrics: EditorMetrics,
    policy_target: &mut Option<PolicyTarget>,
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
                ui.radio(
                    *policy_target
                        == Some(
                            PolicyTarget::Screensaver
                        ),
                    "Screensaver",
                );

            update_hover_help(
                &screensaver_response,
                hover_help_message,
                "Save this policy for screensaver rendering.",
            );

            if screensaver_response.clicked() {
                *policy_target =
                    Some(
                        PolicyTarget::Screensaver
                    );
                *status_message =
                    "Ready".to_string();
            }

            let wallpaper_response =
                ui.radio(
                    *policy_target
                        == Some(
                            PolicyTarget::Wallpaper
                        ),
                    "Wallpaper",
                );

            update_hover_help(
                &wallpaper_response,
                hover_help_message,
                "Save this policy for wallpaper rendering.",
            );

            if wallpaper_response.clicked() {
                *policy_target =
                    Some(
                        PolicyTarget::Wallpaper
                    );
                *status_message =
                    "Ready".to_string();
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
                    "Save is not implemented in this checkpoint"
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


fn draw_editor_slider_row(
    ui: &mut egui::Ui,
    label: &str,
    displayed_value: &str,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    shift_held: bool,
    metrics: EditorMetrics,
    drag_state: &mut Option<SliderDragState>,
) -> egui::Response {
    let available_width =
        ui.available_width();

    let slider_width =
        (
            available_width
                - metrics.label_width
                - metrics.slider_value_width
                - 16.0 * metrics.scale
        )
        .max(
            48.0 * metrics.scale
        );

    let mut slider_response =
        None;

    egui::Grid::new(
        format!(
            "editor_slider_grid_{}",
            label,
        )
    )
    .num_columns(3)
    .spacing(
        egui::vec2(
            8.0 * metrics.scale,
            metrics.row_gap,
        )
    )
    .show(
        ui,
        |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(
                    metrics.label_width,
                    ui.spacing().interact_size.y,
                ),
                egui::Layout::left_to_right(
                    egui::Align::Center
                ),
                |ui| {
                    ui.label(label);
                },
            );

            ui.allocate_ui_with_layout(
                egui::vec2(
                    slider_width,
                    ui.spacing().interact_size.y,
                ),
                egui::Layout::left_to_right(
                    egui::Align::Center
                ),
                |ui| {
                    slider_response =
                        Some(
                            draw_fine_slider(
                                ui,
                                value,
                                minimum,
                                maximum,
                                shift_held,
                                metrics.scale,
                                drag_state,
                            )
                        );
                },
            );

            ui.allocate_ui_with_layout(
                egui::vec2(
                    metrics.slider_value_width,
                    ui.spacing().interact_size.y,
                ),
                egui::Layout::right_to_left(
                    egui::Align::Center
                ),
                |ui| {
                    ui.label(
                        displayed_value
                    );
                },
            );

            ui.end_row();
        },
    );

    slider_response.unwrap_or_else(
        || {
            ui.allocate_response(
                egui::Vec2::ZERO,
                egui::Sense::hover(),
            )
        }
    )
}


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

    let fraction =
        ((*value - minimum)
            / (maximum - minimum).max(f32::EPSILON))
            .clamp(
                0.0,
                1.0,
            );

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
                fraction,
            ),
            rect.center().y,
        );

    ui.painter().circle_filled(
        knob_center,
        7.0 * resolution_scale,
        widget_visuals.fg_stroke.color,
    );

    response
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


