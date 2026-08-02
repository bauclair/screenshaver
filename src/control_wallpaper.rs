//! Cross-process wallpaper pause and resume coordination.
//!
//! The long-running Screenshaver process owns the wallpaper renderer. Command-line
//! preview sessions run in a separate process, so they cannot use the in-memory
//! WallpaperRuntimeControl directly. This module provides a deliberately small,
//! file-based protocol under XDG_RUNTIME_DIR.

use std::fs::{
    self,
    OpenOptions,
};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{
    Duration,
    Instant,
};

const CONTROL_WAIT: Duration =
    Duration::from_millis(500);

const POLL_INTERVAL: Duration =
    Duration::from_millis(2);

const ACTIVE_FILE: &str =
    "screenshaver-wallpaper.active";

const PAUSE_REQUEST_FILE: &str =
    "screenshaver-wallpaper.pause";

const PAUSE_ACK_FILE: &str =
    "screenshaver-wallpaper.paused";

const RESUME_ACK_FILE: &str =
    "screenshaver-wallpaper.resumed";


pub struct WallpaperPauseGuard {
    pause_was_requested: bool,
}


impl WallpaperPauseGuard {

    pub fn acquire() -> Self {

        let pause_was_requested =
            request_pause();

        Self {
            pause_was_requested,
        }
    }
}


impl Drop for WallpaperPauseGuard {

    fn drop(
        &mut self,
    ) {

        if self.pause_was_requested {
            resume_and_wait_for_frame();
        }
    }
}


pub(crate) fn external_pause_requested() -> bool {

    control_path(
        PAUSE_REQUEST_FILE
    )
    .is_some_and(
        |path| path.exists()
    )
}


pub(crate) fn acknowledge_paused() {

    let _ =
        write_marker(
            PAUSE_ACK_FILE
        );
}


pub(crate) fn acknowledge_resumed_frame() {

    let _ =
        write_marker(
            RESUME_ACK_FILE
        );
}


pub(crate) fn set_runtime_active(
    active: bool,
) {

    if active {

        let _ =
            remove_marker(
                PAUSE_ACK_FILE
            );

        let _ =
            remove_marker(
                RESUME_ACK_FILE
            );

        let _ =
            write_marker(
                ACTIVE_FILE
            );

    } else {

        let _ =
            remove_marker(
                ACTIVE_FILE
            );

        let _ =
            remove_marker(
                PAUSE_ACK_FILE
            );

        let _ =
            remove_marker(
                RESUME_ACK_FILE
            );
    }
}


fn request_pause() -> bool {

    if !runtime_is_active() {
        return false;
    }


    let _ =
        remove_marker(
            PAUSE_ACK_FILE
        );

    let _ =
        remove_marker(
            RESUME_ACK_FILE
        );


    if write_marker(
        PAUSE_REQUEST_FILE
    )
    .is_err()
    {
        return false;
    }


    wait_for_marker(
        PAUSE_ACK_FILE
    );

    true
}


fn resume_and_wait_for_frame() {

    let _ =
        remove_marker(
            RESUME_ACK_FILE
        );

    let _ =
        remove_marker(
            PAUSE_ACK_FILE
        );

    let _ =
        remove_marker(
            PAUSE_REQUEST_FILE
        );


    if runtime_is_active() {
        wait_for_marker(
            RESUME_ACK_FILE
        );
    }


    let _ =
        remove_marker(
            RESUME_ACK_FILE
        );
}


fn runtime_is_active() -> bool {

    control_path(
        ACTIVE_FILE
    )
    .is_some_and(
        |path| path.exists()
    )
}


fn wait_for_marker(
    name: &str,
) {

    let Some(path) =
        control_path(
            name
        )
    else {
        return;
    };


    let deadline =
        Instant::now()
            + CONTROL_WAIT;


    while Instant::now()
        < deadline
    {
        if path.exists() {
            return;
        }

        std::thread::sleep(
            POLL_INTERVAL
        );
    }
}


fn write_marker(
    name: &str,
) -> Result<(), std::io::Error> {

    let Some(path) =
        control_path(
            name
        )
    else {
        return Ok(());
    };


    let mut file =
        OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(
                path
            )?;


    writeln!(
        file,
        "{}",
        std::process::id()
    )?;


    file.flush()
}


fn remove_marker(
    name: &str,
) -> Result<(), std::io::Error> {

    let Some(path) =
        control_path(
            name
        )
    else {
        return Ok(());
    };


    match fs::remove_file(
        path
    ) {

        Ok(()) => {
            Ok(())
        }

        Err(error)
            if error.kind()
                == std::io::ErrorKind::NotFound =>
        {
            Ok(())
        }

        Err(error) => {
            Err(
                error
            )
        }
    }
}


fn control_path(
    name: &str,
) -> Option<PathBuf> {

    match runtime_directory() {
        Ok(runtime_directory) => {
            Some(
                runtime_directory.join(
                    name
                )
            )
        }

        Err(error) => {
            log_warning(
                &error
            );

            None
        }
    }
}


fn runtime_directory() -> Result<PathBuf, String> {

    static RUNTIME_DIRECTORY:
        OnceLock<Result<PathBuf, String>> =
            OnceLock::new();

    RUNTIME_DIRECTORY
        .get_or_init(
            resolve_runtime_directory
        )
        .clone()
}


fn resolve_runtime_directory() -> Result<PathBuf, String> {

    if let Some(runtime_directory) =
        std::env::var_os(
            "XDG_RUNTIME_DIR"
        )
    {
        let path =
            PathBuf::from(
                runtime_directory
            );

        if path.is_dir() {
            log_information(
                &format!(
                    "Using XDG runtime directory: {}",
                    path.display(),
                )
            );

            return Ok(
                path
            );
        }

        log_warning(
            &format!(
                "XDG_RUNTIME_DIR does not name an existing directory: {}",
                path.display(),
            )
        );
    }

    let effective_uid =
        unsafe {
            libc::geteuid()
        };

    let fallback =
        PathBuf::from(
            format!(
                "/run/user/{effective_uid}"
            )
        );

    if fallback.is_dir() {
        log_information(
            &format!(
                "XDG_RUNTIME_DIR unavailable; using fallback runtime directory: {}",
                fallback.display(),
            )
        );

        return Ok(
            fallback
        );
    }

    Err(
        format!(
            "Unable to locate a usable wallpaper control runtime directory; checked XDG_RUNTIME_DIR and {}",
            fallback.display(),
        )
    )
}


fn log_information(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();

    crate::logger::information(
        &logfile,
        &format!(
            "[WALLPAPER_CONTROL] {}",
            message
        ),
    );
}


fn log_warning(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();

    crate::logger::warning(
        &logfile,
        &format!(
            "[WALLPAPER_CONTROL] {}",
            message
        ),
    );
}

