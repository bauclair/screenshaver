use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::video::{GLContext, GLProfile, Window};

use crate::fps_monitor::{
    FpsWarningState,
    FrameTimeWindow,
    FPS_CRITICAL_BLINK_INTERVAL,
};


const INPUT_STARTUP_GRACE: Duration = Duration::from_millis(750);
const MOUSE_MOTION_EXIT_THRESHOLD: i32 = 4;

#[derive(Debug)]
struct ActiveShader {
    program: u32,
    shader_name: String,
    channel_usage: crate::preprocess_shader::ShaderChannelUsage,
    shader_inputs: Vec<crate::isf_types::ShaderInput>,
    built_in_default: bool,
}


pub struct FrameRenderer {
    // Keep the engine first so its OpenGL resources are released before the
    // context and window are destroyed.
    engine: FrameRenderEngine,
    event_pump: sdl2::EventPump,
    renderer_started: Instant,
    _gl_context: GLContext,
    window: Window,
}




#[derive(Clone)]
enum RenderFpsPolicy {
    Screensaver {
        global_rendered_fps: u32,
        fps_overrides: Vec<crate::load_config::FpsOverride>,
    },
    Wallpaper(
        crate::load_config::FpsSelectionPolicy,
    ),
}

impl RenderFpsPolicy {
    fn rendered_fps_for_shader(
        &self,
        shader_name: &str,
    ) -> u32 {
        match self {
            Self::Screensaver {
                global_rendered_fps,
                fps_overrides,
            } => {
                resolve_shader_fps(
                    (*global_rendered_fps).max(1),
                    fps_overrides,
                    shader_name,
                )
            }
            Self::Wallpaper(policy) => {
                policy.rendered_fps_for_shader(
                    shader_name,
                    None,
                )
            }
        }
    }
}

pub(crate) struct FrameRenderEngine {
    active_shader: ActiveShader,
    vao: u32,
    start_time: Instant,
    animation_speed: f32,
    animation_speed_policy:
        crate::load_config::AnimationSpeedPolicy,
    last_shader_switch: Instant,
    shader_interval: u64,
    shader_manager: crate::manage_shader::ShaderManager,
    texture_manager: crate::manage_textures::TextureManager,
    subtitles: bool,
    subtitle_placement:
        crate::parse_subtitle_placement::SubtitlePlacement,
    subtitle_overlay:
        Option<crate::display_overlay::OpenGlOverlay>,
    overlay_output_size: (u32, u32),
    fps_policy: RenderFpsPolicy,
    configured_fps: u32,
    fps_warning_state: FpsWarningState,
    fps_blink_visible: bool,
    last_fps_blink: Instant,
    frame_times: FrameTimeWindow,
    target_frame_time: Duration,
    last_frame: Instant,
}




impl FrameRenderEngine {
    pub(crate) fn new(
        shader_manager: crate::manage_shader::ShaderManager,
        shader_interval: u64,
        animation_speed_policy:
            crate::load_config::AnimationSpeedPolicy,
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
        output_width: u32,
        output_height: u32,
    ) -> Result<Self, String> {
        Self::new_with_fps_policy(
            shader_manager,
            shader_interval,
            animation_speed_policy,
            RenderFpsPolicy::Screensaver {
                global_rendered_fps,
                fps_overrides,
            },
            texture_policy,
            subtitles,
            subtitle_placement,
            output_width,
            output_height,
        )
    }


    pub(crate) fn new_for_wallpaper(
        shader_manager: crate::manage_shader::ShaderManager,
        shader_interval: u64,
        animation_speed_policy:
            crate::load_config::AnimationSpeedPolicy,
        fps_policy:
            crate::load_config::FpsSelectionPolicy,
        texture_policy:
            crate::load_config::TextureSelectionPolicy,
        subtitles: bool,
        subtitle_placement:
            crate::parse_subtitle_placement::SubtitlePlacement,
        output_width: u32,
        output_height: u32,
    ) -> Result<Self, String> {
        Self::new_with_fps_policy(
            shader_manager,
            shader_interval,
            animation_speed_policy,
            RenderFpsPolicy::Wallpaper(
                fps_policy
            ),
            texture_policy,
            subtitles,
            subtitle_placement,
            output_width,
            output_height,
        )
    }


