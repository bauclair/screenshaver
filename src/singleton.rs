use fs2::FileExt;

use std::fmt;
use std::fs::{
    File,
    OpenOptions,
};
use std::io::{
    Read,
    Seek,
    SeekFrom,
    Write,
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
    PidWriteFailed(std::io::Error),
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopOutcome {
    StopRequested {
        pid: u32,
    },

    NotRunning,
}


#[derive(Debug)]
pub enum StopError {
    RuntimeDirectoryUnavailable,
    OpenFailed(std::io::Error),
    LockCheckFailed(std::io::Error),
    PidReadFailed(std::io::Error),
    InvalidPid(String),
    SignalFailed {
        pid: u32,
        error: std::io::Error,
    },
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

            Self::PidWriteFailed(error) => {
                write!(
                    formatter,
                    "Failed to write the Screenshaver process ID: {}",
                    error,
                )
            }
        }
    }
}


impl std::error::Error for SingletonError {}


impl fmt::Display for StopError {

    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {

        match self {

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

            Self::LockCheckFailed(error) => {
                write!(
                    formatter,
                    "Failed to inspect the instance lock: {}",
                    error,
                )
            }

            Self::PidReadFailed(error) => {
                write!(
                    formatter,
                    "Failed to read the Screenshaver process ID: {}",
                    error,
                )
            }

            Self::InvalidPid(value) => {
                write!(
                    formatter,
                    "The instance lock contains an invalid process ID: '{}'",
                    value,
                )
            }

            Self::SignalFailed {
                pid,
                error,
            } => {
                write!(
                    formatter,
                    "Failed to stop Screenshaver process {}: {}",
                    pid,
                    error,
                )
            }
        }
    }
}


impl std::error::Error for StopError {}


pub fn acquire() -> Result<Singleton, SingletonError> {

    let lock_path =
        lock_path()
            .map_err(
                |error| {
                    match error {
                        PathError::RuntimeDirectoryUnavailable => {
                            SingletonError::RuntimeDirectoryUnavailable
                        }
                    }
                }
            )?;


    let mut file =
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

            write_current_pid(
                &mut file
            )?;


            log(
                &format!(
                    "Exclusive instance lock acquired: {} (PID {})",
                    lock_path.display(),
                    std::process::id(),
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


pub fn stop() -> Result<StopOutcome, StopError> {

    let lock_path =
        lock_path()
            .map_err(
                |error| {
                    match error {
                        PathError::RuntimeDirectoryUnavailable => {
                            StopError::RuntimeDirectoryUnavailable
                        }
                    }
                }
            )?;


    let mut file =
        match OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
        {
            Ok(file) => file,

            Err(error)
                if error.kind()
                    == std::io::ErrorKind::NotFound =>
            {
                log(
                    "Stop requested, but no instance lock file exists"
                );

                return Ok(
                    StopOutcome::NotRunning
                );
            }

            Err(error) => {
                return Err(
                    StopError::OpenFailed(
                        error
                    )
                );
            }
        };


    match file.try_lock_exclusive() {

        Ok(()) => {

            let _ =
                file.unlock();


            log(
                "Stop requested, but the instance lock is not held"
            );


            Ok(
                StopOutcome::NotRunning
            )
        }

        Err(error)
            if error.kind()
                == std::io::ErrorKind::WouldBlock =>
        {

            let pid =
                read_pid(
                    &mut file
                )?;


            send_sigterm(
                pid
            )?;


            log(
                &format!(
                    "SIGTERM sent to Screenshaver process {}",
                    pid
                )
            );


            Ok(
                StopOutcome::StopRequested {
                    pid,
                }
            )
        }

        Err(error) => {
            Err(
                StopError::LockCheckFailed(
                    error
                )
            )
        }
    }
}


fn write_current_pid(
    file: &mut File,
) -> Result<(), SingletonError> {

    file.set_len(0)
        .map_err(
            SingletonError::PidWriteFailed
        )?;


    file.seek(
        SeekFrom::Start(0)
    )
    .map_err(
        SingletonError::PidWriteFailed
    )?;


    writeln!(
        file,
        "{}",
        std::process::id()
    )
    .map_err(
        SingletonError::PidWriteFailed
    )?;


    file.flush()
        .map_err(
            SingletonError::PidWriteFailed
        )?;


    Ok(())
}


fn read_pid(
    file: &mut File,
) -> Result<u32, StopError> {

    file.seek(
        SeekFrom::Start(0)
    )
    .map_err(
        StopError::PidReadFailed
    )?;


    let mut contents =
        String::new();


    file.read_to_string(
        &mut contents
    )
    .map_err(
        StopError::PidReadFailed
    )?;


    let value =
        contents.trim();


    let pid =
        value.parse::<u32>()
            .map_err(
                |_| {
                    StopError::InvalidPid(
                        value.to_string()
                    )
                }
            )?;


    if pid == 0 {
        return Err(
            StopError::InvalidPid(
                value.to_string()
            )
        );
    }


    Ok(
        pid
    )
}


fn send_sigterm(
    pid: u32,
) -> Result<(), StopError> {

    let result =
        unsafe {
            libc::kill(
                pid as libc::pid_t,
                libc::SIGTERM,
            )
        };


    if result == 0 {
        return Ok(());
    }


    let error =
        std::io::Error::last_os_error();


    if error.raw_os_error()
        == Some(libc::ESRCH)
    {
        return Ok(());
    }


    Err(
        StopError::SignalFailed {
            pid,
            error,
        }
    )
}


#[derive(Debug)]
enum PathError {
    RuntimeDirectoryUnavailable,
}


fn lock_path() -> Result<PathBuf, PathError> {

    let runtime_dir =
        std::env::var_os(
            "XDG_RUNTIME_DIR"
        )
        .ok_or(
            PathError::RuntimeDirectoryUnavailable
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

