//! present_authentication_xfce.rs
//!
//! Xfce GLX 160x100 geometry authentication-child diagnostic.
//!
//! This diagnostic keeps the previously-proven GLX/input-transparent behavior,
//! 100 ms XRaiseWindow() cadence, and 500 ms cyan rendering cadence, while
//! changing exactly one variable: the diagnostic child geometry now matches the
//! previously-visible standalone X11 overlay test at 160x100+50+50.
//!
//! No lock widget or authentication input handling is introduced.
//!
//! This diagnostic intentionally does NOT:
//! - render the Screenshaver lock widget,
//! - read keyboard or pointer input,
//! - grab input,
//! - authenticate,
//! - repeatedly raise or restack its X11 child window.
//!
//! XRaiseWindow() is called every 100 ms while the dialog is visible.
//! OpenGL rendering repeats at most once every 500 ms.
//!
//! Its purpose is to determine whether introducing GLX/OpenGL on the otherwise
//! passive authentication child interferes with xfce4-screensaver-dialog.
//!
//! Xfce remains the complete security boundary.

use std::env;
use std::ffi::{
    CStr,
    CString,
};
use std::os::raw::{
    c_char,
    c_int,
    c_void,
};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{
    Command,
    Stdio,
};
use std::ptr;
use std::thread;
use std::time::{
    Duration,
    Instant,
};

use x11::glx;
use x11::xlib;


const HELPER_ENVIRONMENT_VARIABLE: &str =
    "SCREENSHAVER_XFCE_AUTH_CHILD_TEST_HELPER";

const PARENT_PID_ENVIRONMENT_VARIABLE: &str =
    "SCREENSHAVER_XFCE_AUTH_CHILD_TEST_PARENT_PID";

const XSCREENSAVER_WINDOW_ENVIRONMENT_VARIABLE: &str =
    "XSCREENSAVER_WINDOW";

const AUTH_DIALOG_INSTANCE: &[u8] =
    b"xfce4-screensaver-dialog";

const AUTH_DIALOG_CLASS: &[u8] =
    b"Xfce4-screensaver-dialog";

const POLL_INTERVAL: Duration =
    Duration::from_millis(100);

const RENDER_INTERVAL: Duration =
    Duration::from_millis(500);

const RAISE_INTERVAL: Duration =
    Duration::from_millis(100);

const TEST_WINDOW_WIDTH: u32 =
    160;

const TEST_WINDOW_HEIGHT: u32 =
    100;

const TEST_WINDOW_X: i32 =
    50;

const TEST_WINDOW_Y: i32 =
    50;

const SHAPE_INPUT: c_int =
    2;

const SHAPE_SET: c_int =
    0;

const SHAPE_UNSORTED: c_int =
    0;


#[repr(C)]
#[derive(Clone, Copy)]
struct ShapeRectangle {
    x: i16,
    y: i16,
    width: u16,
    height: u16,
}


type XShapeQueryExtension =
    unsafe extern "C" fn(
        *mut xlib::Display,
        *mut c_int,
        *mut c_int,
    ) -> c_int;

type XShapeCombineRectangles =
    unsafe extern "C" fn(
        *mut xlib::Display,
        xlib::Window,
        c_int,
        c_int,
        c_int,
        *mut ShapeRectangle,
        c_int,
        c_int,
        c_int,
    );


struct ShapeApi {
    library: *mut c_void,
    query_extension: XShapeQueryExtension,
    combine_rectangles: XShapeCombineRectangles,
}


