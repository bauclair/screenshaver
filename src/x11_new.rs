use std::ffi::CString;
use std::time::Duration;

use crate::query_session::{
    SessionBackend,
    SessionError,
    SessionState,
};

use x11::xlib;

pub struct X11Backend {
    display: *mut xlib::Display,
}

impl X11Backend {

    pub fn new(
        _idle_timeout: Duration,
    ) -> Result<Self, SessionError> {

        log("[X11] Opening X11 display");

        unsafe {

            let display =
                xlib::XOpenDisplay(
                    std::ptr::null()
                );

            if display.is_null() {

                return Err(
                    SessionError::BackendUnavailable(
                        "Unable to open X11 display".to_string()
                    )
                );
            }

            let display_name =
                xlib::XDisplayString(display);

            if !display_name.is_null() {

                let name =
                    std::ffi::CStr::from_ptr(
                        display_name
                    );

                log(
                    &format!(
                        "[X11] Connected to {}",
                        name.to_string_lossy()
                    )
                );
            }
            else {

                log(
                    "[X11] Connected (display name unavailable)"
                );
            }

            Ok(
                Self {
                    display,
                }
            )
        }
    }
}

impl SessionBackend for X11Backend {

    fn poll_state(
        &self,
    ) -> Result<SessionState, SessionError> {

        Ok(
            SessionState::Active
        )
    }

    fn backend_name(
        &self,
    ) -> &'static str {

        "x11"
    }
}

impl Drop for X11Backend {

    fn drop(
        &mut self
    ) {

        log(
            "[X11] Closing display"
        );

        unsafe {

            if !self.display.is_null() {

                xlib::XCloseDisplay(
                    self.display
                );
            }
        }
    }
}

fn log(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();

    crate::logger::log(
        &logfile,
        message,
    );

    println!(
        "{}",
        message
    );
}