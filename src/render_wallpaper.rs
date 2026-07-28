use sdl2::video::GLProfile;


pub fn validate_shader_program(
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
                "Screenshaver Wallpaper Shader Validation",
                1,
                1,
            )
            .hidden()
            .opengl()
            .build()
            .map_err(
                |error| {
                    format!(
                        "Unable to create hidden wallpaper validation window: {}",
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
                        "Unable to create wallpaper validation OpenGL context: {}",
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


    let program =
        crate::compile_shader::build_program(
            crate::define_constants::VERTEX_SHADER,
            fragment_source,
        )?;


    unsafe {
        gl::DeleteProgram(
            program
        );
    }


    Ok(())
}

