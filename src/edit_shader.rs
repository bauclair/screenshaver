//! Full-screen interactive shader-edit rendering session.
//!
//! This checkpoint renders one optional shader continuously beneath an empty
//! movable and resizable egui editor window. When no shader is supplied, the
//! session displays a black full-screen background. Ordinary keyboard and mouse
//! activity do not terminate edit mode.

use std::collections::VecDeque;
use std::path::{
    Path,
    PathBuf,
};
use std::sync::Arc;
use std::time::{
    Duration,
    Instant,
};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::video::{
    FullscreenType,
    GLProfile,
};
use crate::parse_texture_specification::TextureSpecification;


const FPS_AVERAGE_WINDOW: Duration =
    Duration::from_secs(5);

const FPS_CRITICAL_BLINK_INTERVAL: Duration =
    Duration::from_millis(500);


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


/// Minimal egui integration for the first graphical-window checkpoint.
///
/// This owns only the egui context, OpenGL painter, pointer events, and the
/// empty movable/resizable window. Shader controls are intentionally omitted.
struct EditWindowOverlay {
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
}


impl EditWindowOverlay {
    fn new(
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
            }
        )
    }


    fn handle_event(
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
                    egui::Modifiers::default(),
            }
        );
    }


    fn display(
        &mut self,
        window: &sdl2::video::Window,
        resolved_fps: u32,
    ) -> Option<u32> {

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
                        "Shader Editor (ESC or Q to exit)"
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
                            ui.horizontal(
                                |ui| {
                                    ui.label(
                                        "FPS"
                                    );

                                    ui.with_layout(
                                        egui::Layout::right_to_left(
                                            egui::Align::Center
                                        ),
                                        |ui| {
                                            ui.label(
                                                format!(
                                                    "{} FPS",
                                                    displayed_fps,
                                                )
                                            );
                                        }
                                    );
                                }
                            );


                            let slider_width =
                                ui.available_width();

                            ui.spacing_mut().slider_width =
                                slider_width;

                            ui.add(
                                egui::Slider::new(
                                    &mut displayed_fps,
                                    crate::define_constants::MIN_RENDER_FPS
                                        ..=crate::define_constants::MAX_RENDER_FPS,
                                )
                                .show_value(
                                    false
                                ),
                            );


                            // Preserve the established editor-window geometry while
                            // this checkpoint contains only the first slider.
                            ui.allocate_space(
                                ui.available_size()
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
            Some(displayed_fps)
        } else {
            None
        }
    }


    fn destroy(
        &mut self,
    ) {
        self.painter.destroy();
    }
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


struct FrameTimeWindow {
    samples: VecDeque<(Instant, Duration)>,
    total: Duration,
}


impl FrameTimeWindow {
    fn new() -> Self {
        Self {
            samples: VecDeque::new(),
            total: Duration::ZERO,
        }
    }


    fn record(
        &mut self,
        elapsed: Duration,
        configured_fps: u32,
    ) -> crate::fps_monitor::FpsWarningState {
        let now = Instant::now();

        self.samples.push_back(
            (now, elapsed)
        );

        self.total += elapsed;

        while let Some((timestamp, duration)) =
            self.samples.front().copied()
        {
            if now.duration_since(timestamp)
                <= FPS_AVERAGE_WINDOW
            {
                break;
            }

            self.samples.pop_front();
            self.total = self.total.saturating_sub(
                duration
            );
        }

        let sample_count =
            self.samples.len() as u32;

        if sample_count == 0 {
            return crate::fps_monitor::FpsWarningState::Normal;
        }

        let average_seconds =
            self.total.as_secs_f64()
                / sample_count as f64;

        let ideal_seconds =
            1.0 / configured_fps.max(1) as f64;

        if average_seconds
            > ideal_seconds * 2.0
        {
            crate::fps_monitor::FpsWarningState::Critical
        } else if average_seconds
            > ideal_seconds * 1.5
        {
            crate::fps_monitor::FpsWarningState::Warning
        } else {
            crate::fps_monitor::FpsWarningState::Normal
        }
    }
}


struct ActivePreviewShader {
    path:
        PathBuf,

    shader_name:
        String,

    program:
        u32,

    channel_usage:
        crate::preprocess_shader::ShaderChannelUsage,

    shader_inputs:
        Vec<crate::isf_types::ShaderInput>,

    texture_manager:
        crate::manage_textures::TextureManager,

