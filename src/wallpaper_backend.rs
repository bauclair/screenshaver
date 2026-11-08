use std::path::{Path, PathBuf};
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

/// Read-only observations. These are not ownership grants or handoff states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlasmaOwnershipObservation {
    NotApplicable,
    NotAssigned,
    AssignedNotReady,
    NativeReadyObserved(u32),
    InspectionFailed(String),
}

/// No filesystem writes, renderer selection, or lifecycle changes.
/// Only full-screen wallpaper (not windowshader/windowed) is eligible.
pub(crate) fn assess_plasma_wallpaper(
    full_screen_wallpaper: bool,
) -> PlasmaOwnershipObservation {
    if !full_screen_wallpaper {
        return PlasmaOwnershipObservation::NotApplicable;
    }
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    if !session.eq_ignore_ascii_case("wayland")
        || !desktop.split(':').any(|part| part.eq_ignore_ascii_case("KDE"))
    {
        return PlasmaOwnershipObservation::NotApplicable;
    }
    match probe_plasma_wallpaper_assignment() {
        Ok(false) => PlasmaOwnershipObservation::NotAssigned,
        Err(error) => PlasmaOwnershipObservation::InspectionFailed(error),
        Ok(true) => match probe_plasma_wallpaper_readiness() {
            Ok(Some(pid)) => PlasmaOwnershipObservation::NativeReadyObserved(pid),
            Ok(None) => PlasmaOwnershipObservation::AssignedNotReady,
            Err(error) => PlasmaOwnershipObservation::InspectionFailed(error),
        },
    }
}

pub(crate) fn report_plasma_wallpaper_observation(observation: &PlasmaOwnershipObservation) {
    match observation {
        PlasmaOwnershipObservation::NotApplicable =>
            println!("[PLASMA WALLPAPER OBSERVATION] NotApplicable"),
        PlasmaOwnershipObservation::NotAssigned =>
            println!("[PLASMA WALLPAPER OBSERVATION] NotAssigned"),
        PlasmaOwnershipObservation::AssignedNotReady =>
            println!("[PLASMA WALLPAPER OBSERVATION] AssignedNotReady"),
        PlasmaOwnershipObservation::NativeReadyObserved(pid) =>
            println!("[PLASMA WALLPAPER OBSERVATION] NativeReadyObserved (plasmashell PID {pid})"),
        PlasmaOwnershipObservation::InspectionFailed(error) =>
            eprintln!("[PLASMA WALLPAPER OBSERVATION] InspectionFailed: {error}"),
    }
    println!("[PLASMA WALLPAPER OBSERVATION] Diagnostic only; native ownership handoff disabled");
}

/// Read-only probe for the saved Plasma desktop wallpaper assignment.
/// This is *not* proof of a live renderer or permission to switch backends.
/// A later phase must establish a live handshake with plasmashell.
pub(crate) fn probe_plasma_wallpaper_assignment() -> Result<bool, String> {
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    if !session_type.eq_ignore_ascii_case("wayland")
        || !desktop.split(':').any(|part| part.eq_ignore_ascii_case("KDE"))
    {
        return Ok(false);
    }

    let config_dir = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => {
            let home = std::env::var_os("HOME")
                .ok_or("HOME and XDG_CONFIG_HOME are unset")?;
            PathBuf::from(home).join(".config")
        }
    };
    let path = config_dir.join("plasma-org.kde.plasma.desktop-appletsrc");
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("Cannot read {}: {error}", path.display())),
    };

    let mut is_desktop_containment = false;
    let mut is_folder = false;
    let mut is_screenshaver = false;
    for raw_line in contents.lines().chain(std::iter::once("[END]")) {
        let line = raw_line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            if is_desktop_containment && is_folder && is_screenshaver {
                return Ok(true);
            }
            // Only direct containment sections, never nested applets/panels.
            is_desktop_containment = line.starts_with("[Containments][")
                && line.ends_with(']')
                && line.matches('[').count() == 2;
            is_folder = false;
            is_screenshaver = false;
        } else if is_desktop_containment {
            if let Some(value) = line.strip_prefix("plugin=") {
                is_folder = value.trim() == "org.kde.plasma.folder";
            } else if let Some(value) = line.strip_prefix("wallpaperplugin=") {
                is_screenshaver = value.trim() == "org.screenshaver.wallpaper";
            }
        }
    }
    Ok(false)
}

