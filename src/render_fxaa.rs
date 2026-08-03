const FXAA_VERTEX_SHADER: &str = r#"
#version 330 core

out vec2 vUv;

void main()
{
    vec2 position;

    if (gl_VertexID == 0) {
        position = vec2(-1.0, -1.0);
    } else if (gl_VertexID == 1) {
        position = vec2(3.0, -1.0);
    } else {
        position = vec2(-1.0, 3.0);
    }

    gl_Position = vec4(position, 0.0, 1.0);
    vUv = position * 0.5 + 0.5;
}
"#;

const FXAA_FRAGMENT_SHADER: &str = r#"
#version 330 core

uniform sampler2D uScene;
uniform vec2 uInverseResolution;

in vec2 vUv;

out vec4 fragColor;

const float EDGE_THRESHOLD_MIN = 0.0312;
const float EDGE_THRESHOLD_MAX = 0.125;
const float SUBPIXEL_QUALITY = 0.75;

float luminance(vec3 color)
{
    return dot(color, vec3(0.299, 0.587, 0.114));
}

void main()
{
    fragColor = vec4(
        1.0,
        0.0,
        1.0,
        1.0
    );
}
"#;

pub(crate) struct FxaaRenderer {
    program: u32,
    vao: u32,
    scene_location: i32,
    inverse_resolution_location: i32,
}

impl FxaaRenderer {
    pub(crate) fn new() -> Result<Self, String> {
        panic!(
            "[FXAA DIAGNOSTIC] FxaaRenderer::new() was called"
        );

        let program = crate::compile_shader::build_program(
            FXAA_VERTEX_SHADER,
            FXAA_FRAGMENT_SHADER,
        )
        .map_err(|error| {
            format!(
                "Unable to build FXAA presentation program: {}",
                error,
            )
        })?;

        let mut vao = 0_u32;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
        }

        if vao == 0 {
            unsafe {
                gl::DeleteProgram(program);
            }

            return Err(
                "OpenGL failed to allocate the FXAA vertex array"
                    .to_string(),
            );
        }

        let scene_location = unsafe {
            gl::GetUniformLocation(
                program,
                b"uScene\0".as_ptr().cast(),
            )
        };

        let inverse_resolution_location = unsafe {
            gl::GetUniformLocation(
                program,
                b"uInverseResolution\0".as_ptr().cast(),
            )
        };

        if scene_location == -1 || inverse_resolution_location == -1 {
            unsafe {
                gl::DeleteVertexArrays(1, &vao);
                gl::DeleteProgram(program);
            }

            return Err(
                "FXAA presentation program is missing a required uniform"
                    .to_string(),
            );
        }

        Ok(Self {
            program,
            vao,
            scene_location,
            inverse_resolution_location,
        })
    }

    pub(crate) fn render(
        &self,
        scene_texture: u32,
        width: u32,
        height: u32,
    ) {
        if width == 0 || height == 0 {
            return;
        }

        let inverse_width = 1.0 / width as f32;
        let inverse_height = 1.0 / height as f32;

        unsafe {
            gl::UseProgram(self.program);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, scene_texture);
            gl::Uniform1i(self.scene_location, 0);
            gl::Uniform2f(
                self.inverse_resolution_location,
                inverse_width,
                inverse_height,
            );
            gl::BindVertexArray(self.vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}

impl Drop for FxaaRenderer {
    fn drop(&mut self) {
        unsafe {
            if self.vao != 0 {
                gl::DeleteVertexArrays(1, &self.vao);
            }

            if self.program != 0 {
                gl::DeleteProgram(self.program);
            }
        }

        self.vao = 0;
        self.program = 0;
    }
}

