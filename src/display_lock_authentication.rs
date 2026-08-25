use std::ffi::{
    c_void,
    CString,
};


const PANEL_VERTEX_SHADER: &str = r#"
#version 330 core

layout(location = 0) in vec2 a_position;

void main() {
    gl_Position =
        vec4(
            a_position,
            0.0,
            1.0
        );
}
"#;


const PANEL_FRAGMENT_SHADER: &str = r#"
#version 330 core

uniform vec4 u_color;

out vec4 frag_color;

void main() {
    frag_color =
        u_color;
}
"#;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationAction {
    None,
    Dismiss,
    Submit,
}


pub struct LockAuthentication {
    username: String,
    password: String,
    status: Option<String>,
    revision: u64,
}


impl LockAuthentication {

    pub fn new() -> Self {
        let username =
            std::env::var(
                "USER"
            )
            .or_else(
                |_| {
                    std::env::var(
                        "LOGNAME"
                    )
                }
            )
            .unwrap_or_else(
                |_| {
                    "current user"
                        .to_string()
                }
            );


        Self {
            username,
            password:
                String::new(),
            status:
                None,
            revision:
                1,
        }
    }


    pub fn username(
        &self
    ) -> &str {
        &self.username
    }


    pub fn password_length(
        &self
    ) -> usize {
        self.password
            .chars()
            .count()
    }


    pub fn status(
        &self
    ) -> Option<&str> {
        self.status.as_deref()
    }


    pub fn revision(
        &self
    ) -> u64 {
        self.revision
    }


    pub fn clear(
        &mut self
    ) {
        self.password.clear();
        self.status =
            None;

        self.bump_revision();
    }


    pub fn handle_key(
        &mut self,
        keysym: u32,
        utf8: &str,
    ) -> AuthenticationAction {
        use xkbcommon::xkb::keysyms;


        match keysym {
            keysyms::KEY_Escape => {
                self.clear();

                AuthenticationAction::Dismiss
            }


            keysyms::KEY_BackSpace => {
                if self.password.pop().is_some() {
                    self.status =
                        None;

                    self.bump_revision();
                }

                AuthenticationAction::None
            }


            keysyms::KEY_Return
            | keysyms::KEY_KP_Enter => {
                // The caller owns authentication policy.  Enter merely
                // requests submission; it never grants unlock authority.
                AuthenticationAction::Submit
            }


            _ => {
                let printable =
                    utf8
                        .chars()
                        .filter(
                            |character| {
                                !character.is_control()
                            }
                        )
                        .collect::<String>();


                if !printable.is_empty() {
                    let remaining =
                        256_usize
                            .saturating_sub(
                                self.password
                                    .chars()
                                    .count()
                            );


                    if remaining > 0 {
                        self.password.extend(
                            printable
                                .chars()
                                .take(
                                    remaining
                                )
                        );

                        self.status =
                            None;

                        self.bump_revision();
                    }
                }


                AuthenticationAction::None
            }
        }
    }


    pub fn take_password(
        &mut self
    ) -> String {
        let password =
            std::mem::take(
                &mut self.password
            );

        self.bump_revision();

        password
    }


    pub fn set_status(
        &mut self,
        status: impl Into<String>,
    ) {
        self.status =
            Some(
                status.into()
            );

        self.bump_revision();
    }


    fn bump_revision(
        &mut self
    ) {
        self.revision =
            self.revision
                .wrapping_add(
                    1
                );
    }
}


pub struct LockAuthenticationPanel {
    program: u32,
    vao: u32,
    vbo: u32,
    username_overlay:
        crate::display_overlay::OpenGlOverlay,
    password_overlay:
        crate::display_overlay::OpenGlOverlay,
    status_overlay:
        Option<crate::display_overlay::OpenGlOverlay>,
}


impl LockAuthenticationPanel {

