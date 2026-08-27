//! manage_screen_lock_kde.rs
//!
//! Installs and manages Screenshaver's KDE Plasma / KScreenLocker integration.
//!
//! KDE Plasma's Wayland screen locker is owned by KWin. KWin passes its own
//! startup environment to KScreenLocker, which launches `kscreenlocker_greet`
//! using that environment. Screenshaver therefore scopes
//! `PLASMA_DEFAULT_SHELL=org.screenshaver` to KWin only through a user-level
//! systemd drop-in for `plasma-kwin_wayland.service`.
//!
//! This module deliberately does NOT modify `plasmashellrc` or replace
//! Plasma's desktop ShellPackage. Plasma's normal desktop shell therefore
//! remains unaffected across logout/login and reboot.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::construct_lock_screen_kde;
use crate::define_lock_screen_widget::LockScreenWidgetConfig;

const SCREENSHAVER_SHELL_PACKAGE: &str = "org.screenshaver";
const KWIN_DROPIN_DIRECTORY: &str = "plasma-kwin_wayland.service.d";
const KWIN_DROPIN_FILENAME: &str = "screenshaver.conf";
const KWIN_DROPIN_CONTENTS: &str =
    "[Service]\nEnvironment=PLASMA_DEFAULT_SHELL=org.screenshaver\n";

const KDE_METADATA_JSON: &str = r#"{
    "KPackageStructure": "Plasma/Shell",
    "KPlugin": {
        "Authors": [
            {
                "Name": "Screenshaver Project"
            }
        ],
        "Description": "Screenshaver KDE lock-screen shell",
        "Id": "org.screenshaver",
        "License": "GPL-3.0-or-later",
        "Name": "Screenshaver Lock Screen",
        "Version": "1.0"
    },
    "X-Plasma-APIVersion": "2"
}
"#;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KdeIntegrationPaths {
    pub shell_package_dir: PathBuf,
    pub metadata_path: PathBuf,
    pub lockscreen_dir: PathBuf,
    pub lockscreen_qml_path: PathBuf,
    pub systemd_user_dir: PathBuf,
    pub kwin_dropin_dir: PathBuf,
    pub kwin_dropin_path: PathBuf,
}

/// Compatibility status structure retained so existing callers do not need to
/// change while KDE integration moves away from `plasmashellrc`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KdeIntegrationStatus {
    pub shell_package_installed: bool,
    pub lockscreen_qml_installed: bool,
    pub active_shell_package: Option<String>,
    pub screenshaver_selected: bool,
    pub previous_shell_package: Option<String>,
}

pub fn integration_paths() -> io::Result<KdeIntegrationPaths> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Unable to manage KDE lock-screen integration because HOME is not defined",
            )
        })?;

    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local").join("share"));

    let config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));

    let shell_package_dir = data_home
        .join("plasma")
        .join("shells")
        .join(SCREENSHAVER_SHELL_PACKAGE);

    let lockscreen_dir = shell_package_dir
        .join("contents")
        .join("lockscreen");

    let systemd_user_dir = config_home
        .join("systemd")
        .join("user");

    let kwin_dropin_dir = systemd_user_dir
        .join(KWIN_DROPIN_DIRECTORY);

    Ok(KdeIntegrationPaths {
        metadata_path: shell_package_dir.join("metadata.json"),
        lockscreen_qml_path: lockscreen_dir.join("LockScreen.qml"),
        kwin_dropin_path: kwin_dropin_dir.join(KWIN_DROPIN_FILENAME),
        shell_package_dir,
        lockscreen_dir,
        systemd_user_dir,
        kwin_dropin_dir,
    })
}

/// Installs or updates the Screenshaver KDE shell package and the KWin-only
/// systemd environment override. This operation is idempotent and never edits
/// `plasmashellrc`.
pub fn install(
    config: &LockScreenWidgetConfig,
) -> io::Result<KdeIntegrationStatus> {
    let paths = integration_paths()?;

    fs::create_dir_all(&paths.lockscreen_dir)?;
    fs::write(&paths.metadata_path, KDE_METADATA_JSON)?;

    construct_lock_screen_kde::write_lock_screen_kde(
        config,
        &paths.lockscreen_qml_path,
    )?;

    fs::create_dir_all(&paths.kwin_dropin_dir)?;

    write_text_atomic(
        &paths.kwin_dropin_path,
        KWIN_DROPIN_CONTENTS,
    )?;

    systemd_user_daemon_reload()?;

    status_from_paths(&paths)
}

