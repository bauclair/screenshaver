//! Graphical layout and input handling for the Shader Policy Editor.
//!
//! This module owns the egui window, controls, layout, styling, and SDL-to-egui
//! input translation. Rendering and shader-session behavior remain in
//! `edit_shader`.

use std::sync::Arc;
use std::time::Instant;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;

// Editor-window geometry is expressed as a fraction of the active display.
// The maximum width and height multiply to exactly 0.125, limiting the
// window to one eighth of the full-screen area.
const EDIT_WINDOW_INITIAL_WIDTH_FRACTION: f32 =
    0.38;

const EDIT_WINDOW_INITIAL_HEIGHT_FRACTION: f32 =
    0.30;

const EDIT_WINDOW_MAXIMUM_WIDTH_FRACTION: f32 =
    0.40;

const EDIT_WINDOW_MAXIMUM_HEIGHT_FRACTION: f32 =
    0.3125;

const EDIT_WINDOW_REFERENCE_HEIGHT: f32 =
    1080.0;

const EDIT_WINDOW_SCALE_MIN: f32 =
    0.80;

const EDIT_WINDOW_SCALE_MAX: f32 =
    1.80;

const EDIT_CONTROL_LABEL_WIDTH: f32 =
    148.0;

const EDIT_CONTROL_VALUE_WIDTH: f32 =
    72.0;


