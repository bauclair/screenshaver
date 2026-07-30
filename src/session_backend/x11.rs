use std::cell::Cell;
use std::os::raw::c_void;
use std::ptr;
use std::time::Duration;

use x11::{xlib, xss};

use crate::query_session::{
    SessionBackend,
    SessionError,
    SessionState,
};
use crate::x11_connection::X11Connection;

pub struct X11Backend {
    connection: X11Connection,
    screensaver_info: *mut xss::XScreenSaverInfo,
    idle_timeout_ms: u64,

    // Diagnostic throttle: write at most one idle-progress message per second.
    last_logged_idle_second: Cell<u64>,
}

impl X11Backend {
    pub fn new(
        idle_timeout: Duration,
    ) -> Result<Self, SessionError> {
        let idle_timeout_ms =
            idle_timeout
                .as_millis()
                .min(u64::MAX as u128) as u64;

        log_debug(
            &format!(
                "[X11] Idle threshold = {} ms",
                idle_timeout_ms,
            )
        );

        let connection =
            X11Connection::connect()
                .map_err(
                    |error| {
                        SessionError::BackendUnavailable(
                            error
                        )
                    }
                )?;

        unsafe {
            let mut event_base: i32 = 0;
            let mut error_base: i32 = 0;

            let extension_available =
                xss::XScreenSaverQueryExtension(
                    connection.display(),
                    &mut event_base,
                    &mut error_base,
                );

            if extension_available == 0 {
                return Err(
                    SessionError::BackendUnavailable(
                        "XScreenSaver extension is unavailable".to_string()
                    )
                );
            }

            log_debug(
                &format!(
                    "[X11] XScreenSaver extension available (event_base={}, error_base={})",
                    event_base,
                    error_base,
                )
            );

            let screensaver_info =
                xss::XScreenSaverAllocInfo();

            if screensaver_info.is_null() {
                return Err(
                    SessionError::BackendUnavailable(
                        "Unable to allocate XScreenSaverInfo".to_string()
                    )
                );
            }

            Ok(
                Self {
                    connection,
                    screensaver_info,
                    idle_timeout_ms,
                    last_logged_idle_second: Cell::new(u64::MAX),
                }
            )
        }
    }
}

impl SessionBackend for X11Backend {
    fn poll_state(
        &self,
    ) -> Result<SessionState, SessionError> {
        let query_succeeded =
            unsafe {
                xss::XScreenSaverQueryInfo(
                    self.connection.display(),
                    self.connection.root_window(),
                    self.screensaver_info,
                )
            };

        if query_succeeded == 0 {
            log_error(
                "[X11] XScreenSaverQueryInfo failed"
            );

            return Err(
                SessionError::QueryFailed(
                    "XScreenSaverQueryInfo failed".to_string()
                )
            );
        }

        let idle_ms =
            unsafe {
                (*self.screensaver_info).idle as u64
            };

        let idle_second =
            idle_ms / 1_000;

        if self.last_logged_idle_second.get() != idle_second {
            self.last_logged_idle_second.set(
                idle_second
            );

            log_debug(
                &format!(
                    "[X11] poll_state: idle={} ms, timeout={} ms",
                    idle_ms,
                    self.idle_timeout_ms,
                )
            );
        }

        if idle_ms >= self.idle_timeout_ms {
            log_information(
                &format!(
                    "[X11] State = Idle (idle={} ms, timeout={} ms)",
                    idle_ms,
                    self.idle_timeout_ms,
                )
            );

            Ok(
                SessionState::Idle
            )
        }
        else {
            Ok(
                SessionState::Active
            )
        }
    }

    fn backend_name(
        &self,
    ) -> &'static str {
        "x11"
    }
}

impl Drop for X11Backend {
    fn drop(
        &mut self,
    ) {
        unsafe {
            if !self.screensaver_info.is_null() {
                xlib::XFree(
                    self.screensaver_info as *mut c_void
                );

                self.screensaver_info =
                    ptr::null_mut();
            }
        }

        // `connection` is dropped automatically after this method returns.
        // X11Connection is the sole owner of the Display* and closes it once.
    }
}

fn log_debug(
    message: &str,
) {
    let logfile =
        crate::locate_paths::runtime_log_path();

    crate::logger::debug(
        &logfile,
        message,
    );
}

fn log_information(
    message: &str,
) {
    let logfile =
        crate::locate_paths::runtime_log_path();

    crate::logger::information(
        &logfile,
        message,
    );
}

fn log_error(
    message: &str,
) {
    let logfile =
        crate::locate_paths::runtime_log_path();

    crate::logger::error(
        &logfile,
        message,
    );
}

