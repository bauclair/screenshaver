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


// Pipeline-neutral callback result. The owning controller decides whether a
// compositor-authorized frame may proceed; the shared executor only applies
// clock/FPS effects. This type does not grant frame readiness.
struct FramePresentationDecision {
    recovery: Option<FrameClockRecovery>,
}

struct FrameClockRecovery {
    rotation_shift: Duration,
    animation_shift: Duration,
}

// Wallpaper's compositor callback accounting is independent of Windowshader.
// This state never grants frame readiness: the dispatch implementation must
// first observe the real wl_surface.frame Done event.
struct WallpaperCallbackController {
    inactive_since: Option<Instant>,
    warned: bool,
    last_report: Option<Instant>,
}

impl WallpaperCallbackController {
    fn new() -> Self {
        Self { inactive_since: None, warned: false, last_report: None }
    }

    // Returns None when the compositor has not authorized another frame.
    // Recovery shifts are returned to the Wallpaper clock owner; this
    // controller never changes Windowshader's rotation or presentation state.
    fn evaluate<S: RenderWaylandDispatch>(
        &mut self,
        state: &mut S,
        targets: &[NativeWallpaperTarget],
        shader_anchor: Instant,
        animation_anchor: Instant,
    ) -> Option<Option<(Duration, Duration)>> {
        if !state.frames_ready_for_presentation(targets) {
            let started = *self.inactive_since.get_or_insert_with(Instant::now);
            let elapsed = started.elapsed();
            if !self.warned && elapsed >= Duration::from_millis(1500) {
                self.warned = true;
                let outstanding = state.outstanding_frame_callbacks(targets);
                crate::logger::warning(
                    &crate::locate_paths::runtime_log_path(),
                    &format!("[WALLPAPER_CALLBACK_DIAG] Frame callback outstanding >1500ms; targets={outstanding:?}"),
                );
            }
            if elapsed >= Duration::from_secs(10)
                && self.last_report.is_none_or(|at| at.elapsed() >= Duration::from_secs(10))
            {
                self.last_report = Some(Instant::now());
                let outstanding = state.outstanding_frame_callbacks(targets);
                state.log_outstanding_callback_details(&outstanding);
                crate::logger::warning(
                    &crate::locate_paths::runtime_log_path(),
                    &format!("[WALLPAPER_CALLBACK_DIAG] Callback stall persists: targets={outstanding:?}, wait_ms={}", elapsed.as_millis()),
                );
            }
            thread::sleep(Duration::from_millis(10));
            return None;
        }

        self.warned = false;
        self.last_report = None;
        let Some(inactive_since) = self.inactive_since.take() else {
            return Some(None);
        };
        let elapsed = inactive_since.elapsed();
        if elapsed < Duration::from_millis(250) {
            return Some(None);
        }
        // Wallpaper's historical callback recovery has no windowed pause
        // interval to exclude. Cap shifts to each clock's elapsed lifetime.
        let rotation_shift = elapsed.min(shader_anchor.elapsed());
        let animation_shift = elapsed.min(animation_anchor.elapsed());
        Some(Some((rotation_shift, animation_shift)))
    }
}

// Stage 4B-5A2: shared GL resources also own the active shader program.
// Texture, post-processing, and frame control remain unchanged. Shader program
// replacement is committed only after preparation of the next shader.
// Creation and explicit release must run with the owning EGL context current.
struct SharedFrameGlResources {
    vertex_array: u32,
    timing_queries: [u32; 2],
    program: u32,
}

impl SharedFrameGlResources {
    fn create(program: u32) -> Self {
        let mut resources = Self {
            vertex_array: 0,
            timing_queries: [0; 2],
            program,
        };
        unsafe {
            gl::GenVertexArrays(1, &mut resources.vertex_array);
            gl::BindVertexArray(resources.vertex_array);
        }
        resources
    }

    fn initialize_timing_queries(&mut self) {
        unsafe {
            gl::GenQueries(2, self.timing_queries.as_mut_ptr());
        }
    }

    // Commit only after the replacement program and its dependent resources
    // have been prepared. The previous program remains valid until this call.
    fn replace_program(&mut self, replacement: u32) {
        unsafe {
            gl::UseProgram(0);
            if self.program != 0 {
                gl::DeleteProgram(self.program);
            }
        }
        self.program = replacement;
    }

    // Stage 4B-5B: the GL execution layer owns GPU timestamp operations.
    // The pipeline controller determines when a frame begins and ends.
    fn begin_gpu_frame(&self) {
        unsafe { gl::QueryCounter(self.timing_queries[0], gl::TIMESTAMP); }
    }