#[derive(Clone, Copy)]
struct SliderDragState {
    anchor_value: f32,
    anchor_pointer_x: f32,
    shift_held: bool,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PolicyTarget {
    Screensaver,
    Wallpaper,
}


#[derive(Clone, Copy)]
struct EditorConfiguration {
    fps: u32,
    animation_speed: f32,
    render_scale: f32,
    policy_target: Option<PolicyTarget>,
}


impl EditorConfiguration {
    fn new(
        fps: u32,
        animation_speed: f32,
        render_scale: f32,
        policy_target: Option<PolicyTarget>,
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
    ) -> Option<(u32, f32, f32)> {

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


        configure_editor_style(
            &self.context,
            resolution_scale,
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
                240.0 * resolution_scale,
                110.0 * resolution_scale,
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


        let baseline_configuration =
            *self.initial_configuration
                .get_or_insert_with(
                    || {
                        EditorConfiguration::new(
                            displayed_fps,
                            displayed_animation_speed,
                            displayed_render_scale,
                            policy_target,
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
                            let mut fps_value =
                                displayed_fps as f32;

                            draw_editor_slider_row(
                                ui,
                                "FPS",
                                &format!(
                                    "{} FPS",
                                    displayed_fps,
                                ),
                                &mut fps_value,
                                crate::define_constants::MIN_RENDER_FPS as f32,
                                crate::define_constants::MAX_RENDER_FPS as f32,
                                shift_held,
                                resolution_scale,
                                &mut fps_drag_state,
                            );

                            displayed_fps =
                                fps_value.round()
                                    .clamp(
                                        crate::define_constants::MIN_RENDER_FPS as f32,
                                        crate::define_constants::MAX_RENDER_FPS as f32,
                                    ) as u32;


                            ui.add_space(
                                10.0 * resolution_scale
                            );


                            draw_editor_slider_row(
                                ui,
                                "Animation Speed",
                                &format!(
                                    "{:.2}x",
                                    displayed_animation_speed,
                                ),
                                &mut displayed_animation_speed,
                                crate::define_constants::SCREENSAVER_SPEED_MIN,
                                crate::define_constants::SCREENSAVER_SPEED_MAX,
                                shift_held,
                                resolution_scale,
                                &mut animation_speed_drag_state,
                            );

                            displayed_animation_speed =
                                (displayed_animation_speed * 100.0)
                                    .round()
                                    / 100.0;


                            ui.add_space(
                                10.0 * resolution_scale
                            );


                            draw_editor_slider_row(
                                ui,
                                "Render Scale",
                                &format!(
                                    "{:.2}x",
                                    displayed_render_scale,
                                ),
                                &mut displayed_render_scale,
                                crate::define_constants::RENDER_SCALE_MIN,
                                crate::define_constants::RENDER_SCALE_MAX,
                                shift_held,
                                resolution_scale,
                                &mut render_scale_drag_state,
                            );

                            displayed_render_scale =
                                (displayed_render_scale * 100.0)
                                    .round()
                                    / 100.0;


                            ui.add_space(
                                12.0 * resolution_scale
                            );

                            ui.separator();

                            ui.horizontal(
                                |ui| {
                                    ui.label(
                                        "Policy Target"
                                    );

                                    if ui.radio(
                                        policy_target
                                            == Some(
                                                PolicyTarget::Screensaver
                                            ),
                                        "Screensaver",
                                    )
                                    .clicked()
                                    {
                                        policy_target =
                                            Some(
                                                PolicyTarget::Screensaver
                                            );
                                        status_message =
                                            "Ready".to_string();
                                    }

                                    if ui.radio(
                                        policy_target
                                            == Some(
                                                PolicyTarget::Wallpaper
                                            ),
                                        "Wallpaper",
                                    )
                                    .clicked()
                                    {
                                        policy_target =
                                            Some(
                                                PolicyTarget::Wallpaper
                                            );
                                        status_message =
                                            "Ready".to_string();
                                    }
                                }
                            );

                            let current_configuration =
                                EditorConfiguration::new(
                                    displayed_fps,
                                    displayed_animation_speed,
                                    displayed_render_scale,
                                    policy_target,
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

                            ui.add_space(
                                8.0 * resolution_scale
                            );

                            ui.horizontal(
                                |ui| {
                                    let save_response =
                                        ui.add_enabled(
                                            can_save,
                                            egui::Button::new(
                                                "Save"
                                            ),
                                        );

                                    if !can_save
                                        && configuration_changed
                                        && policy_target.is_none()
                                    {
                                        status_message =
                                            "Select a policy target before saving"
                                                .to_string();
                                    }

                                    if save_response.clicked() {
                                        status_message =
                                            "Save is not implemented in this checkpoint"
                                                .to_string();
                                    }

                                    let cancel_response =
                                        ui.add_enabled(
                                            can_cancel,
                                            egui::Button::new(
                                                "Cancel"
                                            ),
                                        );

                                    if cancel_response.clicked() {
                                        displayed_fps =
                                            baseline_configuration.fps;
                                        displayed_animation_speed =
                                            baseline_configuration.animation_speed;
                                        displayed_render_scale =
                                            baseline_configuration.render_scale;
                                        policy_target =
                                            baseline_configuration.policy_target;
                                        fps_drag_state =
                                            None;
                                        animation_speed_drag_state =
                                            None;
                                        render_scale_drag_state =
                                            None;
                                        status_message =
                                            "Changes canceled"
                                                .to_string();
                                    }

                                    let delete_response =
                                        ui.add_enabled(
                                            can_delete_shader,
                                            egui::Button::new(
                                                "Delete Shader"
                                            ),
                                        );

                                    if delete_response.clicked() {
                                        status_message =
                                            "Delete Shader is not implemented"
                                                .to_string();
                                    }
                                }
                            );

                            let status_height =
                                24.0 * resolution_scale;

                            let remaining_height =
                                (
                                    ui.available_height()
                                        - status_height
                                )
                                    .max(
                                        0.0
                                    );

                            ui.allocate_space(
                                egui::vec2(
                                    ui.available_width(),
                                    remaining_height,
                                )
                            );

                            ui.separator();

                            ui.horizontal(
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(
                                            status_message.as_str()
                                        )
                                        .font(
                                            egui::FontId::proportional(
                                                13.0 * resolution_scale
                                            )
                                        )
                                    );
                                }
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


        if self.window_open {
            Some(
                (
                    displayed_fps,
                    displayed_animation_speed,
                    displayed_render_scale,
                )
            )
        } else {
            None
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


fn draw_editor_slider_row(
    ui: &mut egui::Ui,
    label: &str,
    displayed_value: &str,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    shift_held: bool,
    resolution_scale: f32,
    drag_state: &mut Option<SliderDragState>,
) -> egui::Response {

    let label_width =
        EDIT_CONTROL_LABEL_WIDTH
            * resolution_scale;

    let value_width =
        EDIT_CONTROL_VALUE_WIDTH
            * resolution_scale;

    let column_gap =
        12.0
            * resolution_scale;

    let row_height =
        ui.spacing().interact_size.y;

    let available_width =
        ui.available_width();

    let slider_width =
        (
            available_width
                - label_width
                - value_width
                - column_gap * 2.0
        )
            .max(
                40.0 * resolution_scale
            );

    let total_width =
        label_width
            + column_gap
            + slider_width
            + column_gap
            + value_width;

    let (
        row_rect,
        _row_response,
    ) =
        ui.allocate_exact_size(
            egui::vec2(
                total_width.min(
                    available_width
                ),
                row_height,
            ),
            egui::Sense::hover(),
        );

    let label_rect =
        egui::Rect::from_min_size(
            row_rect.min,
            egui::vec2(
                label_width,
                row_height,
            ),
        );

    let slider_rect =
        egui::Rect::from_min_size(
            egui::pos2(
                label_rect.right()
                    + column_gap,
                row_rect.top(),
            ),
            egui::vec2(
                slider_width,
                row_height,
            ),
        );

    let value_rect =
        egui::Rect::from_min_size(
            egui::pos2(
                slider_rect.right()
                    + column_gap,
                row_rect.top(),
            ),
            egui::vec2(
                value_width,
                row_height,
            ),
        );

    ui.allocate_ui_at_rect(
        label_rect,
        |ui| {
            ui.with_layout(
                egui::Layout::left_to_right(
                    egui::Align::Center
                ),
                |ui| {
                    ui.label(
                        label
                    );
                },
            );
        },
    );

    let mut slider_response =
        None;

    ui.allocate_ui_at_rect(
        slider_rect,
        |ui| {
            slider_response =
                Some(
                    draw_fine_slider(
                        ui,
                        value,
                        minimum,
                        maximum,
                        shift_held,
                        resolution_scale,
                        drag_state,
                    )
                );
        },
    );

    ui.allocate_ui_at_rect(
        value_rect,
        |ui| {
            ui.with_layout(
                egui::Layout::right_to_left(
                    egui::Align::Center
                ),
                |ui| {
                    ui.label(
                        displayed_value
                    );
                },
            );
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


fn configure_editor_style(
    context: &egui::Context,
    resolution_scale: f32,
) {

    let mut style =
        (*context.style()).clone();


    style.spacing.item_spacing =
        egui::vec2(
            8.0 * resolution_scale,
            6.0 * resolution_scale,
        );

    style.spacing.window_margin =
        egui::Margin::same(
            (8.0 * resolution_scale)
                .round() as i8
        );

    style.spacing.button_padding =
        egui::vec2(
            8.0 * resolution_scale,
            4.0 * resolution_scale,
        );

    style.spacing.interact_size.y =
        18.0 * resolution_scale;

    style.visuals.resize_corner_size =
        12.0 * resolution_scale;


    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(
            14.0 * resolution_scale
        ),
    );

    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(
            14.0 * resolution_scale
        ),
    );

    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(
            16.0 * resolution_scale
        ),
    );


    context.set_style(
        style
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


