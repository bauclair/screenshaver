//! Generic Wayland secure-lock fallback for Screenshaver.
//!
//! Desktop integrations with a supported native locker (currently KDE Plasma,
//! GNOME, and Xfce) keep security and authentication in that desktop's locker
//! and use Screenshaver only for shader presentation. This module is different
//! by design: it is the fallback for Wayland environments where no supported
//! desktop-native locker has been selected.
//!
//! The fallback validates the required Wayland capabilities, acquires
//! `ext-session-lock-v1`, owns the compositor-enforced lock surfaces and input,
//! performs PAM credential verification through `authenticate_user`, and is the
//! only code in this path authorized to request authenticated unlock. The
//! Screenshaver authentication circle is therefore a legitimate part of this
//! fallback backend rather than a desktop-native lock-screen replacement.

use std::ffi::CString;
use std::fs::File;
use std::io::{
    Read,
    Seek,
    SeekFrom,
    Write,
};
use std::os::fd::{
    AsFd,
    FromRawFd,
};
use std::path::Path;
use std::sync::atomic::{
    AtomicBool,
    Ordering,
};
use std::time::{
    Duration,
    Instant,
};

use wayland_client::{
    delegate_noop,
    protocol::{
        wl_buffer,
        wl_compositor,
        wl_output,
        wl_pointer,
        wl_registry,
        wl_seat,
        wl_shm,
        wl_keyboard,
        wl_shm_pool,
        wl_surface,
    },
    Connection,
    Dispatch,
    QueueHandle,
    WEnum,
};

use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1::ExtSessionLockManagerV1,
    ext_session_lock_surface_v1::{
        self,
        ExtSessionLockSurfaceV1,
    },
    ext_session_lock_v1::{
        self,
        ExtSessionLockV1,
    },
};


const AUTH_POINTER_MOVEMENT_THRESHOLD: f64 = 24.0;
const AUTHENTICATION_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(10);


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockInteractionState {
    Rendering,
    AuthenticationRequested,
    AuthenticationSucceeded,
}


struct OutputEntry {
    global_name: u32,
    output: wl_output::WlOutput,
}


struct LockFrame {
    backing_file: File,
    pool: wl_shm_pool::WlShmPool,
    buffer: wl_buffer::WlBuffer,
}


struct LockSurfaceEntry {
    output_name: u32,
    surface: wl_surface::WlSurface,
    lock_surface: ExtSessionLockSurfaceV1,
    configured: bool,
    width: u32,
    height: u32,
    frames: Vec<LockFrame>,
    gl_context: Option<crate::create_wayland_lock_context::WaylandLockContext>,
}


struct ScreenLockState {
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    lock_manager: Option<ExtSessionLockManagerV1>,
    seat: Option<wl_seat::WlSeat>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
    outputs: Vec<OutputEntry>,
    lock_surfaces: Vec<LockSurfaceEntry>,
    locked: bool,
    finished: bool,
    input_events: Vec<String>,
    interaction_state: LockInteractionState,
    pointer_anchor: Option<(f64, f64)>,
    authentication_deadline: Option<Instant>,
    xkb_state: Option<xkbcommon::xkb::State>,
    authentication:
        crate::display_lock_authentication::LockAuthentication,
}


impl ScreenLockState {
    fn new() -> Self {
        Self {
            compositor: None,
            shm: None,
            lock_manager: None,
            seat: None,
            keyboard: None,
            pointer: None,
            outputs: Vec::new(),
            lock_surfaces: Vec::new(),
            locked: false,
            finished: false,
            input_events: Vec::new(),
            interaction_state: LockInteractionState::Rendering,
            pointer_anchor: None,
            authentication_deadline: None,
            xkb_state: None,
            authentication:
                crate::display_lock_authentication::LockAuthentication::new(),
        }
    }


    fn all_surfaces_configured(
        &self
    ) -> bool {
        !self.lock_surfaces.is_empty()
            && self
                .lock_surfaces
                .iter()
                .all(
                    |entry| {
                        entry.configured
                    }
                )
    }


    fn begin_authentication(
        &mut self,
    ) {
        self.interaction_state =
            LockInteractionState::AuthenticationRequested;

        self.authentication.clear();

        self.authentication_deadline =
            Some(
                Instant::now()
                    + AUTHENTICATION_INACTIVITY_TIMEOUT
            );
    }


    fn note_authentication_activity(
        &mut self,
    ) {
        if self.interaction_state
            == LockInteractionState::AuthenticationRequested
        {
            self.authentication_deadline =
                Some(
                    Instant::now()
                        + AUTHENTICATION_INACTIVITY_TIMEOUT
                );
        }
    }


    fn dismiss_authentication_for_timeout(
        &mut self,
    ) -> bool {
        if self.interaction_state
            != LockInteractionState::AuthenticationRequested
        {
            return false;
        }

        let Some(deadline) =
            self.authentication_deadline
        else {
            return false;
        };

        if Instant::now()
            < deadline
        {
            return false;
        }

        self.authentication.clear();

        self.interaction_state =
            LockInteractionState::Rendering;

        self.authentication_deadline =
            None;

        self.pointer_anchor =
            None;

        true
    }
}