    fn end_gpu_frame_before_presentation(&self) {
        unsafe { gl::QueryCounter(self.timing_queries[1], gl::TIMESTAMP); }
    }

    fn completed_gpu_frame_duration(&self) -> Duration {
        // The second timestamp is recorded before compositor presentation,
        // so a hidden workspace cannot inflate the shader's GPU measurement.
        unsafe { gl::Finish(); }
        let mut start_ns = 0_u64;
        let mut end_ns = 0_u64;
        unsafe {
            gl::GetQueryObjectui64v(self.timing_queries[0], gl::QUERY_RESULT, &mut start_ns);
            gl::GetQueryObjectui64v(self.timing_queries[1], gl::QUERY_RESULT, &mut end_ns);
        }
        Duration::from_nanos(end_ns.saturating_sub(start_ns))
    }

    fn release(&mut self) {
        unsafe {
            gl::BindVertexArray(0);
            if self.vertex_array != 0 {
                gl::DeleteVertexArrays(1, &self.vertex_array);
                self.vertex_array = 0;
            }
            if self.program != 0 {
                gl::UseProgram(0);
                gl::DeleteProgram(self.program);
                self.program = 0;
            }
            if self.timing_queries.iter().any(|query| *query != 0) {
                gl::DeleteQueries(2, self.timing_queries.as_ptr());
                self.timing_queries = [0; 2];
            }
        }
    }
}


// Stage 4B-5A consolidated: texture and post-processing resources share a
// persistent owner. All explicit GL releases occur before EGL teardown.
struct SharedFramePipelineResources {
    textures: crate::manage_textures::TextureManager,
    postprocess: HashMap<u32, crate::postprocess_shader::PostprocessPipeline>,
}

impl SharedFramePipelineResources {
    fn new(textures: crate::manage_textures::TextureManager, target_count: usize) -> Self {
        Self { textures, postprocess: HashMap::with_capacity(target_count) }
    }

    // Prepare replacements before calling this method. On failure the current
    // texture manager remains intact and the caller releases the candidate.
    fn commit_textures(&mut self, replacement: crate::manage_textures::TextureManager) {
        self.textures.delete_all();
        self.textures = replacement;
    }

    fn release(&mut self) {
        self.postprocess.clear();
        self.textures.delete_all();
    }
}

// Stage 4B-5C: persistent EGL-frame resource context.  This owns only
// pipeline-neutral GL objects.  Callback eligibility, rotation scheduling,
// pause/resume and dispatch remain with their respective controllers.
// Explicit release is required while the EGL context is current.
struct SharedFrameContext {
    gl: SharedFrameGlResources,
    pipeline: SharedFramePipelineResources,
}

impl SharedFrameContext {
    fn new(gl: SharedFrameGlResources, pipeline: SharedFramePipelineResources) -> Self {
        Self { gl, pipeline }
    }

    // Pipeline-neutral frame execution boundaries.  The owning pipeline
    // remains responsible for callback eligibility, diagnostics, pause state,
    // accounting and pacing.  In particular, this never requests or fabricates
    // a Wayland frame callback.
    fn begin_frame_gpu_measurement(&self) {
        self.gl.begin_gpu_frame();
    }

    fn finish_frame_gpu_measurement(&self) {
        self.gl.end_gpu_frame_before_presentation();
    }

    fn present_frame(
        &self,
        display: EglDisplay,
        surface: *mut std::ffi::c_void,
    ) -> Result<Duration, String> {
        present_egl_frame_target(display, surface)
    }

    fn completed_frame_gpu_duration(&self) -> Duration {
        self.gl.completed_gpu_frame_duration()
    }

    // Resolve all standard uniforms for a candidate program before replacing
    // the active program. Preserve lookup order and propagate failures.
    fn shader_uniform_locations(program: u32) -> Result<(i32, i32, i32, i32), String> {
        Ok((
            uniform_location(program, "iTime")?,
            uniform_location(program, "iResolution")?,
            uniform_location(program, "iMouse")?,
            uniform_location(program, "iFrame")?,
        ))
    }

    // Shader-lifecycle boundary: prepare a replacement texture set without
    // touching the currently active program or texture resources.  The caller
    // retains ownership of the candidate GL program until commit succeeds.
    fn prepare_replacement_textures(
        &self,
        shader: &ActiveWallpaperShader,
        runtime: &crate::define_wallpaper::WallpaperRuntime,
        candidate_program: u32,
    ) -> Result<crate::manage_textures::TextureManager, String> {
        let mut candidate = crate::manage_textures::TextureManager::new(
            runtime.texture_policy.clone(),
        );
        if let Err(error) = candidate.prepare_for_policy_with_path(
            shader.policy_id,
            &shader.shader_name,
            Some(shader.source_path.as_path()),
            shader.channel_usage,
        ) {
            candidate.delete_all();
            return Err(error);
        }
        candidate.configure_program(candidate_program);
        Ok(candidate)
    }