/// Refreshes the generated Plasma package/QML without changing the KWin
/// systemd drop-in.
pub fn refresh(
    config: &LockScreenWidgetConfig,
) -> io::Result<KdeIntegrationStatus> {
    let paths = integration_paths()?;

    fs::create_dir_all(&paths.lockscreen_dir)?;
    fs::write(&paths.metadata_path, KDE_METADATA_JSON)?;

    construct_lock_screen_kde::write_lock_screen_kde(
        config,
        &paths.lockscreen_qml_path,
    )?;

    status_from_paths(&paths)
}

/// Disables Screenshaver's KDE integration by removing only the
/// Screenshaver-owned KWin drop-in. The installed shell package is deliberately
/// retained. No Plasma desktop-shell configuration is changed.
pub fn restore() -> io::Result<KdeIntegrationStatus> {
    let paths = integration_paths()?;

    match fs::remove_file(&paths.kwin_dropin_path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    remove_directory_if_empty(&paths.kwin_dropin_dir)?;
    systemd_user_daemon_reload()?;

    status_from_paths(&paths)
}

/// Engages KDE Plasma's compositor-integrated KScreenLocker and waits until
/// the authenticated lock session has completed.
pub fn run(
    logfile: &Path,
    running: &AtomicBool,
    wallpaper_control:
        &crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
) -> Result<(), String> {
    crate::logger::information(
        logfile,
        "[LOCK] KDE Plasma detected; engaging KScreenLocker backend",
    );

    if !wallpaper_control.request_pause_after_first_frame(running) {
        crate::logger::error(
            logfile,
            "[LOCK] Unable to confirm wallpaper renderer pause before KScreenLocker request",
        );

        return Err(
            "Wallpaper renderer did not acknowledge pause before KDE secure screen lock"
                .to_string(),
        );
    }

    crate::logger::information(
        logfile,
        "[LOCK] Wallpaper renderer paused; requesting KDE secure screen lock",
    );

    let dbus_command = match locate_qdbus_command() {
        Some(command) => command,
        None => {
            resume_wallpaper_after_kde_lock(
                wallpaper_control,
                running,
                logfile,
            );

            return Err(
                "Unable to locate qdbus6 or qdbus for KDE KScreenLocker control"
                    .to_string(),
            );
        }
    };

    let lock_output = match run_qdbus(&dbus_command, "Lock") {
        Ok(output) => output,
        Err(error) => {
            resume_wallpaper_after_kde_lock(
                wallpaper_control,
                running,
                logfile,
            );
            return Err(error);
        }
    };

    if !lock_output.status.success() {
        resume_wallpaper_after_kde_lock(
            wallpaper_control,
            running,
            logfile,
        );

        return Err(format!(
            "KScreenLocker lock request failed: {}",
            command_failure_message(&lock_output),
        ));
    }

    crate::logger::information(
        logfile,
        "[LOCK] KScreenLocker reported secure lock acquisition",
    );

    let observation_started = Instant::now();
    let mut lock_observed = false;
    let mut shutdown_deferred_logged = false;

    loop {
        if !running.load(Ordering::SeqCst)
            && !shutdown_deferred_logged
        {
            crate::logger::information(
                logfile,
                "[LOCK] Shutdown requested while KScreenLocker is active; deferring renderer/session cleanup until authenticated unlock",
            );
            shutdown_deferred_logged = true;
        }

        let active = query_kde_lock_active(&dbus_command)
            .unwrap_or(false);

        let greeter_running = kscreenlocker_greeter_running();

        if active || greeter_running {
            lock_observed = true;
        }

        if lock_observed && !active && !greeter_running {
            break;
        }

        if !lock_observed
            && observation_started.elapsed() >= Duration::from_secs(5)
        {
            crate::logger::warning(
                logfile,
                "[LOCK] KScreenLocker lock request succeeded but lock state could not be observed; waiting for greeter completion as a safety precaution",
            );

            if !greeter_running {
                resume_wallpaper_after_kde_lock(
                    wallpaper_control,
                    running,
                    logfile,
                );

                return Err(
                    "KScreenLocker accepted the lock request, but Screenshaver could not observe an active secure-lock session"
                        .to_string(),
                );
            }
        }

        thread::sleep(Duration::from_millis(100));
    }

    crate::logger::information(
        logfile,
        "[LOCK] KDE KScreenLocker session unlocked successfully",
    );

    resume_wallpaper_after_kde_lock(
        wallpaper_control,
        running,
        logfile,
    );

    Ok(())
}

pub fn status() -> io::Result<KdeIntegrationStatus> {
    let paths = integration_paths()?;
    status_from_paths(&paths)
}

fn status_from_paths(
    paths: &KdeIntegrationPaths,
) -> io::Result<KdeIntegrationStatus> {
    let dropin_active = match fs::read_to_string(&paths.kwin_dropin_path) {
        Ok(contents) => contents == KWIN_DROPIN_CONTENTS,
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => return Err(error),
    };

    Ok(KdeIntegrationStatus {
        shell_package_installed: paths.metadata_path.is_file(),
        lockscreen_qml_installed: paths.lockscreen_qml_path.is_file(),
        screenshaver_selected: dropin_active,
        active_shell_package: if dropin_active {
            Some(SCREENSHAVER_SHELL_PACKAGE.to_string())
        } else {
            None
        },
        previous_shell_package: None,
    })
}

fn systemd_user_daemon_reload() -> io::Result<()> {
    let output = Command::new("systemctl")
        .arg("--user")
        .arg("daemon-reload")
        .output()?;

    if output.status.success() {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        format!(
            "systemctl --user daemon-reload failed: {}",
            command_failure_message(&output),
        ),
    ))
}

