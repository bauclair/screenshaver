//! manage_runtime_xfce.rs
//!
//! Runtime ownership and temporary Xfce saver selection for Screenshaver.
//!
//! xfce4-screensaver remains the secure screen locker. Screenshaver registers a
//! trusted saver child, but that child is authorized to render shaders only
//! while the normal resident Screenshaver process owns this runtime session.
//!
//! The resident process:
//!   * preserves the user's current Xfce saver mode and theme list;
//!   * writes a persistent recovery snapshot;
//!   * establishes the existing PID/start-time ownership marker;
//!   * temporarily selects Screenshaver as the Xfce saver;
//!   * restores the prior native Xfce saver configuration on normal shutdown.
//!
//! A later Screenshaver start also repairs a stale recovery snapshot left by a
//! crashed/killed resident process. Authentication, input isolation, PAM, and
//! unlock authority remain entirely owned by xfce4-screensaver.

use std::fs::{
    self,
    File,
    OpenOptions,
};
use std::io::{
    Read,
    Write,
};
use std::path::{
    Path,
    PathBuf,
};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::{
    OpenOptionsExt,
    PermissionsExt,
};


const RUNTIME_DIRECTORY_NAME: &str =
    "screenshaver";

const MARKER_FILE_NAME: &str =
    "xfce-runtime-owner";

const MARKER_VERSION: u32 =
    1;

const RECOVERY_FILE_NAME: &str =
    "xfce-saver-runtime.restore";

const RECOVERY_VERSION: u32 =
    1;

const XFCONF_QUERY_BINARY: &str =
    "/usr/bin/xfconf-query";

const XFCE_SCREENSAVER_CHANNEL: &str =
    "xfce4-screensaver";

const SAVER_MODE_PATH: &str =
    "/saver/mode";

const SAVER_THEME_LIST_PATH: &str =
    "/saver/themes/list";

const SAVER_MODE_SINGLE: i32 =
    2;

const SCREENSHAVER_THEME_ID: &str =
    "screensavers-screenshaver";


#[derive(Debug, Clone)]
struct XfceSaverConfiguration {
    mode: i32,
    themes: Vec<String>,
}


#[derive(Debug)]
pub(crate) struct XfceRuntimeSession {
    marker_path: PathBuf,
    recovery_path: PathBuf,
    logfile: PathBuf,
    pid: u32,
    process_start_ticks: u64,
    previous_configuration: XfceSaverConfiguration,
    active: bool,
}


#[derive(Debug)]
struct RuntimeMarker {
    pid: u32,
    process_start_ticks: u64,
}


impl XfceRuntimeSession {

    /// Establish runtime ownership for the resident Screenshaver process.
    ///
    /// Call only after the normal resident process has acquired Screenshaver's
    /// singleton.
    pub(crate) fn acquire(
        logfile: &Path,
    ) -> Result<Self, String> {

        let marker_path =
            marker_path()?;

        ensure_runtime_directory(
            marker_path
                .parent()
                .ok_or_else(
                    || {
                        "XFCE runtime marker has no parent directory"
                            .to_string()
                    }
                )?
        )?;


        let marker_was_stale =
            match read_marker(
                &marker_path
            ) {
                Ok(marker) => {
                    if marker_is_live(
                        &marker
                    )? {
                        return Err(
                            format!(
                                "XFCE runtime ownership marker '{}' already belongs to a live process (pid={})",
                                marker_path.display(),
                                marker.pid,
                            )
                        );
                    }

                    fs::remove_file(
                        &marker_path
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Unable to remove stale XFCE runtime ownership marker '{}': {}",
                                marker_path.display(),
                                error,
                            )
                        }
                    )?;

                    true
                }

                Err(error)
                    if error.kind()
                        == std::io::ErrorKind::NotFound =>
                {
                    false
                }

