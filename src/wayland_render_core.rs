//! Shared EGL/OpenGL execution utilities for the Wayland Wallpaper and Windowshader pipelines.
//! Pipeline-specific EGL ownership, Wayland event dispatch, pacing, and lifecycle remain
//! in their respective modules. The legacy shared frame loop remains in Wallpaper
//! pending its controlled extraction; this module does not claim full separation.

use std::ffi::CString;
use std::path::Path;
use std::ptr;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;
use crate::wayland_wallpaper::{ActiveWallpaperShader, EglRenderSession, FrameLoopInitialTiming, NativeWallpaperTarget, RenderWaylandDispatch};
use crate::wayland_wallpaper::{
    eglGetProcAddress, eglMakeCurrent, eglSwapBuffers, eglDestroySurface,
    EglDisplay, EglContext, EglWallpaperTarget, EGL_FALSE, EGL_NO_CONTEXT,
    EGL_NO_SURFACE, egl_failure, uniform_location, set_uniform_1f,
    set_uniform_1i, set_uniform_3f, set_uniform_4f,
    WaylandState, WallpaperSurfaceState,
};
use crate::wayland_wallpaper::select_safe_wallpaper_shader;
use std::collections::HashMap;
use std::time::Instant;
use std::thread;
use std::sync::atomic::Ordering;
use wayland_client::Proxy;


// Common GL entry-point loading; called by each pipeline before frame execution.
pub(crate) fn prepare_egl_frame_processing() {
    gl::load_with(
        |name| {
            let Ok(
                symbol
            ) =
                CString::new(
                    name
                )
            else {
                return ptr::null();
            };


            unsafe {
                eglGetProcAddress(
                    symbol.as_ptr()
                )
            }
        }
    );

}

// Stage 2D-2C Step 16 (consolidated): execution-loop drivers are owned by
// the presentation pipeline. Render Core provides one reusable frame step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FrameLoopAction {
    Continue,
    Stop,
}

pub(crate) type FrameStep<'a> = dyn FnMut() -> Result<FrameLoopAction, String> + 'a;
pub(crate) type FrameLoopDriver = for<'a> fn(&mut FrameStep<'a>) -> Result<(), String>;

pub(crate) fn run_wallpaper_frame_loop(step: &mut FrameStep<'_>) -> Result<(), String> {
    loop {
        match step()? {
            FrameLoopAction::Continue => {},
            FrameLoopAction::Stop => return Ok(()),
        }
    }
}

// Shared frame implementation. The owning pipeline decides when to invoke it.
pub(crate) fn execute_shared_egl_frame_processing<S: RenderWaylandDispatch + 'static>(

    egl_session: &mut EglRenderSession,
    event_queue: &mut wayland_client::EventQueue<S>,
    state: &mut S,
    native_targets: &mut Vec<NativeWallpaperTarget>,
    active_shader: &ActiveWallpaperShader,
    shader_manager: &mut crate::manage_shader::ShaderManager,
    wallpaper_directory: &Path,
    shader_interval: Option<Duration>,
    runtime: &crate::define_wallpaper::WallpaperRuntime,
    running: &Arc<AtomicBool>,
    control: &crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
    initial_timing: FrameLoopInitialTiming,
) -> Result<(), String> {
    execute_egl_frame_processing_with_driver(
        egl_session, event_queue, state, native_targets, active_shader,
        shader_manager, wallpaper_directory, shader_interval, runtime,
        running, control, initial_timing, run_wallpaper_frame_loop,
    )
}

pub(crate) fn execute_egl_frame_processing_with_driver<S: RenderWaylandDispatch + 'static>(
    egl_session: &mut EglRenderSession,
    event_queue: &mut wayland_client::EventQueue<S>,
    state: &mut S,
    native_targets: &mut Vec<NativeWallpaperTarget>,
    active_shader: &ActiveWallpaperShader,
    shader_manager: &mut crate::manage_shader::ShaderManager,
    wallpaper_directory: &Path,
    shader_interval: Option<Duration>,
    runtime: &crate::define_wallpaper::WallpaperRuntime,
    running: &Arc<AtomicBool>,
    control: &crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
    initial_timing: FrameLoopInitialTiming,
    loop_driver: FrameLoopDriver,
) -> Result<(), String> {


    let render_result =
        render_mirror_frames_with_driver(
            egl_session.display,
            egl_session.context,
            &mut egl_session.targets,
            event_queue,
            state,
            native_targets,
            active_shader,
            shader_manager,
            wallpaper_directory,
            shader_interval,
            runtime,
            running,
            control,
            initial_timing,
            loop_driver,
        );


    // The caller owns EGL session teardown, including error paths.
    // The session remains alive while GL frame resources are released.
    let egl_major = egl_session.major;
    let egl_minor = egl_session.minor;

    render_result?;


    println!(
        "Native EGL mirror wallpaper renderer stopped:"
    );


    println!(
        "    EGL version: {}.{}",
        egl_major,
        egl_minor
    );


    println!(
        "    OpenGL context: 3.3 core"
    );


    println!(
        "    Rendered targets: {}",
        native_targets.len()
    );


    println!(
        "    Shutdown reason: {}",
        if state
            .surfaces()
            .iter()
            .any(
                |surface_state| {
                    surface_state.closed
                }
            )
        {
            "compositor closed a layer surface"
        } else if native_targets
            .is_empty()
        {
            "all wallpaper outputs disconnected"
        } else {
            "Screenshaver shutdown requested"
        }
    );


    Ok(
        ()
    )
}


