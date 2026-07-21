use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::video::{GLContext, GLProfile, Window};


const INPUT_STARTUP_GRACE: Duration = Duration::from_millis(750);
const MOUSE_MOTION_EXIT_THRESHOLD: i32 = 4;

const FPS_AVERAGE_WINDOW: Duration =
    Duration::from_secs(5);

const FPS_CRITICAL_BLINK_INTERVAL: Duration =
    Duration::from_millis(500);


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


    fn clear(
        &mut self,
    ) {
        self.samples.clear();
        self.total = Duration::ZERO;
    }


    fn record(
        &mut self,
        elapsed: Duration,
        configured_fps: u32,
    ) -> crate::construct_text_overlay::FpsWarningState {
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
            return crate::construct_text_overlay::FpsWarningState::Normal;
        }

        let average_seconds =
            self.total.as_secs_f64()
                / sample_count as f64;

        let ideal_seconds =
            1.0 / configured_fps.max(1) as f64;

        if average_seconds
            > ideal_seconds * 2.0
        {
            crate::construct_text_overlay::FpsWarningState::Critical
        } else if average_seconds
            > ideal_seconds * 1.5
        {
            crate::construct_text_overlay::FpsWarningState::Warning
        } else {
            crate::construct_text_overlay::FpsWarningState::Normal
        }
    }
}


#[derive(Debug)]
struct ActiveShader {
    program: u32,
    shader_name: String,
    channel_usage: crate::preprocess_shader::ShaderChannelUsage,
    shader_inputs: Vec<crate::isf_types::ShaderInput>,
    built_in_default: bool,
}


pub struct FrameRenderer {
    window: Window,
    _gl_context: GLContext,
    event_pump: sdl2::EventPump,
    active_shader: ActiveShader,
    vao: u32,
    start_time: Instant,
    last_shader_switch: Instant,
    shader_interval: u64,
    shader_manager: crate::manage_shader::ShaderManager,
    texture_manager: crate::manage_textures::TextureManager,
    subtitles: bool,
    subtitle_placement:
        crate::parse_subtitle_placement::SubtitlePlacement,
    subtitle_overlay:
        Option<
            crate::display_overlay::OpenGlOverlay
        >,
    overlay_output_size: (
        u32,
        u32,
    ),
    global_rendered_fps: u32,
    configured_fps: u32,
    fps_overrides:
        Vec<
            crate::load_config::FpsOverride
        >,
    fps_warning_state:
        crate::construct_text_overlay::FpsWarningState,
    fps_blink_visible: bool,
    last_fps_blink: Instant,
    frame_times: FrameTimeWindow,
    target_frame_time: Duration,
    last_frame: Instant,
    renderer_started: Instant,
}