    pub fn new(
        authentication: &LockAuthentication,
        output_width: u32,
        output_height: u32,
    ) -> Result<Self, String> {
        let program =
            build_program()?;


        let mut vao =
            0_u32;

        let mut vbo =
            0_u32;


        unsafe {
            gl::GenVertexArrays(
                1,
                &mut vao,
            );

            gl::GenBuffers(
                1,
                &mut vbo,
            );
        }


        let placement =
            crate::parse_subtitle_placement::SubtitlePlacement {
                horizontal:
                    crate::parse_subtitle_placement::SubtitleHorizontalPosition::Center,
                vertical:
                    crate::parse_subtitle_placement::SubtitleVerticalPosition::Top,
            };


        let username_overlay =
            crate::display_overlay::OpenGlOverlay::new_message(
                &format!(
                    "Screenshaver  |  User: {}",
                    authentication.username(),
                ),
                placement,
                output_width,
                output_height,
            )?;


        // Bullets remain intentionally visible during this diagnostic phase.
        // The production authentication UI will use no password echo.
        let password_mask =
            if authentication.password_length() == 0 {
                "Password: ________"
                    .to_string()
            } else {
                format!(
                    "Password: {}",
                    "•".repeat(
                        authentication.password_length()
                    ),
                )
            };


        let password_overlay =
            crate::display_overlay::OpenGlOverlay::new_message(
                &password_mask,
                placement,
                output_width,
                output_height,
            )?;


        let status_overlay =
            authentication
                .status()
                .map(
                    |status| {
                        crate::display_overlay::OpenGlOverlay::new_message(
                            status,
                            placement,
                            output_width,
                            output_height,
                        )
                    }
                )
                .transpose()?;


        Ok(
            Self {
                program,
                vao,
                vbo,
                username_overlay,
                password_overlay,
                status_overlay,
            }
        )
    }


    pub fn display(
        &self,
        output_width: u32,
        output_height: u32,
    ) {
        if output_width == 0
            || output_height == 0
        {
            return;
        }


        let panel_width =
            (
                output_width as f32
                    * 0.42
            )
            .clamp(
                520.0,
                900.0,
            );

        let panel_height =
            (
                output_height as f32
                    * 0.28
            )
            .clamp(
                300.0,
                520.0,
            );


        let left =
            (
                output_width as f32
                    - panel_width
            )
                * 0.5;

        let right =
            left
                + panel_width;

        let top =
            (
                output_height as f32
                    - panel_height
            )
                * 0.5;

        let bottom =
            top
                + panel_height;


        self.draw_rectangle(
            left,
            top,
            right,
            bottom,
            output_width,
            output_height,
            [
                0.035,
                0.035,
                0.045,
                0.90,
            ],
        );


        let inset =
            2.0_f32;

        self.draw_rectangle(
            left + inset,
            top + inset,
            right - inset,
            top + 4.0,
            output_width,
            output_height,
            [
                0.70,
                0.70,
                0.74,
                0.80,
            ],
        );


        // The text textures use the existing proven text renderer, but their
        // final positions are supplied explicitly here so this is a centered
        // authentication panel rather than a subtitle/description pill.
        let username_y =
            top
                + panel_height
                    * 0.20;

        let password_y =
            top
                + panel_height
                    * 0.46;

        let status_y =
            top
                + panel_height
                    * 0.72;


        self.username_overlay.display_at_center(
            output_width,
            output_height,
            username_y,
        );

        self.password_overlay.display_at_center(
            output_width,
            output_height,
            password_y,
        );


        if let Some(status_overlay) =
            self.status_overlay.as_ref()
        {
            status_overlay.display_at_center(
                output_width,
                output_height,
                status_y,
            );
        }
    }


    #[allow(clippy::too_many_arguments)]
    fn draw_rectangle(
        &self,
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        output_width: u32,
        output_height: u32,
        color: [f32; 4],
    ) {
        let x0 =
            left
                / output_width as f32
                * 2.0
                - 1.0;

        let x1 =
            right
                / output_width as f32
                * 2.0
                - 1.0;

        let y0 =
            1.0
                - top
                    / output_height as f32
                    * 2.0;

        let y1 =
            1.0
                - bottom
                    / output_height as f32
                    * 2.0;


        let vertices: [f32; 12] = [
            x0, y0,
            x0, y1,
            x1, y1,
            x0, y0,
            x1, y1,
            x1, y0,
        ];


        unsafe {
            gl::Enable(
                gl::BLEND
            );

            gl::BlendFunc(
                gl::SRC_ALPHA,
                gl::ONE_MINUS_SRC_ALPHA,
            );

            gl::UseProgram(
                self.program
            );

            let color_location =
                gl::GetUniformLocation(
                    self.program,
                    b"u_color\0".as_ptr()
                        as *const i8,
                );

            gl::Uniform4f(
                color_location,
                color[0],
                color[1],
                color[2],
                color[3],
            );

            gl::BindVertexArray(
                self.vao
            );

            gl::BindBuffer(
                gl::ARRAY_BUFFER,
                self.vbo,
            );

            gl::BufferData(
                gl::ARRAY_BUFFER,
                (
                    vertices.len()
                        * std::mem::size_of::<f32>()
                ) as isize,
                vertices.as_ptr()
                    as *const c_void,
                gl::STREAM_DRAW,
            );

            gl::EnableVertexAttribArray(
                0
            );

            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                (
                    2
                        * std::mem::size_of::<f32>()
                ) as i32,
                std::ptr::null(),
            );

            gl::DrawArrays(
                gl::TRIANGLES,
                0,
                6,
            );

            gl::BindBuffer(
                gl::ARRAY_BUFFER,
                0,
            );

            gl::BindVertexArray(
                0
            );

            gl::UseProgram(
                0
            );

            gl::Disable(
                gl::BLEND
            );
        }
    }
}