    overlay_descriptor:
        crate::construct_text_overlay::OverlayDescriptor,

    subtitle_overlay:
        Option<
            crate::display_overlay::OpenGlOverlay
        >,

    overlay_output_size:
        (
            u32,
            u32,
        ),

    fps_warning_state:
        crate::fps_monitor::FpsWarningState,

    fps_blink_visible:
        bool,

    last_fps_blink:
        Instant,

    frame_times:
        FrameTimeWindow,

    start_time:
        Instant,

    previous_frame:
        Instant,

    frame:
        i32,
}


pub fn run(
    shader_argument: Option<String>,
) -> Result<(), String> {

    let Some(shader_argument) =
        shader_argument
    else {
        return run_empty_session();
    };


    match crate::preview_shader_directory::resolve_preview_target(
        &shader_argument
    )? {

        crate::preview_shader_directory::PreviewTarget::File(
            path
        ) => {

            run_paths(
                vec![
                    path
                ],
                None,
                None,
                None,
                None,
                None,
            )
        }

        crate::preview_shader_directory::PreviewTarget::Directory(
            path
        ) => {

            Err(
                format!(
                    "--edit-shader requires a shader file, not a directory: {}",
                    path.display(),
                )
            )
        }
    }
}


fn run_empty_session() -> Result<(), String> {

    let _wallpaper_pause_guard =
        crate::control_wallpaper::WallpaperPauseGuard::acquire();


    let sdl =
        sdl2::init()
            .map_err(
                |error| {
                    format!(
                        "SDL initialization failed: {}",
                        error,
                    )
                }
            )?;


    let video =
        sdl.video()
            .map_err(
                |error| {
                    format!(
                        "SDL video initialization failed: {}",
                        error,
                    )
                }
            )?;


    {
        let gl_attr =
            video.gl_attr();

        gl_attr.set_context_profile(
            GLProfile::Core
        );

        gl_attr.set_context_version(
            crate::define_constants::GL_MAJOR,
            crate::define_constants::GL_MINOR,
        );
    }


    let mut window =
        video
            .window(
                "Screenshaver Shader Editor",
                0,
                0,
            )
            .fullscreen_desktop()
            .borderless()
            .opengl()
            .build()
            .map_err(
                |error| {
                    format!(
                        "Unable to create edit-shader window: {}",
                        error,
                    )
                }
            )?;


    let gl_context =
        window
            .gl_create_context()
            .map_err(
                |error| {
                    format!(
                        "Unable to create edit-shader OpenGL context: {}",
                        error,
                    )
                }
            )?;


    window.raise();

    let _ =
        window.set_fullscreen(
            FullscreenType::Desktop
        );


    gl::load_with(
        |symbol| {
            video.gl_get_proc_address(
                symbol
            ) as *const _
        }
    );


    let _ =
        video.gl_set_swap_interval(
            0
        );


    let mut edit_window =
        EditWindowOverlay::new(
            &video
        )?;


    let mut event_pump =
        sdl.event_pump()
            .map_err(
                |error| {
                    format!(
                        "Unable to create edit-shader SDL event pump: {}",
                        error,
                    )
                }
            )?;


    'edit_session: loop {

        for event in
            event_pump.poll_iter()
        {
            edit_window.handle_event(
                &event
            );

            if edit_session_should_close(
                &event
            ) {
                break 'edit_session;
            }
        }


        let (
            width,
            height,
        ) =
            window.drawable_size();


        unsafe {
            gl::Viewport(
                0,
                0,
                width.min(i32::MAX as u32) as i32,
                height.min(i32::MAX as u32) as i32,
            );

            gl::ClearColor(
                0.0,
                0.0,
                0.0,
                1.0,
            );

            gl::Clear(
                gl::COLOR_BUFFER_BIT
            );
        }


        if edit_window.display(
            &window,
            crate::define_constants::DEFAULT_RENDER_FPS,
        )
        .is_none()
        {
            break 'edit_session;
        }


        window.gl_swap_window();

        std::thread::sleep(
            Duration::from_millis(10)
        );
    }


    edit_window.destroy();


    drop(
        gl_context
    );


    Ok(())
}


fn edit_session_should_close(
    event: &Event,
) -> bool {

    matches!(
        event,
        Event::Quit {
            ..
        }
        | Event::KeyDown {
            keycode:
                Some(
                    Keycode::Escape
                    | Keycode::Q
                ),
            repeat: false,
            ..
        }
    )
}