fn remove_directory_if_empty(path: &Path) -> io::Result<()> {
    match fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::DirectoryNotEmpty => Ok(()),
        Err(error) => Err(error),
    }
}

fn locate_qdbus_command() -> Option<String> {
    for command in ["qdbus6", "qdbus"] {
        if Command::new(command)
            .arg("--version")
            .output()
            .is_ok()
        {
            return Some(command.to_string());
        }
    }

    None
}

fn run_qdbus(command: &str, method: &str) -> Result<Output, String> {
    Command::new(command)
        .arg("org.freedesktop.ScreenSaver")
        .arg("/ScreenSaver")
        .arg(format!(
            "org.freedesktop.ScreenSaver.{}",
            method,
        ))
        .output()
        .map_err(|error| {
            format!(
                "Unable to execute {} for KDE KScreenLocker control: {}",
                command,
                error,
            )
        })
}

fn query_kde_lock_active(command: &str) -> Result<bool, String> {
    let output = run_qdbus(command, "GetActive")?;

    if !output.status.success() {
        return Err(format!(
            "KScreenLocker GetActive query failed: {}",
            command_failure_message(&output),
        ));
    }

    match String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "true" => Ok(true),
        "false" => Ok(false),
        value => Err(format!(
            "KScreenLocker GetActive returned unexpected value '{}'",
            value,
        )),
    }
}

fn kscreenlocker_greeter_running() -> bool {
    let Ok(entries) = fs::read_dir("/proc") else {
        return false;
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(pid_text) = file_name.to_str() else {
            continue;
        };

        if !pid_text.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }

        let cmdline_path = entry.path().join("cmdline");
        let Ok(cmdline) = fs::read(cmdline_path) else {
            continue;
        };

        if cmdline
            .windows(b"kscreenlocker_greet".len())
            .any(|window| window == b"kscreenlocker_greet")
        {
            return true;
        }
    }

    false
}

fn command_failure_message(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr)
        .trim()
        .to_string();

    if !stderr.is_empty() {
        return stderr;
    }

    let stdout = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

    if !stdout.is_empty() {
        return stdout;
    }

    format!("process exited with status {}", output.status)
}

fn resume_wallpaper_after_kde_lock(
    wallpaper_control:
        &crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
    running: &AtomicBool,
    logfile: &Path,
) {
    if running.load(Ordering::SeqCst) {
        wallpaper_control.resume_and_wait_for_frame(running);

        crate::logger::information(
            logfile,
            "[LOCK] Wallpaper renderer resumed after KDE lock session",
        );
    } else {
        crate::logger::information(
            logfile,
            "[LOCK] KDE lock session ended with shutdown pending; wallpaper renderer will remain stopped",
        );
    }
}

fn write_text_atomic(path: &Path, contents: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Unable to construct temporary file name for {}",
                    path.display(),
                ),
            )
        })?;

    let temporary_path = path.with_file_name(format!(
        ".{}.screenshaver.tmp",
        file_name,
    ));

    fs::write(&temporary_path, contents)?;
    fs::rename(&temporary_path, path)
}
