use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};

pub fn display(
    canvas: &mut Canvas<Window>,
    texture_creator: &TextureCreator<WindowContext>,
    overlay: &crate::construct_text_overlay::ConstructedTextOverlay,
    placement: crate::parse_subtitle_placement::SubtitlePlacement,
    output_width: u32,
    output_height: u32,
) -> Result<(), String> {
    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::RGBA32,
            overlay.width,
            overlay.height,
        )
        .map_err(|error| format!("Unable to create SDL subtitle texture: {}", error))?;

    texture.set_blend_mode(sdl2::render::BlendMode::Blend);

    let pitch = usize::try_from(overlay.width)
        .map_err(|_| "Subtitle width cannot be represented as usize".to_string())?
        .checked_mul(4)
        .ok_or_else(|| "Subtitle pitch overflow".to_string())?;

    texture
        .update(None, &overlay.pixels, pitch)
        .map_err(|error| format!("Unable to upload subtitle pixels: {}", error))?;

    canvas
        .copy(
            &texture,
            None,
            destination_rect(
                overlay.width,
                overlay.height,
                placement,
                output_width,
                output_height,
            ),
        )
        .map_err(|error| format!("Unable to draw subtitle overlay: {}", error))
}

fn destination_rect(
    overlay_width: u32,
    overlay_height: u32,
    placement: crate::parse_subtitle_placement::SubtitlePlacement,
    output_width: u32,
    output_height: u32,
) -> Rect {
    let margin = crate::construct_text_overlay::edge_margin(output_height);

    let x = match placement.horizontal {
        crate::parse_subtitle_placement::SubtitleHorizontalPosition::Left => margin,
        crate::parse_subtitle_placement::SubtitleHorizontalPosition::Center => {
            output_width.saturating_sub(overlay_width) / 2
        }
        crate::parse_subtitle_placement::SubtitleHorizontalPosition::Right => {
            output_width.saturating_sub(overlay_width.saturating_add(margin))
        }
    };

    let y = match placement.vertical {
        crate::parse_subtitle_placement::SubtitleVerticalPosition::Top => margin,
        crate::parse_subtitle_placement::SubtitleVerticalPosition::Bottom => {
            output_height.saturating_sub(overlay_height.saturating_add(margin))
        }
    };

    Rect::new(x as i32, y as i32, overlay_width, overlay_height)
}



// ============================================================
// OpenGL overlay display
// ============================================================

use std::ffi::CString;


const OVERLAY_VERTEX_SHADER: &str = r#"
#version 330 core

uniform vec2 uViewport;
uniform vec2 uOrigin;
uniform vec2 uSize;

out vec2 vUv;

void main()
{
    vec2 corner;

    if (gl_VertexID == 0) {
        corner = vec2(0.0, 0.0);
    } else if (gl_VertexID == 1) {
        corner = vec2(1.0, 0.0);
    } else if (gl_VertexID == 2) {
        corner = vec2(0.0, 1.0);
    } else {
        corner = vec2(1.0, 1.0);
    }

    vec2 pixel = uOrigin + corner * uSize;
    vec2 ndc = pixel / uViewport * 2.0 - 1.0;

    gl_Position = vec4(ndc, 0.0, 1.0);

    // The constructed RGBA image uses top-left row order.
    vUv = vec2(corner.x, 1.0 - corner.y);
}
"#;


const OVERLAY_FRAGMENT_SHADER: &str = r#"
#version 330 core

uniform sampler2D uOverlay;

in vec2 vUv;

out vec4 fragColor;

void main()
{
    fragColor = texture(uOverlay, vUv);
}
"#;


pub struct OpenGlOverlay {
    program: u32,
    vao: u32,
    texture: u32,
    width: u32,
    height: u32,
    placement:
        crate::parse_subtitle_placement::SubtitlePlacement,
}


impl OpenGlOverlay {