/// Engage the generic compositor-enforced Wayland fallback lock and remain
/// locked until Screenshaver PAM authentication succeeds.
///
/// This function is reached only after the desktop-specific KDE, Xfce, and
/// GNOME lock backends have not been selected. It verifies its own Wayland
/// prerequisites, including `ext-session-lock-v1`, before acquiring the secure
/// lock. Unlike the desktop-native integrations, this fallback intentionally
/// owns authentication and authenticated unlock because no supported native
/// locker owns those responsibilities on this path.
///
/// There is deliberately no secure-lock timeout, test bypass, force-unlock
/// parameter, or error-triggered unlock path in this production module. The
/// authentication widget may dismiss itself after input inactivity, but that
/// never releases the compositor-enforced session lock.
pub fn run(
    logfile: &Path,
    running: &AtomicBool,
    wallpaper_control:
        &crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
    shader_manager:
        crate::manage_shader::ShaderManager,
    shader_interval: u64,
    animation_speed_policy:
        crate::load_config::AnimationSpeedPolicy,
    global_rendered_fps: u32,
    fps_policy_entries:
        Vec<crate::load_config::FpsPolicyEntry>,
    texture_policy:
        crate::load_config::TexturePolicy,
    postprocess_policy:
        crate::load_config::PostprocessPolicy,
    audio_bands:
        Option<crate::audio_backend::SharedAudioBands>,
    subtitles: bool,
    subtitle_placement:
        crate::parse_subtitle_placement::SubtitlePlacement,
) -> Result<(), String> {

    crate::logger::information(
        logfile,
        "[LOCK] Connecting to Wayland display",
    );


    let connection =
        Connection::connect_to_env()
            .map_err(
                |error| {
                    format!(
                        "Unable to connect to the Wayland display: {}",
                        error,
                    )
                }
            )?;


    let mut event_queue =
        connection.new_event_queue();

    let qh =
        event_queue.handle();

    let display =
        connection.display();

    display.get_registry(
        &qh,
        ()
    );


    let mut state =
        ScreenLockState::new();


    event_queue
        .roundtrip(
            &mut state
        )
        .map_err(
            |error| {
                format!(
                    "Unable to enumerate Wayland globals: {}",
                    error,
                )
            }
        )?;


    let compositor =
        state
            .compositor
            .clone()
            .ok_or_else(
                || {
                    "Wayland compositor does not advertise wl_compositor"
                        .to_string()
                }
            )?;


    if state.shm.is_none() {
        return Err(
            "Wayland compositor does not advertise wl_shm"
                .to_string()
        );
    }


    let lock_manager =
        state
            .lock_manager
            .clone()
            .ok_or_else(
                || {
                    "Wayland compositor does not advertise ext_session_lock_manager_v1"
                        .to_string()
                }
            )?;


    if state.outputs.is_empty() {
        return Err(
            "Wayland compositor did not advertise any wl_output globals"
                .to_string()
        );
    }


    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] Prerequisites satisfied: {} output(s), wl_compositor, wl_shm, ext-session-lock-v1; wl_seat={}",
            state.outputs.len(),
            if state.seat.is_some() { "yes" } else { "no" },
        ),
    );


    if !wallpaper_control.request_pause_after_first_frame(
        running
    ) {
        crate::logger::error(
            logfile,
            "[LOCK] Unable to confirm wallpaper renderer pause; secure lock renderer will not be started",
        );

        return Err(
            "Wallpaper renderer did not acknowledge pause before secure session lock"
                .to_string()
        );
    }


    crate::logger::information(
        logfile,
        "[LOCK] Requesting secure Wayland session lock",
    );


    let lock =
        lock_manager.lock(
            &qh,
            ()
        );


    for output in &state.outputs {
        let surface =
            compositor.create_surface(
                &qh,
                ()
            );

        let lock_surface =
            lock.get_lock_surface(
                &surface,
                &output.output,
                &qh,
                output.global_name,
            );

        state.lock_surfaces.push(
            LockSurfaceEntry {
                output_name:
                    output.global_name,
                surface,
                lock_surface,
                configured: false,
                width: 0,
                height: 0,
                frames: Vec::new(),
                gl_context: None,
            }
        );
    }


    loop {
        if state.finished
            && !state.locked
        {
            lock.destroy();

            let _ =
                event_queue.roundtrip(
                    &mut state
                );

            return Err(
                "Compositor declined the secure session-lock request"
                    .to_string()
            );
        }


        if state.locked
            && state.all_surfaces_configured()
        {
            break;
        }


        event_queue
            .blocking_dispatch(
                &mut state
            )
            .map_err(
                |error| {
                    format!(
                        "Wayland dispatch failed while acquiring the lock: {}",
                        error,
                    )
                }
            )?;
    }


    crate::logger::information(
        logfile,
        "[LOCK] Secure Wayland lock acquired",
    );


    if state.seat.is_none() {
        crate::logger::warning(
            logfile,
            "[LOCK] Compositor did not advertise wl_seat; authentication input is unavailable while the secure lock remains active",
        );
    }


    let primary_output_name =
        state
            .lock_surfaces
            .first()
            .map(
                |entry| {
                    entry.output_name
                }
            )
            .ok_or_else(
                || {
                    "No configured Wayland lock surface is available"
                        .to_string()
                }
            )?;


    let context_result = {
        let primary_entry =
            state
                .lock_surfaces
                .first_mut()
                .expect(
                    "primary lock surface disappeared after validation"
                );

        crate::create_wayland_lock_context::WaylandLockContext::new(
            &connection,
            &primary_entry.surface,
            primary_entry.width,
            primary_entry.height,
        )
        .map(
            |context| {
                let dimensions =
                    (
                        primary_entry.width,
                        primary_entry.height,
                    );

                primary_entry.gl_context =
                    Some(context);

                dimensions
            }
        )
    };


    let (
        primary_width,
        primary_height,
    ) =
        match context_result {
            Ok(dimensions) => dimensions,

            Err(error) => {
                crate::logger::error(
                    logfile,
                    &format!(
                        "[LOCK] CRITICAL: secure lock acquired but EGL/OpenGL initialization failed: {}. Remaining locked; authentication input remains active.",
                        error,
                    ),
                );

                wait_for_authenticated_unlock_without_renderer(
                    &mut event_queue,
                    &mut state,
                    logfile,
                    running,
                )?;

                finish_authenticated_unlock(
                    &connection,
                    &mut event_queue,
                    &mut state,
                    &lock,
                    logfile,
                )?;

                resume_wallpaper_after_authenticated_unlock(
                    wallpaper_control,
                    running,
                    logfile,
                );

                return Ok(());
            }
        };


    {
        let primary_context =
            state
                .lock_surfaces
                .first()
                .and_then(
                    |entry| {
                        entry.gl_context.as_ref()
                    }
                )
                .ok_or_else(
                    || {
                        "Primary EGL/OpenGL lock context is unavailable after initialization"
                            .to_string()
                    }
                )?;

        if let Err(error) =
            primary_context.make_current()
        {
            crate::logger::error(
                logfile,
                &format!(
                    "[LOCK] CRITICAL: secure lock acquired but the EGL/OpenGL context could not be activated: {}. Remaining locked; authentication input remains active.",
                    error,
                ),
            );

            wait_for_authenticated_unlock_without_renderer(
                &mut event_queue,
                &mut state,
                logfile,
                running,
            )?;

            finish_authenticated_unlock(
                &connection,
                &mut event_queue,
                &mut state,
                &lock,
                logfile,
            )?;

            resume_wallpaper_after_authenticated_unlock(
                wallpaper_control,
                running,
                logfile,
            );

            return Ok(());
        }
    }


    let mut render_engine =
        match crate::render_frame::FrameRenderEngine::new(
            shader_manager,
            shader_interval,
            animation_speed_policy,
            global_rendered_fps,
            fps_policy_entries,
            texture_policy,
            postprocess_policy,
            audio_bands,
            subtitles,
            subtitle_placement,
            primary_width,
            primary_height,
        ) {
            Ok(engine) => engine,

            Err(error) => {
                crate::logger::error(
                    logfile,
                    &format!(
                        "[LOCK] CRITICAL: secure lock acquired but the frame renderer could not be initialized: {}. Remaining locked; authentication input remains active.",
                        error,
                    ),
                );

                wait_for_authenticated_unlock_without_renderer(
                    &mut event_queue,
                    &mut state,
                    logfile,
                    running,
                )?;

                finish_authenticated_unlock(
                    &connection,
                    &mut event_queue,
                    &mut state,
                    &lock,
                    logfile,
                )?;

                resume_wallpaper_after_authenticated_unlock(
                    wallpaper_control,
                    running,
                    logfile,
                );

                return Ok(());
            }
        };


    crate::logger::information(
        logfile,
        "[LOCK] Screenshaver frame renderer initialized on secure Wayland lock surface",
    );


    let mut authentication_panel:
        Option<crate::display_lock_authentication::LockAuthenticationPanel> =
        None;

    let mut rendering_available =
        true;

    let mut shutdown_deferred_logged =
        false;


    loop {
        if let Err(error) =
            event_queue.dispatch_pending(
                &mut state
            )
        {
            hold_locked_after_fatal_error(
                logfile,
                &format!(
                    "Wayland dispatch failed while securely locked: {}",
                    error,
                ),
            );
        }


        log_input_events(
            &mut state,
            logfile,
        );


        if state.dismiss_authentication_for_timeout() {
            crate::logger::information(
                logfile,
                "[LOCK] Authentication dialog dismissed after 10 seconds of inactivity; entered credential cleared and session remains locked",
            );
        }


        if !running.load(Ordering::SeqCst)
            && !shutdown_deferred_logged
        {
            crate::logger::information(
                logfile,
                "[LOCK] Shutdown requested while securely locked; deferring process termination until successful authentication",
            );

            shutdown_deferred_logged =
                true;
        }


        if state.interaction_state
            == LockInteractionState::AuthenticationSucceeded
        {
            crate::logger::information(
                logfile,
                "[LOCK] PAM authentication succeeded; authenticated unlock requested",
            );

            break;
        }


        if state.interaction_state
            == LockInteractionState::Rendering
            && authentication_panel.is_some()
        {
            authentication_panel =
                None;

        }


        if rendering_available
            && state.interaction_state
                == LockInteractionState::AuthenticationRequested
            && authentication_panel.is_none()
        {
            let overlay_result =
                {
                    let primary_context =
                        state
                            .lock_surfaces
                            .first()
                            .and_then(
                                |entry| {
                                    entry.gl_context.as_ref()
                                }
                            )
                            .ok_or_else(
                                || {
                                    "Primary EGL/OpenGL lock context disappeared before authentication dialog creation"
                                        .to_string()
                                }
                            )?;

                    primary_context
                        .make_current()
                        .and_then(
                            |_| {
                                crate::display_lock_authentication::LockAuthenticationPanel::new(
                                    &state.authentication,
                                    primary_width,
                                    primary_height,
                                )
                            }
                        )
                };


            match overlay_result {
                Ok(overlay) => {
                    authentication_panel =
                        Some(
                            overlay
                        );

                }

                Err(error) => {
                    crate::logger::warning(
                        logfile,
                        &format!(
                            "[LOCK] Unable to create centered authentication panel: {}",
                            error,
                        ),
                    );
                }
            }
        }


        if rendering_available {
            let render_result =
                {
                    let primary_context =
                        state
                            .lock_surfaces
                            .first()
                            .and_then(
                                |entry| {
                                    entry.gl_context.as_ref()
                                }
                            )
                            .ok_or_else(
                                || {
                                    "Primary EGL/OpenGL lock context disappeared during rendering"
                                        .to_string()
                                }
                            )?;

                    primary_context
                        .make_current()
                        .and_then(
                            |_| {
                                let _ =
                                    render_engine.render_frame(
                                        primary_width,
                                        primary_height,
                                    );

                                if let Some(panel) =
                                    authentication_panel.as_ref()
                                {
                                    panel.display(
                                        &state.authentication,
                                        primary_width,
                                        primary_height,
                                    );
                                }

                                primary_context.swap_buffers()
                            }
                        )
                };


            if let Err(error) =
                render_result
            {
                crate::logger::error(
                    logfile,
                    &format!(
                        "[LOCK] CRITICAL: secure lock rendering failed on output {}: {}. Remaining locked; authentication input remains active.",
                        primary_output_name,
                        error,
                    ),
                );

                rendering_available =
                    false;

                authentication_panel =
                    None;
            }


            if rendering_available {
                render_engine.limit_fps();
            }
        } else {
            std::thread::sleep(
                Duration::from_millis(
                    16
                )
            );
        }


        if let Err(error) =
            connection.flush()
        {
            hold_locked_after_fatal_error(
                logfile,
                &format!(
                    "Unable to flush Wayland requests while securely locked: {}",
                    error,
                ),
            );
        }
    }


    if let Some(primary_context) =
        state
            .lock_surfaces
            .first()
            .and_then(
                |entry| {
                    entry.gl_context.as_ref()
                }
            )
    {
        let _ =
            primary_context.make_current();
    }


    drop(
        authentication_panel
    );

    drop(
        render_engine
    );


    finish_authenticated_unlock(
        &connection,
        &mut event_queue,
        &mut state,
        &lock,
        logfile,
    )?;


    resume_wallpaper_after_authenticated_unlock(
        wallpaper_control,
        running,
        logfile,
    );


    Ok(())
}


