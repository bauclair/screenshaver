use fs2::FileExt;

use std::fmt;
use std::fs::{
    File,
    OpenOptions,
};
use std::path::PathBuf;


pub struct Singleton {
    _file: File,
}


#[derive(Debug)]
pub enum SingletonError {
    AlreadyRunning,
    RuntimeDirectoryUnavailable,
    OpenFailed(std::io::Error),
    LockFailed(std::io::Error),
}


impl fmt::Display for SingletonError {

    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {

        match self {

            Self::AlreadyRunning => {
                write!(
                    formatter,
                    "Screenshaver is already running"
                )
            }

            Self::RuntimeDirectoryUnavailable => {
                write!(
                    formatter,
                    "XDG_RUNTIME_DIR is unavailable"
                )
            }

            Self::OpenFailed(error) => {
                write!(
                    formatter,
                    "Failed to open instance lock file: {}",
                    error,
                )
            }

            Self::LockFailed(error) => {
                write!(
                    formatter,
                    "Failed to acquire instance lock: {}",
                    error,
                )
            }
        }
    }
}


impl std::error::Error for SingletonError {}


pub fn acquire() -> Result<Singleton, SingletonError> {

    let lock_path =
        lock_path()?;


    let file =
        OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(
                SingletonError::OpenFailed
            )?;


    match file.try_lock_exclusive() {

        Ok(()) => {

            log(
                &format!(
                    "Exclusive instance lock acquired: {}",
                    lock_path.display()
                )
            );


            Ok(
                Singleton {
                    _file: file,
                }
            )
        }

        Err(error)
            if error.kind()
                == std::io::ErrorKind::WouldBlock =>
        {

            log(
                &format!(
                    "Instance lock already held: {}",
                    lock_path.display()
                )
            );


            Err(
                SingletonError::AlreadyRunning
            )
        }

        Err(error) => {

            log(
                &format!(
                    "Failed to acquire instance lock '{}': {}",
                    lock_path.display(),
                    error,
                )
            );


            Err(
                SingletonError::LockFailed(
                    error
                )
            )
        }
    }
}


fn lock_path() -> Result<PathBuf, SingletonError> {

    let runtime_dir =
        std::env::var_os(
            "XDG_RUNTIME_DIR"
        )
        .ok_or(
            SingletonError::RuntimeDirectoryUnavailable
        )?;


    Ok(
        PathBuf::from(runtime_dir)
            .join("screenshaver.lock")
    )
}


fn log(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::log(
        &logfile,
        &format!(
            "[INSTANCE] {}",
            message
        ),
    );
}