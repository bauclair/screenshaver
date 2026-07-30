// x11_wallpaper_20260730_fbconfig_refactor_v1.rs
//
// Native X11 wallpaper backend using a GLX-selected visual.
//
// Stage 4B:
//
// ✓ Selects a modern GLX framebuffer configuration.
// ✓ Creates the X11 window with the visual required by that configuration.
// ✓ Applies EWMH desktop-window semantics.
// ✓ Creates and activates a GLX context.
// ✓ Preserves the five-second solid-red integration test.
// ✓ Releases GLX and X11 resources in dependency order.
//
// OpenGL drawing and shader rendering are intentionally deferred to the next
// stage.

use std::ffi::CString;
use std::path::Path;
use std::sync::{
    atomic::AtomicBool,
    Arc,
};
use std::thread;
use std::time::Duration;

use x11::xlib;

use crate::define_wallpaper::WallpaperRuntime;
use crate::glx_context::{
    GlxContext,
    GlxFramebufferConfig,
};
use crate::manage_shader::ShaderManager;
use crate::manage_wallpaper_runtime::WallpaperRuntimeControl;
use crate::wallpaper_backend::WallpaperBackend;
use crate::x11_connection::X11Connection;

/// Native X11 wallpaper backend.
pub struct X11WallpaperBackend {
    connection: X11Connection,
}

impl X11WallpaperBackend {
    pub fn new() -> Result<Self, String> {
        println!("Probing native X11 wallpaper capabilities...");

        let connection = X11Connection::connect()?;

        Ok(Self { connection })
    }
}

/// Resources owned by the X11 wallpaper window.
///
/// The GLX context remains separately owned by `GlxContext`.
struct X11WallpaperWindow {
    window: xlib::Window,
    colormap: xlib::Colormap,
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

        // Resolve every required atom before allocating window resources so an
        // atom failure cannot leak a newly-created window or colormap.
        println!("Interning EWMH atoms...");

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

        println!("Creating colormap for the GLX-compatible visual...");

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

        // Preserve the existing red-window test until OpenGL rendering is
        // introduced in the next stage.
        attributes.background_pixel = 0x00ff0000;
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

        println!("Applying desktop window hints...");

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
        xlib::XClearWindow(display, window);
        xlib::XFlush(display);

        println!(
            "Created native X11 wallpaper window {} ({}x{})",
            window,
            width,
            height,
        );

        Ok(X11WallpaperWindow {
            window,
            colormap,
        })
    }
}

fn destroy_wallpaper_window(
    connection: &X11Connection,
    wallpaper_window: X11WallpaperWindow,
) {
    unsafe {
        let display = connection.display();

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

        xlib::XFlush(display);
    }

    println!("Closed X11 wallpaper window.");
}

fn run_window_loop() {
    println!("Displaying X11 GLX wallpaper test window for five seconds...");

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

        println!("Creating native X11 wallpaper window...");

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
                wallpaper_window.window,
            )
        {
            glx_context.destroy(display);

            destroy_wallpaper_window(
                &self.connection,
                wallpaper_window,
            );

            return Err(error);
        }

        run_window_loop();

        let release_result =
            GlxContext::release_current(display);

        glx_context.destroy(display);

        destroy_wallpaper_window(
            &self.connection,
            wallpaper_window,
        );

        release_result?;

        Ok(())
    }
}

