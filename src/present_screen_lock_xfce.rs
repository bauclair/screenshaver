use std::env;
use std::mem::MaybeUninit;
use std::path::Path;
use std::thread;
use std::time::Duration;

use x11::glx;
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
        "[LOCK] XFCE OpenGL presentation test: opening X11 display",
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

    let display = connection.display();
    let x11_window = window as xlib::Window;
    let mut attributes =
        MaybeUninit::<xlib::XWindowAttributes>::uninit();

    let status =
        unsafe {
            xlib::XGetWindowAttributes(
                display,
                x11_window,
                attributes.as_mut_ptr(),
            )
        };

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

    if attributes.width <= 0 || attributes.height <= 0 {
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

    run_opengl_clear_test(
        logfile,
        &connection,
        x11_window,
        attributes.width,
        attributes.height,
    )
}


fn run_opengl_clear_test(
    logfile: &Path,
    connection: &crate::x11_connection::X11Connection,
    window: xlib::Window,
    width: i32,
    height: i32,
) -> Result<(), String> {
    let display = connection.display();
    let screen = connection.screen();

    crate::logger::information(
        logfile,
        "[LOCK] XFCE OpenGL presentation test: choosing GLX framebuffer configuration",
    );

    let framebuffer_config =
        crate::glx_context::GlxFramebufferConfig::choose(
            display,
            screen,
        )
        .map_err(|error| {
            format!(
                "Unable to choose GLX framebuffer configuration for XFCE lock presentation: {}",
                error,
            )
        })?;

    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] XFCE OpenGL presentation test: selected GLX visual 0x{:X}",
            framebuffer_config.visual_info().visualid,
        ),
    );

    let context =
        crate::glx_context::GlxContext::create(
            display,
            &framebuffer_config,
        )
        .map_err(|error| {
            format!(
                "Unable to create GLX context for XFCE lock presentation: {}",
                error,
            )
        })?;

    crate::logger::information(
        logfile,
        "[LOCK] XFCE OpenGL presentation test: making context current on supplied window",
    );

    if let Err(error) = context.make_current(display, window) {
        context.destroy(display);

        return Err(
            format!(
                "Unable to make GLX context current on XFCE presentation window 0x{:X}: {}",
                window,
                error,
            )
        );
    }

    crate::logger::information(
        logfile,
        "[LOCK] XFCE OpenGL presentation test: GLX context is current",
    );

    gl::load_with(
        |symbol| {
            let symbol =
                std::ffi::CString::new(symbol)
                    .expect("OpenGL symbol name contained an interior NUL");

            unsafe {
                glx::glXGetProcAddress(
                    symbol.as_ptr() as *const u8
                )
                .map_or(
                    std::ptr::null(),
                    |function| {
                        function as *const () as *const std::ffi::c_void
                    },
                )
            }
        }
    );

    unsafe {
        gl::Viewport(0, 0, width, height);
        gl::ClearColor(0.0, 1.0, 1.0, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
        glx::glXSwapBuffers(display, window);
    }

    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] XFCE OpenGL presentation test drawn: window=0x{:X}, geometry={}x{}, color=cyan",
            window,
            width,
            height,
        ),
    );

    thread::sleep(Duration::from_secs(10));

    crate::glx_context::GlxContext::release_current(display)
        .map_err(|error| {
            format!(
                "Unable to release XFCE lock presentation GLX context: {}",
                error,
            )
        })?;

    context.destroy(display);

    crate::logger::information(
        logfile,
        "[LOCK] XFCE OpenGL presentation test completed",
    );

    Ok(())
}
