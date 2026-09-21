use std::path::Path;
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;

const BENCHMARK_WIDTH: u32 = 1280;
const BENCHMARK_HEIGHT: u32 = 720;
const WARMUP_SECONDS: u64 = 3;
const MEASURE_SECONDS: u64 = 10;

#[derive(Clone, Copy, Debug)]
enum FinishMode {
    Finish,
    Fence,
    Flush,
    None,
}

#[derive(Clone, Copy, Debug)]
struct BenchmarkCase {
    name: &'static str,
    postprocess: bool,
    cached_uniforms: bool,
    finish_mode: FinishMode,
}

#[derive(Debug)]
struct BenchmarkResult {
    name: &'static str,
    frames: usize,
    elapsed: Duration,
    average_fps: f64,
    one_percent_low_fps: f64,
    point_one_percent_low_fps: f64,
    mean_ms: f64,
    median_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    worst_ms: f64,
}

struct UniformLocations {
    time: i32,
    resolution: i32,
}

#[derive(Clone, Debug)]
struct GpuTimingSample {
    frame: usize,
    cpu_wall_ms: f64,
    gpu_scene_ms: f64,
    gpu_postprocess_ms: f64,
    gpu_primary_ms: f64,
    gpu_bloom_extraction_ms: f64,
    gpu_bloom_blur_horizontal_ms: f64,
    gpu_bloom_blur_vertical_ms: f64,
    gpu_bloom_composite_ms: f64,
    gpu_dithering_ms: f64,
    gpu_total_ms: f64,
}

#[derive(Debug)]
struct GpuTimingSummary {
    frames: usize,
    cpu_mean_ms: f64,
    cpu_p99_ms: f64,
    cpu_worst_ms: f64,
    gpu_scene_mean_ms: f64,
    gpu_postprocess_mean_ms: f64,
    gpu_primary_mean_ms: f64,
    gpu_bloom_extraction_mean_ms: f64,
    gpu_bloom_blur_horizontal_mean_ms: f64,
    gpu_bloom_blur_vertical_mean_ms: f64,
    gpu_bloom_composite_mean_ms: f64,
    gpu_dithering_mean_ms: f64,
    gpu_total_mean_ms: f64,
    gpu_total_p99_ms: f64,
    gpu_total_worst_ms: f64,
    largest_wall_minus_gpu_ms: f64,
    worst_cpu_frames: Vec<GpuTimingSample>,
    worst_gpu_frames: Vec<GpuTimingSample>,
    worst_postprocess_frames: Vec<GpuTimingSample>,
    worst_scene_frames: Vec<GpuTimingSample>,
    external_stall_frames: usize,
    scene_heavy_frames: usize,
    postprocess_heavy_frames: usize,
}

