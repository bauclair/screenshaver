//! present_authentication_xfce.rs
//!
//! Presentation-only helper for the Xfce authentication screen.
//!
//! Xfce remains the security boundary. xfce4-screensaver and
//! xfce4-screensaver-dialog retain all input handling, PAM authentication,
//! retry behavior, and unlock authority. This module only creates a passive,
//! input-transparent X11 child surface above the native authentication dialog
//! and draws the existing Screenshaver lock-screen widget into that surface.
//!
//! The normal Screenshaver saver child cannot perform this work because
//! xfce4-screensaver deliberately stops that child while authentication is
//! visible. The saver child therefore launches a detached copy of Screenshaver
//! in this helper mode before entering its render loop.

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
use std::time::Duration;

use x11::glx;
use x11::xlib;

const HELPER_ENVIRONMENT_VARIABLE: &str =
    "SCREENSHAVER_XFCE_AUTH_PRESENTATION_HELPER";

const PARENT_PID_ENVIRONMENT_VARIABLE: &str =
    "SCREENSHAVER_XFCE_AUTH_PRESENTATION_PARENT_PID";

const XSCREENSAVER_WINDOW_ENVIRONMENT_VARIABLE: &str =
    "XSCREENSAVER_WINDOW";

const AUTH_DIALOG_INSTANCE: &[u8] =
    b"xfce4-screensaver-dialog";

const AUTH_DIALOG_CLASS: &[u8] =
    b"Xfce4-screensaver-dialog";

const POLL_INTERVAL: Duration =
    Duration::from_millis(100);

const OVERLAY_SIZE: u32 =
    420;

