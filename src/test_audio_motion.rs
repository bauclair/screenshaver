use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;

const TEST_WIDTH: u32 = 1280;
const TEST_HEIGHT: u32 = 720;

// Experimental values only. The rendering shader always advances normally.
//
// This version treats the completed shader image like a loudspeaker cone.
// Bass energy drives a centered zoom toward the viewer. Attack is fast so
// bass hits register promptly; release is slower so the image settles back
// naturally. Midrange and treble do not participate in this experiment.
const SHADER_SPEED: f32 = 1.0;
// Broad-spectrum transient detector. Any band can kick the cone when its
// normalized level rises sharply from one analyzer update to the next.
// Prioritized continuous three-band cone controller.
//
// Midrange has musical priority: as meaningful midrange energy appears, it
// smoothly suppresses bass and treble and becomes the main cone driver.
// With midrange absent, bass receives the greatest excursion authority.
// Treble remains a deliberately subtle fallback influence.
// Direct transient response for Audio Motion. Musical attacks set outward
// displacement immediately; only inward release is smoothed.
// Direct Audio Motion response. Attacks are applied immediately; only the
// inward return is smoothed. A small bounded continuous component keeps
// sustained passages alive without allowing them to hold the cone open.
const TRANSIENT_BASELINE_RISE_SECONDS: f32 = 0.18;
const TRANSIENT_BASELINE_FALL_SECONDS: f32 = 0.08;
const TRANSIENT_NOISE_FLOOR: f32 = 0.006;
const TRANSIENT_GAIN: f32 = 4.50;
const TRANSIENT_DISPLACEMENT_GAIN: f32 = 1.00;
const CONTINUOUS_DISPLACEMENT_GAIN: f32 = 0.10;
const RELEASE_SECONDS: f32 = 0.075;
const MAX_DISPLACEMENT: f32 = 1.0;
const MAX_SCALE_EXPANSION: f32 = 2.20;
const DIAGNOSTIC_INTERVAL: Duration = Duration::from_millis(100);

// LRCMUX-controlled complementary spectral selection.  These are genuine
// FFT-derived measurements supplied by analyze_audio.rs, not approximations
// made from the three Audio Bloom bands.
//
// vocals OFF  -> NOTCH 1-3 kHz -> use energy outside 1-3 kHz
// vocals ON   -> BANDPASS 1-3 kHz -> use energy inside 1-3 kHz
// no LRCMUX timing -> BYPASS -> use the stronger complementary measurement
const MOTION_NOISE_FLOOR: f32 = 0.035;
const MOTION_RESPONSE_CURVE: f32 = 0.90;
const FILTER_TRANSITION_ATTACK: f32 = 0.32;
const FILTER_TRANSITION_RELEASE: f32 = 0.20;

const BASS_SCALE_FRAGMENT_SHADER: &str = r#"#version 330 core
out vec4 FragColor;

uniform sampler2D uScene;
uniform vec2 uResolution;
uniform float uScale;

void main() {
    vec2 uv = gl_FragCoord.xy / uResolution;
    vec2 centered = uv - vec2(0.5);

    // Measure radius in aspect-correct coordinates so the deformation is
    // circular rather than elliptical on a widescreen display.
    float aspect = uResolution.x / uResolution.y;
    vec2 conePosition = centered;
    conePosition.x *= aspect;

    float radius = length(conePosition);

    // The nearest screen edge is radius 0.5 in these coordinates. The cone
    // is strongest at its center and becomes stationary at that boundary.
    float normalizedRadius =
        clamp(radius / 0.5, 0.0, 1.0);

    // Inverse image of a loudspeaker cone: the center protrudes maximally,
    // then the apparent depth falls away nonlinearly toward the anchored
    // surround. Strong impulses sharpen the central bulge instead of merely
    // performing a uniform zoom.
    float coneWeight =
        1.0 - smoothstep(
            0.0,
            1.0,
            normalizedRadius
        );

    float expansion =
        max(uScale - 1.0, 0.0);

    // As excursion grows, concentrate more deformation into the inner cone.
    float coneExponent =
        mix(
            0.90,
            2.60,
            clamp(expansion / 4.0, 0.0, 1.0)
        );

    coneWeight =
        pow(coneWeight, coneExponent);

    // A quadratic term makes extreme bass impulses increasingly nonlinear:
    // the center appears to thrust out of the screen while the edge remains
    // at unity scale.
    float localExpansion =
        expansion * coneWeight
        + 0.35 * expansion * expansion * coneWeight * coneWeight;

    float localScale =
        1.0 + localExpansion;

    vec2 sampleUv =
        centered / localScale
        + vec2(0.5);

    sampleUv =
        clamp(
            sampleUv,
            vec2(0.001),
            vec2(0.999)
        );

    FragColor =
        texture(uScene, sampleUv);
}
"#;

