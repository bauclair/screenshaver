use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{
    Path,
    PathBuf,
};
use std::process::Command;


const XFCE_SCREENSAVER_BINARY: &str =
    "/usr/bin/xfce4-screensaver";

const XFCONF_QUERY_BINARY: &str =
    "/usr/bin/xfconf-query";

const TRUSTED_PRESENTER_PATH: &str =
    "/usr/libexec/xfce4-screensaver/screenshaver";

const SAVER_THEME_ID: &str =
    "screensavers-screenshaver";

const SAVER_MODE_PATH: &str =
    "/saver/mode";

const SAVER_THEME_LIST_PATH: &str =
    "/saver/themes/list";

const SAVER_MODE_SINGLE: &str =
    "2";

const SAVER_DESKTOP_ENTRY: &str =
    "[Desktop Entry]\n\
Name=Screenshaver\n\
Comment=Screenshaver shader presentation for the Xfce lock screen\n\
Exec=/usr/libexec/xfce4-screensaver/screenshaver\n\
TryExec=/usr/libexec/xfce4-screensaver/screenshaver\n\
StartupNotify=false\n\
Terminal=false\n\
Type=Application\n\
Categories=Screensaver;\n\
OnlyShowIn=XFCE;\n";

const LIGHT_LOCKER_AUTOSTART_OVERRIDE: &str =
    "[Desktop Entry]\n\
Type=Application\n\
Name=Screen Locker\n\
Hidden=true\n";


#[derive(Debug, Clone)]
pub struct XfceLockConfigurationStatus {
    pub xfce_screensaver_available: bool,
    pub xfconf_query_available: bool,
    pub trusted_presenter_installed: bool,
    pub saver_desktop_registered: bool,
    pub screenshaver_selected: bool,
    pub light_locker_autostart_disabled: bool,
}


impl XfceLockConfigurationStatus {

    pub fn ready_for_runtime(
        &self
    ) -> bool {

        self.xfce_screensaver_available
            && self.xfconf_query_available
            && self.trusted_presenter_installed
            && self.saver_desktop_registered
            && self.screenshaver_selected
            && self.light_locker_autostart_disabled
    }
}


pub fn inspect(
) -> Result<XfceLockConfigurationStatus, String> {

    let home =
        home_directory()?;

    let saver_desktop_path =
        saver_desktop_path(
            &home
        );

    let light_locker_override_path =
        light_locker_override_path(
            &home
        );

    Ok(
        XfceLockConfigurationStatus {
            xfce_screensaver_available:
                is_executable(
                    Path::new(
                        XFCE_SCREENSAVER_BINARY
                    )
                ),

            xfconf_query_available:
                is_executable(
                    Path::new(
                        XFCONF_QUERY_BINARY
                    )
                ),

            trusted_presenter_installed:
                is_executable(
                    Path::new(
                        TRUSTED_PRESENTER_PATH
                    )
                ),

            saver_desktop_registered:
                desktop_entry_is_current(
                    &saver_desktop_path
                ),

            screenshaver_selected:
                selected_theme_is_screenshaver(),

            light_locker_autostart_disabled:
                light_locker_override_is_current(
                    &light_locker_override_path
                ),
        }
    )
}