    pub fn new_message(
        message: &str,

        placement:
            crate::parse_subtitle_placement::SubtitlePlacement,

        output_width:
            u32,

        output_height:
            u32,
    ) -> Result<Self, String> {

        let overlay =
            crate::construct_text_overlay::construct_message(
                message,
                output_width,
                output_height,
            )?;


        Self::new_from_constructed(
            overlay,
            placement,
        )
    }


    pub fn new(
        descriptor:
            &crate::construct_text_overlay::OverlayDescriptor,

        placement:
            crate::parse_subtitle_placement::SubtitlePlacement,

        output_width:
            u32,

        output_height:
            u32,
    ) -> Result<Self, String> {

        Self::new_with_optional_fps(
            descriptor,
            None,
            placement,
            output_width,
            output_height,
        )
    }


    pub fn new_with_fps(
        descriptor:
            &crate::construct_text_overlay::OverlayDescriptor,

        rendered_fps:
            u32,

        placement:
            crate::parse_subtitle_placement::SubtitlePlacement,

        output_width:
            u32,

        output_height:
            u32,
    ) -> Result<Self, String> {

        Self::new_with_optional_fps(
            descriptor,
            Some(rendered_fps),
            placement,
            output_width,
            output_height,
        )
    }


    pub fn new_with_fps_warning(
        descriptor:
            &crate::construct_text_overlay::OverlayDescriptor,

        rendered_fps:
            u32,

        warning_state:
            crate::fps_monitor::FpsWarningState,

        placement:
            crate::parse_subtitle_placement::SubtitlePlacement,

        output_width:
            u32,

        output_height:
            u32,
    ) -> Result<Self, String> {

        Self::new_with_optional_fps_warning(
            descriptor,
            Some(rendered_fps),
            warning_state,
            placement,
            output_width,
            output_height,
        )
    }


    fn new_with_optional_fps(
        descriptor:
            &crate::construct_text_overlay::OverlayDescriptor,

        rendered_fps:
            Option<u32>,

        placement:
            crate::parse_subtitle_placement::SubtitlePlacement,

        output_width:
            u32,

        output_height:
            u32,
    ) -> Result<Self, String> {

        Self::new_with_optional_fps_warning(
            descriptor,
            rendered_fps,
            crate::fps_monitor::FpsWarningState::Normal,
            placement,
            output_width,
            output_height,
        )
    }


    fn new_with_optional_fps_warning(
        descriptor:
            &crate::construct_text_overlay::OverlayDescriptor,

        rendered_fps:
            Option<u32>,

        warning_state:
            crate::fps_monitor::FpsWarningState,

        placement:
            crate::parse_subtitle_placement::SubtitlePlacement,

        output_width:
            u32,

        output_height:
            u32,
    ) -> Result<Self, String> {

        let overlay =
            crate::construct_text_overlay::construct_with_fps_warning(
                descriptor,
                rendered_fps,
                warning_state,
                output_width,
                output_height,
            )?;


        Self::new_from_constructed(
            overlay,
            placement,
        )
    }