    fn new_with_fps_policy(
        mut shader_manager: crate::manage_shader::ShaderManager,
        shader_interval: u64,
        animation_speed_policy:
            crate::load_config::AnimationSpeedPolicy,
        fps_policy: RenderFpsPolicy,
        texture_policy:
            crate::load_config::TextureSelectionPolicy,
        subtitles: bool,
        subtitle_placement:
            crate::parse_subtitle_placement::SubtitlePlacement,
        output_width: u32,
        output_height: u32,
    ) -> Result<Self, String> {
        log_information(
            "[RENDER] Initializing shared frame render engine"
        );

        let active_shader =
            select_safe_shader_program(
                &mut shader_manager
            )?;

        let animation_speed =
            animation_speed_policy.animation_speed_for_shader(
                &active_shader.shader_name,
                None,
            );

        log_information(
            &format!(
                "[RENDER] Animation speed: {:.3}x",
                animation_speed,
            )
        );

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

        let configured_fps =
            fps_policy.rendered_fps_for_shader(
                &active_shader.shader_name
            );

        let subtitle_overlay =
            if subtitles {

                Some(
                    build_subtitle_overlay(
                        &active_shader,
                        &texture_manager,
                        true,
                        subtitle_placement,
                        animation_speed,
                        configured_fps,
                        FpsWarningState::Normal,
                        output_width,
                        output_height,
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
                active_shader,
                vao,
                start_time:
                    Instant::now(),
                animation_speed,
                animation_speed_policy,
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
                        output_width,
                        output_height,
                    ),
                fps_policy,
                configured_fps,
                fps_warning_state:
                    FpsWarningState::Normal,
                fps_blink_visible:
                    true,
                last_fps_blink:
                    Instant::now(),
                frame_times:
                    FrameTimeWindow::new(),
                target_frame_time:
                    Duration::from_secs_f64(
                        1.0
                            / configured_fps.max(1) as f64
                    ),
                last_frame:
                    Instant::now(),
            }
        )
    }