/// Configures only the per-user portion of the Xfce lock-screen integration.
///
/// The trusted presenter under /usr/libexec/xfce4-screensaver must already
/// have been installed by the package or installation procedure. Screenshaver
/// deliberately does not elevate privileges or modify package-owned files.
///
/// This function:
///
///   * registers Screenshaver as an Xfce screensaver theme;
///   * selects the Screenshaver theme in xfce4-screensaver;
///   * disables Light Locker autostart for the current user.
///
/// The Light Locker override takes effect on the next Xfce login. The caller
/// should therefore report that a logout/login may be required if Light Locker
/// is already running in the current session.
pub fn configure_user(
) -> Result<XfceLockConfigurationStatus, String> {

    verify_system_requirements()?;

    let home =
        home_directory()?;

    let saver_desktop_path =
        saver_desktop_path(
            &home
        );

    let light_locker_override_path =
        light_locker_override_path(
            &home
        );

    write_user_file(
        &saver_desktop_path,
        SAVER_DESKTOP_ENTRY,
    )?;

    write_user_file(
        &light_locker_override_path,
        LIGHT_LOCKER_AUTOSTART_OVERRIDE,
    )?;

    set_xfce_saver_mode()?;

    select_screenshaver_theme()?;

    let status =
        inspect()?;

    if !status.ready_for_runtime() {
        return Err(
            format!(
                "XFCE lock-screen configuration did not verify successfully: {:?}",
                status,
            )
        );
    }

    Ok(
        status
    )
}


fn verify_system_requirements(
) -> Result<(), String> {

    if !is_executable(
        Path::new(
            XFCE_SCREENSAVER_BINARY
        )
    ) {
        return Err(
            format!(
                "XFCE Screensaver is not available at {}",
                XFCE_SCREENSAVER_BINARY,
            )
        );
    }

    if !is_executable(
        Path::new(
            XFCONF_QUERY_BINARY
        )
    ) {
        return Err(
            format!(
                "xfconf-query is not available at {}",
                XFCONF_QUERY_BINARY,
            )
        );
    }

    if !is_executable(
        Path::new(
            TRUSTED_PRESENTER_PATH
        )
    ) {
        return Err(
            format!(
                "Screenshaver's trusted XFCE presenter is not installed at {}. \
                 Install it through the Screenshaver package or installer before \
                 configuring the per-user XFCE lock integration.",
                TRUSTED_PRESENTER_PATH,
            )
        );
    }

    Ok(())
}


fn home_directory(
) -> Result<PathBuf, String> {

    let home =
        std::env::var_os(
            "HOME"
        )
        .ok_or_else(
            || {
                "HOME is not set; unable to locate the user's XFCE configuration"
                    .to_string()
            }
        )?;

    let home =
        PathBuf::from(
            home
        );

    if !home.is_absolute() {
        return Err(
            format!(
                "HOME does not contain an absolute path: {}",
                home.display(),
            )
        );
    }

    Ok(
        home
    )
}


fn saver_desktop_path(
    home: &Path
) -> PathBuf {

    home.join(
        ".local/share/applications/screensavers/screenshaver.desktop"
    )
}


fn light_locker_override_path(
    home: &Path
) -> PathBuf {

    home.join(
        ".config/autostart/light-locker.desktop"
    )
}


fn desktop_entry_is_current(
    path: &Path
) -> bool {

    match fs::read_to_string(
        path
    ) {
        Ok(contents) => {
            contents
                == SAVER_DESKTOP_ENTRY
        }

        Err(_) => {
            false
        }
    }
}


fn light_locker_override_is_current(
    path: &Path
) -> bool {

    match fs::read_to_string(
        path
    ) {
        Ok(contents) => {
            contents
                == LIGHT_LOCKER_AUTOSTART_OVERRIDE
        }

        Err(_) => {
            false
        }
    }
}


fn selected_theme_is_screenshaver(
) -> bool {

    let output =
        Command::new(
            XFCONF_QUERY_BINARY
        )
        .args(
            [
                "-c",
                "xfce4-screensaver",
                "-p",
                SAVER_THEME_LIST_PATH,
            ]
        )
        .output();

    let Ok(output) =
        output
    else {
        return false;
    };

    if !output.status.success() {
        return false;
    }

    let stdout =
        String::from_utf8_lossy(
            &output.stdout
        );

    stdout
        .lines()
        .any(
            |line| {
                line.trim()
                    == SAVER_THEME_ID
            }
        )
}