fn log_input_events(
    state: &mut ScreenLockState,
    logfile: &Path,
) {
    for input_event in
        state.input_events.drain(..)
    {
        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] INPUT: {}",
                input_event,
            ),
        );
    }
}


fn wait_for_authenticated_unlock_without_renderer(
    event_queue:
        &mut wayland_client::EventQueue<ScreenLockState>,
    state: &mut ScreenLockState,
    logfile: &Path,
    running: &AtomicBool,
) -> Result<(), String> {

    crate::logger::warning(
        logfile,
        "[LOCK] Secure renderer unavailable; fail-closed authentication-only mode active",
    );


    let mut shutdown_deferred_logged =
        false;


    loop {
        if !running.load(Ordering::SeqCst)
            && !shutdown_deferred_logged
        {
            crate::logger::information(
                logfile,
                "[LOCK] Shutdown requested while securely locked in authentication-only mode; deferring process termination until successful authentication",
            );

            shutdown_deferred_logged =
                true;
        }


        if let Err(error) =
            event_queue.blocking_dispatch(
                state
            )
        {
            hold_locked_after_fatal_error(
                logfile,
                &format!(
                    "Wayland dispatch failed in authentication-only lock mode: {}",
                    error,
                ),
            );
        }


        log_input_events(
            state,
            logfile,
        );


        if state.interaction_state
            == LockInteractionState::AuthenticationSucceeded
        {
            return Ok(());
        }
    }
}


