const DITHERING_VERTEX_SHADER: &str = r#"
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

const DITHERING_FRAGMENT_SHADER: &str = r#"
#version 330 core

uniform sampler2D uScene;
uniform float uStrength;

in vec2 vUv;

out vec4 fragColor;

float bayer4x4(ivec2 pixel)
{
    const float matrix[16] = float[16](
         0.0,  8.0,  2.0, 10.0,
        12.0,  4.0, 14.0,  6.0,
         3.0, 11.0,  1.0,  9.0,
        15.0,  7.0, 13.0,  5.0
    );

    ivec2 cell = ivec2(
        pixel.x & 3,
        pixel.y & 3
    );

    return matrix[cell.y * 4 + cell.x] / 16.0;
}

void main()
{
    vec4 scene = texture(uScene, vUv);

    float signedThreshold =
        bayer4x4(ivec2(gl_FragCoord.xy)) - 0.5;

    vec3 dithered =
        clamp(
            scene.rgb + signedThreshold * uStrength,
            0.0,
            1.0
        );

    fragColor = vec4(
        dithered,
        scene.a
    );
}
"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DitheringLevel {
    Off,
    Subtle,
}

impl DitheringLevel {
    pub(crate) fn name(
        self,
    ) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Subtle => "subtle",
        }
    }

    pub(crate) fn strength(
        self,
    ) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Subtle => 0.5 / 255.0,
        }
    }

    pub(crate) fn is_enabled(
        self,
    ) -> bool {
        self != Self::Off
    }
}

pub(crate) struct DitheringRenderer {
    program: u32,
    vao: u32,
    scene_location: i32,
    strength_location: i32,
}

impl DitheringRenderer {
    pub(crate) fn new(
    ) -> Result<Self, String> {
        let program =
            crate::compile_shader::build_program(
                DITHERING_VERTEX_SHADER,
                DITHERING_FRAGMENT_SHADER,
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to build dithering presentation program: {}",
                        error,
                    )
                }
            )?;

        let mut vao =
            0_u32;

        unsafe {
            gl::GenVertexArrays(
                1,
                &mut vao,
            );
        }

        if vao == 0 {
            unsafe {
                gl::DeleteProgram(
                    program
                );
            }

            return Err(
                "OpenGL failed to allocate the dithering vertex array"
                    .to_string()
            );
        }

        let scene_location =
            unsafe {
                gl::GetUniformLocation(
                    program,
                    b"uScene\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let strength_location =
            unsafe {
                gl::GetUniformLocation(
                    program,
                    b"uStrength\0"
                        .as_ptr()
                        .cast(),
                )
            };

        if scene_location == -1
            || strength_location == -1
        {
            unsafe {
                gl::DeleteVertexArrays(
                    1,
                    &vao,
                );

                gl::DeleteProgram(
                    program
                );
            }

            return Err(
                "Dithering presentation program is missing a required uniform"
                    .to_string()
            );
        }

        Ok(
            Self {
                program,
                vao,
                scene_location,
                strength_location,
            }
        )
    }

    pub(crate) fn render(
        &self,
        scene_texture: u32,
        level: DitheringLevel,
    ) {
        if !level.is_enabled() {
            return;
        }

        unsafe {
            gl::UseProgram(
                self.program
            );

            gl::ActiveTexture(
                gl::TEXTURE0
            );

            gl::BindTexture(
                gl::TEXTURE_2D,
                scene_texture,
            );

            gl::Uniform1i(
                self.scene_location,
                0,
            );

            gl::Uniform1f(
                self.strength_location,
                level.strength(),
            );

            gl::BindVertexArray(
                self.vao
            );

            gl::DrawArrays(
                gl::TRIANGLES,
                0,
                3,
            );

            gl::BindTexture(
                gl::TEXTURE_2D,
                0,
            );
        }
    }
}

impl Drop for DitheringRenderer {
    fn drop(
        &mut self,
    ) {
        unsafe {
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

        self.vao =
            0;

        self.program =
            0;
    }
}