pub fn run(
    shader_path: &str,
    config: &crate::load_config::Config,
) -> Result<(), String> {
    let shader_path = Path::new(shader_path);

    if !shader_path.is_file() {
        return Err(
            format!(
                "Benchmark shader does not exist or is not a file: {}",
                shader_path.display(),
            )
        );
    }

    let loaded =
        crate::load_shader::load_shader_for_preview(
            shader_path
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

    println!("Screenshaver Render Benchmark");
    println!("============================");
    println!();
    println!("Shader: {}", shader_path.display());
    println!("Processed shader: {}", shader_name);
    println!(
        "Benchmark size: {}x{}",
        BENCHMARK_WIDTH,
        BENCHMARK_HEIGHT,
    );
    println!(
        "Warm-up: {} s per case; measurement: {} s per case",
        WARMUP_SECONDS,
        MEASURE_SECONDS,
    );
    println!();
    println!(
        "This benchmark intentionally disables the normal FPS limiter and VSync."
    );
    println!(
        "Press Esc or close the benchmark window to abort."
    );
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
                "Screenshaver Render Benchmark",
                BENCHMARK_WIDTH,
                BENCHMARK_HEIGHT,
            )
            .position_centered()
            .opengl()
            .build()
            .map_err(
                |error| {
                    format!(
                        "Unable to create benchmark window: {}",
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
                        "Unable to create benchmark OpenGL context: {}",
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

    let _ =
        video.gl_set_swap_interval(
            0
        );

    print_gl_information();

    let program =
        crate::compile_shader::build_program(
            crate::define_constants::VERTEX_SHADER,
            &source,
        )
        .map_err(
            |error| {
                format!(
                    "Benchmark shader compilation failed: {}",
                    error
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

        gl::BindVertexArray(
            vao
        );
    }

    let cached_uniforms =
        UniformLocations {
            time:
                uniform_location(
                    program,
                    b"iTime\0",
                ),

            resolution:
                uniform_location(
                    program,
                    b"iResolution\0",
                ),
        };

    let mut texture_manager =
        crate::manage_textures::TextureManager::new(
            config.texture_policy.clone()
        );

    texture_manager.prepare_for_policy_with_path(
        0,
        &shader_name,
        Some(shader_path),
        channel_usage,
    )?;

    texture_manager.configure_program(
        program
    );

    let profile =
        config
            .screensaver_postprocess_policy
            .profile_for_policy(
                0,
                &shader_name,
                Some(shader_path),
            );

    let mut postprocess =
        crate::postprocess_shader::PostprocessPipeline::new(
            BENCHMARK_WIDTH,
            BENCHMARK_HEIGHT,
            profile,
        )?;

    let benchmark_audio_bands =
        crate::audio_backend::shared_audio_bands();

    let audio_bloom_active =
        matches!(
            profile.bloom,
            crate::render_bloom::BloomMode::Audio
                | crate::render_bloom::BloomMode::Spectral
                | crate::render_bloom::BloomMode::Loudness
        );

    crate::audio_backend::set_audio_required(
        audio_bloom_active
    );

    if audio_bloom_active {
        println!(
            "Audio-reactive Bloom is active. Play music during the benchmark to exercise live audio input."
        );
    }

    let mut audio_diagnostic =
        BenchmarkAudioDiagnostic::new();

    let cases = [
        BenchmarkCase {
            name: "Production baseline",
            postprocess: true,
            cached_uniforms: false,
            finish_mode: FinishMode::Finish,
        },
        BenchmarkCase {
            name: "Postprocess + fence wait",
            postprocess: true,
            cached_uniforms: false,
            finish_mode: FinishMode::Fence,
        },
        BenchmarkCase {
            name: "Postprocess + glFlush",
            postprocess: true,
            cached_uniforms: false,
            finish_mode: FinishMode::Flush,
        },
        BenchmarkCase {
            name: "Postprocess + no explicit sync",
            postprocess: true,
            cached_uniforms: false,
            finish_mode: FinishMode::None,
        },
        BenchmarkCase {
            name: "Cached uniforms",
            postprocess: true,
            cached_uniforms: true,
            finish_mode: FinishMode::Finish,
        },
        BenchmarkCase {
            name: "Cached uniforms + no explicit sync",
            postprocess: true,
            cached_uniforms: true,
            finish_mode: FinishMode::None,
        },
        BenchmarkCase {
            name: "Raw shader",
            postprocess: false,
            cached_uniforms: true,
            finish_mode: FinishMode::Finish,
        },
        BenchmarkCase {
            name: "Raw shader + no explicit sync",
            postprocess: false,
            cached_uniforms: true,
            finish_mode: FinishMode::None,
        },
    ];

    let mut event_pump =
        sdl.event_pump()
            .map_err(
                |error| {
                    format!(
                        "Unable to create benchmark event pump: {}",
                        error
                    )
                }
            )?;

    let benchmark_start =
        Instant::now();

    let mut results =
        Vec::with_capacity(
            cases.len()
        );

    for (
        index,
        case,
    ) in cases
        .iter()
        .enumerate()
    {
        println!(
            "[{}/{}] {}",
            index + 1,
            cases.len(),
            case.name,
        );

        println!(
            "    warming up..."
        );

        run_for_duration(
            *case,
            Duration::from_secs(
                WARMUP_SECONDS
            ),
            false,
            benchmark_start,
            &window,
            &mut event_pump,
            program,
            vao,
            &cached_uniforms,
            &mut texture_manager,
            &shader_inputs,
            &mut postprocess,
            &benchmark_audio_bands,
            audio_bloom_active,
            &mut audio_diagnostic,
        )?;

        unsafe {
            // Start every measured phase with an empty command queue so work
            // submitted by the preceding phase cannot contaminate its timing.
            gl::Finish();
        }

        println!(
            "    measuring..."
        );

        let result =
            run_for_duration(
                *case,
                Duration::from_secs(
                    MEASURE_SECONDS
                ),
                true,
                benchmark_start,
                &window,
                &mut event_pump,
                program,
                vao,
                &cached_uniforms,
                &mut texture_manager,
                &shader_inputs,
                &mut postprocess,
                &benchmark_audio_bands,
                audio_bloom_active,
                &mut audio_diagnostic,
            )?
            .ok_or_else(
                || {
                    "Benchmark phase produced no timing result"
                        .to_string()
                }
            )?;

        println!(
            "    {:.2} FPS, P99 {:.3} ms",
            result.average_fps,
            result.p99_ms,
        );

        results.push(
            result
        );
    }

    println!();
    println!("GPU timer-query diagnostic");
    println!("--------------------------");
    println!(
        "This 30-second phase uses OpenGL timestamp queries around the scene and post-processing work."
    );
    println!(
        "While it runs, deliberately perform the desktop action that has triggered CRITICAL FPS warnings"
    );
    println!(
        "(for example, switch Mango workspaces several times)."
    );
    println!();

    let gpu_timing =
        run_gpu_timing_diagnostic(
            Duration::from_secs(30),
            benchmark_start,
            &window,
            &mut event_pump,
            program,
            vao,
            &cached_uniforms,
            &mut texture_manager,
            &shader_inputs,
            &mut postprocess,
            &benchmark_audio_bands,
            audio_bloom_active,
            &mut audio_diagnostic,
        )?;

    print_gpu_timing_summary(
        &gpu_timing
    );

    if audio_bloom_active {
        audio_diagnostic.print_summary();
    }

    crate::audio_backend::set_audio_required(
        false
    );

    unsafe {
        gl::Finish();

        if vao != 0 {
            gl::DeleteVertexArrays(
                1,
                &vao,
            );
        }

        if program != 0 {
            gl::DeleteProgram(
                program
            );
        }
    }

    texture_manager.delete_all();

    println!();
    print_results(
        &results
    );

    print_sync_comparison(
        &results
    );

    println!();
    println!(
        "Notes:"
    );
    println!(
        "  * 'Production baseline' reproduces the current postprocess + per-frame uniform lookup + glFinish synchronization pattern."
    );
    println!(
        "  * Raw-shader cases bypass Screenshaver post-processing and establish the shader's approximate rendering ceiling."
    );
    println!(
        "  * PBO is not included in this first harness because the audited production path does not currently perform a per-frame CPU pixel transfer for a PBO to replace."
    );
    println!(
        "  * The GPU timer-query diagnostic separates CPU wall time from GPU scene and post-processing time."
    );
    println!(
        "  * It also retains the 20 worst CPU-wall and 20 worst GPU-total frames with correlated per-frame measurements."
    );
    println!(
        "  * A large CPU-wall spike without a comparable GPU-time spike indicates presentation/compositor/driver waiting rather than an intrinsically slow shader frame."
    );
    println!(
        "  * The fence-wait case inserts GL_SYNC_GPU_COMMANDS_COMPLETE after the frame and waits with glClientWaitSync, allowing a direct comparison with the production glFinish barrier."
    );
    println!(
        "  * The diagnostic times the actual production post-processing passes individually: primary/AA, Bloom extraction, both blur passes, composition, and dithering/final output."
    );
    println!(
        "  * Audio-reactive Bloom requests the normal audio backend and feeds live AudioBands into every postprocessed benchmark phase and the detailed diagnostic."
    );
    println!(
        "  * Reduced/fused post-processing and alternate Bloom-resolution experiments can now be designed from measured per-pass costs."
    );
    println!(
        "  * Additional outlier tables rank the worst post-processing and scene-shader frames and retain every measured post-processing stage."
    );
    println!(
        "  * Diagnostic counters separately identify large CPU/presentation waits, scene-heavy GPU frames, and postprocess-heavy GPU frames."
    );
    println!(
        "  * Live AudioBands are printed about once per second and summarized with peak/nonzero counts so Audio Bloom input can be verified objectively."
    );

    Ok(())
}


#[derive(Debug)]
struct BenchmarkAudioDiagnostic {
    last_report: Instant,
    peak_bass: f32,
    peak_midrange: f32,
    peak_treble: f32,
    nonzero_samples: usize,
    samples: usize,
}

impl BenchmarkAudioDiagnostic {
    fn new() -> Self {
        Self {
            last_report: Instant::now(),
            peak_bass: 0.0,
            peak_midrange: 0.0,
            peak_treble: 0.0,
            nonzero_samples: 0,
            samples: 0,
        }
    }

    fn observe(
        &mut self,
        bands: crate::analyze_audio::AudioBands,
        print_live: bool,
    ) {
        self.samples += 1;

        self.peak_bass =
            self.peak_bass.max(bands.bass);
        self.peak_midrange =
            self.peak_midrange.max(bands.midrange);
        self.peak_treble =
            self.peak_treble.max(bands.treble);

        if bands.bass > 0.000_001
            || bands.midrange > 0.000_001
            || bands.treble > 0.000_001
        {
            self.nonzero_samples += 1;
        }

        if print_live
            && self.last_report.elapsed()
                >= Duration::from_secs(1)
        {
            println!(
                "    [AUDIO] live bands: bass={:.3} midrange={:.3} treble={:.3}",
                bands.bass,
                bands.midrange,
                bands.treble,
            );

            self.last_report =
                Instant::now();
        }
    }

    fn print_summary(&self) {
        println!();
        println!("Audio input diagnostic");
        println!(
            "    Samples observed:              {}",
            self.samples,
        );
        println!(
            "    Nonzero samples:               {}",
            self.nonzero_samples,
        );
        println!(
            "    Peak bass:                     {:.3}",
            self.peak_bass,
        );
        println!(
            "    Peak midrange:                 {:.3}",
            self.peak_midrange,
        );
        println!(
            "    Peak treble:                   {:.3}",
            self.peak_treble,
        );

        if self.nonzero_samples == 0 {
            println!(
                "    AUDIO INPUT FAILURE: no nonzero AudioBands were observed while Audio Bloom was requested."
            );
        } else {
            println!(
                "    Audio input confirmed: live nonzero AudioBands reached the benchmark."
            );
        }
    }
}


#[allow(clippy::too_many_arguments)]
fn run_for_duration(
    case: BenchmarkCase,
    duration: Duration,
    collect: bool,
    benchmark_start: Instant,
    window: &sdl2::video::Window,
    event_pump: &mut sdl2::EventPump,
    program: u32,
    vao: u32,
    cached_uniforms: &UniformLocations,
    texture_manager: &mut crate::manage_textures::TextureManager,
    shader_inputs: &[crate::isf_types::ShaderInput],
    postprocess: &mut crate::postprocess_shader::PostprocessPipeline,
    audio_bands: &crate::audio_backend::SharedAudioBands,
    audio_bloom_active: bool,
    audio_diagnostic: &mut BenchmarkAudioDiagnostic,
) -> Result<Option<BenchmarkResult>, String> {
    let phase_start =
        Instant::now();

    let mut frame_times =
        Vec::<f64>::new();

    let mut frames =
        0_usize;

    while phase_start.elapsed()
        < duration
    {
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
                    return Err(
                        "Benchmark aborted by user"
                            .to_string()
                    );
                }

                _ => {}
            }
        }

        let frame_start =
            Instant::now();

        if case.postprocess
            && audio_bloom_active
        {
            crate::audio_backend::set_audio_required(
                true
            );

            let current_audio_bands =
                audio_bands
                    .read()
                    .ok()
                    .map(|bands| *bands)
                    .unwrap_or_default();

            audio_diagnostic.observe(
                current_audio_bands,
                collect,
            );

            postprocess.set_audio_bands(
                current_audio_bands
            );
        }

        render_one_frame(
            case,
            benchmark_start,
            program,
            vao,
            cached_uniforms,
            texture_manager,
            shader_inputs,
            postprocess,
        )?;

        window.gl_swap_window();

        match case.finish_mode {
            FinishMode::Finish => {
                // The production renderer currently uses glFinish before
                // recording its frame duration. Keep it in the baseline.
            }

            FinishMode::Fence => {
                // The fence is inserted and waited in render_one_frame at the
                // same location as the production glFinish barrier.
            }

            FinishMode::Flush => {
                unsafe {
                    gl::Flush();
                }
            }

            FinishMode::None => {}
        }

        if collect {
            frame_times.push(
                frame_start
                    .elapsed()
                    .as_secs_f64()
            );
        }

        frames +=
            1;
    }

    // For asynchronous cases, force all queued GPU work to complete before
    // the phase's total elapsed time is finalized. This prevents an apparent
    // FPS gain caused only by leaving work queued after the measurement.
    unsafe {
        gl::Finish();
    }

    if !collect {
        return Ok(
            None
        );
    }

    let elapsed =
        phase_start.elapsed();

    Ok(
        Some(
            summarize(
                case.name,
                frames,
                elapsed,
                frame_times,
            )
        )
    )
}

#[allow(clippy::too_many_arguments)]
fn render_one_frame(
    case: BenchmarkCase,
    benchmark_start: Instant,
    program: u32,
    vao: u32,
    cached_uniforms: &UniformLocations,
    texture_manager: &mut crate::manage_textures::TextureManager,
    shader_inputs: &[crate::isf_types::ShaderInput],
    postprocess: &mut crate::postprocess_shader::PostprocessPipeline,
) -> Result<(), String> {
    if let Err(error) =
        texture_manager
            .update_animations()
    {
        return Err(
            format!(
                "Unable to update benchmark animated texture: {}",
                error
            )
        );
    }

    let (
        scene_width,
        scene_height,
    ) =
        if case.postprocess {
            postprocess.bind_scene_target();

            postprocess.scene_dimensions()
        } else {
            unsafe {
                gl::BindFramebuffer(
                    gl::FRAMEBUFFER,
                    0,
                );

                gl::Viewport(
                    0,
                    0,
                    BENCHMARK_WIDTH as i32,
                    BENCHMARK_HEIGHT as i32,
                );
            }

            (
                BENCHMARK_WIDTH,
                BENCHMARK_HEIGHT,
            )
        };

    unsafe {
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
            shader_inputs,
        );

        gl::BindVertexArray(
            vao
        );

        let (
            time_location,
            resolution_location,
        ) =
            if case.cached_uniforms {
                (
                    cached_uniforms.time,
                    cached_uniforms.resolution,
                )
            } else {
                (
                    uniform_location(
                        program,
                        b"iTime\0",
                    ),
                    uniform_location(
                        program,
                        b"iResolution\0",
                    ),
                )
            };

        if time_location != -1 {
            gl::Uniform1f(
                time_location,
                benchmark_start
                    .elapsed()
                    .as_secs_f32(),
            );
        }

        if resolution_location != -1 {
            gl::Uniform3f(
                resolution_location,
                scene_width as f32,
                scene_height as f32,
                1.0,
            );
        }

        gl::DrawArrays(
            gl::TRIANGLES,
            0,
            3,
        );
    }

    if case.postprocess {
        postprocess
            .present_scene_to_framebuffer(
                0
            );
    }

    match case.finish_mode {
        FinishMode::Finish => {
            unsafe {
                gl::Finish();
            }
        }

        FinishMode::Fence => {
            wait_for_frame_fence()?;
        }

        FinishMode::Flush
        | FinishMode::None => {}
    }

    Ok(())
}

fn wait_for_frame_fence() -> Result<(), String> {
    unsafe {
        let fence =
            gl::FenceSync(
                gl::SYNC_GPU_COMMANDS_COMPLETE,
                0,
            );

        if fence.is_null() {
            return Err(
                "OpenGL failed to create the benchmark frame fence"
                    .to_string()
            );
        }

        // ClientWaitSync with SYNC_FLUSH_COMMANDS_BIT both submits preceding
        // commands and waits only for this frame's fence rather than invoking
        // the global glFinish completion barrier.
        loop {
            let status =
                gl::ClientWaitSync(
                    fence,
                    gl::SYNC_FLUSH_COMMANDS_BIT,
                    1_000_000_000,
                );

            if status
                == gl::ALREADY_SIGNALED
                || status
                    == gl::CONDITION_SATISFIED
            {
                gl::DeleteSync(
                    fence
                );

                break;
            }

            if status
                == gl::WAIT_FAILED
            {
                gl::DeleteSync(
                    fence
                );

                return Err(
                    "OpenGL frame-fence wait failed"
                        .to_string()
                );
            }

            // TIMEOUT_EXPIRED is intentionally retried. The one-second wait
            // above is far longer than a normal Screenshaver frame, but this
            // keeps the benchmark correct if the driver is temporarily
            // stalled rather than silently treating unfinished work as done.
        }
    }

    Ok(())
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

fn summarize(
    name: &'static str,
    frames: usize,
    elapsed: Duration,
    mut frame_times: Vec<f64>,
) -> BenchmarkResult {
    frame_times.sort_by(
        |left, right| {
            left.total_cmp(
                right
            )
        }
    );

    let average_fps =
        frames as f64
            / elapsed
                .as_secs_f64()
                .max(
                    f64::EPSILON
                );

    let mean_seconds =
        if frame_times.is_empty() {
            0.0
        } else {
            frame_times
                .iter()
                .sum::<f64>()
                / frame_times.len() as f64
        };

    let one_percent_low_fps =
        low_fps(
            &frame_times,
            0.01
        );

    let point_one_percent_low_fps =
        low_fps(
            &frame_times,
            0.001
        );

    BenchmarkResult {
        name,
        frames,
        elapsed,
        average_fps,
        one_percent_low_fps,
        point_one_percent_low_fps,
        mean_ms:
            mean_seconds
                * 1000.0,
        median_ms:
            percentile(
                &frame_times,
                0.50
            ) * 1000.0,
        p95_ms:
            percentile(
                &frame_times,
                0.95
            ) * 1000.0,
        p99_ms:
            percentile(
                &frame_times,
                0.99
            ) * 1000.0,
        worst_ms:
            frame_times
                .last()
                .copied()
                .unwrap_or(
                    0.0
                )
                * 1000.0,
    }
}

fn percentile(
    sorted: &[f64],
    percentile: f64,
) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }

    let position =
        (
            percentile
                .clamp(
                    0.0,
                    1.0,
                )
            * (sorted.len() - 1) as f64
        )
        .round()
        as usize;

    sorted[
        position.min(
            sorted.len() - 1
        )
    ]
}

