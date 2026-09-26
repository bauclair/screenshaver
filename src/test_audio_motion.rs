use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;

const TEST_WIDTH: u32 = 1280;
const TEST_HEIGHT: u32 = 720;

// Experimental mirrored multi-channel FFT deformation.
//
// The completed shader image is the visualization: no FFT line or bars are
// drawn over it. Each logarithmic FFT channel locally deforms the image at its
// horizontal position. The same deformation is mirrored above and below the
// vertical center.
const SHADER_SPEED: f32 = 1.0;
const TRACE_CHANNELS: usize = crate::analyze_audio::AUDIO_MOTION_TRACE_CHANNELS;

// Proof-of-concept display range. The shared analyzer remains untouched;
// this harness remaps its logarithmic 20 Hz..20 kHz trace so all 48 displayed
// channels span 100 Hz..12 kHz.
const SOURCE_TRACE_MIN_HZ: f32 = 20.0;
const SOURCE_TRACE_MAX_HZ: f32 = 20_000.0;
const TEST_TRACE_MIN_HZ: f32 = 100.0;
const TEST_TRACE_MAX_HZ: f32 = 3_800.0;

// Per-channel visual peak compression.
//
// Once a frequency channel rises above the observable threshold, compress its
// dynamic range aggressively toward 90% of the available channel height.
// This is intentionally not frame normalization: each of the 48 frequencies
// is treated independently, so simultaneous observable peaks can all approach
// the same visual height.
//
// Values below the threshold remain zero so silence and background noise are
// not promoted into visible full-height motion.
const TRACE_COMPRESSED_PEAK: f32 = 0.90;
const TRACE_COMPRESSION_THRESHOLD: f32 = 0.025;
const TRACE_COMPRESSION_CURVE: f32 = 0.12;
const TRACE_GAIN: f32 = 0.34;
const TRACE_NOISE_FLOOR: f32 = 0.045;
const TRACE_RESPONSE_CURVE: f32 = 0.80;
const TRACE_RELEASE_SECONDS: f32 = 0.085;
const TRACE_SPATIAL_WIDTH: f32 = 0.105;
const DIAGNOSTIC_INTERVAL: Duration = Duration::from_millis(250);

const MIRRORED_FFT_FRAGMENT_SHADER: &str = r#"#version 330 core
out vec4 FragColor;

uniform sampler2D uScene;
uniform vec2 uResolution;
uniform float uSpectrum[48];
uniform float uTraceGain;
uniform float uSpatialWidth;

float spectrumAt(float x) {
    float p = clamp(x, 0.0, 1.0) * 47.0;
    int i0 = int(floor(p));
    int i1 = min(i0 + 1, 47);
    float f = fract(p);

    // Smooth interpolation avoids visible 48-column steps.
    f = f * f * (3.0 - 2.0 * f);
    return mix(uSpectrum[i0], uSpectrum[i1], f);
}

void main() {
    vec2 uv = gl_FragCoord.xy / uResolution;

    float amplitude = spectrumAt(uv.x);
    float excursion = amplitude * uTraceGain;

    // Distance from the horizontal center. The upper and lower halves use the
    // same field, producing a perfect vertical mirror.
    float side = uv.y >= 0.5 ? 1.0 : -1.0;
    float distanceFromCenter = abs(uv.y - 0.5);

    // The instantaneous FFT peak at this X coordinate lives this far from the
    // centerline. Pixels around that peak are what deform most strongly.
    float peakDistance = excursion;

    // Localized ridge around the FFT contour. This is deliberately not a
    // centerline translation: the image under each peak is pulled into that
    // peak while regions away from the contour progressively remain anchored.
    float distanceFromPeak = abs(distanceFromCenter - peakDistance);
    float ridge = exp(
        -(distanceFromPeak * distanceFromPeak)
        / max(2.0 * uSpatialWidth * uSpatialWidth, 0.000001)
    );

    // Give the material between the center and the peak enough coupling to
    // stretch naturally into the ridge instead of tearing or producing a
    // narrow optical line.
    float interior = 1.0 - smoothstep(
        0.0,
        max(peakDistance + uSpatialWidth, 0.0001),
        distanceFromCenter
    );

    float influence = max(ridge, interior * 0.58);

    // Feather the mirror junction around the vertical center.  The original
    // version switched displacement direction abruptly at y=0.5, which made
    // the join read as a sharp horizontal seam.  This blend keeps displacement
    // exactly zero at the center and smoothly reaches full mirrored motion
    // outside a narrow transition zone.
    float centerFeatherWidth = max(uSpatialWidth * 0.72, 0.025);
    float centerFeather = smoothstep(
        0.0,
        centerFeatherWidth,
        distanceFromCenter
    );

    // Inverse-map the displaced image. Upper and lower motion remain exact
    // mirrors, but the center junction now merges continuously.
    float displacement = excursion * influence * centerFeather;
    vec2 sampleUv = uv;
    sampleUv.y -= side * displacement;

    sampleUv = clamp(sampleUv, vec2(0.001), vec2(0.999));
    FragColor = texture(uScene, sampleUv);
}
"#;