    // The candidate program and textures are already prepared.  Preserve the
    // established commit ordering: retire old textures, then old program.
    fn commit_shader_replacement(
        &mut self,
        candidate_program: u32,
        candidate_textures: crate::manage_textures::TextureManager,
    ) {
        self.pipeline.commit_textures(candidate_textures);
        self.gl.replace_program(candidate_program);
    }

    fn release(&mut self) {
        // Post-processing targets and textures must be destroyed before the
        // shader program and timing objects, while EGL is still current.
        self.pipeline.release();
        unsafe { gl::UseProgram(0); }
        self.gl.release();
    }
}

// Persistent frame-control clock state. Keep animation and rotation clocks
// together so pause/resume and shader replacement cannot accidentally update
// one clock while leaving the other in an unrelated local variable.
// This is shared state storage, not presentation authorization.
struct SharedFrameClockState {
    animation_started: Instant,
    shader_switched: Instant,
    paused: bool,
    pause_started: Option<Instant>,
}

impl SharedFrameClockState {
    fn new() -> Self {
        Self {
            animation_started: Instant::now(),
            shader_switched: Instant::now(),
            paused: false,
            pause_started: None,
        }
    }
}

// Frame numbering and GPU-derived FPS warning state remain together across
// shader switches, pauses, and successful presentations. This structure does
// not determine compositor eligibility or substitute for frame callbacks.
struct SharedFrameAccountingState {
    frame: i32,
    frame_times: crate::fps_monitor::FrameTimeWindow,
    fps_warning_state: crate::fps_monitor::FpsWarningState,
}

impl SharedFrameAccountingState {
    fn new() -> Self {
        Self {
            frame: 0,
            frame_times: crate::fps_monitor::FrameTimeWindow::new(),
            fps_warning_state: crate::fps_monitor::FpsWarningState::Normal,
        }
    }
}

// Stage 4B-5D: persistent active-policy/shader selection state. The shared
// renderer still performs the GL replacement; this object records the accepted
// shader only after the replacement has succeeded. Wallpaper and Windowshader
// retain their existing selection and scheduling behavior.
struct SharedActiveShaderState {
    current: ActiveWallpaperShader,
}

impl SharedActiveShaderState {
    fn new(initial: ActiveWallpaperShader) -> Self {
        Self { current: initial }
    }

    fn accept_replacement(&mut self, shader: ActiveWallpaperShader) {
        self.current = shader;
    }
}

// Stage 4B-5D: runtime controls shared by the current frame executor.
// These are policy-derived values and status cadence, not compositor
// presentation authorization. The independent Windowshader loop will
// eventually own the control sequence that updates them.
struct SharedFrameRuntimeControls {
    animation_speed: f32,
    audio_required: bool,
    status_heartbeat: Instant,
}