fn low_fps(
    sorted_frame_times: &[f64],
    fraction: f64,
) -> f64 {
    if sorted_frame_times.is_empty() {
        return 0.0;
    }

    let sample_count =
        (
            sorted_frame_times.len() as f64
                * fraction
        )
        .ceil()
        .max(
            1.0
        )
        as usize;

    let start =
        sorted_frame_times
            .len()
            .saturating_sub(
                sample_count
            );

    let slow_mean =
        sorted_frame_times[
            start..
        ]
        .iter()
        .sum::<f64>()
        / sample_count as f64;

    if slow_mean
        <= f64::EPSILON
    {
        0.0
    } else {
        1.0
            / slow_mean
    }
}

fn print_results(
    results: &[BenchmarkResult],
) {
    println!(
        "Results"
    );

    println!(
        "-------"
    );

    println!(
        "{:<38} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}",
        "Configuration",
        "Avg FPS",
        "1% Low",
        "0.1% Low",
        "Mean ms",
        "P99 ms",
        "Worst ms",
    );

    println!(
        "{}",
        "-".repeat(
            101
        )
    );

    for result in
        results
    {
        println!(
            "{:<38} {:>9.2} {:>9.2} {:>9.2} {:>9.3} {:>9.3} {:>9.3}",
            result.name,
            result.average_fps,
            result.one_percent_low_fps,
            result.point_one_percent_low_fps,
            result.mean_ms,
            result.p99_ms,
            result.worst_ms,
        );
    }

    println!();
    println!(
        "Detailed frame-time percentiles"
    );

    println!(
        "{:<38} {:>9} {:>9} {:>9} {:>10}",
        "Configuration",
        "Median",
        "P95",
        "P99",
        "Frames",
    );

    println!(
        "{}",
        "-".repeat(
            80
        )
    );

    for result in
        results
    {
        println!(
            "{:<38} {:>9.3} {:>9.3} {:>9.3} {:>10}",
            result.name,
            result.median_ms,
            result.p95_ms,
            result.p99_ms,
            result.frames,
        );
    }

    if let Some(
        baseline
    ) =
        results.first()
    {
        println!();
        println!(
            "Change versus production baseline"
        );

        for result in
            results
                .iter()
                .skip(
                    1
                )
        {
            let fps_change =
                if baseline.average_fps
                    > f64::EPSILON
                {
                    (
                        result.average_fps
                            / baseline.average_fps
                        - 1.0
                    )
                    * 100.0
                } else {
                    0.0
                };

            println!(
                "  {:<36} {:+8.2}% average FPS",
                result.name,
                fps_change,
            );
        }

        println!(
            "  Baseline measured duration: {:.3} s",
            baseline.elapsed.as_secs_f64(),
        );
    }
}

