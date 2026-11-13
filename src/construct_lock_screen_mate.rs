//! Per-user registration of the native MATE screensaver theme.
//!
//! The approved executable must be installed separately by the package or
//! installer. This module never elevates privileges, changes MATE's selected
//! theme, or replaces MATE's authentication mechanism.

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const MATE_SCREENSAVER: &str = "/usr/bin/mate-screensaver";
const TRUSTED_PRESENTER: &str = "/usr/libexec/mate-screensaver/screenshaver";
const THEME_FILENAME: &str = "screenshaver-mate.desktop";

#[derive(Debug, Clone)]
pub struct MateLockConfigurationStatus {
    pub mate_screensaver_available: bool,
    pub trusted_presenter_installed: bool,
    pub saver_desktop_registered: bool,
}

impl MateLockConfigurationStatus {
    pub fn ready_for_runtime(&self) -> bool {
        self.mate_screensaver_available
            && self.trusted_presenter_installed
            && self.saver_desktop_registered
    }
}

// The privileged launcher is installed by packaging, not by this module.
// Verify the two essential production contract lines before advertising readiness.
fn trusted_presenter_ready() -> bool {
    let path = Path::new(TRUSTED_PRESENTER);
    if !is_executable(path) {
        return false;
    }
    let Ok(contents) = fs::read_to_string(path) else {
        return false;
    };
    contents.lines().any(|line| line.trim() == "export SCREENSHAVER_MATE_PRESENTATION=1")
        && contents.lines().any(|line| line.trim() == "exec /usr/local/bin/screenshaver")
}

fn theme_entry() -> String {
    format!(
        "[Desktop Entry]\n\
Name=Screenshaver\n\
Comment=Screenshaver shader presentation for the native MATE screensaver\n\
Exec={TRUSTED_PRESENTER}\n\
TryExec={TRUSTED_PRESENTER}\n\
StartupNotify=false\n\
Terminal=false\n\
Type=Application\n\
Categories=Screensaver;\n\
OnlyShowIn=MATE;\n"
    )
}

fn theme_path() -> Result<PathBuf, String> {
    let data_home = match std::env::var_os("XDG_DATA_HOME") {
        Some(value) => {
            let path = PathBuf::from(value);
            if !path.is_absolute() {
                return Err("XDG_DATA_HOME must be an absolute path".into());
            }
            path
        }
        None => {
            let home = PathBuf::from(std::env::var_os("HOME")
                .ok_or("HOME is unset")?);
            if !home.is_absolute() {
                return Err("HOME must be an absolute path".into());
            }
            home.join(".local/share")
        }
    };
    Ok(data_home.join("applications/screensavers").join(THEME_FILENAME))
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

pub fn inspect() -> Result<MateLockConfigurationStatus, String> {
    let theme = theme_path()?;
    Ok(MateLockConfigurationStatus {
        mate_screensaver_available: is_executable(Path::new(MATE_SCREENSAVER)),
        trusted_presenter_installed: trusted_presenter_ready(),
        saver_desktop_registered: fs::read_to_string(&theme)
            .map(|contents| contents == theme_entry())
            .unwrap_or(false),
    })
}

/// Register the theme without selecting it. A separate runtime integration
/// must manage temporary selection and restoration before production use.
pub fn configure_user() -> Result<MateLockConfigurationStatus, String> {
    if !is_executable(Path::new(MATE_SCREENSAVER)) {
        return Err(format!("MATE Screensaver is unavailable at {MATE_SCREENSAVER}"));
    }
    if !trusted_presenter_ready() {
        return Err(format!(
            "MATE trusted presenter at {TRUSTED_PRESENTER} is missing, not executable, or lacks the required production launcher lines; install the launcher through the package or installer first"
        ));
    }

    let destination = theme_path()?;
    let expected = theme_entry();
    match fs::read_to_string(&destination) {
        Ok(existing) if existing == expected => return inspect(),
        Ok(_) => return Err(format!(
            "Refusing to overwrite an existing nonmatching MATE theme: {}",
            destination.display()
        )),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(format!("Unable to inspect {}: {error}", destination.display()));
        }
        Err(_) => {}
    }

    let parent = destination.parent().ok_or("Invalid MATE theme destination")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Unable to create {}: {error}", parent.display()))?;

    // create_new prevents overwriting a theme created by another process.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|error| format!("Unable to create {}: {error}", destination.display()))?;
    if let Err(error) = file.write_all(expected.as_bytes()).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(&destination);
        return Err(format!("Unable to write {}: {error}", destination.display()));
    }

    let status = inspect()?;
    if !status.ready_for_runtime() {
        return Err(format!("MATE theme registration verification failed: {status:?}"));
    }
    Ok(status)
}