                Err(error) => {
                    return Err(
                        format!(
                            "Unable to inspect existing XFCE runtime ownership marker '{}': {}",
                            marker_path.display(),
                            error,
                        )
                    );
                }
            };


        let recovery_path =
            recovery_path()?;


        if marker_was_stale
            || recovery_path.exists()
        {
            recover_stale_configuration(
                logfile,
                &recovery_path,
            )?;
        }


        let previous_configuration =
            read_xfce_saver_configuration()?;


        write_recovery_snapshot(
            &recovery_path,
            &previous_configuration,
        )?;


        let pid =
            std::process::id();

        let process_start_ticks =
            process_start_ticks(
                pid
            )?;


        if let Err(error) =
            write_marker(
                &marker_path,
                pid,
                process_start_ticks,
            )
        {
            let _ =
                fs::remove_file(
                    &recovery_path
                );

            return Err(
                error
            );
        }


        // Authorization exists before Xfce is pointed at Screenshaver. If a
        // lock happens in the tiny interval before the theme switch, Xfce still
        // uses its native saver rather than an unauthorized Screenshaver child.
        if let Err(error) =
            select_xfce_saver_themes(
                &[
                    SCREENSHAVER_THEME_ID.to_string()
                ]
            )
        {
            cleanup_failed_activation(
                logfile,
                &marker_path,
                &recovery_path,
                &previous_configuration,
                &format!(
                    "Unable to select Screenshaver as the XFCE saver theme: {}",
                    error,
                ),
            );

            return Err(
                error
            );
        }


        if let Err(error) =
            set_xfce_saver_mode(
                SAVER_MODE_SINGLE
            )
        {
            cleanup_failed_activation(
                logfile,
                &marker_path,
                &recovery_path,
                &previous_configuration,
                &format!(
                    "Unable to select XFCE single-saver mode for Screenshaver: {}",
                    error,
                ),
            );

            return Err(
                error
            );
        }


        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] XFCE runtime ownership established: pid={} marker={}",
                pid,
                marker_path.display(),
            ),
        );

        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] XFCE saver temporarily assigned to Screenshaver; previous mode={} themes={}",
                previous_configuration.mode,
                describe_theme_list(
                    &previous_configuration.themes
                ),
            ),
        );


        Ok(
            Self {
                marker_path,
                recovery_path,
                logfile:
                    logfile.to_path_buf(),
                pid,
                process_start_ticks,
                previous_configuration,
                active:
                    true,
            }
        )
    }


    fn release(
        &mut self,
    ) {

        if !self.active {
            return;
        }


        let marker_still_ours =
            match read_marker(
                &self.marker_path
            ) {
                Ok(marker) => {
                    marker.pid == self.pid
                        && marker.process_start_ticks
                            == self.process_start_ticks
                }

                Err(error)
                    if error.kind()
                        == std::io::ErrorKind::NotFound =>
                {
                    true
                }

                Err(error) => {
                    crate::logger::warning(
                        &self.logfile,
                        &format!(
                            "[LOCK] Unable to inspect XFCE runtime ownership marker during cleanup: {}",
                            error,
                        ),
                    );

                    false
                }
            };


        let restored =
            if marker_still_ours {
                match restore_xfce_saver_configuration(
                    &self.previous_configuration
                ) {
                    Ok(()) => {
                        crate::logger::information(
                            &self.logfile,
                            &format!(
                                "[LOCK] XFCE saver configuration restored after Screenshaver runtime: mode={} themes={}",
                                self.previous_configuration.mode,
                                describe_theme_list(
                                    &self.previous_configuration.themes
                                ),
                            ),
                        );

                        true
                    }

                    Err(error) => {
                        crate::logger::warning(
                            &self.logfile,
                            &format!(
                                "[LOCK] Unable to restore previous XFCE saver configuration after Screenshaver runtime: {}",
                                error,
                            ),
                        );

                        false
                    }
                }
            } else {
                crate::logger::warning(
                    &self.logfile,
                    "[LOCK] XFCE runtime ownership marker no longer belongs to this runtime; saver configuration was not changed during cleanup",
                );

                false
            };


        // Restore native Xfce behavior before revoking authorization. This
        // avoids a normal-shutdown interval where Xfce still points at
        // Screenshaver but the trusted child is no longer authorized.
        if marker_still_ours {
            match fs::remove_file(
                &self.marker_path
            ) {
                Ok(()) => {
                    crate::logger::information(
                        &self.logfile,
                        "[LOCK] XFCE runtime ownership released",
                    );
                }

                Err(error)
                    if error.kind()
                        == std::io::ErrorKind::NotFound =>
                {}

                Err(error) => {
                    crate::logger::warning(
                        &self.logfile,
                        &format!(
                            "[LOCK] Unable to remove XFCE runtime ownership marker '{}': {}",
                            self.marker_path.display(),
                            error,
                        ),
                    );
                }
            }
        }


        if restored {
            match fs::remove_file(
                &self.recovery_path
            ) {
                Ok(()) => {}

                Err(error)
                    if error.kind()
                        == std::io::ErrorKind::NotFound =>
                {}

                Err(error) => {
                    crate::logger::warning(
                        &self.logfile,
                        &format!(
                            "[LOCK] XFCE saver recovery snapshot could not be removed after successful restoration: {}",
                            error,
                        ),
                    );
                }
            }
        }


        self.active =
            false;
    }
}