fn print_sync_comparison(
    results: &[BenchmarkResult],
) {
    let baseline =
        results
            .iter()
            .find(
                |result| {
                    result.name
                        == "Production baseline"
                }
            );

    let fence =
        results
            .iter()
            .find(
                |result| {
                    result.name
                        == "Postprocess + fence wait"
                }
            );

    let flush =
        results
            .iter()
            .find(
                |result| {
                    result.name
                        == "Postprocess + glFlush"
                }
            );

    let no_sync =
        results
            .iter()
            .find(
                |result| {
                    result.name
                        == "Postprocess + no explicit sync"
                }
            );

    let Some(
        baseline
    ) = baseline
    else {
        return;
    };

    println!();
    println!(
        "Synchronization comparison"
    );
    println!(
        "---------------------------"
    );
    println!(
        "{:<32} {:>10} {:>12} {:>12}",
        "Configuration",
        "Avg FPS",
        "vs baseline",
        "P99 ms",
    );
    println!(
        "{}",
        "-".repeat(
            70
        )
    );

    for result in [
        Some(
            baseline
        ),
        fence,
        flush,
        no_sync,
    ]
    .into_iter()
    .flatten()
    {
        let change =
            if baseline.average_fps
                > 0.0
            {
                (
                    result.average_fps
                        / baseline.average_fps
                    - 1.0
                )
                    * 100.0
            } else {
                0.0
            };

        println!(
            "{:<32} {:>10.2} {:>+11.2}% {:>12.3}",
            result.name,
            result.average_fps,
            change,
            result.p99_ms,
        );
    }

    if let Some(
        fence
    ) = fence
    {
        println!();

        if fence.average_fps
            > baseline.average_fps
                * 1.01
        {
            println!(
                "Fence observation: the scoped fence wait outperformed the production glFinish barrier in this run."
            );
        } else {
            println!(
                "Fence observation: the scoped fence wait did not materially outperform the production glFinish barrier in this run."
            );
        }
    }
}