fn resume_wallpaper_after_authenticated_unlock(
    wallpaper_control:
        &crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
    running: &AtomicBool,
    logfile: &Path,
) {
    if running.load(Ordering::SeqCst) {
        wallpaper_control.resume_and_wait_for_frame(
            running
        );
    } else {
        crate::logger::information(
            logfile,
            "[LOCK] Authenticated unlock completed with shutdown pending; wallpaper renderer will remain stopped",
        );
    }
}


fn hold_locked_after_fatal_error(
    logfile: &Path,
    message: &str,
) -> ! {

    crate::logger::error(
        logfile,
        &format!(
            "[LOCK] CRITICAL: {}. Screenshaver will remain fail-closed and will not request an unlock.",
            message,
        ),
    );


    loop {
        std::thread::sleep(
            Duration::from_secs(
                60
            )
        );
    }
}


fn finish_authenticated_unlock(
    connection: &Connection,
    event_queue:
        &mut wayland_client::EventQueue<ScreenLockState>,
    state: &mut ScreenLockState,
    lock: &ExtSessionLockV1,
    logfile: &Path,
) -> Result<(), String> {

    if state.interaction_state
        != LockInteractionState::AuthenticationSucceeded
    {
        return Err(
            "Internal security invariant violated: unlock requested without successful authentication"
                .to_string()
        );
    }


    crate::logger::information(
        logfile,
        "[LOCK] Requesting authenticated controlled unlock",
    );


    lock.unlock_and_destroy();


    event_queue
        .roundtrip(
            state
        )
        .map_err(
            |error| {
                format!(
                    "Unable to synchronize authenticated unlock with the compositor: {}",
                    error,
                )
            }
        )?;


    for entry in
        &mut state.lock_surfaces
    {
        entry.gl_context =
            None;

        for frame in
            entry.frames.drain(..)
        {
            frame.buffer.destroy();
            frame.pool.destroy();

            drop(
                frame.backing_file
            );
        }

        entry.lock_surface.destroy();
    }


    let _ =
        connection.flush();


    crate::logger::information(
        logfile,
        "[LOCK] Session unlocked successfully",
    );


    Ok(())
}


