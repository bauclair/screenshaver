use std::collections::HashSet;
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
use std::time::{Duration, Instant};

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


const TEST_SECONDS: u64 = 30;
const AUTH_POINTER_MOVEMENT_THRESHOLD: f64 = 24.0;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockInteractionState {
    Rendering,
    AuthenticationRequested,
}


struct OutputEntry {
    global_name: u32,
    output: wl_output::WlOutput,
}


struct DiagnosticFrame {
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
    frames: Vec<DiagnosticFrame>,
    gl_context: Option<crate::create_wayland_lock_context::WaylandLockContext>,
}


struct TestState {
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
    xkb_state: Option<xkbcommon::xkb::State>,
    authentication:
        crate::display_lock_authentication::LockAuthentication,
}


impl TestState {
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
}


pub fn run(
    logfile: &Path,
) -> Result<(), String> {
    println!(
        "[LOCK TEST] Connecting to Wayland display..."
    );

    crate::logger::information(
        logfile,
        "[LOCK TEST] Connecting to Wayland display",
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
        TestState::new();


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
            "[LOCK TEST] Prerequisites satisfied: {} output(s), wl_compositor, wl_shm, ext-session-lock-v1; wl_seat={}",
            state.outputs.len(),
            if state.seat.is_some() { "yes" } else { "no" },
        ),
    );


    // Load all policy/configuration prerequisites before requesting the secure
    // lock.  OpenGL resources cannot be created until the compositor gives us
    // a configured lock surface, but database/configuration failures should
    // still occur while the desktop is fully accessible.
    let database_connection =
        crate::evaluate_database::evaluate()
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare Screenshaver database for lock-render test: {}",
                        error,
                    )
                }
            )?;


    drop(
        database_connection
    );


    let config_path =
        crate::locate_paths::config_path();


    let config_result =
        crate::load_config::load_config(
            &config_path
        )
        .map_err(
            |error| {
                format!(
                    "Unable to load Screenshaver configuration for lock-render test: {}",
                    error,
                )
            }
        )?;


    let cfg =
        config_result.config;


    // Audio Bloom is the purpose of this checkpoint, so audio capture is a
    // required pre-lock prerequisite rather than an optional runtime feature.
    // If capture cannot be initialized, abort while the desktop is still
    // fully accessible.
    let audio_backend =
        crate::audio_backend::create_backend()
            .map_err(
                |error| {
                    format!(
                        "Unable to initialize the audio backend for the secure-lock Audio Bloom test: {}",
                        error,
                    )
                }
            )?;


    let audio_bands =
        audio_backend.shared_bands();


    crate::logger::information(
        logfile,
        &format!(
            "[LOCK TEST] Audio backend ready for secure-lock Audio Bloom test: {}",
            audio_backend.backend_name(),
        ),
    );


    let audio_bloom_policy_ids =
        load_audio_bloom_screensaver_policy_ids()?;


    if audio_bloom_policy_ids.is_empty() {
        return Err(
            "No Screensaver-target policies with bloom_mode='audio' are available for the secure-lock Audio Bloom test"
                .to_string()
        );
    }


    let mut shader_entries =
        crate::manage_shader::ShaderManager::load_shader_entries();


    shader_entries.retain(
        |entry| {
            audio_bloom_policy_ids.contains(
                &entry.policy_id
            )
        }
    );


    let selected_entry =
        shader_entries
            .first()
            .cloned()
            .ok_or_else(
                || {
                    "Audio Bloom policies exist in the database, but none are currently eligible for Screensaver rendering"
                        .to_string()
                }
            )?;


    println!(
        "[LOCK TEST] Audio Bloom rotation will begin with policy '{}' (policy_id={}, shader={})",
        selected_entry.policy_name,
        selected_entry.policy_id,
        selected_entry.name,
    );


    crate::logger::information(
        logfile,
        &format!(
            "[LOCK TEST] Secure-lock Audio Bloom rotation: {} eligible policy/policies; first='{}' (policy_id={}, shader={}); interval=5s",
            shader_entries.len(),
            selected_entry.policy_name,
            selected_entry.policy_id,
            selected_entry.name,
        ),
    );


    let shader_manager =
        crate::manage_shader::ShaderManager::from_shader_entries(
            crate::manage_shader::ShaderMode::Ordered,
            shader_entries,
        );


    println!(
        "[LOCK TEST] Requesting secure session lock..."
    );

    crate::logger::information(
        logfile,
        "[LOCK TEST] Requesting secure Wayland session lock",
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


    println!(
        "[LOCK TEST] Secure Wayland lock acquired."
    );

    crate::logger::information(
        logfile,
        "[LOCK TEST] Secure Wayland lock acquired",
    );


    if state.seat.is_some() {
        println!(
            "[LOCK TEST] Wayland input test armed: keyboard/mouse activity will be logged but will NOT unlock the session."
        );

        crate::logger::information(
            logfile,
            "[LOCK TEST] Wayland input test armed; input cannot trigger unlock",
        );
    } else {
        println!(
            "[LOCK TEST] WARNING: compositor did not advertise wl_seat; no keyboard/mouse input can be observed in this test."
        );

        crate::logger::warning(
            logfile,
            "[LOCK TEST] Compositor did not advertise wl_seat; secure-lock input detection unavailable",
        );
    }


    println!(
        "[LOCK TEST] Creating EGL/OpenGL context on primary secure lock surface..."
    );


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


    let (
        primary_width,
        primary_height,
    ) = {
        let primary_entry =
            state
                .lock_surfaces
                .first_mut()
                .expect(
                    "primary lock surface disappeared after validation"
                );


        match crate::create_wayland_lock_context::WaylandLockContext::new(
            &connection,
            &primary_entry.surface,
            primary_entry.width,
            primary_entry.height,
        ) {
            Ok(context) => {
                primary_entry.gl_context =
                    Some(context);

                (
                    primary_entry.width,
                    primary_entry.height,
                )
            }

            Err(error) => {
                let error =
                    format!(
                        "Unable to initialize EGL/OpenGL on primary output {}: {}",
                        primary_entry.output_name,
                        error,
                    );

                eprintln!(
                    "[LOCK TEST] {}",
                    error
                );

                crate::logger::error(
                    logfile,
                    &format!(
                        "[LOCK TEST] {}",
                        error,
                    ),
                );

                println!(
                    "[LOCK TEST] EGL setup failed; requesting immediate controlled unlock..."
                );

                lock.unlock_and_destroy();

                let _ =
                    event_queue.roundtrip(
                        &mut state
                    );

                return Err(error);
            }
        }
    };


    crate::logger::information(
        logfile,
        &format!(
            "[LOCK TEST] EGL/OpenGL lock-surface context initialized on output {} ({}x{})",
            primary_output_name,
            primary_width,
            primary_height,
        ),
    );


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


        primary_context
            .make_current()
            .map_err(
                |error| {
                    format!(
                        "Unable to activate primary lock OpenGL context before renderer initialization: {}",
                        error,
                    )
                }
            )?;
    }


    println!(
        "[LOCK TEST] Initializing Screenshaver frame render engine..."
    );


    let mut render_engine =
        match crate::render_frame::FrameRenderEngine::new(
            shader_manager,

            // Exercise the normal renderer-owned shader scheduler while the
            // compositor-enforced lock remains continuously active.
            5,

            cfg.screensaver_speed_policy.clone(),
            cfg.global_rendered_fps,
            cfg.screensaver_fps_policy_entries.clone(),
            cfg.texture_policy.clone(),
            cfg.screensaver_postprocess_policy.clone(),

            Some(
                audio_bands
            ),

            cfg.subtitles,
            cfg.subtitle_placement,
            primary_width,
            primary_height,
        ) {
            Ok(engine) => {
                engine
            }

            Err(error) => {
                let error =
                    format!(
                        "Unable to initialize Screenshaver frame render engine on secure lock surface: {}",
                        error,
                    );

                eprintln!(
                    "[LOCK TEST] {}",
                    error
                );

                crate::logger::error(
                    logfile,
                    &format!(
                        "[LOCK TEST] {}",
                        error,
                    ),
                );

                println!(
                    "[LOCK TEST] Renderer setup failed; requesting immediate controlled unlock..."
                );

                lock.unlock_and_destroy();

                let _ =
                    event_queue.roundtrip(
                        &mut state
                    );

                return Err(error);
            }
        };


    crate::logger::information(
        logfile,
        "[LOCK TEST] Real Screenshaver frame renderer initialized on secure Wayland lock surface",
    );


    let mut authentication_panel:
        Option<crate::display_lock_authentication::LockAuthenticationPanel> =
        None;

    let mut authentication_panel_revision =
        0_u64;


    let render_started =
        Instant::now();

    let test_duration =
        Duration::from_secs(
            TEST_SECONDS
        );

    let mut last_reported_remaining =
        TEST_SECONDS + 1;


    while render_started.elapsed()
        < test_duration
    {
        let elapsed_seconds =
            render_started
                .elapsed()
                .as_secs();

        let remaining =
            TEST_SECONDS
                .saturating_sub(
                    elapsed_seconds
                )
                .max(1);


        if remaining
            != last_reported_remaining
        {
            println!(
                "[LOCK TEST] Automatic unlock in {} second{}...",
                remaining,
                if remaining == 1 {
                    ""
                } else {
                    "s"
                },
            );

            last_reported_remaining =
                remaining;
        }


        // Process input and other Wayland events without blocking the renderer.
        // Input events are diagnostic only: they never request or authorize
        // an unlock.
        event_queue
            .dispatch_pending(
                &mut state
            )
            .map_err(
                |error| {
                    format!(
                        "Wayland dispatch failed during secure-lock input test: {}",
                        error,
                    )
                }
            )?;


        for input_event in state.input_events.drain(..) {
            println!(
                "[LOCK TEST] INPUT: {} (session remains locked)",
                input_event,
            );

            crate::logger::information(
                logfile,
                &format!(
                    "[LOCK TEST] INPUT: {} (session remains locked)",
                    input_event,
                ),
            );
        }


        if state.interaction_state
            == LockInteractionState::Rendering
            && authentication_panel.is_some()
        {
            authentication_panel =
                None;

            authentication_panel_revision =
                0;
        }


        if state.interaction_state
            == LockInteractionState::AuthenticationRequested
            && (
                authentication_panel.is_none()
                    || authentication_panel_revision
                        != state.authentication.revision()
            )
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

                    authentication_panel_revision =
                        state.authentication.revision();

                    crate::logger::information(
                        logfile,
                        "[LOCK TEST] Authentication dialog shell refreshed; session remains securely locked",
                    );
                }

                Err(error) => {
                    crate::logger::warning(
                        logfile,
                        &format!(
                            "[LOCK TEST] Unable to create centered authentication panel: {}",
                            error,
                        ),
                    );
                }
            }
        }


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
            let error =
                format!(
                    "Screenshaver lock-surface rendering failed on output {}: {}",
                    primary_output_name,
                    error,
                );

            eprintln!(
                "[LOCK TEST] {}",
                error
            );

            crate::logger::error(
                logfile,
                &format!(
                    "[LOCK TEST] {}",
                    error,
                ),
            );

            println!(
                "[LOCK TEST] Rendering failed; requesting immediate controlled unlock..."
            );


            // FrameRenderEngine owns GL resources, so destroy it while the
            // primary lock context still exists.
            drop(
                render_engine
            );


            lock.unlock_and_destroy();

            let _ =
                event_queue.roundtrip(
                    &mut state
                );

            return Err(error);
        }


        render_engine.limit_fps();
    }


    // FrameRenderEngine owns shader programs, textures, VAOs, post-processing
    // targets and overlays.  Drop it before tearing down the EGL context.
    {
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
    }


    drop(
        authentication_panel
    );


    drop(
        render_engine
    );


    println!(
        "[LOCK TEST] Requesting controlled unlock..."
    );

    crate::logger::information(
        logfile,
        "[LOCK TEST] Requesting controlled unlock",
    );


    lock.unlock_and_destroy();


    event_queue
        .roundtrip(
            &mut state
        )
        .map_err(
            |error| {
                format!(
                    "Unable to synchronize the controlled unlock with the compositor: {}",
                    error,
                )
            }
        )?;


    for entry in &mut state.lock_surfaces {
        entry.gl_context =
            None;

        for frame in entry.frames.drain(..) {
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


    println!(
        "[LOCK TEST] Session unlocked successfully."
    );

    crate::logger::information(
        logfile,
        "[LOCK TEST] Session unlocked successfully",
    );


    Ok(())
}


impl Dispatch<
    wl_registry::WlRegistry,
    ()
> for TestState {
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
> for TestState {
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
> for TestState {
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
                    return;
                }


                if state.interaction_state
                    == LockInteractionState::Rendering
                {
                    // The wake key requests the dialog but is deliberately not
                    // inserted into the password field.
                    state.interaction_state =
                        LockInteractionState::AuthenticationRequested;

                    state.authentication.clear();

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


                match state.authentication.handle_key(
                    keysym.raw(),
                    &utf8,
                ) {
                    crate::display_lock_authentication::AuthenticationAction::None => {}


                    crate::display_lock_authentication::AuthenticationAction::Dismiss => {
                        state.interaction_state =
                            LockInteractionState::Rendering;

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
                                &password,
                            );


                        // Drop the local credential copy immediately after PAM
                        // returns.  Never log it or include it in diagnostics.
                        drop(
                            password
                        );


                        match result {
                            crate::authenticate_user::AuthenticationResult::Success => {
                                state.authentication.set_status(
                                    "PAM authentication succeeded — diagnostic mode remains locked"
                                );

                                state.input_events.push(
                                    "PAM authentication succeeded; diagnostic checkpoint deliberately withheld unlock authority"
                                        .to_string()
                                );
                            }


                            crate::authenticate_user::AuthenticationResult::Rejected => {
                                state.authentication.set_status(
                                    "Authentication failed"
                                );

                                state.input_events.push(
                                    "PAM authentication rejected credentials; session remains locked"
                                        .to_string()
                                );
                            }


                            crate::authenticate_user::AuthenticationResult::Error(error) => {
                                state.authentication.set_status(
                                    "Authentication service error"
                                );

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
> for TestState {
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
                                state.interaction_state =
                                    LockInteractionState::AuthenticationRequested;

                                state.authentication.clear();

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
                        state.interaction_state =
                            LockInteractionState::AuthenticationRequested;

                        state.authentication.clear();
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
            }


            _ => {}
        }
    }
}


impl Dispatch<
    ExtSessionLockV1,
    ()
> for TestState {
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
> for TestState {
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


        match create_diagnostic_buffer(
            &shm,
            qh,
            width,
            height,
            0,
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
                    DiagnosticFrame {
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
                    "[LOCK TEST] Unable to create diagnostic lock buffer for output {}: {}",
                    output_name,
                    error,
                );
            }
        }
    }
}


fn load_audio_bloom_screensaver_policy_ids(
) -> Result<HashSet<i64>, String> {
    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open Screenshaver database while locating Audio Bloom policies: {}",
                        error,
                    )
                }
            )?;


    let mut statement =
        connection
            .prepare(
                "SELECT policy_id
                   FROM shader_policies
                  WHERE policy_target = 'screensaver'
                    AND lower(COALESCE(bloom_mode, 'off')) = 'audio'
                  ORDER BY policy_id"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare Audio Bloom policy query: {}",
                        error,
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                [],
                |row| {
                    row.get::<_, i64>(
                        0
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query Audio Bloom Screensaver policies: {}",
                        error,
                    )
                }
            )?;


    let mut policy_ids =
        HashSet::new();


    for row in rows {
        let policy_id =
            row
                .map_err(
                    |error| {
                        format!(
                            "Unable to decode Audio Bloom policy id: {}",
                            error,
                        )
                    }
                )?;


        policy_ids.insert(
            policy_id
        );
    }


    Ok(
        policy_ids
    )
}


