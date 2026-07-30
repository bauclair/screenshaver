// x11_wallpaper.rs
//
// Native X11 wallpaper backend using a GLX-selected visual.
//
// Stage 4D:
//
// ✓ Selects a modern GLX framebuffer configuration.
// ✓ Creates the X11 window with the visual required by that configuration.
// ✓ Creates an explicit GLXWindow drawable for the X11 window.
// ✓ Applies EWMH desktop-window semantics.
// ✓ Creates and activates a GLX context.
// ✓ Loads OpenGL functions through GLX.
// ✓ Logs the OpenGL vendor, renderer, version, and GLSL version.
// ✓ Clears the drawable to a diagnostic blue color.
// ✓ Presents the OpenGL back buffer with glXSwapBuffers().
// ✓ Releases GLX and X11 resources in dependency order.

use std::ffi::{
    CStr,
    CString,
};
use std::path::Path;
use std::sync::{
    atomic::AtomicBool,
    Arc,
};
use std::thread;
use std::time::Duration;

use x11::{
    glx,
    xlib,
};

use crate::define_wallpaper::WallpaperRuntime;
use crate::glx_context::{
    GlxContext,
    GlxFramebufferConfig,
};
use crate::manage_shader::ShaderManager;
use crate::manage_wallpaper_runtime::WallpaperRuntimeControl;
use crate::wallpaper_backend::WallpaperBackend;
use crate::x11_connection::X11Connection;

pub struct X11WallpaperBackend {
    connection: X11Connection,
}

fn diagnostic(message: &str) {
    println!("{message}");
}

fn diagnostic_value(label: &str, value: impl std::fmt::Display) {
    diagnostic(&format!("{label}{value}"));
}

impl X11WallpaperBackend {
    pub fn new() -> Result<Self, String> {
        diagnostic("Probing native X11 wallpaper capabilities...");
        let connection = X11Connection::connect()?;
        Ok(Self { connection })
    }
}

struct X11WallpaperWindow {
    window: xlib::Window,
    glx_window: glx::GLXWindow,
    colormap: xlib::Colormap,
    width: i32,
    height: i32,
}

fn intern_atom(
    display: *mut xlib::Display,
    name: &str,
) -> Result<xlib::Atom, String> {
    let name = CString::new(name)
        .map_err(|_| format!("Invalid atom name: {name}"))?;

    let atom =
        unsafe { xlib::XInternAtom(display, name.as_ptr(), xlib::False) };

    if atom == 0 {
        Err(format!("Unable to resolve X11 atom '{:?}'", name))
    } else {
        Ok(atom)
    }
}

fn set_atom_property(
    display: *mut xlib::Display,
    window: xlib::Window,
    property: xlib::Atom,
    values: &[xlib::Atom],
) {
    unsafe {
        xlib::XChangeProperty(
            display,
            window,
            property,
            xlib::XA_ATOM,
            32,
            xlib::PropModeReplace,
            values.as_ptr() as *const u8,
            values.len() as i32,
        );
    }
}