fn run_paths(
    shader_paths: Vec<PathBuf>,
    shader_texture: Option<TextureSpecification>,
    shader_palette: Option<String>,
    interval_seconds: Option<u64>,
    command_line_fps: Option<u32>,
    animation_speed: Option<f32>,
) -> Result<(), String> {

    if shader_paths.is_empty() {
        return Err(
            "No shader path was supplied for editing"
                .to_string()
        );
    }


    let animation_speed =
        animation_speed.unwrap_or(
            1.0
        );


    let config_result =
        crate::load_config::load_config(
            &crate::locate_paths::config_path()
        )?;


    let config =
        config_result.config;


    let subtitles =
        config.subtitles;


    let subtitle_placement =
        config.subtitle_placement;


    let global_rendered_fps =
        config.global_rendered_fps;


    let fps_policy_entries =
        config.screensaver_fps_policy_entries;


    let texture_policy =
        config.texture_policy;

    let postprocess_policy =
        config.screensaver_postprocess_policy;


    let preview_selection =
        parse_preview_selection(
            shader_texture.as_ref(),
            shader_palette.as_deref(),
        )?;


    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::information(
        &logfile,
        &format!(
            "[EDIT_SHADER] Edit session contains {} shader path(s)",
            shader_paths.len(),
        ),
    );


    crate::logger::information(
        &logfile,
        &format!(
            "[EDIT_SHADER] Animation speed: {:.3}x",
            animation_speed,
        ),
    );


    let _wallpaper_pause_guard =
        crate::control_wallpaper::WallpaperPauseGuard::acquire();


    let sdl =
        sdl2::init()
            .map_err(
                |error| {
                    format!(
                        "SDL initialization failed: {}",
                        error,
                    )
                }
            )?;


    let video =
        sdl.video()
            .map_err(
                |error| {
                    format!(
                        "SDL video initialization failed: {}",
                        error,
                    )
                }
            )?;


    {
        let gl_attr =
            video.gl_attr();


        gl_attr.set_context_profile(
            GLProfile::Core
        );


        gl_attr.set_context_version(
            crate::define_constants::GL_MAJOR,
            crate::define_constants::GL_MINOR,
        );
    }


    let mut window =
        video
            .window(
                "Screenshaver Shader Editor",
                0,
                0,
            )
            .fullscreen_desktop()
            .borderless()
            .opengl()
            .build()
            .map_err(
                |error| {
                    format!(
                        "Unable to create edit-shader window: {}",
                        error,
                    )
                }
            )?;


    let gl_context =
        window
            .gl_create_context()
            .map_err(
                |error| {
                    format!(
                        "Unable to create OpenGL context: {}",
                        error,
                    )
                }
            )?;


    window.raise();


    let _ =
        window.set_fullscreen(
            FullscreenType::Desktop
        );


    gl::load_with(
        |symbol| {
            video.gl_get_proc_address(
                symbol
            ) as *const _
        }
    );


    let _ =
        video.gl_set_swap_interval(
            0
        );


    let mut edit_window =
        EditWindowOverlay::new(
            &video
        )?;


    let (
        width,
        height,
    ) =
        window.size();


    let (
        mut active,
        mut active_index,
    ) =
        load_first_usable_shader(
            &shader_paths,
            0,
            &texture_policy,
            preview_selection,
            subtitles,
            subtitle_placement,
            global_rendered_fps,
            &fps_policy_entries,
            command_line_fps,
            animation_speed,
            width,
            height,
        )?;


    let initial_postprocess_profile =
        postprocess_policy.profile_for_shader(
            &active.shader_name
        );


    let mut postprocess =
        crate::postprocess_shader::PostprocessPipeline::new(
            width,
            height,
            initial_postprocess_profile,
        )?;


    let mut configured_fps =
        resolve_preview_fps(
            global_rendered_fps,
            &fps_policy_entries,
            command_line_fps,
            &active.shader_name,
        );


    let mut target_frame_time =
        Duration::from_secs_f64(
            1.0
                / configured_fps.max(1) as f64
        );


    let mut vao =
        0_u32;


    unsafe {
        gl::GenVertexArrays(
            1,
            &mut vao,
        );


        gl::BindVertexArray(
            vao
        );
    }


    let mut event_pump =
        sdl.event_pump()
            .map_err(
                |error| {
                    format!(
                        "Unable to create SDL event pump: {}",
                        error,
                    )
                }
            )?;


    discard_startup_input(
        &mut event_pump
    );


    let mut last_switch =
        Instant::now();


    let result =
        'preview: loop {

            for event in
                event_pump.poll_iter()
            {
                edit_window.handle_event(
                    &event
                );

                match event {

                    ref event
                        if edit_session_should_close(
                            event
                        ) =>
                    {
                        break 'preview Ok(());
                    }

                    _ => {
                        // Keyboard and mouse input remain active in edit mode
                        // and do not automatically terminate the session.
                    }
                }
            }


            if let Some(interval) =
                interval_seconds
            {
                if last_switch.elapsed()
                    >= Duration::from_secs(
                        interval
                    )
                {
                    let next_start =
                        (
                            active_index
                                + 1
                        )
                            % shader_paths.len();


                    match load_first_usable_shader(
                        &shader_paths,
                        next_start,
                        &texture_policy,
                        preview_selection,
                        subtitles,
                        subtitle_placement,
                        global_rendered_fps,
                        &fps_policy_entries,
                        command_line_fps,
                        animation_speed,
                        window.size().0,
                        window.size().1,
                    ) {

                        Ok(
                            (
                                replacement,
                                replacement_index,
                            )
                        ) => {

                            destroy_active_shader(
                                &mut active
                            );


                            active =
                                replacement;


                            postprocess.set_profile(
                                postprocess_policy.profile_for_shader(
                                    &active.shader_name
                                )
                            )?;


                            configured_fps =
                                resolve_preview_fps(
                                    global_rendered_fps,
                                    &fps_policy_entries,
                                    command_line_fps,
                                    &active.shader_name,
                                );


                            target_frame_time =
                                Duration::from_secs_f64(
                                    1.0
                                        / configured_fps.max(1) as f64
                                );


                            active_index =
                                replacement_index;


                            last_switch =
                                Instant::now();


                            log_information(
                                &format!(
                                    "[EDIT_SHADER] Switched to {}",
                                    active.path.display(),
                                )
                            );
                        }

                        Err(error) => {

                            log_warning(
                                &format!(
                                    "[EDIT_SHADER] Unable to select another usable shader: {}",
                                    error,
                                )
                            );


                            last_switch =
                                Instant::now();
                        }
                    }
                }
            }


            let frame_start =
                Instant::now();


            let (
                width,
                height,
            ) =
                window.size();


            let elapsed =
                active.start_time
                    .elapsed()
                    .as_secs_f32()
                    * animation_speed;


            let delta =
                active.previous_frame
                    .elapsed()
                    .as_secs_f32()
                    * animation_speed;


            let shader_render_start =
                Instant::now();


            postprocess.resize(
                width,
                height,
            )?;


            postprocess.bind_scene_target();


            let (
                scene_width,
                scene_height,
            ) =
                postprocess.scene_dimensions();


            unsafe {
                gl::ClearColor(
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                );


                gl::Clear(
                    gl::COLOR_BUFFER_BIT
                );


                gl::UseProgram(
                    active.program
                );


                active.texture_manager
                    .bind_channels();


                crate::apply_shader_inputs::apply(
                    active.program,
                    &active.shader_inputs,
                );


                gl::BindVertexArray(
                    vao
                );


                set_uniform_1f(
                    active.program,
                    b"iTime\0",
                    elapsed,
                );


                set_uniform_1f(
                    active.program,
                    b"iTimeDelta\0",
                    delta,
                );


                set_uniform_1i(
                    active.program,
                    b"iFrame\0",
                    active.frame,
                );


                set_uniform_3f(
                    active.program,
                    b"iResolution\0",
                    scene_width as f32,
                    scene_height as f32,
                    1.0,
                );


                set_uniform_4f(
                    active.program,
                    b"iMouse\0",
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                );


                gl::DrawArrays(
                    gl::TRIANGLES,
                    0,
                    3,
                );
            }


            postprocess.present_scene();


            unsafe {
                gl::Finish();
            }


            let warning_state =
                active.frame_times.record(
                    shader_render_start.elapsed(),
                    configured_fps,
                );


            let warning_changed =
                warning_state
                    != active.fps_warning_state;


            if warning_changed {
                active.fps_warning_state =
                    warning_state;

                active.fps_blink_visible =
                    true;

                active.last_fps_blink =
                    Instant::now();
            }


            let mut blink_changed =
                false;


            if active.fps_warning_state
                == crate::fps_monitor::FpsWarningState::Critical
                && active.last_fps_blink.elapsed()
                    >= FPS_CRITICAL_BLINK_INTERVAL
            {
                active.fps_blink_visible =
                    !active.fps_blink_visible;

                active.last_fps_blink =
                    Instant::now();

                blink_changed =
                    true;
            }


            let overlay_warning_state =
                if active.fps_warning_state
                    == crate::fps_monitor::FpsWarningState::Critical
                    && !active.fps_blink_visible
                {
                    crate::fps_monitor::FpsWarningState::CriticalHidden
                } else {
                    active.fps_warning_state
                };


            let warning_overlay_active =
                active.fps_warning_state
                    != crate::fps_monitor::FpsWarningState::Normal;


            let overlay_should_display =
                subtitles
                    || warning_overlay_active;


            if overlay_should_display {

                let current_size =
                    (
                        width,
                        height,
                    );


                if current_size
                    != active.overlay_output_size
                    || warning_changed
                    || blink_changed
                    || active.subtitle_overlay.is_none()
                {
                    let warning_only_descriptor =
                        crate::construct_text_overlay::OverlayDescriptor::default();


                    let overlay_descriptor =
                        if subtitles {
                            &active.overlay_descriptor
                        } else {
                            &warning_only_descriptor
                        };


                    active.subtitle_overlay =
                        Some(
                            crate::display_overlay::OpenGlOverlay::new_with_fps_warning(
                                overlay_descriptor,
                                configured_fps,
                                overlay_warning_state,
                                subtitle_placement,
                                width,
                                height,
                            )?
                        );


                    active.overlay_output_size =
                        current_size;
                }


                if let Some(overlay) =
                    active.subtitle_overlay.as_ref()
                {
                    overlay.display(
                        width,
                        height,
                    );
                }

            } else {

                active.subtitle_overlay =
                    None;
            }


            let Some(selected_fps) =
                edit_window.display(
                    &window,
                    configured_fps,
                )
            else {
                break 'preview Ok(());
            };


            if selected_fps
                != configured_fps
            {
                configured_fps =
                    selected_fps;

                target_frame_time =
                    Duration::from_secs_f64(
                        1.0
                            / configured_fps.max(1) as f64
                    );

                active.frame_times =
                    FrameTimeWindow::new();

                active.fps_warning_state =
                    crate::fps_monitor::FpsWarningState::Normal;

                active.fps_blink_visible =
                    true;

                active.last_fps_blink =
                    Instant::now();

                active.subtitle_overlay =
                    None;

                log_information(
                    &format!(
                        "[EDIT_SHADER] Live FPS target changed to {}",
                        configured_fps,
                    )
                );
            }


            window.gl_swap_window();


            active.frame =
                active.frame.saturating_add(
                    1
                );


            active.previous_frame =
                Instant::now();


            let render_elapsed =
                frame_start.elapsed();


            if render_elapsed
                < target_frame_time
            {
                std::thread::sleep(
                    target_frame_time
                        - render_elapsed
                );
            }
        };


    destroy_active_shader(
        &mut active
    );


    unsafe {
        if vao
            != 0
        {
            gl::DeleteVertexArrays(
                1,
                &vao,
            );
        }
    }


    drop(
        postprocess
    );


    edit_window.destroy();


    drop(
        gl_context
    );


    log_information(
        "[EDIT_SHADER] Edit session closed"
    );


    result
}