impl SharedFrameRuntimeControls {
    fn new(animation_speed: f32, audio_required: bool) -> Self {
        Self {
            animation_speed,
            audio_required,
            status_heartbeat: Instant::now() - Duration::from_secs(2),
        }
    }
}

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

    let initial_program =
        crate::compile_shader::build_program(
            crate::define_constants::VERTEX_SHADER,
            &active_shader.source,
        )?;


    let mut frame_gl = SharedFrameGlResources::create(initial_program);

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
        frame_gl.release();


        return Err(
            format!(
                "Unable to prepare textures for wallpaper shader '{}': {}",
                active_shader.shader_name,
                error,
            )
        );
    }


    texture_manager.configure_program(
        frame_gl.program
    );


    let mut frame_context = SharedFrameContext::new(
        frame_gl,
        SharedFramePipelineResources::new(texture_manager, native_targets.len()),
    );

    let initial_animation_speed =
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


    let initial_postprocess_profile =
        runtime.postprocess_policy
            .profile_for_policy(
                active_shader.policy_id,
                &active_shader.shader_name,
                Some(
                    active_shader.source_path.as_path()
                ),
            );

    let initial_audio_required =
        matches!(
            initial_postprocess_profile.bloom.name(),
            "audio" | "spectral" | "loudness"
        ) || initial_postprocess_profile.audio_motion.is_enabled();

    let mut runtime_controls = SharedFrameRuntimeControls::new(
        initial_animation_speed,
        initial_audio_required,
    );

    let mut notification_state =
        crate::notify_wallpaper::WallpaperNotificationState::new();


    notify_active_wallpaper(
        &mut notification_state,
        true,
        runtime.notifications,
        active_shader,
        &frame_context.pipeline.textures,
        rendered_fps,
        runtime_controls.animation_speed,
        crate::fps_monitor::FpsWarningState::Normal,
    );


    unsafe {
        gl::UseProgram(frame_context.gl.program);
    }

    crate::apply_shader_inputs::apply(
        frame_context.gl.program,
        &active_shader.shader_inputs,
    );


    // FPS Warning/CRITICAL must measure GPU rendering work rather than CPU
    // wall-clock time.  This is especially important for Windowshader: a
    // compositor may throttle or defer a non-visible xdg_toplevel while its
    // workspace is inactive.  CPU elapsed time across that lifecycle event is
    // not evidence that the shader itself is slow.
    frame_context.gl.initialize_timing_queries();

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


        frame_context.pipeline.postprocess.insert(
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



    let audio_demand = crate::audio_backend::register_audio_demand();



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


    let (mut i_time, mut i_resolution, mut i_mouse, mut i_frame) =
        SharedFrameContext::shader_uniform_locations(frame_context.gl.program)?;


    let mut frame_clock = SharedFrameClockState::new();

    let mut frame_accounting = SharedFrameAccountingState::new();


    let mut active_shader_state =
        SharedActiveShaderState::new(active_shader.clone());


    // The initial wallpaper has now loaded, compiled, prepared textures,
    // created post-processing pipelines, and resolved its uniforms. Persist
    // Ordered position only after reaching this accepted render state.
    shader_manager.record_rendered_policy(
        active_shader_state.current.policy_id
    );


    // Track the most recent pause interval so an outstanding compositor
    // callback cannot charge the same inactive time to both clocks twice.

    // Diagnostics only: preserve compositor-driven pacing. In particular,
    // never synthesize a wl_surface.frame completion after a screensaver pause.
    // Windowshader owns its callback, rotation and presentation bookkeeping.
    // Keep it grouped while the shared GL frame executor is extracted in stages.
    let windowed_rotation = runtime.display_format
        == crate::manage_configuration::WallpaperDisplayFormat::Windowed;
    // A Wallpaper run must never construct or advance Windowshader control state.
    // This is a temporary boundary until the Windowshader loop is fully moved
    // into wayland_windowshader.rs; the shared renderer still executes frames.
    let mut windowshader_state = if windowed_rotation {
        Some(crate::wayland_windowshader::initialize_windowshader_frame_controller(
            state,
            native_targets,
            frame_clock.shader_switched,
            next_frame_deadline,
        ))
    } else {
        None
    };
    let mut wallpaper_callbacks = WallpaperCallbackController::new();


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
    

    let result = loop_driver(&mut || -> Result<FrameLoopAction, String> {

            // The Windowshader pipeline owns the ordered pre-frame control
            // sequence. The shared core supplies only target reconciliation.
            if windowed_rotation {
                let should_continue = crate::wayland_windowshader::run_windowshader_frame_preflight(
                    windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode"),
                    event_queue, state, native_targets, frame_clock.paused,
                    control, shader_interval, &active_shader_state.current.shader_name,
                    active_shader_state.current.policy_id, frame_context.gl.program, frame_accounting.frame,
                    running,
                    |state, native_targets| reconcile_shared_frame_targets(
                        display, context, state, egl_targets, native_targets,
                        &mut frame_context.pipeline.postprocess,
                    ),
                )?;
                if !should_continue {
                    return Ok(FrameLoopAction::Stop);
                }
            } else {
                control.diagnostic_checkpoint(1);
                S::pump_events(event_queue, state)?;
                control.diagnostic_checkpoint(2);
                if !reconcile_shared_frame_targets(
                    display, context, state, egl_targets, native_targets,
                    &mut frame_context.pipeline.postprocess,
                )? {
                    println!("All wallpaper outputs have been disconnected.");
                    return Ok(FrameLoopAction::Stop);
                }
                let compositor_closed = state.surfaces().iter().any(|surface| surface.closed);
                if compositor_closed {
                    if S::compositor_close_requests_shutdown() {
                        running.store(false, Ordering::SeqCst);
                    }
                    return Ok(FrameLoopAction::Stop);
                }
                if !running.load(Ordering::SeqCst) {
                    return Ok(FrameLoopAction::Stop);
                }
            }


            if let Some(reload) =
                control.take_policy_reload()
            {
                let mut replacement_texture_manager =
                    crate::manage_textures::TextureManager::new(
                        reload.texture_policy.clone()
                    );

                match replacement_texture_manager.prepare_for_policy_with_path(
                    active_shader_state.current.policy_id,
                    &active_shader_state.current.shader_name,
                    Some(
                        active_shader_state.current.source_path.as_path()
                    ),
                    active_shader_state.current.channel_usage,
                ) {
                    Ok(()) => {
                        let replacement_profile =
                            reload.postprocess_policy
                                .profile_for_policy(
                                    active_shader_state.current.policy_id,
                                    &active_shader_state.current.shader_name,
                                    Some(
                                        active_shader_state.current.source_path.as_path()
                                    ),
                                );

                        let mut profile_error =
                            None;

                        for pipeline in
                            frame_context.pipeline.postprocess.values_mut()
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
                                frame_context.gl.program
                            );

                            frame_context.pipeline.commit_textures(replacement_texture_manager);

                            runtime_controls.audio_required =
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
                                    frame_context.pipeline.postprocess.values_mut()
                                {
                                    pipeline.set_audio_motion_scale(
                                        1.0
                                    );
                                }
                            }

                            last_audio_motion_update =
                                Instant::now();

                            runtime_controls.animation_speed =
                                reload.animation_speed_policy
                                    .animation_speed_for_policy(
                                        active_shader_state.current.policy_id,
                                        &active_shader_state.current.shader_name,
                                        Some(
                                            active_shader_state.current.source_path.as_path()
                                        ),
                                        None,
                                    );

                            rendered_fps =
                                reload.fps_policy
                                    .rendered_fps_for_policy(
                                        active_shader_state.current.policy_id,
                                        &active_shader_state.current.shader_name,
                                        Some(
                                            active_shader_state.current.source_path.as_path()
                                        ),
                                        None,
                                    );

                            frame_duration =
                                frame_duration_for_fps(
                                    rendered_fps
                                );

                            if windowed_rotation {
                                windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode").on_schedule_discontinuity(
                                    crate::wayland_windowshader::WindowshaderScheduleReset::PolicyOrShaderChange,
                                );
                            } else {
                                S::reset_frame_deadline(&mut next_frame_deadline);
                            }

                            frame_accounting.frame_times.clear();
                            frame_accounting.fps_warning_state =
                                crate::fps_monitor::FpsWarningState::Normal;

                            notify_active_wallpaper(
                                &mut notification_state,
                                false,
                                runtime.notifications,
                                &active_shader_state.current,
                                &frame_context.pipeline.textures,
                                rendered_fps,
                                runtime_controls.animation_speed,
                                frame_accounting.fps_warning_state,
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
                if !frame_clock.paused {
                    if windowed_rotation {
                        windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode")
                            .enter_pause_with_dispatch(state, native_targets, control, status_enabled);
                    } else {
                        crate::logger::warning(
                            &crate::locate_paths::runtime_log_path(),
                            "[AMBIENT_HANDOFF_DIAG] Wayland renderer detected pause request",
                        );
                        control.acknowledge_pause_detected();
                        state.on_pause_entering();
                        control.diagnostic_checkpoint(8);
                        state.on_pause_entered(native_targets);
                        frame_clock.pause_started = Some(Instant::now());
                        control.acknowledge_paused();
                        if status_enabled {
                            crate::renderer_status::publish(status_pipeline, crate::renderer_status::State::Paused);
                        }
                        println!("Wayland wallpaper rendering paused.");
                    }
                    frame_clock.paused = true;
                }


                thread::sleep(
                    Duration::from_millis(10)
                );


                return Ok(FrameLoopAction::Continue);
            }


            if frame_clock.paused {
                let pause_ended_at = Instant::now();
                // Windowshader owns its pause-release decision and rotation
                // adjustment; Render Core applies only the animation shift.
                let windowshader_release = if windowed_rotation {
                    Some(windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode")
                        .finish_pause_with_dispatch(state, pause_ended_at))
                } else {
                    None
                };
                if let Some(release) = windowshader_release {
                    if let Some(duration) = release.animation_shift {
                        frame_clock.animation_started += duration;
                    }
                } else if let Some(paused_at) = frame_clock.pause_started.take() {
                    let duration = pause_ended_at.duration_since(paused_at);
                    frame_clock.animation_started += duration;
                    frame_clock.shader_switched += duration;
                }


                if !windowed_rotation {
                    S::reset_frame_deadline(&mut next_frame_deadline);
                }

                frame_accounting.frame_times.clear();
                // The pause has ended even if the compositor has not yet
                // allowed another windowshader presentation. Keep the separate pending
                // flag until a successful EGL swap acknowledges resumption.
                frame_clock.paused = false;

            }


            // Wallpaper retains its historical checkpoint location; Windowshader
            // owns its checkpoint and callback-authorization sequence.
            if !windowed_rotation {
                control.diagnostic_checkpoint(4);
            }
            // Do not use xdg_toplevel::State::Activated as a visibility
            // signal.  Activated represents compositor activation/focus, so
            // Mango legitimately removes it when the pointer/keyboard focus
            // leaves a still-visible Windowshader.  Instead, pace Windowed
            // presentation with wl_surface.frame callbacks.  A visible,
            // unfocused Windowshader continues receiving callbacks, while a
            // surface on a non-visible workspace can stop receiving them.
            // The render worker therefore remains in this event/control loop
            // instead of entering a compositor-throttled eglSwapBuffers().
            // Windowshader now owns the entire callback gate/recovery decision.
            // Keep Wallpaper's historical path intact until its own extraction.
            let (presentation, rotation_requested) = if windowed_rotation {
                let Some((recovery, rotate)) = windowshader_state.as_mut()
                    .expect("Windowshader controller exists only in Windowed mode")
                    .authorize_windowshader_frame(
                        control, state, native_targets, shader_interval, frame_clock.animation_started, frame_clock.paused,
                        &active_shader_state.current.shader_name, active_shader_state.current.policy_id,
                        frame_context.gl.program,
                    ) else {
                    return Ok(FrameLoopAction::Continue);
                };
                (FramePresentationDecision {
                    recovery: recovery.map(|shift| FrameClockRecovery {
                        rotation_shift: shift.rotation_shift,
                        animation_shift: shift.animation_shift,
                    }),
                }, rotate)
            } else {
                let Some(recovery) = wallpaper_callbacks.evaluate(
                    state, native_targets, frame_clock.shader_switched, frame_clock.animation_started,
                ) else {
                    return Ok(FrameLoopAction::Continue);
                };
                (FramePresentationDecision {
                    recovery: recovery.map(|(rotation_shift, animation_shift)| FrameClockRecovery {
                        rotation_shift,
                        animation_shift,
                    }),
                }, !frame_clock.paused && shader_interval.is_some_and(|interval| frame_clock.shader_switched.elapsed() >= interval))
            };

            // Render Core applies only shared animation/FPS accounting.
            // Windowshader has already reset its own deadline on recovery.
            if let Some(recovery) = presentation.recovery {
                frame_clock.animation_started += recovery.animation_shift;
                if !windowed_rotation {
                    frame_clock.shader_switched += recovery.rotation_shift;
                    S::reset_frame_deadline(&mut next_frame_deadline);
                }
                frame_accounting.frame_times.clear();
                frame_accounting.fps_warning_state = crate::fps_monitor::FpsWarningState::Normal;
            }

            if rotation_requested {
                if !windowed_rotation {
                    frame_clock.shader_switched = Instant::now();
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
                                let next_texture_manager = match frame_context
                                    .prepare_replacement_textures(
                                        &next_shader,
                                        runtime,
                                        next_program,
                                    )
                                {
                                    Ok(textures) => textures,
                                    Err(error) => {
                                        println!(
                                            "Wallpaper shader texture preparation failed; keeping the current shader:"
                                        );
                                        println!("    Shader: {}", next_shader.shader_name);
                                        println!("    Error: {}", error);
                                        println!();
                                        unsafe { gl::DeleteProgram(next_program); }
                                        shader_manager.remove_shader(&next_shader.manager_name);
                                        return Ok(FrameLoopAction::Continue);
                                    }
                                };

                                unsafe {
                                    gl::UseProgram(
                                        next_program
                                    );
                                }


                                crate::apply_shader_inputs::apply(
                                    next_program,
                                    &next_shader.shader_inputs,
                                );


                                let (next_i_time, next_i_resolution, next_i_mouse, next_i_frame) =
                                    SharedFrameContext::shader_uniform_locations(next_program)?;


                                frame_context.commit_shader_replacement(
                                    next_program,
                                    next_texture_manager,
                                );


                                i_time =
                                    next_i_time;


                                i_resolution =
                                    next_i_resolution;


                                i_mouse =
                                    next_i_mouse;


                                i_frame =
                                    next_i_frame;


                                frame_clock.animation_started =
                                    Instant::now();


                                frame_accounting.frame =
                                    0;


                                runtime_controls.animation_speed =
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
                                windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode").on_schedule_discontinuity(
                                    crate::wayland_windowshader::WindowshaderScheduleReset::PolicyOrShaderChange,
                                );
                            } else {
                                S::reset_frame_deadline(&mut next_frame_deadline);
                            }


                                active_shader_state.accept_replacement(next_shader.clone());
                                if windowed_rotation {
                                    windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode").rotation.rotation_succeeded(
                                        &active_shader_state.current.shader_name, active_shader_state.current.policy_id, frame_context.gl.program,
                                    );
                                }


                                let postprocess_profile =
                                    runtime.postprocess_policy
                                        .profile_for_policy(
                                            active_shader_state.current.policy_id,
                                            &active_shader_state.current.shader_name,
                                            Some(
                                                active_shader_state.current.source_path.as_path()
                                            ),
                                        );


                                runtime_controls.audio_required =
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
                                    frame_context.pipeline.postprocess.values_mut()
                                {
                                    pipeline.set_profile(
                                        postprocess_profile
                                    )?;
                                }


                                runtime.tray_status.set_active(
                                    active_shader_state.current.policy_id,
                                    active_shader_state.current.shader_name.clone(),
                                    active_shader_state.current.source_path.clone(),
                                );


                                shader_manager.record_rendered_policy(
                                    active_shader_state.current.policy_id
                                );


                                frame_accounting.frame_times.clear();


                                frame_accounting.fps_warning_state =
                                    crate::fps_monitor::FpsWarningState::Normal;


                                notify_active_wallpaper(
                                    &mut notification_state,
                                    true,
                                    runtime.notifications,
                                    &active_shader_state.current,
                                    &frame_context.pipeline.textures,
                                    rendered_fps,
                                    runtime_controls.animation_speed,
                                    frame_accounting.fps_warning_state,
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
                                    runtime_controls.animation_speed
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
                                if windowed_rotation {
                                    windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode").rotation.rotation_compilation_failed(
                                        &next_shader.shader_name, &error.to_string(),
                                    );
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
                        if windowed_rotation {
                            windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode").rotation.rotation_selection_failed(&error.to_string());
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
                frame_clock.animation_started.elapsed();


            frame_context.begin_frame_gpu_measurement();

            audio_demand.set_required(
                runtime_controls.audio_required
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
                bind_egl_frame_target(display, context, egl_target.surface)?;

                let postprocess =
                    frame_context.pipeline.postprocess
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


                execute_shader_scene_frame(
                    &frame_context.gl,
                    &frame_context.pipeline.textures,
                    postprocess,
                    ShaderSceneUniforms {
                        time: elapsed.as_secs_f32() * runtime_controls.animation_speed,
                        frame: frame_accounting.frame,
                        i_time,
                        i_resolution,
                        i_mouse,
                        i_frame,
                    },
                );

                // Diagnostic only: compose after post-processing, before EGL windowshader presentation.
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
                frame_context.finish_frame_gpu_measurement();

                control.diagnostic_checkpoint(5);
                let presentation_duration = frame_context.present_frame(display, egl_target.surface)?;

                control.diagnostic_checkpoint(6);
                state.on_frame_swap_completed(native_target.info.registry_name);
                // Pipeline-specific presentation accounting belongs to the
                // Windowshader controller; Wallpaper retains its own heartbeat.
                let publish_rendering = if windowed_rotation {
                    windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode").record_windowshader_swap_and_check_status(
                        frame_clock.paused, runtime_controls.status_heartbeat.elapsed(),
                    )
                } else {
                    frame_clock.paused || runtime_controls.status_heartbeat.elapsed() >= Duration::from_secs(1)
                };
                if status_enabled && publish_rendering
                {
                    crate::renderer_status::publish(status_pipeline, crate::renderer_status::State::Rendering);
                    runtime_controls.status_heartbeat = Instant::now();
                }

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
                    frame_accounting.frame_times.clear();

                    frame_accounting.fps_warning_state =
                        crate::fps_monitor::FpsWarningState::Normal;
                }
            }


            // Windowshader owns its post-pause presentation acknowledgment.
            // Wallpaper retains its existing local paused-state transition.
            let presentation_resumed = if windowed_rotation {
                windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode").complete_presented_frame_and_pace(frame_clock.paused, frame_duration)
            } else {
                frame_clock.paused
            };
            if presentation_resumed {
                frame_clock.paused = false;

                // Acknowledge only after a successful EGL presentation. This
                // also covers pauses observed by the independent lighting
                // coordinator while EGL was blocked.
                control.acknowledge_resumed_frame();
                state.on_resumed_frame_presented();


                println!(
                    "Wayland wallpaper rendering resumed."
                );
            }


            let gpu_render_duration = frame_context.completed_frame_gpu_duration();

            let performance_status =
                frame_accounting.frame_times.record(
                    gpu_render_duration,
                    rendered_fps,
                );


            let new_warning_state =
                performance_status.warning_state;


            if new_warning_state
                != frame_accounting.fps_warning_state
            {
                frame_accounting.fps_warning_state =
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
                    frame_accounting.fps_warning_state
                );


                println!();


                if frame_accounting.fps_warning_state
                    != crate::fps_monitor::FpsWarningState::Normal
                {
                    notify_active_wallpaper(
                        &mut notification_state,
                        false,
                        runtime.notifications,
                        &active_shader_state.current,
                        &frame_context.pipeline.textures,
                        rendered_fps,
                        runtime_controls.animation_speed,
                        frame_accounting.fps_warning_state,
                    );
                }
            }


            frame_accounting.frame =
                frame_accounting.frame.saturating_add(
                    1
                );


            control.diagnostic_checkpoint(7);
            // Dispatch pacing to the owning rendering pipeline.
            if !windowed_rotation {
                S::pace_frame(&mut next_frame_deadline, frame_duration);
            }
            Ok(FrameLoopAction::Continue)
        });


    frame_context.release();




    if status_enabled {
        crate::renderer_status::publish(status_pipeline, crate::renderer_status::State::Stopping);
        crate::renderer_status::clear(status_pipeline);
    }
    result
}