impl Dispatch<
    wl_registry::WlRegistry,
    ()
> for ScreenLockState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _connection: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                match interface.as_str() {
                    "wl_compositor" => {
                        if state.compositor.is_none() {
                            state.compositor =
                                Some(
                                    registry.bind::<
                                        wl_compositor::WlCompositor,
                                        _,
                                        _
                                    >(
                                        name,
                                        version.min(4),
                                        qh,
                                        (),
                                    )
                                );
                        }
                    }


                    "wl_shm" => {
                        if state.shm.is_none() {
                            state.shm =
                                Some(
                                    registry.bind::<
                                        wl_shm::WlShm,
                                        _,
                                        _
                                    >(
                                        name,
                                        1,
                                        qh,
                                        (),
                                    )
                                );
                        }
                    }


                    "wl_seat" => {
                        if state.seat.is_none() {
                            state.seat =
                                Some(
                                    registry.bind::<
                                        wl_seat::WlSeat,
                                        _,
                                        _
                                    >(
                                        name,
                                        version.min(9),
                                        qh,
                                        (),
                                    )
                                );
                        }
                    }


                    "wl_output" => {
                        let output =
                            registry.bind::<
                                wl_output::WlOutput,
                                _,
                                _
                            >(
                                name,
                                version.min(4),
                                qh,
                                (),
                            );

                        state.outputs.push(
                            OutputEntry {
                                global_name: name,
                                output,
                            }
                        );
                    }


                    "ext_session_lock_manager_v1" => {
                        if state.lock_manager.is_none() {
                            state.lock_manager =
                                Some(
                                    registry.bind::<
                                        ExtSessionLockManagerV1,
                                        _,
                                        _
                                    >(
                                        name,
                                        1,
                                        qh,
                                        (),
                                    )
                                );
                        }
                    }


                    _ => {}
                }
            }


            wl_registry::Event::GlobalRemove {
                name,
            } => {
                state.outputs.retain(
                    |entry| {
                        entry.global_name != name
                    }
                );
            }


            _ => {}
        }
    }
}


impl Dispatch<
    wl_seat::WlSeat,
    ()
> for ScreenLockState {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _connection: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let wl_seat::Event::Capabilities {
            capabilities,
        } = event
        else {
            return;
        };


        let WEnum::Value(
            capabilities
        ) = capabilities
        else {
            return;
        };


        if capabilities.contains(
            wl_seat::Capability::Keyboard
        ) {
            if state.keyboard.is_none() {
                state.keyboard =
                    Some(
                        seat.get_keyboard(
                            qh,
                            (),
                        )
                    );
            }
        } else if let Some(keyboard) =
            state.keyboard.take()
        {
            keyboard.release();
        }


        if capabilities.contains(
            wl_seat::Capability::Pointer
        ) {
            if state.pointer.is_none() {
                state.pointer =
                    Some(
                        seat.get_pointer(
                            qh,
                            (),
                        )
                    );
            }
        } else if let Some(pointer) =
            state.pointer.take()
        {
            pointer.release();
        }
    }
}


impl Dispatch<
    wl_keyboard::WlKeyboard,
    ()