fn resolve_preview_fps(
    global_rendered_fps: u32,
    fps_policy_entries:
        &[
            crate::load_config::FpsPolicyEntry
        ],
    command_line_fps: Option<u32>,
    shader_name: &str,
) -> u32 {

    if let Some(fps) =
        command_line_fps
    {
        return fps.max(
            1
        );
    }


    fps_policy_entries
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
            global_rendered_fps
        )
        .max(
            1
        )
}


fn load_first_usable_shader(
    paths: &[PathBuf],
    start_index: usize,
    texture_policy:
        &crate::load_config::TexturePolicy,
    preview_selection:
        crate::manage_textures::PreviewTextureSelection,
    subtitles: bool,
    subtitle_placement:
        crate::parse_subtitle_placement::SubtitlePlacement,
    global_rendered_fps: u32,
    fps_policy_entries:
        &[
            crate::load_config::FpsPolicyEntry
        ],
    command_line_fps: Option<u32>,
    animation_speed: f32,
    output_width: u32,
    output_height: u32,
) -> Result<
    (
        ActivePreviewShader,
        usize,
    ),
    String,
> {

    for offset in
        0..paths.len()
    {
        let index =
            (
                start_index
                    + offset
            )
                % paths.len();


        let path =
            &paths[
                index
            ];


        let shader_name =
            path.file_name()
                .and_then(
                    |value| {
                        value.to_str()
                    }
                )
                .unwrap_or_default();


        let configured_fps =
            resolve_preview_fps(
                global_rendered_fps,
                fps_policy_entries,
                command_line_fps,
                shader_name,
            );


        match load_active_shader(
            path,
            texture_policy,
            preview_selection,
            subtitles,
            subtitle_placement,
            configured_fps,
            animation_speed,
            output_width,
            output_height,
        ) {

            Ok(shader) => {
                return Ok(
                    (
                        shader,
                        index,
                    )
                );
            }

            Err(error) => {

                log_warning(
                    &format!(
                        "[EDIT_SHADER] Skipping '{}': {}",
                        path.display(),
                        error,
                    )
                );
            }
        }
    }


    Err(
        "The selected shader could not be loaded for editing"
            .to_string()
    )
}