// Stage 4B-5B: context binding and presentation are reusable EGL operations.
// Wayland frame-callback eligibility and callback requests remain exclusively
// with the owning pipeline; these helpers never synthesize frame callbacks.
// Stage 4B-5B: one shared scene executor, independent of Wayland dispatch,
// frame eligibility, swap scheduling, overlays, and shader rotation policy.
// The caller must have made the target's EGL context current and resized its
// post-processing pipeline before entering this function.
struct ShaderSceneUniforms {
    time: f32,
    frame: i32,
    i_time: i32,
    i_resolution: i32,
    i_mouse: i32,
    i_frame: i32,
}

fn execute_shader_scene_frame(
    gl_resources: &SharedFrameGlResources,
    textures: &crate::manage_textures::TextureManager,
    postprocess: &mut crate::postprocess_shader::PostprocessPipeline,
    uniforms: ShaderSceneUniforms,
) {
    postprocess.bind_scene_target();
    let (scene_width, scene_height) = postprocess.scene_dimensions();
    unsafe {
        gl::Disable(gl::BLEND);
        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        gl::ClearColor(0.0, 0.0, 0.0, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::FALSE);
        gl::UseProgram(gl_resources.program);
    }
    textures.bind_channels();
    unsafe {
        set_uniform_1f(uniforms.i_time, uniforms.time);
        set_uniform_3f(uniforms.i_resolution, scene_width as f32, scene_height as f32, 1.0);
        set_uniform_4f(uniforms.i_mouse, 0.0, 0.0, 0.0, 0.0);
        set_uniform_1i(uniforms.i_frame, uniforms.frame);
        gl::BindVertexArray(gl_resources.vertex_array);
        gl::DrawArrays(gl::TRIANGLES, 0, 3);
        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
    }
    postprocess.present_scene();
}