#[allow(clippy::too_many_arguments)]
fn run_gpu_timing_diagnostic(
    duration: Duration,
    benchmark_start: Instant,
    window: &sdl2::video::Window,
    event_pump: &mut sdl2::EventPump,
    program: u32,
    vao: u32,
    cached_uniforms: &UniformLocations,
    texture_manager: &mut crate::manage_textures::TextureManager,
    shader_inputs: &[crate::isf_types::ShaderInput],
    postprocess: &mut crate::postprocess_shader::PostprocessPipeline,
    audio_bands: &std::sync::Arc<
        std::sync::RwLock<
            crate::analyze_audio::AudioBands
        >
    >,
    audio_bloom_active: bool,
    audio_diagnostic: &mut BenchmarkAudioDiagnostic,
) -> Result<GpuTimingSummary, String> {
    let mut queries =
        [0_u32; 2];

    unsafe {
        gl::GenQueries(
            2,
            queries.as_mut_ptr(),
        );
    }

    if queries.iter().any(
        |query| *query == 0
    ) {
        unsafe {
            gl::DeleteQueries(
                2,
                queries.as_ptr(),
            );
        }

        return Err(
            "OpenGL timer-query objects could not be created"
                .to_string()
        );
    }

    let phase_start =
        Instant::now();

    let mut samples =
        Vec::<GpuTimingSample>::new();

    let mut frame_number =
        0_usize;

    while phase_start.elapsed()
        < duration
    {
        for event in
            event_pump.poll_iter()
        {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode:
                        Some(
                            Keycode::Escape
                        ),
                    ..
                } => {
                    unsafe {
                        gl::DeleteQueries(
                            2,
                            queries.as_ptr(),
                        );
                    }

                    return Err(
                        "Benchmark aborted by user"
                            .to_string()
                    );
                }

                _ => {}
            }
        }

        if let Err(error) =
            texture_manager
                .update_animations()
        {
            unsafe {
                gl::DeleteQueries(
                    3,
                    queries.as_ptr(),
                );
            }

            return Err(
                format!(
                    "Unable to update benchmark animated texture: {}",
                    error
                )
            );
        }

        if audio_bloom_active {
            crate::audio_backend::set_audio_required(
                true
            );

            let current_audio_bands =
                audio_bands
                    .read()
                    .ok()
                    .map(|bands| *bands)
                    .unwrap_or_default();

            audio_diagnostic.observe(
                current_audio_bands,
                true,
            );

            postprocess.set_audio_bands(
                current_audio_bands
            );
        }

        let cpu_start =
            Instant::now();

        postprocess.bind_scene_target();

        let (
            scene_width,
            scene_height,
        ) =
            postprocess.scene_dimensions();

        unsafe {
            gl::QueryCounter(
                queries[0],
                gl::TIMESTAMP,
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
                shader_inputs,
            );

            gl::BindVertexArray(
                vao
            );

            if cached_uniforms.time != -1 {
                gl::Uniform1f(
                    cached_uniforms.time,
                    benchmark_start
                        .elapsed()
                        .as_secs_f32(),
                );
            }

            if cached_uniforms.resolution != -1 {
                gl::Uniform3f(
                    cached_uniforms.resolution,
                    scene_width as f32,
                    scene_height as f32,
                    1.0,
                );
            }

            gl::DrawArrays(
                gl::TRIANGLES,
                0,
                3,
            );

            gl::QueryCounter(
                queries[1],
                gl::TIMESTAMP,
            );
        }

        let postprocess_timings =
            postprocess
                .benchmark_present_scene_timed_to_framebuffer(
                    0
                )?;

        window.gl_swap_window();

        unsafe {
            gl::Finish();
        }

        let cpu_wall_ms =
            cpu_start
                .elapsed()
                .as_secs_f64()
                * 1000.0;

        let mut timestamps =
            [0_u64; 2];

        unsafe {
            for index in 0..2 {
                gl::GetQueryObjectui64v(
                    queries[index],
                    gl::QUERY_RESULT,
                    &mut timestamps[index],
                );
            }
        }

        let scene_ns =
            timestamps[1]
                .saturating_sub(
                    timestamps[0]
                );

        let postprocess_ns =
            (
                postprocess_timings.total_ms
                    * 1_000_000.0
            ) as u64;

        let total_ns =
            scene_ns.saturating_add(
                postprocess_ns
            );

        frame_number +=
            1;

        samples.push(
            GpuTimingSample {
                frame:
                    frame_number,
                cpu_wall_ms,
                gpu_scene_ms:
                    scene_ns as f64
                        / 1_000_000.0,
                gpu_postprocess_ms:
                    postprocess_ns as f64
                        / 1_000_000.0,
                gpu_primary_ms:
                    postprocess_timings.primary_ms,
                gpu_bloom_extraction_ms:
                    postprocess_timings.bloom_extraction_ms,
                gpu_bloom_blur_horizontal_ms:
                    postprocess_timings.bloom_blur_horizontal_ms,
                gpu_bloom_blur_vertical_ms:
                    postprocess_timings.bloom_blur_vertical_ms,
                gpu_bloom_composite_ms:
                    postprocess_timings.bloom_composite_ms,
                gpu_dithering_ms:
                    postprocess_timings.dithering_ms,
                gpu_total_ms:
                    total_ns as f64
                        / 1_000_000.0,
            }
        );
    }

    unsafe {
        gl::DeleteQueries(
            2,
            queries.as_ptr(),
        );
    }

    summarize_gpu_timing(
        &samples
    )
}

