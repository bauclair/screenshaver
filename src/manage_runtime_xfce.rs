//! manage_runtime_xfce.rs
//!
//! Runtime-ownership handshake for the Xfce lock-screen presenter.
//!
//! xfce4-screensaver may launch Screenshaver's trusted saver executable at any
//! time because Screenshaver is registered as an Xfce saver theme.  That launch
//! alone does not authorize shader rendering.  Shader presentation is permitted
//! only while the normal resident Screenshaver process is active.
//!
//! The resident process creates a small ownership marker in XDG_RUNTIME_DIR
//! after it has acquired Screenshaver's singleton.  The Xfce saver child checks
//! that marker before rendering.  The marker records both PID and Linux process
//! start time so a stale marker cannot become valid merely because the kernel
//! later reuses the same PID.
//!
//! This module does not participate in authentication, input handling, PAM, or
//! unlock authority.  Those remain entirely owned by xfce4-screensaver.

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

const XFCONF_QUERY_BINARY: &str =
    "/usr/bin/xfconf-query";

const XFCE_SCREENSAVER_CHANNEL: &str =
    "xfce4-screensaver";

const SAVER_THEME_LIST_PATH: &str =
    "/saver/themes/list";

const SCREENSHAVER_THEME_ID: &str =
    "screensavers-screenshaver";


#[derive(Debug)]
pub(crate) struct XfceRuntimeSession {
    marker_path: PathBuf,
    logfile: PathBuf,
    pid: u32,
    process_start_ticks: u64,
    previous_saver_themes: Vec<String>,
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
    /// This should be called only after the normal resident process has
    /// successfully acquired Screenshaver's singleton.
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


        // Remove only demonstrably stale state.  A live marker is never
        // overwritten.
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
            }


            Err(error)
                if error.kind()
                    == std::io::ErrorKind::NotFound =>
            {}


            Err(error) => {
                return Err(
                    format!(
                        "Unable to inspect existing XFCE runtime ownership marker '{}': {}",
                        marker_path.display(),
                        error,
                    )
                );
            }
        }


        let pid =
            std::process::id();

        let process_start_ticks =
            process_start_ticks(
                pid
            )?;


        let previous_saver_themes =
            read_xfce_saver_themes()?;


        select_xfce_saver_themes(
            &[
                SCREENSHAVER_THEME_ID.to_string()
            ]
        )?;


        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] XFCE saver theme selected for Screenshaver runtime; previous themes: {}",
                describe_theme_list(
                    &previous_saver_themes
                ),
            ),
        );


        if let Err(error) =
            write_marker(
                &marker_path,
                pid,
                process_start_ticks,
            )
        {
            let restore_result =
                select_xfce_saver_themes(
                    &previous_saver_themes
                );

            return Err(
                match restore_result {
                    Ok(()) => {
                        format!(
                            "{}; previous XFCE saver themes were restored",
                            error,
                        )
                    }

                    Err(restore_error) => {
                        format!(
                            "{}; additionally unable to restore previous XFCE saver themes: {}",
                            error,
                            restore_error,
                        )
                    }
                }
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


        Ok(
            Self {
                marker_path,
                logfile:
                    logfile.to_path_buf(),
                pid,
                process_start_ticks,
                previous_saver_themes,
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


        match read_marker(
            &self.marker_path
        ) {
            Ok(marker)
                if marker.pid == self.pid
                    && marker.process_start_ticks
                        == self.process_start_ticks =>
            {
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


            Ok(_) => {
                crate::logger::warning(
                    &self.logfile,
                    &format!(
                        "[LOCK] XFCE runtime ownership marker '{}' no longer belongs to this Screenshaver runtime; it was not removed",
                        self.marker_path.display(),
                    ),
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
                        "[LOCK] Unable to inspect XFCE runtime ownership marker during cleanup: {}",
                        error,
                    ),
                );
            }
        }


        match select_xfce_saver_themes(
            &self.previous_saver_themes
        ) {
            Ok(()) => {
                crate::logger::information(
                    &self.logfile,
                    &format!(
                        "[LOCK] XFCE saver themes restored after Screenshaver runtime: {}",
                        describe_theme_list(
                            &self.previous_saver_themes
                        ),
                    ),
                );
            }

            Err(error) => {
                crate::logger::warning(
                    &self.logfile,
                    &format!(
                        "[LOCK] Unable to restore previous XFCE saver themes after Screenshaver runtime: {}",
                        error,
                    ),
                );
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
            format!(
                "Unable to query XFCE saver themes: {}",
                String::from_utf8_lossy(
                    &output.stderr
                )
                .trim(),
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
                |line| {
                    line.to_string()
                }
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
            format!(
                "Unable to configure XFCE saver themes: {}",
                String::from_utf8_lossy(
                    &output.stderr
                )
                .trim(),
            )
        );
    }


    Ok(())
}


fn describe_theme_list(
    themes: &[String],
) -> String {

    if themes.is_empty() {
        return "<none>"
            .to_string();
    }


    themes.join(
        ", "
    )
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