// Stage 2D-2C Step 15B: shared GL frame execution and its associated
// resource/rotation helpers live in the neutral rendering core.
pub(crate) fn render_mirror_frames_with_driver<S: RenderWaylandDispatch + 'static>(
    display: EglDisplay,
    context: EglContext,
    egl_targets: &mut Vec<EglWallpaperTarget>,
    event_queue: &mut wayland_client::EventQueue<S>,
    state: &mut S,
    native_targets: &mut Vec<NativeWallpaperTarget>,
    active_shader: &ActiveWallpaperShader,
    shader_manager: &mut crate::manage_shader::ShaderManager,
    wallpaper_directory: &Path,
    shader_interval: Option<Duration>,
    runtime: &crate::define_wallpaper::WallpaperRuntime,
    running: &Arc<AtomicBool>,
    control: &crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
    initial_timing: FrameLoopInitialTiming,
    loop_driver: FrameLoopDriver,
) -> Result<(), String> {

    let mut program =
        crate::compile_shader::build_program(
            crate::define_constants::VERTEX_SHADER,
            &active_shader.source,
        )?;


    let mut texture_manager =
        crate::manage_textures::TextureManager::new(
            runtime.texture_policy.clone()
        );


    if let Err(error) =
        texture_manager.prepare_for_policy_with_path(
            active_shader.policy_id,
            &active_shader.shader_name,
            Some(
                active_shader.source_path.as_path()
            ),
            active_shader.channel_usage,
        )
    {
        unsafe {
            gl::DeleteProgram(
                program
            );
        }


        return Err(
            format!(
                "Unable to prepare textures for wallpaper shader '{}': {}",
                active_shader.shader_name,
                error,
            )
        );
    }


    texture_manager.configure_program(
        program
    );


    let mut animation_speed =
        runtime.animation_speed_policy
            .animation_speed_for_policy(
                active_shader.policy_id,
                &active_shader.shader_name,
                Some(
                    active_shader.source_path.as_path()
                ),
                None,
            );


    let FrameLoopInitialTiming {
        mut rendered_fps,
        mut frame_duration,
        mut next_frame_deadline,
    } = initial_timing;


    let mut notification_state =
        crate::notify_wallpaper::WallpaperNotificationState::new();


    notify_active_wallpaper(
        &mut notification_state,
        true,
        runtime.notifications,
        active_shader,
        &texture_manager,
        rendered_fps,
        animation_speed,
        crate::fps_monitor::FpsWarningState::Normal,
    );


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

        gl::UseProgram(
            program
        );
    }


    crate::apply_shader_inputs::apply(
        program,
        &active_shader.shader_inputs,
    );


    // FPS Warning/CRITICAL must measure GPU rendering work rather than CPU
    // wall-clock time.  This is especially important for Windowshader: a
    // compositor may throttle or defer a non-visible xdg_toplevel while its
    // workspace is inactive.  CPU elapsed time across that lifecycle event is
    // not evidence that the shader itself is slow.
    let mut performance_queries =
        [0_u32; 2];

    unsafe {
        gl::GenQueries(
            2,
            performance_queries.as_mut_ptr(),
        );
    }


    let mut postprocess_pipelines =
        HashMap::with_capacity(
            native_targets.len()
        );


    for target in native_targets.iter() {
        let width =
            u32::try_from(
                target.width
            )
            .map_err(
                |_| {
                    format!(
                        "Wallpaper target {} has an invalid post-processing width: {}",
                        target.info.registry_name,
                        target.width,
                    )
                }
            )?;


        let height =
            u32::try_from(
                target.height
            )
            .map_err(
                |_| {
                    format!(
                        "Wallpaper target {} has an invalid post-processing height: {}",
                        target.info.registry_name,
                        target.height,
                    )
                }
            )?;


        let postprocess_profile =
            runtime.postprocess_policy.profile_for_policy(
                active_shader.policy_id,
                &active_shader.shader_name,
                Some(
                    active_shader.source_path.as_path()
                ),
            );

        let pipeline =
            crate::postprocess_shader::PostprocessPipeline::new(
                width,
                height,
                postprocess_profile,
            )?;


        postprocess_pipelines.insert(
            target.info.registry_name,
            pipeline,
        );
    }


    // Per-output diagnostic overlays; resources are owned by the active EGL context.
    let mut renderer_status_panels:
        HashMap<u32, crate::display_overlay::RendererStatusPanel> = HashMap::new();

    let mut lyrics_overlays:
        HashMap<u32, crate::display_overlay::OpenGlOverlay> =
            HashMap::new();

    let mut lyrics_overlay_states:
        HashMap<u32, (crate::manage_lyrics::LyricsState, u32, u32)> =
            HashMap::new();


    let initial_postprocess_profile =
        runtime.postprocess_policy
            .profile_for_policy(
                active_shader.policy_id,
                &active_shader.shader_name,
                Some(
                    active_shader.source_path.as_path()
                ),
            );


    let audio_demand = crate::audio_backend::register_audio_demand();

    let mut audio_required =
        matches!(
            initial_postprocess_profile.bloom.name(),
            "audio" | "spectral" | "loudness"
        ) || initial_postprocess_profile.audio_motion.is_enabled();


    let mut audio_motion_effect =
        initial_postprocess_profile.audio_motion;


    let mut audio_motion_state =
        crate::render_audio_motion::AudioMotionState::default();


    let mut audio_motion_lyrics_manager =
        if matches!(
            audio_motion_effect,
            crate::render_audio_motion::AudioMotionEffect::WooferFromHell
        ) {
            crate::manage_lyrics::LyricsManager::start().ok()
        } else {
            None
        };


    let mut last_audio_motion_update =
        Instant::now();


    let mut i_time =
        uniform_location(
            program,
            "iTime",
        )?;


    let mut i_resolution =
        uniform_location(
            program,
            "iResolution",
        )?;


    let mut i_mouse =
        uniform_location(
            program,
            "iMouse",
        )?;


    let mut i_frame =
        uniform_location(
            program,
            "iFrame",
        )?;


    let mut start_time =
        Instant::now();


    let mut last_shader_switch =
        Instant::now();


    let mut frame =
        0_i32;


    let mut current_shader =
        active_shader.clone();


    // The initial wallpaper has now loaded, compiled, prepared textures,
    // created post-processing pipelines, and resolved its uniforms. Persist
    // Ordered position only after reaching this accepted render state.
    shader_manager.record_rendered_policy(
        current_shader.policy_id
    );


    let mut frame_times =
        crate::fps_monitor::FrameTimeWindow::new();


    let mut fps_warning_state =
        crate::fps_monitor::FpsWarningState::Normal;


    let mut paused =
        false;


    let mut pause_started: Option<Instant> =
        None;


    // Track the most recent pause interval so an outstanding compositor
    // callback cannot charge the same inactive time to both clocks twice.

    // Diagnostics only: preserve compositor-driven pacing. In particular,
    // never synthesize a wl_surface.frame completion after a screensaver pause.
    // Windowshader owns its callback, rotation and presentation bookkeeping.
    // Keep it grouped while the shared GL frame executor is extracted in stages.
    let mut windowshader_state = crate::wayland_windowshader::WindowshaderFrameLoopState::new(
        last_shader_switch,
        state.presentation_diagnostics(native_targets).configure_count,
        next_frame_deadline,
    );
    let windowed_rotation = runtime.display_format
        == crate::manage_configuration::WallpaperDisplayFormat::Windowed;


    let status_pipeline = if runtime.display_format
        == crate::manage_configuration::WallpaperDisplayFormat::Windowed
    {
        crate::renderer_status::Pipeline::Windowshader
    } else {
        crate::renderer_status::Pipeline::Wallpaper
    };
    let status_enabled = crate::renderer_status::enabled();
    if status_enabled {
        crate::renderer_status::publish(status_pipeline, crate::renderer_status::State::Starting);
    }
    let mut status_heartbeat = Instant::now() - Duration::from_secs(2);

    let result = loop_driver(&mut || -> Result<FrameLoopAction, String> {

            control.diagnostic_checkpoint(1);
            S::pump_events(event_queue, state)?;


            control.diagnostic_checkpoint(2);
            if windowed_rotation {
                let diagnostics = state.presentation_diagnostics(native_targets);
                let configure_pending = state.surfaces().iter().any(|surface| {
                    surface.registry_name == 0 && surface.configured.is_some()
                });
                windowshader_state.report_presentation_diagnostics(
                    &diagnostics,
                    configure_pending,
                    paused,
                    control.pause_requested(),
                    shader_interval,
                    &current_shader.shader_name,
                    current_shader.policy_id,
                    program,
                    frame,
                );
            }
            remove_disconnected_targets_from_names(
                display,
                context,
                state.removed_outputs(),
                egl_targets,
                native_targets,
                &mut postprocess_pipelines,
            )?;
            // Prune records for disconnected outputs after EGL cleanup.
            state.surfaces().retain(|surface| native_targets.iter().any(|target| target.info.registry_name == surface.registry_name));

            if native_targets
                .is_empty()
            {
                println!(
                    "All wallpaper outputs have been disconnected."
                );


                return Ok(FrameLoopAction::Stop);
            }


            apply_pending_target_resizes_from_states(
                state.surfaces(),
                native_targets,
            )?;


            let compositor_closed = state
                .surfaces()
                .iter()
                .any(|surface_state| surface_state.closed);

            if compositor_closed {
                // Windowshader's XDG close quits its supervisor; Wallpaper's
                // layer-surface close only ends this presentation loop.
                if S::compositor_close_requests_shutdown() {
                    running.store(false, Ordering::SeqCst);
                }
                return Ok(FrameLoopAction::Stop);
            }
            if !running.load(Ordering::SeqCst) {
                return Ok(FrameLoopAction::Stop);
            }


            if let Some(reload) =
                control.take_policy_reload()
            {
                let mut replacement_texture_manager =
                    crate::manage_textures::TextureManager::new(
                        reload.texture_policy.clone()
                    );

                match replacement_texture_manager.prepare_for_policy_with_path(
                    current_shader.policy_id,
                    &current_shader.shader_name,
                    Some(
                        current_shader.source_path.as_path()
                    ),
                    current_shader.channel_usage,
                ) {
                    Ok(()) => {
                        let replacement_profile =
                            reload.postprocess_policy
                                .profile_for_policy(
                                    current_shader.policy_id,
                                    &current_shader.shader_name,
                                    Some(
                                        current_shader.source_path.as_path()
                                    ),
                                );

                        let mut profile_error =
                            None;

                        for pipeline in
                            postprocess_pipelines.values_mut()
                        {
                            if let Err(error) =
                                pipeline.set_profile(
                                    replacement_profile
                                )
                            {
                                profile_error =
                                    Some(error);
                                break;
                            }
                        }

                        if let Some(error) =
                            profile_error
                        {
                            replacement_texture_manager.delete_all();
                            println!(
                                "Unable to reload Wayland active wallpaper policy; keeping the previous settings: {}",
                                error,
                            );
                        } else {
                            replacement_texture_manager.configure_program(
                                program
                            );

                            texture_manager.delete_all();
                            texture_manager =
                                replacement_texture_manager;

                            audio_required =
                                matches!(
                                    replacement_profile.bloom.name(),
                                    "audio" | "spectral" | "loudness"
                                ) || replacement_profile.audio_motion.is_enabled();

                            audio_motion_effect =
                                replacement_profile.audio_motion;

                            if matches!(
                                audio_motion_effect,
                                crate::render_audio_motion::AudioMotionEffect::WooferFromHell
                            ) && audio_motion_lyrics_manager.is_none()
                            {
                                audio_motion_lyrics_manager =
                                    crate::manage_lyrics::LyricsManager::start().ok();
                            }

                            if !matches!(
                                audio_motion_effect,
                                crate::render_audio_motion::AudioMotionEffect::WooferFromHell
                            ) {
                                audio_motion_lyrics_manager = None;
                            }

                            if !audio_motion_effect.is_enabled() {
                                audio_motion_lyrics_manager = None;
                                audio_motion_state.reset();

                                for pipeline in
                                    postprocess_pipelines.values_mut()
                                {
                                    pipeline.set_audio_motion_scale(
                                        1.0
                                    );
                                }
                            }

                            last_audio_motion_update =
                                Instant::now();

                            animation_speed =
                                reload.animation_speed_policy
                                    .animation_speed_for_policy(
                                        current_shader.policy_id,
                                        &current_shader.shader_name,
                                        Some(
                                            current_shader.source_path.as_path()
                                        ),
                                        None,
                                    );

                            rendered_fps =
                                reload.fps_policy
                                    .rendered_fps_for_policy(
                                        current_shader.policy_id,
                                        &current_shader.shader_name,
                                        Some(
                                            current_shader.source_path.as_path()
                                        ),
                                        None,
                                    );

                            frame_duration =
                                frame_duration_for_fps(
                                    rendered_fps
                                );

                            if windowed_rotation {
                                windowshader_state.reset_presentation_deadline();
                            } else {
                                S::reset_frame_deadline(&mut next_frame_deadline);
                            }

                            frame_times.clear();
                            fps_warning_state =
                                crate::fps_monitor::FpsWarningState::Normal;

                            notify_active_wallpaper(
                                &mut notification_state,
                                false,
                                runtime.notifications,
                                &current_shader,
                                &texture_manager,
                                rendered_fps,
                                animation_speed,
                                fps_warning_state,
                            );

                            println!(
                                "Wayland active wallpaper policy reloaded."
                            );
                        }
                    }

                    Err(error) => {
                        println!(
                            "Unable to reload Wayland active wallpaper texture policy; keeping the previous settings: {}",
                            error,
                        );
                    }
                }
            }

            control.diagnostic_checkpoint(3);
            if control.pause_requested() {
                if !paused {
                    crate::logger::warning(
                        &crate::locate_paths::runtime_log_path(),
                        "[AMBIENT_HANDOFF_DIAG] Wayland renderer detected pause request",
                    );
                    control.acknowledge_pause_detected();
                    state.on_pause_entering();
                    control.diagnostic_checkpoint(8);
                    state.on_pause_entered(native_targets);
                    paused =
                        true;


                    let pause_began_at = Instant::now();
                    if windowed_rotation {
                        windowshader_state.on_pause_entering(pause_began_at);
                    } else {
                        pause_started = Some(pause_began_at);
                    }
                    control.acknowledge_paused();
                    if status_enabled {
                        crate::renderer_status::publish(status_pipeline, crate::renderer_status::State::Paused);
                    }


                    println!(
                        "Wayland wallpaper rendering paused."
                    );
                }


                thread::sleep(
                    Duration::from_millis(10)
                );


                return Ok(FrameLoopAction::Continue);
            }


            if paused {
                let pause_ended_at = Instant::now();
                // Windowshader owns its pause-release decision and rotation
                // adjustment; Render Core applies only the animation shift.
                let windowshader_release = if windowed_rotation {
                    Some(windowshader_state.release_pause(pause_ended_at))
                } else {
                    None
                };
                if let Some(release) = windowshader_release {
                    if let Some(duration) = release.animation_shift {
                        start_time += duration;
                    }
                } else if let Some(paused_at) = pause_started.take() {
                    let duration = pause_ended_at.duration_since(paused_at);
                    start_time += duration;
                    last_shader_switch += duration;
                }


                if windowed_rotation {
                                windowshader_state.reset_presentation_deadline();
                            } else {
                                S::reset_frame_deadline(&mut next_frame_deadline);
                            }


                frame_times.clear();
                // The pause has ended even if the compositor has not yet
                // allowed another windowshader_state.presentation. Keep the separate pending
                // flag until a successful EGL swap acknowledges resumption.
                paused = false;
                if windowshader_release.is_some_and(|release| release.notify_dispatch) {
                    state.on_pause_released();
                }
            }


            control.diagnostic_checkpoint(4);
            // Do not use xdg_toplevel::State::Activated as a visibility
            // signal.  Activated represents compositor activation/focus, so
            // Mango legitimately removes it when the pointer/keyboard focus
            // leaves a still-visible Windowshader.  Instead, pace Windowed
            // presentation with wl_surface.frame callbacks.  A visible,
            // unfocused Windowshader continues receiving callbacks, while a
            // surface on a non-visible workspace can stop receiving them.
            // The render worker therefore remains in this event/control loop
            // instead of entering a compositor-throttled eglSwapBuffers().
            let windowed_frame_ready = state.frames_ready_for_presentation(native_targets);
            let frame_eligibility = windowshader_state.assess_frame_eligibility(
                windowed_frame_ready,
                shader_interval,
                windowed_rotation,
            );

            if let crate::wayland_windowshader::WindowshaderFrameEligibility::Blocked {
                initial_warning, recurring_warning, post_pause_warning,
            } = frame_eligibility {
                // Windowshader owns blocked-callback diagnostics and waiting.
                // Render Core does not infer frame readiness or alter its clocks.
                windowshader_state.handle_blocked_callback(
                    state,
                    native_targets,
                    initial_warning,
                    recurring_warning,
                    post_pause_warning,
                );
                return Ok(FrameLoopAction::Continue);
            }

            let inactive_since = match frame_eligibility {
                crate::wayland_windowshader::WindowshaderFrameEligibility::Ready {
                    inactive_since,
                } => inactive_since,
                crate::wayland_windowshader::WindowshaderFrameEligibility::Blocked { .. } => {
                    unreachable!("blocked callback returns before rendering")
                }
            };

            // Windowshader owns the callback-recovery clock accounting.
            // Render Core retains the shared animation/FPS reset operations.
            let shader_anchor = if windowed_rotation {
                windowshader_state.rotation.last_shader_switch
            } else {
                last_shader_switch
            };
            if let Some(recovery) = windowshader_state.account_callback_recovery(
                inactive_since,
                shader_anchor,
                start_time,
                windowed_rotation,
                windowed_frame_ready,
                paused,
                state.presentation_diagnostics(native_targets).activated,
            ) {
                start_time += recovery.animation_shift;
                if !windowed_rotation {
                    last_shader_switch += recovery.rotation_shift;
                }
                if windowed_rotation {
                                windowshader_state.reset_presentation_deadline();
                            } else {
                                S::reset_frame_deadline(&mut next_frame_deadline);
                            }
                frame_times.clear();
                fps_warning_state = crate::fps_monitor::FpsWarningState::Normal;
            }

            if !paused
                && shader_interval
                    .map(
                        |interval| {
                            if windowed_rotation {
                                windowshader_state.rotation.rotation_due(Some(interval))
                            } else {
                                last_shader_switch.elapsed() >= interval
                            }
                        }
                    )
                    .unwrap_or(
                        false
                    )
            {
                if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed {
                    windowshader_state.rotation.windowshader_rotation_attempts = windowshader_state.rotation.windowshader_rotation_attempts.wrapping_add(1);
                    crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                        &format!("[WINDOWSHADER_ROTATION_DIAG] Rotation attempt={} previous_shader='{}' policy_id={} elapsed_ms={} interval_ms={:?} frame_ready={} program={}",
                            windowshader_state.rotation.windowshader_rotation_attempts, current_shader.shader_name,
                            current_shader.policy_id, (if windowed_rotation { windowshader_state.rotation.last_shader_switch } else { last_shader_switch }).elapsed().as_millis(),
                            shader_interval.map(|interval| interval.as_millis()), windowed_frame_ready, program));
                }
                if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed {
                    windowshader_state.rotation.rotation_clock_last_shift_reason = "rotation_attempt_reset";
                    windowshader_state.rotation.rotation_clock_last_shift_ms = (if windowed_rotation { windowshader_state.rotation.last_shader_switch } else { last_shader_switch }).elapsed().as_millis();
                    crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                        &format!("[WINDOWSHADER_CLOCK_DIAG] reason=rotation_attempt_reset elapsed_before_ms={} interval_ms={:?} attempts={}",
                            windowshader_state.rotation.rotation_clock_last_shift_ms, shader_interval.map(|v| v.as_millis()), windowshader_state.rotation.windowshader_rotation_attempts));
                }
                if windowed_rotation {
                    windowshader_state.rotation.last_shader_switch = Instant::now();
                } else {
                    last_shader_switch = Instant::now();
                }


                if let Some(
                    first_target
                ) = egl_targets.first()
                {
                    if unsafe {
                        eglMakeCurrent(
                            display,
                            first_target.surface,
                            first_target.surface,
                            context,
                        )
                    } == EGL_FALSE
                    {
                        return Err(
                            egl_failure(
                                "eglMakeCurrent before switching wallpaper shaders"
                            )
                        );
                    }
                }


                match select_safe_wallpaper_shader(
                    shader_manager,
                    wallpaper_directory,
                ) {
                    Ok(
                        next_shader
                    ) => {
                        match crate::compile_shader::build_program(
                            crate::define_constants::VERTEX_SHADER,
                            &next_shader.source,
                        ) {
                            Ok(
                                next_program
                            ) => {
                                let mut next_texture_manager =
                                    crate::manage_textures::TextureManager::new(
                                        runtime.texture_policy.clone()
                                    );


                                if let Err(
                                    error
                                ) =
                                    next_texture_manager.prepare_for_policy_with_path(
                                        next_shader.policy_id,
                                        &next_shader.shader_name,
                                        Some(
                                            next_shader.source_path.as_path()
                                        ),
                                        next_shader.channel_usage,
                                    )
                                {
                                    println!(
                                        "Wallpaper shader texture preparation failed; keeping the current shader:"
                                    );


                                    println!(
                                        "    Shader: {}",
                                        next_shader.shader_name
                                    );


                                    println!(
                                        "    Error: {}",
                                        error
                                    );


                                    println!();


                                    unsafe {
                                        gl::DeleteProgram(
                                            next_program
                                        );
                                    }


                                    shader_manager.remove_shader(
                                        &next_shader.manager_name
                                    );


                                    return Ok(FrameLoopAction::Continue);
                                }


                                next_texture_manager.configure_program(
                                    next_program
                                );


                                unsafe {
                                    gl::UseProgram(
                                        next_program
                                    );
                                }


                                crate::apply_shader_inputs::apply(
                                    next_program,
                                    &next_shader.shader_inputs,
                                );


                                let next_i_time =
                                    uniform_location(
                                        next_program,
                                        "iTime",
                                    )?;


                                let next_i_resolution =
                                    uniform_location(
                                        next_program,
                                        "iResolution",
                                    )?;


                                let next_i_mouse =
                                    uniform_location(
                                        next_program,
                                        "iMouse",
                                    )?;


                                let next_i_frame =
                                    uniform_location(
                                        next_program,
                                        "iFrame",
                                    )?;


                                texture_manager.delete_all();


                                texture_manager =
                                    next_texture_manager;


                                unsafe {
                                    gl::UseProgram(
                                        0
                                    );

                                    gl::DeleteProgram(
                                        program
                                    );
                                }


                                program =
                                    next_program;


                                i_time =
                                    next_i_time;


                                i_resolution =
                                    next_i_resolution;


                                i_mouse =
                                    next_i_mouse;


                                i_frame =
                                    next_i_frame;


                                start_time =
                                    Instant::now();


                                frame =
                                    0;


                                animation_speed =
                                    runtime.animation_speed_policy
                                        .animation_speed_for_policy(
                                            next_shader.policy_id,
                                            &next_shader.shader_name,
                                            Some(
                                                next_shader.source_path.as_path()
                                            ),
                                            None,
                                        );


                                rendered_fps =
                                    runtime.fps_policy.rendered_fps_for_policy(
                                        next_shader.policy_id,
                                        &next_shader.shader_name,
                                        Some(
                                            next_shader.source_path.as_path()
                                        ),
                                        None,
                                    );


                                frame_duration =
                                    frame_duration_for_fps(
                                        rendered_fps
                                    );


                                if windowed_rotation {
                                windowshader_state.reset_presentation_deadline();
                            } else {
                                S::reset_frame_deadline(&mut next_frame_deadline);
                            }


                                current_shader =
                                    next_shader.clone();
                                if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed {
                                    windowshader_state.rotation.windowshader_rotation_successes = windowshader_state.rotation.windowshader_rotation_successes.wrapping_add(1);
                                    crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                                        &format!("[WINDOWSHADER_ROTATION_DIAG] Rotation succeeded shader='{}' policy_id={} program={} successes={}",
                                            current_shader.shader_name, current_shader.policy_id, program, windowshader_state.rotation.windowshader_rotation_successes));
                                }


                                let postprocess_profile =
                                    runtime.postprocess_policy
                                        .profile_for_policy(
                                            current_shader.policy_id,
                                            &current_shader.shader_name,
                                            Some(
                                                current_shader.source_path.as_path()
                                            ),
                                        );


                                audio_required =
                                    matches!(
                                        postprocess_profile.bloom.name(),
                                        "audio" | "spectral" | "loudness"
                                    ) || postprocess_profile.audio_motion.is_enabled();


                                audio_motion_effect =
                                    postprocess_profile.audio_motion;


                                if matches!(
                                    audio_motion_effect,
                                    crate::render_audio_motion::AudioMotionEffect::WooferFromHell
                                ) && audio_motion_lyrics_manager.is_none()
                                {
                                    audio_motion_lyrics_manager =
                                        crate::manage_lyrics::LyricsManager::start().ok();
                                }

                                if !matches!(
                                    audio_motion_effect,
                                    crate::render_audio_motion::AudioMotionEffect::WooferFromHell
                                ) {
                                    audio_motion_lyrics_manager = None;
                                }


                                if !audio_motion_effect.is_enabled() {
                                    audio_motion_lyrics_manager =
                                        None;

                                    audio_motion_state.reset();
                                }


                                last_audio_motion_update =
                                    Instant::now();


                                for pipeline in
                                    postprocess_pipelines.values_mut()
                                {
                                    pipeline.set_profile(
                                        postprocess_profile
                                    )?;
                                }


                                runtime.tray_status.set_active(
                                    current_shader.policy_id,
                                    current_shader.shader_name.clone(),
                                    current_shader.source_path.clone(),
                                );


                                shader_manager.record_rendered_policy(
                                    current_shader.policy_id
                                );


                                frame_times.clear();


                                fps_warning_state =
                                    crate::fps_monitor::FpsWarningState::Normal;


                                notify_active_wallpaper(
                                    &mut notification_state,
                                    true,
                                    runtime.notifications,
                                    &current_shader,
                                    &texture_manager,
                                    rendered_fps,
                                    animation_speed,
                                    fps_warning_state,
                                );


                                println!(
                                    "Wallpaper shader changed:"
                                );


                                println!(
                                    "    Shader: {}",
                                    next_shader.shader_name
                                );


                                println!(
                                    "    FPS: {}",
                                    rendered_fps
                                );


                                println!(
                                    "    Animation speed: {:.3}x",
                                    animation_speed
                                );


                                if let Some(
                                    interval
                                ) = shader_interval
                                {
                                    println!(
                                        "    Interval: {} seconds",
                                        interval.as_secs()
                                    );
                                }


                                println!();
                            }


                            Err(
                                error
                            ) => {
                                if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed {
                                    windowshader_state.rotation.windowshader_rotation_failures = windowshader_state.rotation.windowshader_rotation_failures.wrapping_add(1);
                                    crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                                        &format!("[WINDOWSHADER_ROTATION_DIAG] Compilation failed shader='{}' error={}", next_shader.shader_name, error));
                                }
                                println!(
                                    "Wallpaper shader compilation failed; keeping the current shader:"
                                );


                                println!(
                                    "    Shader: {}",
                                    next_shader.shader_name
                                );


                                println!(
                                    "    Error: {}",
                                    error
                                );


                                println!();


                                shader_manager.remove_shader(
                                    &next_shader.manager_name
                                );
                            }
                        }
                    }


                    Err(
                        error
                    ) => {
                        if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed {
                            windowshader_state.rotation.windowshader_rotation_failures = windowshader_state.rotation.windowshader_rotation_failures.wrapping_add(1);
                            crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                                &format!("[WINDOWSHADER_ROTATION_DIAG] Selection failed: {}", error));
                        }
                        println!(
                            "Wallpaper shader selection failed; keeping the current shader:"
                        );


                        println!(
                            "    Error: {}",
                            error
                        );


                        println!();
                    }
                }
            }


            let elapsed =
                start_time.elapsed();


            unsafe {
                gl::QueryCounter(
                    performance_queries[0],
                    gl::TIMESTAMP,
                );
            }


            audio_demand.set_required(
                audio_required
            );


            let current_audio_bands =
                runtime.audio_bands
                    .as_ref()
                    .and_then(
                        |shared| {
                            shared
                                .read()
                                .ok()
                                .map(
                                    |bands| *bands
                                )
                        }
                    )
                    .unwrap_or_default();


            let now =
                Instant::now();

            let audio_motion_frame_seconds =
                now.duration_since(
                    last_audio_motion_update
                )
                .as_secs_f32();

            last_audio_motion_update =
                now;

            let mut audio_motion_scale =
                1.0_f32;

            let mut audio_motion_fft_trace =
                [0.0_f32; crate::analyze_audio::AUDIO_MOTION_TRACE_CHANNELS];

            match audio_motion_effect {
                crate::render_audio_motion::AudioMotionEffect::WooferFromHell => {
                    let spectrum =
                        crate::analyze_audio::shared_audio_motion_spectrum()
                            .read()
                            .ok()
                            .map(
                                |value| *value
                            )
                            .unwrap_or_default();

                    let vocal_timing =
                        crate::manage_lyrics::shared_audio_motion_vocal_timing_state()
                            .lock()
                            .ok()
                            .map(
                                |value| value.clone()
                            )
                            .unwrap_or_default();

                    audio_motion_scale =
                        audio_motion_state.update(
                            spectrum,
                            audio_motion_frame_seconds,
                            &vocal_timing,
                        );
                }

                crate::render_audio_motion::AudioMotionEffect::FftMirrorWarp => {
                    let spectrum =
                        crate::analyze_audio::shared_audio_motion_fft_trace()
                            .read()
                            .ok()
                            .map(
                                |value| *value
                            )
                            .unwrap_or_default();

                    audio_motion_fft_trace =
                        audio_motion_state.update_fft_mirror_warp(
                            spectrum,
                            audio_motion_frame_seconds,
                        );
                }

                crate::render_audio_motion::AudioMotionEffect::PolarPropeller => {
                    let spectrum =
                        crate::analyze_audio::shared_audio_motion_fft_trace()
                            .read()
                            .ok()
                            .map(
                                |value| *value
                            )
                            .unwrap_or_default();

                    audio_motion_fft_trace =
                        audio_motion_state.update_polar_propeller(
                            spectrum,
                            audio_motion_frame_seconds,
                        );
                }

                crate::render_audio_motion::AudioMotionEffect::Off => {
                    audio_motion_state.reset();
                }
            }


            let frame_queue_handle =
                event_queue.handle();


            for (
                egl_target,
                native_target,
            ) in egl_targets
                .iter()
                .zip(
                    native_targets.iter()
                )
            {
                if unsafe {
                    eglMakeCurrent(
                        display,
                        egl_target.surface,
                        egl_target.surface,
                        context,
                    )
                } == EGL_FALSE
                {
                    return Err(
                        egl_failure(
                            "eglMakeCurrent"
                        )
                    );
                }


                let postprocess =
                    postprocess_pipelines
                        .get_mut(
                            &native_target.info.registry_name
                        )
                        .ok_or_else(
                            || {
                                format!(
                                    "No post-processing pipeline exists for wallpaper target {}",
                                    native_target.info.registry_name,
                                )
                            }
                        )?;


                postprocess.set_audio_bands(
                    current_audio_bands
                );


                postprocess.set_audio_motion_scale(
                    audio_motion_scale
                );

                postprocess.set_audio_motion_fft_trace(
                    audio_motion_fft_trace
                );
                postprocess.set_audio_motion_rotation_turns(
                    audio_motion_state.polar_rotation_turns()
                );


                let target_width =
                    u32::try_from(
                        native_target.width
                    )
                    .map_err(
                        |_| {
                            format!(
                                "Wallpaper target {} has an invalid width: {}",
                                native_target.info.registry_name,
                                native_target.width,
                            )
                        }
                    )?;


                let target_height =
                    u32::try_from(
                        native_target.height
                    )
                    .map_err(
                        |_| {
                            format!(
                                "Wallpaper target {} has an invalid height: {}",
                                native_target.info.registry_name,
                                native_target.height,
                            )
                        }
                    )?;


                postprocess.resize(
                    target_width,
                    target_height,
                )?;


                postprocess.bind_scene_target();


                let (
                    scene_width,
                    scene_height,
                ) =
                    postprocess.scene_dimensions();


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

                    gl::ColorMask(
                        gl::TRUE,
                        gl::TRUE,
                        gl::TRUE,
                        gl::FALSE,
                    );

                    gl::UseProgram(
                        program
                    );
                }


                texture_manager.bind_channels();


                unsafe {
                    set_uniform_1f(
                        i_time,
                        elapsed.as_secs_f32()
                            * animation_speed,
                    );

                    set_uniform_3f(
                        i_resolution,
                        scene_width as f32,
                        scene_height as f32,
                        1.0,
                    );

                    set_uniform_4f(
                        i_mouse,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                    );

                    set_uniform_1i(
                        i_frame,
                        frame,
                    );

                    gl::BindVertexArray(
                        vao
                    );

                    gl::DrawArrays(
                        gl::TRIANGLES,
                        0,
                        3,
                    );

                    gl::ColorMask(
                        gl::TRUE,
                        gl::TRUE,
                        gl::TRUE,
                        gl::TRUE,
                    );
                }


                postprocess.present_scene();

                // Diagnostic only: compose after post-processing, before EGL windowshader_state.presentation.
                let status_text = crate::renderer_status::enabled()
                    .then(crate::renderer_status::panel_text);
                renderer_status_panels
                    .entry(native_target.info.registry_name)
                    .or_insert_with(crate::display_overlay::RendererStatusPanel::new)
                    .draw(target_width, target_height, status_text.as_deref());

                if runtime.display_format
                    == crate::manage_configuration::WallpaperDisplayFormat::Windowed
                {
                    let registry_name =
                        native_target.info.registry_name;

                    let lyrics_state =
                        runtime.lyrics_state
                            .as_ref()
                            .and_then(
                                |shared| {
                                    shared
                                        .lock()
                                        .ok()
                                        .map(
                                            |state| state.clone()
                                        )
                                }
                            )
                            .unwrap_or_default();

                    if lyrics_state.current.is_none() {
                        lyrics_overlays.remove(
                            &registry_name
                        );
                        lyrics_overlay_states.remove(
                            &registry_name
                        );
                    } else {
                        let needs_rebuild =
                            lyrics_overlay_states
                                .get(
                                    &registry_name
                                )
                                .is_none_or(
                                    |(state, width, height)| {
                                        state != &lyrics_state
                                            || *width != target_width
                                            || *height != target_height
                                    }
                                );

                        if needs_rebuild {
                            match crate::construct_text_overlay::construct_lyrics_panel(
                                lyrics_state.previous.as_deref(),
                                lyrics_state.current.as_deref(),
                                lyrics_state.next.as_deref(),
                                target_width,
                                target_height,
                            )
                            .and_then(
                                |constructed| {
                                    crate::display_overlay::OpenGlOverlay::new_from_constructed(
                                        constructed,
                                        crate::parse_subtitle_placement::parse(None).placement,
                                    )
                                }
                            ) {
                                Ok(overlay) => {
                                    lyrics_overlays.insert(
                                        registry_name,
                                        overlay,
                                    );
                                    lyrics_overlay_states.insert(
                                        registry_name,
                                        (
                                            lyrics_state,
                                            target_width,
                                            target_height,
                                        ),
                                    );
                                }

                                Err(error) => {
                                    crate::logger::warning(
                                        &crate::locate_paths::runtime_log_path(),
                                        &format!(
                                            "[LYRICS] Unable to build Wayland Windowpaper lyrics overlay: {}",
                                            error,
                                        ),
                                    );
                                    lyrics_overlays.remove(
                                        &registry_name
                                    );
                                    lyrics_overlay_states.remove(
                                        &registry_name
                                    );
                                }
                            }
                        }

                        if let Some(overlay) =
                            lyrics_overlays.get(
                                &registry_name
                            )
                        {
                            overlay.display_bottom_edge(
                                target_width,
                                target_height,
                            );
                        }
                    }
                }


                state.request_next_presentation_frame(
                    &native_target.surface,
                    native_target.info.registry_name,
                    &frame_queue_handle,
                );


                // Finish measuring Screenshaver's GPU work before handing
                // the completed frame to the Wayland compositor.  An
                // xdg_toplevel on an inactive workspace may block for seconds
                // in eglSwapBuffers(); that presentation wait is not shader
                // rendering cost.
                unsafe {
                    gl::QueryCounter(
                        performance_queries[1],
                        gl::TIMESTAMP,
                    );
                }


                control.diagnostic_checkpoint(5);
                let presentation_started =
                    Instant::now();


                if unsafe {
                    eglSwapBuffers(
                        display,
                        egl_target.surface,
                    )
                } == EGL_FALSE
                {
                    return Err(
                        egl_failure(
                            "eglSwapBuffers"
                        )
                    );
                }


                control.diagnostic_checkpoint(6);
                state.on_frame_swap_completed(native_target.info.registry_name);
                windowshader_state.presentation.last_successful_swap = Instant::now();
                if status_enabled && (paused || windowshader_state.resume_presentation_pending
                    || status_heartbeat.elapsed() >= Duration::from_secs(1))
                {
                    crate::renderer_status::publish(status_pipeline, crate::renderer_status::State::Rendering);
                    status_heartbeat = Instant::now();
                }

                if runtime.display_format
                    == crate::manage_configuration::WallpaperDisplayFormat::Windowed
                {
                    windowshader_state.presentation.presentation_watchdog_swaps = windowshader_state.presentation.presentation_watchdog_swaps.wrapping_add(1);
                    windowshader_state.presentation.presentation_watchdog_last_report = None;
                }
                let presentation_duration =
                    presentation_started.elapsed();

                // Diagnostic only: report a compositor wait that overlapped a
                // pause request. Do not change frame pacing or handoff behavior.
                if control.pause_requested() {
                    crate::logger::warning(
                        &crate::locate_paths::runtime_log_path(),
                        &format!(
                            "[AMBIENT_HANDOFF_DIAG] eglSwapBuffers returned after {}ms while pause pending",
                            presentation_duration.as_millis(),
                        ),
                    );
                }


                // A long compositor wait breaks continuous presentation.
                // Begin a new rolling performance epoch instead of carrying
                // samples from before the hidden/inactive interval forward.
                if presentation_duration
                    >= Duration::from_millis(250)
                {
                    frame_times.clear();

                    fps_warning_state =
                        crate::fps_monitor::FpsWarningState::Normal;
                }
            }


            // Windowshader owns its post-pause presentation acknowledgment.
            // Wallpaper retains its existing local paused-state transition.
            let presentation_resumed = if windowed_rotation {
                windowshader_state.on_successful_presentation(paused)
            } else {
                paused
            };
            if presentation_resumed {
                paused = false;

                // Acknowledge only after a successful EGL presentation. This
                // also covers pauses observed by the independent lighting
                // coordinator while EGL was blocked.
                control.acknowledge_resumed_frame();
                state.on_resumed_frame_presented();


                println!(
                    "Wayland wallpaper rendering resumed."
                );
            }


            unsafe {
                // Preserve the existing synchronization behavior.  The end
                // timestamp was issued before eglSwapBuffers(), so any
                // compositor presentation stall is excluded from the GPU
                // performance sample.
                gl::Finish();
            }


            let mut gpu_start_ns =
                0_u64;

            let mut gpu_end_ns =
                0_u64;

            unsafe {
                gl::GetQueryObjectui64v(
                    performance_queries[0],
                    gl::QUERY_RESULT,
                    &mut gpu_start_ns,
                );

                gl::GetQueryObjectui64v(
                    performance_queries[1],
                    gl::QUERY_RESULT,
                    &mut gpu_end_ns,
                );
            }


            let gpu_render_duration =
                Duration::from_nanos(
                    gpu_end_ns.saturating_sub(
                        gpu_start_ns
                    )
                );


            let performance_status =
                frame_times.record(
                    gpu_render_duration,
                    rendered_fps,
                );


            let new_warning_state =
                performance_status.warning_state;


            if new_warning_state
                != fps_warning_state
            {
                fps_warning_state =
                    new_warning_state;


                println!(
                    "Wallpaper performance changed:"
                );


                println!(
                    "    Target FPS: {}",
                    rendered_fps
                );


                println!(
                    "    Average FPS: {}",
                    performance_status.average_fps
                );


                println!(
                    "    State: {:?}",
                    fps_warning_state
                );


                println!();


                if fps_warning_state
                    != crate::fps_monitor::FpsWarningState::Normal
                {
                    notify_active_wallpaper(
                        &mut notification_state,
                        false,
                        runtime.notifications,
                        &current_shader,
                        &texture_manager,
                        rendered_fps,
                        animation_speed,
                        fps_warning_state,
                    );
                }
            }


            frame =
                frame.saturating_add(
                    1
                );


            control.diagnostic_checkpoint(7);
            // Dispatch pacing to the owning rendering pipeline.
            if windowed_rotation {
                windowshader_state.pace_presented_frame(frame_duration);
            } else {
                S::pace_frame(&mut next_frame_deadline, frame_duration);
            }
            Ok(FrameLoopAction::Continue)
        });


    drop(
        postprocess_pipelines
    );


    texture_manager.delete_all();


    unsafe {
        gl::UseProgram(
            0
        );

        gl::BindVertexArray(
            0
        );

        gl::DeleteVertexArrays(
            1,
            &vao,
        );

        gl::DeleteQueries(
            2,
            performance_queries.as_ptr(),
        );

        gl::DeleteProgram(
            program
        );
    }


    if status_enabled {
        crate::renderer_status::publish(status_pipeline, crate::renderer_status::State::Stopping);
        crate::renderer_status::clear(status_pipeline);
    }
    result
}


