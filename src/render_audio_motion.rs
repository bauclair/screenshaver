#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AudioMotionEffect {
    #[default]
    Off,
    WooferFromHell,
}

impl AudioMotionEffect {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "woofer_from_hell" | "woofer-from-hell" | "woofer from hell" => Ok(Self::WooferFromHell),
            other => Err(format!("Unsupported Audio Motion effect '{}'; supported values: off, woofer_from_hell", other)),
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::WooferFromHell => "Woofer from Hell",
        }
    }

    pub fn database_name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::WooferFromHell => "woofer_from_hell",
        }
    }

    pub fn is_enabled(self) -> bool {
        !matches!(self, Self::Off)
    }
}

const TRANSIENT_BASELINE_RISE_SECONDS: f32 = 0.18;
const TRANSIENT_BASELINE_FALL_SECONDS: f32 = 0.08;
const TRANSIENT_NOISE_FLOOR: f32 = 0.006;
const TRANSIENT_GAIN: f32 = 4.50;
const TRANSIENT_DISPLACEMENT_GAIN: f32 = 1.00;
const CONTINUOUS_DISPLACEMENT_GAIN: f32 = 0.10;
const RELEASE_SECONDS: f32 = 0.075;
const MAX_DISPLACEMENT: f32 = 1.0;
const MAX_SCALE_EXPANSION: f32 = 2.20;
const MOTION_NOISE_FLOOR: f32 = 0.035;
const MOTION_RESPONSE_CURVE: f32 = 0.90;
const FILTER_TRANSITION_ATTACK: f32 = 0.32;
const FILTER_TRANSITION_RELEASE: f32 = 0.20;

#[derive(Clone, Copy, Debug)]
pub struct AudioMotionState {
    displacement: f32,
    scale: f32,
    filter_blend: f32,
    transient_baseline: f32,
}

impl Default for AudioMotionState {
    fn default() -> Self {
        Self { displacement: 0.0, scale: 1.0, filter_blend: 0.0, transient_baseline: 0.0 }
    }
}

