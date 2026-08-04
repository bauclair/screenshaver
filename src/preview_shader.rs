//! Full-screen command-line shader preview.
//!
//! Supports one shader file or an ordered playlist supplied by the
//! directory-preview module. The OpenGL window remains active while
//! shaders rotate at the requested interval.

use std::collections::VecDeque;
use std::path::{
    Path,
    PathBuf,
};
use std::time::{
    Duration,
    Instant,
};

use sdl2::event::Event;
use sdl2::video::{
    FullscreenType,
    GLProfile,
};
use crate::parse_texture_specification::TextureSpecification;


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
    shader_argument: String,
    shader_texture: Option<TextureSpecification>,
    shader_palette: Option<String>,
    interval_seconds: Option<u64>,
    command_line_fps: Option<u32>,
    animation_speed: Option<f32>,
) -> Result<(), String> {

    match crate::preview_shader_directory::resolve_preview_target(
        &shader_argument
    )? {

        crate::preview_shader_directory::PreviewTarget::File(
            path
        ) => {

            if interval_seconds.is_some() {
                log_warning(
                    "[PREVIEW_SHADER] --interval ignored for a single shader file"
                );
            }


            run_paths(
                vec![
                    path
                ],
                shader_texture,
                shader_palette,
                None,
                command_line_fps,
                animation_speed,
            )
        }

        crate::preview_shader_directory::PreviewTarget::Directory(
            path
        ) => {

            crate::preview_shader_directory::run(
                path.display()
                    .to_string(),
                shader_texture,
                shader_palette,
                interval_seconds,
                command_line_fps,
                animation_speed,
            )
        }
    }
}


pub fn run_paths(
    shader_paths: Vec<PathBuf>,
    shader_texture: Option<TextureSpecification>,
    shader_palette: Option<String>,
    interval_seconds: Option<u64>,
    command_line_fps: Option<u32>,
    animation_speed: Option<f32>,
) -> Result<(), String> {

    if shader_paths.is_empty() {
        return Err(
            "No shader paths were supplied for preview"
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


    let fps_overrides =
        config.screensaver_fps_overrides;


    let texture_policy =
        config.texture_policy;


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
            "[PREVIEW_SHADER] Preview playlist contains {} shader path(s)",
            shader_paths.len(),
        ),
    );


    crate::logger::information(
        &logfile,
        &format!(
            "[PREVIEW_SHADER] Animation speed: {:.3}x",
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
                "Screenshaver Shader Preview",
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
                        "Unable to create shader preview window: {}",
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


    let (
        width,
        height,
    ) =
        window.size();


    let mut postprocess =
        crate::postprocess_shader::PostprocessPipeline::new(
            width,
            height,
        )?;


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
            &fps_overrides,
            command_line_fps,
            animation_speed,
            width,
            height,
        )?;


    let mut configured_fps =
        resolve_preview_fps(
            global_rendered_fps,
            &fps_overrides,
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


    let mut accumulated_mouse_x =
        0_i32;


    let mut accumulated_mouse_y =
        0_i32;


    let mouse_threshold =
        8_i32;


    let mut last_switch =
        Instant::now();


    let result =
        'preview: loop {

            for event in
                event_pump.poll_iter()
            {
                match event {

                    Event::Quit {
                        ..
                    }
                    | Event::KeyDown {
                        ..
                    }
                    | Event::MouseButtonDown {
                        ..
                    } => {
                        break 'preview Ok(());
                    }

                    Event::MouseMotion {
                        xrel,
                        yrel,
                        ..
                    } => {
                        accumulated_mouse_x +=
                            xrel.abs();

                        accumulated_mouse_y +=
                            yrel.abs();


                        if accumulated_mouse_x
                            >= mouse_threshold
                            || accumulated_mouse_y
                                >= mouse_threshold
                        {
                            break 'preview Ok(());
                        }
                    }

                    _ => {}
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
                        &fps_overrides,
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


                            configured_fps =
                                resolve_preview_fps(
                                    global_rendered_fps,
                                    &fps_overrides,
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
                                    "[PREVIEW_SHADER] Switched to {}",
                                    active.path.display(),
                                )
                            );
                        }

                        Err(error) => {

                            log_warning(
                                &format!(
                                    "[PREVIEW_SHADER] Unable to select another usable shader: {}",
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
                    width as f32,
                    height as f32,
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


    drop(
        gl_context
    );


    log_information(
        "[PREVIEW_SHADER] Preview closed"
    );


    result
}


fn resolve_preview_fps(
    global_rendered_fps: u32,
    fps_overrides:
        &[
            crate::load_config::FpsOverride
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


fn load_first_usable_shader(
    paths: &[PathBuf],
    start_index: usize,
    texture_policy:
        &crate::load_config::TextureSelectionPolicy,
    preview_selection:
        crate::manage_textures::PreviewTextureSelection,
    subtitles: bool,
    subtitle_placement:
        crate::parse_subtitle_placement::SubtitlePlacement,
    global_rendered_fps: u32,
    fps_overrides:
        &[
            crate::load_config::FpsOverride
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
                fps_overrides,
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
                        "[PREVIEW_SHADER] Skipping '{}': {}",
                        path.display(),
                        error,
                    )
                );
            }
        }
    }


    Err(
        "No usable shaders were found in the preview playlist"
            .to_string()
    )
}


fn load_active_shader(
    path: &Path,
    texture_policy:
        &crate::load_config::TextureSelectionPolicy,
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
            "[PREVIEW_SHADER] Active shader: {}",
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