    fn new_from_constructed(
        overlay:
            crate::construct_text_overlay::ConstructedTextOverlay,

        placement:
            crate::parse_subtitle_placement::SubtitlePlacement,
    ) -> Result<Self, String> {

        let program =
            build_overlay_program()?;


        let mut vao =
            0_u32;


        let mut texture =
            0_u32;


        unsafe {
            gl::GenVertexArrays(
                1,
                &mut vao,
            );


            gl::GenTextures(
                1,
                &mut texture,
            );


            gl::BindTexture(
                gl::TEXTURE_2D,
                texture,
            );


            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                gl::LINEAR as i32,
            );


            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                gl::LINEAR as i32,
            );


            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_EDGE as i32,
            );


            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_EDGE as i32,
            );


            gl::PixelStorei(
                gl::UNPACK_ALIGNMENT,
                1,
            );


            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as i32,
                overlay.width as i32,
                overlay.height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                overlay.pixels
                    .as_ptr()
                    .cast(),
            );


            gl::BindTexture(
                gl::TEXTURE_2D,
                0,
            );
        }


        if vao == 0
            || texture == 0
        {
            unsafe {
                if vao != 0 {
                    gl::DeleteVertexArrays(
                        1,
                        &vao,
                    );
                }


                if texture != 0 {
                    gl::DeleteTextures(
                        1,
                        &texture,
                    );
                }


                gl::DeleteProgram(
                    program
                );
            }


            return Err(
                "OpenGL failed to allocate subtitle overlay resources"
                    .to_string()
            );
        }


        Ok(
            Self {
                program,
                vao,
                texture,
                width:
                    overlay.width,
                height:
                    overlay.height,
                placement,
            }
        )
    }


    pub fn display_at_center(
        &self,
        output_width: u32,
        output_height: u32,
        center_y: f32,
    ) {
        if output_width == 0
            || output_height == 0
        {
            return;
        }


        let x =
            output_width
                .saturating_sub(
                    self.width
                )
                / 2;


        // OpenGL overlay coordinates use a lower-left origin.  center_y is
        // supplied in ordinary top-left UI coordinates by the lock panel, so
        // convert the requested center position into the lower-left origin
        // expected by the existing overlay shader.
        let top =
            (
                center_y
                    - self.height as f32
                        * 0.5
            )
            .max(
                0.0
            );


        let y =
            output_height
                .saturating_sub(
                    (
                        top
                            + self.height as f32
                    )
                    .round()
                    .max(
                        0.0
                    ) as u32
                );


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


            set_vec2(
                self.program,
                "uViewport",
                output_width as f32,
                output_height as f32,
            );


            set_vec2(
                self.program,
                "uOrigin",
                x as f32,
                y as f32,
            );


            set_vec2(
                self.program,
                "uSize",
                self.width as f32,
                self.height as f32,
            );


            gl::ActiveTexture(
                gl::TEXTURE7
            );


            gl::BindTexture(
                gl::TEXTURE_2D,
                self.texture,
            );


            set_int(
                self.program,
                "uOverlay",
                7,
            );


            gl::BindVertexArray(
                self.vao
            );


            gl::DrawArrays(
                gl::TRIANGLE_STRIP,
                0,
                4,
            );


            gl::BindTexture(
                gl::TEXTURE_2D,
                0,
            );


            gl::ActiveTexture(
                gl::TEXTURE0
            );


            gl::Disable(
                gl::BLEND
            );
        }
    }


    pub fn display(
        &self,
        output_width: u32,
        output_height: u32,
    ) {

        let margin =
            crate::construct_text_overlay::edge_margin(
                output_height
            );


        let x =
            match self.placement.horizontal {

                crate::parse_subtitle_placement::SubtitleHorizontalPosition::Left => {
                    margin
                }

                crate::parse_subtitle_placement::SubtitleHorizontalPosition::Center => {
                    output_width
                        .saturating_sub(
                            self.width
                        )
                        / 2
                }

                crate::parse_subtitle_placement::SubtitleHorizontalPosition::Right => {
                    output_width
                        .saturating_sub(
                            self.width
                                .saturating_add(
                                    margin
                                )
                        )
                }
            };


        // OpenGL coordinates begin at the lower-left corner.
        let y =
            match self.placement.vertical {

                crate::parse_subtitle_placement::SubtitleVerticalPosition::Bottom => {
                    margin
                }

                crate::parse_subtitle_placement::SubtitleVerticalPosition::Top => {
                    output_height
                        .saturating_sub(
                            self.height
                                .saturating_add(
                                    margin
                                )
                        )
                }
            };


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


            set_vec2(
                self.program,
                "uViewport",
                output_width as f32,
                output_height as f32,
            );


            set_vec2(
                self.program,
                "uOrigin",
                x as f32,
                y as f32,
            );


            set_vec2(
                self.program,
                "uSize",
                self.width as f32,
                self.height as f32,
            );


            gl::ActiveTexture(
                gl::TEXTURE7
            );


            gl::BindTexture(
                gl::TEXTURE_2D,
                self.texture,
            );


            set_int(
                self.program,
                "uOverlay",
                7,
            );


            gl::BindVertexArray(
                self.vao
            );


            gl::DrawArrays(
                gl::TRIANGLE_STRIP,
                0,
                4,
            );


            gl::BindTexture(
                gl::TEXTURE_2D,
                0,
            );


            gl::ActiveTexture(
                gl::TEXTURE0
            );


            gl::Disable(
                gl::BLEND
            );
        }
    }
}


