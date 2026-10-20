//! Windowshader-owned Wayland geometry and presentation-state persistence.
//! XDG presentation is owned here; EGL frame processing remains shared.

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct WindowshaderGeometry {
    #[serde(default)]
    pub(crate) x: Option<i32>,
    #[serde(default)]
    pub(crate) y: Option<i32>,
    pub(crate) width: i32,
    pub(crate) height: i32,
    #[serde(default)]
    pub(crate) maximized: bool,
}

impl Default for WindowshaderGeometry {
    fn default() -> Self {
        Self { x: None, y: None, width: 960, height: 540, maximized: false }
    }
}

pub(crate) fn load_windowshader_geometry() -> WindowshaderGeometry {
    let default = WindowshaderGeometry::default();
    let geometry = crate::manage_runtime_state::read_state().ok()
        .and_then(|root| root.get("windowshader").cloned())
        .and_then(|value| serde_json::from_value::<WindowshaderGeometry>(value).ok());
    match geometry {
        Some(geometry) if (64..=16384).contains(&geometry.width)
            && (64..=16384).contains(&geometry.height) => geometry,
        _ => default,
    }
}

pub(crate) fn save_windowshader_geometry(geometry: WindowshaderGeometry) -> Result<(), String> {
    let value = serde_json::to_value(geometry).map_err(|error| error.to_string())?;
    crate::manage_runtime_state::update_state(move |root| {
        root.as_object_mut().ok_or("Runtime state root is not an object")?
            .insert("windowshader".to_string(), value);
        Ok(())
    })
}


// The XDG toplevel lifecycle is owned here; EGL/frame processing remains shared.
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use wayland_client::Proxy;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;
use crate::wayland_wallpaper::{
    WallpaperSurfaceState, WallpaperTargetInfo, NativeWallpaperTarget,
    select_safe_wallpaper_shader, print_active_wallpaper_shader,
    configured_dimension,
};

// Stage 2D-2B Step 1: Windowshader owns the frame-loop entry point.
// Windowshader owns EGL session acquisition and release; GL frame processing
// remains shared until the separate rendering loop is extracted.
// Keeping this boundary here makes the future Windowshader loop replacement
// local to this module without changing XDG surface ownership or callbacks.
fn render_windowshader_frames(
    connection: &wayland_client::Connection,
    event_queue: &mut wayland_client::EventQueue<WindowshaderDispatchState>,
    state: &mut WindowshaderDispatchState,
    native_targets: &mut Vec<NativeWallpaperTarget>,
    active_shader: &crate::wayland_wallpaper::ActiveWallpaperShader,
    shader_manager: &mut crate::manage_shader::ShaderManager,
    wallpaper_directory: &Path,
    shader_interval: Option<Duration>,
    runtime: &crate::define_wallpaper::WallpaperRuntime,
    running: &Arc<AtomicBool>,
    control: &crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
) -> Result<(), String> {
    // Windowshader explicitly acquires and owns its EGL session.
    // Dropping this local session releases EGL on success or failure.
    let mut egl_session = crate::wayland_wallpaper::EglRenderSession::initialize(
        connection, native_targets,
    )?;
    run_windowshader_frame_loop(
        &mut egl_session, event_queue, state, native_targets, active_shader,
        shader_manager, wallpaper_directory, shader_interval, runtime,
        running, control,
    )
}

// Stage 2D-2C Step 1: Windowshader owns frame execution sequencing.
// OpenGL entry-point preparation and frame processing remain common utilities.
// This boundary will allow Windowshader-specific loop state to move here later.
fn run_windowshader_frame_loop(
    egl_session: &mut crate::wayland_wallpaper::EglRenderSession,
    event_queue: &mut wayland_client::EventQueue<WindowshaderDispatchState>,
    state: &mut WindowshaderDispatchState,
    native_targets: &mut Vec<NativeWallpaperTarget>,
    active_shader: &crate::wayland_wallpaper::ActiveWallpaperShader,
    shader_manager: &mut crate::manage_shader::ShaderManager,
    wallpaper_directory: &Path,
    shader_interval: Option<Duration>,
    runtime: &crate::define_wallpaper::WallpaperRuntime,
    running: &Arc<AtomicBool>,
    control: &crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
) -> Result<(), String> {
    let initial_timing = crate::wayland_wallpaper::FrameLoopInitialTiming::for_shader(
        runtime, active_shader,
    );
    crate::wayland_render_core::prepare_egl_frame_processing();
    crate::wayland_render_core::execute_egl_frame_processing_with_driver(
        egl_session, event_queue, state, native_targets, active_shader,
        shader_manager, wallpaper_directory, shader_interval, runtime,
        running, control, initial_timing, drive_windowshader_frame_loop,
    )
}

// Windowshader owns the iteration boundary and the termination decision.
// Render Core owns the reusable GL/frame step and resource cleanup. Never
// fabricate compositor callbacks to advance this loop.
fn drive_windowshader_frame_loop(
    step: &mut crate::wayland_render_core::FrameStep<'_>,
) -> Result<(), String> {
    loop {
        match step()? {
            crate::wayland_render_core::FrameLoopAction::Continue => {},
            crate::wayland_render_core::FrameLoopAction::Stop => return Ok(()),
        }
    }
}

// Stage 2D-2B Step 2: Windowshader owns its post-render surface lifecycle.
// Preserve geometry persistence, XDG destruction, and compositor flush order.
fn finish_windowshader_surface(
    native_targets: Vec<NativeWallpaperTarget>,
    state: &mut WindowshaderDispatchState,
    saved_geometry: WindowshaderGeometry,
    last_position: Option<(i32, i32)>,
    xdg_wm_base: &wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase,
    connection: &wayland_client::Connection,
) -> Result<(), String> {
    if let Some(target) = native_targets.first() {
        let position = last_position
            .or(saved_geometry.x.zip(saved_geometry.y));
        let geometry = crate::wayland_windowshader::WindowshaderGeometry {
            x: position.map(|(x, _)| x),
            y: position.map(|(_, y)| y),
            width: if state.windowshader.windowed_maximized {
                state.windowshader.windowed_normal_width
            } else { target.width },
            height: if state.windowshader.windowed_maximized {
                state.windowshader.windowed_normal_height
            } else { target.height },
            maximized: state.windowshader.windowed_maximized,
        };
        if let Err(error) = crate::wayland_windowshader::save_windowshader_geometry(geometry) {
            eprintln!("[WINDOWSHADER] Unable to save window geometry: {}", error);
        }
    }

    for target in native_targets {

        drop(
            target.egl_window
        );


        if let Some(decoration) = target.toplevel_decoration {
            decoration.destroy();
        }

        if let Some(xdg_toplevel) =
            target.xdg_toplevel
        {
            xdg_toplevel.destroy();
        }


        if let Some(xdg_surface) =
            target.xdg_surface
        {
            xdg_surface.destroy();
        }


        target.surface.destroy();
    }


    if let Some(manager) = state.decoration_manager.take() {
        manager.destroy();
    }

    xdg_wm_base.destroy();


    connection
        .flush()
        .map_err(
            |error| {
                format!(
                    "Unable to send Windowed wallpaper cleanup requests: {}",
                    error,
                )
            }
        )?;


    Ok(())
}