pub(crate) fn frame_duration_for_fps(
    fps: u32,
) -> Duration {

    Duration::from_secs_f64(
        1.0 / fps.max(1) as f64
    )
}


fn notify_active_wallpaper(
    notification_state:
        &mut crate::notify_wallpaper::WallpaperNotificationState,
    shader_changed: bool,
    enabled: bool,
    shader: &ActiveWallpaperShader,
    texture_manager: &crate::manage_textures::TextureManager,
    fps: u32,
    animation_speed: f32,
    warning_state: crate::fps_monitor::FpsWarningState,
) {

    let selection =
        texture_manager.active_specification_selection();


    let metadata =
        crate::notify_wallpaper::WallpaperMetadata {
            policy_name:
                shader.policy_name.clone(),

            animation_speed,

            texture:
                selection.map(
                    |(specification, _)| {
                        specification.display_name()
                    }
                ),

            palette:
                selection.map(
                    |(_, palette)| {
                        palette.name()
                            .to_string()
                    }
                ),

            fps:
                fps.max(1),

            warning_state,
        };


    if shader_changed {
        notification_state
            .show_shader_changed(
                enabled,
                &metadata,
            );
    } else {
        notification_state
            .show_update(
                enabled,
                &metadata,
            );
    }
}


fn remove_disconnected_targets(
    display: EglDisplay,
    context: EglContext,
    state: &mut WaylandState,
    egl_targets: &mut Vec<EglWallpaperTarget>,
    native_targets: &mut Vec<NativeWallpaperTarget>,
    postprocess_pipelines:
        &mut HashMap<
            u32,
            crate::postprocess_shader::PostprocessPipeline,
        >,
) -> Result<(), String> {

    remove_disconnected_targets_from_names(
        display,
        context,
        &mut state.removed_output_names,
        egl_targets,
        native_targets,
        postprocess_pipelines,
    )
}