impl AudioMotionState {
    fn normalized_motion(value: f32) -> f32 {
        (((value.clamp(0.0, 1.0) - MOTION_NOISE_FLOOR) / (1.0 - MOTION_NOISE_FLOOR))
            .clamp(0.0, 1.0))
            .powf(MOTION_RESPONSE_CURVE)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn update(
        &mut self,
        spectrum: crate::analyze_audio::AudioMotionSpectrum,
        frame_seconds: f32,
        vocal_timing: &crate::manage_lyrics::VocalTimingState,
    ) -> f32 {
        let filter_target = if vocal_timing.available {
            if vocal_timing.active { 1.0 } else { 0.0 }
        } else {
            self.filter_blend
        };

        let nominal_frames = (frame_seconds * 60.0).max(0.0);
        let filter_base_alpha = if filter_target > self.filter_blend {
            FILTER_TRANSITION_ATTACK
        } else {
            FILTER_TRANSITION_RELEASE
        };
        let filter_alpha = 1.0 - (1.0 - filter_base_alpha).powf(nominal_frames);
        self.filter_blend += (filter_target - self.filter_blend) * filter_alpha;
        self.filter_blend = self.filter_blend.clamp(0.0, 1.0);

        let inside = Self::normalized_motion(spectrum.inside_1k_3k);
        let outside = Self::normalized_motion(spectrum.outside_1k_3k);
        let selected = if vocal_timing.available {
            outside + (inside - outside) * self.filter_blend
        } else {
            inside.max(outside)
        }
        .clamp(0.0, 1.0);

        let dt = frame_seconds.clamp(0.0, 1.0 / 30.0);
        let baseline_seconds = if selected > self.transient_baseline {
            TRANSIENT_BASELINE_RISE_SECONDS
        } else {
            TRANSIENT_BASELINE_FALL_SECONDS
        };
        let baseline_alpha = if baseline_seconds > 0.0 {
            1.0 - (-dt / baseline_seconds).exp()
        } else {
            1.0
        };
        self.transient_baseline += (selected - self.transient_baseline) * baseline_alpha;
        self.transient_baseline = self.transient_baseline.clamp(0.0, 1.0);

        let transient_raw = (selected - self.transient_baseline).max(0.0);
        let transient = ((transient_raw - TRANSIENT_NOISE_FLOOR).max(0.0) * TRANSIENT_GAIN)
            .clamp(0.0, 1.0);
        let target_displacement =
            (transient * TRANSIENT_DISPLACEMENT_GAIN + selected * CONTINUOUS_DISPLACEMENT_GAIN)
                .clamp(0.0, MAX_DISPLACEMENT);

        if target_displacement >= self.displacement {
            self.displacement = target_displacement;
        } else {
            let release_alpha = if RELEASE_SECONDS > 0.0 {
                1.0 - (-dt / RELEASE_SECONDS).exp()
            } else {
                1.0
            };
            self.displacement += (target_displacement - self.displacement) * release_alpha;
        }

        self.displacement = self.displacement.clamp(0.0, MAX_DISPLACEMENT);
        self.scale = 1.0 + self.displacement * MAX_SCALE_EXPANSION;
        self.scale
    }
}

const FRAGMENT_SHADER: &str = r#"#version 330 core
out vec4 FragColor;
uniform sampler2D uScene;
uniform vec2 uResolution;
uniform float uScale;
void main() {
    vec2 uv = gl_FragCoord.xy / uResolution;
    vec2 centered = uv - vec2(0.5);
    float aspect = uResolution.x / uResolution.y;
    vec2 conePosition = centered;
    conePosition.x *= aspect;
    float radius = length(conePosition);
    float normalizedRadius = clamp(radius / 0.5, 0.0, 1.0);
    float coneWeight = 1.0 - smoothstep(0.0, 1.0, normalizedRadius);
    float expansion = max(uScale - 1.0, 0.0);
    float coneExponent = mix(0.90, 2.60, clamp(expansion / 4.0, 0.0, 1.0));
    coneWeight = pow(coneWeight, coneExponent);
    float localExpansion = expansion * coneWeight
        + 0.35 * expansion * expansion * coneWeight * coneWeight;
    float localScale = 1.0 + localExpansion;
    vec2 sampleUv = centered / localScale + vec2(0.5);
    sampleUv = clamp(sampleUv, vec2(0.001), vec2(0.999));
    FragColor = texture(uScene, sampleUv);
}
"#;

pub struct AudioMotionRenderer {
    program: u32,
    scene_location: i32,
    resolution_location: i32,
    scale_location: i32,
}

impl AudioMotionRenderer {
    pub fn new() -> Result<Self, String> {
        let program = crate::compile_shader::build_program(
            crate::define_constants::VERTEX_SHADER,
            FRAGMENT_SHADER,
        )?;
        let scene_location = uniform_location(program, "uScene")?;
        let resolution_location = uniform_location(program, "uResolution")?;
        let scale_location = uniform_location(program, "uScale")?;
        Ok(Self { program, scene_location, resolution_location, scale_location })
    }

    pub fn render(&self, source_texture: u32, width: u32, height: u32, scale: f32) {
        unsafe {
            gl::UseProgram(self.program);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, source_texture);
            gl::Uniform1i(self.scene_location, 0);
            gl::Uniform2f(self.resolution_location, width as f32, height as f32);
            gl::Uniform1f(self.scale_location, scale.max(1.0));
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::UseProgram(0);
        }
    }
}

impl Drop for AudioMotionRenderer {
    fn drop(&mut self) {
        if self.program != 0 {
            unsafe { gl::DeleteProgram(self.program); }
            self.program = 0;
        }
    }
}

fn uniform_location(program: u32, name: &str) -> Result<i32, String> {
    let c_name = std::ffi::CString::new(name).map_err(|_| format!("Invalid uniform name: {}", name))?;
    let location = unsafe { gl::GetUniformLocation(program, c_name.as_ptr()) };
    if location < 0 {
        Err(format!("Audio Motion shader uniform '{}' was not found", name))
    } else {
        Ok(location)
    }
}
