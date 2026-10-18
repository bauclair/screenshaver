//! Windowshader-owned Wayland geometry and presentation-state persistence.
//! The native XDG/EGL loop remains in wayland_wallpaper during incremental extraction.

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
use std::path::Path;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use wayland_client::Proxy;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;
use crate::wayland_wallpaper::{
    WallpaperSurfaceState, WallpaperTargetInfo, NativeWallpaperTarget,
    select_safe_wallpaper_shader, print_active_wallpaper_shader,
    connect_and_bind_windowed, configured_dimension, render_egl_wallpapers,
};

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
        connect_and_bind_windowed()?;


    state.windowed_normal_width = saved_geometry.width;
    state.windowed_normal_height = saved_geometry.height;

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
        render_egl_wallpapers(
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


    if let Some(target) = native_targets.first() {
        let position = _kwin_guard.as_ref().and_then(|guard| guard.last_position())
            .or(saved_geometry.x.zip(saved_geometry.y));
        let geometry = crate::wayland_windowshader::WindowshaderGeometry {
            x: position.map(|(x, _)| x),
            y: position.map(|(_, y)| y),
            width: if state.windowed_maximized {
                state.windowed_normal_width
            } else { target.width },
            height: if state.windowed_maximized {
                state.windowed_normal_height
            } else { target.height },
            maximized: state.windowed_maximized,
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


    render_result
}

