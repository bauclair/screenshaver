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
use std::cell::RefCell;
use std::rc::Rc;
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
    windowed: bool,
    wm_delete: xlib::Atom,
    normal_width: i32,
    normal_height: i32,
    maximized: bool,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
struct WindowGeometry {
    #[serde(default)] x: Option<i32>,
    #[serde(default)] y: Option<i32>,
    width: i32,
    height: i32,
    #[serde(default)] maximized: bool,
}

fn load_geometry() -> WindowGeometry {
    let default = WindowGeometry { x: None, y: None, width: 960, height: 540, maximized: false };
    crate::manage_runtime_state::read_state().ok()
        .and_then(|root| root.get("windowshader").cloned())
        .and_then(|value| serde_json::from_value::<WindowGeometry>(value).ok())
        .filter(|geometry| (64..=16384).contains(&geometry.width)
            && (64..=16384).contains(&geometry.height))
        .unwrap_or(default)
}

fn save_geometry(geometry: WindowGeometry) -> Result<(), String> {
    let value = serde_json::to_value(geometry).map_err(|error| error.to_string())?;
    crate::manage_runtime_state::update_state(move |root| {
        root.as_object_mut().ok_or("Runtime state root is not an object")?
            .insert("windowshader".to_string(), value);
        Ok(())
    })
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
    windowed: bool,
) -> Result<X11WallpaperWindow, String> {
    unsafe {
        let display = connection.display();

        if display.is_null() {
            return Err(
                "Cannot create an X11 wallpaper window with a null display."
                    .to_string(),
            );
        }

        let saved = load_geometry();
        let width = if windowed { saved.width as u32 } else { connection.width() };
        let height = if windowed { saved.height as u32 } else { connection.height() };
        let x = if windowed { saved.x.unwrap_or(80) } else { 0 };
        let y = if windowed { saved.y.unwrap_or(80) } else { 0 };
        let visual_info = glx_config.visual_info();

        diagnostic("Interning EWMH atoms...");

        let wm_type = intern_atom(display, "_NET_WM_WINDOW_TYPE")?;
        let wm_type_desktop =
            intern_atom(display, "_NET_WM_WINDOW_TYPE_DESKTOP")?;
        let wm_state = intern_atom(display, "_NET_WM_STATE")?;
        let wm_state_below =
            intern_atom(display, "_NET_WM_STATE_BELOW")?;
        let wm_state_above =
            intern_atom(display, "_NET_WM_STATE_ABOVE")?;
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
            x,
            y,
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

        let mut wm_delete = 0;
        if windowed {
            diagnostic("Applying normal Windowshader window properties...");
            let normal = intern_atom(display, "_NET_WM_WINDOW_TYPE_NORMAL")?;
            set_atom_property(display, window, wm_type, &[normal]);
            let title = CString::new("Screenshaver Windowshader").unwrap();
            xlib::XStoreName(display, window, title.as_ptr());

            // When a previous Windowshader position exists, tell the window
            // manager that the restored coordinates are intentional. Without
            // WM_NORMAL_HINTS/PPosition, a managed top-level window manager
            // such as Openbox may apply its own placement policy instead of
            // honoring the x/y supplied to XCreateWindow().
            if saved.x.is_some() && saved.y.is_some() {
                let mut size_hints: xlib::XSizeHints = std::mem::zeroed();
                size_hints.flags = xlib::PPosition;
                size_hints.x = x;
                size_hints.y = y;
                xlib::XSetWMNormalHints(display, window, &mut size_hints);
            }

            wm_delete = intern_atom(display, "WM_DELETE_WINDOW")?;
            xlib::XSetWMProtocols(display, window, &mut wm_delete, 1);
            if saved.maximized {
                let max_v = intern_atom(display, "_NET_WM_STATE_MAXIMIZED_VERT")?;
                let max_h = intern_atom(display, "_NET_WM_STATE_MAXIMIZED_HORZ")?;
                set_atom_property(
                    display,
                    window,
                    wm_state,
                    &[wm_state_above, max_v, max_h],
                );
            } else {
                set_atom_property(display, window, wm_state, &[wm_state_above]);
            }
            xlib::XMapWindow(display, window);
        } else {
            diagnostic("Applying desktop window hints...");
            set_atom_property(display, window, wm_type, &[wm_type_desktop]);
            set_atom_property(display, window, wm_state,
                &[wm_state_below, wm_state_skip_taskbar, wm_state_skip_pager]);
            xlib::XMapRaised(display, window);
        }
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
            windowed,
            wm_delete,
            normal_width: saved.width,
            normal_height: saved.height,
            maximized: saved.maximized,
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
                        metadata.policy_id,
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

fn drain_x11_events(display: *mut xlib::Display, window: &mut X11WallpaperWindow,
    running: &AtomicBool) {
    unsafe {
        while xlib::XPending(display) > 0 {
            let mut event: xlib::XEvent = std::mem::zeroed();
            xlib::XNextEvent(display, &mut event);
            if !window.windowed { continue; }
            match event.get_type() {
                xlib::ConfigureNotify => {
                    let e = event.configure;
                    if e.window == window.window {
                        window.width = e.width.max(1);
                        window.height = e.height.max(1);
                        if !window.maximized {
                            window.normal_width = window.width;
                            window.normal_height = window.height;
                        }
                    }
                }
                xlib::ClientMessage => {
                    let e = event.client_message;
                    if e.window == window.window && e.data.get_long(0) as xlib::Atom == window.wm_delete {
                        running.store(false, Ordering::SeqCst);
                    }
                }
                xlib::DestroyNotify => {
                    if event.destroy_window.window == window.window {
                        window.window = 0;
                        running.store(false, Ordering::SeqCst);
                    }
                }
                _ => {}
            }
        }
    }
}

// The GL callback never performs OpenRGB network I/O; it submits to the
// existing bounded worker. All ownership changes occur in the render loop.
struct WallpaperAmbient {
    sampler: crate::manage_ambient_lighting::FramebufferSampler,
    session: Option<crate::manage_openrgb_session::ShaderLightingSession>,
    disabled: bool,
}

impl WallpaperAmbient {
    fn new() -> Result<Self, String> {
        Ok(Self {
            sampler: crate::manage_ambient_lighting::FramebufferSampler::new(
                22, 12, Duration::from_millis(50))?,
            session: None,
            disabled: false,
        })
    }

    fn acquire(&mut self) {
        if self.disabled || self.session.is_some() { return; }
        let Some(controller_id) = std::env::var("SCREENSHAVER_AMBIENT_LIVE_CONTROLLER")
            .ok().and_then(|value| value.parse::<u32>().ok()) else { return; };
        let endpoint = std::env::var("SCREENSHAVER_AMBIENT_LIVE_ENDPOINT")
            .unwrap_or_else(|_| "127.0.0.1:6742".to_string());
        let address = match endpoint.parse::<std::net::SocketAddr>() {
            Ok(address) => address,
            Err(error) => { diagnostic(&format!("[AMBIENT_OPENRGB] Invalid endpoint: {error}")); self.disabled = true; return; }
        };
        match crate::manage_openrgb_session::ShaderLightingSession::start(address, controller_id) {
            Ok(session) => { self.session = Some(session); diagnostic("[AMBIENT_OPENRGB] X11 wallpaper acquired keyboard"); }
            Err(error) => { diagnostic(&format!("[AMBIENT_OPENRGB] X11 wallpaper acquisition refused: {error}")); self.disabled = true; }
        }
    }

    fn release(&mut self) {
        if let Some(session) = self.session.take() {
            match session.stop() {
                Ok(_) => diagnostic("[AMBIENT_OPENRGB] X11 wallpaper restored keyboard"),
                Err(error) => { diagnostic(&format!("[AMBIENT_OPENRGB] X11 wallpaper release unverified: {error}")); self.disabled = true; }
            }
        }
    }

    fn observe(&mut self, framebuffer: u32, width: u32, height: u32) {
        if self.disabled || framebuffer != 0 || width == 0 || height == 0 { return; }
        let source = crate::manage_ambient_lighting::FrameSource { framebuffer, width, height };
        match self.sampler.sample(source) {
            Ok(Some(frame)) => {
                if let Some(session) = self.session.as_mut() {
                    if let Err(error) = session.submit(&frame) {
                        diagnostic(&format!("[AMBIENT_OPENRGB] X11 wallpaper update failed: {error}"));
                        self.disabled = true;
                        self.release();
                    }
                }
            }
            Ok(None) => {}
            Err(error) => { diagnostic(&format!("[AMBIENT_OPENRGB] X11 wallpaper sampling disabled: {error}")); self.disabled = true; self.release(); }
        }
    }
}

fn run_window_loop(
    display: *mut xlib::Display,
    wallpaper_window: &mut X11WallpaperWindow,
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
    let ambient = if std::env::var_os("SCREENSHAVER_AMBIENT_LIVE_CONTROLLER").is_some() {
        match WallpaperAmbient::new() {
            Ok(ambient) => Some(Rc::new(RefCell::new(ambient))),
            Err(error) => { diagnostic(&format!("[AMBIENT_OPENRGB] X11 wallpaper sampler unavailable: {error}")); None }
        }
    } else { None };
    if let Some(ref ambient) = ambient {
        let observer = Rc::clone(ambient);
        engine.set_ambient_frame_hook(Some(Box::new(move |fb, w, h| {
            observer.borrow_mut().observe(fb, w, h);
        })));
    }

    while running.load(Ordering::SeqCst) {
        drain_x11_events(display, wallpaper_window, running);
        if !running.load(Ordering::SeqCst) { break; }

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
                if let Some(ref ambient) = ambient { ambient.borrow_mut().release(); }
                control.acknowledge_paused();
                diagnostic("X11 wallpaper rendering paused.");
            }

            thread::sleep(
                Duration::from_millis(10)
            );

            continue;
        }

        if paused {
            engine.reset_after_pause();
        }
        if let Some(ref ambient) = ambient { ambient.borrow_mut().acquire(); }

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

    engine.set_ambient_frame_hook(None);
    if let Some(ref ambient) = ambient {
        let mut ambient = ambient.borrow_mut();
        ambient.release();
        unsafe { ambient.sampler.destroy(); }
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

        let mut wallpaper_window =
            create_wallpaper_window(
                &self.connection,
                &glx_config,
                runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed,
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
                    runtime.lyrics_state.clone(),
                    wallpaper_window.width as u32,
                    wallpaper_window.height as u32,
                )?;

            let initial_metadata =
                engine.current_metadata();

            if let Some(shader_path) = initial_metadata.shader_path.clone() {
                runtime.tray_status.set_active(
                    initial_metadata.policy_id,
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
                &mut wallpaper_window,
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

        if wallpaper_window.windowed {
            unsafe {
                if wallpaper_window.window != 0 {
                    let mut child = 0;
                    let mut client_x = 0;
                    let mut client_y = 0;

                    if xlib::XTranslateCoordinates(
                        display,
                        wallpaper_window.window,
                        self.connection.root_window(),
                        0,
                        0,
                        &mut client_x,
                        &mut client_y,
                        &mut child,
                    ) != 0
                    {
                        // EWMH _NET_FRAME_EXTENTS contains:
                        // left, right, top, bottom. XTranslateCoordinates()
                        // gives the root-relative client origin, so subtract
                        // the left/top decoration extents before persisting.
                        // This stores the outer frame position and prevents
                        // decorations from accumulating across restarts.
                        let frame_extents =
                            intern_atom(display, "_NET_FRAME_EXTENTS").ok();

                        let mut left: i32 = 0;
                        let mut top: i32 = 0;

                        if let Some(frame_extents) = frame_extents {
                            let mut actual_type: xlib::Atom = 0;
                            let mut actual_format: i32 = 0;
                            let mut item_count: libc::c_ulong = 0;
                            let mut bytes_after: libc::c_ulong = 0;
                            let mut property: *mut u8 = std::ptr::null_mut();

                            let status = xlib::XGetWindowProperty(
                                display,
                                wallpaper_window.window,
                                frame_extents,
                                0,
                                4,
                                xlib::False,
                                xlib::XA_CARDINAL,
                                &mut actual_type,
                                &mut actual_format,
                                &mut item_count,
                                &mut bytes_after,
                                &mut property,
                            );

                            if status == xlib::Success as i32
                                && !property.is_null()
                                && actual_format == 32
                                && item_count >= 4
                            {
                                let values =
                                    std::slice::from_raw_parts(
                                        property as *const libc::c_ulong,
                                        item_count as usize,
                                    );

                                left = values[0] as i32;
                                top = values[2] as i32;
                            }

                            if !property.is_null() {
                                xlib::XFree(property as *mut libc::c_void);
                            }
                        }

                        if let Err(error) = save_geometry(WindowGeometry {
                            x: Some(client_x - left),
                            y: Some(client_y - top),
                            width: wallpaper_window.normal_width,
                            height: wallpaper_window.normal_height,
                            maximized: wallpaper_window.maximized,
                        }) {
                            eprintln!(
                                "[WINDOWSHADER] Unable to save X11 window geometry: {}",
                                error
                            );
                        }
                    } else {
                        eprintln!(
                            "[WINDOWSHADER] Unable to query X11 Windowshader position for persistence."
                        );
                    }
                }
            }
        }
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

