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
use crate::wayland_wallpaper::{eglGetProcAddress};

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


    let render_result =
        crate::wayland_wallpaper::render_mirror_frames(
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