fn create_wallpaper_window(
    connection: &X11Connection,
    glx_config: &GlxFramebufferConfig,
) -> Result<X11WallpaperWindow, String> {
    unsafe {
        let display = connection.display();

        if display.is_null() {
            return Err(
                "Cannot create an X11 wallpaper window with a null display."
                    .to_string(),
            );
        }

        let width = connection.width() as u32;
        let height = connection.height() as u32;
        let visual_info = glx_config.visual_info();

        diagnostic("Interning EWMH atoms...");

        let wm_type = intern_atom(display, "_NET_WM_WINDOW_TYPE")?;
        let wm_type_desktop =
            intern_atom(display, "_NET_WM_WINDOW_TYPE_DESKTOP")?;
        let wm_state = intern_atom(display, "_NET_WM_STATE")?;
        let wm_state_below =
            intern_atom(display, "_NET_WM_STATE_BELOW")?;
        let wm_state_skip_taskbar =
            intern_atom(display, "_NET_WM_STATE_SKIP_TASKBAR")?;
        let wm_state_skip_pager =
            intern_atom(display, "_NET_WM_STATE_SKIP_PAGER")?;

        diagnostic("Creating colormap for the GLX-compatible visual...");

        let colormap = xlib::XCreateColormap(
            display,
            connection.root_window(),
            visual_info.visual,
            xlib::AllocNone,
        );

        if colormap == 0 {
            return Err(
                "Unable to create an X11 colormap for the GLX visual."
                    .to_string(),
            );
        }

        let mut attributes: xlib::XSetWindowAttributes =
            std::mem::zeroed();

        attributes.background_pixel = 0x00000000;
        attributes.border_pixel = 0;
        attributes.colormap = colormap;
        attributes.event_mask =
            xlib::ExposureMask
                | xlib::StructureNotifyMask
                | xlib::KeyPressMask;

        let attribute_mask =
            xlib::CWBackPixel
                | xlib::CWBorderPixel
                | xlib::CWColormap
                | xlib::CWEventMask;

        let window = xlib::XCreateWindow(
            display,
            connection.root_window(),
            0,
            0,
            width,
            height,
            0,
            visual_info.depth,
            xlib::InputOutput as u32,
            visual_info.visual,
            attribute_mask,
            &mut attributes,
        );

        if window == 0 {
            xlib::XFreeColormap(display, colormap);

            return Err(
                "Unable to create native X11 wallpaper window with the GLX visual."
                    .to_string(),
            );
        }

        diagnostic("Creating GLXWindow drawable...");

        let glx_window = glx::glXCreateWindow(
            display,
            glx_config.fb_config(),
            window,
            std::ptr::null(),
        );

        if glx_window == 0 {
            xlib::XDestroyWindow(display, window);
            xlib::XFreeColormap(display, colormap);

            return Err("glXCreateWindow() failed.".to_string());
        }

        diagnostic("Applying desktop window hints...");

        set_atom_property(
            display,
            window,
            wm_type,
            &[wm_type_desktop],
        );

        set_atom_property(
            display,
            window,
            wm_state,
            &[
                wm_state_below,
                wm_state_skip_taskbar,
                wm_state_skip_pager,
            ],
        );

        xlib::XMapRaised(display, window);
        xlib::XSync(display, xlib::False);

        diagnostic(&format!(
            "Created native X11 wallpaper window {} and GLX drawable {} ({}x{})",
            window,
            glx_window,
            width,
            height,
        ));

        Ok(X11WallpaperWindow {
            window,
            glx_window,
            colormap,
            width: width as i32,
            height: height as i32,
        })
    }
}

fn destroy_wallpaper_window(
    connection: &X11Connection,
    wallpaper_window: X11WallpaperWindow,
) {
    unsafe {
        let display = connection.display();

        if wallpaper_window.glx_window != 0 {
            glx::glXDestroyWindow(
                display,
                wallpaper_window.glx_window,
            );
        }

        if wallpaper_window.window != 0 {
            xlib::XDestroyWindow(
                display,
                wallpaper_window.window,
            );
        }

        if wallpaper_window.colormap != 0 {
            xlib::XFreeColormap(
                display,
                wallpaper_window.colormap,
            );
        }

        xlib::XSync(display, xlib::False);
    }

    diagnostic("Closed X11 wallpaper window.");
}

fn load_opengl_functions() -> Result<(), String> {
    diagnostic("Loading OpenGL functions through GLX...");

    gl::load_with(|symbol| {
        let symbol = match CString::new(symbol) {
            Ok(symbol) => symbol,
            Err(_) => return std::ptr::null(),
        };

        unsafe {
            glx::glXGetProcAddress(symbol.as_ptr() as *const u8)
                .map_or(std::ptr::null(), |function| {
                    function as *const () as *const std::ffi::c_void
                })
        }
    });

    if !gl::Viewport::is_loaded()
        || !gl::ClearColor::is_loaded()
        || !gl::Clear::is_loaded()
        || !gl::Finish::is_loaded()
        || !gl::GetString::is_loaded()
        || !gl::GetError::is_loaded()
        || !gl::GetIntegerv::is_loaded()
        || !gl::ReadBuffer::is_loaded()
        || !gl::ReadPixels::is_loaded()
    {
        return Err(
            "GLX context became current, but required OpenGL functions could not be loaded."
                .to_string(),
        );
    }

    diagnostic("Loaded required OpenGL functions.");
    Ok(())
}

fn opengl_string(name: u32) -> String {
    let value = unsafe { gl::GetString(name) };

    if value.is_null() {
        return "<unavailable>".to_string();
    }

    unsafe {
        CStr::from_ptr(value as *const i8)
            .to_string_lossy()
            .into_owned()
    }
}

fn report_opengl_information() {
    diagnostic("OpenGL context information:");
    diagnostic_value("    Vendor: ", opengl_string(gl::VENDOR));
    diagnostic_value("    Renderer: ", opengl_string(gl::RENDERER));
    diagnostic_value("    Version: ", opengl_string(gl::VERSION));
    diagnostic_value(
        "    GLSL version: ",
        opengl_string(gl::SHADING_LANGUAGE_VERSION),
    );
}