impl Drop for XfceRuntimeSession {

    fn drop(
        &mut self,
    ) {

        self.release();
    }
}


/// Return true only when the runtime marker identifies a currently-live
/// resident Screenshaver process.
///
/// This is a read-only check used by the separately launched Xfce saver child.
pub(crate) fn resident_runtime_active(
) -> Result<bool, String> {

    let marker_path =
        marker_path()?;


    let marker =
        match read_marker(
            &marker_path
        ) {
            Ok(marker) => {
                marker
            }

            Err(error)
                if error.kind()
                    == std::io::ErrorKind::NotFound =>
            {
                return Ok(
                    false
                );
            }

            Err(error) => {
                return Err(
                    format!(
                        "Unable to read XFCE runtime ownership marker '{}': {}",
                        marker_path.display(),
                        error,
                    )
                );
            }
        };


    marker_is_live(
        &marker
    )
}


fn read_xfce_saver_configuration(
) -> Result<XfceSaverConfiguration, String> {

    Ok(
        XfceSaverConfiguration {
            mode:
                read_xfce_saver_mode()?,

            themes:
                read_xfce_saver_themes()?,
        }
    )
}


fn restore_xfce_saver_configuration(
    configuration: &XfceSaverConfiguration,
) -> Result<(), String> {

    // Restore themes first while runtime authorization is still present. Then
    // restore the user's original mode.
    select_xfce_saver_themes(
        &configuration.themes
    )?;

    set_xfce_saver_mode(
        configuration.mode
    )
}


fn read_xfce_saver_mode(
) -> Result<i32, String> {

    let output =
        Command::new(
            XFCONF_QUERY_BINARY
        )
        .args(
            [
                "-c",
                XFCE_SCREENSAVER_CHANNEL,
                "-p",
                SAVER_MODE_PATH,
            ]
        )
        .output()
        .map_err(
            |error| {
                format!(
                    "Unable to query XFCE saver mode: {}",
                    error,
                )
            }
        )?;


    if !output.status.success() {
        return Err(
            command_failure(
                "Unable to query XFCE saver mode",
                &output,
            )
        );
    }


    let value =
        String::from_utf8_lossy(
            &output.stdout
        )
        .trim()
        .to_string();


    value
        .parse::<i32>()
        .map_err(
            |error| {
                format!(
                    "Unable to parse XFCE saver mode '{}': {}",
                    value,
                    error,
                )
            }
        )
}


