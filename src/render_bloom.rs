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

pub(crate) const BLOOM_SATURATION_MIN: f32 =
    1.0;

pub(crate) const BLOOM_SATURATION_MAX: f32 =
    2.0;

pub(crate) const BLOOM_SATURATION_DEFAULT: f32 =
    1.0;

pub(crate) const BLOOM_THRESHOLD_MIN: f32 =
    0.0;

pub(crate) const BLOOM_THRESHOLD_MAX: f32 =
    2.0;

pub(crate) const BLOOM_THRESHOLD_DEFAULT: f32 =
    0.80;

pub(crate) const BLOOM_FREQUENCY_ROTATION_MIN: f32 =
    0.0;

pub(crate) const BLOOM_FREQUENCY_ROTATION_MAX: f32 =
    360.0;

pub(crate) const BLOOM_FREQUENCY_ROTATION_DEFAULT: f32 =
    0.0;

pub(crate) const BLOOM_FREQUENCY_INVERT_DEFAULT: bool =
    false;


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

    Audio,

    Spectral,

    Loudness,
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

            "audio" => {
                Ok(
                    Self::Audio
                )
            }

            "spectral" => {
                Ok(
                    Self::Spectral
                )
            }

            "loudness" => {
                Ok(
                    Self::Loudness
                )
            }

            other => {
                Err(
                    format!(
                        "Unsupported bloom mode '{}'; supported values: off, audio, spectral, loudness",
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
            Self::Audio => "audio",
            Self::Spectral => "spectral",
            Self::Loudness => "loudness",
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

pub(crate) fn validate_bloom_saturation(
    value: f32,
) -> Result<f32, String> {

    if value.is_finite()
        && (BLOOM_SATURATION_MIN
            ..=BLOOM_SATURATION_MAX)
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
            "Bloom saturation {} is outside the supported range {:.2}-{:.2}",
            value,
            BLOOM_SATURATION_MIN,
            BLOOM_SATURATION_MAX,
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


pub(crate) fn validate_bloom_frequency_rotation(
    value: f32,
) -> Result<f32, String> {

    if value.is_finite()
        && (BLOOM_FREQUENCY_ROTATION_MIN
            ..=BLOOM_FREQUENCY_ROTATION_MAX)
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
            "Bloom frequency rotation {} is outside the supported range {:.1}-{:.1} degrees",
            value,
            BLOOM_FREQUENCY_ROTATION_MIN,
            BLOOM_FREQUENCY_ROTATION_MAX,
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
uniform float uSaturation;
uniform vec3 uAudioBands;
uniform float uFrequencyRotation;
uniform int uFrequencyInvert;
uniform int uDiagnostic;

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

vec3 boostSaturation(vec3 color, float amount)
{
    // Preserve the brightest channel (HSV value) while pushing the other
    // channels away from it. 1.0 is exactly neutral. At 2.0 the available
    // chroma is expanded by up to 3x, which is deliberately stronger than a
    // conventional saturation multiplier so the Control Center experiment
    // produces a clearly visible change on moderately saturated bloom.
    float maxChannel = max(color.r, max(color.g, color.b));
    float minChannel = min(color.r, min(color.g, color.b));
    float chroma = maxChannel - minChannel;

    if (chroma <= 0.00001 || maxChannel <= 0.00001) {
        return max(color, vec3(0.0));
    }

    float strength =
        mix(
            1.0,
            3.0,
            clamp(amount - 1.0, 0.0, 1.0)
        );

    float targetChroma =
        min(
            maxChannel,
            chroma * strength
        );

    float scale = targetChroma / chroma;

    return clamp(
        vec3(maxChannel)
            + (color - vec3(maxChannel)) * scale,
        vec3(0.0),
        vec3(maxChannel)
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

    // Frequency Mapping operates only on the hue used for Audio Bloom
    // classification. It never alters sceneColor.
    //
    // Invert Frequency Mapping reverses the direction of the hue-to-frequency
    // map itself rather than merely exchanging two band amplitudes. The
    // reflection axis is chosen so that, at zero Frequency Rotation, the
    // historical red/orange bass region maps to the high-frequency end while
    // the historical indigo/purple high-frequency region maps to bass. This
    // makes inversion perceptually meaningful even when independently
    // normalized live bass and treble amplitudes happen to be similar.
    //
    // Frequency Rotation is applied after that optional reflection, so it
    // remains an independent positional control. Inversion chooses mapping
    // direction; Frequency Rotation chooses where that mapping sits on the
    // hue wheel.
    const float FREQUENCY_INVERT_REFLECTION_DEGREES = 292.5;

    float mappedHue =
        uFrequencyInvert != 0
            ? FREQUENCY_INVERT_REFLECTION_DEGREES - hue
            : hue;

    mappedHue =
        mod(
            mappedHue + uFrequencyRotation + 360.0,
            360.0
        );

    // Live Audio Bloom modulation. The analyzer publishes already-normalized
    // and attack/release-smoothed energy for the three frequency bands.
    // These values drive ordinary Audio Bloom. The Control Center diagnostic
    // bypasses audio-band participation later so Ctrl can show the raw Bloom
    // Threshold extraction.
    float bassEnergy =
        clamp(uAudioBands.x, 0.0, 1.0);

    float midEnergy =
        clamp(uAudioBands.y, 0.0, 1.0);

    float highEnergy =
        clamp(uAudioBands.z, 0.0, 1.0);

    // The historical band windows remain unchanged. Inversion changes the hue
    // coordinate presented to those windows, which reverses the complete
    // color-to-frequency map rather than just swapping live amplitudes.
    float bassMatch =
        (mappedHue >= 0.0 && mappedHue < 45.0)
            ? bassEnergy
            : 0.0;

    float midMatch =
        (mappedHue >= 45.0 && mappedHue < 150.0)
            ? midEnergy
            : 0.0;

    float highMatch =
        (mappedHue >= 240.0 && mappedHue < 300.0)
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

    float energy =
        clamp(
            bandMatch,
            0.0,
            1.0
        );

    // Audio energy controls both extraction strength and participation.
    // At low energy, the effective threshold is pushed toward the top of the
    // supported color-strength range, so only the strongest matching colors
    // can contribute. As energy rises, the effective threshold moves smoothly
    // toward the user's configured Bloom Threshold, progressively admitting
    // more eligible pixels.
    float effectiveThreshold =
        mix(
            2.0,
            uThreshold,
            energy
        );

    float response;

    if (uDiagnostic != 0) {
        // Control Center Bloom Threshold diagnostic. Show the raw Audio Bloom
        // color-participation extraction at the configured threshold without
        // live audio energy or frequency-band gating. This makes Ctrl a direct
        // preview of the Bloom Threshold slider while leaving ordinary Audio
        // Bloom behavior unchanged.
        response =
            smoothstep(
                uThreshold,
                min(
                    uThreshold + 0.35,
                    2.0001
                ),
                colorStrength
            );
    } else {
        response =
            energy
                * smoothstep(
                    effectiveThreshold,
                    min(
                        effectiveThreshold + 0.35,
                        2.0001
                    ),
                    colorStrength
                );
    }

    vec3 bloomColor =
        boostSaturation(
            sceneColor,
            uSaturation
        );

    fragColor = vec4(
        bloomColor * response,
        1.0
    );
}
"#;


const BLOOM_SPECTRAL_FRAGMENT_SHADER: &str = r#"
#version 330 core

uniform sampler2D uScene;
uniform float uThreshold;
uniform float uSaturation;
uniform vec3 uAudioBands;
uniform float uDominantFrequencyHz;
uniform float uFrequencyRotation;
uniform int uFrequencyInvert;
uniform int uDiagnostic;

in vec2 vUv;

out vec4 fragColor;

vec3 hueToRgb(float hueDegrees)
{
    float hue = mod(hueDegrees + 360.0, 360.0) / 60.0;
    float x = 1.0 - abs(mod(hue, 2.0) - 1.0);

    if (hue < 1.0) {
        return vec3(1.0, x, 0.0);
    }
    if (hue < 2.0) {
        return vec3(x, 1.0, 0.0);
    }
    if (hue < 3.0) {
        return vec3(0.0, 1.0, x);
    }
    if (hue < 4.0) {
        return vec3(0.0, x, 1.0);
    }
    if (hue < 5.0) {
        return vec3(x, 0.0, 1.0);
    }

    return vec3(1.0, 0.0, x);
}

float frequencyToHue(float frequencyHz)
{
    // Musical pitch and human frequency perception are logarithmic. Map the
    // useful analysis range across the historical Audio Bloom color arc:
    // low frequencies begin at red/orange and high frequencies end at violet.
    const float MIN_HZ = 40.0;
    const float MAX_HZ = 12000.0;
    const float LOW_HUE = 0.0;
    const float HIGH_HUE = 285.0;

    float clampedFrequency =
        clamp(
            frequencyHz,
            MIN_HZ,
            MAX_HZ
        );

    float position =
        log2(
            clampedFrequency / MIN_HZ
        )
        / log2(
            MAX_HZ / MIN_HZ
        );

    return mix(
        LOW_HUE,
        HIGH_HUE,
        clamp(position, 0.0, 1.0)
    );
}

float mappedFrequencyHue(float baseHue)
{
    // In Spectral mode inversion reverses the low-to-high color progression.
    // Frequency Rotation is then applied as a conventional hue-wheel offset.
    float mappedHue =
        uFrequencyInvert != 0
            ? 285.0 - baseHue
            : baseHue;

    return mod(
        mappedHue + uFrequencyRotation + 360.0,
        360.0
    );
}

void main()
{
    vec3 sceneColor =
        texture(
            uScene,
            vUv
        ).rgb;

    // Source RGB supplies structure only. Spectral Bloom deliberately ignores
    // source hue so white, gray, and low-saturation shaders can bloom.
    float luminance =
        dot(
            max(sceneColor, vec3(0.0)),
            vec3(0.2126, 0.7152, 0.0722)
        );

    float energy =
        clamp(
            max(
                uAudioBands.x,
                max(
                    uAudioBands.y,
                    uAudioBands.z
                )
            ),
            0.0,
            1.0
        );

    float dominantFrequencyHz =
        uDominantFrequencyHz;

    if (uDiagnostic != 0) {
        // Deterministic 110-Hz test tone: Ctrl shows the complete extraction
        // mask using a stable low-frequency spectral color.
        dominantFrequencyHz = 110.0;
        energy = 1.0;
    }

    if (dominantFrequencyHz <= 0.0
        || energy <= 0.0001)
    {
        fragColor =
            vec4(
                0.0,
                0.0,
                0.0,
                1.0
            );

        return;
    }

    float spectralHue =
        mappedFrequencyHue(
            frequencyToHue(
                dominantFrequencyHz
            )
        );

    // The FFT-derived hue is intentionally generated as a pure spectral color.
    // There is no bass/mid/treble RGB averaging step, so simultaneous energy in
    // several bands cannot drive the bloom toward white.
    vec3 spectralColor =
        hueToRgb(
            spectralHue
        );

    // Keep Bloom Saturation wired for the Control Center experiment without
    // allowing it to wash the FFT-derived color toward white. Because the
    // generated hue is already fully saturated, values above 1.0 primarily act
    // as a small chroma-preserving color emphasis rather than another mixer.
    float saturationEmphasis =
        mix(
            1.0,
            1.20,
            clamp(
                uSaturation - 1.0,
                0.0,
                1.0
            )
        );

    spectralColor =
        min(
            spectralColor
                * saturationEmphasis,
            vec3(1.0)
        );

    // Bloom Threshold retains its Spectral meaning: source luminance
    // eligibility on the established 0.0-2.0 scale.
    float brightnessStrength =
        luminance * 2.0;

    float effectiveThreshold =
        mix(
            2.0,
            uThreshold,
            energy
        );

    float response =
        energy
            * smoothstep(
                effectiveThreshold,
                min(
                    effectiveThreshold + 0.35,
                    2.0001
                ),
                brightnessStrength
            );

    fragColor = vec4(
        spectralColor
            * response
            * luminance,
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
uniform float uSaturation;
uniform int uSpectral;

in vec2 vUv;

out vec4 fragColor;

float maxChannel(
    vec3 color
)
{
    return max(
        color.r,
        max(
            color.g,
            color.b
        )
    );
}

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

    vec3 additiveColor =
        sceneColor
            + bloomColor * uIntensity;

    if (uSpectral == 0) {
        fragColor = vec4(
            additiveColor,
            1.0
        );

        return;
    }

    float bloomPeak =
        maxChannel(
            bloomColor
        );

    if (bloomPeak <= 0.000001) {
        fragColor = vec4(
            sceneColor,
            1.0
        );

        return;
    }

    // Spectral Bloom is fundamentally different from Audio Bloom:
    // its FFT-derived hue should dominate the bloom region instead
    // of being additively washed toward white by a bright source.
    //
    // Normalize the blurred bloom back to its spectral hue while
    // preserving its spatial magnitude separately.
    vec3 spectralHue =
        clamp(
            bloomColor / bloomPeak,
            0.0,
            1.0
        );

    float bloomDrive =
        max(
            bloomPeak * uIntensity,
            0.0
        );

    // Preserve the useful brightness of the traditional additive
    // result, but express that brightness primarily through the
    // FFT-derived spectral hue.
    float targetBrightness =
        maxChannel(
            additiveColor
        );

    vec3 spectralTarget =
        spectralHue
            * targetBrightness;

    // Bloom Saturation controls how aggressively neutral/white
    // source color is replaced by the spectral hue.  Even at 1.0,
    // Spectral Bloom is intentionally strongly colorized.
    float saturationAmount =
        clamp(
            uSaturation - 1.0,
            0.0,
            1.0
        );

    float colorizeGain =
        mix(
            2.5,
            6.0,
            saturationAmount
        );

    float colorizeAmount =
        clamp(
            bloomDrive
                * colorizeGain,
            0.0,
            1.0
        );

    vec3 finalColor =
        mix(
            additiveColor,
            spectralTarget,
            colorizeAmount
        );

    fragColor = vec4(
        finalColor,
        1.0
    );
}
"#;


pub(crate) struct BloomRenderer {
    highlight_program: u32,
    audio_program: u32,
    spectral_program: u32,
    blur_program: u32,
    composite_program: u32,
    vao: u32,
    scene_location: i32,
    threshold_location: i32,
    audio_scene_location: i32,
    audio_threshold_location: i32,
    audio_saturation_location: i32,
    audio_bands_location: i32,
    audio_frequency_rotation_location: i32,
    audio_frequency_invert_location: i32,
    audio_diagnostic_location: i32,
    spectral_scene_location: i32,
    spectral_threshold_location: i32,
    spectral_saturation_location: i32,
    spectral_bands_location: i32,
    spectral_dominant_frequency_location: i32,
    spectral_frequency_rotation_location: i32,
    spectral_frequency_invert_location: i32,
    spectral_diagnostic_location: i32,
    blur_source_location: i32,
    blur_texel_step_location: i32,
    composite_scene_location: i32,
    composite_bloom_location: i32,
    composite_intensity_location: i32,
    composite_saturation_location: i32,
    composite_spectral_location: i32,
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


        let spectral_program =
            crate::compile_shader::build_program(
                BLOOM_VERTEX_SHADER,
                BLOOM_SPECTRAL_FRAGMENT_SHADER,
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
                        "Unable to build Bloom spectral color-extraction program: {}",
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

                        gl::DeleteProgram(
                            spectral_program
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
                            spectral_program
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
                    spectral_program
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

        let audio_saturation_location =
            unsafe {
                gl::GetUniformLocation(
                    audio_program,
                    b"uSaturation\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let audio_bands_location =
            unsafe {
                gl::GetUniformLocation(
                    audio_program,
                    b"uAudioBands\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let audio_frequency_rotation_location =
            unsafe {
                gl::GetUniformLocation(
                    audio_program,
                    b"uFrequencyRotation\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let audio_frequency_invert_location =
            unsafe {
                gl::GetUniformLocation(
                    audio_program,
                    b"uFrequencyInvert\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let audio_diagnostic_location =
            unsafe {
                gl::GetUniformLocation(
                    audio_program,
                    b"uDiagnostic\0"
                        .as_ptr()
                        .cast(),
                )
            };


        let spectral_scene_location =
            unsafe {
                gl::GetUniformLocation(
                    spectral_program,
                    b"uScene\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let spectral_threshold_location =
            unsafe {
                gl::GetUniformLocation(
                    spectral_program,
                    b"uThreshold\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let spectral_saturation_location =
            unsafe {
                gl::GetUniformLocation(
                    spectral_program,
                    b"uSaturation\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let spectral_bands_location =
            unsafe {
                gl::GetUniformLocation(
                    spectral_program,
                    b"uAudioBands\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let spectral_dominant_frequency_location =
            unsafe {
                gl::GetUniformLocation(
                    spectral_program,
                    b"uDominantFrequencyHz\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let spectral_frequency_rotation_location =
            unsafe {
                gl::GetUniformLocation(
                    spectral_program,
                    b"uFrequencyRotation\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let spectral_frequency_invert_location =
            unsafe {
                gl::GetUniformLocation(
                    spectral_program,
                    b"uFrequencyInvert\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let spectral_diagnostic_location =
            unsafe {
                gl::GetUniformLocation(
                    spectral_program,
                    b"uDiagnostic\0"
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

        let composite_saturation_location =
            unsafe {
                gl::GetUniformLocation(
                    composite_program,
                    b"uSaturation\0"
                        .as_ptr()
                        .cast(),
                )
            };

        let composite_spectral_location =
            unsafe {
                gl::GetUniformLocation(
                    composite_program,
                    b"uSpectral\0"
                        .as_ptr()
                        .cast(),
                )
            };

        if scene_location == -1
            || threshold_location == -1
            || audio_scene_location == -1
            || audio_threshold_location == -1
            || audio_saturation_location == -1
            || audio_bands_location == -1
            || audio_frequency_rotation_location == -1
            || audio_frequency_invert_location == -1
            || audio_diagnostic_location == -1
            || spectral_scene_location == -1
            || spectral_threshold_location == -1
            || spectral_saturation_location == -1
            || spectral_bands_location == -1
            || spectral_dominant_frequency_location == -1
            || spectral_frequency_rotation_location == -1
            || spectral_frequency_invert_location == -1
            || spectral_diagnostic_location == -1
            || blur_source_location == -1
            || blur_texel_step_location == -1
            || composite_scene_location == -1
            || composite_bloom_location == -1
            || composite_intensity_location == -1
            || composite_saturation_location == -1
            || composite_spectral_location == -1
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
                    spectral_program
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
                spectral_program,
                blur_program,
                composite_program,
                vao,
                scene_location,
                threshold_location,
                audio_scene_location,
                audio_threshold_location,
                audio_saturation_location,
                audio_bands_location,
                audio_frequency_rotation_location,
                audio_frequency_invert_location,
                audio_diagnostic_location,
                spectral_scene_location,
                spectral_threshold_location,
                spectral_saturation_location,
                spectral_bands_location,
                spectral_dominant_frequency_location,
                spectral_frequency_rotation_location,
                spectral_frequency_invert_location,
                spectral_diagnostic_location,
                blur_source_location,
                blur_texel_step_location,
                composite_scene_location,
                composite_bloom_location,
                composite_intensity_location,
                composite_saturation_location,
                composite_spectral_location,
            }
        )
    }


    /// Diagnostic/internal bright-region extraction retained independently of
    /// the retired user-facing Highlight Bloom mode.
    #[allow(dead_code)]
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


    /// Draw pixels belonging to the three Audio Bloom hue bands, modulated by
    /// the latest normalized and smoothed live audio energy.
    pub(crate) fn render_audio_colors(
        &self,
        scene_texture: u32,
        threshold: f32,
        saturation: f32,
        bands: crate::analyze_audio::AudioBands,
        frequency_rotation: f32,
        frequency_invert: bool,
        diagnostic: bool,
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
                self.audio_saturation_location,
                saturation,
            );

            gl::Uniform3f(
                self.audio_bands_location,
                bands.bass,
                bands.midrange,
                bands.treble,
            );

            gl::Uniform1f(
                self.audio_frequency_rotation_location,
                frequency_rotation,
            );

            gl::Uniform1i(
                self.audio_frequency_invert_location,
                if frequency_invert {
                    1
                } else {
                    0
                },
            );

            gl::Uniform1i(
                self.audio_diagnostic_location,
                if diagnostic {
                    1
                } else {
                    0
                },
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


    /// Extract bright scene structure and color its bloom directly from the
    /// latest normalized and smoothed audio spectrum. Unlike Audio Bloom,
    /// source hue is irrelevant, allowing grayscale shaders to participate.
    pub(crate) fn render_spectral_colors(
        &self,
        scene_texture: u32,
        threshold: f32,
        saturation: f32,
        bands: crate::analyze_audio::AudioBands,
        frequency_rotation: f32,
        frequency_invert: bool,
        diagnostic: bool,
    ) {

        unsafe {
            gl::UseProgram(
                self.spectral_program
            );

            gl::ActiveTexture(
                gl::TEXTURE0
            );

            gl::BindTexture(
                gl::TEXTURE_2D,
                scene_texture,
            );

            gl::Uniform1i(
                self.spectral_scene_location,
                0,
            );

            gl::Uniform1f(
                self.spectral_threshold_location,
                threshold,
            );

            gl::Uniform1f(
                self.spectral_saturation_location,
                saturation,
            );

            gl::Uniform3f(
                self.spectral_bands_location,
                bands.bass,
                bands.midrange,
                bands.treble,
            );

            gl::Uniform1f(
                self.spectral_dominant_frequency_location,
                bands.dominant_frequency_hz,
            );

            gl::Uniform1f(
                self.spectral_frequency_rotation_location,
                frequency_rotation,
            );

            gl::Uniform1i(
                self.spectral_frequency_invert_location,
                if frequency_invert {
                    1
                } else {
                    0
                },
            );

            gl::Uniform1i(
                self.spectral_diagnostic_location,
                if diagnostic {
                    1
                } else {
                    0
                },
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


    /// Map normalized apparent loudness onto the same red-to-violet
    /// color arc used by Spectral Bloom.  Rotation and inversion are
    /// intentionally forced off for Loudness Bloom.
    pub(crate) fn render_loudness_colors(
        &self,
        scene_texture: u32,
        threshold: f32,
        saturation: f32,
        bands: crate::analyze_audio::AudioBands,
        diagnostic: bool,
    ) {

        let loudness =
            if diagnostic {
                0.50
            } else {
                bands.loudness
                    .clamp(
                        0.0,
                        1.0,
                    )
            };

        // The spectral shader maps frequency logarithmically from 40 Hz
        // to 12 kHz onto 0..285 degrees.  Invert that mapping here so a
        // normalized loudness value of 0.0 becomes red and 1.0 violet.
        let pseudo_frequency_hz =
            40.0_f32
                * (
                    12_000.0_f32
                        / 40.0_f32
                )
                    .powf(
                        loudness
                    );

        // Loudness determines the Bloom color, not its visibility.
        //
        // Earlier prototypes also multiplied the extraction energy by
        // loudness.  That made red/orange (quiet) colors nearly disappear
        // exactly when we needed to see them.  Keep the extraction energy
        // at full scale here and let Bloom Intensity / Threshold control
        // the visible strength of the effect.
        let extraction_energy =
            1.0_f32;

        let loudness_bands =
            crate::analyze_audio::AudioBands {
                bass:
                    extraction_energy,

                midrange:
                    extraction_energy,

                treble:
                    extraction_energy,

                dominant_frequency_hz:
                    pseudo_frequency_hz,

                loudness,
            };

        self.render_spectral_colors(
            scene_texture,
            threshold,
            saturation,
            loudness_bands,
            0.0,
            false,
            false,
        );
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
        saturation: f32,
        spectral: bool,
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

            gl::Uniform1f(
                self.composite_saturation_location,
                saturation,
            );

            gl::Uniform1i(
                self.composite_spectral_location,
                if spectral { 1 } else { 0 },
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

            if self.spectral_program != 0 {
                gl::DeleteProgram(
                    self.spectral_program
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

        self.spectral_program =
            0;

        self.blur_program =
            0;

        self.composite_program =
            0;
    }
}