impl FrameRenderer {
    pub fn new(
        sdl: &sdl2::Sdl,
        mut shader_manager: crate::manage_shader::ShaderManager,
        shader_interval: u64,
        global_rendered_fps: u32,
        fps_overrides:
            Vec<
                crate::load_config::FpsOverride
            >,
        texture_policy:
            crate::load_config::TextureSelectionPolicy,
        subtitles: bool,
        subtitle_placement:
            crate::parse_subtitle_placement::SubtitlePlacement,
    ) -> Result<Self, String> {
        log("[RENDER] Initializing frame renderer");

        let video = sdl.video()?;

        let event_pump = sdl
            .event_pump()
            .map_err(
                |error| {
                    format!(
                        "Failed to create SDL event pump: {error}"
                    )
                }
            )?;

        {
            let gl_attr = video.gl_attr();

            gl_attr.set_context_profile(
                GLProfile::Core
            );

            gl_attr.set_context_version(
                crate::define_constants::GL_MAJOR,
                crate::define_constants::GL_MINOR,
            );
        }

        let mut window = video
            .window(
                crate::define_constants::WINDOW_TITLE,
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
                        "Failed to create renderer window: {error}"
                    )
                }
            )?;

        let (window_width, window_height) =
            window.size();

        log(
            &format!(
                "[RENDER] Window created: {window_width}x{window_height}"
            )
        );

        let gl_context = window
            .gl_create_context()
            .map_err(
                |error| {
                    format!(
                        "Failed to create OpenGL context: {error}"
                    )
                }
            )?;

        window.raise();

        let _ = window.set_fullscreen(
            sdl2::video::FullscreenType::Desktop
        );

        gl::load_with(
            |symbol| {
                video.gl_get_proc_address(
                    symbol
                ) as *const _
            }
        );

        let _ = video.gl_set_swap_interval(
            0
        );

        let active_shader =
            select_safe_shader_program(
                &mut shader_manager
            )?;

        log_active_shader(
            &active_shader
        );

        let mut texture_manager =
            crate::manage_textures::TextureManager::new(
                texture_policy
            );

        texture_manager.prepare_for_shader(
            &active_shader.shader_name,
            active_shader.channel_usage,
        )?;

        texture_manager.configure_program(
            active_shader.program,
        );

        let safe_fps =
            if global_rendered_fps == 0 {
                crate::define_constants::DEFAULT_RENDER_FPS
            } else {
                global_rendered_fps
            };

        let configured_fps =
            resolve_shader_fps(
                safe_fps,
                &fps_overrides,
                &active_shader.shader_name,
            );

        let subtitle_overlay =
            if subtitles {

                Some(
                    build_subtitle_overlay(
                        &active_shader,
                        &texture_manager,
                        true,
                        subtitle_placement,
                        configured_fps,
                        crate::construct_text_overlay::FpsWarningState::Normal,
                        window_width,
                        window_height,
                    )?
                )

            } else {

                None
            };

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

        Ok(
            Self {
                window,
                _gl_context:
                    gl_context,
                event_pump,
                active_shader,
                vao,
                start_time:
                    Instant::now(),
                last_shader_switch:
                    Instant::now(),
                shader_interval,
                shader_manager,
                texture_manager,
                subtitles,
                subtitle_placement,
                subtitle_overlay,
                overlay_output_size:
                    (
                        window_width,
                        window_height,
                    ),
                global_rendered_fps:
                    safe_fps,
                configured_fps,
                fps_overrides,
                fps_warning_state:
                    crate::construct_text_overlay::FpsWarningState::Normal,
                fps_blink_visible:
                    true,
                last_fps_blink:
                    Instant::now(),
                frame_times:
                    FrameTimeWindow::new(),
                target_frame_time:
                    Duration::from_secs_f64(
                        1.0
                            / configured_fps as f64
                    ),
                last_frame:
                    Instant::now(),
                renderer_started:
                    Instant::now(),
            }
        )
    }


    pub fn run(
        &mut self,
        running: &AtomicBool,
    ) {
        log(
            "[RENDER] Entering renderer-owned event loop"
        );

        while running.load(Ordering::SeqCst) {
            if self.pump_events() {
                log(
                    "[RENDER] User input requested renderer exit"
                );

                break;
            }

            self.render_frame();
        }

        log(
            "[RENDER] Leaving renderer-owned event loop"
        );
    }


    pub fn render_frame(
        &mut self,
    ) {
        self.maybe_switch_shader();

        let program =
            self.active_shader.program;

        let (
            width,
            height,
        ) =
            self.window.size();

        let shader_render_start =
            Instant::now();

        unsafe {
            gl::Viewport(
                0,
                0,
                width as i32,
                height as i32,
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

            gl::UseProgram(
                program
            );

            self.texture_manager
                .bind_channels();

            crate::apply_shader_inputs::apply(
                program,
                &self.active_shader.shader_inputs,
            );

            gl::BindVertexArray(
                self.vao
            );

            let time =
                self.start_time
                    .elapsed()
                    .as_secs_f32();

            let time_location =
                gl::GetUniformLocation(
                    program,
                    b"iTime\0"
                        .as_ptr()
                        as *const _,
                );

            if time_location
                != -1
            {
                gl::Uniform1f(
                    time_location,
                    time,
                );
            }

            let resolution_location =
                gl::GetUniformLocation(
                    program,
                    b"iResolution\0"
                        .as_ptr()
                        as *const _,
                );

            if resolution_location
                != -1
            {
                gl::Uniform3f(
                    resolution_location,
                    width as f32,
                    height as f32,
                    1.0,
                );
            }

            gl::DrawArrays(
                gl::TRIANGLES,
                0,
                3,
            );

            gl::Finish();
        }

        let warning_state =
            self.frame_times.record(
                shader_render_start.elapsed(),
                self.configured_fps,
            );

        let warning_changed =
            warning_state
                != self.fps_warning_state;

        if warning_changed {
            self.fps_warning_state =
                warning_state;
            self.fps_blink_visible =
                true;
            self.last_fps_blink =
                Instant::now();
        }

        let mut blink_changed =
            false;

        if self.fps_warning_state
            == crate::construct_text_overlay::FpsWarningState::Critical
            && self.last_fps_blink.elapsed()
                >= FPS_CRITICAL_BLINK_INTERVAL
        {
            self.fps_blink_visible =
                !self.fps_blink_visible;
            self.last_fps_blink =
                Instant::now();
            blink_changed =
                true;
        }

        let overlay_warning_state =
            if self.fps_warning_state
                == crate::construct_text_overlay::FpsWarningState::Critical
                && !self.fps_blink_visible
            {
                crate::construct_text_overlay::FpsWarningState::CriticalHidden
            } else {
                self.fps_warning_state
            };

        let warning_overlay_active =
            self.fps_warning_state
                != crate::construct_text_overlay::FpsWarningState::Normal;

        let overlay_should_display =
            self.subtitles
                || warning_overlay_active;

        if overlay_should_display {

            let current_size =
                (
                    width,
                    height,
                );

            if current_size
                != self.overlay_output_size
                || warning_changed
                || blink_changed
                || self.subtitle_overlay.is_none()
            {
                match build_subtitle_overlay(
                    &self.active_shader,
                    &self.texture_manager,
                    self.subtitles,
                    self.subtitle_placement,
                    self.configured_fps,
                    overlay_warning_state,
                    width,
                    height,
                ) {

                    Ok(overlay) => {
                        self.subtitle_overlay =
                            Some(
                                overlay
                            );

                        self.overlay_output_size =
                            current_size;
                    }

                    Err(error) => {
                        log(
                            &format!(
                                "[SUBTITLE] Unable to rebuild overlay after resize: {}",
                                error,
                            )
                        );

                        self.subtitle_overlay =
                            None;
                    }
                }
            }

            if let Some(overlay) =
                self.subtitle_overlay
                    .as_ref()
            {
                overlay.display(
                    width,
                    height,
                );
            }

        } else {

            self.subtitle_overlay =
                None;
        }

        self.window.gl_swap_window();
        self.limit_fps();
    }


    fn pump_events(
        &mut self,
    ) -> bool {
        let mouse_motion_enabled =
            self.renderer_started.elapsed()
                >= INPUT_STARTUP_GRACE;

        for event in
            self.event_pump.poll_iter()
        {
            match event {
                Event::Quit {
                    ..
                } => {
                    log(
                        "[RENDER] SDL quit event received"
                    );

                    return true;
                }

                Event::Window {
                    win_event,
                    ..
                } => {
                    log(
                        &format!(
                            "[RENDER] SDL window event: {win_event:?}"
                        )
                    );
                }

                Event::KeyDown {
                    ..
                } => {
                    log(
                        "[RENDER] SDL keydown event: exiting"
                    );

                    return true;
                }

                Event::MouseButtonDown {
                    ..
                } => {
                    log(
                        "[RENDER] SDL mouse button event: exiting"
                    );

                    return true;
                }

                Event::MouseWheel {
                    ..
                } => {
                    log(
                        "[RENDER] SDL mouse wheel event: exiting"
                    );

                    return true;
                }

                Event::MouseMotion {
                    xrel,
                    yrel,
                    ..
                } => {
                    if !mouse_motion_enabled {
                        log(
                            "[RENDER] Ignoring startup mouse motion"
                        );

                        continue;
                    }

                    if xrel.abs()
                        >= MOUSE_MOTION_EXIT_THRESHOLD
                        || yrel.abs()
                            >= MOUSE_MOTION_EXIT_THRESHOLD
                    {
                        log(
                            &format!(
                                "[RENDER] SDL mouse motion event: exiting (xrel={xrel}, yrel={yrel})"
                            )
                        );

                        return true;
                    }
                }

                _ => {}
            }
        }

        false
    }

    fn maybe_switch_shader(
        &mut self,
    ) {
        if self.shader_interval == 0
            || self
                .last_shader_switch
                .elapsed()
                .as_secs()
                < self.shader_interval
        {
            return;
        }

        match select_safe_shader_program(
            &mut self.shader_manager
        ) {
            Ok(new_shader) => {
                if let Err(error) =
                    self.texture_manager.prepare_for_shader(
                        &new_shader.shader_name,
                        new_shader.channel_usage,
                    )
                {
                    unsafe {
                        if new_shader.program
                            != 0
                        {
                            gl::DeleteProgram(
                                new_shader.program
                            );
                        }
                    }

                    log(
                        &format!(
                            "[RENDER] Replacement shader texture preparation failed: {error}"
                        )
                    );

                    self.last_shader_switch =
                        Instant::now();

                    return;
                }

                self.texture_manager.configure_program(
                    new_shader.program,
                );

                let new_configured_fps =
                    resolve_shader_fps(
                        self.global_rendered_fps,
                        &self.fps_overrides,
                        &new_shader.shader_name,
                    );

                let new_overlay =
                    if self.subtitles {

                        let (
                            width,
                            height,
                        ) =
                            self.window.size();

                        match build_subtitle_overlay(
                            &new_shader,
                            &self.texture_manager,
                            true,
                            self.subtitle_placement,
                            new_configured_fps,
                            crate::construct_text_overlay::FpsWarningState::Normal,
                            width,
                            height,
                        ) {

                            Ok(overlay) => {
                                Some(
                                    overlay
                                )
                            }

                            Err(error) => {
                                log(
                                    &format!(
                                        "[SUBTITLE] Unable to construct replacement shader overlay: {}",
                                        error,
                                    )
                                );

                                None
                            }
                        }

                    } else {

                        None
                    };

                let old_program =
                    self.active_shader.program;

                self.active_shader =
                    new_shader;

                self.configured_fps =
                    new_configured_fps;

                self.target_frame_time =
                    Duration::from_secs_f64(
                        1.0
                            / new_configured_fps.max(1) as f64
                    );

                self.subtitle_overlay =
                    new_overlay;

                self.overlay_output_size =
                    self.window.size();

                self.fps_warning_state =
                    crate::construct_text_overlay::FpsWarningState::Normal;

                self.frame_times.clear();

                unsafe {
                    if old_program
                        != 0
                    {
                        gl::DeleteProgram(
                            old_program
                        );
                    }
                }

                self.last_shader_switch =
                    Instant::now();

                log_active_shader(
                    &self.active_shader
                );

                log(
                    "[RENDER] Shader switch complete"
                );
            }

            Err(error) => {
                log(
                    &format!(
                        "[RENDER] No replacement shader available: {error}"
                    )
                );

                self.last_shader_switch =
                    Instant::now();
            }
        }
    }


    fn limit_fps(
        &mut self,
    ) {
        let elapsed =
            self.last_frame.elapsed();

        if elapsed
            < self.target_frame_time
        {
            std::thread::sleep(
                self.target_frame_time
                    - elapsed
            );
        }

        self.last_frame =
            Instant::now();
    }
}


