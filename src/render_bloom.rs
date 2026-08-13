//! Bloom post-processing definitions and renderer support.
//!
//! Checkpoint 3 adds the first OpenGL stage: highlight extraction.  When
//! Highlight Bloom is selected, the post-processing pipeline temporarily
//! presents this extracted bright-pass image directly for visual tuning.
//! Blur and final bloom composition remain deferred to later checkpoints.

pub(crate) const BLOOM_INTENSITY_MIN: f32 =
    0.0;

pub(crate) const BLOOM_INTENSITY_MAX: f32 =
    2.0;

pub(crate) const BLOOM_INTENSITY_DEFAULT: f32 =
    1.0;

pub(crate) const BLOOM_THRESHOLD_MIN: f32 =
    0.0;

pub(crate) const BLOOM_THRESHOLD_MAX: f32 =
    2.0;

pub(crate) const BLOOM_THRESHOLD_DEFAULT: f32 =
    0.80;


#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
)]
pub(crate) enum BloomMode {

    #[default]
    Off,

    Highlight,
}


impl BloomMode {

    pub(crate) fn parse(
        value: &str,
    ) -> Result<Self, String> {

        match value
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "off" => {
                Ok(
                    Self::Off
                )
            }

            "highlight" => {
                Ok(
                    Self::Highlight
                )
            }

            other => {
                Err(
                    format!(
                        "Unsupported bloom mode '{}'; supported values: off, highlight",
                        other,
                    )
                )
            }
        }
    }


    pub(crate) fn name(
        self,
    ) -> &'static str {

        match self {
            Self::Off => "off",
            Self::Highlight => "highlight",
        }
    }


    #[allow(dead_code)]
    pub(crate) fn is_enabled(
        self,
    ) -> bool {

        !matches!(
            self,
            Self::Off
        )
    }
}


pub(crate) fn validate_bloom_intensity(
    value: f32,
) -> Result<f32, String> {

    if value.is_finite()
        && (BLOOM_INTENSITY_MIN
            ..=BLOOM_INTENSITY_MAX)
            .contains(
                &value
            )
    {
        return Ok(
            value
        );
    }


    Err(
        format!(
            "Bloom intensity {} is outside the supported range {:.2}-{:.2}",
            value,
            BLOOM_INTENSITY_MIN,
            BLOOM_INTENSITY_MAX,
        )
    )
}

pub(crate) fn validate_bloom_threshold(
    value: f32,
) -> Result<f32, String> {

    if value.is_finite()
        && (BLOOM_THRESHOLD_MIN
            ..=BLOOM_THRESHOLD_MAX)
            .contains(
                &value
            )
    {
        return Ok(
            value
        );
    }

    Err(
        format!(
            "Bloom threshold {} is outside the supported range {:.2}-{:.2}",
            value,
            BLOOM_THRESHOLD_MIN,
            BLOOM_THRESHOLD_MAX,
        )
    )
}


// ============================================================
// Highlight extraction diagnostic renderer
// ============================================================


const BLOOM_VERTEX_SHADER: &str = r#"
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


const BLOOM_HIGHLIGHT_FRAGMENT_SHADER: &str = r#"
#version 330 core

uniform sampler2D uScene;
uniform float uThreshold;

in vec2 vUv;

out vec4 fragColor;

float luminance(vec3 color)
{
    // Rec. 709 / sRGB luminance weights.  These weights make the extraction
    // respond to perceived brightness rather than simply the largest channel.
    return dot(
        color,
        vec3(0.2126, 0.7152, 0.0722)
    );
}

void main()
{
    vec4 sceneSample = texture(uScene, vUv);
    float brightness = luminance(sceneSample.rgb);

    // Preserve the Checkpoint 3 response for ordinary SDR thresholds: when
    // uThreshold is below 1.0, a white pixel reaches full extracted strength.
    // For thresholds at or above 1.0, use a one-luminance-unit ramp so HDR
    // values remain meaningful instead of producing an invalid denominator.
    float excess = max(
        brightness - uThreshold,
        0.0
    );

    float responseRange = uThreshold < 1.0
        ? max(1.0 - uThreshold, 0.0001)
        : 1.0;

    float highlightScale = excess / responseRange;

    fragColor = vec4(
        sceneSample.rgb * highlightScale,
        1.0
    );
}
"#;


pub(crate) struct BloomRenderer {
    highlight_program: u32,
    vao: u32,
    scene_location: i32,
    threshold_location: i32,
}


impl BloomRenderer {

    pub(crate) fn new() -> Result<Self, String> {

        let highlight_program =
            crate::compile_shader::build_program(
                BLOOM_VERTEX_SHADER,
                BLOOM_HIGHLIGHT_FRAGMENT_SHADER,
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to build Bloom highlight-extraction program: {}",
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
                    highlight_program
                );
            }

            return Err(
                "OpenGL failed to allocate the Bloom vertex array"
                    .to_string()
            );
        }

        let scene_location =
            unsafe {
                gl::GetUniformLocation(
                    highlight_program,
                    b"uScene\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let threshold_location =
            unsafe {
                gl::GetUniformLocation(
                    highlight_program,
                    b"uThreshold\0"
                        .as_ptr()
                        .cast(),
                )
            };

        if scene_location == -1
            || threshold_location == -1
        {
            unsafe {
                gl::DeleteVertexArrays(
                    1,
                    &vao,
                );

                gl::DeleteProgram(
                    highlight_program
                );
            }

            return Err(
                "Bloom highlight-extraction program is missing a required uniform"
                    .to_string()
            );
        }

        Ok(
            Self {
                highlight_program,
                vao,
                scene_location,
                threshold_location,
            }
        )
    }


    /// Draw only pixels whose luminance exceeds the current highlight
    /// threshold.  Checkpoint 3 presents this output directly as a diagnostic;
    /// later checkpoints will feed it into reduced-resolution blur targets.
    pub(crate) fn render_highlights(
        &self,
        scene_texture: u32,
        threshold: f32,
    ) {

        unsafe {
            gl::UseProgram(
                self.highlight_program
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
                self.threshold_location,
                threshold,
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


impl Drop for BloomRenderer {

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

            if self.highlight_program != 0 {
                gl::DeleteProgram(
                    self.highlight_program
                );
            }
        }

        self.vao =
            0;

        self.highlight_program =
            0;
    }
}

