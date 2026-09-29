use std::ffi::CString;


type TextFormatter =
    fn(
        key: &str,
        params: &[(&str, &str)],
    ) -> String;

static TEXT_FORMATTER:
    std::sync::OnceLock<TextFormatter> =
    std::sync::OnceLock::new();


pub fn set_text_formatter(
    formatter: TextFormatter,
) -> Result<(), &'static str> {

    TEXT_FORMATTER
        .set(
            formatter
        )
        .map_err(
            |_| "compile-shader text formatter is already initialized"
        )
}


fn runtime_text(
    key: &str,
) -> String {

    format_text(
        key,
        &[],
    )
}


fn runtime_text_with_params(
    key: &str,
    params: &[(&str, &str)],
) -> String {

    format_text(
        key,
        params,
    )
}


fn format_text(
    key: &str,
    params: &[(&str, &str)],
) -> String {

    if let Some(formatter) =
        TEXT_FORMATTER.get()
    {
        return formatter(
            key,
            params,
        );
    }

    default_text(
        key,
        params,
    )
}


fn default_text(
    key: &str,
    params: &[(&str, &str)],
) -> String {

    let template =
        match key {
            "compile_shader.kind.vertex" =>
                "Vertex",
            "compile_shader.kind.fragment" =>
                "Fragment",
            "compile_shader.kind.unknown" =>
                "Unknown",
            "compile_shader.error.create_shader_object" =>
                "Unable to create OpenGL {kind} shader object",
            "compile_shader.error.interior_null" =>
                "{kind} shader source contained an interior null byte",
            "compile_shader.error.create_program_object" =>
                "Unable to create OpenGL shader program object",
            "compile_shader.error.link_no_diagnostic" =>
                "Shader program linking failed without an OpenGL diagnostic",
            "compile_shader.error.link_failed" =>
                "Shader program linking failed:\n{error}",
            "compile_shader.error.compile_no_diagnostic" =>
                "{kind} shader compilation failed without an OpenGL diagnostic",
            "compile_shader.error.compile_failed" =>
                "{kind} shader compilation failed:\n{error}",
            _ =>
                key,
        };

    let mut rendered =
        template.to_string();

    for (name, value) in params {
        rendered =
            rendered.replace(
                &format!(
                    "{{{}}}",
                    name
                ),
                value,
            );
    }

    rendered
}



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
                runtime_text_with_params(
                    "compile_shader.error.create_shader_object",
                    &[
                        ("kind", &shader_kind_display_name(kind)),
                    ],
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
                        runtime_text_with_params(
                            "compile_shader.error.interior_null",
                            &[
                                ("kind", &shader_kind_display_name(kind)),
                            ],
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
                runtime_text(
                    "compile_shader.error.create_program_object",
                )
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

                    runtime_text(
                        "compile_shader.error.link_no_diagnostic",
                    )

                } else {

                    runtime_text_with_params(
                        "compile_shader.error.link_failed",
                        &[
                            ("error", &error),
                        ],
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
) -> String {

    match kind {

        gl::VERTEX_SHADER => {
            runtime_text(
                "compile_shader.kind.vertex",
            )
        }

        gl::FRAGMENT_SHADER => {
            runtime_text(
                "compile_shader.kind.fragment",
            )
        }

        _ => {
            runtime_text(
                "compile_shader.kind.unknown",
            )
        }
    }
}


fn format_shader_failure(
    kind: u32,
    error: &str,
) -> String {

    if error.is_empty() {

        runtime_text_with_params(
            "compile_shader.error.compile_no_diagnostic",
            &[
                ("kind", &shader_kind_display_name(kind)),
            ],
        )

    } else {

        runtime_text_with_params(
            "compile_shader.error.compile_failed",
            &[
                ("kind", &shader_kind_display_name(kind)),
                ("error", error),
            ],
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