impl ShapeApi {
    fn load() -> Result<Self, String> {
        let library_name =
            CString::new(
                "libXext.so.6"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare Xext library name: {}",
                        error,
                    )
                }
            )?;

        let library =
            unsafe {
                libc::dlopen(
                    library_name.as_ptr(),
                    libc::RTLD_LAZY
                        | libc::RTLD_LOCAL,
                )
            };

        if library.is_null() {
            return Err(
                "Unable to load libXext.so.6; refusing to create the diagnostic child without X11 Shape input transparency."
                    .to_string()
            );
        }

        let query_extension =
            match unsafe {
                load_symbol::<XShapeQueryExtension>(
                    library,
                    "XShapeQueryExtension",
                )
            } {
                Ok(function) =>
                    function,

                Err(error) => {
                    unsafe {
                        libc::dlclose(
                            library
                        );
                    }

                    return Err(
                        error
                    );
                }
            };

        let combine_rectangles =
            match unsafe {
                load_symbol::<XShapeCombineRectangles>(
                    library,
                    "XShapeCombineRectangles",
                )
            } {
                Ok(function) =>
                    function,

                Err(error) => {
                    unsafe {
                        libc::dlclose(
                            library
                        );
                    }

                    return Err(
                        error
                    );
                }
            };

        Ok(
            Self {
                library,
                query_extension,
                combine_rectangles,
            }
        )
    }


    fn verify(
        &self,
        display: *mut xlib::Display,
    ) -> Result<(), String> {
        let mut event_base =
            0;

        let mut error_base =
            0;

        let available =
            unsafe {
                (
                    self.query_extension
                )(
                    display,
                    &mut event_base,
                    &mut error_base,
                )
            };

        if available == 0 {
            Err(
                "The X11 Shape extension is unavailable; refusing to create the diagnostic child because it could intercept pointer input."
                    .to_string()
            )
        } else {
            Ok(())
        }
    }


    fn make_input_transparent(
        &self,
        display: *mut xlib::Display,
        window: xlib::Window,
    ) {
        unsafe {
            (
                self.combine_rectangles
            )(
                display,
                window,
                SHAPE_INPUT,
                0,
                0,
                ptr::null_mut(),
                0,
                SHAPE_SET,
                SHAPE_UNSORTED,
            );
        }
    }
}


impl Drop
    for ShapeApi
{
    fn drop(
        &mut self,
    ) {
        if !self.library.is_null() {
            unsafe {
                libc::dlclose(
                    self.library
                );
            }

            self.library =
                ptr::null_mut();
        }
    }
}


unsafe fn load_symbol<T>(
    library: *mut c_void,
    symbol_name: &str,
) -> Result<T, String>
where
    T: Copy,
{
    let symbol_name_c =
        CString::new(
            symbol_name
        )
        .map_err(
            |error| {
                format!(
                    "Unable to prepare Xext symbol name '{}': {}",
                    symbol_name,
                    error,
                )
            }
        )?;

    let symbol =
        unsafe {
            libc::dlsym(
                library,
                symbol_name_c.as_ptr(),
            )
        };

    if symbol.is_null() {
        return Err(
            format!(
                "Unable to resolve Xext symbol '{}'.",
                symbol_name,
            )
        );
    }

    Ok(
        unsafe {
            std::mem::transmute_copy::<*mut c_void, T>(
                &symbol
            )
        }
    )
}


/// True only inside the detached diagnostic helper process.
pub(crate) fn is_helper_process(
) -> bool {
    env::var_os(
        HELPER_ENVIRONMENT_VARIABLE
    )
        .is_some()
}


/// Launch a detached diagnostic helper before Xfce suspends the normal saver
/// presentation child.
pub(crate) fn launch_helper(
    logfile: &Path,
) -> Result<(), String> {
    let executable =
        env::current_exe()
            .map_err(
                |error| {
                    format!(
                        "Unable to locate the Screenshaver executable for the XFCE authentication-child diagnostic: {}",
                        error,
                    )
                }
            )?;

    let parent_pid =
        std::process::id();

    let mut command =
        Command::new(
            executable
        );

    command
        .env(
            HELPER_ENVIRONMENT_VARIABLE,
            "1",
        )
        .env(
            PARENT_PID_ENVIRONMENT_VARIABLE,
            parent_pid.to_string(),
        )
        .env_remove(
            XSCREENSAVER_WINDOW_ENVIRONMENT_VARIABLE
        )
        .stdin(
            Stdio::null()
        )
        .stdout(
            Stdio::null()
        )
        .stderr(
            Stdio::null()
        );

    unsafe {
        command.pre_exec(
            || {
                if libc::setsid() < 0 {
                    return Err(
                        std::io::Error::last_os_error()
                    );
                }

                Ok(())
            }
        );
    }

    let child =
        command
            .spawn()
            .map_err(
                |error| {
                    format!(
                        "Unable to launch the XFCE authentication-child diagnostic helper: {}",
                        error,
                    )
                }
            )?;

    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] XFCE GLX 160x100 authentication-child diagnostic helper launched: pid={}",
            child.id(),
        ),
    );

    Ok(())
}