fn set_xfce_saver_mode(
    mode: i32,
) -> Result<(), String> {

    let mode =
        mode.to_string();


    let output =
        Command::new(
            XFCONF_QUERY_BINARY
        )
        .args(
            [
                "-c",
                XFCE_SCREENSAVER_CHANNEL,
                "-p",
                SAVER_MODE_PATH,
                "-s",
                mode.as_str(),
            ]
        )
        .output()
        .map_err(
            |error| {
                format!(
                    "Unable to configure XFCE saver mode: {}",
                    error,
                )
            }
        )?;


    if !output.status.success() {
        return Err(
            command_failure(
                "Unable to configure XFCE saver mode",
                &output,
            )
        );
    }


    Ok(())
}


fn read_xfce_saver_themes(
) -> Result<Vec<String>, String> {

    let output =
        Command::new(
            XFCONF_QUERY_BINARY
        )
        .args(
            [
                "-c",
                XFCE_SCREENSAVER_CHANNEL,
                "-p",
                SAVER_THEME_LIST_PATH,
            ]
        )
        .output()
        .map_err(
            |error| {
                format!(
                    "Unable to query XFCE saver themes: {}",
                    error,
                )
            }
        )?;


    if !output.status.success() {
        return Err(
            command_failure(
                "Unable to query XFCE saver themes",
                &output,
            )
        );
    }


    let stdout =
        String::from_utf8_lossy(
            &output.stdout
        );


    let themes =
        stdout
            .lines()
            .map(
                |line| {
                    line.trim()
                }
            )
            .filter(
                |line| {
                    !line.is_empty()
                        && !line.starts_with(
                            "Value is an array with "
                        )
                }
            )
            .map(
                str::to_string
            )
            .collect::<Vec<_>>();


    if themes.is_empty() {
        return Err(
            "XFCE saver theme list is empty; refusing to replace an unknown configuration"
                .to_string()
        );
    }


    Ok(
        themes
    )
}


fn select_xfce_saver_themes(
    themes: &[String],
) -> Result<(), String> {

    if themes.is_empty() {
        return Err(
            "XFCE saver theme list cannot be empty"
                .to_string()
        );
    }


    let mut command =
        Command::new(
            XFCONF_QUERY_BINARY
        );

    command.args(
        [
            "-c",
            XFCE_SCREENSAVER_CHANNEL,
            "-p",
            SAVER_THEME_LIST_PATH,
        ]
    );


    for theme in themes {
        command.args(
            [
                "-t",
                "string",
                "-s",
                theme.as_str(),
            ]
        );
    }


    command.arg(
        "--force-array"
    );


    let output =
        command
            .output()
            .map_err(
                |error| {
                    format!(
                        "Unable to configure XFCE saver themes: {}",
                        error,
                    )
                }
            )?;


    if !output.status.success() {
        return Err(
            command_failure(
                "Unable to configure XFCE saver themes",
                &output,
            )
        );
    }


    Ok(())
}


fn command_failure(
    context: &str,
    output: &std::process::Output,
) -> String {

    let stderr =
        String::from_utf8_lossy(
            &output.stderr
        );

    let stderr =
        stderr.trim();


    if stderr.is_empty() {
        format!(
            "{}; command exited with status {}",
            context,
            output.status,
        )
    } else {
        format!(
            "{}: {}",
            context,
            stderr,
        )
    }
}


fn describe_theme_list(
    themes: &[String],
) -> String {

    themes.join(
        ", "
    )
}


fn recovery_path(
) -> Result<PathBuf, String> {

    let config_root =
        match std::env::var_os(
            "XDG_CONFIG_HOME"
        ) {
            Some(path)
                if PathBuf::from(
                    &path
                )
                .is_absolute() =>
            {
                PathBuf::from(
                    path
                )
            }

            _ => {
                let home =
                    std::env::var_os(
                        "HOME"
                    )
                    .ok_or_else(
                        || {
                            "HOME is unavailable while locating the XFCE saver recovery snapshot"
                                .to_string()
                        }
                    )?;

                PathBuf::from(
                    home
                )
                .join(
                    ".config"
                )
            }
        };


    Ok(
        config_root
            .join(
                RUNTIME_DIRECTORY_NAME
            )
            .join(
                RECOVERY_FILE_NAME
            )
    )
}


