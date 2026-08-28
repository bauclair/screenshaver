//! manage_screen_lock_kde.rs
//!
//! Installs and manages Screenshaver's KDE Plasma / KScreenLocker integration.
//!
//! Screenshaver deliberately leaves KDE's normal `org.kde.plasma.desktop`
//! shell identity in place. KDE's system shell package is copied into a
//! same-ID user overlay under XDG_DATA_HOME, where Screenshaver can later add
//! its lock-screen rendering integration without changing `plasmashellrc`,
//! `PLASMA_DEFAULT_SHELL`, or KWin's environment.
//!
//! This first-stage overlay manager intentionally does not install the native
//! renderer plugin yet. It establishes ownership, refresh, and restore rules
//! for the safe user overlay. Native renderer/QML installation is layered on
//! top in the following integration step.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::define_lock_screen_widget::LockScreenWidgetConfig;

const KDE_SHELL_PACKAGE: &str = "org.kde.plasma.desktop";
const OWNERSHIP_MARKER_FILENAME: &str = ".screenshaver-overlay";
const OWNERSHIP_MARKER_MAGIC: &str = "screenshaver-kde-overlay-v1";
const LOCKSCREEN_QML_RELATIVE_PATH: &str = "contents/lockscreen/LockScreenUi.qml";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KdeIntegrationPaths {
    pub shell_package_dir: PathBuf,
    pub metadata_path: PathBuf,
    pub lockscreen_dir: PathBuf,
    pub lockscreen_qml_path: PathBuf,
    pub ownership_marker_path: PathBuf,
}

/// Compatibility status structure retained so existing callers do not need to
/// change while KDE integration moves to a same-ID user shell overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KdeIntegrationStatus {
    pub shell_package_installed: bool,
    pub lockscreen_qml_installed: bool,
    pub active_shell_package: Option<String>,
    pub screenshaver_selected: bool,
    pub previous_shell_package: Option<String>,
}

/// Lifetime-owned inhibition of KDE's own idle-triggered screen locking.
///
/// While this object exists, KDE's native idle timer is suppressed.
/// Screenshaver can still explicitly request KScreenLocker at its own
/// configured idle threshold.
///
/// The session D-Bus connection is intentionally retained for the full
/// lifetime of this object. If Screenshaver terminates abnormally, the
/// connection disappears and KDE releases the inhibition automatically.
pub struct KdeIdleLockInhibitor {
    connection: zbus::blocking::Connection,
    cookie: u32,
}

impl KdeIdleLockInhibitor {
    pub fn acquire() -> Result<Self, String> {
        let connection =
            zbus::blocking::Connection::session()
                .map_err(|error| {
                    format!(
                        "Unable to connect to the session D-Bus for KDE screen-lock inhibition: {}",
                        error,
                    )
                })?;

        let reply =
            connection
                .call_method(
                    Some("org.freedesktop.ScreenSaver"),
                    "/ScreenSaver",
                    Some("org.freedesktop.ScreenSaver"),
                    "Inhibit",
                    &(
                        "Screenshaver",
                        "Screenshaver manages idle screen locking",
                    ),
                )
                .map_err(|error| {
                    format!(
                        "Unable to inhibit KDE's native idle screen lock: {}",
                        error,
                    )
                })?;

        let cookie =
            reply
                .body()
                .deserialize::<u32>()
                .map_err(|error| {
                    format!(
                        "Unable to read KDE screen-lock inhibition cookie: {}",
                        error,
                    )
                })?;

        Ok(Self {
            connection,
            cookie,
        })
    }

    pub fn cookie(&self) -> u32 {
        self.cookie
    }
}

impl Drop for KdeIdleLockInhibitor {
    fn drop(&mut self) {
        let _ =
            self.connection
                .call_method(
                    Some("org.freedesktop.ScreenSaver"),
                    "/ScreenSaver",
                    Some("org.freedesktop.ScreenSaver"),
                    "UnInhibit",
                    &(self.cookie,),
                );
    }
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

    let shell_package_dir = data_home
        .join("plasma")
        .join("shells")
        .join(KDE_SHELL_PACKAGE);

    let lockscreen_dir = shell_package_dir
        .join("contents")
        .join("lockscreen");