pub fn run_egl_windowed_surface(
    mut shader_manager: crate::manage_shader::ShaderManager,
    wallpaper_directory: &Path,
    shader_interval: Option<Duration>,
    runtime: &crate::define_wallpaper::WallpaperRuntime,
    running: Arc<AtomicBool>,
    control: crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
) -> Result<(), String> {

    const WINDOWED_TARGET_ID: u32 = 0;
    let saved_geometry = crate::wayland_windowshader::load_windowshader_geometry();
    // A failed KDE integration must never prevent Windowshader rendering.
    let _kwin_guard = match crate::manage_windowshader_kde::WindowshaderKwinGuard::start(saved_geometry.x.zip(saved_geometry.y)) {
        Ok(guard) => guard,
        Err(error) => {
            eprintln!("[WINDOWSHADER] KDE window integration unavailable: {}", error);
            None
        }
    };


    runtime.tray_status
        .set_starting();


    let active_shader =
        select_safe_wallpaper_shader(
            &mut shader_manager,
            wallpaper_directory,
        )?;


    print_active_wallpaper_shader(
        &active_shader
    );


    runtime.tray_status.set_active(
        active_shader.policy_id,
        active_shader.shader_name.clone(),
        active_shader.source_path.clone(),
    );


    let (
        connection,
        mut event_queue,
        mut state,
    ) =
        connect_and_bind_windowshader_dispatch()?;


    state.windowshader.windowed_normal_width = saved_geometry.width;
    state.windowshader.windowed_normal_height = saved_geometry.height;

    let queue_handle =
        event_queue.handle();


    let compositor =
        state
            .compositor
            .clone()
            .ok_or_else(
                || {
                    "The Wayland compositor did not advertise wl_compositor"
                        .to_string()
                }
            )?;


    let xdg_wm_base =
        state
            .xdg_wm_base
            .clone()
            .ok_or_else(
                || {
                    "The Wayland compositor did not advertise xdg_wm_base"
                        .to_string()
                }
            )?;


    state.surface_states.push(
        WallpaperSurfaceState {
            registry_name:
                WINDOWED_TARGET_ID,

            configured:
                None,

            closed:
                false,
        }
    );


    let surface =
        compositor.create_surface(
            &queue_handle,
            (),
        );


    let xdg_surface =
        xdg_wm_base.get_xdg_surface(
            &surface,
            &queue_handle,
            (),
        );


    let xdg_toplevel =
        xdg_surface.get_toplevel(
            &queue_handle,
            (),
        );


    let toplevel_decoration =
        state.decoration_manager.as_ref().map(|manager| {
            let decoration = manager.get_toplevel_decoration(
                &xdg_toplevel,
                &queue_handle,
                (),
            );
            decoration.set_mode(zxdg_toplevel_decoration_v1::Mode::ServerSide);
            decoration
        });

    xdg_toplevel.set_title(
        "Screenshaver Windowshader"
            .to_string()
    );


    xdg_toplevel.set_app_id(
        "screenshaver"
            .to_string()
    );

    xdg_toplevel.set_min_size(64, 64);
    xdg_toplevel.set_max_size(0, 0);
    // Let xdg_surface derive its window geometry from the resized wl_surface.
    // A fixed startup geometry would constrain the KWin frame after EGL grows.
    if saved_geometry.maximized {
        xdg_toplevel.set_maximized();
    }


    surface.commit();


    connection
        .flush()
        .map_err(
            |error| {
                format!(
                    "Unable to send Windowed wallpaper surface requests: {}",
                    error,
                )
            }
        )?;


    while state
        .surface_states
        .iter()
        .any(
            |surface_state| {
                surface_state.registry_name
                    == WINDOWED_TARGET_ID
                    && surface_state.configured.is_none()
                    && !surface_state.closed
            }
        )
    {
        event_queue
            .blocking_dispatch(
                &mut state
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to receive Windowed wallpaper configure events: {}",
                        error,
                    )
                }
            )?;
    }


    let surface_state =
        state
            .surface_states
            .iter_mut()
            .find(
                |surface_state| {
                    surface_state.registry_name
                        == WINDOWED_TARGET_ID
                }
            )
            .ok_or_else(
                || {
                    "Windowed wallpaper surface state was not created"
                        .to_string()
                }
            )?;


    if surface_state.closed
        && surface_state.configured.is_none()
    {
        // A compositor Close request for Windowshader is an application quit.
        running.store(false, Ordering::SeqCst);
        return Err(
            "The compositor closed the Windowed wallpaper before configuring it"
                .to_string()
        );
    }


    let configuration =
        surface_state
            .configured
            .take()
            .ok_or_else(
                || {
                    "The compositor did not configure the Windowed wallpaper"
                        .to_string()
                }
            )?;


    let width =
        configured_dimension(
            configuration.width,
            saved_geometry.width,
            "width",
        )?;


    let height =
        configured_dimension(
            configuration.height,
            saved_geometry.height,
            "height",
        )?;


    println!(
        "Wayland Windowed wallpaper configured successfully:"
    );


    println!(
        "    Width: {}",
        width
    );


    println!(
        "    Height: {}",
        height
    );


    println!(
        "    Configure serial: {}",
        configuration.serial
    );


    println!(
        "    Window role: xdg_toplevel"
    );


    println!(
        "    Placement: compositor-managed"
    );


    println!();


    let egl_window =
        wayland_egl::WlEglSurface::new(
            surface.id(),
            width,
            height,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to create wl_egl_window for Windowed wallpaper: {}",
                    error,
                )
            }
        )?;


    let info =
        WallpaperTargetInfo {
            registry_name:
                WINDOWED_TARGET_ID,

            description:
                Some(
                    "Windowed wallpaper"
                        .to_string()
                ),

            mode_width:
                width,

            mode_height:
                height,

            scale:
                1,

            complete:
                true,

            ..WallpaperTargetInfo::default()
        };


    let mut native_targets =
        vec![
            NativeWallpaperTarget {
                info,
                surface,
                layer_surface:
                    None,
                input_region:
                    None,
                xdg_surface:
                    Some(
                        xdg_surface
                    ),
                xdg_toplevel:
                    Some(
                        xdg_toplevel
                    ),
                toplevel_decoration,
                egl_window,
                width,
                height,
            }
        ];


    let render_result =
        render_windowshader_frames(
            &connection,
            &mut event_queue,
            &mut state,
            &mut native_targets,
            &active_shader,
            &mut shader_manager,
            wallpaper_directory,
            shader_interval,
            runtime,
            &running,
            &control,
        );


    let last_position = _kwin_guard.as_ref().and_then(|guard| guard.last_position());
    finish_windowshader_surface(
        native_targets, &mut state, saved_geometry, last_position,
        &xdg_wm_base, &connection,
    )?;

    render_result
}


// XDG and frame-callback event dispatch belongs to Windowshader.
#[derive(Debug, Clone)]
pub(crate) struct FrameCallbackDispatchData {
    pub(crate) registry_name: u32,
    pub(crate) sequence: u64,
}

#[derive(Debug)]
pub(crate) struct WindowshaderCallbackDiagnostic {
    pub(crate) sequence: u64,
    pub(crate) requested_at: Instant,
    pub(crate) swap_completed: bool,
    pub(crate) flushed: bool,
}