fn write_recovery_snapshot(
    path: &Path,
    configuration: &XfceSaverConfiguration,
) -> Result<(), String> {

    let parent =
        path.parent()
            .ok_or_else(
                || {
                    "XFCE saver recovery snapshot has no parent directory"
                        .to_string()
                }
            )?;


    fs::create_dir_all(
        parent
    )
    .map_err(
        |error| {
            format!(
                "Unable to create XFCE saver recovery directory '{}': {}",
                parent.display(),
                error,
            )
        }
    )?;


    #[cfg(unix)]
    fs::set_permissions(
        parent,
        fs::Permissions::from_mode(
            0o700
        ),
    )
    .map_err(
        |error| {
            format!(
                "Unable to set XFCE saver recovery directory permissions on '{}': {}",
                parent.display(),
                error,
            )
        }
    )?;


    let temporary_path =
        path.with_extension(
            "restore.tmp"
        );


    let mut options =
        OpenOptions::new();

    options
        .write(
            true
        )
        .create(
            true
        )
        .truncate(
            true
        );


    #[cfg(unix)]
    options.mode(
        0o600
    );


    let mut file =
        options
            .open(
                &temporary_path
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to create XFCE saver recovery snapshot '{}': {}",
                        temporary_path.display(),
                        error,
                    )
                }
            )?;


    writeln!(
        file,
        "version={}",
        RECOVERY_VERSION
    )
    .map_err(
        |error| error.to_string()
    )?;

    writeln!(
        file,
        "mode={}",
        configuration.mode
    )
    .map_err(
        |error| error.to_string()
    )?;


    for theme in &configuration.themes {
        if theme.contains(
            '\n'
        ) || theme.contains(
            '\r'
        ) {
            let _ =
                fs::remove_file(
                    &temporary_path
                );

            return Err(
                "XFCE saver theme contains an invalid line break"
                    .to_string()
            );
        }

        writeln!(
            file,
            "theme={}",
            theme
        )
        .map_err(
            |error| error.to_string()
        )?;
    }


    file.sync_all()
        .map_err(
            |error| {
                format!(
                    "Unable to synchronize XFCE saver recovery snapshot '{}': {}",
                    temporary_path.display(),
                    error,
                )
            }
        )?;


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
                "Unable to install XFCE saver recovery snapshot '{}': {}",
                path.display(),
                error,
            )
        }
    )
}


fn read_recovery_snapshot(
    path: &Path,
) -> Result<XfceSaverConfiguration, String> {

    let contents =
        fs::read_to_string(
            path
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read XFCE saver recovery snapshot '{}': {}",
                    path.display(),
                    error,
                )
            }
        )?;


    let mut version =
        None;

    let mut mode =
        None;

    let mut themes =
        Vec::new();


    for line in contents.lines() {
        if let Some(value) =
            line.strip_prefix(
                "version="
            )
        {
            version =
                Some(
                    value
                        .parse::<u32>()
                        .map_err(
                            |error| {
                                format!(
                                    "Invalid XFCE saver recovery version '{}': {}",
                                    value,
                                    error,
                                )
                            }
                        )?
                );
        } else if let Some(value) =
            line.strip_prefix(
                "mode="
            )
        {
            mode =
                Some(
                    value
                        .parse::<i32>()
                        .map_err(
                            |error| {
                                format!(
                                    "Invalid XFCE saver recovery mode '{}': {}",
                                    value,
                                    error,
                                )
                            }
                        )?
                );
        } else if let Some(value) =
            line.strip_prefix(
                "theme="
            )
        {
            if !value.is_empty() {
                themes.push(
                    value.to_string()
                );
            }
        }
    }


    if version != Some(
        RECOVERY_VERSION
    ) {
        return Err(
            format!(
                "Unsupported or missing XFCE saver recovery snapshot version: {:?}",
                version,
            )
        );
    }


    let mode =
        mode
            .ok_or_else(
                || {
                    "XFCE saver recovery snapshot is missing its saver mode"
                        .to_string()
                }
            )?;


    if themes.is_empty() {
        return Err(
            "XFCE saver recovery snapshot contains no saver themes"
                .to_string()
        );
    }


    Ok(
        XfceSaverConfiguration {
            mode,
            themes,
        }
    )
}


