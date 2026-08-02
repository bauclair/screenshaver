use std::path::Path;
use std::sync::{
    atomic::AtomicBool,
    Arc,
};
use std::time::Duration;

use crate::define_wallpaper::WallpaperRuntime;
use crate::manage_shader::ShaderManager;
use crate::manage_wallpaper_runtime::WallpaperRuntimeControl;

/// Platform-specific wallpaper renderer.
///
/// A wallpaper backend owns platform capability probing, capability reporting,
/// and the native surface/rendering implementation. Wallpaper configuration,
/// shader enumeration, and ShaderManager construction remain the responsibility
/// of manage_wallpaper.rs.
pub trait WallpaperBackend {
    fn backend_name(
        &self,
    ) -> &'static str;

    fn report_capabilities(
        &self,
    );

    fn run(
        self: Box<Self>,
        shader_manager: ShaderManager,
        wallpaper_directory: &Path,
        shader_interval: Option<Duration>,
        runtime: &WallpaperRuntime,
        running: Arc<AtomicBool>,
        control: WallpaperRuntimeControl,
    ) -> Result<(), String>;
}

/// Select the first usable native wallpaper backend.
///
/// Wayland remains the preferred backend. If native Wayland wallpaper
/// capability probing fails, Screenshaver falls back to X11.
pub fn create_backend(
) -> Result<Box<dyn WallpaperBackend>, String> {
    match WaylandWallpaperBackend::new() {
        Ok(backend) => {
            announce_backend(
                &backend
            );

            Ok(
                Box::new(backend)
            )
        }

        Err(wayland_error) => {
            println!(
                "Native Wayland wallpaper backend is unavailable: {}",
                wayland_error,
            );

            println!();

            match crate::x11_wallpaper::X11WallpaperBackend::new() {
                Ok(backend) => {
                    announce_backend(
                        &backend
                    );

                    Ok(
                        Box::new(backend)
                    )
                }

                Err(x11_error) => {
                    Err(
                        format!(
                            "No compatible wallpaper backend is available. Wayland: {}. X11: {}.",
                            wayland_error,
                            x11_error,
                        )
                    )
                }
            }
        }
    }
}

fn announce_backend(
    backend: &dyn WallpaperBackend,
) {
    println!(
        "Selected native [{}] wallpaper backend",
        backend.backend_name().to_uppercase()
    );

    println!();
}

struct WaylandWallpaperBackend {
    capabilities: crate::wayland_wallpaper::WallpaperWaylandCapabilities,
}

impl WaylandWallpaperBackend {
    fn new(
    ) -> Result<Self, String> {
        println!(
            "Probing native Wayland wallpaper capabilities..."
        );

        let capabilities =
            crate::wayland_wallpaper::probe_capabilities()?;

        Ok(
            Self {
                capabilities,
            }
        )
    }
}

impl WallpaperBackend for WaylandWallpaperBackend {
    fn backend_name(
        &self,
    ) -> &'static str {
        "wayland"
    }

    fn report_capabilities(
        &self,
    ) {
        println!(
            "Wayland wallpaper capabilities are available:"
        );

        println!(
            "    wl_compositor: version {}",
            self.capabilities
                .compositor_version
                .unwrap_or(0)
        );

        println!(
            "    zwlr_layer_shell_v1: version {}",
            self.capabilities
                .layer_shell_version
                .unwrap_or(0)
        );

        println!(
            "    Wallpaper targets: {}",
            self.capabilities.targets.len()
        );

        for (
            index,
            output,
        ) in self.capabilities
            .targets
            .iter()
            .enumerate()
        {
            println!();

            println!(
                "    Target {}:",
                index + 1
            );

            println!(
                "        Registry name: {}",
                output.registry_name
            );

            println!(
                "        Connector: {}",
                output
                    .connector_name
                    .as_deref()
                    .unwrap_or("<not advertised>")
            );

            println!(
                "        Description: {}",
                output
                    .description
                    .as_deref()
                    .unwrap_or("<not advertised>")
            );

            println!(
                "        Make: {}",
                output
                    .make
                    .as_deref()
                    .unwrap_or("<not advertised>")
            );

            println!(
                "        Model: {}",
                output
                    .model
                    .as_deref()
                    .unwrap_or("<not advertised>")
            );

            println!(
                "        Position: {},{}",
                output.logical_x,
                output.logical_y
            );

            println!(
                "        Current mode: {}x{} @ {:.3} Hz",
                output.mode_width,
                output.mode_height,
                output.refresh_millihertz as f64 / 1000.0
            );

            println!(
                "        Physical size: {}x{} mm",
                output.physical_width_mm,
                output.physical_height_mm
            );

            println!(
                "        Scale: {}",
                output.scale
            );

            println!(
                "        Transform: {}",
                output
                    .transform
                    .as_deref()
                    .unwrap_or("<not advertised>")
            );

            println!(
                "        Metadata complete: {}",
                output.complete
            );
        }

        println!();
    }

    fn run(
        self: Box<Self>,
        shader_manager: ShaderManager,
        wallpaper_directory: &Path,
        shader_interval: Option<Duration>,
        runtime: &WallpaperRuntime,
        running: Arc<AtomicBool>,
        control: WallpaperRuntimeControl,
    ) -> Result<(), String> {
        println!(
            "Starting native Wayland/EGL mirror wallpaper renderer..."
        );

        crate::wayland_wallpaper::run_egl_background_surface(
            shader_manager,
            wallpaper_directory,
            shader_interval,
            runtime,
            running,
            control,
        )?;

        println!();

        println!(
            "Native Wayland/EGL mirror wallpaper renderer ended cleanly."
        );

        Ok(())
    }
}

