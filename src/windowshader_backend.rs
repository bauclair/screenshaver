//! Windowshader-owned backend negotiation.
//! Native Wayland/X11 rendering implementations remain shared at this stage.
//! Only windowed presentation is dispatched by the Windowshader supervisor.

use crate::wallpaper_backend::WallpaperBackend;
use crate::define_wallpaper::WallpaperRuntime;
use crate::manage_shader::ShaderManager;
use crate::manage_wallpaper_runtime::WallpaperRuntimeControl;
use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

struct WaylandWindowshaderBackend {
    capabilities: crate::wayland_wallpaper::WallpaperWaylandCapabilities,
}

impl WaylandWindowshaderBackend {
    fn new() -> Result<Self, String> {
        let capabilities = crate::wayland_wallpaper::probe_windowshader_capabilities()?;
        Ok(Self { capabilities })
    }
}

impl WallpaperBackend for WaylandWindowshaderBackend {
    fn backend_name(&self) -> &'static str { "wayland" }

    fn report_capabilities(&self) {
        println!("Wayland Windowshader capabilities available:");
        println!("    wl_compositor: version {}", self.capabilities.compositor_version.unwrap_or(0));
        println!("    xdg_wm_base: verified");
        println!("    Outputs detected: {}", self.capabilities.output_count);
    }

    fn run(self: Box<Self>, shader_manager: ShaderManager, wallpaper_directory: &Path,
           shader_interval: Option<Duration>, runtime: &WallpaperRuntime,
           running: Arc<AtomicBool>, control: WallpaperRuntimeControl) -> Result<(), String> {
        if runtime.display_format != crate::manage_configuration::WallpaperDisplayFormat::Windowed {
            return Err("Wayland Windowshader backend requires Windowed display format".into());
        }
        crate::wayland_wallpaper::run_egl_windowed_surface(
            shader_manager, wallpaper_directory, shader_interval, runtime, running, control)
    }
}


pub(crate) fn create_backend() -> Result<Box<dyn WallpaperBackend>, String> {
    // Preserve the established Wayland-first, X11-fallback negotiation order.
    match WaylandWindowshaderBackend::new() {
        Ok(backend) => {
            println!("Selected native [WAYLAND] Windowshader backend");
            Ok(Box::new(backend))
        }
        Err(wayland_error) => {
            println!("Native Wayland Windowshader backend unavailable: {wayland_error}");
            match crate::x11_windowshader::X11WindowshaderBackend::new() {
                Ok(backend) => {
                    println!("Selected native [X11] Windowshader backend");
                    Ok(Box::new(backend))
                }
                Err(x11_error) => Err(format!(
                    "No compatible Windowshader backend is available. Wayland: {wayland_error}. X11: {x11_error}."
                )),
            }
        }
    }
}