const SHAPE_BOUNDING: c_int =
    0;

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
                "Unable to load libXext.so.6; passive X11 input shaping is unavailable."
                    .to_string()
            );
        }

        let query_extension =
            unsafe {
                load_symbol::<XShapeQueryExtension>(
                    library,
                    "XShapeQueryExtension",
                )
            };

        let query_extension =
            match query_extension {
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
            unsafe {
                load_symbol::<XShapeCombineRectangles>(
                    library,
                    "XShapeCombineRectangles",
                )
            };

        let combine_rectangles =
            match combine_rectangles {
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
                "The X11 Shape extension is unavailable; refusing to create an authentication overlay that could intercept input."
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


    fn apply_circular_bounding_shape(
        &self,
        display: *mut xlib::Display,
        window: xlib::Window,
    ) {
        let radius =
            (
                OVERLAY_SIZE as f32
                    * 0.5
            )
                - 4.0;

        let center =
            OVERLAY_SIZE as f32
                * 0.5;

        let mut rectangles =
            Vec::<ShapeRectangle>::with_capacity(
                OVERLAY_SIZE as usize
            );

        for y in
            0..OVERLAY_SIZE
        {
            let dy =
                y as f32
                    + 0.5
                    - center;

            let squared =
                radius * radius
                    - dy * dy;

            if squared <= 0.0 {
                continue;
            }

            let half_width =
                squared.sqrt();

            let left =
                (
                    center
                        - half_width
                )
                    .floor()
                    .max(
                        0.0
                    )
                    as i32;

            let right =
                (
                    center
                        + half_width
                )
                    .ceil()
                    .min(
                        OVERLAY_SIZE as f32
                    )
                    as i32;

            let width =
                (
                    right
                        - left
                )
                    .max(
                        0
                    ) as u16;

            if width == 0 {
                continue;
            }

            rectangles.push(
                ShapeRectangle {
                    x:
                        left as i16,

                    y:
                        y as i16,

                    width,

                    height:
                        1,
                }
            );
        }

        unsafe {
            (
                self.combine_rectangles
            )(
                display,
                window,
                SHAPE_BOUNDING,
                0,
                0,
                rectangles.as_mut_ptr(),
                rectangles.len() as c_int,
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


/// Return true only in the detached Xfce authentication-presentation helper.
pub(crate) fn is_helper_process(
) -> bool {
    env::var_os(
        HELPER_ENVIRONMENT_VARIABLE
    )
        .is_some()
}


/// Launch the detached authentication-presentation helper.
///
/// The helper is placed in its own session so a stop signal directed at the
/// normal saver child's process group cannot suspend presentation with it.
pub(crate) fn launch_helper(
    logfile: &Path,
) -> Result<(), String> {
    let executable =
        env::current_exe()
            .map_err(
                |error| {
                    format!(
                        "Unable to locate the Screenshaver executable for the XFCE authentication-presentation helper: {}",
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
                        "Unable to launch the XFCE authentication-presentation helper: {}",
                        error,
                    )
                }
            )?;

    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] XFCE authentication-presentation helper launched: pid={}",
            child.id(),
        ),
    );

    Ok(())
}


/// Run the detached Xfce authentication-presentation helper.
///
/// This function never reads keyboard or pointer input and never participates
/// in authentication. It only tracks Xfce's native authentication dialog and
/// maintains a passive visual child surface while that dialog is viewable.
pub(crate) fn run_helper(
    logfile: &Path,
) -> Result<(), String> {
    let parent_pid =
        env::var(
            PARENT_PID_ENVIRONMENT_VARIABLE
        )
        .map_err(
            |_| {
                "XFCE authentication-presentation helper parent PID is missing."
                    .to_string()
            }
        )?
        .parse::<libc::pid_t>()
        .map_err(
            |error| {
                format!(
                    "Invalid XFCE authentication-presentation helper parent PID: {}",
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
            "Unable to open the X11 display for XFCE authentication presentation."
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
                    "[LOCK] XFCE authentication-presentation helper started: parent_pid={}, geometry={}x{}",
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
                Option<AuthOverlay> =
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
                            let overlay_missing =
                                overlay
                                    .as_ref()
                                    .map(
                                        |overlay| {
                                            !window_exists(
                                                display,
                                                overlay.window
                                            )
                                        }
                                    )
                                    .unwrap_or(
                                        true
                                    );

                            if active_dialog
                                != dialog_window
                                || active_parent
                                    != dialog_parent
                                || overlay_missing
                            {
                                if let Some(existing) =
                                    overlay.take()
                                {
                                    existing.destroy(
                                        display
                                    );
                                }

                                match AuthOverlay::create(
                                    display,
                                    screen,
                                    root,
                                    dialog_parent,
                                    &shape_api,
                                ) {
                                    Ok(new_overlay) => {
                                        crate::logger::information(
                                            logfile,
                                            &format!(
                                                "[LOCK] XFCE static authentication widget mapped: dialog=0x{:X}, parent=0x{:X}, overlay=0x{:X}",
                                                dialog_window,
                                                dialog_parent,
                                                new_overlay.window,
                                            ),
                                        );

                                        overlay =
                                            Some(
                                                new_overlay
                                            );

                                        active_dialog =
                                            dialog_window;

                                        active_parent =
                                            dialog_parent;
                                    }

                                    Err(error) => {
                                        crate::logger::warning(
                                            logfile,
                                            &format!(
                                                "[LOCK] Unable to create XFCE static authentication widget: {}",
                                                error,
                                            ),
                                        );

                                        active_dialog =
                                            dialog_window;

                                        active_parent =
                                            dialog_parent;
                                    }
                                }
                            }

                            if let Some(current_overlay) =
                                overlay.as_ref()
                            {
                                if window_exists(
                                    display,
                                    current_overlay.window,
                                ) {
                                    unsafe {
                                        xlib::XRaiseWindow(
                                            display,
                                            current_overlay.window,
                                        );

                                        xlib::XFlush(
                                            display
                                        );
                                    }
                                }
                            }
                        }
                    }

                    None => {
                        if let Some(existing) =
                            overlay.take()
                        {
                            existing.destroy(
                                display
                            );

                            crate::logger::information(
                                logfile,
                                "[LOCK] XFCE authentication dialog hidden; static authentication widget removed.",
                            );
                        }

                        active_dialog =
                            0;

                        active_parent =
                            0;
                    }
                }

                thread::sleep(
                    POLL_INTERVAL
                );
            }

            if let Some(existing) =
                overlay.take()
            {
                existing.destroy(
                    display
                );
            }

            crate::logger::information(
                logfile,
                "[LOCK] XFCE authentication-presentation helper stopped because the saver child exited.",
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


struct AuthOverlay {
    window: xlib::Window,
    colormap: xlib::Colormap,

    context:
        Option<crate::glx_context::GlxContext>,

    renderer:
        Option<crate::lock_screen_widget::LockScreenWidgetRenderer>,

    widget:
        crate::lock_screen_widget::LockScreenWidget,
}


impl AuthOverlay {
    fn create(
        display: *mut xlib::Display,
        screen: c_int,
        root: xlib::Window,
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

        let framebuffer_config =
            crate::glx_context::GlxFramebufferConfig::choose(
                display,
                screen,
            )?;

        let visual_info =
            framebuffer_config
                .visual_info();

        let colormap =
            unsafe {
                xlib::XCreateColormap(
                    display,
                    root,
                    visual_info.visual,
                    xlib::AllocNone,
                )
            };

        if colormap == 0 {
            return Err(
                "Unable to create a colormap for the XFCE authentication widget."
                    .to_string()
            );
        }

        let overlay_width =
            OVERLAY_SIZE.min(
                parent_attributes.width as u32
            );

        let overlay_height =
            OVERLAY_SIZE.min(
                parent_attributes.height as u32
            );

        let overlay_x =
            (
                parent_attributes.width
                    - overlay_width as i32
            )
                / 2;

        let overlay_y =
            (
                parent_attributes.height
                    - overlay_height as i32
            )
                / 2;

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

        attributes.override_redirect =
            xlib::True;

        attributes.event_mask =
            0;

        let attribute_mask =
            xlib::CWBackPixel
                | xlib::CWBorderPixel
                | xlib::CWColormap
                | xlib::CWOverrideRedirect
                | xlib::CWEventMask;

        let window =
            unsafe {
                xlib::XCreateWindow(
                    display,
                    parent,
                    overlay_x,
                    overlay_y,
                    overlay_width,
                    overlay_height,
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
                "Unable to create the XFCE authentication widget X11 window."
                    .to_string()
            );
        }

        shape_api.make_input_transparent(
            display,
            window,
        );

        shape_api.apply_circular_bounding_shape(
            display,
            window,
        );

        unsafe {
            xlib::XMapRaised(
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
                        error
                    );
                }
            };

        if let Err(error) =
            context.make_current(
                display,
                window,
            )
        {
            context.destroy(
                display
            );

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
                error
            );
        }

        load_opengl_functions();

        let renderer =
            match crate::lock_screen_widget::LockScreenWidgetRenderer::new() {
                Ok(renderer) =>
                    renderer,

                Err(error) => {
                    let _ =
                        crate::glx_context::GlxContext::release_current(
                            display
                        );

                    context.destroy(
                        display
                    );

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
                        error
                    );
                }
            };

        let widget =
            crate::lock_screen_widget::LockScreenWidget::new();

        unsafe {
            gl::Viewport(
                0,
                0,
                overlay_width as i32,
                overlay_height as i32,
            );

            gl::ClearColor(
                0.0,
                0.0,
                0.0,
                0.0,
            );

            gl::Clear(
                gl::COLOR_BUFFER_BIT
            );
        }

        renderer.display_centered(
            &widget,
            overlay_width,
            overlay_height,
        );

        unsafe {
            glx::glXSwapBuffers(
                display,
                window,
            );

            xlib::XRaiseWindow(
                display,
                window,
            );

            xlib::XFlush(
                display
            );
        }

        Ok(
            Self {
                window,
                colormap,

                context:
                    Some(
                        context
                    ),

                renderer:
                    Some(
                        renderer
                    ),

                widget,
            }
        )
    }


    fn destroy(
        mut self,
        display: *mut xlib::Display,
    ) {
        let _ =
            &self.widget;

        if let Some(renderer) =
            self.renderer.take()
        {
            drop(
                renderer
            );
        }

        let _ =
            crate::glx_context::GlxContext::release_current(
                display
            );

        if let Some(context) =
            self.context.take()
        {
            context.destroy(
                display
            );
        }

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

            xlib::XSync(
                display,
                xlib::False,
            );
        }

        self.window =
            0;

        self.colormap =
            0;
    }
}


fn load_opengl_functions(
) {
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
