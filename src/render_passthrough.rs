const PASSTHROUGH_VERTEX_SHADER: &str = r#"
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

const PASSTHROUGH_FRAGMENT_SHADER: &str = r#"
#version 330 core

uniform sampler2D uScene;
uniform bool uInvertColors;
uniform float uHueRotation;

in vec2 vUv;

out vec4 fragColor;

vec3 rotateHue(vec3 color, float degrees)
{
    float angle = radians(degrees);
    float cosine = cos(angle);
    float sine = sin(angle);

    float y = dot(color, vec3(0.299, 0.587, 0.114));
    float i = dot(color, vec3(0.596, -0.274, -0.322));
    float q = dot(color, vec3(0.211, -0.523, 0.312));

    float rotatedI = i * cosine - q * sine;
    float rotatedQ = i * sine + q * cosine;

    return clamp(
        vec3(
            y + 0.956 * rotatedI + 0.621 * rotatedQ,
            y - 0.272 * rotatedI - 0.647 * rotatedQ,
            y - 1.106 * rotatedI + 1.703 * rotatedQ
        ),
        0.0,
        1.0
    );
}

void main()
{
    vec4 scene = texture(uScene, vUv);

    if (uInvertColors) {
        scene.rgb = vec3(1.0) - scene.rgb;
    }

    if (abs(uHueRotation) > 0.0001) {
        scene.rgb = rotateHue(scene.rgb, uHueRotation);
    }

    fragColor = scene;
}
"#;

pub(crate) struct PassthroughRenderer {
    program: u32,
    vao: u32,
    scene_location: i32,
    invert_colors_location: i32,
    hue_rotation_location: i32,
}

impl PassthroughRenderer {
    pub(crate) fn new() -> Result<Self, String> {
        let program = crate::compile_shader::build_program(
            PASSTHROUGH_VERTEX_SHADER,
            PASSTHROUGH_FRAGMENT_SHADER,
        )
        .map_err(|error| {
            format!(
                "Unable to build passthrough presentation program: {}",
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
                "OpenGL failed to allocate the passthrough vertex array"
                    .to_string(),
            );
        }

        let scene_location = unsafe {
            gl::GetUniformLocation(program, b"uScene\0".as_ptr().cast())
        };

        let invert_colors_location = unsafe {
            gl::GetUniformLocation(program, b"uInvertColors\0".as_ptr().cast())
        };

        let hue_rotation_location = unsafe {
            gl::GetUniformLocation(
                program,
                b"uHueRotation\0".as_ptr().cast(),
            )
        };

        if scene_location == -1
            || invert_colors_location == -1
            || hue_rotation_location == -1
        {
            unsafe {
                gl::DeleteVertexArrays(1, &vao);
                gl::DeleteProgram(program);
            }

            return Err(
                "Passthrough presentation program does not expose the uScene sampler"
                    .to_string(),
            );
        }

        Ok(Self {
            program,
            vao,
            scene_location,
            invert_colors_location,
            hue_rotation_location,
        })
    }

    pub(crate) fn render(
        &self,
        scene_texture: u32,
        invert_colors: bool,
        hue_rotation: f32,
    ) {
        unsafe {
            gl::UseProgram(self.program);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, scene_texture);
            gl::Uniform1i(self.scene_location, 0);
            gl::Uniform1i(
                self.invert_colors_location,
                if invert_colors { 1 } else { 0 },
            );
            gl::Uniform1f(
                self.hue_rotation_location,
                hue_rotation,
            );
            gl::BindVertexArray(self.vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}

impl Drop for PassthroughRenderer {
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