/// Run the detached minimal GLX diagnostic.
///
/// A 16x16 child is created once when the native authentication dialog appears.
/// The child has no selected events and an empty ShapeInput region. It is mapped
/// once and is never raised again. A GLX context is made current only long
/// enough to clear the child once and swap buffers once.
pub(crate) fn run_helper(
    logfile: &Path,
) -> Result<(), String> {
    let parent_pid =
        env::var(
            PARENT_PID_ENVIRONMENT_VARIABLE
        )
        .map_err(
            |_| {
                "XFCE authentication-child diagnostic parent PID is missing."
                    .to_string()
            }
        )?
        .parse::<libc::pid_t>()
        .map_err(
            |error| {
                format!(
                    "Invalid XFCE authentication-child diagnostic parent PID: {}",
                    error,
                )
            }
        )?;

    let display =
        unsafe {
            xlib::XOpenDisplay(
                ptr::null()
            )
        };

    if display.is_null() {
        return Err(
            "Unable to open the X11 display for the XFCE authentication-child diagnostic."
                .to_string()
        );
    }

    unsafe {
        xlib::XSetErrorHandler(
            Some(
                ignore_x11_error
            )
        );
    }

    let result =
        (|| -> Result<(), String> {
            let shape_api =
                ShapeApi::load()?;

            shape_api.verify(
                display
            )?;

            let screen =
                unsafe {
                    xlib::XDefaultScreen(
                        display
                    )
                };

            let root =
                unsafe {
                    xlib::XRootWindow(
                        display,
                        screen,
                    )
                };

            let root_width =
                unsafe {
                    xlib::XDisplayWidth(
                        display,
                        screen,
                    )
                }
                    .max(
                        1
                    );

            let root_height =
                unsafe {
                    xlib::XDisplayHeight(
                        display,
                        screen,
                    )
                }
                    .max(
                        1
                    );

            crate::logger::information(
                logfile,
                &format!(
                    "[LOCK] XFCE GLX 160x100 authentication-child diagnostic helper started: parent_pid={}, geometry={}x{}",
                    parent_pid,
                    root_width,
                    root_height,
                ),
            );

            let mut active_dialog:
                xlib::Window =
                0;

            let mut active_parent:
                xlib::Window =
                0;

            let mut overlay:
                Option<GlxTestOverlay> =
                None;

            let mut last_render:
                Option<Instant> =
                None;

            let mut last_raise:
                Option<Instant> =
                None;

            while process_exists(
                parent_pid
            ) {
                let dialog =
                    find_authentication_dialog(
                        display,
                        root,
                        root_width,
                        root_height,
                    );

                match dialog {
                    Some(dialog_window) => {
                        let dialog_parent =
                            query_parent(
                                display,
                                dialog_window,
                            );

                        if let Some(dialog_parent) =
                            dialog_parent
                        {
                            let relationship_changed =
                                active_dialog
                                    != dialog_window
                                || active_parent
                                    != dialog_parent;

                            let child_missing =
                                overlay
                                    .as_ref()
                                    .map(
                                        |overlay| {
                                            !window_exists(
                                                display,
                                                overlay.window,
                                            )
                                        }
                                    )
                                    .unwrap_or(
                                        true
                                    );

                            if relationship_changed
                                || child_missing
                            {
                                if let Some(existing_overlay) =
                                    overlay.take()
                                {
                                    let old_window =
                                        existing_overlay.window;

                                    existing_overlay.destroy(
                                        display
                                    );

                                    crate::logger::information(
                                        logfile,
                                        &format!(
                                            "[LOCK] XFCE GLX 160x100 authentication-child diagnostic window destroyed before recreation: window=0x{:X}",
                                            old_window,
                                        ),
                                    );
                                }

                                let new_overlay =
                                    GlxTestOverlay::create(
                                        display,
                                        dialog_parent,
                                        &shape_api,
                                    )?;

                                let new_window =
                                    new_overlay.window;

                                overlay =
                                    Some(
                                        new_overlay
                                    );

                                active_dialog =
                                    dialog_window;

                                active_parent =
                                    dialog_parent;

                                last_render =
                                    None;

                                last_raise =
                                    Some(
                                        Instant::now()
                                    );

                                crate::logger::information(
                                    logfile,
                                    &format!(
                                        "[LOCK] XFCE GLX 160x100 authentication-child diagnostic window mapped once: dialog=0x{:X}, parent=0x{:X}, window=0x{:X}, geometry={}x{}+{}+{}, input_shape=empty, event_mask=0, raise=every-100ms, glx_clear=cyan-every-500ms",
                                        dialog_window,
                                        dialog_parent,
                                        new_window,
                                        TEST_WINDOW_WIDTH,
                                        TEST_WINDOW_HEIGHT,
                                        TEST_WINDOW_X,
                                        TEST_WINDOW_Y,
                                    ),
                                );
                            }

                            if let Some(current_overlay) =
                                overlay.as_ref()
                            {
                                let should_raise =
                                    last_raise
                                        .map(
                                            |instant| {
                                                instant.elapsed()
                                                    >= RAISE_INTERVAL
                                            }
                                        )
                                        .unwrap_or(
                                            true
                                        );

                                if should_raise {
                                    unsafe {
                                        xlib::XRaiseWindow(
                                            display,
                                            current_overlay.window,
                                        );

                                        xlib::XFlush(
                                            display
                                        );
                                    }

                                    last_raise =
                                        Some(
                                            Instant::now()
                                        );
                                }

                                let should_render =
                                    last_render
                                        .map(
                                            |instant| {
                                                instant.elapsed()
                                                    >= RENDER_INTERVAL
                                            }
                                        )
                                        .unwrap_or(
                                            true
                                        );

                                if should_render {
                                    current_overlay.render_cyan(
                                        display
                                    )?;

                                    last_render =
                                        Some(
                                            Instant::now()
                                        );
                                }
                            }
                        }
                    }

                    None => {
                        if let Some(existing_overlay) =
                            overlay.take()
                        {
                            let old_window =
                                existing_overlay.window;

                            existing_overlay.destroy(
                                display
                            );

                            crate::logger::information(
                                logfile,
                                &format!(
                                    "[LOCK] XFCE authentication dialog not viewable; minimal GLX 500ms-render diagnostic window destroyed: window=0x{:X}",
                                    old_window,
                                ),
                            );
                        }

                        active_dialog =
                            0;

                        active_parent =
                            0;

                        last_render =
                            None;

                        last_raise =
                            None;
                    }
                }

                thread::sleep(
                    POLL_INTERVAL
                );
            }

            if let Some(existing_overlay) =
                overlay.take()
            {
                existing_overlay.destroy(
                    display
                );
            }

            crate::logger::information(
                logfile,
                "[LOCK] XFCE GLX 160x100 authentication-child diagnostic helper stopped because the saver presentation child exited.",
            );

            Ok(())
        })();

    unsafe {
        xlib::XCloseDisplay(
            display
        );
    }

    result
}