struct AudioCaptureGuard;

impl Drop for AudioCaptureGuard {
    fn drop(&mut self) {
        crate::audio_backend::set_audio_required(
            false
        );
    }
}

struct PrioritizedConeState {
    shader_time: f32,
    displacement: f32,
    scale: f32,
    last_diagnostic: Instant,
    filter_blend: f32,
    transient_baseline: f32,
}

impl PrioritizedConeState {
    fn new() -> Self {
        Self {
            shader_time: 0.0,
            displacement: 0.0,
            scale: 1.0,
            last_diagnostic: Instant::now(),
            filter_blend: 0.0,
            transient_baseline: 0.0,
        }
    }

    fn normalized_motion(value: f32) -> f32 {
        (
            (value.clamp(0.0, 1.0) - MOTION_NOISE_FLOOR)
                / (1.0 - MOTION_NOISE_FLOOR)
        )
        .clamp(0.0, 1.0)
        .powf(MOTION_RESPONSE_CURVE)
    }

    fn update(
        &mut self,
        spectrum: crate::analyze_audio::AudioMotionSpectrum,
        frame_seconds: f32,
        vocal_timing: &crate::manage_lyrics::VocalTimingState,
    ) {
        self.shader_time += frame_seconds * SHADER_SPEED;

        // 0.0 selects the 1-3 kHz NOTCH result (outside energy).
        // 1.0 selects the 1-3 kHz BANDPASS result (inside energy).
        // Without LRCMUX timing, bypass lyric gating and use whichever
        // complementary measurement currently carries more energy.
        let filter_target = if vocal_timing.available {
            if vocal_timing.active { 1.0_f32 } else { 0.0_f32 }
        } else {
            self.filter_blend
        };

        let nominal_frames = (frame_seconds * 60.0).max(0.0);
        let filter_base_alpha = if filter_target > self.filter_blend {
            FILTER_TRANSITION_ATTACK
        } else {
            FILTER_TRANSITION_RELEASE
        };
        let filter_alpha =
            1.0 - (1.0 - filter_base_alpha).powf(nominal_frames);

        self.filter_blend +=
            (filter_target - self.filter_blend) * filter_alpha;
        self.filter_blend = self.filter_blend.clamp(0.0, 1.0);

        let inside = Self::normalized_motion(spectrum.inside_1k_3k);
        let outside = Self::normalized_motion(spectrum.outside_1k_3k);

        let selected = if vocal_timing.available {
            outside + (inside - outside) * self.filter_blend
        } else {
            inside.max(outside)
        }
        .clamp(0.0, 1.0);

        // Track sustained selected energy with a short adaptive baseline.
        // Positive excursions above that baseline are attacks. The attack path
        // is intentionally unsmoothed: a new musical event may move the cone
        // outward immediately on this update.
        let dt = frame_seconds.clamp(0.0, 1.0 / 30.0);

        let baseline_seconds =
            if selected > self.transient_baseline {
                TRANSIENT_BASELINE_RISE_SECONDS
            } else {
                TRANSIENT_BASELINE_FALL_SECONDS
            };

        let baseline_alpha =
            if baseline_seconds > 0.0 {
                1.0 - (-dt / baseline_seconds).exp()
            } else {
                1.0
            };

        self.transient_baseline +=
            (selected - self.transient_baseline) * baseline_alpha;
        self.transient_baseline = self.transient_baseline.clamp(0.0, 1.0);

        let transient_raw =
            (selected - self.transient_baseline).max(0.0);

        let transient =
            ((transient_raw - TRANSIENT_NOISE_FLOOR).max(0.0)
                * TRANSIENT_GAIN)
                .clamp(0.0, 1.0);

        let transient_displacement =
            transient * TRANSIENT_DISPLACEMENT_GAIN;

        let continuous_displacement =
            selected * CONTINUOUS_DISPLACEMENT_GAIN;

        let target_displacement =
            (transient_displacement + continuous_displacement)
                .clamp(0.0, MAX_DISPLACEMENT);

        if target_displacement >= self.displacement {
            // No attack interpolation. Preserve the immediacy and some of the
            // natural jitter of rapidly changing musical attacks.
            self.displacement = target_displacement;
        } else {
            // Smooth only the return toward the current target. This prevents
            // harsh frame-to-frame collapse without damping the next attack.
            let release_alpha =
                if RELEASE_SECONDS > 0.0 {
                    1.0 - (-dt / RELEASE_SECONDS).exp()
                } else {
                    1.0
                };

            self.displacement +=
                (target_displacement - self.displacement) * release_alpha;
        }

        self.displacement =
            self.displacement.clamp(0.0, MAX_DISPLACEMENT);

        self.scale = 1.0 + self.displacement * MAX_SCALE_EXPANSION;

        if self.last_diagnostic.elapsed() >= DIAGNOSTIC_INTERVAL {
            println!(
                "[AUDIO MOTION] lrcmux={} vocal={} filter={} blend={:.3} spectrum=(inside_1_3k:{:.3},outside_1_3k:{:.3}) selected={:.3} baseline={:.3} transient_raw={:.3} transient={:.3} transient_disp={:.3} continuous_disp={:.3} target={:.3} displacement={:.3} scale={:.4} shader_time={:.2}",
                if vocal_timing.available { "READY" } else { "NONE" },
                if vocal_timing.active { "ACTIVE" } else { "OFF" },
                if !vocal_timing.available {
                    "BYPASS"
                } else if vocal_timing.active {
                    "BANDPASS_1-3KHZ"
                } else {
                    "NOTCH_1-3KHZ"
                },
                self.filter_blend,
                spectrum.inside_1k_3k,
                spectrum.outside_1k_3k,
                selected,
                self.transient_baseline,
                transient_raw,
                transient,
                transient_displacement,
                continuous_displacement,
                target_displacement,
                self.displacement,
                self.scale,
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
    println!("Effect: LRCMUX-controlled 1-3 kHz spectral notch/bandpass inverse-cone deformation");
    println!("Vocal timing: LRCMUX start/end intervals only");
    println!("Shader animation speed: {:.2}", SHADER_SPEED);
    println!("Vocal spectral range: 1.0-3.0 kHz");
    println!("Lyrics inactive: NOTCH 1-3 kHz (motion uses outside energy)");
    println!("Lyrics active: BANDPASS 1-3 kHz (motion uses inside energy)");
    println!("No LRCMUX timing: BYPASS lyric gating");
    println!("Motion noise floor: {:.3}", MOTION_NOISE_FLOOR);
    println!("Motion response curve: {:.2}", MOTION_RESPONSE_CURVE);
    println!("Transient baseline rise: {:.3}s", TRANSIENT_BASELINE_RISE_SECONDS);
    println!("Transient baseline fall: {:.3}s", TRANSIENT_BASELINE_FALL_SECONDS);
    println!("Transient noise floor: {:.3}", TRANSIENT_NOISE_FLOOR);
    println!("Transient gain: {:.2}", TRANSIENT_GAIN);
    println!("Transient displacement gain: {:.2}", TRANSIENT_DISPLACEMENT_GAIN);
    println!("Continuous displacement gain: {:.2}", CONTINUOUS_DISPLACEMENT_GAIN);
    println!("Release time: {:.3}s", RELEASE_SECONDS);
    println!("Attack smoothing: NONE (immediate outward response)");
    println!(
        "Maximum center scale: {:.1}% plus inverse-cone deformation",
        (1.0 + MAX_SCALE_EXPANSION) * 100.0
    );
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
            BASS_SCALE_FRAGMENT_SHADER,
        )
        .map_err(|error| {
            format!("Audio Motion bass-scale post-process shader compilation failed: {}", error)
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

    let scale_value_location =
        uniform_location(
            postprocess_program,
            b"uScale\0",
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

    let lyrics_manager =
        crate::manage_lyrics::LyricsManager::start()
            .map_err(|error| {
                format!(
                    "Unable to start LRCMUX timing for Audio Motion test: {}",
                    error
                )
            })?;

    let vocal_timing_state =
        lyrics_manager.shared_vocal_timing_state();

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

    let mut bass_scale =
        PrioritizedConeState::new();

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
            crate::analyze_audio::shared_audio_motion_spectrum()
                .read()
                .ok()
                .map(|spectrum| *spectrum)
                .unwrap_or_default();

        let vocal_timing =
            vocal_timing_state
                .lock()
                .map(|state| state.clone())
                .unwrap_or_default();

        bass_scale.update(
            spectrum,
            frame_seconds,
            &vocal_timing,
        );

        let shader_time =
            bass_scale.shader_time;

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

            if scale_value_location != -1 {
                gl::Uniform1f(
                    scale_value_location,
                    bass_scale.scale,
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
    println!("Direct transient spectral Audio Motion test ended.");

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