fn load_active_shader(
    path: &Path,
    texture_policy:
        &crate::load_config::TexturePolicy,
    preview_selection:
        crate::manage_textures::PreviewTextureSelection,
    subtitles: bool,
    subtitle_placement:
        crate::parse_subtitle_placement::SubtitlePlacement,
    configured_fps: u32,
    animation_speed: f32,
    output_width: u32,
    output_height: u32,
) -> Result<
    ActivePreviewShader,
    String,
> {

    let loaded =
        crate::load_shader::load_shader_for_preview(
            path
        );


    let (
        source,
        shader_name,
        channel_usage,
        shader_inputs,
    ) =
        match loaded {

            crate::load_shader::ShaderLoadResult::Ready {
                source,
                shader_name,
                channel_usage,
                shader_inputs,
                ..
            } => {
                (
                    source,
                    shader_name,
                    channel_usage,
                    shader_inputs,
                )
            }

            crate::load_shader::ShaderLoadResult::Rejected {
                reasons,
                ..
            } => {
                return Err(
                    format!(
                        "rejected: {}",
                        reasons.join(
                            "; "
                        ),
                    )
                );
            }

            crate::load_shader::ShaderLoadResult::Unavailable {
                error,
                ..
            } => {
                return Err(
                    format!(
                        "unavailable: {}",
                        error,
                    )
                );
            }
        };


    let program =
        crate::compile_shader::build_program(
            crate::define_constants::VERTEX_SHADER,
            &source,
        )?;


    let mut texture_manager =
        crate::manage_textures::TextureManager::new(
            texture_policy.clone()
        );


    if let Err(error) =
        texture_manager.prepare_for_shader_with_selection(
            &shader_name,
            channel_usage,
            preview_selection,
        )
    {
        unsafe {
            gl::DeleteProgram(
                program
            );
        }


        return Err(
            error
        );
    }


    texture_manager.configure_program(
        program
    );


    let (
        texture,
        palette,
    ) =
        texture_manager
            .active_specification_selection()
            .map(
                |(
                    specification,
                    palette,
                )| {
                    let texture_name =
                        specification.display_name();


                    let texture_description =
                        if specification.count_was_explicit {

                            texture_name

                        } else {

                            format!(
                                "{} ({})",
                                texture_name,
                                specification.requested_primitive_count,
                            )
                        };


                    (
                        Some(
                            texture_description
                        ),
                        Some(
                            palette.to_string()
                        ),
                    )
                }
            )
            .unwrap_or(
                (
                    None,
                    None,
                )
            );


    let overlay_descriptor =
        crate::construct_text_overlay::OverlayDescriptor {
            shader:
                Some(
                    format!(
                        "{} | {}",
                        shader_name,
                        format_animation_speed(
                            animation_speed
                        ),
                    )
                ),

            texture,

            palette,
        };


    let subtitle_overlay =
        if subtitles {

            Some(
                crate::display_overlay::OpenGlOverlay::new_with_fps_warning(
                    &overlay_descriptor,
                    configured_fps,
                    crate::fps_monitor::FpsWarningState::Normal,
                    subtitle_placement,
                    output_width,
                    output_height,
                )?
            )

        } else {

            None
        };


    log_information(
        &format!(
            "[EDIT_SHADER] Active shader: {}",
            path.display(),
        )
    );


    Ok(
        ActivePreviewShader {
            path:
                path.to_path_buf(),
            shader_name,
            program,
            channel_usage,
            shader_inputs,
            texture_manager,
            overlay_descriptor,
            subtitle_overlay,
            overlay_output_size:
                (
                    output_width,
                    output_height,
                ),
            fps_warning_state:
                crate::fps_monitor::FpsWarningState::Normal,
            fps_blink_visible:
                true,
            last_fps_blink:
                Instant::now(),
            frame_times:
                FrameTimeWindow::new(),
            start_time:
                Instant::now(),
            previous_frame:
                Instant::now(),
            frame:
                0,
        }
    )
}