struct GlxTestOverlay {
    window: xlib::Window,
    colormap: xlib::Colormap,
    context: crate::glx_context::GlxContext,
}


impl GlxTestOverlay {
    fn create(
        display: *mut xlib::Display,
        parent: xlib::Window,
        shape_api: &ShapeApi,
    ) -> Result<Self, String> {
        let parent_attributes =
            window_attributes(
                display,
                parent,
            )
            .ok_or_else(
                || {
                    format!(
                        "Unable to query XFCE authentication container 0x{:X}.",
                        parent,
                    )
                }
            )?;

        if parent_attributes.width <= 0
            || parent_attributes.height <= 0
        {
            return Err(
                format!(
                    "XFCE authentication container 0x{:X} has invalid geometry {}x{}.",
                    parent,
                    parent_attributes.width,
                    parent_attributes.height,
                )
            );
        }

        let screen =
            unsafe {
                xlib::XDefaultScreen(
                    display
                )
            };

        let framebuffer_config =
            crate::glx_context::GlxFramebufferConfig::choose(
                display,
                screen,
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to choose a GLX framebuffer configuration for the XFCE authentication-child diagnostic: {}",
                        error,
                    )
                }
            )?;

        let visual_info =
            framebuffer_config
                .visual_info();

        let colormap =
            unsafe {
                xlib::XCreateColormap(
                    display,
                    parent,
                    visual_info.visual,
                    xlib::AllocNone,
                )
            };

        if colormap == 0 {
            return Err(
                "Unable to create a colormap for the XFCE GLX authentication-child diagnostic."
                    .to_string()
            );
        }

        let mut attributes:
            xlib::XSetWindowAttributes =
            unsafe {
                std::mem::zeroed()
            };

        attributes.background_pixel =
            0;

        attributes.border_pixel =
            0;

        attributes.colormap =
            colormap;

        attributes.event_mask =
            0;

        let attribute_mask =
            xlib::CWBackPixel
                | xlib::CWBorderPixel
                | xlib::CWColormap
                | xlib::CWEventMask;

        let window =
            unsafe {
                xlib::XCreateWindow(
                    display,
                    parent,
                    TEST_WINDOW_X,
                    TEST_WINDOW_Y,
                    TEST_WINDOW_WIDTH,
                    TEST_WINDOW_HEIGHT,
                    0,
                    visual_info.depth,
                    xlib::InputOutput as u32,
                    visual_info.visual,
                    attribute_mask,
                    &mut attributes,
                )
            };

        if window == 0 {
            unsafe {
                xlib::XFreeColormap(
                    display,
                    colormap,
                );
            }

            return Err(
                "XCreateWindow() failed for the XFCE GLX authentication-child diagnostic."
                    .to_string()
            );
        }

        shape_api.make_input_transparent(
            display,
            window,
        );

        unsafe {
            xlib::XMapWindow(
                display,
                window,
            );

            xlib::XRaiseWindow(
                display,
                window,
            );

            xlib::XSync(
                display,
                xlib::False,
            );
        }

        let context =
            match crate::glx_context::GlxContext::create(
                display,
                &framebuffer_config,
            ) {
                Ok(context) =>
                    context,

                Err(error) => {
                    unsafe {
                        xlib::XDestroyWindow(
                            display,
                            window,
                        );

                        xlib::XFreeColormap(
                            display,
                            colormap,
                        );
                    }

                    return Err(
                        format!(
                            "Unable to create the persistent GLX context for the XFCE authentication-child diagnostic: {}",
                            error,
                        )
                    );
                }
            };

        gl::load_with(
            |symbol| {
                let symbol =
                    CString::new(
                        symbol
                    )
                    .expect(
                        "OpenGL symbol name contained an interior NUL"
                    );

                unsafe {
                    glx::glXGetProcAddress(
                        symbol.as_ptr()
                            as *const u8
                    )
                    .map_or(
                        ptr::null(),
                        |function| {
                            function
                                as *const ()
                                as *const c_void
                        },
                    )
                }
            }
        );

        Ok(
            Self {
                window,
                colormap,
                context,
            }
        )
    }


    fn render_cyan(
        &self,
        display: *mut xlib::Display,
    ) -> Result<(), String> {
        self.context
            .make_current(
                display,
                self.window,
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to make the persistent XFCE authentication-child GLX context current: {}",
                        error,
                    )
                }
            )?;

        unsafe {
            gl::Viewport(
                0,
                0,
                TEST_WINDOW_WIDTH as i32,
                TEST_WINDOW_HEIGHT as i32,
            );

            gl::ClearColor(
                0.0,
                1.0,
                1.0,
                1.0,
            );

            gl::Clear(
                gl::COLOR_BUFFER_BIT
            );

            glx::glXSwapBuffers(
                display,
                self.window,
            );

            xlib::XFlush(
                display
            );
        }

        crate::glx_context::GlxContext::release_current(
            display
        )
        .map_err(
            |error| {
                format!(
                    "Unable to release the persistent XFCE authentication-child GLX context after rendering: {}",
                    error,
                )
            }
        )?;

        Ok(())
    }


    fn destroy(
        self,
        display: *mut xlib::Display,
    ) {
        let _ =
            crate::glx_context::GlxContext::release_current(
                display
            );

        self.context.destroy(
            display
        );

        unsafe {
            if self.window != 0 {
                xlib::XDestroyWindow(
                    display,
                    self.window,
                );
            }

            if self.colormap != 0 {
                xlib::XFreeColormap(
                    display,
                    self.colormap,
                );
            }

            xlib::XFlush(
                display
            );
        }
    }
}