/// Diagnostic-only process-instance check. A matching marker is evidence of one
/// successful render call, not proof of current frame delivery or ownership.
/// No markers are modified and backend selection is unchanged.
fn probe_plasma_wallpaper_readiness() -> Result<Option<u32>, String> {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or("XDG_RUNTIME_DIR is not set")?;

    let entries = std::fs::read_dir(&runtime_dir)
        .map_err(|error| format!("Cannot inspect {}: {error}", runtime_dir.display()))?;

    for entry in entries {
        let entry = entry.map_err(|error| format!("Cannot enumerate runtime markers: {error}"))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(rest) = name.strip_prefix("screenshaver-plasma-frame-") else { continue };
        let Some(rest) = rest.strip_suffix(".ready") else { continue };
        let Some((pid_text, node_id)) = rest.split_once('-') else { continue };
        if node_id.is_empty() || !node_id.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        let Ok(pid) = pid_text.parse::<u32>() else { continue };
        if pid == 0 { continue; }

        // Check the actual process identity, not just PID existence. This also
        // prevents an orphaned marker from a terminated Plasma instance from
        // being accepted as current readiness.
        let cmdline = match std::fs::read(format!("/proc/{pid}/cmdline")) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let executable = cmdline.split(|byte| *byte == 0).next().unwrap_or_default();
        let executable = String::from_utf8_lossy(executable);
        let executable_name = std::path::Path::new(executable.as_ref())
            .file_name().and_then(|name| name.to_str()).unwrap_or("");
        if executable_name != "plasmashell" && executable_name != ".plasmashell-wrapped" {
            continue;
        }

        let contents = match std::fs::read_to_string(entry.path()) {
            Ok(contents) => contents,
            Err(_) => continue,
        };
        // The marker must identify this particular Plasma process instance,
        // not merely a PID that might have been reused after a crash.
        let stat = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            Ok(stat) => stat,
            Err(_) => continue,
        };
        if marker_matches_process_instance(pid_text, &contents, &stat) {
            return Ok(Some(pid));
        }
    }
    Ok(None)
}

/// Validate readiness against the exact Linux process start time. This is
/// evidence of a past successful frame, never an ownership grant.
fn marker_matches_process_instance(pid_text: &str, marker: &str, stat: &str) -> bool {
    let mut fields = marker.split_whitespace();
    let (Some(marker_pid), Some(marker_start), None) =
        (fields.next(), fields.next(), fields.next()) else { return false };
    if marker_pid != pid_text || marker_start.is_empty()
        || !marker_start.bytes().all(|b| b.is_ascii_digit())
    {
        return false;
    }
    // /proc/PID/stat field 2 (comm) can contain spaces and parentheses.
    // Field 22 (starttime) is index 19 after field 2's closing ')'.
    let Some((_, after_comm)) = stat.rsplit_once(')') else { return false };
    after_comm.split_whitespace().nth(19) == Some(marker_start)
}

#[cfg(test)]
mod plasma_observation_tests {
    use super::*;

    fn stat_with_start(start: &str) -> String {
        let mut fields = vec!["0".to_owned(); 19];
        fields.push(start.to_owned());
        format!("123 (plasma shell) {}", fields.join(" "))
    }

    #[test]
    fn matching_process_instance_is_observed() {
        assert!(marker_matches_process_instance("123", "123 456\n", &stat_with_start("456")));
    }

    #[test]
    fn stale_pid_reuse_marker_is_rejected() {
        assert!(!marker_matches_process_instance("123", "123 456\n", &stat_with_start("789")));
    }

    #[test]
    fn wrong_pid_and_malformed_markers_are_rejected() {
        let stat = stat_with_start("456");
        for marker in ["124 456", "123", "123 abc", "123 456 extra", "", "123 456  \n extra"] {
            assert!(!marker_matches_process_instance("123", marker, &stat), "{marker:?}");
        }
    }

    #[test]
    fn malformed_proc_stat_is_rejected() {
        assert!(!marker_matches_process_instance("123", "123 456", "invalid stat"));
    }

    #[test]
    fn observations_do_not_encode_ownership() {
        assert_ne!(PlasmaOwnershipObservation::NativeReadyObserved(123),
                   PlasmaOwnershipObservation::AssignedNotReady);
        // Backend factory remains independent of Plasma observations.
        // No renderer selection or filesystem mutation is exercised here.
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

pub(crate) struct WaylandWallpaperBackend {
    capabilities: crate::wayland_wallpaper::WallpaperWaylandCapabilities,
}

impl WaylandWallpaperBackend {
    pub(crate) fn new(
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
        match runtime.display_format {
            crate::manage_configuration::WallpaperDisplayFormat::FullScreen => {
                println!(
                    "Wallpaper display format: Full-screen"
                );


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
            }

            crate::manage_configuration::WallpaperDisplayFormat::Windowed => {
                println!(
                    "Wallpaper display format: Windowed"
                );


                println!(
                    "Starting native Wayland/EGL Windowed wallpaper renderer..."
                );


                crate::wayland_wallpaper::run_egl_windowed_surface(
                    shader_manager, wallpaper_directory, shader_interval,
                    runtime, running, control,
                )?;

            }
        }

        println!();

        println!(
            "Native Wayland/EGL mirror wallpaper renderer ended cleanly."
        );

        Ok(())
    }
}