impl Drop for LockAuthenticationPanel {

    fn drop(
        &mut self
    ) {
        unsafe {
            if self.vbo != 0 {
                gl::DeleteBuffers(
                    1,
                    &self.vbo,
                );
            }

            if self.vao != 0 {
                gl::DeleteVertexArrays(
                    1,
                    &self.vao,
                );
            }

            if self.program != 0 {
                gl::DeleteProgram(
                    self.program
                );
            }
        }
    }
}


fn build_program(
) -> Result<u32, String> {
    let vertex_shader =
        compile_shader(
            gl::VERTEX_SHADER,
            PANEL_VERTEX_SHADER,
        )?;

    let fragment_shader =
        compile_shader(
            gl::FRAGMENT_SHADER,
            PANEL_FRAGMENT_SHADER,
        )?;


    let program =
        unsafe {
            gl::CreateProgram()
        };


    unsafe {
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

        gl::DeleteShader(
            vertex_shader
        );

        gl::DeleteShader(
            fragment_shader
        );
    }


    let mut link_status =
        0_i32;


    unsafe {
        gl::GetProgramiv(
            program,
            gl::LINK_STATUS,
            &mut link_status,
        );
    }


    if link_status
        != gl::TRUE as i32
    {
        let message =
            program_log(
                program
            );

        unsafe {
            gl::DeleteProgram(
                program
            );
        }

        return Err(
            format!(
                "Unable to link lock-authentication panel shader: {}",
                message,
            )
        );
    }


    Ok(
        program
    )
}


fn compile_shader(
    shader_type: u32,
    source: &str,
) -> Result<u32, String> {
    let shader =
        unsafe {
            gl::CreateShader(
                shader_type
            )
        };


    let source =
        CString::new(
            source
        )
        .map_err(
            |error| {
                format!(
                    "Lock-authentication shader contains an interior NUL: {}",
                    error,
                )
            }
        )?;


    unsafe {
        gl::ShaderSource(
            shader,
            1,
            &source.as_ptr(),
            std::ptr::null(),
        );

        gl::CompileShader(
            shader
        );
    }


    let mut compile_status =
        0_i32;


    unsafe {
        gl::GetShaderiv(
            shader,
            gl::COMPILE_STATUS,
            &mut compile_status,
        );
    }


    if compile_status
        != gl::TRUE as i32
    {
        let message =
            shader_log(
                shader
            );

        unsafe {
            gl::DeleteShader(
                shader
            );
        }

        return Err(
            format!(
                "Unable to compile lock-authentication panel shader: {}",
                message,
            )
        );
    }


    Ok(
        shader
    )
}


fn shader_log(
    shader: u32,
) -> String {
    let mut length =
        0_i32;


    unsafe {
        gl::GetShaderiv(
            shader,
            gl::INFO_LOG_LENGTH,
            &mut length,
        );
    }


    if length <= 1 {
        return "no shader compiler log was provided"
            .to_string();
    }


    let mut buffer =
        vec![
            0_u8;
            length as usize
        ];

    let mut written =
        0_i32;


    unsafe {
        gl::GetShaderInfoLog(
            shader,
            length,
            &mut written,
            buffer.as_mut_ptr()
                as *mut i8,
        );
    }


    String::from_utf8_lossy(
        &buffer[
            ..written.max(0) as usize
        ]
    )
        .to_string()
}


fn program_log(
    program: u32,
) -> String {
    let mut length =
        0_i32;


    unsafe {
        gl::GetProgramiv(
            program,
            gl::INFO_LOG_LENGTH,
            &mut length,
        );
    }


    if length <= 1 {
        return "no program linker log was provided"
            .to_string();
    }


    let mut buffer =
        vec![
            0_u8;
            length as usize
        ];

    let mut written =
        0_i32;


    unsafe {
        gl::GetProgramInfoLog(
            program,
            length,
            &mut written,
            buffer.as_mut_ptr()
                as *mut i8,
        );
    }


    String::from_utf8_lossy(
        &buffer[
            ..written.max(0) as usize
        ]
    )
        .to_string()
}