impl Drop for OpenGlOverlay {

    fn drop(
        &mut self,
    ) {

        unsafe {
            if self.texture != 0 {
                gl::DeleteTextures(
                    1,
                    &self.texture,
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


fn build_overlay_program() -> Result<u32, String> {

    let vertex_shader =
        compile_overlay_shader(
            OVERLAY_VERTEX_SHADER,
            gl::VERTEX_SHADER,
        )?;


    let fragment_shader =
        match compile_overlay_shader(
            OVERLAY_FRAGMENT_SHADER,
            gl::FRAGMENT_SHADER,
        ) {

            Ok(shader) => shader,

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


    let mut linked =
        0_i32;


    unsafe {
        gl::GetProgramiv(
            program,
            gl::LINK_STATUS,
            &mut linked,
        );
    }


    if linked == 0 {

        let error =
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
                "Unable to link subtitle overlay program: {}",
                error,
            )
        );
    }


    Ok(
        program
    )
}


fn compile_overlay_shader(
    source: &str,
    kind: u32,
) -> Result<u32, String> {

    let shader =
        unsafe {
            gl::CreateShader(
                kind
            )
        };


    let source =
        CString::new(
            source
        )
        .map_err(
            |_| {
                "Subtitle overlay shader source contains a null byte"
                    .to_string()
            }
        )?;


    unsafe {
        let pointer =
            source.as_ptr();


        gl::ShaderSource(
            shader,
            1,
            &pointer,
            std::ptr::null(),
        );


        gl::CompileShader(
            shader
        );
    }


    let mut compiled =
        0_i32;


    unsafe {
        gl::GetShaderiv(
            shader,
            gl::COMPILE_STATUS,
            &mut compiled,
        );
    }


    if compiled == 0 {

        let error =
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
                "Unable to compile subtitle overlay shader: {}",
                error,
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
        return "unknown shader compilation error"
            .to_string();
    }


    let mut buffer =
        vec![
            0_u8;
            length as usize
        ];


    unsafe {
        gl::GetShaderInfoLog(
            shader,
            length,
            std::ptr::null_mut(),
            buffer.as_mut_ptr()
                .cast(),
        );
    }


    String::from_utf8_lossy(
        &buffer
    )
    .trim_matches(
        '\0'
    )
    .trim()
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
        return "unknown program link error"
            .to_string();
    }


    let mut buffer =
        vec![
            0_u8;
            length as usize
        ];


    unsafe {
        gl::GetProgramInfoLog(
            program,
            length,
            std::ptr::null_mut(),
            buffer.as_mut_ptr()
                .cast(),
        );
    }


    String::from_utf8_lossy(
        &buffer
    )
    .trim_matches(
        '\0'
    )
    .trim()
    .to_string()
}


unsafe fn set_vec2(
    program: u32,
    name: &str,
    x: f32,
    y: f32,
) {

    let name =
        CString::new(
            name
        )
        .expect(
            "static subtitle uniform name"
        );


    let location =
        gl::GetUniformLocation(
            program,
            name.as_ptr(),
        );


    if location != -1 {
        gl::Uniform2f(
            location,
            x,
            y,
        );
    }
}


unsafe fn set_int(
    program: u32,
    name: &str,
    value: i32,
) {

    let name =
        CString::new(
            name
        )
        .expect(
            "static subtitle uniform name"
        );


    let location =
        gl::GetUniformLocation(
            program,
            name.as_ptr(),
        );


    if location != -1 {
        gl::Uniform1i(
            location,
            value,
        );
    }
}

