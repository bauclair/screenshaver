// x11_wallpaper.rs
//
// Native X11 wallpaper backend using a GLX-selected visual.
//
// Stage 4E:
//
// ✓ Selects a modern GLX framebuffer configuration.
// ✓ Creates the X11 window with the visual required by that configuration.
// ✓ Creates an explicit GLXWindow drawable for the X11 window.
// ✓ Applies EWMH desktop-window semantics.
// ✓ Creates and activates a GLX context.
// ✓ Loads OpenGL functions through GLX.
// ✓ Logs the OpenGL vendor, renderer, version, and GLSL version.
// ✓ Renders continuously through the shared FrameRenderEngine.
// ✓ Presents each frame with glXSwapBuffers().
// ✓ Honors wallpaper pause/resume and shutdown control.
// ✓ Releases GLX and X11 resources in dependency order.

use std::ffi::{
    CStr,
    CString,
};
use std::path::Path;
use std::sync::{
    atomic::{
        AtomicBool,
        Ordering,
    },
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
use crate::render_frame::{
    FrameRenderEngine,
    FrameRenderEvent,
    FrameRenderMetadata,
    FrameRenderEvents,
};
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

fn render_shared_engine_frame(
    display: *mut xlib::Display,
    wallpaper_window: &X11WallpaperWindow,
    engine: &mut FrameRenderEngine,
) -> FrameRenderEvents {
    let status =
        engine.render_frame(
            wallpaper_window.width as u32,
            wallpaper_window.height as u32,
        );

    unsafe {
        glx::glXSwapBuffers(
            display,
            wallpaper_window.glx_window,
        );
    }

    engine.limit_fps();

    status
}


fn wallpaper_metadata(
    metadata: &FrameRenderMetadata,
) -> crate::notify_wallpaper::WallpaperMetadata {

    crate::notify_wallpaper::WallpaperMetadata {
        policy_name:
            metadata.policy_name.clone(),
        animation_speed:
            metadata.animation_speed,
        texture:
            metadata.texture.clone(),
        palette:
            metadata.palette.clone(),
        fps:
            metadata.configured_fps.max(1),
        warning_state:
            metadata.warning_state,
    }
}


fn notify_wallpaper_events(
    enabled: bool,
    frame_events: FrameRenderEvents,
    tray_status: &crate::tray_icon::TrayStatusControl,
    notification_state:
        &mut crate::notify_wallpaper::WallpaperNotificationState,
) {
    for event in frame_events.events {
        match event {
            FrameRenderEvent::ShaderChanged(metadata) => {
                if let Some(shader_path) = metadata.shader_path.clone() {
                    tray_status.set_active(
                        metadata.shader_name.clone(),
                        shader_path,
                    );
                }

                let wallpaper_metadata =
                    wallpaper_metadata(
                        &metadata
                    );

                notification_state
                    .show_shader_changed(
                        enabled,
                        &wallpaper_metadata,
                    );
            }

            FrameRenderEvent::PerformanceChanged(metadata) => {
                if metadata.warning_state
                    != crate::fps_monitor::FpsWarningState::Normal
                {
                    let wallpaper_metadata =
                        wallpaper_metadata(
                            &metadata
                        );

                    notification_state
                        .show_update(
                            enabled,
                            &wallpaper_metadata,
                        );
                }
            }
        }
    }
}

fn drain_x11_events(
    display: *mut xlib::Display,
) {
    unsafe {
        while xlib::XPending(display) > 0 {
            let mut event: xlib::XEvent =
                std::mem::zeroed();

            xlib::XNextEvent(
                display,
                &mut event,
            );
        }
    }
}

fn run_window_loop(
    display: *mut xlib::Display,
    wallpaper_window: &X11WallpaperWindow,
    engine: &mut FrameRenderEngine,
    running: &AtomicBool,
    control: &WallpaperRuntimeControl,
    notifications_enabled: bool,
    tray_status: &crate::tray_icon::TrayStatusControl,
    notification_state:
        &mut crate::notify_wallpaper::WallpaperNotificationState,
) {
    diagnostic("Entering continuous X11 wallpaper render loop...");

    let mut paused = false;
    let mut first_frame_presented = false;

    while running.load(Ordering::SeqCst) {
        drain_x11_events(display);

        if let Some(reload) =
            control.take_policy_reload()
        {
            match engine.reconfigure_active_wallpaper(
                reload,
                wallpaper_window.width as u32,
                wallpaper_window.height as u32,
            ) {
                Ok(()) => {
                    diagnostic(
                        "X11 active wallpaper policy reloaded."
                    );
                }
                Err(error) => {
                    diagnostic(
                        &format!(
                            "Unable to reload X11 active wallpaper policy; keeping the previous settings: {}",
                            error,
                        )
                    );
                }
            }
        }

        if control.pause_requested() {
            if !paused {
                paused = true;
                control.acknowledge_paused();
                diagnostic("X11 wallpaper rendering paused.");
            }

            thread::sleep(
                Duration::from_millis(10)
            );

            continue;
        }

        let status =
            render_shared_engine_frame(
                display,
                wallpaper_window,
                engine,
            );

        notify_wallpaper_events(
            notifications_enabled,
            status,
            tray_status,
            notification_state,
        );

        if paused {
            paused = false;
            control.acknowledge_resumed_frame();
            diagnostic("X11 wallpaper rendering resumed.");
        }

        if !first_frame_presented {
            first_frame_presented = true;
            diagnostic("Presented first continuous X11 wallpaper frame.");
        }
    }

    diagnostic("Leaving continuous X11 wallpaper render loop...");
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
        shader_manager: ShaderManager,
        wallpaper_directory: &Path,
        shader_interval: Option<Duration>,
        runtime: &WallpaperRuntime,
        running: Arc<AtomicBool>,
        control: WallpaperRuntimeControl,
    ) -> Result<(), String> {
        runtime.tray_status
            .set_starting();


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

            let parsed_subtitle_placement =
                crate::parse_subtitle_placement::parse(
                    None
                );

            let mut engine =
                FrameRenderEngine::new_for_wallpaper(
                    shader_manager,
                    wallpaper_directory,
                    shader_interval
                        .map(
                            |interval| {
                                interval.as_secs()
                            }
                        )
                        .unwrap_or(
                            0
                        ),
                    runtime.animation_speed_policy.clone(),
                    runtime.fps_policy.clone(),
                    runtime.texture_policy.clone(),
                    runtime.postprocess_policy.clone(),
                    runtime.audio_bands.clone(),
                    false,
                    parsed_subtitle_placement.placement,
                    wallpaper_window.width as u32,
                    wallpaper_window.height as u32,
                )?;

            let initial_metadata =
                engine.current_metadata();

            if let Some(shader_path) = initial_metadata.shader_path.clone() {
                runtime.tray_status.set_active(
                    initial_metadata.shader_name.clone(),
                    shader_path,
                );
            }

            let mut notification_state =
                crate::notify_wallpaper::WallpaperNotificationState::new();


            let initial_wallpaper_metadata =
                wallpaper_metadata(
                    &initial_metadata
                );


            notification_state
                .show_shader_changed(
                    runtime.notifications,
                    &initial_wallpaper_metadata,
                );


            run_window_loop(
                display,
                &wallpaper_window,
                &mut engine,
                running.as_ref(),
                &control,
                runtime.notifications,
                &runtime.tray_status,
                &mut notification_state,
            );

            // `engine` is dropped here while the GLX context is still current.
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