fn format_animation_speed(
    speed: f32,
) -> String {

    if speed.fract()
        == 0.0
    {
        format!(
            "×{speed:.1}"
        )
    } else {
        format!(
            "×{speed}"
        )
    }
}


fn destroy_active_shader(
    active: &mut ActivePreviewShader,
) {

    active.subtitle_overlay =
        None;


    active.texture_manager
        .delete_all();


    unsafe {
        if active.program
            != 0
        {
            gl::DeleteProgram(
                active.program
            );


            active.program =
                0;
        }
    }
}


fn parse_preview_selection(
    texture_specification: Option<&TextureSpecification>,
    palette_name: Option<&str>,
) -> Result<
    crate::manage_textures::PreviewTextureSelection,
    String,
> {

let texture =
    match texture_specification.cloned() {

        Some(specification) => {

            Some(
                crate::manage_textures::PreviewSelectionValue::Specific(
                    specification
                )
            )
        }

        None => {
            None
        }
    };


    let palette =
        match palette_name {

            Some(
                "random"
            ) => {
                Some(
                    crate::manage_textures::PreviewSelectionValue::Random
                )
            }

            Some(
                name
            ) => {
                Some(
                    crate::manage_textures::PreviewSelectionValue::Specific(
                        crate::palettes::Palette::from_name(
                            name
                        )?
                    )
                )
            }

            None => {
                None
            }
        };


    Ok(
        crate::manage_textures::PreviewTextureSelection {
            texture,
            palette,
        }
    )
}


