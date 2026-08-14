//! Bloom post-processing definitions and renderer support.
//!
//! Checkpoint 5 adds final additive Bloom composition. Highlight extraction
//! and reduced-resolution separable blur are combined with the normally
//! presented scene, scaled by the resolved Bloom intensity.

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

    Audio,
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

            "audio" => {
                Ok(
                    Self::Audio
                )
            }

            other => {
                Err(
                    format!(
                        "Unsupported bloom mode '{}'; supported values: off, highlight, audio",
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
            Self::Audio => "audio",
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


const BLOOM_AUDIO_FRAGMENT_SHADER: &str = r#"
#version 330 core

uniform sampler2D uScene;
uniform float uThreshold;
uniform float uSyntheticPhase;

in vec2 vUv;

out vec4 fragColor;

vec3 rgbToHsv(vec3 c)
{
    float maxChannel = max(c.r, max(c.g, c.b));
    float minChannel = min(c.r, min(c.g, c.b));
    float chroma = maxChannel - minChannel;

    float hue = 0.0;

    if (chroma > 0.00001) {
        if (maxChannel == c.r) {
            hue = mod((c.g - c.b) / chroma, 6.0);
        } else if (maxChannel == c.g) {
            hue = ((c.b - c.r) / chroma) + 2.0;
        } else {
            hue = ((c.r - c.g) / chroma) + 4.0;
        }

        hue *= 60.0;

        if (hue < 0.0) {
            hue += 360.0;
        }
    }

    float saturation =
        maxChannel > 0.00001
            ? chroma / maxChannel
            : 0.0;

    return vec3(
        hue,
        saturation,
        maxChannel
    );
}

void main()
{
    vec3 sceneColor =
        texture(
            uScene,
            vUv
        ).rgb;

    vec3 hsv =
        rgbToHsv(
            max(
                sceneColor,
                vec3(0.0)
            )
        );

    float hue = hsv.x;
    float saturation = hsv.y;
    float value = hsv.z;

    // Synthetic Audio Bloom checkpoint 2:
    // three slowly phase-shifted envelopes verify that each color family can
    // brighten and fade independently before live audio capture/FFT is added.
    const float tau = 6.28318530718;

    float bassEnergy;
    float midEnergy;
    float highEnergy;

    if (uSyntheticPhase < 0.0) {
        // Control Center Ctrl diagnostic: show all eligible color families at
        // full extraction strength, independent of synthetic modulation.
        bassEnergy = 1.0;
        midEnergy = 1.0;
        highEnergy = 1.0;
    } else {
        bassEnergy =
            0.15
            + 0.85
                * (0.5 + 0.5 * sin(tau * uSyntheticPhase));

        midEnergy =
            0.15
            + 0.85
                * (0.5 + 0.5 * sin(tau * (uSyntheticPhase - 0.3333333)));

        highEnergy =
            0.15
            + 0.85
                * (0.5 + 0.5 * sin(tau * (uSyntheticPhase - 0.6666667)));
    }

    // Bass: red through orange.
    float bassMatch =
        (hue >= 0.0 && hue < 45.0)
            ? bassEnergy
            : 0.0;

    // Midrange: yellow through green.
    float midMatch =
        (hue >= 45.0 && hue < 150.0)
            ? midEnergy
            : 0.0;

    // High frequencies: indigo through purple.
    float highMatch =
        (hue >= 240.0 && hue < 300.0)
            ? highEnergy
            : 0.0;

    float bandMatch =
        max(
            bassMatch,
            max(
                midMatch,
                highMatch
            )
        );

    // In Audio mode Bloom Threshold measures color participation rather than
    // luminance. Saturation is mapped to the existing 0.0-2.0 threshold
    // range. A small value factor prevents nearly-black pixels from blooming
    // merely because their mathematical hue falls inside a target band.
    float colorStrength =
        saturation
            * 2.0
            * smoothstep(
                0.02,
                0.15,
                value
            );

    float response =
        bandMatch
            * smoothstep(
                uThreshold,
                min(
                    uThreshold + 0.20,
                    2.0001
                ),
                colorStrength
            );

    fragColor = vec4(
        sceneColor * response,
        1.0
    );
}
"#;


const BLOOM_BLUR_FRAGMENT_SHADER: &str = r#"
#version 330 core

uniform sampler2D uSource;
uniform vec2 uTexelStep;

in vec2 vUv;

out vec4 fragColor;

void main()
{
    const float w0 = 0.2270270270;
    const float w1 = 0.1945945946;
    const float w2 = 0.1216216216;
    const float w3 = 0.0540540541;
    const float w4 = 0.0162162162;

    vec3 color =
        texture(uSource, vUv).rgb * w0;

    color += texture(uSource, vUv + uTexelStep * 1.0).rgb * w1;
    color += texture(uSource, vUv - uTexelStep * 1.0).rgb * w1;

    color += texture(uSource, vUv + uTexelStep * 2.0).rgb * w2;
    color += texture(uSource, vUv - uTexelStep * 2.0).rgb * w2;

    color += texture(uSource, vUv + uTexelStep * 3.0).rgb * w3;
    color += texture(uSource, vUv - uTexelStep * 3.0).rgb * w3;

    color += texture(uSource, vUv + uTexelStep * 4.0).rgb * w4;
    color += texture(uSource, vUv - uTexelStep * 4.0).rgb * w4;

    fragColor = vec4(
        color,
        1.0
    );
}
"#;


const BLOOM_COMPOSITE_FRAGMENT_SHADER: &str = r#"
#version 330 core

uniform sampler2D uScene;
uniform sampler2D uBloom;
uniform float uIntensity;

in vec2 vUv;

out vec4 fragColor;

void main()
{
    vec3 sceneColor =
        texture(
            uScene,
            vUv
        ).rgb;

    vec3 bloomColor =
        texture(
            uBloom,
            vUv
        ).rgb;

    fragColor = vec4(
        sceneColor
            + bloomColor * uIntensity,
        1.0
    );
}
"#;


pub(crate) struct BloomRenderer {
    highlight_program: u32,
    audio_program: u32,
    blur_program: u32,
    composite_program: u32,
    vao: u32,
    scene_location: i32,
    threshold_location: i32,
    audio_scene_location: i32,
    audio_threshold_location: i32,
    audio_synthetic_phase_location: i32,
    blur_source_location: i32,
    blur_texel_step_location: i32,
    composite_scene_location: i32,
    composite_bloom_location: i32,
    composite_intensity_location: i32,
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


        let audio_program =
            crate::compile_shader::build_program(
                BLOOM_VERTEX_SHADER,
                BLOOM_AUDIO_FRAGMENT_SHADER,
            )
            .map_err(
                |error| {
                    unsafe {
                        gl::DeleteProgram(
                            highlight_program
                        );
                    }

                    format!(
                        "Unable to build Bloom audio color-extraction program: {}",
                        error,
                    )
                }
            )?;


        let blur_program =
            crate::compile_shader::build_program(
                BLOOM_VERTEX_SHADER,
                BLOOM_BLUR_FRAGMENT_SHADER,
            )
            .map_err(
                |error| {
                    unsafe {
                        gl::DeleteProgram(
                            highlight_program
                        );

                        gl::DeleteProgram(
                            audio_program
                        );
                    }

                    format!(
                        "Unable to build Bloom blur program: {}",
                        error,
                    )
                }
            )?;


        let composite_program =
            crate::compile_shader::build_program(
                BLOOM_VERTEX_SHADER,
                BLOOM_COMPOSITE_FRAGMENT_SHADER,
            )
            .map_err(
                |error| {
                    unsafe {
                        gl::DeleteProgram(
                            highlight_program
                        );

                        gl::DeleteProgram(
                            audio_program
                        );

                        gl::DeleteProgram(
                            blur_program
                        );
                    }

                    format!(
                        "Unable to build Bloom composition program: {}",
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

                gl::DeleteProgram(
                    audio_program
                );

                gl::DeleteProgram(
                    blur_program
                );

                gl::DeleteProgram(
                    composite_program
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


        let audio_scene_location =
            unsafe {
                gl::GetUniformLocation(
                    audio_program,
                    b"uScene\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let audio_threshold_location =
            unsafe {
                gl::GetUniformLocation(
                    audio_program,
                    b"uThreshold\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let audio_synthetic_phase_location =
            unsafe {
                gl::GetUniformLocation(
                    audio_program,
                    b"uSyntheticPhase\0"
                        .as_ptr()
                        .cast(),
                )
            };


        let blur_source_location =
            unsafe {
                gl::GetUniformLocation(
                    blur_program,
                    b"uSource\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let blur_texel_step_location =
            unsafe {
                gl::GetUniformLocation(
                    blur_program,
                    b"uTexelStep\0"
                        .as_ptr()
                        .cast(),
                )
            };


        let composite_scene_location =
            unsafe {
                gl::GetUniformLocation(
                    composite_program,
                    b"uScene\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let composite_bloom_location =
            unsafe {
                gl::GetUniformLocation(
                    composite_program,
                    b"uBloom\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let composite_intensity_location =
            unsafe {
                gl::GetUniformLocation(
                    composite_program,
                    b"uIntensity\0"
                        .as_ptr()
                        .cast(),
                )
            };

        if scene_location == -1
            || threshold_location == -1
            || audio_scene_location == -1
            || audio_threshold_location == -1
            || audio_synthetic_phase_location == -1
            || blur_source_location == -1
            || blur_texel_step_location == -1
            || composite_scene_location == -1
            || composite_bloom_location == -1
            || composite_intensity_location == -1
        {
            unsafe {
                gl::DeleteVertexArrays(
                    1,
                    &vao,
                );

                gl::DeleteProgram(
                    highlight_program
                );

                gl::DeleteProgram(
                    audio_program
                );

                gl::DeleteProgram(
                    blur_program
                );

                gl::DeleteProgram(
                    composite_program
                );
            }

            return Err(
                "Bloom post-processing program is missing a required uniform"
                    .to_string()
            );
        }

        Ok(
            Self {
                highlight_program,
                audio_program,
                blur_program,
                composite_program,
                vao,
                scene_location,
                threshold_location,
                audio_scene_location,
                audio_threshold_location,
                audio_synthetic_phase_location,
                blur_source_location,
                blur_texel_step_location,
                composite_scene_location,
                composite_bloom_location,
                composite_intensity_location,
            }
        )
    }


    /// Draw only pixels whose luminance exceeds the current highlight
    /// threshold. The result becomes the source for the reduced-resolution
    /// Bloom blur chain.
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


    /// Draw pixels belonging to the three synthetic Audio Bloom hue bands.
    /// Checkpoint 1 holds bass, midrange, and high-frequency energy at 1.0 so
    /// extraction can be evaluated independently of live audio analysis.
    pub(crate) fn render_audio_colors(
        &self,
        scene_texture: u32,
        threshold: f32,
        synthetic_phase: f32,
    ) {

        unsafe {
            gl::UseProgram(
                self.audio_program
            );

            gl::ActiveTexture(
                gl::TEXTURE0
            );

            gl::BindTexture(
                gl::TEXTURE_2D,
                scene_texture,
            );

            gl::Uniform1i(
                self.audio_scene_location,
                0,
            );

            gl::Uniform1f(
                self.audio_threshold_location,
                threshold,
            );

            gl::Uniform1f(
                self.audio_synthetic_phase_location,
                synthetic_phase,
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


    pub(crate) fn render_blur(
        &self,
        source_texture: u32,
        texel_step_x: f32,
        texel_step_y: f32,
    ) {

        unsafe {
            gl::UseProgram(
                self.blur_program
            );

            gl::ActiveTexture(
                gl::TEXTURE0
            );

            gl::BindTexture(
                gl::TEXTURE_2D,
                source_texture,
            );

            gl::Uniform1i(
                self.blur_source_location,
                0,
            );

            gl::Uniform2f(
                self.blur_texel_step_location,
                texel_step_x,
                texel_step_y,
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


    /// Add the blurred Bloom contribution to the normally presented scene.
    pub(crate) fn render_composite(
        &self,
        scene_texture: u32,
        bloom_texture: u32,
        intensity: f32,
    ) {

        unsafe {
            gl::UseProgram(
                self.composite_program
            );

            gl::ActiveTexture(
                gl::TEXTURE0
            );

            gl::BindTexture(
                gl::TEXTURE_2D,
                scene_texture,
            );

            gl::Uniform1i(
                self.composite_scene_location,
                0,
            );

            gl::ActiveTexture(
                gl::TEXTURE1
            );

            gl::BindTexture(
                gl::TEXTURE_2D,
                bloom_texture,
            );

            gl::Uniform1i(
                self.composite_bloom_location,
                1,
            );

            gl::Uniform1f(
                self.composite_intensity_location,
                intensity,
            );

            gl::BindVertexArray(
                self.vao
            );

            gl::DrawArrays(
                gl::TRIANGLES,
                0,
                3,
            );

            gl::ActiveTexture(
                gl::TEXTURE1
            );

            gl::BindTexture(
                gl::TEXTURE_2D,
                0,
            );

            gl::ActiveTexture(
                gl::TEXTURE0
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

            if self.audio_program != 0 {
                gl::DeleteProgram(
                    self.audio_program
                );
            }

            if self.blur_program != 0 {
                gl::DeleteProgram(
                    self.blur_program
                );
            }

            if self.composite_program != 0 {
                gl::DeleteProgram(
                    self.composite_program
                );
            }
        }

        self.vao =
            0;

        self.highlight_program =
            0;

        self.audio_program =
            0;

        self.blur_program =
            0;

        self.composite_program =
            0;
    }
}

