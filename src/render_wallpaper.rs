use std::ffi::CString;
use std::time::Instant;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;


pub fn run_test_window(
    fragment_source: &str,
) -> Result<(), String> {

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


    let window =
        video
            .window(
                "Screenshaver Wallpaper Rendering Test",
                1280,
                720,
            )
            .position_centered()
            .resizable()
            .opengl()
            .build()
            .map_err(
                |error| {
                    format!(
                        "Unable to create wallpaper rendering test window: {}",
                        error,
                    )
                }
            )?;


    let _gl_context =
        window
            .gl_create_context()
            .map_err(
                |error| {
                    format!(
                        "Unable to create wallpaper test OpenGL context: {}",
                        error,
                    )
                }
            )?;


    gl::load_with(
        |symbol| {
            video.gl_get_proc_address(
                symbol
            ) as *const _
        }
    );


    let _ =
        video.gl_set_swap_interval(
            1
        );


    let program =
        crate::compile_shader::build_program(
            crate::define_constants::VERTEX_SHADER,
            fragment_source,
        )?;


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


        gl::UseProgram(
            program
        );
    }


    let i_time =
        uniform_location(
            program,
            "iTime",
        )?;


    let i_resolution =
        uniform_location(
            program,
            "iResolution",
        )?;


    let i_mouse =
        uniform_location(
            program,
            "iMouse",
        )?;


    let i_frame =
        uniform_location(
            program,
            "iFrame",
        )?;


    let mut event_pump =
        sdl.event_pump()
            .map_err(
                |error| {
                    format!(
                        "Unable to create wallpaper test event pump: {}",
                        error,
                    )
                }
            )?;


    let start_time =
        Instant::now();


    let mut frame =
        0_i32;


    let mut mouse_x =
        0.0_f32;


    let mut mouse_y =
        0.0_f32;


    let result =
        'render: loop {

            for event in
                event_pump.poll_iter()
            {
                match event {

                    Event::Quit {
                        ..
                    }
                    | Event::KeyDown {
                        keycode:
                            Some(
                                Keycode::Escape
                            ),
                        ..
                    } => {
                        break 'render Ok(());
                    }


                    Event::MouseMotion {
                        x,
                        y,
                        ..
                    } => {
                        mouse_x =
                            x as f32;


                        mouse_y =
                            y as f32;
                    }


                    _ => {}
                }
            }


            let (
                drawable_width,
                drawable_height,
            ) =
                window.drawable_size();


            if drawable_width == 0
                || drawable_height == 0
            {
                continue;
            }


            let elapsed =
                start_time.elapsed()
                    .as_secs_f32();


            let shader_mouse_y =
                drawable_height as f32
                    - mouse_y;


            unsafe {
                gl::Viewport(
                    0,
                    0,
                    drawable_width as i32,
                    drawable_height as i32,
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


                set_uniform_1f(
                    i_time,
                    elapsed,
                );


                set_uniform_3f(
                    i_resolution,
                    drawable_width as f32,
                    drawable_height as f32,
                    1.0,
                );


                set_uniform_4f(
                    i_mouse,
                    mouse_x,
                    shader_mouse_y,
                    0.0,
                    0.0,
                );


                set_uniform_1i(
                    i_frame,
                    frame,
                );


                gl::BindVertexArray(
                    vao
                );


                gl::DrawArrays(
                    gl::TRIANGLES,
                    0,
                    3,
                );
            }


            window.gl_swap_window();


            frame =
                frame.saturating_add(
                    1
                );
        };


    unsafe {
        gl::UseProgram(
            0
        );


        gl::BindVertexArray(
            0
        );


        gl::DeleteVertexArrays(
            1,
            &vao,
        );


        gl::DeleteProgram(
            program
        );
    }


    result
}


fn uniform_location(
    program: u32,
    name: &str,
) -> Result<i32, String> {

    let c_name =
        CString::new(
            name
        )
        .map_err(
            |_| {
                format!(
                    "Uniform name contains an interior null byte: {}",
                    name,
                )
            }
        )?;


    let location =
        unsafe {
            gl::GetUniformLocation(
                program,
                c_name.as_ptr(),
            )
        };


    Ok(
        location
    )
}


fn set_uniform_1f(
    location: i32,
    value: f32,
) {

    if location >= 0 {
        unsafe {
            gl::Uniform1f(
                location,
                value,
            );
        }
    }
}


fn set_uniform_1i(
    location: i32,
    value: i32,
) {

    if location >= 0 {
        unsafe {
            gl::Uniform1i(
                location,
                value,
            );
        }
    }
}


fn set_uniform_3f(
    location: i32,
    x: f32,
    y: f32,
    z: f32,
) {

    if location >= 0 {
        unsafe {
            gl::Uniform3f(
                location,
                x,
                y,
                z,
            );
        }
    }
}


fn set_uniform_4f(
    location: i32,
    x: f32,
    y: f32,
    z: f32,
    w: f32,
) {

    if location >= 0 {
        unsafe {
            gl::Uniform4f(
                location,
                x,
                y,
                z,
                w,
            );
        }
    }
}