    Ok(KdeIntegrationPaths {
        metadata_path: shell_package_dir.join("metadata.json"),
        lockscreen_qml_path: shell_package_dir.join(LOCKSCREEN_QML_RELATIVE_PATH),
        ownership_marker_path: shell_package_dir.join(OWNERSHIP_MARKER_FILENAME),
        shell_package_dir,
        lockscreen_dir,
    })
}

/// Installs the safe same-ID KDE shell overlay.
///
/// The user's system `org.kde.plasma.desktop` package is copied verbatim into
/// XDG_DATA_HOME. This stage does not yet modify LockScreenUi.qml or install
/// the native renderer plugin. If a user overlay already exists and is not
/// marked as Screenshaver-owned, installation refuses to alter it.
pub fn install(
    _config: &LockScreenWidgetConfig,
) -> io::Result<KdeIntegrationStatus> {
    let paths = integration_paths()?;

    if paths.shell_package_dir.exists()
        && !overlay_is_screenshaver_owned(&paths)?
    {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "Refusing to install Screenshaver KDE integration because {} already exists and is not Screenshaver-owned",
                paths.shell_package_dir.display(),
            ),
        ));
    }

    rebuild_owned_overlay(&paths)?;
    status_from_paths(&paths)
}

/// Refreshes a Screenshaver-owned overlay from KDE's currently installed
/// system shell package.
///
/// Refresh deliberately rebuilds from the current system package rather than
/// preserving an old copied KDE tree across Plasma upgrades. A foreign user
/// overlay is never overwritten.
pub fn refresh(
    _config: &LockScreenWidgetConfig,
) -> io::Result<KdeIntegrationStatus> {
    let paths = integration_paths()?;

    if paths.shell_package_dir.exists()
        && !overlay_is_screenshaver_owned(&paths)?
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "Refusing to refresh Screenshaver KDE integration because {} is not Screenshaver-owned",
                paths.shell_package_dir.display(),
            ),
        ));
    }

    rebuild_owned_overlay(&paths)?;
    status_from_paths(&paths)
}

/// Restores KDE's normal system-shell behavior by removing only a
/// Screenshaver-owned same-ID user overlay.
///
/// If the overlay is absent, restore is already complete. If a foreign user
/// overlay occupies the same path, restore refuses to remove it.
pub fn restore() -> io::Result<KdeIntegrationStatus> {
    let paths = integration_paths()?;

    if !paths.shell_package_dir.exists() {
        return status_from_paths(&paths);
    }

    if !overlay_is_screenshaver_owned(&paths)? {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "Refusing to remove KDE user overlay {} because it is not Screenshaver-owned",
                paths.shell_package_dir.display(),
            ),
        ));
    }

    fs::remove_dir_all(&paths.shell_package_dir)?;
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
    let owned = overlay_is_screenshaver_owned(paths)?;
    let overlay_exists = paths.shell_package_dir.is_dir();

    Ok(KdeIntegrationStatus {
        shell_package_installed: overlay_exists,
        lockscreen_qml_installed: owned && paths.lockscreen_qml_path.is_file(),
        screenshaver_selected: owned,
        active_shell_package: if owned {
            Some(KDE_SHELL_PACKAGE.to_string())
        } else {
            None
        },
        previous_shell_package: None,
    })
}

fn rebuild_owned_overlay(paths: &KdeIntegrationPaths) -> io::Result<()> {
    let system_shell_dir = locate_system_shell_package()?;

    validate_system_shell_package(&system_shell_dir)?;

    let parent = paths
        .shell_package_dir
        .parent()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Unable to determine parent directory for {}",
                    paths.shell_package_dir.display(),
                ),
            )
        })?;

    fs::create_dir_all(parent)?;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let staging_dir = parent.join(format!(
        ".{}.screenshaver-staging-{}-{}",
        KDE_SHELL_PACKAGE,
        std::process::id(),
        nonce,
    ));

    let previous_dir = parent.join(format!(
        ".{}.screenshaver-previous-{}-{}",
        KDE_SHELL_PACKAGE,
        std::process::id(),
        nonce,
    ));

    if staging_dir.exists() {
        fs::remove_dir_all(&staging_dir)?;
    }

    copy_directory_tree(&system_shell_dir, &staging_dir)?;

    let staged_marker = staging_dir.join(OWNERSHIP_MARKER_FILENAME);
    write_ownership_marker(&staged_marker, &system_shell_dir)?;

    let staged_lockscreen = staging_dir.join(LOCKSCREEN_QML_RELATIVE_PATH);
    if !staged_lockscreen.is_file() {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "KDE system shell package {} does not contain {}",
                system_shell_dir.display(),
                LOCKSCREEN_QML_RELATIVE_PATH,
            ),
        ));
    }

    if paths.shell_package_dir.exists() {
        fs::rename(&paths.shell_package_dir, &previous_dir)?;
    }

    match fs::rename(&staging_dir, &paths.shell_package_dir) {
        Ok(()) => {
            if previous_dir.exists() {
                fs::remove_dir_all(previous_dir)?;
            }
            Ok(())
        }
        Err(error) => {
            if previous_dir.exists() && !paths.shell_package_dir.exists() {
                let _ = fs::rename(&previous_dir, &paths.shell_package_dir);
            }
            let _ = fs::remove_dir_all(&staging_dir);
            Err(error)
        }
    }
}

