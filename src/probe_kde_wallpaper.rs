//! Non-invasive discovery of the experimental Plasma shared-memory wallpaper package.
//! Discovery does not imply that Plasma has selected or loaded the wallpaper.
use std::path::{Path, PathBuf};

pub const PLUGIN_ID: &str = "org.screenshaver.wallpaper.shmtest";

#[derive(Debug, Clone)]
pub struct KdeWallpaperProbe {
    pub plasma_wayland_session: bool,
    pub package_path: Option<PathBuf>,
    pub package_metadata: Option<PathBuf>,
}

impl KdeWallpaperProbe {
    pub fn package_installed(&self) -> bool { self.package_path.is_some() }
    pub fn eligible_for_activation_check(&self) -> bool {
        self.plasma_wayland_session && self.package_installed()
    }
}

pub fn probe() -> KdeWallpaperProbe {
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let desktops = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let plasma_wayland_session = session_type.eq_ignore_ascii_case("wayland")
        && desktops.split(':').any(|s| s.eq_ignore_ascii_case("KDE"));
    let mut roots = Vec::new();
    if let Some(home) = std::env::var_os("XDG_DATA_HOME") {
        if !home.is_empty() { roots.push(PathBuf::from(home)); }
    } else if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join(".local/share"));
    }
    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    roots.extend(std::env::split_paths(&data_dirs));

    for root in roots {
        let package = root.join("plasma/wallpapers").join(PLUGIN_ID);
        let metadata = package.join("metadata.json");
        let qml = package.join("contents/ui/main.qml");
        if metadata.is_file() && qml.is_file() && metadata_has_plugin_id(&metadata) {
            return KdeWallpaperProbe {
                plasma_wayland_session,
                package_path: Some(package),
                package_metadata: Some(metadata),
            };
        }
    }
    KdeWallpaperProbe { plasma_wayland_session, package_path: None, package_metadata: None }
}

fn metadata_has_plugin_id(path: &Path) -> bool {
    // This deliberately avoids adding a JSON dependency. It is a conservative
    // discovery hint, not a trusted validation of package contents.
    std::fs::read_to_string(path).ok().is_some_and(|s| {
        s.contains("\"Id\"") && s.contains(&format!("\"{}\"", PLUGIN_ID))
            && s.contains("Plasma/Wallpaper")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plugin_id_is_stable() {
        assert_eq!(PLUGIN_ID, "org.screenshaver.wallpaper.shmtest");
    }
}