fn set_xfce_saver_mode(
) -> Result<(), String> {

    let query =
        Command::new(
            XFCONF_QUERY_BINARY
        )
        .args(
            [
                "-c",
                "xfce4-screensaver",
                "-p",
                SAVER_MODE_PATH,
            ]
        )
        .output()
        .map_err(
            |error| {
                format!(
                    "Unable to query XFCE screensaver mode: {}",
                    error,
                )
            }
        )?;

    let mut command =
        Command::new(
            XFCONF_QUERY_BINARY
        );

    command.args(
        [
            "-c",
            "xfce4-screensaver",
            "-p",
            SAVER_MODE_PATH,
        ]
    );

    if !query.status.success() {
        command.args(
            [
                "--create",
                "-t",
                "int",
            ]
        );
    }

    command.args(
        [
            "-s",
            SAVER_MODE_SINGLE,
        ]
    );

    let output =
        command
            .output()
            .map_err(
                |error| {
                    format!(
                        "Unable to configure XFCE screensaver mode: {}",
                        error,
                    )
                }
            )?;

    require_command_success(
        output,
        "Unable to configure XFCE screensaver mode",
    )
}


fn select_screenshaver_theme(
) -> Result<(), String> {

    let output =
        Command::new(
            XFCONF_QUERY_BINARY
        )
        .args(
            [
                "-c",
                "xfce4-screensaver",
                "-p",
                SAVER_THEME_LIST_PATH,
                "-t",
                "string",
                "-s",
                SAVER_THEME_ID,
                "--force-array",
            ]
        )
        .output()
        .map_err(
            |error| {
                format!(
                    "Unable to select Screenshaver as the XFCE screensaver theme: {}",
                    error,
                )
            }
        )?;

    require_command_success(
        output,
        "Unable to select Screenshaver as the XFCE screensaver theme",
    )
}


fn require_command_success(
    output: std::process::Output,
    context: &str,
) -> Result<(), String> {

    if output.status.success() {
        return Ok(
            ()
        );
    }

    let stderr =
        String::from_utf8_lossy(
            &output.stderr
        );

    let stderr =
        stderr.trim();

    if stderr.is_empty() {
        return Err(
            format!(
                "{}; command exited with status {}",
                context,
                output.status,
            )
        );
    }

    Err(
        format!(
            "{}: {}",
            context,
            stderr,
        )
    )
}


fn write_user_file(
    path: &Path,
    contents: &str,
) -> Result<(), String> {

    let parent =
        path.parent()
            .ok_or_else(
                || {
                    format!(
                        "Unable to determine parent directory for {}",
                        path.display(),
                    )
                }
            )?;

    fs::create_dir_all(
        parent
    )
    .map_err(
        |error| {
            format!(
                "Unable to create directory {}: {}",
                parent.display(),
                error,
            )
        }
    )?;

    let temporary_path =
        path.with_extension(
            "desktop.tmp"
        );

    {
        let mut file =
            fs::File::create(
                &temporary_path
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to create temporary file {}: {}",
                        temporary_path.display(),
                        error,
                    )
                }
            )?;

        file.write_all(
            contents.as_bytes()
        )
        .map_err(
            |error| {
                format!(
                    "Unable to write temporary file {}: {}",
                    temporary_path.display(),
                    error,
                )
            }
        )?;

        file.sync_all()
            .map_err(
                |error| {
                    format!(
                        "Unable to synchronize temporary file {}: {}",
                        temporary_path.display(),
                        error,
                    )
                }
            )?;
    }

    fs::rename(
        &temporary_path,
        path,
    )
    .map_err(
        |error| {
            let _ =
                fs::remove_file(
                    &temporary_path
                );

            format!(
                "Unable to install user configuration file {}: {}",
                path.display(),
                error,
            )
        }
    )?;

    Ok(
        ()
    )
}


fn is_executable(
    path: &Path
) -> bool {

    let Ok(metadata) =
        fs::metadata(
            path
        )
    else {
        return false;
    };

    if !metadata.is_file() {
        return false;
    }

    metadata.permissions().mode()
        & 0o111
        != 0
}