    pub(crate) fn limit_fps(
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


    fn render_scene(
        &mut self,
        width: u32,
        height: u32,
    ) -> FpsWarningState {
        let program =
            self.active_shader.program;

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
                    .as_secs_f32()
                    * self.animation_speed;

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

        self.frame_times.record(
            shader_render_start.elapsed(),
            self.configured_fps,
        ).warning_state
    }


    pub(crate) fn render_frame(
        &mut self,
        width: u32,
        height: u32,
    ) {
        self.maybe_switch_shader(
            width,
            height,
        );

        let warning_state =
            self.render_scene(
                width,
                height,
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
            == FpsWarningState::Critical
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
                == FpsWarningState::Critical
                && !self.fps_blink_visible
            {
                FpsWarningState::CriticalHidden
            } else {
                self.fps_warning_state
            };

        let warning_overlay_active =
            self.fps_warning_state
                != FpsWarningState::Normal;

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
                    self.animation_speed,
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
                        log_warning(
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
    }


    fn maybe_switch_shader(
        &mut self,
        width: u32,
        height: u32,
    ) {
        if self.shader_interval == 0
            || self.last_shader_switch
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

                    log_warning(
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

                let new_animation_speed =
                    self.animation_speed_policy
                        .animation_speed_for_shader(
                            &new_shader.shader_name,
                            None,
                        );


                let new_configured_fps =
                    self.fps_policy
                        .rendered_fps_for_shader(
                            &new_shader.shader_name
                        );

                let new_overlay =
                    if self.subtitles {

                        match build_subtitle_overlay(
                            &new_shader,
                            &self.texture_manager,
                            true,
                            self.subtitle_placement,
                            new_animation_speed,
                            new_configured_fps,
                            FpsWarningState::Normal,
                            width,
                            height,
                        ) {

                            Ok(overlay) => {
                                Some(
                                    overlay
                                )
                            }

                            Err(error) => {
                                log_warning(
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

                self.start_time =
                    Instant::now();

                self.animation_speed =
                    new_animation_speed;

                log_information(
                    &format!(
                        "[RENDER] Animation speed: {:.3}x",
                        self.animation_speed,
                    )
                );

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
                    (
                        width,
                        height,
                    );

                self.fps_warning_state =
                    FpsWarningState::Normal;

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

                log_information(
                    "[RENDER] Shader switch complete"
                );
            }

            Err(error) => {
                log_warning(
                    &format!(
                        "[RENDER] No replacement shader available: {error}"
                    )
                );

                self.last_shader_switch =
                    Instant::now();
            }
        }
    }
}

impl FrameRenderer {
    pub fn new(
        sdl: &sdl2::Sdl,
        shader_manager: crate::manage_shader::ShaderManager,
        shader_interval: u64,
        animation_speed_policy:
            crate::load_config::AnimationSpeedPolicy,
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
        log_information("[RENDER] Initializing frame renderer");

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

        log_information(
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

        let engine =
            FrameRenderEngine::new(
                shader_manager,
                shader_interval,
                animation_speed_policy,
                global_rendered_fps,
                fps_overrides,
                texture_policy,
                subtitles,
                subtitle_placement,
                window_width,
                window_height,
            )?;

        Ok(
            Self {
                engine,
                event_pump,
                renderer_started:
                    Instant::now(),
                _gl_context:
                    gl_context,
                window,
            }
        )
    }


    pub fn run(
        &mut self,
        running: &AtomicBool,
        wallpaper_control: &crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
    ) {
        log_information(
            "[RENDER] Entering renderer-owned event loop"
        );

        let mut first_frame_presented =
            false;


        while running.load(Ordering::SeqCst) {
            if self.pump_events() {
                log_information(
                    "[RENDER] User input requested renderer exit"
                );


                wallpaper_control.resume_and_wait_for_frame(
                    running
                );


                break;
            }


            self.render_frame();


            if !first_frame_presented {
                first_frame_presented =
                    true;


                wallpaper_control.request_pause_after_first_frame(
                    running
                );
            }
        }

        log_information(
            "[RENDER] Leaving renderer-owned event loop"
        );
    }


    pub fn render_frame(
        &mut self,
    ) {
        let (
            width,
            height,
        ) =
            self.window.drawable_size();

        self.engine.render_frame(
            width,
            height,
        );

        self.window.gl_swap_window();
        self.engine.limit_fps();
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
                    log_information(
                        "[RENDER] SDL quit event received"
                    );

                    return true;
                }

                Event::Window {
                    win_event,
                    ..
                } => {
                    log_debug(
                        &format!(
                            "[RENDER] SDL window event: {win_event:?}"
                        )
                    );
                }

                Event::KeyDown {
                    ..
                } => {
                    log_information(
                        "[RENDER] SDL keydown event: exiting"
                    );

                    return true;
                }

                Event::MouseButtonDown {
                    ..
                } => {
                    log_information(
                        "[RENDER] SDL mouse button event: exiting"
                    );

                    return true;
                }

                Event::MouseWheel {
                    ..
                } => {
                    log_information(
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
                        log_debug(
                            "[RENDER] Ignoring startup mouse motion"
                        );

                        continue;
                    }

                    if xrel.abs()
                        >= MOUSE_MOTION_EXIT_THRESHOLD
                        || yrel.abs()
                            >= MOUSE_MOTION_EXIT_THRESHOLD
                    {
                        log_information(
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


}


impl Drop for FrameRenderEngine {
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

        log_debug(
            "[RENDER] Frame render engine dropped"
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


fn format_animation_speed(
    speed: f32,
) -> String {

    if speed.fract() == 0.0 {
        format!(
            "×{speed:.1}"
        )
    } else {
        format!(
            "×{speed}"
        )
    }
}


fn build_subtitle_overlay(
    shader: &ActiveShader,
    texture_manager:
        &crate::manage_textures::TextureManager,
    include_descriptor:
        bool,
    placement:
        crate::parse_subtitle_placement::SubtitlePlacement,
    animation_speed: f32,
    configured_fps: u32,
    warning_state:
        FpsWarningState,
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
                            if specification.count_was_explicit {
                                specification.display_name()
                            } else {
                                format!(
                                    "{} ({})",
                                    specification.display_name(),
                                    specification.requested_primitive_count,
                                )
                            }
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

            let shader_file_name =
                Path::new(
                    &shader.shader_name
                )
                .file_name()
                .and_then(
                    |name| name.to_str()
                );

            let shader_label =
                if shader.built_in_default
                    || shader_file_name
                        .is_some_and(
                            |name| {
                                name.eq_ignore_ascii_case(
                                    "default.glsl"
                                )
                            }
                        )
                {
                    "Collect more shaders at https://editor.isf.video/shaders and https://shadertoy.com/browse"
                        .to_string()
                } else {
                    shader.shader_name
                        .clone()
                };

            let shader_label =
                format!(
                    "{} | {}",
                    shader_label,
                    format_animation_speed(
                        animation_speed
                    ),
                );

            crate::construct_text_overlay::OverlayDescriptor {
                shader:
                    Some(
                        shader_label
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

        log_debug(
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
                match crate::compile_shader::build_program(
                    crate::define_constants::VERTEX_SHADER,
                    &source,
                ) {

                    Ok(program) => {

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

                    Err(error) => {

                        log_warning(
                            &format!(
                                "[RENDER] Shader compilation failed: {} ({})",
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

            crate::load_shader::ShaderLoadResult::Rejected {
                reasons,
                ..
            } => {
                log_warning(
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
                log_warning(
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

    log_warning(
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
                )
                .map_err(
                    |error| {
                        format!(
                            "Built-in default shader compilation failed: {}",
                            error,
                        )
                    }
                )?;

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

    log_information(
        &format!(
            "[RENDER] Active shader: {} (built-in: {}, channels: {}, mipmaps: {})",
            shader.shader_name,
            shader.built_in_default,
            channel_description,
            shader.channel_usage.requires_mipmaps,
        )
    );
}


fn log_debug(
    message: &str,
) {
    let logfile: PathBuf =
        crate::locate_paths::runtime_log_path();

    crate::logger::debug(
        &logfile,
        message,
    );
}


fn log_information(
    message: &str,
) {
    let logfile: PathBuf =
        crate::locate_paths::runtime_log_path();

    crate::logger::information(
        &logfile,
        message,
    );
}


fn log_warning(
    message: &str,
) {
    let logfile: PathBuf =
        crate::locate_paths::runtime_log_path();

    crate::logger::warning(
        &logfile,
        message,
    );
}