fn recover_stale_configuration(
    logfile: &Path,
    recovery_path: &Path,
) -> Result<(), String> {

    if !recovery_path.exists() {
        return Ok(
            ()
        );
    }


    let saved =
        read_recovery_snapshot(
            recovery_path
        )?;

    let current =
        read_xfce_saver_configuration()?;


    let screenshaver_still_selected =
        current.themes.len() == 1
            && current.themes[0]
                == SCREENSHAVER_THEME_ID;


    if screenshaver_still_selected {
        restore_xfce_saver_configuration(
            &saved
        )?;

        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] Recovered stale XFCE saver configuration from a previous Screenshaver runtime: mode={} themes={}",
                saved.mode,
                describe_theme_list(
                    &saved.themes
                ),
            ),
        );
    } else {
        crate::logger::information(
            logfile,
            "[LOCK] Discarding stale XFCE saver recovery snapshot because Xfce is no longer assigned to Screenshaver",
        );
    }


    fs::remove_file(
        recovery_path
    )
    .map_err(
        |error| {
            format!(
                "Unable to remove stale XFCE saver recovery snapshot '{}': {}",
                recovery_path.display(),
                error,
            )
        }
    )
}


fn cleanup_failed_activation(
    logfile: &Path,
    marker_path: &Path,
    recovery_path: &Path,
    previous_configuration: &XfceSaverConfiguration,
    context: &str,
) {

    crate::logger::warning(
        logfile,
        &format!(
            "[LOCK] {}",
            context,
        ),
    );


    match restore_xfce_saver_configuration(
        previous_configuration
    ) {
        Ok(()) => {
            let _ =
                fs::remove_file(
                    recovery_path
                );
        }

        Err(error) => {
            crate::logger::warning(
                logfile,
                &format!(
                    "[LOCK] Unable to roll back XFCE saver configuration after activation failure: {}",
                    error,
                ),
            );
        }
    }


    let _ =
        fs::remove_file(
            marker_path
        );
}


fn marker_path(
) -> Result<PathBuf, String> {

    let runtime_directory =
        match std::env::var_os(
            "XDG_RUNTIME_DIR"
        ) {
            Some(runtime_directory) => {
                PathBuf::from(
                    runtime_directory
                )
            }


            None => {
                // xfce4-screensaver launches the trusted saver child with a
                // deliberately small environment.  On the tested Xfce path,
                // XDG_RUNTIME_DIR is not inherited even though the child runs
                // as the same logged-in user.
                //
                // Recover the standard Linux per-user runtime directory from
                // this process's effective UID.  The resident Screenshaver
                // process creates the marker under XDG_RUNTIME_DIR, which on
                // Linux is /run/user/<uid>; deriving the same path here lets
                // the separately launched Xfce child validate that marker
                // without weakening the runtime-ownership check.
                let effective_uid =
                    effective_uid_from_proc()?;


                PathBuf::from(
                    "/run/user"
                )
                .join(
                    effective_uid.to_string()
                )
            }
        };


    Ok(
        runtime_directory
            .join(
                RUNTIME_DIRECTORY_NAME
            )
            .join(
                MARKER_FILE_NAME
            )
    )
}


