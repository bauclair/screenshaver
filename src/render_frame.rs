use std::path::PathBuf;
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::video::{GLContext, GLProfile, Window};

pub struct FrameRenderer {
    window: Window,
    _gl_context: GLContext,
    event_pump: sdl2::EventPump,
    program: u32,
    vao: u32,
    start_time: Instant,
    last_shader_switch: Instant,
    shader_interval: u64,
    shader_manager: crate::manage_shader::ShaderManager,
    target_frame_time: Duration,
    last_frame: Instant,
}

impl FrameRenderer {
    pub fn new(
        sdl: &sdl2::Sdl,
        mut shader_manager: crate::manage_shader::ShaderManager,
        shader_interval: u64,
        fps: u32,
    ) -> Result<Self, String> {
        log("[RENDER] Initializing frame renderer");

        let video = sdl.video()?;
        let event_pump = sdl
            .event_pump()
            .map_err(|e| format!("Failed to create SDL event pump: {e}"))?;

        {
            let gl_attr = video.gl_attr();
            gl_attr.set_context_profile(GLProfile::Core);
            gl_attr.set_context_version(
                crate::define_constants::GL_MAJOR,
                crate::define_constants::GL_MINOR,
            );
        }

        let mut window = video
            .window(crate::define_constants::WINDOW_TITLE, 0, 0)
            .fullscreen_desktop()
            .borderless()
            .opengl()
            .build()
            .map_err(|e| format!("Failed to create renderer window: {e}"))?;

        let (window_width, window_height) = window.size();
        log(&format!(
            "[RENDER] Window created: {window_width}x{window_height}"
        ));

        let gl_context = window
            .gl_create_context()
            .map_err(|e| format!("Failed to create OpenGL context: {e}"))?;

        window.raise();
        let _ = window.set_fullscreen(sdl2::video::FullscreenType::Desktop);

        gl::load_with(|s| video.gl_get_proc_address(s) as *const _);
        let _ = video.gl_set_swap_interval(0);

        let program = select_safe_shader_program(&mut shader_manager)?;

        let mut vao = 0_u32;
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);
        }

        let safe_fps = if fps == 0 {
            crate::define_constants::DEFAULT_RENDER_FPS
        } else {
            fps
        };

        Ok(Self {
            window,
            _gl_context: gl_context,
            event_pump,
            program,
            vao,
            start_time: Instant::now(),
            last_shader_switch: Instant::now(),
            shader_interval,
            shader_manager,
            target_frame_time: Duration::from_secs_f64(1.0 / safe_fps as f64),
            last_frame: Instant::now(),
        })
    }

    pub fn render_frame(&mut self) {
        self.pump_events();
        self.maybe_switch_shader();

        unsafe {
            let (w, h) = self.window.size();
            gl::Viewport(0, 0, w as i32, h as i32);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::UseProgram(self.program);
            gl::BindVertexArray(self.vao);

            let time = self.start_time.elapsed().as_secs_f32();
            let time_loc = gl::GetUniformLocation(
                self.program,
                b"iTime\0".as_ptr() as *const _,
            );
            if time_loc != -1 {
                gl::Uniform1f(time_loc, time);
            }

            let res_loc = gl::GetUniformLocation(
                self.program,
                b"iResolution\0".as_ptr() as *const _,
            );
            if res_loc != -1 {
                gl::Uniform3f(res_loc, w as f32, h as f32, 1.0);
            }

            gl::DrawArrays(gl::TRIANGLES, 0, 3);
        }

        self.window.gl_swap_window();
        self.limit_fps();
    }

    fn pump_events(&mut self) {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => log("[RENDER] SDL quit event received"),
                Event::Window { win_event, .. } => {
                    log(&format!("[RENDER] SDL window event: {win_event:?}"));
                }
                Event::KeyDown { .. } => log("[RENDER] SDL keydown event observed"),
                Event::MouseMotion { .. } => log("[RENDER] SDL mouse motion event observed"),
                Event::MouseButtonDown { .. } => {
                    log("[RENDER] SDL mouse button event observed");
                }
                _ => {}
            }
        }
    }

    fn maybe_switch_shader(&mut self) {
        if self.shader_interval == 0
            || self.last_shader_switch.elapsed().as_secs() < self.shader_interval
        {
            return;
        }

        match select_safe_shader_program(&mut self.shader_manager) {
            Ok(new_program) => {
                unsafe {
                    gl::DeleteProgram(self.program);
                }
                self.program = new_program;
                self.last_shader_switch = Instant::now();
                log("[RENDER] Shader switch complete");
            }
            Err(error) => {
                log(&format!(
                    "[RENDER] No replacement shader available: {error}"
                ));
                self.last_shader_switch = Instant::now();
            }
        }
    }

    fn limit_fps(&mut self) {
        let elapsed = self.last_frame.elapsed();
        if elapsed < self.target_frame_time {
            std::thread::sleep(self.target_frame_time - elapsed);
        }
        self.last_frame = Instant::now();
    }
}

impl Drop for FrameRenderer {
    fn drop(&mut self) {
        unsafe {
            if self.program != 0 {
                gl::DeleteProgram(self.program);
            }
            if self.vao != 0 {
                gl::DeleteVertexArrays(1, &self.vao);
            }
        }
        log("[RENDER] Frame renderer dropped");
    }
}

fn select_safe_shader_program(
    shader_manager: &mut crate::manage_shader::ShaderManager,
) -> Result<u32, String> {
    let maximum_attempts = shader_manager.shader_count();

    for _ in 0..maximum_attempts {
        let Some(shader_name) = shader_manager.next() else {
            break;
        };

        log(&format!("[RENDER] Evaluating shader: {shader_name}"));

        match crate::load_shader::load_shader(&shader_name) {
            crate::load_shader::ShaderLoadResult::Ready { source, .. } => {
                let program = crate::compile_shader::build_program(
                    crate::define_constants::VERTEX_SHADER,
                    &source,
                );
                return Ok(program);
            }

            crate::load_shader::ShaderLoadResult::Rejected { reasons, .. } => {
                log(&format!(
                    "[RENDER] Shader rejected: {} ({})",
                    shader_name,
                    reasons.join("; ")
                ));
                shader_manager.remove_shader(&shader_name);
            }

            crate::load_shader::ShaderLoadResult::Unavailable { error, .. } => {
                log(&format!(
                    "[RENDER] Shader unavailable: {} ({})",
                    shader_name,
                    error
                ));
                shader_manager.remove_shader(&shader_name);
            }
        }
    }

    log("[RENDER] No usable user shaders remain; loading built-in default");

    match crate::load_shader::load_builtin_default_shader() {
        crate::load_shader::ShaderLoadResult::Ready { source, .. } => Ok(
            crate::compile_shader::build_program(
                crate::define_constants::VERTEX_SHADER,
                &source,
            ),
        ),
        crate::load_shader::ShaderLoadResult::Rejected { reasons, .. } => Err(format!(
            "Built-in default shader was rejected: {}",
            reasons.join("; ")
        )),
        crate::load_shader::ShaderLoadResult::Unavailable { error, .. } => Err(format!(
            "Built-in default shader is unavailable: {error}"
        )),
    }
}

fn log(message: &str) {
    let logfile: PathBuf = crate::locate_paths::runtime_log_path();
    crate::logger::log(&logfile, message);
}