fn summarize_gpu_timing(
    samples: &[GpuTimingSample],
) -> Result<GpuTimingSummary, String> {
    if samples.is_empty() {
        return Err(
            "GPU timer diagnostic produced no samples"
                .to_string()
        );
    }

    let mut cpu =
        samples
            .iter()
            .map(
                |sample| {
                    sample.cpu_wall_ms
                }
            )
            .collect::<Vec<_>>();

    let mut gpu_total =
        samples
            .iter()
            .map(
                |sample| {
                    sample.gpu_total_ms
                }
            )
            .collect::<Vec<_>>();

    cpu.sort_by(
        |left, right| {
            left.total_cmp(
                right
            )
        }
    );

    gpu_total.sort_by(
        |left, right| {
            left.total_cmp(
                right
            )
        }
    );

    let mean =
        |values: &[f64]| {
            values.iter().sum::<f64>()
                / values.len() as f64
        };

    let gpu_scene_mean_ms =
        samples
            .iter()
            .map(
                |sample| {
                    sample.gpu_scene_ms
                }
            )
            .sum::<f64>()
            / samples.len() as f64;

    let gpu_postprocess_mean_ms =
        samples
            .iter()
            .map(
                |sample| {
                    sample.gpu_postprocess_ms
                }
            )
            .sum::<f64>()
            / samples.len() as f64;

    let stage_mean =
        |selector: fn(&GpuTimingSample) -> f64| {
            samples
                .iter()
                .map(selector)
                .sum::<f64>()
                / samples.len() as f64
        };

    let largest_wall_minus_gpu_ms =
        samples
            .iter()
            .map(
                |sample| {
                    (
                        sample.cpu_wall_ms
                            - sample.gpu_total_ms
                    )
                    .max(
                        0.0
                    )
                }
            )
            .fold(
                0.0_f64,
                f64::max,
            );

    let mut worst_cpu_frames =
        samples.to_vec();

    worst_cpu_frames.sort_by(
        |left, right| {
            right
                .cpu_wall_ms
                .total_cmp(
                    &left.cpu_wall_ms
                )
        }
    );

    worst_cpu_frames.truncate(
        20
    );

    let mut worst_gpu_frames =
        samples.to_vec();

    worst_gpu_frames.sort_by(
        |left, right| {
            right
                .gpu_total_ms
                .total_cmp(
                    &left.gpu_total_ms
                )
        }
    );

    worst_gpu_frames.truncate(
        20
    );

    let mut worst_postprocess_frames = samples.to_vec();
    worst_postprocess_frames.sort_by(
        |left, right| {
            right.gpu_postprocess_ms
                .total_cmp(&left.gpu_postprocess_ms)
        }
    );
    worst_postprocess_frames.truncate(20);

    let mut worst_scene_frames = samples.to_vec();
    worst_scene_frames.sort_by(
        |left, right| {
            right.gpu_scene_ms
                .total_cmp(&left.gpu_scene_ms)
        }
    );
    worst_scene_frames.truncate(20);

    // Diagnostic classifications only; production warning thresholds remain unchanged.
    let external_stall_frames =
        samples.iter()
            .filter(|sample| sample.cpu_wall_ms - sample.gpu_total_ms >= 5.0)
            .count();

    let scene_heavy_frames =
        samples.iter()
            .filter(|sample| sample.gpu_scene_ms >= 5.0)
            .count();

    let postprocess_heavy_frames =
        samples.iter()
            .filter(|sample| sample.gpu_postprocess_ms >= 2.0)
            .count();

    Ok(
        GpuTimingSummary {
            frames:
                samples.len(),
            cpu_mean_ms:
                mean(
                    &cpu
                ),
            cpu_p99_ms:
                percentile_ms(
                    &cpu,
                    0.99,
                ),
            cpu_worst_ms:
                cpu.last()
                    .copied()
                    .unwrap_or(
                        0.0
                    ),
            gpu_scene_mean_ms,
            gpu_postprocess_mean_ms,
            gpu_primary_mean_ms:
                stage_mean(|sample| sample.gpu_primary_ms),
            gpu_bloom_extraction_mean_ms:
                stage_mean(|sample| sample.gpu_bloom_extraction_ms),
            gpu_bloom_blur_horizontal_mean_ms:
                stage_mean(|sample| sample.gpu_bloom_blur_horizontal_ms),
            gpu_bloom_blur_vertical_mean_ms:
                stage_mean(|sample| sample.gpu_bloom_blur_vertical_ms),
            gpu_bloom_composite_mean_ms:
                stage_mean(|sample| sample.gpu_bloom_composite_ms),
            gpu_dithering_mean_ms:
                stage_mean(|sample| sample.gpu_dithering_ms),
            gpu_total_mean_ms:
                mean(
                    &gpu_total
                ),
            gpu_total_p99_ms:
                percentile_ms(
                    &gpu_total,
                    0.99,
                ),
            gpu_total_worst_ms:
                gpu_total.last()
                    .copied()
                    .unwrap_or(
                        0.0
                    ),
            largest_wall_minus_gpu_ms,
            worst_cpu_frames,
            worst_gpu_frames,
            worst_postprocess_frames,
            worst_scene_frames,
            external_stall_frames,
            scene_heavy_frames,
            postprocess_heavy_frames,
        }
    )
}

