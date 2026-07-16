//! Full-screen command-line shader preview.
//!
//! This path bypasses idle detection, shader rotation, splash
//! display, and screen locking. It renders one selected shader
//! until meaningful keyboard or mouse input is received.

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


pub fn run(
    shader_argument: String,
    shader_texture: Option<String>,
    shader_palette: Option<String>,
) -> Result<(), String> {

    let shader_path =
        resolve_shader_path(
            &shader_argument
        )?;


    let shader_name =
        shader_path
            .file_name()
            .and_then(
                |name| {
                    name.to_str()
                }
            )
            .ok_or_else(
                || {
                    format!(
                        "Unable to determine shader filename from '{}'",
                        shader_path.display(),
                    )
                }
            )?
            .to_string();


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


    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::create_log(
        &logfile
    );


    crate::logger::log(
        &logfile,
        &format!(
            "[PREVIEW_SHADER] Preview requested: {}",
            shader_path.display(),
        ),
    );


    let preview_selection =
        parse_preview_selection(
            shader_texture.as_deref(),
            shader_palette.as_deref(),
        )?;


    let loaded =
        crate::load_shader::load_shader_for_preview(
            &shader_path
        );


    let (
        source,
        channel_usage,
    ) =
        match loaded {

            crate::load_shader::ShaderLoadResult::Ready {
                source,
                channel_usage,
                ..
            } => {
                (
                    source,
                    channel_usage,
                )
            }

            crate::load_shader::ShaderLoadResult::Rejected {
                reasons,
                ..
            } => {
                return Err(
                    format!(
                        "Shader '{}' was rejected: {}",
                        shader_path.display(),
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
                        "Unable to load shader '{}': {}",
                        shader_path.display(),
                        error,
                    )
                );
            }
        };


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


    let program =
        crate::compile_shader::build_program(
            crate::define_constants::VERTEX_SHADER,
            &source,
        );


    let mut texture_manager =
        crate::manage_textures::TextureManager::new(
            config.texture_policy
        );


    texture_manager
        .prepare_for_shader_with_selection(
            &shader_name,
            channel_usage,
            preview_selection,
        )?;


    texture_manager.configure_program(
        program
    );


    let (
        selected_texture,
        selected_palette,
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


    let overlay_descriptor =
        crate::construct_text_overlay::OverlayDescriptor {
            shader:
                Some(
                    shader_name.clone()
                ),

            texture:
                selected_texture,

            palette:
                selected_palette,
        };


    let (
        initial_width,
        initial_height,
    ) =
        window.size();


    let mut overlay_output_size =
        (
            initial_width,
            initial_height,
        );


    let mut subtitle_overlay =
        if subtitles {

            Some(
                crate::display_overlay::OpenGlOverlay::new(
                    &overlay_descriptor,
                    subtitle_placement,
                    initial_width,
                    initial_height,
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


    let start_time =
        Instant::now();


    let mut previous_frame =
        Instant::now();


    let mut frame =
        0_i32;


    let mut accumulated_mouse_x =
        0_i32;


    let mut accumulated_mouse_y =
        0_i32;


    let mouse_threshold =
        8_i32;


    let fps =
        crate::define_constants::DEFAULT_RENDER_FPS
            .max(
                1
            );


    let target_frame_time =
        Duration::from_secs_f64(
            1.0
                / fps as f64
        );


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


            let frame_start =
                Instant::now();


            let (
                width,
                height,
            ) =
                window.size();


            let elapsed =
                start_time
                    .elapsed()
                    .as_secs_f32();


            let delta =
                previous_frame
                    .elapsed()
                    .as_secs_f32();


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


                texture_manager
                    .bind_channels();


                gl::BindVertexArray(
                    vao
                );


                set_uniform_1f(
                    program,
                    b"iTime\0",
                    elapsed,
                );


                set_uniform_1f(
                    program,
                    b"iTimeDelta\0",
                    delta,
                );


                set_uniform_1i(
                    program,
                    b"iFrame\0",
                    frame,
                );


                set_uniform_3f(
                    program,
                    b"iResolution\0",
                    width as f32,
                    height as f32,
                    1.0,
                );


                set_uniform_4f(
                    program,
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


            if subtitles {

                let current_size =
                    (
                        width,
                        height,
                    );


                if current_size
                    != overlay_output_size
                {
                    subtitle_overlay =
                        Some(
                            crate::display_overlay::OpenGlOverlay::new(
                                &overlay_descriptor,
                                subtitle_placement,
                                width,
                                height,
                            )?
                        );


                    overlay_output_size =
                        current_size;
                }


                if let Some(overlay) =
                    subtitle_overlay.as_ref()
                {
                    overlay.display(
                        width,
                        height,
                    );
                }
            }


            window.gl_swap_window();


            frame =
                frame.saturating_add(
                    1
                );


            previous_frame =
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


    drop(
        subtitle_overlay
    );


    texture_manager.delete_all();


    unsafe {
        if program
            != 0
        {
            gl::DeleteProgram(
                program
            );
        }


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
        gl_context
    );


    crate::logger::log(
        &logfile,
        "[PREVIEW_SHADER] Preview closed",
    );


    result
}


fn resolve_shader_path(
    shader_argument: &str,
) -> Result<PathBuf, String> {

    let supplied_path =
        PathBuf::from(
            shader_argument
        );


    let resolved =
        if has_explicit_path(
            &supplied_path
        ) {
            supplied_path
        } else {
            crate::locate_paths::shader_dir()
                .join(
                    supplied_path
                )
        };


    if !resolved.is_file() {
        return Err(
            format!(
                "Shader file not found: {}",
                resolved.display(),
            )
        );
    }


    Ok(
        resolved
    )
}


fn has_explicit_path(
    path: &Path,
) -> bool {

    path.is_absolute()
        || path.components()
            .count()
            > 1
}


fn parse_preview_selection(
    texture_name: Option<&str>,
    palette_name: Option<&str>,
) -> Result<
    crate::manage_textures::PreviewTextureSelection,
    String,
> {

    let texture =
        match texture_name {

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
                let family =
                    crate::generate_textures::TextureFamily::from_name(
                        name
                    )?;


                if family
                    == crate::generate_textures::TextureFamily::Julia
                {
                    return Err(
                        "Julia texture generation is not yet implemented"
                            .to_string()
                    );
                }


                Some(
                    crate::manage_textures::PreviewSelectionValue::Specific(
                        family
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