// Windowshader-only XDG geometry, activation, and callback bookkeeping.
// Frame readiness and callback diagnostics are Windowshader-owned.
#[derive(Debug, Default)]
pub(crate) struct WindowshaderWaylandState {
    pub(crate) windowed_pending_width: i32,
    pub(crate) windowed_pending_height: i32,
    pub(crate) windowed_maximized: bool,
    pub(crate) windowed_normal_width: i32,
    pub(crate) windowed_normal_height: i32,
    pub(crate) windowed_activation_established: bool,
    pub(crate) windowed_activated: bool,
    pub(crate) windowshader_next_callback_sequence: u64,
    pub(crate) frame_ready: HashMap<u32, bool>,
    pub(crate) windowshader_pending_callbacks: HashMap<u32, WindowshaderCallbackDiagnostic>,
    // Diagnostic only: capture the first configure of each kind after a pause.
    pub(crate) windowshader_trace_surface_configure: bool,
    pub(crate) windowshader_trace_toplevel_configure: bool,
    pub(crate) windowshader_configure_count: u64,
}


use std::time::Instant;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_client::protocol::{wl_callback};
use wayland_protocols::xdg::shell::client::{xdg_wm_base::{self, XdgWmBase}, xdg_surface::{self, XdgSurface}, xdg_toplevel::{self, XdgToplevel}};
use crate::wayland_wallpaper::{WaylandState, WallpaperSurfaceConfiguration};