// Keep EGL cleanup independent of the Wayland event-dispatch state type.
// A Windowshader-owned queue can supply its own removed-output list.
fn remove_disconnected_targets_from_names(
    display: EglDisplay,
    context: EglContext,
    removed_output_names: &mut Vec<u32>,
    egl_targets: &mut Vec<EglWallpaperTarget>,
    native_targets: &mut Vec<NativeWallpaperTarget>,
    postprocess_pipelines: &mut HashMap<u32, crate::postprocess_shader::PostprocessPipeline>,
) -> Result<(), String> {
    let removed_names = std::mem::take(removed_output_names);

    for registry_name in removed_names {

        let Some(
            egl_index
        ) =
            egl_targets
                .iter()
                .position(
                    |target| {
                        target.registry_name
                            == registry_name
                    }
                )
        else {
            continue;
        };


        let Some(
            native_index
        ) =
            native_targets
                .iter()
                .position(
                    |target| {
                        target.info.registry_name
                            == registry_name
                    }
                )
        else {
            return Err(
                format!(
                    "Wallpaper output {} disappeared, but its native target was not found",
                    registry_name,
                )
            );
        };


        let removed_surface =
            egl_targets[
                egl_index
            ]
            .surface;


        if unsafe {
            eglMakeCurrent(
                display,
                removed_surface,
                removed_surface,
                context,
            )
        } == EGL_FALSE
        {
            return Err(
                egl_failure(
                    "eglMakeCurrent before removing wallpaper post-processing resources"
                )
            );
        }


        postprocess_pipelines.remove(
            &registry_name
        );


        if unsafe {
            eglMakeCurrent(
                display,
                EGL_NO_SURFACE,
                EGL_NO_SURFACE,
                EGL_NO_CONTEXT,
            )
        } == EGL_FALSE
        {
            return Err(
                egl_failure(
                    "eglMakeCurrent while removing a wallpaper target"
                )
            );
        }


        let egl_target =
            egl_targets.remove(
                egl_index
            );


        if unsafe {
            eglDestroySurface(
                display,
                egl_target.surface,
            )
        } == EGL_FALSE
        {
            return Err(
                egl_failure(
                    "eglDestroySurface while removing a wallpaper target"
                )
            );
        }


        let native_target =
            native_targets.remove(
                native_index
            );


        let connector =
            native_target
                .info
                .connector_name
                .clone()
                .unwrap_or_else(
                    || {
                        "<unknown>"
                            .to_string()
                    }
                );


        let NativeWallpaperTarget {
            surface,
            layer_surface,
            input_region,
            xdg_surface,
            xdg_toplevel,
            toplevel_decoration,
            egl_window,
            ..
        } = native_target;


        drop(
            egl_window
        );


        if let Some(layer_surface) =
            layer_surface
        {
            layer_surface.destroy();
        }


        if let Some(decoration) = toplevel_decoration {
            decoration.destroy();
        }

        if let Some(xdg_toplevel) =
            xdg_toplevel
        {
            xdg_toplevel.destroy();
        }


        if let Some(xdg_surface) =
            xdg_surface
        {
            xdg_surface.destroy();
        }


        surface.destroy();


        if let Some(input_region) =
            input_region
        {
            input_region.destroy();
        }


        println!(
            "Wallpaper target removed:"
        );


        println!(
            "    Registry name: {}",
            registry_name
        );


        println!(
            "    Connector: {}",
            connector
        );


        println!(
            "    Remaining targets: {}",
            native_targets.len()
        );


        println!();


        if let Some(
            next_target
        ) =
            egl_targets.first()
        {
            if unsafe {
                eglMakeCurrent(
                    display,
                    next_target.surface,
                    next_target.surface,
                    context,
                )
            } == EGL_FALSE
            {
                return Err(
                    egl_failure(
                        "eglMakeCurrent after removing a wallpaper target"
                    )
                );
            }
        }
    }


    Ok(())
}