fn discard_startup_input(
    event_pump: &mut sdl2::EventPump,
) {

    let input_arm_time =
        Instant::now()
            + Duration::from_millis(
                500
            );


    while Instant::now()
        < input_arm_time
    {
        for _event in
            event_pump.poll_iter()
        {
            // Intentionally discarded.
        }


        std::thread::sleep(
            Duration::from_millis(
                10
            )
        );
    }
}


fn log_warning(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::warning(
        &logfile,
        message,
    );
}


fn log_information(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::information(
        &logfile,
        message,
    );
}


unsafe fn set_uniform_1f(
    program: u32,
    name: &[u8],
    value: f32,
) {

    let location =
        gl::GetUniformLocation(
            program,
            name.as_ptr()
                .cast(),
        );


    if location
        != -1
    {
        gl::Uniform1f(
            location,
            value,
        );
    }
}


unsafe fn set_uniform_1i(
    program: u32,
    name: &[u8],
    value: i32,
) {

    let location =
        gl::GetUniformLocation(
            program,
            name.as_ptr()
                .cast(),
        );


    if location
        != -1
    {
        gl::Uniform1i(
            location,
            value,
        );
    }
}


unsafe fn set_uniform_3f(
    program: u32,
    name: &[u8],
    x: f32,
    y: f32,
    z: f32,
) {

    let location =
        gl::GetUniformLocation(
            program,
            name.as_ptr()
                .cast(),
        );


    if location
        != -1
    {
        gl::Uniform3f(
            location,
            x,
            y,
            z,
        );
    }
}


unsafe fn set_uniform_4f(
    program: u32,
    name: &[u8],
    x: f32,
    y: f32,
    z: f32,
    w: f32,
) {

    let location =
        gl::GetUniformLocation(
            program,
            name.as_ptr()
                .cast(),
        );


    if location
        != -1
    {
        gl::Uniform4f(
            location,
            x,
            y,
            z,
            w,
        );
    }
}