> for ScreenLockState {
    fn event(
        state: &mut Self,
        _keyboard: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Keymap {
                format,
                fd,
                size,
            } => {
                let WEnum::Value(
                    wl_keyboard::KeymapFormat::XkbV1
                ) = format
                else {
                    state.input_events.push(
                        "compositor supplied an unsupported keyboard keymap format"
                            .to_string()
                    );

                    return;
                };


                let mut file =
                    File::from(
                        fd
                    );

                // The keymap fd is seekable shared memory.  Do not assume
                // that the received descriptor's current offset is zero.
                // Rewind it before reading the compositor-advertised keymap
                // size; otherwise read_exact() can encounter an immediate or
                // premature EOF and leave XKB permanently uninitialized.
                if let Err(error) =
                    file.seek(
                        SeekFrom::Start(
                            0
                        )
                    )
                {
                    state.input_events.push(
                        format!(
                            "unable to rewind compositor XKB keymap: {}",
                            error,
                        )
                    );

                    return;
                }


                let mut bytes =
                    vec![
                        0_u8;
                        size as usize
                    ];


                if let Err(error) =
                    file.read_exact(
                        &mut bytes
                    )
                {
                    state.input_events.push(
                        format!(
                            "unable to read compositor XKB keymap after rewind: {}",
                            error,
                        )
                    );

                    return;
                }


                while bytes.last()
                    == Some(
                        &0
                    )
                {
                    bytes.pop();
                }


                let keymap_text =
                    match String::from_utf8(
                        bytes
                    ) {
                        Ok(text) => {
                            text
                        }

                        Err(error) => {
                            state.input_events.push(
                                format!(
                                    "compositor XKB keymap is not valid UTF-8: {}",
                                    error,
                                )
                            );

                            return;
                        }
                    };


                let context =
                    xkbcommon::xkb::Context::new(
                        xkbcommon::xkb::CONTEXT_NO_FLAGS
                    );


                let Some(keymap) =
                    xkbcommon::xkb::Keymap::new_from_string(
                        &context,
                        keymap_text,
                        xkbcommon::xkb::KEYMAP_FORMAT_TEXT_V1,
                        xkbcommon::xkb::KEYMAP_COMPILE_NO_FLAGS,
                    )
                else {
                    state.input_events.push(
                        "libxkbcommon rejected the compositor keyboard keymap"
                            .to_string()
                    );

                    return;
                };


                state.xkb_state =
                    Some(
                        xkbcommon::xkb::State::new(
                            &keymap
                        )
                    );

                state.input_events.push(
                    "compositor XKB keymap loaded for authentication input"
                        .to_string()
                );
            }


            wl_keyboard::Event::Enter {
                ..
            } => {
                state.input_events.push(
                    "keyboard focus entered secure lock surface"
                        .to_string()
                );
            }


            wl_keyboard::Event::Leave {
                ..
            } => {
                state.input_events.push(
                    "keyboard focus left secure lock surface"
                        .to_string()
                );
            }


            wl_keyboard::Event::Key {
                key,
                state: key_state,
                ..
            } => {
                let pressed =
                    matches!(
                        key_state,
                        WEnum::Value(
                            wl_keyboard::KeyState::Pressed
                        )
                    );


                if !pressed {
                    if state.interaction_state
                        == LockInteractionState::AuthenticationRequested
                    {
                        state.authentication.handle_key_release();
                    }

                    return;
                }


                if state.interaction_state
                    == LockInteractionState::Rendering
                {
                    // The wake key requests the dialog but is deliberately not
                    // inserted into the password field.
                    state.begin_authentication();

                    state.input_events.push(
                        "keyboard activity requested authentication dialog"
                            .to_string()
                    );

                    return;
                }


                let Some(xkb_state) =
                    state.xkb_state.as_ref()
                else {
                    state.input_events.push(
                        "keyboard press received before XKB keymap initialization"
                            .to_string()
                    );

                    return;
                };


                // Wayland key events use evdev keycodes. XKB keycodes are
                // conventionally evdev + 8.
                let xkb_keycode =
                    xkbcommon::xkb::Keycode::new(
                        key.saturating_add(
                            8
                        )
                    );

                let keysym =
                    xkb_state
                        .key_get_one_sym(
                            xkb_keycode
                        );

                let utf8 =
                    xkb_state
                        .key_get_utf8(
                            xkb_keycode
                        );


                let authentication_revision_before =
                    state.authentication.revision();

                let authentication_action =
                    state.authentication.handle_key(
                        keysym.raw(),
                        &utf8,
                    );

                let authentication_revision_changed =
                    state.authentication.revision()
                        != authentication_revision_before;

                if authentication_revision_changed
                    || keysym.raw()
                        == xkbcommon::xkb::keysyms::KEY_BackSpace
                {
                    state.note_authentication_activity();
                }


                match authentication_action {
                    crate::display_lock_authentication::AuthenticationAction::None => {}


                    crate::display_lock_authentication::AuthenticationAction::Dismiss => {
                        state.interaction_state =
                            LockInteractionState::Rendering;

                        state.authentication_deadline =
                            None;

                        state.pointer_anchor =
                            None;

                        state.input_events.push(
                            "authentication dialog dismissed with Escape; session remains locked"
                                .to_string()
                        );
                    }


                    crate::display_lock_authentication::AuthenticationAction::Submit => {
                        let username =
                            state.authentication
                                .username()
                                .to_string();

                        let password =
                            state.authentication
                                .take_password();


                        let result =
                            crate::authenticate_user::authenticate(
                                &username,
                                password.as_str(),
                            );


                        // The submitted credential is Zeroizing<String>.
                        // Dropping it explicitly wipes Screenshaver's
                        // submitted Rust-owned buffer after PAM returns.
                        drop(
                            password
                        );


                        match result {
                            crate::authenticate_user::AuthenticationResult::Success => {
                                state.authentication.set_status(
                                    "Authentication successful"
                                );

                                state.interaction_state =
                                    LockInteractionState::AuthenticationSucceeded;

                                state.authentication_deadline =
                                    None;

                                state.input_events.push(
                                    "PAM authentication succeeded; authenticated controlled unlock authorized"
                                        .to_string()
                                );
                            }


                            crate::authenticate_user::AuthenticationResult::Rejected => {
                                state.authentication.set_status(
                                    "Authentication failed"
                                );

                                state.authentication.authentication_failed();

                                state.note_authentication_activity();

                                state.input_events.push(
                                    "PAM authentication rejected credentials; session remains locked"
                                        .to_string()
                                );
                            }


                            crate::authenticate_user::AuthenticationResult::Error(error) => {
                                state.authentication.set_status(
                                    "Authentication service error"
                                );

                                state.authentication.authentication_failed();

                                state.note_authentication_activity();

                                state.input_events.push(
                                    format!(
                                        "PAM authentication service error: {}; session remains locked",
                                        error,
                                    )
                                );
                            }
                        }
                    }
                }
            }


            wl_keyboard::Event::Modifiers {
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                ..
            } => {
                if let Some(xkb_state) =
                    state.xkb_state.as_mut()
                {
                    xkb_state.update_mask(
                        mods_depressed,
                        mods_latched,
                        mods_locked,
                        0,
                        0,
                        group,
                    );
                }
            }


            _ => {}
        }
    }
}


