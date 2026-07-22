use std::ffi::CString;


pub fn compile_shader(
    source: &str,
    kind: u32,
) -> Result<u32, String> {

    unsafe {
        let shader =
            gl::CreateShader(
                kind
            );


        if shader
            == 0
        {
            return Err(
                format!(
                    "Unable to create OpenGL {} shader object",
                    shader_kind_name(
                        kind
                    ),
                )
            );
        }


        let c_source =
            match CString::new(
                source
            ) {

                Ok(source) => {
                    source
                }

                Err(_) => {

                    gl::DeleteShader(
                        shader
                    );


                    return Err(
                        format!(
                            "{} shader source contained an interior null byte",
                            shader_kind_display_name(
                                kind
                            ),
                        )
                    );
                }
            };


        gl::ShaderSource(
            shader,
            1,
            &c_source.as_ptr(),
            std::ptr::null(),
        );


        gl::CompileShader(
            shader
        );


        if !shader_compile_success(
            shader
        ) {
            let error =
                shader_info_log(
                    shader
                );


            gl::DeleteShader(
                shader
            );


            return Err(
                format_shader_failure(
                    kind,
                    &error,
                )
            );
        }


        Ok(
            shader
        )
    }
}


pub fn link_program(
    vertex_shader: u32,
    fragment_shader: u32,
) -> Result<u32, String> {

    unsafe {
        let program =
            gl::CreateProgram();


        if program
            == 0
        {
            return Err(
                "Unable to create OpenGL shader program object"
                    .to_string()
            );
        }


        gl::AttachShader(
            program,
            vertex_shader,
        );


        gl::AttachShader(
            program,
            fragment_shader,
        );


        gl::LinkProgram(
            program
        );


        if !program_link_success(
            program
        ) {
            let error =
                program_info_log(
                    program
                );


            gl::DeleteProgram(
                program
            );


            return Err(
                if error.is_empty() {

                    "Shader program linking failed without an OpenGL diagnostic"
                        .to_string()

                } else {

                    format!(
                        "Shader program linking failed:\n{}",
                        error,
                    )
                }
            );
        }


        Ok(
            program
        )
    }
}


pub fn build_program(
    vertex_source: &str,
    fragment_source: &str,
) -> Result<u32, String> {

    let vertex_shader =
        compile_shader(
            vertex_source,
            gl::VERTEX_SHADER,
        )?;


    let fragment_shader =
        match compile_shader(
            fragment_source,
            gl::FRAGMENT_SHADER,
        ) {

            Ok(shader) => {
                shader
            }

            Err(error) => {

                unsafe {
                    gl::DeleteShader(
                        vertex_shader
                    );
                }


                return Err(
                    error
                );
            }
        };


    let program =
        link_program(
            vertex_shader,
            fragment_shader,
        );


    unsafe {
        gl::DeleteShader(
            vertex_shader
        );


        gl::DeleteShader(
            fragment_shader
        );
    }


    program
}


fn shader_kind_name(
    kind: u32,
) -> &'static str {

    match kind {

        gl::VERTEX_SHADER => {
            "vertex"
        }

        gl::FRAGMENT_SHADER => {
            "fragment"
        }

        _ => {
            "unknown"
        }
    }
}


fn shader_kind_display_name(
    kind: u32,
) -> &'static str {

    match kind {

        gl::VERTEX_SHADER => {
            "Vertex"
        }

        gl::FRAGMENT_SHADER => {
            "Fragment"
        }

        _ => {
            "Unknown"
        }
    }
}


fn format_shader_failure(
    kind: u32,
    error: &str,
) -> String {

    if error.is_empty() {

        format!(
            "{} shader compilation failed without an OpenGL diagnostic",
            shader_kind_display_name(
                kind
            ),
        )

    } else {

        format!(
            "{} shader compilation failed:\n{}",
            shader_kind_display_name(
                kind
            ),
            error,
        )
    }
}


fn shader_compile_success(
    shader: u32,
) -> bool {

    unsafe {
        let mut success: i32 =
            0;


        gl::GetShaderiv(
            shader,
            gl::COMPILE_STATUS,
            &mut success,
        );


        success
            != 0
    }
}


fn program_link_success(
    program: u32,
) -> bool {

    unsafe {
        let mut success: i32 =
            0;


        gl::GetProgramiv(
            program,
            gl::LINK_STATUS,
            &mut success,
        );


        success
            != 0
    }
}


fn shader_info_log(
    shader: u32,
) -> String {

    unsafe {
        let mut len: i32 =
            0;


        gl::GetShaderiv(
            shader,
            gl::INFO_LOG_LENGTH,
            &mut len,
        );


        if len
            <= 0
        {
            return String::new();
        }


        let mut buffer =
            vec![
                0u8;
                len as usize
            ];


        gl::GetShaderInfoLog(
            shader,
            len,
            std::ptr::null_mut(),
            buffer.as_mut_ptr() as *mut _,
        );


        String::from_utf8_lossy(
            &buffer
        )
            .trim_end_matches(
                '\0'
            )
            .to_string()
    }
}


fn program_info_log(
    program: u32,
) -> String {

    unsafe {
        let mut len: i32 =
            0;


        gl::GetProgramiv(
            program,
            gl::INFO_LOG_LENGTH,
            &mut len,
        );


        if len
            <= 0
        {
            return String::new();
        }


        let mut buffer =
            vec![
                0u8;
                len as usize
            ];


        gl::GetProgramInfoLog(
            program,
            len,
            std::ptr::null_mut(),
            buffer.as_mut_ptr() as *mut _,
        );


        String::from_utf8_lossy(
            &buffer
        )
            .trim_end_matches(
                '\0'
            )
            .to_string()
    }
}