fn bind_egl_frame_target(display: EglDisplay, context: EglContext, surface: *mut std::ffi::c_void) -> Result<(), String> {
    if unsafe { eglMakeCurrent(display, surface, surface, context) } == EGL_FALSE {
        return Err(egl_failure("eglMakeCurrent"));
    }
    Ok(())
}

fn present_egl_frame_target(display: EglDisplay, surface: *mut std::ffi::c_void) -> Result<Duration, String> {
    let started = Instant::now();
    if unsafe { eglSwapBuffers(display, surface) } == EGL_FALSE {
        return Err(egl_failure("eglSwapBuffers"));
    }
    Ok(started.elapsed())
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

// Pipeline-neutral target maintenance, invoked after the owning pipeline has
// dispatched compositor events.  The false result means no presentation
// targets remain; it does not decide whether a frame is callback-authorized.
// Preserve the original ordering: destroy disconnected EGL targets, prune
// surface records, check for zero targets, then apply pending resizes.
fn reconcile_shared_frame_targets<S: RenderWaylandDispatch>(
    display: EglDisplay,
    context: EglContext,
    state: &mut S,
    egl_targets: &mut Vec<EglWallpaperTarget>,
    native_targets: &mut Vec<NativeWallpaperTarget>,
    postprocess_pipelines: &mut HashMap<u32, crate::postprocess_shader::PostprocessPipeline>,
) -> Result<bool, String> {
    remove_disconnected_targets_from_names(
        display, context, state.removed_outputs(), egl_targets,
        native_targets, postprocess_pipelines,
    )?;
    state.surfaces().retain(|surface| {
        native_targets.iter().any(|target| {
            target.info.registry_name == surface.registry_name
        })
    });
    if native_targets.is_empty() {
        return Ok(false);
    }
    apply_pending_target_resizes_from_states(state.surfaces(), native_targets)?;
    Ok(true)
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