fn locate_system_shell_package() -> io::Result<PathBuf> {
    let mut data_dirs = Vec::new();

    if let Some(value) = env::var_os("XDG_DATA_DIRS") {
        for path in env::split_paths(&value) {
            push_unique_path(&mut data_dirs, path);
        }
    }

    for path in [
        PathBuf::from("/run/current-system/sw/share"),
        PathBuf::from("/usr/local/share"),
        PathBuf::from("/usr/share"),
    ] {
        push_unique_path(&mut data_dirs, path);
    }

    for data_dir in data_dirs {
        let candidate = data_dir
            .join("plasma")
            .join("shells")
            .join(KDE_SHELL_PACKAGE);

        if validate_system_shell_package(&candidate).is_ok() {
            return Ok(candidate);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "Unable to locate KDE system shell package {} in XDG_DATA_DIRS or standard system data paths",
            KDE_SHELL_PACKAGE,
        ),
    ))
}

fn validate_system_shell_package(path: &Path) -> io::Result<()> {
    if !path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "KDE shell package directory does not exist: {}",
                path.display(),
            ),
        ));
    }

    if !path.join("metadata.json").is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "KDE shell package is missing metadata.json: {}",
                path.display(),
            ),
        ));
    }

    if !path.join(LOCKSCREEN_QML_RELATIVE_PATH).is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "KDE shell package is missing {}: {}",
                LOCKSCREEN_QML_RELATIVE_PATH,
                path.display(),
            ),
        ));
    }

    Ok(())
}

fn overlay_is_screenshaver_owned(
    paths: &KdeIntegrationPaths,
) -> io::Result<bool> {
    match fs::read_to_string(&paths.ownership_marker_path) {
        Ok(contents) => Ok(
            contents
                .lines()
                .next()
                .map(str::trim)
                == Some(OWNERSHIP_MARKER_MAGIC),
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn write_ownership_marker(
    marker_path: &Path,
    system_shell_dir: &Path,
) -> io::Result<()> {
    let contents = format!(
        "{}\nsystem_source={}\n",
        OWNERSHIP_MARKER_MAGIC,
        system_shell_dir.display(),
    );

    fs::write(marker_path, contents)
}

fn copy_directory_tree(source: &Path, destination: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(source)?;

    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Expected directory while copying KDE shell package: {}",
                source.display(),
            ),
        ));
    }

    fs::create_dir_all(destination)?;
    fs::set_permissions(destination, metadata.permissions())?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        copy_tree_entry(&source_path, &destination_path)?;
    }

    Ok(())
}

fn copy_tree_entry(source: &Path, destination: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    let file_type = metadata.file_type();

    if file_type.is_dir() {
        return copy_directory_tree(source, destination);
    }

    if file_type.is_file() {
        fs::copy(source, destination)?;
        fs::set_permissions(destination, metadata.permissions())?;
        return Ok(());
    }

    if file_type.is_symlink() {
        let target = fs::read_link(source)?;
        return create_symlink(&target, destination, source);
    }

    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        format!(
            "Unsupported file type in KDE shell package: {}",
            source.display(),
        ),
    ))
}

#[cfg(unix)]
fn create_symlink(
    target: &Path,
    destination: &Path,
    _source: &Path,
) -> io::Result<()> {
    std::os::unix::fs::symlink(target, destination)
}

#[cfg(not(unix))]
fn create_symlink(
    _target: &Path,
    _destination: &Path,
    source: &Path,
) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        format!(
            "KDE shell-package symlink copying is unsupported on this platform: {}",
            source.display(),
        ),
    ))
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
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
