use std::ffi::CString;
use std::path::PathBuf;

pub fn compile_shader(
    source: &str,
    kind: u32,
) -> u32 {

    log("Entered compile_shader()");

    unsafe {
        let shader =
            gl::CreateShader(kind);

        if shader == 0 {
            log("ERROR: gl::CreateShader returned 0");
            panic!("Shader creation failed");
        }

        let c_source =
            CString::new(source)
                .expect("Shader source contained interior null byte");

        gl::ShaderSource(
            shader,
            1,
            &c_source.as_ptr(),
            std::ptr::null(),
        );

        gl::CompileShader(shader);

        if !shader_compile_success(shader) {
            let error =
                shader_info_log(shader);

            log(
                &format!(
                    "ERROR: shader compilation failed:\n{}",
                    error
                )
            );

            panic!(
                "Shader compile error:\n{}",
                error
            );
        }

        shader
    }
}

pub fn link_program(
    vertex_shader: u32,
    fragment_shader: u32,
) -> u32 {

    unsafe {
        let program =
            gl::CreateProgram();

        if program == 0 {
            log("ERROR: gl::CreateProgram returned 0");
            panic!("Program creation failed");
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

        if !program_link_success(program) {
            let error =
                program_info_log(program);

            log(
                &format!(
                    "ERROR: shader program link failed:\n{}",
                    error
                )
            );

            panic!(
                "Program link error:\n{}",
                error
            );
        }

        program
    }
}

pub fn build_program(
    vertex_source: &str,
    fragment_source: &str,
) -> u32 {

    unsafe {
        log("Compiling vertex shader");

        let vertex_shader =
            compile_shader(
                vertex_source,
                gl::VERTEX_SHADER,
            );

        log("Compiling fragment shader");

        let fragment_shader =
            compile_shader(
                fragment_source,
                gl::FRAGMENT_SHADER,
            );

        log("Linking shader program");

        let program =
            link_program(
                vertex_shader,
                fragment_shader,
            );

        gl::DeleteShader(
            vertex_shader
        );

        gl::DeleteShader(
            fragment_shader
        );

        log("Shader program built successfully");

        program
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

        success != 0
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

        success != 0
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

        if len <= 0 {
            return String::new();
        }

        let mut buffer =
            vec![0u8; len as usize];

        gl::GetShaderInfoLog(
            shader,
            len,
            std::ptr::null_mut(),
            buffer.as_mut_ptr() as *mut _,
        );

        String::from_utf8_lossy(&buffer)
            .trim_end_matches('\0')
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

        if len <= 0 {
            return String::new();
        }

        let mut buffer =
            vec![0u8; len as usize];

        gl::GetProgramInfoLog(
            program,
            len,
            std::ptr::null_mut(),
            buffer.as_mut_ptr() as *mut _,
        );

        String::from_utf8_lossy(&buffer)
            .trim_end_matches('\0')
            .to_string()
    }
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