fn apply_pending_target_resizes(
    state: &mut WaylandState,
    native_targets: &mut [NativeWallpaperTarget],
) -> Result<(), String> {
    apply_pending_target_resizes_from_states(&mut state.surface_states, native_targets)
}

// Resizing is independent of the dispatch-state type; only surface records
// and native EGL targets are required.
fn apply_pending_target_resizes_from_states(
    surface_states: &mut [WallpaperSurfaceState],
    native_targets: &mut [NativeWallpaperTarget],
) -> Result<(), String> {

    for surface_state in surface_states.iter_mut() {

        let Some(
            configuration
        ) =
            surface_state.configured.take()
        else {
            continue;
        };


        let Some(
            target
        ) =
            native_targets
                .iter_mut()
                .find(
                    |target| {
                        target.info.registry_name
                            == surface_state.registry_name
                    }
                )
        else {
            continue;
        };


        let new_width =
            if configuration.width
                == 0
            {
                target.width
            } else {
                i32::try_from(
                    configuration.width
                )
                .map_err(
                    |_| {
                        "Wallpaper resize width exceeds the EGL window range"
                            .to_string()
                    }
                )?
            };


        let new_height =
            if configuration.height
                == 0
            {
                target.height
            } else {
                i32::try_from(
                    configuration.height
                )
                .map_err(
                    |_| {
                        "Wallpaper resize height exceeds the EGL window range"
                            .to_string()
                    }
                )?
            };


        if new_width
            == target.width
            && new_height
                == target.height
        {
            continue;
        }


        target
            .egl_window
            .resize(
                new_width,
                new_height,
                0,
                0,
            );


        target.width =
            new_width;


        target.height =
            new_height;


        println!(
            "Wayland wallpaper target resized:"
        );


        println!(
            "    Connector: {}",
            target
                .info
                .connector_name
                .as_deref()
                .unwrap_or(
                    "<not advertised>"
                )
        );


        println!(
            "    Width: {}",
            new_width
        );


        println!(
            "    Height: {}",
            new_height
        );


        println!(
            "    Configure serial: {}",
            configuration.serial
        );
    }


    Ok(
        ()
    )
}