fn percentile_ms(
    sorted: &[f64],
    percentile: f64,
) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }

    let position =
        (
            percentile
                .clamp(
                    0.0,
                    1.0,
                )
            * (sorted.len() - 1) as f64
        )
        .round()
        as usize;

    sorted[
        position.min(
            sorted.len() - 1
        )
    ]
}

fn print_detailed_gpu_outlier_table(
    title: &str,
    samples: &[GpuTimingSample],
) {
    println!();
    println!("{title}");
    println!(
        "{:>7} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>9}",
        "Frame", "CPU", "Scene", "Primary", "Extract",
        "H-Blur", "V-Blur", "Compos.", "Dither", "GPU total",
    );
    println!("{}", "-".repeat(101));

    for sample in samples {
        println!(
            "{:>7} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>9.3}",
            sample.frame,
            sample.cpu_wall_ms,
            sample.gpu_scene_ms,
            sample.gpu_primary_ms,
            sample.gpu_bloom_extraction_ms,
            sample.gpu_bloom_blur_horizontal_ms,
            sample.gpu_bloom_blur_vertical_ms,
            sample.gpu_bloom_composite_ms,
            sample.gpu_dithering_ms,
            sample.gpu_total_ms,
        );
    }
}


fn print_gpu_timing_summary(
    summary: &GpuTimingSummary,
) {
    println!();
    println!(
        "GPU timing diagnostic results"
    );

    println!(
        "    Frames sampled:                {}",
        summary.frames,
    );

    println!(
        "    CPU wall mean:                 {:.3} ms",
        summary.cpu_mean_ms,
    );

    println!(
        "    CPU wall P99:                  {:.3} ms",
        summary.cpu_p99_ms,
    );

    println!(
        "    CPU wall worst:                {:.3} ms",
        summary.cpu_worst_ms,
    );

    println!(
        "    GPU scene mean:                {:.3} ms",
        summary.gpu_scene_mean_ms,
    );

    println!(
        "    GPU postprocess mean:          {:.3} ms",
        summary.gpu_postprocess_mean_ms,
    );

    println!(
        "        Primary / AA:              {:.3} ms",
        summary.gpu_primary_mean_ms,
    );
    println!(
        "        Bloom extraction:          {:.3} ms",
        summary.gpu_bloom_extraction_mean_ms,
    );
    println!(
        "        Bloom horizontal blur:     {:.3} ms",
        summary.gpu_bloom_blur_horizontal_mean_ms,
    );
    println!(
        "        Bloom vertical blur:       {:.3} ms",
        summary.gpu_bloom_blur_vertical_mean_ms,
    );
    println!(
        "        Bloom composite:           {:.3} ms",
        summary.gpu_bloom_composite_mean_ms,
    );
    println!(
        "        Dithering / final:         {:.3} ms",
        summary.gpu_dithering_mean_ms,
    );

    if summary.gpu_postprocess_mean_ms > 0.0 {
        println!("    Postprocess stage shares:");
        println!("        Primary / AA:              {:>6.1}%",
            summary.gpu_primary_mean_ms / summary.gpu_postprocess_mean_ms * 100.0);
        println!("        Bloom extraction:          {:>6.1}%",
            summary.gpu_bloom_extraction_mean_ms / summary.gpu_postprocess_mean_ms * 100.0);
        println!("        Bloom horizontal blur:     {:>6.1}%",
            summary.gpu_bloom_blur_horizontal_mean_ms / summary.gpu_postprocess_mean_ms * 100.0);
        println!("        Bloom vertical blur:       {:>6.1}%",
            summary.gpu_bloom_blur_vertical_mean_ms / summary.gpu_postprocess_mean_ms * 100.0);
        println!("        Bloom composite:           {:>6.1}%",
            summary.gpu_bloom_composite_mean_ms / summary.gpu_postprocess_mean_ms * 100.0);
        println!("        Dithering / final:         {:>6.1}%",
            summary.gpu_dithering_mean_ms / summary.gpu_postprocess_mean_ms * 100.0);
    }

    println!(
        "    GPU total mean:                {:.3} ms",
        summary.gpu_total_mean_ms,
    );

    println!(
        "    GPU total P99:                 {:.3} ms",
        summary.gpu_total_p99_ms,
    );

    println!(
        "    GPU total worst:               {:.3} ms",
        summary.gpu_total_worst_ms,
    );

    println!(
        "    Largest CPU-wall minus GPU:    {:.3} ms",
        summary.largest_wall_minus_gpu_ms,
    );

    println!();
    println!("Diagnostic frame classifications");
    println!(
        "    External-stall frames (CPU-GPU >= 5 ms): {}",
        summary.external_stall_frames,
    );
    println!(
        "    Scene-heavy frames (scene >= 5 ms):      {}",
        summary.scene_heavy_frames,
    );
    println!(
        "    Postprocess-heavy frames (post >= 2 ms): {}",
        summary.postprocess_heavy_frames,
    );
    println!(
        "    Diagnostic counters only; production Warning/CRITICAL thresholds are unchanged."
    );

    println!();
    println!(
        "Worst CPU-wall frames (correlated measurements)"
    );
    print_gpu_outlier_table(
        &summary.worst_cpu_frames
    );

    println!();
    println!(
        "Worst GPU-total frames (correlated measurements)"
    );
    print_gpu_outlier_table(
        &summary.worst_gpu_frames
    );

    println!();

    print_detailed_gpu_outlier_table(
        "Worst post-processing frames (individual GPU stages)",
        &summary.worst_postprocess_frames,
    );

    print_detailed_gpu_outlier_table(
        "Worst scene-shader frames (individual GPU stages)",
        &summary.worst_scene_frames,
    );

    print_detailed_gpu_outlier_table(
        "Worst CPU-wall frames (individual GPU stages)",
        &summary.worst_cpu_frames,
    );


    if summary.cpu_worst_ms
        > summary.gpu_total_worst_ms
            + 5.0
    {
        println!(
            "    Observation: a substantial CPU/presentation wait occurred that was not matched by GPU render time."
        );
    } else {
        println!(
            "    Observation: CPU-wall and GPU timing remained comparatively close during the diagnostic."
        );
    }
}