fn find_authentication_dialog(
    display: *mut xlib::Display,
    root: xlib::Window,
    root_width: i32,
    root_height: i32,
) -> Option<xlib::Window> {
    find_authentication_dialog_recursive(
        display,
        root,
        0,
        root_width,
        root_height,
    )
}


fn find_authentication_dialog_recursive(
    display: *mut xlib::Display,
    window: xlib::Window,
    depth: usize,
    root_width: i32,
    root_height: i32,
) -> Option<xlib::Window> {
    if depth > 8 {
        return None;
    }

    if is_authentication_dialog(
        display,
        window,
        root_width,
        root_height,
    ) {
        return Some(
            window
        );
    }

    let mut root_return:
        xlib::Window =
        0;

    let mut parent_return:
        xlib::Window =
        0;

    let mut children:
        *mut xlib::Window =
        ptr::null_mut();

    let mut child_count:
        u32 =
        0;

    let succeeded =
        unsafe {
            xlib::XQueryTree(
                display,
                window,
                &mut root_return,
                &mut parent_return,
                &mut children,
                &mut child_count,
            )
        };

    if succeeded == 0 {
        return None;
    }

    let mut result =
        None;

    if !children.is_null() {
        let child_slice =
            unsafe {
                std::slice::from_raw_parts(
                    children,
                    child_count as usize,
                )
            };

        for child in
            child_slice
        {
            result =
                find_authentication_dialog_recursive(
                    display,
                    *child,
                    depth + 1,
                    root_width,
                    root_height,
                );

            if result.is_some() {
                break;
            }
        }

        unsafe {
            xlib::XFree(
                children as *mut c_void
            );
        }
    }

    result
}


