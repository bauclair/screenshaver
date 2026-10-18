//! Windowshader-owned backend negotiation.
//! Native Wayland/X11 rendering implementations remain shared at this stage.
//! Only windowed presentation is dispatched by the Windowshader supervisor.

use crate::wallpaper_backend::WallpaperBackend;

pub(crate) fn create_backend() -> Result<Box<dyn WallpaperBackend>, String> {
    // Preserve the established Wayland-first, X11-fallback negotiation order.
    match crate::wallpaper_backend::WaylandWallpaperBackend::new() {
        Ok(backend) => {
            println!("Selected native [WAYLAND] Windowshader backend");
            Ok(Box::new(backend))
        }
        Err(wayland_error) => {
            println!("Native Wayland Windowshader backend unavailable: {wayland_error}");
            match crate::x11_wallpaper::X11WallpaperBackend::new() {
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