struct AudioCaptureGuard;

impl Drop for AudioCaptureGuard {
    fn drop(&mut self) {
        crate::audio_backend::set_audio_required(false);
    }
}

struct MirroredFftState {
    shader_time: f32,
    channels: [f32; TRACE_CHANNELS],
    last_diagnostic: Instant,
}

impl MirroredFftState {
    fn remap_test_frequency_range(
        spectrum: crate::analyze_audio::AudioMotionFftTrace,
    ) -> [f32; TRACE_CHANNELS] {
        let mut remapped = [0.0_f32; TRACE_CHANNELS];

        let source_log_span =
            (SOURCE_TRACE_MAX_HZ / SOURCE_TRACE_MIN_HZ).ln();
        let test_log_span =
            (TEST_TRACE_MAX_HZ / TEST_TRACE_MIN_HZ).ln();

        for output_index in 0..TRACE_CHANNELS {
            let output_fraction =
                output_index as f32 / (TRACE_CHANNELS - 1) as f32;

            let frequency =
                TEST_TRACE_MIN_HZ
                    * (test_log_span * output_fraction).exp();

            let source_fraction =
                (frequency / SOURCE_TRACE_MIN_HZ).ln()
                    / source_log_span;

            let source_position =
                source_fraction.clamp(0.0, 1.0)
                    * (TRACE_CHANNELS - 1) as f32;

            let source_low = source_position.floor() as usize;
            let source_high = (source_low + 1).min(TRACE_CHANNELS - 1);
            let blend = source_position - source_low as f32;

            remapped[output_index] =
                spectrum.channels[source_low]
                    + (spectrum.channels[source_high]
                        - spectrum.channels[source_low])
                        * blend;
        }

        remapped
    }

    fn new() -> Self {
        Self {
            shader_time: 0.0,
            channels: [0.0; TRACE_CHANNELS],
            last_diagnostic: Instant::now(),
        }
    }

    fn normalized(value: f32) -> f32 {
        ((value.clamp(0.0, 1.0) - TRACE_NOISE_FLOOR)
            / (1.0 - TRACE_NOISE_FLOOR))
            .clamp(0.0, 1.0)
            .powf(TRACE_RESPONSE_CURVE)
    }

    fn update(
        &mut self,
        spectrum: crate::analyze_audio::AudioMotionFftTrace,
        frame_seconds: f32,
    ) {
        self.shader_time += frame_seconds * SHADER_SPEED;

        let dt = frame_seconds.clamp(0.0, 1.0 / 30.0);
        let release_alpha = if TRACE_RELEASE_SECONDS > 0.0 {
            1.0 - (-dt / TRACE_RELEASE_SECONDS).exp()
        } else {
            1.0
        };

        let remapped =
            Self::remap_test_frequency_range(
                spectrum
            );

        for index in 0..TRACE_CHANNELS {
            let level =
                Self::normalized(
                    remapped[index]
                );

            let target =
                if level >= TRACE_COMPRESSION_THRESHOLD {
                    // Rebase the observable range to 0..1, then apply a very
                    // shallow power curve. The exponent below 1.0 strongly
                    // compresses differences between audible/observable peaks:
                    // even modest peaks are lifted close to the 90% ceiling,
                    // while stronger peaks retain a small amount of contour.
                    let observable =
                        (
                            (level - TRACE_COMPRESSION_THRESHOLD)
                                / (1.0 - TRACE_COMPRESSION_THRESHOLD)
                        )
                        .clamp(0.0, 1.0);

                    TRACE_COMPRESSED_PEAK
                        * observable.powf(
                            TRACE_COMPRESSION_CURVE
                        )
                } else {
                    0.0
                };

            if target >= self.channels[index] {
                // Preserve fast musical attacks.
                self.channels[index] = target;
            } else {
                self.channels[index] +=
                    (target - self.channels[index]) * release_alpha;
            }

            self.channels[index] = self.channels[index].clamp(0.0, 1.0);
        }

        if self.last_diagnostic.elapsed() >= DIAGNOSTIC_INTERVAL {
            let peak = self.channels.iter().copied().fold(0.0_f32, f32::max);
            let active = self.channels.iter().filter(|&&v| v > 0.05).count();
            println!(
                "[AUDIO MOTION FFT] channels={} active={} peak={:.3} gain={:.3} width={:.3} shader_time={:.2}",
                TRACE_CHANNELS,
                active,
                peak,
                TRACE_GAIN,
                TRACE_SPATIAL_WIDTH,
                self.shader_time,
            );
            self.last_diagnostic = Instant::now();
        }
    }
}

