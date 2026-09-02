use std::env;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::path::Path;
use std::thread;
use std::time::Duration;

use x11::xlib;

const XSCREENSAVER_WINDOW_ENV: &str = "XSCREENSAVER_WINDOW";

/// Returns the X11 window supplied by xfce4-screensaver for an external
/// screensaver child.
///
/// xfce4-screensaver exports XSCREENSAVER_WINDOW before launching the
/// configured screensaver program. Screenshaver must render into this window;
/// it must not create or manage the secure lock window itself.
pub(crate) fn detect_presentation_window(
    logfile: &Path,
) -> Result<u64, String> {
    let raw_window =
        env::var(XSCREENSAVER_WINDOW_ENV)
            .map_err(|_| {
                format!(
                    "{} is not set; Screenshaver was not launched by an \
                     XScreenSaver-compatible host",
                    XSCREENSAVER_WINDOW_ENV,
                )
            })?;

    let raw_window =
        raw_window.trim();

    if raw_window.is_empty() {
        return Err(
            format!(
                "{} is empty",
                XSCREENSAVER_WINDOW_ENV,
            )
        );
    }

    let window =
        if let Some(hexadecimal) =
            raw_window
                .strip_prefix("0x")
                .or_else(|| raw_window.strip_prefix("0X"))
        {
            u64::from_str_radix(
                hexadecimal,
                16,
            )
            .map_err(|error| {
                format!(
                    "Unable to parse {} value '{}': {}",
                    XSCREENSAVER_WINDOW_ENV,
                    raw_window,
                    error,
                )
            })?
        } else {
            raw_window
                .parse::<u64>()
                .map_err(|error| {
                    format!(
                        "Unable to parse {} value '{}': {}",
                        XSCREENSAVER_WINDOW_ENV,
                        raw_window,
                        error,
                    )
                })?
        };

    if window == 0 {
        return Err(
            format!(
                "{} contains an invalid zero X11 window ID",
                XSCREENSAVER_WINDOW_ENV,
            )
        );
    }

    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] XFCE lock presentation window detected: 0x{:X}",
            window,
        ),
    );

    verify_presentation_window(
        logfile,
        window,
    )?;

    Ok(
        window
    )
}


fn verify_presentation_window(
    logfile: &Path,
    window: u64,
) -> Result<(), String> {
    crate::logger::information(
        logfile,
        "[LOCK] XFCE presentation-window verification: opening X11 display",
    );

    let connection =
        crate::x11_connection::X11Connection::connect()
            .map_err(|error| {
                format!(
                    "Unable to connect to the X11 display while verifying XFCE presentation window 0x{:X}: {}",
                    window,
                    error,
                )
            })?;

    crate::logger::information(
        logfile,
        "[LOCK] XFCE presentation-window verification: X11 display opened",
    );

    let x11_window =
        window as xlib::Window;

    let mut attributes =
        MaybeUninit::<xlib::XWindowAttributes>::uninit();

    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] XFCE presentation-window verification: querying attributes for 0x{:X}",
            window,
        ),
    );

    let status =
        unsafe {
            xlib::XGetWindowAttributes(
                connection.display(),
                x11_window,
                attributes.as_mut_ptr(),
            )
        };

    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] XFCE presentation-window verification: XGetWindowAttributes returned status={}",
            status,
        ),
    );

    if status == 0 {
        return Err(
            format!(
                "XGetWindowAttributes failed for XFCE presentation window 0x{:X}",
                window,
            )
        );
    }

    let attributes =
        unsafe {
            attributes.assume_init()
        };

    if attributes.width <= 0
        || attributes.height <= 0
    {
        return Err(
            format!(
                "XFCE presentation window 0x{:X} has invalid geometry {}x{}",
                window,
                attributes.width,
                attributes.height,
            )
        );
    }

    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] XFCE lock presentation window verified: 0x{:X}, geometry={}x{}, depth={}, map_state={}",
            window,
            attributes.width,
            attributes.height,
            attributes.depth,
            attributes.map_state,
        ),
    );

    println!(
        "XFCE presentation window verified: 0x{:X}, {}x{}, depth={}, map_state={}",
        window,
        attributes.width,
        attributes.height,
        attributes.depth,
        attributes.map_state,
    );

    draw_solid_test_color(
        logfile,
        &connection,
        x11_window,
        attributes.width as u32,
        attributes.height as u32,
    )?;

    Ok(())
}


fn draw_solid_test_color(
    logfile: &Path,
    connection: &crate::x11_connection::X11Connection,
    window: xlib::Window,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let display =
        connection.display();

    let screen =
        connection.screen();

    let colormap =
        unsafe {
            xlib::XDefaultColormap(
                display,
                screen,
            )
        };

    let color_name =
        CString::new("magenta")
            .map_err(|error| {
                format!(
                    "Unable to construct XFCE test color name: {}",
                    error,
                )
            })?;

    let mut screen_color =
        MaybeUninit::<xlib::XColor>::uninit();

    let mut exact_color =
        MaybeUninit::<xlib::XColor>::uninit();

    let color_status =
        unsafe {
            xlib::XAllocNamedColor(
                display,
                colormap,
                color_name.as_ptr(),
                screen_color.as_mut_ptr(),
                exact_color.as_mut_ptr(),
            )
        };

    if color_status == 0 {
        return Err(
            "Unable to allocate XFCE lock presentation test color"
                .to_string()
        );
    }

    let screen_color =
        unsafe {
            screen_color.assume_init()
        };

    let gc =
        unsafe {
            xlib::XCreateGC(
                display,
                window,
                0,
                std::ptr::null_mut(),
            )
        };

    if gc.is_null() {
        return Err(
            "Unable to create X11 graphics context for XFCE lock presentation test"
                .to_string()
        );
    }

    unsafe {
        xlib::XSetForeground(
            display,
            gc,
            screen_color.pixel,
        );

        xlib::XFillRectangle(
            display,
            window,
            gc,
            0,
            0,
            width,
            height,
        );

        xlib::XFlush(
            display
        );
    }

    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] XFCE solid-color presentation test drawn: window=0x{:X}, geometry={}x{}, color=magenta",
            window,
            width,
            height,
        ),
    );

    println!(
        "XFCE solid-color presentation test drawn: 0x{:X}, {}x{}, magenta",
        window,
        width,
        height,
    );

    thread::sleep(
        Duration::from_secs(10)
    );

    unsafe {
        xlib::XFreeGC(
            display,
            gc,
        );
    }

    Ok(())
}