fn effective_uid_from_proc(
) -> Result<u32, String> {

    let status =
        fs::read_to_string(
            "/proc/self/status"
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read /proc/self/status while deriving XFCE runtime directory: {}",
                    error,
                )
            }
        )?;


    let uid_line =
        status
            .lines()
            .find(
                |line| {
                    line.starts_with(
                        "Uid:"
                    )
                }
            )
            .ok_or_else(
                || {
                    "Unable to locate Uid field in /proc/self/status"
                        .to_string()
                }
            )?;


    let mut fields =
        uid_line
            .split_whitespace();


    let _ =
        fields.next();


    let _real_uid =
        fields
            .next()
            .ok_or_else(
                || {
                    "Uid field in /proc/self/status is missing real UID"
                        .to_string()
                }
            )?;


    let effective_uid =
        fields
            .next()
            .ok_or_else(
                || {
                    "Uid field in /proc/self/status is missing effective UID"
                        .to_string()
                }
            )?;


    effective_uid
        .parse::<u32>()
        .map_err(
            |error| {
                format!(
                    "Unable to parse effective UID '{}' from /proc/self/status: {}",
                    effective_uid,
                    error,
                )
            }
        )
}


fn ensure_runtime_directory(
    directory: &Path,
) -> Result<(), String> {

    fs::create_dir_all(
        directory
    )
    .map_err(
        |error| {
            format!(
                "Unable to create XFCE runtime directory '{}': {}",
                directory.display(),
                error,
            )
        }
    )?;


    #[cfg(unix)]
    {
        fs::set_permissions(
            directory,
            fs::Permissions::from_mode(
                0o700
            ),
        )
        .map_err(
            |error| {
                format!(
                    "Unable to set XFCE runtime directory permissions on '{}': {}",
                    directory.display(),
                    error,
                )
            }
        )?;
    }


    Ok(())
}


fn write_marker(
    marker_path: &Path,
    pid: u32,
    process_start_ticks: u64,
) -> Result<(), String> {

    let mut options =
        OpenOptions::new();

    options
        .write(
            true
        )
        .create_new(
            true
        );


    #[cfg(unix)]
    {
        options.mode(
            0o600
        );
    }


    let mut file =
        options
            .open(
                marker_path
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to create XFCE runtime ownership marker '{}': {}",
                        marker_path.display(),
                        error,
                    )
                }
            )?;


    let contents =
        format!(
            "version={}\npid={}\nprocess_start_ticks={}\n",
            MARKER_VERSION,
            pid,
            process_start_ticks,
        );


    if let Err(error) =
        file.write_all(
            contents.as_bytes()
        )
    {
        let _ =
            fs::remove_file(
                marker_path
            );

        return Err(
            format!(
                "Unable to write XFCE runtime ownership marker '{}': {}",
                marker_path.display(),
                error,
            )
        );
    }


    file.sync_all()
        .map_err(
            |error| {
                format!(
                    "Unable to synchronize XFCE runtime ownership marker '{}': {}",
                    marker_path.display(),
                    error,
                )
            }
        )?;


    Ok(())
}


fn read_marker(
    marker_path: &Path,
) -> Result<RuntimeMarker, std::io::Error> {

    let mut file =
        File::open(
            marker_path
        )?;

    let mut contents =
        String::new();

    file.read_to_string(
        &mut contents
    )?;


    parse_marker(
        &contents
    )
    .map_err(
        |message| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                message,
            )
        }
    )
}