pub fn run(
    shader_argument: &str,
    config: &crate::load_config::Config,
) -> Result<(), String> {
    let shader_path =
        resolve_shader_path(
            shader_argument
        )?;

    let loaded =
        crate::load_shader::load_shader_for_preview(
            &shader_path
        );

    let (
        source,
        shader_name,
        channel_usage,
        shader_inputs,
    ) =
        match loaded {
            crate::load_shader::ShaderLoadResult::Ready {
                source,
                shader_name,
                channel_usage,
                shader_inputs,
                ..
            } => {
                (
                    source,
                    shader_name,
                    channel_usage,
                    shader_inputs,
                )
            }

            crate::load_shader::ShaderLoadResult::Rejected {
                shader_name,
                reasons,
            } => {
                return Err(
                    format!(
                        "Shader '{}' was rejected: {}",
                        shader_name,
                        reasons.join("; "),
                    )
                );
            }

            crate::load_shader::ShaderLoadResult::Unavailable {
                shader_name,
                error,
            } => {
                return Err(
                    format!(
                        "Shader '{}' is unavailable: {}",
                        shader_name,
                        error,
                    )
                );
            }
        };

    println!("Screenshaver Audio Motion Test");
    println!("==============================");
    println!();
    println!("Shader: {}", shader_path.display());
    println!("Processed shader: {}", shader_name);
    println!("Test size: {}x{}", TEST_WIDTH, TEST_HEIGHT);
    println!("Effect: mirrored multi-channel FFT shader deformation");
    println!("FFT channels: {} logarithmic buckets", TRACE_CHANNELS);
    println!(
        "FFT display range: {:.0} Hz..{:.0} Hz",
        TEST_TRACE_MIN_HZ,
        TEST_TRACE_MAX_HZ,
    );
    println!(
        "FFT compressed peak ceiling: {:.0}%",
        TRACE_COMPRESSED_PEAK * 100.0,
    );
    println!(
        "FFT compression threshold: {:.3}",
        TRACE_COMPRESSION_THRESHOLD,
    );
    println!(
        "FFT compression curve: {:.3}",
        TRACE_COMPRESSION_CURVE,
    );
    println!("Trace gain: {:.3}", TRACE_GAIN);
    println!("Trace spatial width: {:.3}", TRACE_SPATIAL_WIDTH);
    println!("No FFT line or bars are drawn; the shader image itself is deformed.");
    println!();
    println!("Play audio to exercise motion. Press Esc or close the window to exit.");
    println!();

    let sdl =
        sdl2::init()
            .map_err(
                |error| {
                    format!(
                        "SDL initialization failed: {}",
                        error
                    )
                }
            )?;

    let video =
        sdl.video()
            .map_err(
                |error| {
                    format!(
                        "SDL video initialization failed: {}",
                        error
                    )
                }
            )?;

    {
        let gl_attr =
            video.gl_attr();

        gl_attr.set_context_profile(
            GLProfile::Core
        );

        gl_attr.set_context_version(
            crate::define_constants::GL_MAJOR,
            crate::define_constants::GL_MINOR,
        );
    }

    let window =
        video
            .window(
                "Screenshaver Audio Motion Test",
                TEST_WIDTH,
                TEST_HEIGHT,
            )
            .position_centered()
            .opengl()
            .build()
            .map_err(
                |error| {
                    format!(
                        "Unable to create Audio Motion test window: {}",
                        error
                    )
                }
            )?;

    let _gl_context =
        window
            .gl_create_context()
            .map_err(
                |error| {
                    format!(
                        "Unable to create Audio Motion OpenGL context: {}",
                        error
                    )
                }
            )?;

    gl::load_with(
        |symbol| {
            video.gl_get_proc_address(
                symbol
            ) as *const _
        }
    );

    // VSync keeps frame-time integration stable and makes this a visual
    // experiment rather than an uncapped rendering benchmark.
    let _ =
        video.gl_set_swap_interval(
            1
        );

    let program =
        crate::compile_shader::build_program(
            crate::define_constants::VERTEX_SHADER,
            &source,
        )
        .map_err(
            |error| {
                format!(
                    "Audio Motion test shader compilation failed: {}",
                    error
                )
            }
        )?;

    let postprocess_program =
        crate::compile_shader::build_program(
            crate::define_constants::VERTEX_SHADER,
            MIRRORED_FFT_FRAGMENT_SHADER,
        )
        .map_err(|error| {
            format!("Audio Motion mirrored-FFT post-process shader compilation failed: {}", error)
        })?;

    let mut scene_fbo = 0_u32;
    let mut scene_texture = 0_u32;

    unsafe {
        gl::GenFramebuffers(1, &mut scene_fbo);
        gl::BindFramebuffer(gl::FRAMEBUFFER, scene_fbo);

        gl::GenTextures(1, &mut scene_texture);
        gl::BindTexture(gl::TEXTURE_2D, scene_texture);
        gl::TexImage2D(
            gl::TEXTURE_2D, 0, gl::RGBA8 as i32,
            TEST_WIDTH as i32, TEST_HEIGHT as i32,
            0, gl::RGBA, gl::UNSIGNED_BYTE, std::ptr::null(),
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D, scene_texture, 0,
        );

        if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
            gl::DeleteTextures(1, &scene_texture);
            gl::DeleteFramebuffers(1, &scene_fbo);
            gl::DeleteProgram(postprocess_program);
            gl::DeleteProgram(program);
            return Err("Audio Motion framebuffer is incomplete.".to_string());
        }
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
    }

    let mut vao =
        0_u32;

    unsafe {
        gl::GenVertexArrays(
            1,
            &mut vao,
        );

        gl::BindVertexArray(
            vao
        );
    }

    let time_location =
        uniform_location(
            program,
            b"iTime\0",
        );

    let resolution_location =
        uniform_location(
            program,
            b"iResolution\0",
        );

    let scale_scene_location =
        uniform_location(
            postprocess_program,
            b"uScene\0",
        );

    let scale_resolution_location =
        uniform_location(
            postprocess_program,
            b"uResolution\0",
        );

    let spectrum_location =
        uniform_location(
            postprocess_program,
            b"uSpectrum[0]\0",
        );

    let trace_gain_location =
        uniform_location(
            postprocess_program,
            b"uTraceGain\0",
        );

    let spatial_width_location =
        uniform_location(
            postprocess_program,
            b"uSpatialWidth\0",
        );

    let mut texture_manager =
        crate::manage_textures::TextureManager::new(
            config.texture_policy.clone()
        );

    texture_manager.prepare_for_policy_with_path(
        0,
        &shader_name,
        Some(&shader_path),
        channel_usage,
    )?;

    texture_manager.configure_program(
        program
    );

    crate::audio_backend::set_audio_required(
        true
    );

    let _audio_capture_guard =
        AudioCaptureGuard;


    let mut event_pump =
        sdl.event_pump()
            .map_err(
                |error| {
                    format!(
                        "Unable to create Audio Motion event pump: {}",
                        error
                    )
                }
            )?;

    let mut fft_state =
        MirroredFftState::new();

    let mut previous_frame =
        Instant::now();

    'running: loop {
        for event in
            event_pump.poll_iter()
        {
            match event {
                Event::Quit {
                    ..
                }
                | Event::KeyDown {
                    keycode:
                        Some(
                            Keycode::Escape
                        ),
                    ..
                } => {
                    break 'running;
                }

                _ => {}
            }
        }

        let now =
            Instant::now();

        let frame_seconds =
            now.duration_since(
                previous_frame
            )
                .as_secs_f32()
                .clamp(
                    0.0,
                    0.100,
                );

        previous_frame =
            now;

        let spectrum =
            crate::analyze_audio::shared_audio_motion_fft_trace()
                .read()
                .ok()
                .map(|spectrum| *spectrum)
                .unwrap_or_default();

        fft_state.update(
            spectrum,
            frame_seconds,
        );

        let shader_time =
            fft_state.shader_time;

        texture_manager
            .update_animations()
            .map_err(
                |error| {
                    format!(
                        "Unable to update Audio Motion animated texture: {}",
                        error
                    )
                }
            )?;

        unsafe {
            gl::BindFramebuffer(
                gl::FRAMEBUFFER,
                scene_fbo,
            );

            gl::Viewport(
                0,
                0,
                TEST_WIDTH as i32,
                TEST_HEIGHT as i32,
            );

            gl::Disable(
                gl::BLEND
            );

            gl::ColorMask(
                gl::TRUE,
                gl::TRUE,
                gl::TRUE,
                gl::TRUE,
            );

            gl::ClearColor(
                0.0,
                0.0,
                0.0,
                1.0,
            );

            gl::Clear(
                gl::COLOR_BUFFER_BIT
            );

            gl::UseProgram(
                program
            );

            texture_manager.bind_channels();

            crate::apply_shader_inputs::apply(
                program,
                &shader_inputs,
            );

            gl::BindVertexArray(
                vao
            );

            if time_location != -1 {
                gl::Uniform1f(
                    time_location,
                    shader_time,
                );
            }

            if resolution_location != -1 {
                gl::Uniform3f(
                    resolution_location,
                    TEST_WIDTH as f32,
                    TEST_HEIGHT as f32,
                    1.0,
                );
            }

            gl::DrawArrays(
                gl::TRIANGLES,
                0,
                3,
            );

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::Viewport(0, 0, TEST_WIDTH as i32, TEST_HEIGHT as i32);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::UseProgram(
                postprocess_program
            );

            gl::ActiveTexture(
                gl::TEXTURE0
            );

            gl::BindTexture(
                gl::TEXTURE_2D,
                scene_texture,
            );

            if scale_scene_location != -1 {
                gl::Uniform1i(
                    scale_scene_location,
                    0,
                );
            }

            if scale_resolution_location != -1 {
                gl::Uniform2f(
                    scale_resolution_location,
                    TEST_WIDTH as f32,
                    TEST_HEIGHT as f32,
                );
            }

            if spectrum_location != -1 {
                gl::Uniform1fv(
                    spectrum_location,
                    TRACE_CHANNELS as i32,
                    fft_state.channels.as_ptr(),
                );
            }

            if trace_gain_location != -1 {
                gl::Uniform1f(
                    trace_gain_location,
                    TRACE_GAIN,
                );
            }

            if spatial_width_location != -1 {
                gl::Uniform1f(
                    spatial_width_location,
                    TRACE_SPATIAL_WIDTH,
                );
            }

            gl::BindVertexArray(vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
        }

        window.gl_swap_window();
    }

    unsafe {
        gl::Finish();

        if vao != 0 {
            gl::DeleteVertexArrays(
                1,
                &vao,
            );
        }

        if scene_texture != 0 {
            gl::DeleteTextures(1, &scene_texture);
        }

        if scene_fbo != 0 {
            gl::DeleteFramebuffers(1, &scene_fbo);
        }

        if postprocess_program != 0 {
            gl::DeleteProgram(postprocess_program);
        }

        if program != 0 {
            gl::DeleteProgram(
                program
            );
        }
    }

    texture_manager.delete_all();

    println!();
    println!("Mirrored multi-channel FFT Audio Motion test ended.");

    Ok(())
}

fn resolve_shader_path(
    argument: &str,
) -> Result<PathBuf, String> {
    let supplied =
        PathBuf::from(
            argument
        );

    let resolved =
        if supplied.is_absolute()
            || supplied.components()
                .count()
                > 1
        {
            supplied
        } else {
            let managed =
                crate::locate_paths::shader_dir()
                    .join(
                        &supplied
                    );

            if managed.is_file() {
                managed
            } else {
                supplied
            }
        };

    if !Path::new(
        &resolved
    ).is_file()
    {
        return Err(
            format!(
                "Audio Motion test shader does not exist or is not a file: {}",
                resolved.display(),
            )
        );
    }

    Ok(
        resolved
    )
}

fn uniform_location(
    program: u32,
    name: &'static [u8],
) -> i32 {
    unsafe {
        gl::GetUniformLocation(
            program,
            name.as_ptr()
                as *const _,
        )
    }
}
