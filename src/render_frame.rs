use std::path::PathBuf;
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::video::{GLContext, GLProfile, Window};


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
    target_frame_time: Duration,
    last_frame: Instant,
}


impl FrameRenderer {
    pub fn new(
        sdl: &sdl2::Sdl,
        mut shader_manager: crate::manage_shader::ShaderManager,
        shader_interval: u64,
        fps: u32,
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

        let subtitle_overlay =
            if subtitles {

                Some(
                    build_subtitle_overlay(
                        &active_shader,
                        &texture_manager,
                        subtitle_placement,
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

        let safe_fps =
            if fps == 0 {
                crate::define_constants::DEFAULT_RENDER_FPS
            } else {
                fps
            };

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
                target_frame_time:
                    Duration::from_secs_f64(
                        1.0
                            / safe_fps as f64
                    ),
                last_frame:
                    Instant::now(),
            }
        )
    }


    pub fn render_frame(
        &mut self,
    ) {
        self.pump_events();
        self.maybe_switch_shader();

        let program =
            self.active_shader.program;

        let (
            width,
            height,
        ) =
            self.window.size();

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
        }

        if self.subtitles {

            let current_size =
                (
                    width,
                    height,
                );

            if current_size
                != self.overlay_output_size
            {
                match build_subtitle_overlay(
                    &self.active_shader,
                    &self.texture_manager,
                    self.subtitle_placement,
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
        }

        self.window.gl_swap_window();
        self.limit_fps();
    }


    fn pump_events(
        &mut self,
    ) {
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
                        "[RENDER] SDL keydown event observed"
                    );
                }

                Event::MouseMotion {
                    ..
                } => {
                    log(
                        "[RENDER] SDL mouse motion event observed"
                    );
                }

                Event::MouseButtonDown {
                    ..
                } => {
                    log(
                        "[RENDER] SDL mouse button event observed"
                    );
                }

                _ => {}
            }
        }
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
                            self.subtitle_placement,
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

                self.subtitle_overlay =
                    new_overlay;

                self.overlay_output_size =
                    self.window.size();

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


fn build_subtitle_overlay(
    shader: &ActiveShader,
    texture_manager:
        &crate::manage_textures::TextureManager,
    placement:
        crate::parse_subtitle_placement::SubtitlePlacement,
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
            .active_selection()
            .map(
                |(
                    family,
                    palette,
                )| {
                    (
                        Some(
                            family.to_string()
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
        crate::construct_text_overlay::OverlayDescriptor {
            shader:
                Some(
                    shader.shader_name
                        .clone()
                ),
            texture,
            palette,
        };

    crate::display_overlay::OpenGlOverlay::new(
        &descriptor,
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