fn parse_marker(
    contents: &str,
) -> Result<RuntimeMarker, String> {

    let mut version:
        Option<u32> =
        None;

    let mut pid:
        Option<u32> =
        None;

    let mut process_start_ticks_value:
        Option<u64> =
        None;


    for line in contents.lines() {

        if let Some(value) =
            line.strip_prefix(
                "version="
            )
        {
            version =
                Some(
                    value
                        .parse::<u32>()
                        .map_err(
                            |error| {
                                format!(
                                    "Invalid XFCE runtime marker version '{}': {}",
                                    value,
                                    error,
                                )
                            }
                        )?
                );
        } else if let Some(value) =
            line.strip_prefix(
                "pid="
            )
        {
            pid =
                Some(
                    value
                        .parse::<u32>()
                        .map_err(
                            |error| {
                                format!(
                                    "Invalid XFCE runtime marker PID '{}': {}",
                                    value,
                                    error,
                                )
                            }
                        )?
                );
        } else if let Some(value) =
            line.strip_prefix(
                "process_start_ticks="
            )
        {
            process_start_ticks_value =
                Some(
                    value
                        .parse::<u64>()
                        .map_err(
                            |error| {
                                format!(
                                    "Invalid XFCE runtime marker process start time '{}': {}",
                                    value,
                                    error,
                                )
                            }
                        )?
                );
        }
    }


    let version =
        version
            .ok_or_else(
                || {
                    "XFCE runtime marker is missing its version"
                        .to_string()
                }
            )?;


    if version != MARKER_VERSION {
        return Err(
            format!(
                "Unsupported XFCE runtime marker version {}",
                version,
            )
        );
    }


    let pid =
        pid
            .ok_or_else(
                || {
                    "XFCE runtime marker is missing its PID"
                        .to_string()
                }
            )?;


    if pid == 0 {
        return Err(
            "XFCE runtime marker contains PID 0"
                .to_string()
        );
    }


    let process_start_ticks =
        process_start_ticks_value
            .ok_or_else(
                || {
                    "XFCE runtime marker is missing its process start time"
                        .to_string()
                }
            )?;


    Ok(
        RuntimeMarker {
            pid,
            process_start_ticks,
        }
    )
}


fn marker_is_live(
    marker: &RuntimeMarker,
) -> Result<bool, String> {

    match process_start_ticks(
        marker.pid
    ) {
        Ok(current_start_ticks) => {
            Ok(
                current_start_ticks
                    == marker.process_start_ticks
            )
        }


        Err(error)
            if error.contains(
                "does not exist"
            ) =>
        {
            Ok(
                false
            )
        }


        Err(error) => {
            Err(
                error
            )
        }
    }
}


/// Read field 22 (starttime) from /proc/<pid>/stat.
///
/// The command name in field 2 is enclosed in parentheses and may itself
/// contain spaces or ')' characters, so locate the final ')' and parse the
/// remaining fields relative to field 3.
fn process_start_ticks(
    pid: u32,
) -> Result<u64, String> {

    let stat_path =
        PathBuf::from(
            "/proc"
        )
        .join(
            pid.to_string()
        )
        .join(
            "stat"
        );


    let contents =
        match fs::read_to_string(
            &stat_path
        ) {
            Ok(contents) => {
                contents
            }


            Err(error)
                if error.kind()
                    == std::io::ErrorKind::NotFound =>
            {
                return Err(
                    format!(
                        "Process {} does not exist",
                        pid,
                    )
                );
            }


            Err(error) => {
                return Err(
                    format!(
                        "Unable to read Linux process state '{}': {}",
                        stat_path.display(),
                        error,
                    )
                );
            }
        };


    let command_end =
        contents
            .rfind(
                ')'
            )
            .ok_or_else(
                || {
                    format!(
                        "Unable to parse Linux process state for PID {}",
                        pid,
                    )
                }
            )?;


    let fields_after_command =
        contents[
            command_end + 1..
        ]
        .split_whitespace()
        .collect::<Vec<_>>();


    // fields_after_command[0] is field 3 (state), therefore field 22
    // (starttime) is index 19.
    let start_time =
        fields_after_command
            .get(
                19
            )
            .ok_or_else(
                || {
                    format!(
                        "Linux process state for PID {} does not contain starttime",
                        pid,
                    )
                }
            )?;


    start_time
        .parse::<u64>()
        .map_err(
            |error| {
                format!(
                    "Unable to parse Linux process starttime '{}' for PID {}: {}",
                    start_time,
                    pid,
                    error,
                )
            }
        )
}