impl Drop for FrameRenderer {
    fn drop(
        &mut self,
    ) {
        self.subtitle_overlay =
            None;

        self.texture_manager
            .delete_all();

        unsafe {
            if self.active_shader.program
                != 0
            {
                gl::DeleteProgram(
                    self.active_shader.program
                );
            }

            if self.vao
                != 0
            {
                gl::DeleteVertexArrays(
                    1,
                    &self.vao,
                );
            }
        }

        log(
            "[RENDER] Frame renderer dropped"
        );
    }
}


fn resolve_shader_fps(
    global_rendered_fps: u32,
    fps_overrides:
        &[
            crate::load_config::FpsOverride
        ],
    shader_name: &str,
) -> u32 {

    fps_overrides
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
            global_rendered_fps
        )
        .max(
            1
        )
}


fn build_subtitle_overlay(
    shader: &ActiveShader,
    texture_manager:
        &crate::manage_textures::TextureManager,
    include_descriptor:
        bool,
    placement:
        crate::parse_subtitle_placement::SubtitlePlacement,
    configured_fps: u32,
    warning_state:
        crate::construct_text_overlay::FpsWarningState,
    output_width: u32,
    output_height: u32,
) -> Result<
    crate::display_overlay::OpenGlOverlay,
    String,
> {

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
                    (
                        Some(
                            specification.display_name()
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

    let descriptor =
        if include_descriptor {

            crate::construct_text_overlay::OverlayDescriptor {
                shader:
                    Some(
                        shader.shader_name
                            .clone()
                    ),
                texture,
                palette,
            }

        } else {

            crate::construct_text_overlay::OverlayDescriptor::default()
        };

    crate::display_overlay::OpenGlOverlay::new_with_fps_warning(
        &descriptor,
        configured_fps,
        warning_state,
        placement,
        output_width,
        output_height,
    )
}


fn select_safe_shader_program(
    shader_manager: &mut crate::manage_shader::ShaderManager,
) -> Result<ActiveShader, String> {
    let maximum_attempts =
        shader_manager.shader_count();

    for _ in
        0..maximum_attempts
    {
        let Some(requested_shader_name) =
            shader_manager.next()
        else {
            break;
        };

        log(
            &format!(
                "[RENDER] Evaluating shader: {requested_shader_name}"
            )
        );

        match crate::load_shader::load_shader(
            &requested_shader_name
        ) {
            crate::load_shader::ShaderLoadResult::Ready {
                source,
                shader_name,
                built_in_default,
                channel_usage,
                shader_inputs,
            } => {
                let program =
                    crate::compile_shader::build_program(
                        crate::define_constants::VERTEX_SHADER,
                        &source,
                    );

                return Ok(
                    ActiveShader {
                        program,
                        shader_name,
                        channel_usage,
                        shader_inputs,
                        built_in_default,
                    }
                );
            }

            crate::load_shader::ShaderLoadResult::Rejected {
                reasons,
                ..
            } => {
                log(
                    &format!(
                        "[RENDER] Shader rejected: {} ({})",
                        requested_shader_name,
                        reasons.join(
                            "; "
                        ),
                    )
                );

                shader_manager.remove_shader(
                    &requested_shader_name
                );
            }

            crate::load_shader::ShaderLoadResult::Unavailable {
                error,
                ..
            } => {
                log(
                    &format!(
                        "[RENDER] Shader unavailable: {} ({})",
                        requested_shader_name,
                        error,
                    )
                );

                shader_manager.remove_shader(
                    &requested_shader_name
                );
            }
        }
    }

    log(
        "[RENDER] No usable user shaders remain; loading built-in default"
    );

    match crate::load_shader::load_builtin_default_shader() {
        crate::load_shader::ShaderLoadResult::Ready {
            source,
            shader_name,
            built_in_default,
            channel_usage,
            shader_inputs,
        } => {
            let program =
                crate::compile_shader::build_program(
                    crate::define_constants::VERTEX_SHADER,
                    &source,
                );

            Ok(
                ActiveShader {
                    program,
                    shader_name,
                    channel_usage,
                    shader_inputs,
                    built_in_default,
                }
            )
        }

        crate::load_shader::ShaderLoadResult::Rejected {
            reasons,
            ..
        } => {
            Err(
                format!(
                    "Built-in default shader was rejected: {}",
                    reasons.join(
                        "; "
                    ),
                )
            )
        }

        crate::load_shader::ShaderLoadResult::Unavailable {
            error,
            ..
        } => {
            Err(
                format!(
                    "Built-in default shader is unavailable: {error}"
                )
            )
        }
    }
}


fn log_active_shader(
    shader: &ActiveShader,
) {
    let used_channels =
        shader
            .channel_usage
            .channels
            .iter()
            .enumerate()
            .filter_map(
                |(index, used)| {
                    if *used {
                        Some(
                            format!(
                                "iChannel{index}"
                            )
                        )
                    } else {
                        None
                    }
                }
            )
            .collect::<Vec<_>>();

    let channel_description =
        if used_channels.is_empty() {
            "none".to_string()
        } else {
            used_channels.join(
                ", "
            )
        };

    log(
        &format!(
            "[RENDER] Active shader: {} (built-in: {}, channels: {}, mipmaps: {})",
            shader.shader_name,
            shader.built_in_default,
            channel_description,
            shader.channel_usage.requires_mipmaps,
        )
    );
}


fn log(
    message: &str,
) {
    let logfile: PathBuf =
        crate::locate_paths::runtime_log_path();

    crate::logger::log(
        &logfile,
        message,
    );
}

