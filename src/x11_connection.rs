use std::ffi::CStr;
use std::ptr;

use x11::xlib;

/// Shared ownership boundary for a native X11 display connection.
///
/// This type owns the `Display*` returned by `XOpenDisplay()` and closes it
/// exactly once when dropped. Higher-level X11 components may borrow the raw
/// display and root-window handles, but they must not close the display.
pub struct X11Connection {
    display: *mut xlib::Display,
    root_window: xlib::Window,
    screen: i32,
    display_name: String,
    width: u32,
    height: u32,
    depth: u32,
}

impl X11Connection {
    pub fn connect(
    ) -> Result<Self, String> {
        log_debug(
            "[X11] Opening X11 display"
        );

        unsafe {
            let display =
                xlib::XOpenDisplay(
                    ptr::null()
                );

            if display.is_null() {
                return Err(
                    "Unable to open X11 display".to_string()
                );
            }

            let display_name_pointer =
                xlib::XDisplayString(
                    display
                );

            let display_name =
                if display_name_pointer.is_null() {
                    "<unavailable>".to_string()
                }
                else {
                    CStr::from_ptr(
                        display_name_pointer
                    )
                    .to_string_lossy()
                    .into_owned()
                };

            log_debug(
                &format!(
                    "[X11] Connected to {}",
                    display_name,
                )
            );

            let screen =
                xlib::XDefaultScreen(
                    display
                );

            let root_window =
                xlib::XRootWindow(
                    display,
                    screen,
                );

            if root_window == 0 {
                xlib::XCloseDisplay(
                    display
                );

                return Err(
                    "Unable to obtain the X11 root window".to_string()
                );
            }

            let width =
                xlib::XDisplayWidth(
                    display,
                    screen,
                )
                .max(0) as u32;

            let height =
                xlib::XDisplayHeight(
                    display,
                    screen,
                )
                .max(0) as u32;

            let depth =
                xlib::XDefaultDepth(
                    display,
                    screen,
                )
                .max(0) as u32;

            log_debug(
                &format!(
                    "[X11] Screen = {}, root window = {}, geometry = {}x{}, depth = {}",
                    screen,
                    root_window,
                    width,
                    height,
                    depth,
                )
            );

            Ok(
                Self {
                    display,
                    root_window,
                    screen,
                    display_name,
                    width,
                    height,
                    depth,
                }
            )
        }
    }

    pub fn display(
        &self,
    ) -> *mut xlib::Display {
        self.display
    }

    pub fn root_window(
        &self,
    ) -> xlib::Window {
        self.root_window
    }

    pub fn screen(
        &self,
    ) -> i32 {
        self.screen
    }

    pub fn display_name(
        &self,
    ) -> &str {
        &self.display_name
    }

    pub fn width(
        &self,
    ) -> u32 {
        self.width
    }

    pub fn height(
        &self,
    ) -> u32 {
        self.height
    }

    pub fn depth(
        &self,
    ) -> u32 {
        self.depth
    }
}

impl Drop for X11Connection {
    fn drop(
        &mut self,
    ) {
        if self.display.is_null() {
            return;
        }

        log_debug(
            "[X11] Closing X11 display"
        );

        unsafe {
            xlib::XCloseDisplay(
                self.display
            );
        }

        self.display =
            ptr::null_mut();
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