impl Dispatch<XdgWmBase, ()>
    for WaylandState
{
    fn event(
        _state: &mut Self,
        xdg_wm_base: &XdgWmBase,
        event: xdg_wm_base::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {

        if let xdg_wm_base::Event::Ping {
            serial,
        } = event
        {
            xdg_wm_base.pong(
                serial
            );
        }
    }
}


impl Dispatch<XdgSurface, ()>
    for WaylandState
{
    fn event(
        state: &mut Self,
        xdg_surface: &XdgSurface,
        event: xdg_surface::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {

        if let xdg_surface::Event::Configure {
            serial,
        } = event
        {
            if state.windowshader.windowshader_trace_surface_configure {
                state.windowshader.windowshader_trace_surface_configure = false;
                let pending = state.windowshader.windowshader_pending_callbacks.get(&0);
                crate::logger::warning(
                    &crate::locate_paths::runtime_log_path(),
                    &format!("[WINDOWSHADER_CONFIGURE_DIAG] First xdg_surface.configure after pause: serial={} pending_callback_seq={:?} pending_callback_age_ms={:?} frame_ready={} activated={} activation_established={}",
                        serial, pending.map(|p| p.sequence),
                        pending.map(|p| p.requested_at.elapsed().as_millis()),
                        state.windowshader.frame_ready.get(&0).copied().unwrap_or(true),
                        state.windowshader.windowed_activated, state.windowshader.windowed_activation_established),
                );
            }
            state.windowshader.windowshader_configure_count = state.windowshader.windowshader_configure_count.wrapping_add(1);
            xdg_surface.ack_configure(
                serial
            );


            let configured_width =
                u32::try_from(
                    state.windowshader.windowed_pending_width
                )
                .unwrap_or(
                    0
                );


            let configured_height =
                u32::try_from(
                    state.windowshader.windowed_pending_height
                )
                .unwrap_or(
                    0
                );


            if let Some(
                surface_state
            ) =
                state
                    .surface_states
                    .iter_mut()
                    .find(
                        |surface_state| {
                            surface_state.registry_name
                                == 0
                        }
                    )
            {
                surface_state.configured =
                    Some(
                        WallpaperSurfaceConfiguration {
                            width:
                                configured_width,

                            height:
                                configured_height,

                            serial,
                        }
                    );
            }
        }
    }
}


impl Dispatch<XdgToplevel, ()>
    for WaylandState
{
    fn event(
        state: &mut Self,
        _xdg_toplevel: &XdgToplevel,
        event: xdg_toplevel::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {

        match event {

            xdg_toplevel::Event::Configure {
                width,
                height,
                states,
            } => {
                if state.windowshader.windowshader_trace_toplevel_configure {
                    state.windowshader.windowshader_trace_toplevel_configure = false;
                    let pending = state.windowshader.windowshader_pending_callbacks.get(&0);
                    crate::logger::warning(
                        &crate::locate_paths::runtime_log_path(),
                        &format!("[WINDOWSHADER_CONFIGURE_DIAG] First xdg_toplevel.configure after pause: width={} height={} state_bytes={} pending_callback_seq={:?} pending_callback_age_ms={:?} frame_ready={} previous_activated={}",
                            width, height, states.len(), pending.map(|p| p.sequence),
                            pending.map(|p| p.requested_at.elapsed().as_millis()),
                            state.windowshader.frame_ready.get(&0).copied().unwrap_or(true), state.windowshader.windowed_activated),
                    );
                }

                // The first xdg_toplevel configure may legitimately contain
                // no states.  Do not treat that initial, unestablished state
                // as an inactive workspace.  Once the compositor supplies a
                // non-empty state set, track Activated as the Windowshader's
                // presentation-visibility signal.
                if !states.is_empty() {
                    let activation_was_established = state.windowshader.windowed_activation_established;
                    state.windowshader.windowed_activation_established = true;


                    let previous_activation = state.windowshader.windowed_activated;
                    state.windowshader.windowed_activated =
                        states
                            .chunks_exact(
                                std::mem::size_of::<u32>()
                            )
                            .map(
                                |bytes| {
                                    u32::from_ne_bytes(
                                        [
                                            bytes[0],
                                            bytes[1],
                                            bytes[2],
                                            bytes[3],
                                        ]
                                    )
                                }
                            )
                            .any(
                                |raw_state| {
                                    matches!(
                                        xdg_toplevel::State::try_from(
                                            raw_state
                                        ),
                                        Ok(
                                            xdg_toplevel::State::Activated
                                        )
                                    )
                                }
                            );
                    if !activation_was_established || previous_activation != state.windowshader.windowed_activated {
                        crate::logger::warning(
                            &crate::locate_paths::runtime_log_path(),
                            &format!("[WINDOWSHADER_CALLBACK_DIAG] Activation transition: activated={} (focus signal, not visibility)", state.windowshader.windowed_activated),
                        );
                    }
                }


                let maximized = states.chunks_exact(4)
                    .map(|bytes| u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                    .any(|raw| matches!(
                        xdg_toplevel::State::try_from(raw),
                        Ok(xdg_toplevel::State::Maximized)
                    ));
                state.windowshader.windowed_maximized = maximized;
                if !maximized {
                    if width > 0 { state.windowshader.windowed_normal_width = width; }
                    if height > 0 { state.windowshader.windowed_normal_height = height; }
                }

                state.windowshader.windowed_pending_width =
                    width;


                state.windowshader.windowed_pending_height =
                    height;
            }


            xdg_toplevel::Event::Close => {

                if let Some(
                    surface_state
                ) =
                    state
                        .surface_states
                        .iter_mut()
                        .find(
                            |surface_state| {
                                surface_state.registry_name
                                    == 0
                            }
                        )
                {
                    surface_state.closed =
                        true;
                }
            }


            _ => {}
        }
    }
}


impl Dispatch<wl_callback::WlCallback, FrameCallbackDispatchData>
    for WaylandState
{
    fn event(
        state: &mut Self,
        _callback: &wl_callback::WlCallback,
        event: wl_callback::Event,
        data: &FrameCallbackDispatchData,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        if matches!(
            event,
            wl_callback::Event::Done { .. }
        ) {
            let pending = state.windowshader.windowshader_pending_callbacks.remove(&data.registry_name);
            if let Some(pending) = pending {
                if pending.requested_at.elapsed() >= Duration::from_millis(250) || pending.sequence != data.sequence {
                crate::logger::warning(
                    &crate::locate_paths::runtime_log_path(),
                    &format!("[WINDOWSHADER_CALLBACK_DIAG] Callback acknowledged: target={} seq={} age_ms={} swap_completed={} flushed={} matching_sequence={}",
                        data.registry_name, data.sequence, pending.requested_at.elapsed().as_millis(),
                        pending.swap_completed, pending.flushed, pending.sequence == data.sequence),
                );
                }
            } else if data.sequence != 0 {
                crate::logger::warning(
                    &crate::locate_paths::runtime_log_path(),
                    &format!("[WINDOWSHADER_CALLBACK_DIAG] Callback without pending record: target={} seq={}", data.registry_name, data.sequence),
                );
            }
            state.windowshader.frame_ready.insert(
                data.registry_name,
                true,
            );
        }
    }
}





// Stage 2B-4B2B: Windowshader owns registration of its XDG globals.
// The shared dispatch-state and event queue are intentionally retained here;
// switching their type requires a separate renderer-interface change.
pub(crate) fn bind_windowshader_registry_global(
    state: &mut WaylandState,
    registry: &wayland_client::protocol::wl_registry::WlRegistry,
    name: u32,
    interface: &str,
    version: u32,
    queue_handle: &QueueHandle<WaylandState>,
) {
    match interface {
        "xdg_wm_base" => {
            state.xdg_wm_base_version = Some(version);
            if state.xdg_wm_base.is_none() {
                state.xdg_wm_base = Some(registry.bind::<XdgWmBase, _, _>(
                    name, version.min(6), queue_handle, (),
                ));
            }
        }
        "zxdg_decoration_manager_v1" => {
            if state.decoration_manager.is_none() {
                state.decoration_manager = Some(registry.bind::<
                    wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
                    _, _,
                >(name, version.min(1), queue_handle, ()));
            }
        }
        _ => {}
    }
}

// Windowshader owns creation and capability negotiation of its Wayland connection
// and event queue. The queue state type remains shared until the render-loop
// interface and registry dispatch can be separated safely.
pub(crate) fn connect_and_bind_windowed(
) -> Result<
    (
        Connection,
        wayland_client::EventQueue<WaylandState>,
        WaylandState,
    ),
    String,
> {

    let connection =
        Connection::connect_to_env()
            .map_err(
                |error| {
                    format!(
                        "Unable to connect to the Wayland compositor: {}",
                        error,
                    )
                }
            )?;


    let display =
        connection.display();


    let mut event_queue =
        connection.new_event_queue();


    let queue_handle =
        event_queue.handle();


    let _registry =
        display.get_registry(
            &queue_handle,
            (),
        );


    let mut state =
        WaylandState::default();


    event_queue
        .roundtrip(
            &mut state
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read Wayland compositor capabilities for Windowed wallpaper: {}",
                    error,
                )
            }
        )?;


    if state
        .compositor
        .is_none()
    {
        return Err(
            "The Wayland compositor did not advertise wl_compositor"
                .to_string()
        );
    }


    if state
        .xdg_wm_base
        .is_none()
    {
        return Err(
            "The Wayland compositor did not advertise xdg_wm_base"
                .to_string()
        );
    }


    Ok(
        (
            connection,
            event_queue,
            state,
        )
    )
}


// Stage 2B-4B2C: independent Windowshader Wayland dispatch ownership.
// This is additive until the renderer interface accepts this queue type.
#[derive(Debug, Default)]
pub(crate) struct WindowshaderDispatchState {
    pub(crate) compositor: Option<wayland_client::protocol::wl_compositor::WlCompositor>,
    pub(crate) xdg_wm_base: Option<XdgWmBase>,
    pub(crate) decoration_manager: Option<wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    pub(crate) compositor_version: Option<u32>,
    pub(crate) xdg_wm_base_version: Option<u32>,
    pub(crate) output_count: usize,
    pub(crate) surface_states: Vec<WallpaperSurfaceState>,
    pub(crate) removed_output_names: Vec<u32>,
    pub(crate) windowshader: WindowshaderWaylandState,
}

// Stage 2D-2C Step 12: Windowshader owns callback diagnostics following a
// successful Wayland flush. The shared pump invokes this only after flush().
impl WindowshaderDispatchState {
    fn on_successful_wayland_flush(&mut self) {
        for (target, pending) in self.windowshader.windowshader_pending_callbacks.iter_mut() {
            if !pending.flushed {
                pending.flushed = true;
                if pending.requested_at.elapsed() >= Duration::from_millis(250) {
                    crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                        &format!("[WINDOWSHADER_CALLBACK_DIAG] Wayland flush succeeded: target={} seq={} age_ms={} swap_completed={}",
                            target, pending.sequence, pending.requested_at.elapsed().as_millis(), pending.swap_completed));
                }
            }
        }
    }
}

impl crate::wayland_wallpaper::RenderWaylandDispatch for WindowshaderDispatchState {
    fn surfaces(&mut self) -> &mut Vec<WallpaperSurfaceState> { &mut self.surface_states }
    fn presentation_diagnostics(&self, native_targets: &[NativeWallpaperTarget]) -> crate::wayland_wallpaper::PresentationDiagnostics {
        let pending = self.windowshader.windowshader_pending_callbacks.get(&0);
        crate::wayland_wallpaper::PresentationDiagnostics {
            frame_ready: native_targets.iter().all(|target| self.windowshader.frame_ready
                .get(&target.info.registry_name).copied().unwrap_or(true)),
            pending_description: pending.map_or_else(|| "none".to_string(), |record| format!(
                "seq={} age_ms={} swap_completed={} flushed={}", record.sequence,
                record.requested_at.elapsed().as_millis(), record.swap_completed, record.flushed)),
            pending_sequence: pending.map(|record| record.sequence),
            pending_age_ms: pending.map(|record| record.requested_at.elapsed().as_millis()),
            activated: self.windowshader.windowed_activated,
            activation_established: self.windowshader.windowed_activation_established,
            configure_count: self.windowshader.windowshader_configure_count,
        }
    }
    fn removed_outputs(&mut self) -> &mut Vec<u32> { &mut self.removed_output_names }
    fn compositor_close_requests_shutdown() -> bool { true }
    fn on_pause_entering(&mut self) {
        self.windowshader.windowshader_trace_surface_configure = true;
        self.windowshader.windowshader_trace_toplevel_configure = true;
    }
    fn on_pause_entered(&mut self, native_targets: &[NativeWallpaperTarget]) {
        if let Some(pending) = self.windowshader.windowshader_pending_callbacks.get(&0) {
            crate::logger::warning(
                &crate::locate_paths::runtime_log_path(),
                &format!("[WINDOWSHADER_CALLBACK_DIAG] Pause entered: seq={} age_ms={} swap_completed={} flushed={}",
                    pending.sequence, pending.requested_at.elapsed().as_millis(), pending.swap_completed, pending.flushed),
            );
        }
        let outstanding = native_targets.iter()
            .filter(|target| !self.windowshader.frame_ready.get(&target.info.registry_name)
                .copied().unwrap_or(true))
            .count();
        crate::logger::warning(
            &crate::locate_paths::runtime_log_path(),
            &format!("[WINDOWSHADER_RESUME_DIAG] Pause entered; outstanding_frame_callbacks={outstanding}"),
        );
    }

    fn on_pause_released(&mut self) {
        if let Some(pending) = self.windowshader.windowshader_pending_callbacks.get(&0) {
            crate::logger::warning(
                &crate::locate_paths::runtime_log_path(),
                &format!("[WINDOWSHADER_CALLBACK_DIAG] Pause released: seq={} age_ms={} swap_completed={} flushed={}",
                    pending.sequence, pending.requested_at.elapsed().as_millis(), pending.swap_completed, pending.flushed),
            );
        }
        crate::logger::warning(
            &crate::locate_paths::runtime_log_path(),
            "[WINDOWSHADER_RESUME_DIAG] Pause released; awaiting compositor-paced presentation",
        );
    }
    fn on_resumed_frame_presented(&mut self) {
        crate::logger::warning(
            &crate::locate_paths::runtime_log_path(),
            "[WINDOWSHADER_RESUME_DIAG] Successful post-pause EGL presentation acknowledged",
        );
    }
    fn on_frame_swap_completed(&mut self, registry_name: u32) {
        if let Some(pending) = self.windowshader.windowshader_pending_callbacks.get_mut(&registry_name) {
            pending.swap_completed = true;
        }
    }
    fn frames_ready_for_presentation(&mut self, native_targets: &[NativeWallpaperTarget]) -> bool {
        all_frames_ready(&self.windowshader, native_targets)
    }
    fn outstanding_frame_callbacks(&mut self, native_targets: &[NativeWallpaperTarget]) -> Vec<u32> {
        native_targets.iter()
            .filter(|target| !self.windowshader.frame_ready.get(&target.info.registry_name)
                .copied().unwrap_or(true))
            .map(|target| target.info.registry_name).collect()
    }
    fn log_outstanding_callback_details(&mut self, outstanding: &[u32]) {
        let callback_details: Vec<String> = outstanding.iter().map(|target| {
            match self.windowshader.windowshader_pending_callbacks.get(target) {
                Some(pending) => format!(
                    "target={} seq={} age_ms={} swap_completed={} flushed={}",
                    target, pending.sequence, pending.requested_at.elapsed().as_millis(),
                    pending.swap_completed, pending.flushed,
                ),
                None => format!("target={} missing_pending_record", target),
            }
        }).collect();
        crate::logger::warning(
            &crate::locate_paths::runtime_log_path(),
            &format!("[WINDOWSHADER_CALLBACK_DIAG] Outstanding callback details: {}", callback_details.join("; ")),
        );
    }
    fn request_next_presentation_frame(
        &mut self, surface: &wayland_client::protocol::wl_surface::WlSurface,
        registry_name: u32, queue: &QueueHandle<Self>,
    ) {
        request_next_frame(&mut self.windowshader, surface, registry_name, queue);
    }
    fn reset_frame_deadline(deadline: &mut Instant) {
        crate::wayland_windowshader::reset_frame_deadline(deadline);
    }
    fn pace_frame(deadline: &mut Instant, frame_duration: Duration) {
        pace_next_frame(deadline, frame_duration);
    }
    fn pump_events(queue: &mut wayland_client::EventQueue<Self>, state: &mut Self) -> Result<(), String> {
        crate::wayland_wallpaper::process_wayland_events_shared(queue, state, |state| {
            state.on_successful_wayland_flush();
        })
    }
}

impl Dispatch<wayland_client::protocol::wl_registry::WlRegistry, ()> for WindowshaderDispatchState {
    fn event(
        state: &mut Self,
        registry: &wayland_client::protocol::wl_registry::WlRegistry,
        event: wayland_client::protocol::wl_registry::Event,
        _: &(), _: &Connection, queue: &QueueHandle<Self>,
    ) {
        use wayland_client::protocol::{wl_compositor, wl_registry};
        if let wl_registry::Event::Global { name, interface, version } = event {
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor_version = Some(version);
                    if state.compositor.is_none() {
                        state.compositor = Some(registry.bind::<wl_compositor::WlCompositor, _, _>(
                            name, version.min(6), queue, (),
                        ));
                    }
                }
                "xdg_wm_base" => {
                    state.xdg_wm_base_version = Some(version);
                    if state.xdg_wm_base.is_none() {
                        state.xdg_wm_base = Some(registry.bind::<XdgWmBase, _, _>(
                            name, version.min(6), queue, (),
                        ));
                    }
                }
                "zxdg_decoration_manager_v1" => {
                    if state.decoration_manager.is_none() {
                        state.decoration_manager = Some(registry.bind::<
                            wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, _, _
                        >(name, version.min(1), queue, ()));
                    }
                }
                "wl_output" => { state.output_count += 1; }
                _ => {}
            }
        }
    }
}

