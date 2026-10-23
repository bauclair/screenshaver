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
pub(crate) struct FramePresentationDecision {
    pub(crate) recovery: Option<FrameClockRecovery>,
}

pub(crate) struct FrameClockRecovery {
    pub(crate) rotation_shift: Duration,
    pub(crate) animation_shift: Duration,
}

impl FramePresentationDecision {
    // The pipeline's callback controller has already authorized this frame.
    // These constructors only normalize its clock-recovery result; they
    // cannot create compositor presentation eligibility.
    pub(crate) fn from_recovery_durations(recovery: Option<(Duration, Duration)>) -> Self {
        Self {
            recovery: recovery.map(|(rotation_shift, animation_shift)| FrameClockRecovery {
                rotation_shift,
                animation_shift,
            }),
        }
    }
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
    // Uniform locations belong to the active GL program, not the frame loop.
    uniforms: (i32, i32, i32, i32),
}

impl SharedFrameContext {
    fn new(gl: SharedFrameGlResources, pipeline: SharedFramePipelineResources) -> Self {
        Self { gl, pipeline, uniforms: (-1, -1, -1, -1) }
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

    // Complete the GPU measurement before the potentially blocking compositor
    // swap, then acknowledge the successful swap to the owning dispatch state.
    // The caller alone owns wl_surface.frame callback requests/eligibility.
    fn present_and_acknowledge<S: RenderWaylandDispatch>(
        &self,
        display: EglDisplay,
        surface: *mut std::ffi::c_void,
        registry_name: u32,
        state: &mut S,
        control: &crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
    ) -> Result<Duration, String> {
        self.finish_frame_gpu_measurement();
        control.diagnostic_checkpoint(5);
        let duration = present_egl_frame_target(display, surface)?;
        control.diagnostic_checkpoint(6);
        state.on_frame_swap_completed(registry_name);
        Ok(duration)
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

    // Activate a prepared rotation candidate. Uniform lookup must succeed
    // before retiring the active textures/program, as in the original loop.
    fn activate_rotation_candidate(
        &mut self,
        program: u32,
        textures: crate::manage_textures::TextureManager,
        shader_inputs: &[crate::isf_types::ShaderInput],
    ) -> Result<(i32, i32, i32, i32), String> {
        unsafe { gl::UseProgram(program); }
        crate::apply_shader_inputs::apply(program, shader_inputs);
        let uniforms = Self::shader_uniform_locations(program)?;
        self.commit_shader_replacement(program, textures);
        Ok(uniforms)
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

    // Account only for completed GPU work, never compositor swap latency.
    // Returns true precisely when the warning state changes to non-Normal;
    // the pipeline caller retains ownership of user notifications.
    fn record_completed_gpu_frame(&mut self, gpu_duration: Duration, target_fps: u32) -> bool {
        let performance = self.frame_times.record(gpu_duration, target_fps);
        if performance.warning_state == self.fps_warning_state {
            return false;
        }
        self.fps_warning_state = performance.warning_state;
        println!("Wallpaper performance changed:");
        println!("    Target FPS: {}", target_fps);
        println!("    Average FPS: {}", performance.average_fps);
        println!("    State: {:?}", self.fps_warning_state);
        println!();
        self.fps_warning_state != crate::fps_monitor::FpsWarningState::Normal
    }

    // A compositor stall must not contaminate the next GPU FPS epoch.
    fn reset_after_presentation_stall(&mut self, swap_duration: Duration) {
        if swap_duration >= Duration::from_millis(250) {
            self.frame_times.clear();
            self.fps_warning_state = crate::fps_monitor::FpsWarningState::Normal;
        }
    }

    fn advance_presented_frame(&mut self) {
        self.frame = self.frame.saturating_add(1);
    }
}

// Increment 3: keep the configured frame cadence and Wallpaper deadline in
// one persistent owner. Windowshader continues to own its separate compositor-
// callback pacing controller; this state never authorizes presentation.
struct SharedFramePacingState {
    rendered_fps: u32,
    frame_duration: Duration,
    next_frame_deadline: Instant,
}

impl SharedFramePacingState {
    fn new(initial: FrameLoopInitialTiming) -> Self {
        Self {
            rendered_fps: initial.rendered_fps,
            frame_duration: initial.frame_duration,
            next_frame_deadline: initial.next_frame_deadline,
        }
    }

    fn update_rendered_fps(&mut self, rendered_fps: u32) {
        self.rendered_fps = rendered_fps;
        self.frame_duration = frame_duration_for_fps(rendered_fps);
    }

    fn reset_wallpaper_deadline<S: RenderWaylandDispatch>(&mut self) {
        S::reset_frame_deadline(&mut self.next_frame_deadline);
    }

    fn pace_wallpaper<S: RenderWaylandDispatch>(&mut self) {
        S::pace_frame(&mut self.next_frame_deadline, self.frame_duration);
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

// Stage 4B-5E: consolidate the persistent frame-control state into one
// lifetime. This is storage ownership only; the Windowshader callback gate
// and Wallpaper loop semantics are deliberately unchanged.
struct SharedFrameRuntimeState {
    clock: SharedFrameClockState,
    accounting: SharedFrameAccountingState,
    shader: SharedActiveShaderState,
    controls: SharedFrameRuntimeControls,
    pacing: SharedFramePacingState,
    notifications: crate::notify_wallpaper::WallpaperNotificationState,
}

impl SharedFrameRuntimeState {
    fn new(
        shader: ActiveWallpaperShader,
        animation_speed: f32,
        audio_required: bool,
        initial_timing: FrameLoopInitialTiming,
    ) -> Self {
        Self {
            clock: SharedFrameClockState::new(),
            accounting: SharedFrameAccountingState::new(),
            shader: SharedActiveShaderState::new(shader),
            controls: SharedFrameRuntimeControls::new(animation_speed, audio_required),
            pacing: SharedFramePacingState::new(initial_timing),
            notifications: crate::notify_wallpaper::WallpaperNotificationState::new(),
        }
    }
}

// Audio-motion state is persistent rendering state, not a compositor scheduling
// decision. Keep effect selection, smoothing, lyric tracking, and update timing
// together so a future pipeline-owned loop can borrow them as one unit.
struct SharedFrameAudioMotionState {
    effect: crate::render_audio_motion::AudioMotionEffect,
    motion: crate::render_audio_motion::AudioMotionState,
    lyrics_manager: Option<crate::manage_lyrics::LyricsManager>,
    last_update: Instant,
}

impl SharedFrameAudioMotionState {
    fn new(effect: crate::render_audio_motion::AudioMotionEffect) -> Self {
        let lyrics_manager = if matches!(
            effect,
            crate::render_audio_motion::AudioMotionEffect::WooferFromHell
        ) {
            crate::manage_lyrics::LyricsManager::start().ok()
        } else {
            None
        };
        Self {
            effect,
            motion: crate::render_audio_motion::AudioMotionState::default(),
            lyrics_manager,
            last_update: Instant::now(),
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

// Shared policy-reload preparation transaction. The candidate texture manager
// remains isolated from the active rendering context until the caller commits
// it after the post-processing profile has been applied successfully.
// Windowshader owns *when* a reload is requested; this function owns only
// the reusable, pipeline-neutral preparation operation.
fn prepare_shared_policy_reload_textures(
    reload: &crate::manage_wallpaper_runtime::WallpaperPolicyReload,
    shader: &ActiveWallpaperShader,
) -> Result<crate::manage_textures::TextureManager, String> {
    let mut candidate = crate::manage_textures::TextureManager::new(
        reload.texture_policy.clone(),
    );
    candidate.prepare_for_policy_with_path(
        shader.policy_id,
        &shader.shader_name,
        Some(shader.source_path.as_path()),
        shader.channel_usage,
    )?;
    Ok(candidate)
}

// Stage 2D-2C Step 16 (consolidated): execution-loop drivers are owned by
// the presentation pipeline. Render Core provides one reusable frame step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FrameLoopAction {
    Continue,
    Stop,
}

// Shared shader-rotation execution prerequisite.  Both Wallpaper and
// Windowshader use the same EGL context-binding operation; neither pipeline
// delegates its rotation scheduling or callback authorization here.
fn bind_rotation_target(
    display: EglDisplay,
    context: EglContext,
    targets: &[EglWallpaperTarget],
) -> Result<(), String> {
    if let Some(first_target) = targets.first() {
        if unsafe {
            eglMakeCurrent(display, first_target.surface, first_target.surface, context)
        } == EGL_FALSE {
            return Err(egl_failure("eglMakeCurrent before switching wallpaper shaders"));
        }
    }
    Ok(())
}

// Select and compile a rotation candidate without changing the active GL
// program or its textures. The caller retains responsibility for recording
// pipeline-specific outcomes and committing a successful replacement.
fn select_and_compile_rotation_candidate(
    shader_manager: &mut crate::manage_shader::ShaderManager,
    wallpaper_directory: &Path,
) -> Result<(ActiveWallpaperShader, Result<u32, String>), String> {
    let shader = select_safe_wallpaper_shader(shader_manager, wallpaper_directory)?;
    let program = crate::compile_shader::build_program(
        crate::define_constants::VERTEX_SHADER,
        &shader.source,
    ).map_err(|error| error.to_string());
    Ok((shader, program))
}

// A pipeline-neutral rotation executor. The owning pipeline decides *when*
// rotation is permitted; this operation only selects, compiles, prepares and
// activates the candidate. Failure outcomes preserve the caller's existing
// diagnostics and rotation bookkeeping.
enum RotationExecutionOutcome {
    Activated { shader: ActiveWallpaperShader, uniforms: (i32, i32, i32, i32) },
    SelectionFailed(String),
    CompilationFailed { shader: ActiveWallpaperShader, error: String },
    TextureFailed { shader: ActiveWallpaperShader, error: String },
}

fn execute_shader_rotation(
    display: EglDisplay,
    context: EglContext,
    targets: &[EglWallpaperTarget],
    shader_manager: &mut crate::manage_shader::ShaderManager,
    wallpaper_directory: &Path,
    frame_context: &mut SharedFrameContext,
    runtime: &crate::define_wallpaper::WallpaperRuntime,
) -> Result<RotationExecutionOutcome, String> {
    bind_rotation_target(display, context, targets)?;
    let (shader, compilation) = match select_and_compile_rotation_candidate(
        shader_manager, wallpaper_directory,
    ) {
        Ok(candidate) => candidate,
        Err(error) => return Ok(RotationExecutionOutcome::SelectionFailed(error)),
    };
    let program = match compilation {
        Ok(program) => program,
        Err(error) => return Ok(RotationExecutionOutcome::CompilationFailed { shader, error }),
    };
    let textures = match frame_context.prepare_replacement_textures(&shader, runtime, program) {
        Ok(textures) => textures,
        Err(error) => {
            unsafe { gl::DeleteProgram(program); }
            return Ok(RotationExecutionOutcome::TextureFailed { shader, error });
        }
    };
    let uniforms = frame_context.activate_rotation_candidate(
        program, textures, &shader.shader_inputs,
    )?;
    Ok(RotationExecutionOutcome::Activated { shader, uniforms })
}

// Increment 36: explicit scheduling result before shared GPU execution.
// The compositor-owning pipeline decides whether a frame is eligible;
// a skipped iteration must never reach GL drawing or EGL presentation.
enum FrameSchedulingDecision {
    Skip,
    Ready {
        presentation: FramePresentationDecision,
        rotate: bool,
    },
}

pub(crate) type FrameStep<'a> = dyn FnMut() -> Result<FrameLoopAction, String> + 'a;
pub(crate) type FrameLoopDriver = for<'a> fn(&mut FrameStep<'a>) -> Result<(), String>;

// Increment 37: a scoped execution session keeps the per-frame operation
// borrowed for the duration of the owning pipeline's loop.  The session
// must be dropped before Render Core releases persistent GL resources.
struct FrameExecutionSession<F> {
    step: F,
}

impl<F> FrameExecutionSession<F>
where
    F: FnMut() -> Result<FrameLoopAction, String>,
{
    fn run(&mut self, driver: FrameLoopDriver) -> Result<(), String> {
        driver(&mut self.step)
    }
}

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

    let mut frame_runtime = SharedFrameRuntimeState::new(
        active_shader.clone(),
        initial_animation_speed,
        initial_audio_required,
        initial_timing,
    );

    notify_active_wallpaper(
        &mut frame_runtime.notifications,
        true,
        runtime.notifications,
        active_shader,
        &frame_context.pipeline.textures,
        frame_runtime.pacing.rendered_fps,
        frame_runtime.controls.animation_speed,
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



    let mut frame_audio = SharedFrameAudioMotionState::new(
        initial_postprocess_profile.audio_motion,
    );


    frame_context.uniforms =
        SharedFrameContext::shader_uniform_locations(frame_context.gl.program)?;




    // The initial wallpaper has now loaded, compiled, prepared textures,
    // created post-processing pipelines, and resolved its uniforms. Persist
    // Ordered position only after reaching this accepted render state.
    shader_manager.record_rendered_policy(
        frame_runtime.shader.current.policy_id
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
            frame_runtime.clock.shader_switched,
            frame_runtime.pacing.next_frame_deadline,
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
    

    // Keep the GPU execution step separate from the pipeline-owned loop driver.
    // This closure is still hosted by Render Core; the next ownership transfer
    // will move its scheduling decisions into the Windowshader controller.
    let mut execute_frame_step = || -> Result<FrameLoopAction, String> {

            // The Windowshader pipeline owns the ordered pre-frame control
            // sequence. The shared core supplies only target reconciliation.
            let pending_policy_reload = if windowed_rotation {
                let preflight = crate::wayland_windowshader::run_windowshader_frame_preflight(
                    windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode"),
                    event_queue, state, native_targets, frame_runtime.clock.paused,
                    control, shader_interval, &frame_runtime.shader.current.shader_name,
                    frame_runtime.shader.current.policy_id, frame_context.gl.program, frame_runtime.accounting.frame,
                    running,
                    |state, native_targets| reconcile_shared_frame_targets(
                        display, context, state, egl_targets, native_targets,
                        &mut frame_context.pipeline.postprocess,
                    ),
                )?;
                match preflight {
                    crate::wayland_windowshader::WindowshaderPreflightOutcome::Stop => {
                        return Ok(FrameLoopAction::Stop);
                    }
                    crate::wayland_windowshader::WindowshaderPreflightOutcome::Continue {
                        pending_policy_reload,
                    } => pending_policy_reload,
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
                control.take_policy_reload()
            };

            // The Windowshader preflight owns reload-request consumption;
            // the shared executor still applies GL/policy updates transactionally.
            if let Some(reload) = pending_policy_reload {
                match prepare_shared_policy_reload_textures(
                    &reload,
                    &frame_runtime.shader.current,
                ) {
                    Ok(mut replacement_texture_manager) => {
                        let replacement_profile =
                            reload.postprocess_policy
                                .profile_for_policy(
                                    frame_runtime.shader.current.policy_id,
                                    &frame_runtime.shader.current.shader_name,
                                    Some(
                                        frame_runtime.shader.current.source_path.as_path()
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

                            frame_runtime.controls.audio_required =
                                matches!(
                                    replacement_profile.bloom.name(),
                                    "audio" | "spectral" | "loudness"
                                ) || replacement_profile.audio_motion.is_enabled();

                            frame_audio.effect =
                                replacement_profile.audio_motion;

                            if matches!(
                                frame_audio.effect,
                                crate::render_audio_motion::AudioMotionEffect::WooferFromHell
                            ) && frame_audio.lyrics_manager.is_none()
                            {
                                frame_audio.lyrics_manager =
                                    crate::manage_lyrics::LyricsManager::start().ok();
                            }

                            if !matches!(
                                frame_audio.effect,
                                crate::render_audio_motion::AudioMotionEffect::WooferFromHell
                            ) {
                                frame_audio.lyrics_manager = None;
                            }

                            if !frame_audio.effect.is_enabled() {
                                frame_audio.lyrics_manager = None;
                                frame_audio.motion.reset();

                                for pipeline in
                                    frame_context.pipeline.postprocess.values_mut()
                                {
                                    pipeline.set_audio_motion_scale(
                                        1.0
                                    );
                                }
                            }

                            frame_audio.last_update =
                                Instant::now();

                            frame_runtime.controls.animation_speed =
                                reload.animation_speed_policy
                                    .animation_speed_for_policy(
                                        frame_runtime.shader.current.policy_id,
                                        &frame_runtime.shader.current.shader_name,
                                        Some(
                                            frame_runtime.shader.current.source_path.as_path()
                                        ),
                                        None,
                                    );

                            frame_runtime.pacing.update_rendered_fps(
                                reload.fps_policy.rendered_fps_for_policy(
                                        frame_runtime.shader.current.policy_id,
                                        &frame_runtime.shader.current.shader_name,
                                        Some(
                                            frame_runtime.shader.current.source_path.as_path()
                                        ),
                                        None,
                                    )
                            );

                            if windowed_rotation {
                                windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode").on_schedule_discontinuity(
                                    crate::wayland_windowshader::WindowshaderScheduleReset::PolicyOrShaderChange,
                                );
                            } else {
                                frame_runtime.pacing.reset_wallpaper_deadline::<S>();
                            }

                            frame_runtime.accounting.frame_times.clear();
                            frame_runtime.accounting.fps_warning_state =
                                crate::fps_monitor::FpsWarningState::Normal;

                            notify_active_wallpaper(
                                &mut frame_runtime.notifications,
                                false,
                                runtime.notifications,
                                &frame_runtime.shader.current,
                                &frame_context.pipeline.textures,
                                frame_runtime.pacing.rendered_fps,
                                frame_runtime.controls.animation_speed,
                                frame_runtime.accounting.fps_warning_state,
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
            // Windowshader owns its complete pause transition, including the
            // idle wait and release of its animation clock. Wallpaper retains
            // its original pause path below.
            if windowed_rotation {
                match windowshader_state.as_mut()
                    .expect("Windowshader controller exists only in Windowed mode")
                    .process_pause_transition(
                        state, native_targets, control, status_enabled,
                        &mut frame_runtime.clock.paused,
                        &mut frame_runtime.clock.animation_started,
                    ) {
                    crate::wayland_windowshader::WindowshaderPauseTransition::Idle => {
                        return Ok(FrameLoopAction::Continue);
                    }
                    crate::wayland_windowshader::WindowshaderPauseTransition::Resumed => {
                        frame_runtime.accounting.frame_times.clear();
                    }
                    crate::wayland_windowshader::WindowshaderPauseTransition::Running => {}
                }
            } else {
                if control.pause_requested() {
                    if !frame_runtime.clock.paused {
                        crate::logger::warning(
                            &crate::locate_paths::runtime_log_path(),
                            "[AMBIENT_HANDOFF_DIAG] Wayland renderer detected pause request",
                        );
                        control.acknowledge_pause_detected();
                        state.on_pause_entering();
                        control.diagnostic_checkpoint(8);
                        state.on_pause_entered(native_targets);
                        frame_runtime.clock.pause_started = Some(Instant::now());
                        control.acknowledge_paused();
                        if status_enabled {
                            crate::renderer_status::publish(status_pipeline, crate::renderer_status::State::Paused);
                        }
                        println!("Wayland wallpaper rendering paused.");
                        frame_runtime.clock.paused = true;
                    }
                    thread::sleep(Duration::from_millis(10));
                    return Ok(FrameLoopAction::Continue);
                }

                if frame_runtime.clock.paused {
                    let pause_ended_at = Instant::now();
                    if let Some(paused_at) = frame_runtime.clock.pause_started.take() {
                        let duration = pause_ended_at.duration_since(paused_at);
                        frame_runtime.clock.animation_started += duration;
                        frame_runtime.clock.shader_switched += duration;
                    }
                    frame_runtime.pacing.reset_wallpaper_deadline::<S>();
                    frame_runtime.accounting.frame_times.clear();
                    frame_runtime.clock.paused = false;
                }
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
            let scheduling = if windowed_rotation {
                match crate::wayland_windowshader::authorize_windowshader_presentation(
                    windowshader_state.as_mut()
                        .expect("Windowshader controller exists only in Windowed mode"),
                    control, state, native_targets, shader_interval,
                    &mut frame_runtime.clock.animation_started, frame_runtime.clock.paused,
                    &frame_runtime.shader.current.shader_name,
                    frame_runtime.shader.current.policy_id, frame_context.gl.program,
                ) {
                    crate::wayland_windowshader::WindowshaderFrameGate::Skip => {
                        FrameSchedulingDecision::Skip
                    }
                    crate::wayland_windowshader::WindowshaderFrameGate::Present {
                        presentation, rotate,
                    } => FrameSchedulingDecision::Ready { presentation, rotate },
                }
            } else {
                match wallpaper_callbacks.evaluate(
                    state, native_targets, frame_runtime.clock.shader_switched,
                    frame_runtime.clock.animation_started,
                ) {
                    Some(recovery) => {
                        let rotate = !frame_runtime.clock.paused
                            && shader_interval.is_some_and(|interval| {
                                frame_runtime.clock.shader_switched.elapsed() >= interval
                            });
                        FrameSchedulingDecision::Ready {
                            presentation: FramePresentationDecision::from_recovery_durations(recovery),
                            rotate,
                        }
                    }
                    None => FrameSchedulingDecision::Skip,
                }
            };
            let (presentation, rotation_requested) = match scheduling {
                FrameSchedulingDecision::Skip => return Ok(FrameLoopAction::Continue),
                FrameSchedulingDecision::Ready { presentation, rotate } => (presentation, rotate),
            };

            // Each pipeline applies its own callback-recovery clock policy.
            // Windowshader never advances shader rotation for hidden time;
            // Wallpaper retains its historical rotation/deadline adjustment.
            let recovered_from_callback_gap = if windowed_rotation {
                // Windowshader applied its animation-clock recovery while
                // authorizing this compositor callback.
                presentation.recovery.is_some()
            } else if let Some(recovery) = presentation.recovery.as_ref() {
                frame_runtime.clock.animation_started += recovery.animation_shift;
                frame_runtime.clock.shader_switched += recovery.rotation_shift;
                frame_runtime.pacing.reset_wallpaper_deadline::<S>();
                true
            } else {
                false
            };
            if recovered_from_callback_gap {
                frame_runtime.accounting.frame_times.clear();
                frame_runtime.accounting.fps_warning_state = crate::fps_monitor::FpsWarningState::Normal;
            }

            // Resolve Wallpaper rotation-clock ownership before entering the
            // pipeline-neutral shader selection/compilation execution block.
            // This remains after callback-gap recovery to preserve the prior
            // clock adjustment order. Windowshader resets its own rotation
            // anchor in begin_due_rotation().
            if rotation_requested && !windowed_rotation {
                frame_runtime.clock.shader_switched = Instant::now();
            }

            if rotation_requested {
                match execute_shader_rotation(
                    display, context, egl_targets, shader_manager,
                    wallpaper_directory, &mut frame_context, runtime,
                )? {
                    RotationExecutionOutcome::Activated {
                        shader: next_shader,
                        uniforms: (next_i_time, next_i_resolution, next_i_mouse, next_i_frame),
                    } => {
                                frame_context.uniforms = (
                                    next_i_time,
                                    next_i_resolution,
                                    next_i_mouse,
                                    next_i_frame,
                                );

                                frame_runtime.clock.animation_started =
                                    Instant::now();


                                frame_runtime.accounting.frame =
                                    0;


                                frame_runtime.controls.animation_speed =
                                    runtime.animation_speed_policy
                                        .animation_speed_for_policy(
                                            next_shader.policy_id,
                                            &next_shader.shader_name,
                                            Some(
                                                next_shader.source_path.as_path()
                                            ),
                                            None,
                                        );


                                frame_runtime.pacing.update_rendered_fps(
                                runtime.fps_policy.rendered_fps_for_policy(
                                        next_shader.policy_id,
                                        &next_shader.shader_name,
                                        Some(
                                            next_shader.source_path.as_path()
                                        ),
                                        None,
                                    )
                            );


                                if windowed_rotation {
                                windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode").on_schedule_discontinuity(
                                    crate::wayland_windowshader::WindowshaderScheduleReset::PolicyOrShaderChange,
                                );
                            } else {
                                frame_runtime.pacing.reset_wallpaper_deadline::<S>();
                            }


                                frame_runtime.shader.accept_replacement(next_shader.clone());
                                if windowed_rotation {
                                    windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode").record_rotation_success(
                                        &frame_runtime.shader.current.shader_name, frame_runtime.shader.current.policy_id, frame_context.gl.program,
                                    );
                                }


                                let postprocess_profile =
                                    runtime.postprocess_policy
                                        .profile_for_policy(
                                            frame_runtime.shader.current.policy_id,
                                            &frame_runtime.shader.current.shader_name,
                                            Some(
                                                frame_runtime.shader.current.source_path.as_path()
                                            ),
                                        );


                                frame_runtime.controls.audio_required =
                                    matches!(
                                        postprocess_profile.bloom.name(),
                                        "audio" | "spectral" | "loudness"
                                    ) || postprocess_profile.audio_motion.is_enabled();


                                frame_audio.effect =
                                    postprocess_profile.audio_motion;


                                if matches!(
                                    frame_audio.effect,
                                    crate::render_audio_motion::AudioMotionEffect::WooferFromHell
                                ) && frame_audio.lyrics_manager.is_none()
                                {
                                    frame_audio.lyrics_manager =
                                        crate::manage_lyrics::LyricsManager::start().ok();
                                }

                                if !matches!(
                                    frame_audio.effect,
                                    crate::render_audio_motion::AudioMotionEffect::WooferFromHell
                                ) {
                                    frame_audio.lyrics_manager = None;
                                }


                                if !frame_audio.effect.is_enabled() {
                                    frame_audio.lyrics_manager =
                                        None;

                                    frame_audio.motion.reset();
                                }


                                frame_audio.last_update =
                                    Instant::now();


                                for pipeline in
                                    frame_context.pipeline.postprocess.values_mut()
                                {
                                    pipeline.set_profile(
                                        postprocess_profile
                                    )?;
                                }


                                runtime.tray_status.set_active(
                                    frame_runtime.shader.current.policy_id,
                                    frame_runtime.shader.current.shader_name.clone(),
                                    frame_runtime.shader.current.source_path.clone(),
                                );


                                shader_manager.record_rendered_policy(
                                    frame_runtime.shader.current.policy_id
                                );


                                frame_runtime.accounting.frame_times.clear();


                                frame_runtime.accounting.fps_warning_state =
                                    crate::fps_monitor::FpsWarningState::Normal;


                                notify_active_wallpaper(
                                    &mut frame_runtime.notifications,
                                    true,
                                    runtime.notifications,
                                    &frame_runtime.shader.current,
                                    &frame_context.pipeline.textures,
                                    frame_runtime.pacing.rendered_fps,
                                    frame_runtime.controls.animation_speed,
                                    frame_runtime.accounting.fps_warning_state,
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
                                    frame_runtime.pacing.rendered_fps
                                );


                                println!(
                                    "    Animation speed: {:.3}x",
                                    frame_runtime.controls.animation_speed
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
                    RotationExecutionOutcome::TextureFailed { shader, error } => {
                        println!("Wallpaper shader texture preparation failed; keeping the current shader:");
                        println!("    Shader: {}", shader.shader_name);
                        println!("    Error: {}", error);
                        println!();
                        shader_manager.remove_shader(&shader.manager_name);
                        return Ok(FrameLoopAction::Continue);
                    }
                    RotationExecutionOutcome::CompilationFailed { shader, error } => {
                        if windowed_rotation {
                            windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode")
                                .record_rotation_compile_failure(&shader.shader_name, &error);
                        }
                        println!("Wallpaper shader compilation failed; keeping the current shader:");
                        println!("    Shader: {}", shader.shader_name);
                        println!("    Error: {}", error);
                        println!();
                        shader_manager.remove_shader(&shader.manager_name);
                    }
                    RotationExecutionOutcome::SelectionFailed(error) => {
                        if windowed_rotation {
                            windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode")
                                .record_rotation_selection_failure(&error);
                        }
                        println!("Wallpaper shader selection failed; keeping the current shader:");
                        println!("    Error: {}", error);
                        println!();
                    }
                }
            }


            let elapsed =
                frame_runtime.clock.animation_started.elapsed();


            frame_context.begin_frame_gpu_measurement();

            audio_demand.set_required(
                frame_runtime.controls.audio_required
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
                    frame_audio.last_update
                )
                .as_secs_f32();

            frame_audio.last_update =
                now;

            let mut audio_motion_scale =
                1.0_f32;

            let mut audio_motion_fft_trace =
                [0.0_f32; crate::analyze_audio::AUDIO_MOTION_TRACE_CHANNELS];

            match frame_audio.effect {
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
                        frame_audio.motion.update(
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
                        frame_audio.motion.update_fft_mirror_warp(
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
                        frame_audio.motion.update_polar_propeller(
                            spectrum,
                            audio_motion_frame_seconds,
                        );
                }

                crate::render_audio_motion::AudioMotionEffect::Off => {
                    frame_audio.motion.reset();
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


                let (target_width, target_height) = execute_target_shader_scene(
                    &frame_context.gl,
                    &frame_context.pipeline.textures,
                    postprocess,
                    native_target.info.registry_name,
                    native_target.width,
                    native_target.height,
                    FrameSceneEffects {
                        audio_bands: current_audio_bands,
                        audio_motion_scale,
                        audio_motion_fft_trace,
                        rotation_turns: frame_audio.motion.polar_rotation_turns(),
                    },
                    ShaderSceneUniforms {
                        time: elapsed.as_secs_f32() * frame_runtime.controls.animation_speed,
                        frame: frame_runtime.accounting.frame,
                        i_time: frame_context.uniforms.0,
                        i_resolution: frame_context.uniforms.1,
                        i_mouse: frame_context.uniforms.2,
                        i_frame: frame_context.uniforms.3,
                    },
                )?;

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
                let presentation_duration = frame_context.present_and_acknowledge(
                    display,
                    egl_target.surface,
                    native_target.info.registry_name,
                    state,
                    control,
                )?;
                // Pipeline-specific presentation accounting belongs to the
                // Windowshader controller; Wallpaper retains its own heartbeat.
                if windowed_rotation {
                    // Windowshader owns its post-swap status heartbeat decision
                    // and timestamp update; only successful swaps reach here.
                    windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode")
                        .record_windowshader_swap_and_publish_status(
                            frame_runtime.clock.paused,
                            &mut frame_runtime.controls.status_heartbeat,
                            status_enabled,
                            status_pipeline,
                        );
                } else {
                    let publish_rendering = frame_runtime.clock.paused
                        || frame_runtime.controls.status_heartbeat.elapsed() >= Duration::from_secs(1);
                    if status_enabled && publish_rendering {
                        crate::renderer_status::publish(status_pipeline, crate::renderer_status::State::Rendering);
                        frame_runtime.controls.status_heartbeat = Instant::now();
                    }
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
                frame_runtime.accounting.reset_after_presentation_stall(presentation_duration);
            }


            // Windowshader owns its post-pause presentation acknowledgment.
            // Wallpaper retains its existing local paused-state transition.
            let presentation_resumed = if windowed_rotation {
                windowshader_state.as_mut().expect("Windowshader controller exists only in Windowed mode").complete_presented_frame_and_pace(frame_runtime.clock.paused, frame_runtime.pacing.frame_duration)
            } else {
                frame_runtime.clock.paused
            };
            if presentation_resumed {
                frame_runtime.clock.paused = false;

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
            if frame_runtime.accounting.record_completed_gpu_frame(gpu_render_duration, frame_runtime.pacing.rendered_fps) {
                notify_active_wallpaper(
                    &mut frame_runtime.notifications,
                    false,
                    runtime.notifications,
                    &frame_runtime.shader.current,
                    &frame_context.pipeline.textures,
                    frame_runtime.pacing.rendered_fps,
                    frame_runtime.controls.animation_speed,
                    frame_runtime.accounting.fps_warning_state,
                );
            }
            frame_runtime.accounting.advance_presented_frame();

            control.diagnostic_checkpoint(7);
            // Dispatch pacing to the owning rendering pipeline.
            if !windowed_rotation {
                frame_runtime.pacing.pace_wallpaper::<S>();
            }
            Ok(FrameLoopAction::Continue)
        };
    let mut execution_session = FrameExecutionSession { step: execute_frame_step };
    let result = execution_session.run(loop_driver);
    // Release all borrows held by the frame operation before GL teardown.
    drop(execution_session);

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

// Pipeline-neutral per-target scene execution. The caller owns compositor
// eligibility, EGL context binding, overlays and presentation. Keeping these
// operations together ensures effect state and target dimensions are applied
// before the scene draw on both Wallpaper and Windowshader paths.
struct FrameSceneEffects {
    audio_bands: crate::analyze_audio::AudioBands,
    audio_motion_scale: f32,
    audio_motion_fft_trace: [f32; crate::analyze_audio::AUDIO_MOTION_TRACE_CHANNELS],
    rotation_turns: f32,
}

fn execute_target_shader_scene(
    gl_resources: &SharedFrameGlResources,
    textures: &crate::manage_textures::TextureManager,
    postprocess: &mut crate::postprocess_shader::PostprocessPipeline,
    registry_name: u32,
    width: i32,
    height: i32,
    effects: FrameSceneEffects,
    uniforms: ShaderSceneUniforms,
) -> Result<(u32, u32), String> {
    configure_frame_postprocess_effects(
        postprocess,
        effects.audio_bands,
        effects.audio_motion_scale,
        effects.audio_motion_fft_trace,
        effects.rotation_turns,
    );
    let dimensions = resize_frame_scene_target(postprocess, registry_name, width, height)?;
    execute_shader_scene_frame(gl_resources, textures, postprocess, uniforms);
    Ok(dimensions)
}

// Shared per-target effect configuration. Both rendering controllers can
// eventually call this before the pipeline-neutral shader scene executor.
// This operation does not request frames or authorize EGL presentation.
fn configure_frame_postprocess_effects(
    postprocess: &mut crate::postprocess_shader::PostprocessPipeline,
    audio_bands: crate::analyze_audio::AudioBands,
    audio_motion_scale: f32,
    audio_motion_fft_trace: [f32; crate::analyze_audio::AUDIO_MOTION_TRACE_CHANNELS],
    rotation_turns: f32,
) {
    postprocess.set_audio_bands(audio_bands);
    postprocess.set_audio_motion_scale(audio_motion_scale);
    postprocess.set_audio_motion_fft_trace(audio_motion_fft_trace);
    postprocess.set_audio_motion_rotation_turns(rotation_turns);
}

// Validate native target dimensions and resize the scene framebuffer before
// rendering. This does not grant presentation eligibility or swap EGL buffers.
fn resize_frame_scene_target(
    postprocess: &mut crate::postprocess_shader::PostprocessPipeline,
    registry_name: u32,
    width: i32,
    height: i32,
) -> Result<(u32, u32), String> {
    let target_width = u32::try_from(width).map_err(|_| {
        format!("Wallpaper target {} has an invalid width: {}", registry_name, width)
    })?;
    let target_height = u32::try_from(height).map_err(|_| {
        format!("Wallpaper target {} has an invalid height: {}", registry_name, height)
    })?;
    postprocess.resize(target_width, target_height)?;
    Ok((target_width, target_height))
}

// Prepare the offscreen scene and bind the active shader inputs. This is
// GPU execution only: it neither requests nor authorizes Wayland presentation.
fn prepare_shader_scene_draw(
    gl_resources: &SharedFrameGlResources,
    textures: &crate::manage_textures::TextureManager,
    postprocess: &mut crate::postprocess_shader::PostprocessPipeline,
) -> (u32, u32) {
    postprocess.bind_scene_target();
    let dimensions = postprocess.scene_dimensions();
    unsafe {
        gl::Disable(gl::BLEND);
        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        gl::ClearColor(0.0, 0.0, 0.0, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::FALSE);
        gl::UseProgram(gl_resources.program);
    }
    textures.bind_channels();
    dimensions
}

fn execute_shader_scene_frame(
    gl_resources: &SharedFrameGlResources,
    textures: &crate::manage_textures::TextureManager,
    postprocess: &mut crate::postprocess_shader::PostprocessPipeline,
    uniforms: ShaderSceneUniforms,
) {
    let (scene_width, scene_height) = prepare_shader_scene_draw(
        gl_resources,
        textures,
        postprocess,
    );
    draw_shader_scene_triangle(gl_resources, uniforms, scene_width, scene_height);
    postprocess.present_scene();
}

// Apply the active shader's per-frame uniforms and execute its fullscreen
// triangle. The caller owns scene preparation and post-processing; no Wayland
// presentation authorization or EGL buffer swap occurs here.
fn draw_shader_scene_triangle(
    gl_resources: &SharedFrameGlResources,
    uniforms: ShaderSceneUniforms,
    scene_width: u32,
    scene_height: u32,
) {
    unsafe {
        set_uniform_1f(uniforms.i_time, uniforms.time);
        set_uniform_3f(uniforms.i_resolution, scene_width as f32, scene_height as f32, 1.0);
        set_uniform_4f(uniforms.i_mouse, 0.0, 0.0, 0.0, 0.0);
        set_uniform_1i(uniforms.i_frame, uniforms.frame);
        gl::BindVertexArray(gl_resources.vertex_array);
        gl::DrawArrays(gl::TRIANGLES, 0, 3);
        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
    }
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