fn is_authentication_dialog(
    display: *mut xlib::Display,
    window: xlib::Window,
    root_width: i32,
    root_height: i32,
) -> bool {
    let attributes =
        match window_attributes(
            display,
            window,
        ) {
            Some(attributes) =>
                attributes,

            None =>
                return false,
        };

    if attributes.map_state
        != xlib::IsViewable
    {
        return false;
    }

    if attributes.width
        < root_width * 2 / 3
        || attributes.height
            < root_height * 2 / 3
    {
        return false;
    }

    let mut class_hint:
        xlib::XClassHint =
        unsafe {
            std::mem::zeroed()
        };

    let has_class_hint =
        unsafe {
            xlib::XGetClassHint(
                display,
                window,
                &mut class_hint,
            )
        };

    if has_class_hint == 0 {
        return false;
    }

    let instance_matches =
        c_string_matches(
            class_hint.res_name,
            AUTH_DIALOG_INSTANCE,
        );

    let class_matches =
        c_string_matches(
            class_hint.res_class,
            AUTH_DIALOG_CLASS,
        );

    unsafe {
        if !class_hint.res_name.is_null() {
            xlib::XFree(
                class_hint.res_name
                    as *mut c_void
            );
        }

        if !class_hint.res_class.is_null() {
            xlib::XFree(
                class_hint.res_class
                    as *mut c_void
            );
        }
    }

    instance_matches
        && class_matches
}


fn c_string_matches(
    value: *mut c_char,
    expected: &[u8],
) -> bool {
    if value.is_null() {
        return false;
    }

    unsafe {
        CStr::from_ptr(
            value
        )
            .to_bytes()
            == expected
    }
}


fn query_parent(
    display: *mut xlib::Display,
    window: xlib::Window,
) -> Option<xlib::Window> {
    let mut root_return:
        xlib::Window =
        0;

    let mut parent_return:
        xlib::Window =
        0;

    let mut children:
        *mut xlib::Window =
        ptr::null_mut();

    let mut child_count:
        u32 =
        0;

    let succeeded =
        unsafe {
            xlib::XQueryTree(
                display,
                window,
                &mut root_return,
                &mut parent_return,
                &mut children,
                &mut child_count,
            )
        };

    if !children.is_null() {
        unsafe {
            xlib::XFree(
                children as *mut c_void
            );
        }
    }

    if succeeded == 0
        || parent_return == 0
    {
        None
    } else {
        Some(
            parent_return
        )
    }
}


fn window_attributes(
    display: *mut xlib::Display,
    window: xlib::Window,
) -> Option<xlib::XWindowAttributes> {
    let mut attributes:
        xlib::XWindowAttributes =
        unsafe {
            std::mem::zeroed()
        };

    let succeeded =
        unsafe {
            xlib::XGetWindowAttributes(
                display,
                window,
                &mut attributes,
            )
        };

    if succeeded == 0 {
        None
    } else {
        Some(
            attributes
        )
    }
}


fn window_exists(
    display: *mut xlib::Display,
    window: xlib::Window,
) -> bool {
    window != 0
        && window_attributes(
            display,
            window,
        )
            .is_some()
}


fn process_exists(
    pid: libc::pid_t,
) -> bool {
    if pid <= 0 {
        return false;
    }

    let result =
        unsafe {
            libc::kill(
                pid,
                0,
            )
        };

    if result == 0 {
        return true;
    }

    std::io::Error::last_os_error()
        .raw_os_error()
        == Some(
            libc::EPERM
        )
}


unsafe extern "C" fn ignore_x11_error(
    _display: *mut xlib::Display,
    _error: *mut xlib::XErrorEvent,
) -> c_int {
    0
}