impl Dispatch<
    wl_pointer::WlPointer,
    ()
> for ScreenLockState {
    fn event(
        state: &mut Self,
        _pointer: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                surface_x,
                surface_y,
                ..
            } => {
                state.pointer_anchor =
                    Some(
                        (
                            surface_x,
                            surface_y,
                        )
                    );

                state.input_events.push(
                    format!(
                        "pointer entered secure lock surface at ({:.1}, {:.1})",
                        surface_x,
                        surface_y,
                    )
                );
            }


            wl_pointer::Event::Leave {
                ..
            } => {
                state.pointer_anchor =
                    None;

                state.input_events.push(
                    "pointer left secure lock surface"
                        .to_string()
                );
            }


            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                if state.interaction_state
                    == LockInteractionState::Rendering
                {
                    match state.pointer_anchor {
                        Some(
                            (
                                anchor_x,
                                anchor_y,
                            )
                        ) => {
                            let delta_x =
                                surface_x
                                    - anchor_x;

                            let delta_y =
                                surface_y
                                    - anchor_y;

                            let distance_squared =
                                delta_x
                                    * delta_x
                                    + delta_y
                                        * delta_y;

                            if distance_squared
                                >= AUTH_POINTER_MOVEMENT_THRESHOLD
                                    * AUTH_POINTER_MOVEMENT_THRESHOLD
                            {
                                state.begin_authentication();

                                state.pointer_anchor =
                                    Some(
                                        (
                                            surface_x,
                                            surface_y,
                                        )
                                    );

                                state.input_events.push(
                                    format!(
                                        "meaningful pointer movement detected ({:.1}px threshold exceeded)",
                                        AUTH_POINTER_MOVEMENT_THRESHOLD,
                                    )
                                );
                            }
                        }

                        None => {
                            state.pointer_anchor =
                                Some(
                                    (
                                        surface_x,
                                        surface_y,
                                    )
                                );
                        }
                    }
                } else if state.interaction_state
                    == LockInteractionState::AuthenticationRequested
                {
                    match state.pointer_anchor {
                        Some((anchor_x, anchor_y)) => {
                            let delta_x =
                                surface_x - anchor_x;

                            let delta_y =
                                surface_y - anchor_y;

                            let distance_squared =
                                delta_x * delta_x
                                    + delta_y * delta_y;

                            if distance_squared
                                >= AUTH_POINTER_MOVEMENT_THRESHOLD
                                    * AUTH_POINTER_MOVEMENT_THRESHOLD
                            {
                                state.pointer_anchor =
                                    Some(
                                        (
                                            surface_x,
                                            surface_y,
                                        )
                                    );

                                state.note_authentication_activity();
                            }
                        }

                        None => {
                            state.pointer_anchor =
                                Some(
                                    (
                                        surface_x,
                                        surface_y,
                                    )
                                );
                        }
                    }
                }
            }


            wl_pointer::Event::Button {
                button,
                state: button_state,
                ..
            } => {
                state.input_events.push(
                    format!(
                        "pointer button={} state={:?}",
                        button,
                        button_state,
                    )
                );


                if matches!(
                    button_state,
                    WEnum::Value(
                        wl_pointer::ButtonState::Pressed
                    )
                ) {
                    if state.interaction_state
                        == LockInteractionState::Rendering
                    {
                        state.begin_authentication();
                    } else if state.interaction_state
                        == LockInteractionState::AuthenticationRequested
                    {
                        state.note_authentication_activity();
                    }
                }
            }


            wl_pointer::Event::Axis {
                axis,
                value,
                ..
            } => {
                state.input_events.push(
                    format!(
                        "pointer axis={:?} value={:.3}",
                        axis,
                        value,
                    )
                );


                if state.interaction_state
                    == LockInteractionState::AuthenticationRequested
                    && value.abs() > f64::EPSILON
                {
                    state.note_authentication_activity();
                }
            }


            _ => {}
        }
    }
}


impl Dispatch<
    ExtSessionLockV1,
    ()
> for ScreenLockState {
    fn event(
        state: &mut Self,
        _proxy: &ExtSessionLockV1,
        event: ext_session_lock_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_session_lock_v1::Event::Locked => {
                state.locked =
                    true;
            }


            ext_session_lock_v1::Event::Finished => {
                state.finished =
                    true;
            }


            _ => {}
        }
    }
}


impl Dispatch<
    ExtSessionLockSurfaceV1,
    u32