wayland_client::delegate_noop!(WindowshaderDispatchState: ignore wayland_client::protocol::wl_compositor::WlCompositor);
wayland_client::delegate_noop!(WindowshaderDispatchState: ignore wayland_client::protocol::wl_surface::WlSurface);
wayland_client::delegate_noop!(WindowshaderDispatchState: ignore wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1);

impl Dispatch<wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, ()> for WindowshaderDispatchState {
    fn event(_: &mut Self, _: &wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        _: zxdg_toplevel_decoration_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<XdgWmBase, ()>
    for WindowshaderDispatchState
{
    fn event(
        _state: &mut Self,
        xdg_wm_base: &XdgWmBase,
        event: xdg_wm_base::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {

        if let xdg_wm_base::Event::Ping {
            serial,
        } = event
        {
            xdg_wm_base.pong(
                serial
            );
        }
    }
}


impl Dispatch<XdgSurface, ()>
    for WindowshaderDispatchState
{
    fn event(
        state: &mut Self,
        xdg_surface: &XdgSurface,
        event: xdg_surface::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {

        if let xdg_surface::Event::Configure {
            serial,
        } = event
        {
            if state.windowshader.windowshader_trace_surface_configure {
                state.windowshader.windowshader_trace_surface_configure = false;
                let pending = state.windowshader.windowshader_pending_callbacks.get(&0);
                crate::logger::warning(
                    &crate::locate_paths::runtime_log_path(),
                    &format!("[WINDOWSHADER_CONFIGURE_DIAG] First xdg_surface.configure after pause: serial={} pending_callback_seq={:?} pending_callback_age_ms={:?} frame_ready={} activated={} activation_established={}",
                        serial, pending.map(|p| p.sequence),
                        pending.map(|p| p.requested_at.elapsed().as_millis()),
                        state.windowshader.frame_ready.get(&0).copied().unwrap_or(true),
                        state.windowshader.windowed_activated, state.windowshader.windowed_activation_established),
                );
            }
            state.windowshader.windowshader_configure_count = state.windowshader.windowshader_configure_count.wrapping_add(1);
            xdg_surface.ack_configure(
                serial
            );


            let configured_width =
                u32::try_from(
                    state.windowshader.windowed_pending_width
                )
                .unwrap_or(
                    0
                );


            let configured_height =
                u32::try_from(
                    state.windowshader.windowed_pending_height
                )
                .unwrap_or(
                    0
                );


            if let Some(
                surface_state
            ) =
                state
                    .surface_states
                    .iter_mut()
                    .find(
                        |surface_state| {
                            surface_state.registry_name
                                == 0
                        }
                    )
            {
                surface_state.configured =
                    Some(
                        WallpaperSurfaceConfiguration {
                            width:
                                configured_width,

                            height:
                                configured_height,

                            serial,
                        }
                    );
            }
        }
    }
}


impl Dispatch<XdgToplevel, ()>
    for WindowshaderDispatchState
{
    fn event(
        state: &mut Self,
        _xdg_toplevel: &XdgToplevel,
        event: xdg_toplevel::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {

        match event {

            xdg_toplevel::Event::Configure {
                width,
                height,
                states,
            } => {
                if state.windowshader.windowshader_trace_toplevel_configure {
                    state.windowshader.windowshader_trace_toplevel_configure = false;
                    let pending = state.windowshader.windowshader_pending_callbacks.get(&0);
                    crate::logger::warning(
                        &crate::locate_paths::runtime_log_path(),
                        &format!("[WINDOWSHADER_CONFIGURE_DIAG] First xdg_toplevel.configure after pause: width={} height={} state_bytes={} pending_callback_seq={:?} pending_callback_age_ms={:?} frame_ready={} previous_activated={}",
                            width, height, states.len(), pending.map(|p| p.sequence),
                            pending.map(|p| p.requested_at.elapsed().as_millis()),
                            state.windowshader.frame_ready.get(&0).copied().unwrap_or(true), state.windowshader.windowed_activated),
                    );
                }

                // The first xdg_toplevel configure may legitimately contain
                // no states.  Do not treat that initial, unestablished state
                // as an inactive workspace.  Once the compositor supplies a
                // non-empty state set, track Activated as the Windowshader's
                // presentation-visibility signal.
                if !states.is_empty() {
                    let activation_was_established = state.windowshader.windowed_activation_established;
                    state.windowshader.windowed_activation_established = true;


                    let previous_activation = state.windowshader.windowed_activated;
                    state.windowshader.windowed_activated =
                        states
                            .chunks_exact(
                                std::mem::size_of::<u32>()
                            )
                            .map(
                                |bytes| {
                                    u32::from_ne_bytes(
                                        [
                                            bytes[0],
                                            bytes[1],
                                            bytes[2],
                                            bytes[3],
                                        ]
                                    )
                                }
                            )
                            .any(
                                |raw_state| {
                                    matches!(
                                        xdg_toplevel::State::try_from(
                                            raw_state
                                        ),
                                        Ok(
                                            xdg_toplevel::State::Activated
                                        )
                                    )
                                }
                            );
                    if !activation_was_established || previous_activation != state.windowshader.windowed_activated {
                        crate::logger::warning(
                            &crate::locate_paths::runtime_log_path(),
                            &format!("[WINDOWSHADER_CALLBACK_DIAG] Activation transition: activated={} (focus signal, not visibility)", state.windowshader.windowed_activated),
                        );
                    }
                }


                let maximized = states.chunks_exact(4)
                    .map(|bytes| u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                    .any(|raw| matches!(
                        xdg_toplevel::State::try_from(raw),
                        Ok(xdg_toplevel::State::Maximized)
                    ));
                state.windowshader.windowed_maximized = maximized;
                if !maximized {
                    if width > 0 { state.windowshader.windowed_normal_width = width; }
                    if height > 0 { state.windowshader.windowed_normal_height = height; }
                }

                state.windowshader.windowed_pending_width =
                    width;


                state.windowshader.windowed_pending_height =
                    height;
            }


            xdg_toplevel::Event::Close => {

                if let Some(
                    surface_state
                ) =
                    state
                        .surface_states
                        .iter_mut()
                        .find(
                            |surface_state| {
                                surface_state.registry_name
                                    == 0
                            }
                        )
                {
                    surface_state.closed =
                        true;
                }
            }


            _ => {}
        }
    }
}


impl Dispatch<wl_callback::WlCallback, FrameCallbackDispatchData>
    for WindowshaderDispatchState
{
    fn event(
        state: &mut Self,
        _callback: &wl_callback::WlCallback,
        event: wl_callback::Event,
        data: &FrameCallbackDispatchData,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        if matches!(
            event,
            wl_callback::Event::Done { .. }
        ) {
            let pending = state.windowshader.windowshader_pending_callbacks.remove(&data.registry_name);
            if let Some(pending) = pending {
                if pending.requested_at.elapsed() >= Duration::from_millis(250) || pending.sequence != data.sequence {
                crate::logger::warning(
                    &crate::locate_paths::runtime_log_path(),
                    &format!("[WINDOWSHADER_CALLBACK_DIAG] Callback acknowledged: target={} seq={} age_ms={} swap_completed={} flushed={} matching_sequence={}",
                        data.registry_name, data.sequence, pending.requested_at.elapsed().as_millis(),
                        pending.swap_completed, pending.flushed, pending.sequence == data.sequence),
                );
                }
            } else if data.sequence != 0 {
                crate::logger::warning(
                    &crate::locate_paths::runtime_log_path(),
                    &format!("[WINDOWSHADER_CALLBACK_DIAG] Callback without pending record: target={} seq={}", data.registry_name, data.sequence),
                );
            }
            state.windowshader.frame_ready.insert(
                data.registry_name,
                true,
            );
        }
    }
}






/// New dispatch queue for the Windowshader backend. The existing connector
/// remains active until the shared renderer accepts this state type.
pub(crate) fn connect_and_bind_windowshader_dispatch()
    -> Result<(Connection, wayland_client::EventQueue<WindowshaderDispatchState>, WindowshaderDispatchState), String>
{
    let connection = Connection::connect_to_env()
        .map_err(|e| format!("Unable to connect Windowshader to Wayland compositor: {e}"))?;
    let mut queue = connection.new_event_queue::<WindowshaderDispatchState>();
    let handle = queue.handle();
    let _registry = connection.display().get_registry(&handle, ());
    let mut state = WindowshaderDispatchState::default();
    queue.roundtrip(&mut state)
        .map_err(|e| format!("Unable to read Windowshader Wayland capabilities: {e}"))?;
    if state.compositor.is_none() { return Err("Missing wl_compositor".into()); }
    if state.xdg_wm_base.is_none() { return Err("Missing xdg_wm_base".into()); }
    Ok((connection, queue, state))
}


// Stage 2D-2A Step 2: Windowshader owns compositor callback pacing.
// A missing callback record means that the surface has not yet requested a
// frame; preserve the existing first-frame-ready default.
pub(crate) fn all_frames_ready(
    state: &WindowshaderWaylandState,
    targets: &[NativeWallpaperTarget],
) -> bool {
    targets.iter().all(|target| {
        state.frame_ready
            .get(&target.info.registry_name)
            .copied()
            .unwrap_or(true)
    })
}

// Must be called on the owning Wayland event-queue thread before committing
// the surface. The matching callback is acknowledged by Windowshader dispatch.
pub(crate) fn request_next_frame<S>(
    state: &mut WindowshaderWaylandState,
    surface: &wayland_client::protocol::wl_surface::WlSurface,
    registry_name: u32,
    queue: &QueueHandle<S>,
) where
    S: Dispatch<wl_callback::WlCallback, FrameCallbackDispatchData> + 'static,
{
    state.windowshader_next_callback_sequence += 1;
    let sequence = state.windowshader_next_callback_sequence;
    surface.frame(queue, FrameCallbackDispatchData { registry_name, sequence });
    state.windowshader_pending_callbacks.insert(
        registry_name,
        WindowshaderCallbackDiagnostic {
            sequence,
            requested_at: Instant::now(),
            swap_completed: false,
            flushed: false,
        },
    );
    state.frame_ready.insert(registry_name, false);
}


// Stage 2D-2A Step 3: Windowshader-owned frame-deadline scheduling.
// Called only after a frame is presented; compositor callback readiness is
// checked separately before rendering. This deliberately preserves the
// previous shared-loop timing algorithm, including its overrun behavior.
// Stage 2D-2C Step 3: Windowshader owns deadline resets after shader changes,
// screensaver pauses, and compositor callback suppression.
pub(crate) fn reset_frame_deadline(deadline: &mut Instant) {
    *deadline = Instant::now();
}

pub(crate) fn pace_next_frame(deadline: &mut Instant, frame_duration: Duration) {
    *deadline += frame_duration;
    let now = Instant::now();
    if *deadline > now {
        std::thread::sleep(*deadline - now);
    } else {
        *deadline = now;
    }
}


// Stage 2D-2A Step 4: Windowshader owns the accounting boundaries
// between screensaver pauses and compositor-suppressed frame callbacks.
// Keep these helpers free of sleeps, global state, and timer creation.
pub(crate) fn callback_clock_adjustments(
    callback_started: Instant,
    callback_ended: Instant,
    last_pause: Option<(Instant, Instant)>,
    shader_anchor: Instant,
    animation_anchor: Instant,
) -> (Duration, Duration, Duration) {
    let callback_wait = callback_ended.duration_since(callback_started);
    let pause_overlap = last_pause.map(|(pause_start, pause_end)| {
        let overlap_start = callback_started.max(pause_start);
        let overlap_end = callback_ended.min(pause_end);
        overlap_end.saturating_duration_since(overlap_start)
    }).unwrap_or_default();
    let effective_wait = callback_wait.saturating_sub(pause_overlap);
    // Never advance an anchor beyond the present instant.
    let rotation_shift = effective_wait.min(shader_anchor.elapsed());
    let animation_shift = effective_wait.min(animation_anchor.elapsed());
    (pause_overlap, rotation_shift, animation_shift)
}



// Stage 2D-2C Step 14: bounded callback-stall reporting clock.
#[derive(Debug, Default)]
pub(crate) struct WindowshaderCallbackStallClock {
    pub(crate) warned: bool,
    pub(crate) last_report: Option<Instant>,
}
impl WindowshaderCallbackStallClock {
    pub(crate) fn initial_warning_due(&mut self, since: Option<Instant>) -> bool {
        if !self.warned && since.is_some_and(|at| at.elapsed() >= Duration::from_millis(1500)) {
            self.warned = true;
            return true;
        }
        false
    }
    pub(crate) fn recurring_report_due(&mut self, since: Option<Instant>) -> bool {
        if since.is_some_and(|at| at.elapsed() >= Duration::from_secs(10))
            && self.last_report.is_none_or(|at| at.elapsed() >= Duration::from_secs(10)) {
            self.last_report = Some(Instant::now());
            return true;
        }
        false
    }
    pub(crate) fn recovered(&mut self) -> bool {
        let warned = self.warned;
        self.warned = false;
        self.last_report = None;
        warned
    }
}

// Stage 2D-2A Step 4B: Windowshader-only rotation accounting and diagnostics.
// This state belongs to the Windowshader rendering invocation, not the
// Wallpaper renderer or a global/static clock. The shared render loop still
// owns the live shader-switch Instant until the loop is separated.
#[derive(Debug)]
pub(crate) struct WindowshaderRotationClockState {
    pub(crate) last_shader_switch: Instant,
    pub(crate) last_windowed_pause_interval: Option<(Instant, Instant)>,
    pub(crate) rotation_clock_pause_shifts: u64,
    pub(crate) rotation_clock_callback_shifts: u64,
    pub(crate) rotation_clock_callback_shift_ms: u128,
    pub(crate) rotation_clock_callback_short_shifts: u64,
    pub(crate) rotation_clock_last_shift_reason: &'static str,
    pub(crate) rotation_clock_last_shift_ms: u128,
    pub(crate) windowshader_rotation_attempts: u64,
    pub(crate) windowshader_rotation_successes: u64,
    pub(crate) windowshader_rotation_failures: u64,
    pub(crate) windowshader_rotation_block_reported: bool,
}

impl Default for WindowshaderRotationClockState {
    fn default() -> Self {
        Self {
            last_shader_switch: Instant::now(),
            last_windowed_pause_interval: None,
            rotation_clock_pause_shifts: 0,
            rotation_clock_callback_shifts: 0,
            rotation_clock_callback_shift_ms: 0,
            rotation_clock_callback_short_shifts: 0,
            rotation_clock_last_shift_reason: "none",
            rotation_clock_last_shift_ms: 0,
            windowshader_rotation_attempts: 0,
            windowshader_rotation_successes: 0,
            windowshader_rotation_failures: 0,
            windowshader_rotation_block_reported: false,
        }
    }
}

impl WindowshaderRotationClockState {
    pub(crate) fn new(initial_switch: Instant) -> Self {
        Self { last_shader_switch: initial_switch, ..Self::default() }
    }

    // Step 16B: Windowshader owns the decision whether its rotation clock is due.
    // No compositor callback is synthesized and no clock is advanced here.
    pub(crate) fn rotation_due(&self, interval: Option<Duration>) -> bool {
        interval.is_some_and(|interval| self.last_shader_switch.elapsed() >= interval)
    }

    // A blocked rotation is reported once until the compositor allows another frame.
    // This is diagnostic only; the compositor callback remains authoritative.
    pub(crate) fn report_blocked_rotation_if_due(&mut self, interval: Option<Duration>) {
        if !self.windowshader_rotation_block_reported && self.rotation_due(interval) {
            self.windowshader_rotation_block_reported = true;
            crate::logger::warning(
                &crate::locate_paths::runtime_log_path(),
                "[WINDOWSHADER_ROTATION_DIAG] Rotation deadline reached while awaiting compositor frame callback; no rotation attempted",
            );
        }
    }

    pub(crate) fn clear_blocked_rotation_report(&mut self) {
        self.windowshader_rotation_block_reported = false;
    }

    // Step 16C-1: Windowshader owns the presentation-eligibility transition
    // for rotation reporting. The compositor callback remains authoritative;
    // neither method grants frame readiness or changes rotation timestamps.
    pub(crate) fn on_presentation_blocked(&mut self, interval: Option<Duration>) {
        self.report_blocked_rotation_if_due(interval);
    }

    pub(crate) fn on_presentation_eligible(&mut self) {
        self.clear_blocked_rotation_report();
    }

    // Stage 2D-2C Step 14: pause-clock accounting belongs to Windowshader.
    pub(crate) fn record_pause_shift(&mut self, duration: Duration) {
        let before_ms = self.last_shader_switch.elapsed().as_millis();
        self.rotation_clock_pause_shifts = self.rotation_clock_pause_shifts.wrapping_add(1);
        self.rotation_clock_last_shift_reason = "screensaver_pause";
        self.rotation_clock_last_shift_ms = duration.as_millis();
        crate::logger::warning(&crate::locate_paths::runtime_log_path(),
            &format!("[WINDOWSHADER_CLOCK_DIAG] reason=screensaver_pause shift_ms={} elapsed_before_ms={} elapsed_after_expected_ms={} pause_shift_count={}",
                duration.as_millis(), before_ms, before_ms.saturating_sub(duration.as_millis()), self.rotation_clock_pause_shifts));
    }

    // Record callback exclusion using the same thresholds and diagnostics as before.
    pub(crate) fn record_callback_shift(&mut self, callback_wait: Duration, pause_overlap: Duration,
        rotation_shift: Duration,
        frame_ready: bool, paused: bool, activated: bool) {
        let before_ms = self.last_shader_switch.elapsed().as_millis();
        self.rotation_clock_callback_shifts = self.rotation_clock_callback_shifts.wrapping_add(1);
        self.rotation_clock_callback_shift_ms += rotation_shift.as_millis();
        if callback_wait < Duration::from_secs(2) {
            self.rotation_clock_callback_short_shifts = self.rotation_clock_callback_short_shifts.wrapping_add(1);
        }
        self.rotation_clock_last_shift_reason = "callback_wait";
        self.rotation_clock_last_shift_ms = rotation_shift.as_millis();
        if self.rotation_clock_callback_shifts <= 5 || self.rotation_clock_callback_shifts % 100 == 0 || callback_wait >= Duration::from_secs(2) {
            crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                &format!("[WINDOWSHADER_CLOCK_DIAG] reason=callback_wait raw_ms={} pause_overlap_ms={} shift_ms={} elapsed_before_ms={} elapsed_after_expected_ms={} callback_shifts={} short_under_2s={} cumulative_shift_ms={} frame_ready={} paused={} activated={}",
                    callback_wait.as_millis(), pause_overlap.as_millis(), rotation_shift.as_millis(), before_ms,
                    before_ms.saturating_sub(rotation_shift.as_millis()), self.rotation_clock_callback_shifts,
                    self.rotation_clock_callback_short_shifts, self.rotation_clock_callback_shift_ms,
                    frame_ready, paused, activated));
        }
    }
}

// Stage 2D-2C Step 16A: Windowshader-owned presentation diagnostics.
// This state is observational only. Never use it to synthesize callbacks,
// advance a frame deadline, or decide whether a frame may be presented.
pub(crate) struct WindowshaderPresentationDiagnostics {
    pub(crate) last_successful_swap: Instant,
    pub(crate) presentation_watchdog_last_report: Option<Instant>,
    pub(crate) presentation_watchdog_loop_ticks: u64,
    pub(crate) presentation_watchdog_last_ticks: u64,
    pub(crate) presentation_watchdog_swaps: u64,
    pub(crate) presentation_watchdog_last_swaps: u64,
    pub(crate) windowshader_summary_at: Instant,
    pub(crate) windowshader_last_configure_count: u64,
}

impl WindowshaderPresentationDiagnostics {
    pub(crate) fn new(initial_configure_count: u64) -> Self {
        let now = Instant::now();
        Self {
            last_successful_swap: now,
            presentation_watchdog_last_report: None,
            presentation_watchdog_loop_ticks: 0,
            presentation_watchdog_last_ticks: 0,
            presentation_watchdog_swaps: 0,
            presentation_watchdog_last_swaps: 0,
            windowshader_summary_at: now,
            windowshader_last_configure_count: initial_configure_count,
        }
    }
}


// A pipeline-owned assessment of compositor callback eligibility. This is
// bookkeeping only: wl_surface.frame Done remains the sole readiness authority.
pub(crate) enum WindowshaderFrameEligibility {
    Blocked {
        initial_warning: bool,
        recurring_warning: bool,
        post_pause_warning: bool,
    },
    Ready {
        callback_recovered: bool,
        inactive_since: Option<Instant>,
    },
}

// Windowshader-owned mutable frame-loop bookkeeping. The shared GL executor
// borrows this state during the staged extraction; it does not own the clocks.
// Frame eligibility and Wayland callback pacing remain unchanged.
pub(crate) struct WindowshaderFrameLoopState {
    pub(crate) callback_stall: WindowshaderCallbackStallClock,
    pub(crate) rotation: WindowshaderRotationClockState,
    pub(crate) presentation: WindowshaderPresentationDiagnostics,
    pub(crate) inactive_since: Option<Instant>,
    pub(crate) resume_presentation_pending: bool,
    pub(crate) resume_wait_reported: bool,
}

impl WindowshaderFrameLoopState {
    pub(crate) fn new(shader_switch: Instant, configure_count: u64) -> Self {
        Self {
            callback_stall: WindowshaderCallbackStallClock::default(),
            rotation: WindowshaderRotationClockState::new(shader_switch),
            presentation: WindowshaderPresentationDiagnostics::new(configure_count),
            inactive_since: None,
            resume_presentation_pending: false,
            resume_wait_reported: false,
        }
    }

    // Screensaver pause bookkeeping is Windowshader-owned. The shared executor
    // continues to manage the Wallpaper clock and the animation time anchor.
    pub(crate) fn on_pause_entering(&mut self) {
        self.resume_presentation_pending = true;
        self.resume_wait_reported = false;
    }

    // Preserve the original diagnostic ordering: record the pause shift before
    // advancing the Windowshader rotation anchor. Do not change callback readiness.
    pub(crate) fn on_pause_ended(
        &mut self,
        pause_started: Instant,
        pause_ended: Instant,
    ) {
        let duration = pause_ended.duration_since(pause_started);
        self.rotation.last_windowed_pause_interval = Some((pause_started, pause_ended));
        self.rotation.record_pause_shift(duration);
        self.rotation.last_shader_switch += duration;
    }

    pub(crate) fn assess_frame_eligibility(
        &mut self,
        compositor_ready: bool,
        shader_interval: Option<Duration>,
        windowed: bool,
    ) -> WindowshaderFrameEligibility {
        if compositor_ready {
            let (callback_recovered, inactive_since) = self.on_callback_ready(windowed);
            WindowshaderFrameEligibility::Ready { callback_recovered, inactive_since }
        } else {
            let (initial_warning, recurring_warning, post_pause_warning) =
                self.on_callback_blocked(shader_interval, windowed);
            WindowshaderFrameEligibility::Blocked {
                initial_warning, recurring_warning, post_pause_warning,
            }
        }
    }

    // A missing compositor frame callback is authoritative: record the wait,
    // but never manufacture frame readiness or advance the rotation clock.
    pub(crate) fn on_callback_blocked(&mut self, interval: Option<Duration>, windowed: bool) -> (bool, bool, bool) {
        if windowed {
            self.rotation.on_presentation_blocked(interval);
        }
        if self.inactive_since.is_none() {
            self.inactive_since = Some(Instant::now());
        }
        let initial = self.callback_stall.initial_warning_due(self.inactive_since);
        let recurring = self.callback_stall.recurring_report_due(self.inactive_since);
        let post_pause = self.resume_presentation_pending && !self.resume_wait_reported;
        if post_pause {
            self.resume_wait_reported = true;
        }
        (initial, recurring, post_pause)
    }

    // Return the beginning of a completed callback wait for the caller's
    // existing animation/FPS clock adjustments. No timing policy changes.
    pub(crate) fn on_callback_ready(&mut self, windowed: bool) -> (bool, Option<Instant>) {
        if windowed {
            self.rotation.on_presentation_eligible();
        }
        (self.callback_stall.recovered(), self.inactive_since.take())
    }
}