fn render_diagnostic_frame(
    display: *mut xlib::Display,
    wallpaper_window: &X11WallpaperWindow,
) -> Result<(), String> {
    diagnostic("Rendering diagnostic OpenGL frame...");

    unsafe {
        // Remove any stale error so every reported value belongs to this test.
        while gl::GetError() != gl::NO_ERROR {}

        let mut viewport = [0_i32; 4];
        gl::GetIntegerv(gl::VIEWPORT, viewport.as_mut_ptr());

        diagnostic(&format!(
            "OpenGL viewport before update: {}, {}, {}, {}",
            viewport[0], viewport[1], viewport[2], viewport[3],
        ));

        gl::Viewport(
            0,
            0,
            wallpaper_window.width,
            wallpaper_window.height,
        );

        let viewport_error = gl::GetError();
        diagnostic(&format!(
            "glViewport error: 0x{viewport_error:04X}"
        ));

        gl::ClearColor(0.0, 0.25, 1.0, 1.0);

        let clear_color_error = gl::GetError();
        diagnostic(&format!(
            "glClearColor error: 0x{clear_color_error:04X}"
        ));

        gl::Clear(gl::COLOR_BUFFER_BIT);

        let clear_error = gl::GetError();
        diagnostic(&format!(
            "glClear error: 0x{clear_error:04X}"
        ));

        gl::Finish();

        let finish_error = gl::GetError();
        diagnostic(&format!(
            "glFinish error: 0x{finish_error:04X}"
        ));

        // Read one pixel from the back buffer before swapping.  A successful
        // blue clear should report approximately RGBA 0, 64, 255, 255.
        let mut pixel = [0_u8; 4];
        gl::ReadBuffer(gl::BACK);
        gl::ReadPixels(
            wallpaper_window.width / 2,
            wallpaper_window.height / 2,
            1,
            1,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            pixel.as_mut_ptr() as *mut std::ffi::c_void,
        );

        let read_error = gl::GetError();
        diagnostic(&format!(
            "Back-buffer center pixel: R={} G={} B={} A={}; glReadPixels error: 0x{read_error:04X}",
            pixel[0], pixel[1], pixel[2], pixel[3],
        ));

        glx::glXSwapBuffers(
            display,
            wallpaper_window.glx_window,
        );

        xlib::XSync(display, xlib::False);

        let swap_error = gl::GetError();
        diagnostic(&format!(
            "OpenGL error after glXSwapBuffers: 0x{swap_error:04X}"
        ));
    }

    diagnostic("Presented diagnostic OpenGL frame with glXSwapBuffers().");
    Ok(())
}

fn run_window_loop() {
    diagnostic("Displaying OpenGL diagnostic frame for five seconds...");
    thread::sleep(Duration::from_secs(5));
}

impl WallpaperBackend for X11WallpaperBackend {
    fn backend_name(&self) -> &'static str {
        "x11"
    }

    fn report_capabilities(&self) {
        println!("X11 wallpaper capabilities are available:");
        println!("    Display: {}", self.connection.display_name());
        println!("    Screen: {}", self.connection.screen());
        println!(
            "    Current geometry: {}x{}",
            self.connection.width(),
            self.connection.height(),
        );
        println!("    Default depth: {}", self.connection.depth());
        println!("    Root window: {}", self.connection.root_window());
        println!();
    }

    fn run(
        self: Box<Self>,
        _shader_manager: ShaderManager,
        _wallpaper_directory: &Path,
        _shader_interval: Option<Duration>,
        _runtime: &WallpaperRuntime,
        _running: Arc<AtomicBool>,
        _control: WallpaperRuntimeControl,
    ) -> Result<(), String> {
        let display = self.connection.display();

        let glx_config =
            GlxFramebufferConfig::choose(
                display,
                self.connection.screen(),
            )?;

        diagnostic("Creating native X11 wallpaper window...");

        let wallpaper_window =
            create_wallpaper_window(
                &self.connection,
                &glx_config,
            )?;

        let glx_context =
            match GlxContext::create(
                display,
                &glx_config,
            ) {
                Ok(context) => context,

                Err(error) => {
                    destroy_wallpaper_window(
                        &self.connection,
                        wallpaper_window,
                    );

                    return Err(error);
                }
            };

        if let Err(error) =
            glx_context.make_current(
                display,
                wallpaper_window.glx_window,
            )
        {
            glx_context.destroy(display);

            destroy_wallpaper_window(
                &self.connection,
                wallpaper_window,
            );

            return Err(error);
        }

        let render_result = (|| -> Result<(), String> {
            load_opengl_functions()?;
            report_opengl_information();
            render_diagnostic_frame(display, &wallpaper_window)?;
            run_window_loop();

            Ok(())
        })();

        let release_result =
            GlxContext::release_current(display);

        glx_context.destroy(display);

        destroy_wallpaper_window(
            &self.connection,
            wallpaper_window,
        );

        render_result?;
        release_result?;

        Ok(())
    }
}

