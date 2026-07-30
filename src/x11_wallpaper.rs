use std::path::Path;
use std::sync::{
    atomic::AtomicBool,
    Arc,
};
use std::thread;
use std::time::Duration;

use x11::xlib;

use crate::define_wallpaper::WallpaperRuntime;
use crate::manage_shader::ShaderManager;
use crate::manage_wallpaper_runtime::WallpaperRuntimeControl;
use crate::wallpaper_backend::WallpaperBackend;
use crate::x11_connection::X11Connection;

/// Native X11 wallpaper backend.
///
/// Stage 3A:
/// * Creates a native X11 window
/// * Displays a solid red background
/// * Keeps the window visible for five seconds
/// * Destroys the window cleanly
///
/// GLX/OpenGL integration will be added in a later stage.
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

fn create_wallpaper_window(
    connection: &X11Connection,
) -> Result<xlib::Window, String> {
    unsafe {
        let display = connection.display();

        let width = connection.width() as u32;
        let height = connection.height() as u32;

        let window = xlib::XCreateSimpleWindow(
            display,
            connection.root_window(),
            0,
            0,
            width,
            height,
            0,
            0,
            0,
        );

        if window == 0 {
            return Err("Unable to create native X11 wallpaper window.".to_string());
        }

        xlib::XSetWindowBackground(display, window, 0x00ff0000);
        xlib::XSelectInput(
            display,
            window,
            xlib::ExposureMask | xlib::StructureNotifyMask | xlib::KeyPressMask,
        );

        xlib::XMapRaised(display, window);
        xlib::XClearWindow(display, window);
        xlib::XFlush(display);

        println!(
            "Created native X11 wallpaper window {} ({}x{})",
            window, width, height
        );

        Ok(window)
    }
}

fn run_window_loop(
    connection: &X11Connection,
    window: xlib::Window,
) {
    println!("Displaying X11 wallpaper test window for five seconds...");

    thread::sleep(Duration::from_secs(5));

    unsafe {
        xlib::XDestroyWindow(connection.display(), window);
        xlib::XFlush(connection.display());
    }

    println!("Closed X11 wallpaper window.");
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
        println!("Creating native X11 wallpaper window...");

        let window = create_wallpaper_window(&self.connection)?;

        run_window_loop(&self.connection, window);

        Ok(())
    }
}