fn update_diagnostic_frames(
    state: &mut TestState,
    qh: &QueueHandle<TestState>,
    phase: u32,
) -> Result<(), String> {
    let shm =
        state
            .shm
            .clone()
            .ok_or_else(
                || {
                    "Wayland wl_shm became unavailable during lock test"
                        .to_string()
                }
            )?;


    for entry in &mut state.lock_surfaces {
        if !entry.configured
            || entry.width == 0
            || entry.height == 0
        {
            continue;
        }


        let (
            file,
            pool,
            buffer,
        ) =
            create_diagnostic_buffer(
                &shm,
                qh,
                entry.width,
                entry.height,
                phase,
            )?;


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
            entry.width as i32,
            entry.height as i32,
        );

        entry.surface.commit();


        entry.frames.push(
            DiagnosticFrame {
                backing_file: file,
                pool,
                buffer,
            }
        );
    }


    Ok(())
}


fn create_diagnostic_buffer(
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<TestState>,
    width: u32,
    height: u32,
    phase: u32,
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
                "Unable to size diagnostic SHM buffer: {}",
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
                "Unable to seek diagnostic SHM buffer: {}",
                error,
            )
        }
    )?;


    // XRGB8888 on little-endian Linux is stored in memory as B, G, R, X.
    // Generate a moving two-dimensional diagnostic gradient.  This is not
    // intended as production rendering; it only proves that secure lock
    // surfaces can receive repeated visual updates while the session remains
    // compositor-locked.
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

            let movement =
                phase
                    .wrapping_mul(29) as u8;

            let red =
                horizontal
                    .wrapping_add(
                        movement
                    );

            let green =
                vertical
                    .wrapping_add(
                        movement / 2
                    );

            let blue =
                horizontal
                    .wrapping_add(
                        vertical
                    )
                    .wrapping_add(
                        movement
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
                    "Unable to write diagnostic SHM buffer: {}",
                    error,
                )
            }
        )?;
    }


    file.flush()
        .map_err(
            |error| {
                format!(
                    "Unable to flush diagnostic SHM buffer: {}",
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
            "screenshaver-lock-test"
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
    TestState:
        ignore wl_compositor::WlCompositor
);

delegate_noop!(
    TestState:
        ignore wl_output::WlOutput
);

delegate_noop!(
    TestState:
        ignore wl_shm::WlShm
);

delegate_noop!(
    TestState:
        ignore wl_shm_pool::WlShmPool
);

delegate_noop!(
    TestState:
        ignore wl_buffer::WlBuffer
);

delegate_noop!(
    TestState:
        ignore wl_surface::WlSurface
);

delegate_noop!(
    TestState:
        ignore ExtSessionLockManagerV1
);