fn print_gpu_outlier_table(
    samples: &[GpuTimingSample],
) {
    println!(
        "{:>8} {:>11} {:>11} {:>11} {:>11} {:>12}",
        "Frame",
        "CPU wall",
        "GPU scene",
        "GPU post",
        "GPU total",
        "CPU-GPU",
    );

    println!(
        "{}",
        "-".repeat(
            72
        )
    );

    for sample in
        samples
    {
        println!(
            "{:>8} {:>10.3} {:>10.3} {:>10.3} {:>10.3} {:>11.3}",
            sample.frame,
            sample.cpu_wall_ms,
            sample.gpu_scene_ms,
            sample.gpu_postprocess_ms,
            sample.gpu_total_ms,
            sample.cpu_wall_ms
                - sample.gpu_total_ms,
        );
    }
}


fn print_gl_information() {
    println!(
        "OpenGL environment:"
    );

    println!(
        "    Vendor:   {}",
        gl_string(
            gl::VENDOR
        )
    );

    println!(
        "    Renderer: {}",
        gl_string(
            gl::RENDERER
        )
    );

    println!(
        "    Version:  {}",
        gl_string(
            gl::VERSION
        )
    );

    println!();
}

fn gl_string(
    name: u32,
) -> String {
    unsafe {
        let pointer =
            gl::GetString(
                name
            );

        if pointer.is_null() {
            return "<unavailable>"
                .to_string();
        }

        std::ffi::CStr::from_ptr(
            pointer
                as *const std::os::raw::c_char
        )
        .to_string_lossy()
        .into_owned()
    }
}