> for ScreenLockState {
    fn event(
        state: &mut Self,
        proxy: &ExtSessionLockSurfaceV1,
        event: ext_session_lock_surface_v1::Event,
        output_name: &u32,
        _connection: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let ext_session_lock_surface_v1::Event::Configure {
            serial,
            width,
            height,
        } = event
        else {
            return;
        };


        let Some(shm) =
            state.shm.clone()
        else {
            return;
        };


        let Some(entry) =
            state
                .lock_surfaces
                .iter_mut()
                .find(
                    |entry| {
                        entry.output_name
                            == *output_name
                    }
                )
        else {
            return;
        };


        if width == 0
            || height == 0
        {
            return;
        }


        match create_lock_buffer(
            &shm,
            qh,
            width,
            height,
        ) {
            Ok(
                (
                    file,
                    pool,
                    buffer,
                )
            ) => {
                proxy.ack_configure(
                    serial
                );

                entry.surface.attach(
                    Some(
                        &buffer
                    ),
                    0,
                    0,
                );

                entry.surface.damage(
                    0,
                    0,
                    width as i32,
                    height as i32,
                );

                entry.surface.commit();


                entry.width =
                    width;

                entry.height =
                    height;

                entry.frames.push(
                    LockFrame {
                        backing_file: file,
                        pool,
                        buffer,
                    }
                );

                entry.configured =
                    true;
            }


            Err(error) => {
                eprintln!(
                    "[LOCK] Unable to create lock-surface buffer for output {}: {}",
                    output_name,
                    error,
                );
            }
        }
    }
}


fn create_lock_buffer(
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<ScreenLockState>,
    width: u32,
    height: u32,
) -> Result<
    (
        File,
        wl_shm_pool::WlShmPool,
        wl_buffer::WlBuffer,
    ),
    String,
> {
    let stride =
        width
            .checked_mul(4)
            .ok_or_else(
                || {
                    "Lock-surface stride overflow"
                        .to_string()
                }
            )?;


    let size =
        stride
            .checked_mul(
                height
            )
            .ok_or_else(
                || {
                    "Lock-surface buffer-size overflow"
                        .to_string()
                }
            )?;


    if size
        > i32::MAX as u32
    {
        return Err(
            "Lock-surface buffer exceeds Wayland SHM size limit"
                .to_string()
        );
    }


    let mut file =
        create_memfd()?;


    file.set_len(
        size as u64
    )
    .map_err(
        |error| {
            format!(
                "Unable to size lock-surface SHM buffer: {}",
                error,
            )
        }
    )?;


    file.seek(
        SeekFrom::Start(0)
    )
    .map_err(
        |error| {
            format!(
                "Unable to seek lock-surface SHM buffer: {}",
                error,
            )
        }
    )?;


    // XRGB8888 on little-endian Linux is stored in memory as B, G, R, X.
    // Generate a static two-dimensional fallback gradient. This buffer provides a compositor-visible fallback surface before EGL rendering begins.
    let mut row =
        Vec::with_capacity(
            stride as usize
        );


    for y in 0..height {
        row.clear();

        let vertical =
            (
                (
                    y as u64 * 255
                )
                    / height.max(1) as u64
            ) as u8;

        for x in 0..width {
            let horizontal =
                (
                    (
                        x as u64 * 255
                    )
                        / width.max(1) as u64
                ) as u8;

            let red =
                horizontal;

            let green =
                vertical;

            let blue =
                horizontal
                    .wrapping_add(
                        vertical
                    );

            row.extend_from_slice(
                &[
                    blue,
                    green,
                    red,
                    0xff,
                ]
            );
        }


        file.write_all(
            &row
        )
        .map_err(
            |error| {
                format!(
                    "Unable to write lock-surface SHM buffer: {}",
                    error,
                )
            }
        )?;
    }


    file.flush()
        .map_err(
            |error| {
                format!(
                    "Unable to flush lock-surface SHM buffer: {}",
                    error,
                )
            }
        )?;


    let pool =
        shm.create_pool(
            file.as_fd(),
            size as i32,
            qh,
            (),
        );


    let buffer =
        pool.create_buffer(
            0,
            width as i32,
            height as i32,
            stride as i32,
            wl_shm::Format::Xrgb8888,
            qh,
            (),
        );


    Ok(
        (
            file,
            pool,
            buffer,
        )
    )
}


fn create_memfd() -> Result<File, String> {
    let name =
        CString::new(
            "screenshaver-lock"
        )
        .map_err(
            |error| {
                format!(
                    "Unable to create SHM name: {}",
                    error,
                )
            }
        )?;


    let fd =
        unsafe {
            libc::memfd_create(
                name.as_ptr(),
                libc::MFD_CLOEXEC,
            )
        };


    if fd < 0 {
        return Err(
            format!(
                "memfd_create failed: {}",
                std::io::Error::last_os_error(),
            )
        );
    }


    let file =
        unsafe {
            File::from_raw_fd(
                fd
            )
        };


    Ok(file)
}


delegate_noop!(
    ScreenLockState:
        ignore wl_compositor::WlCompositor
);

delegate_noop!(
    ScreenLockState:
        ignore wl_output::WlOutput
);

delegate_noop!(
    ScreenLockState:
        ignore wl_shm::WlShm
);

delegate_noop!(
    ScreenLockState:
        ignore wl_shm_pool::WlShmPool
);

delegate_noop!(
    ScreenLockState:
        ignore wl_buffer::WlBuffer
);

delegate_noop!(
    ScreenLockState:
        ignore wl_surface::WlSurface
);

delegate_noop!(
    ScreenLockState:
        ignore ExtSessionLockManagerV1
);
