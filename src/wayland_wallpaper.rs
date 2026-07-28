use wayland_client::protocol::{
    wl_compositor,
    wl_output,
    wl_region,
    wl_registry,
    wl_surface,
};
use std::ffi::{
    c_char,
    c_void,
    CString,
};
use std::ptr;
use std::thread;
use std::time::{
    Duration,
    Instant,
};

use wayland_client::{
    delegate_noop,
    Connection,
    Dispatch,
    Proxy,
    QueueHandle,
};

use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{
        self,
        Layer,
        ZwlrLayerShellV1,
    },
    zwlr_layer_surface_v1::{
        self,
        Anchor,
        KeyboardInteractivity,
        ZwlrLayerSurfaceV1,
    },
};


#[derive(Debug, Default)]
pub struct WallpaperWaylandCapabilities {
    pub compositor_version: Option<u32>,
    pub layer_shell_version: Option<u32>,
    pub output_count: usize,
}


#[derive(Debug)]
pub struct WallpaperSurfaceConfiguration {
    pub width: u32,
    pub height: u32,
    pub serial: u32,
}


#[derive(Debug, Default)]
struct WaylandState {
    compositor: Option<wl_compositor::WlCompositor>,
    layer_shell: Option<ZwlrLayerShellV1>,
    output: Option<wl_output::WlOutput>,

    compositor_version: Option<u32>,
    layer_shell_version: Option<u32>,
    output_count: usize,

    configured: Option<WallpaperSurfaceConfiguration>,
    closed: bool,
}


impl Dispatch<wl_registry::WlRegistry, ()>
    for WaylandState
{
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _connection: &Connection,
        queue_handle: &QueueHandle<Self>,
    ) {

        let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event else {
            return;
        };


        match interface.as_str() {

            "wl_compositor" => {

                state.compositor_version =
                    Some(
                        version
                    );


                if state
                    .compositor
                    .is_none()
                {
                    state.compositor =
                        Some(
                            registry.bind::<
                                wl_compositor::WlCompositor,
                                _,
                                _
                            >(
                                name,
                                version.min(
                                    6
                                ),
                                queue_handle,
                                (),
                            )
                        );
                }
            }


            "zwlr_layer_shell_v1" => {

                state.layer_shell_version =
                    Some(
                        version
                    );


                if state
                    .layer_shell
                    .is_none()
                {
                    state.layer_shell =
                        Some(
                            registry.bind::<
                                ZwlrLayerShellV1,
                                _,
                                _
                            >(
                                name,
                                version.min(
                                    4
                                ),
                                queue_handle,
                                (),
                            )
                        );
                }
            }


            "wl_output" => {

                state.output_count +=
                    1;


                if state
                    .output
                    .is_none()
                {
                    state.output =
                        Some(
                            registry.bind::<
                                wl_output::WlOutput,
                                _,
                                _
                            >(
                                name,
                                version.min(
                                    4
                                ),
                                queue_handle,
                                (),
                            )
                        );
                }
            }


            _ => {}
        }
    }
}


impl Dispatch<ZwlrLayerSurfaceV1, ()>
    for WaylandState
{
    fn event(
        state: &mut Self,
        layer_surface: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {

        match event {

            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {

                layer_surface.ack_configure(
                    serial
                );


                state.configured =
                    Some(
                        WallpaperSurfaceConfiguration {
                            width,
                            height,
                            serial,
                        }
                    );
            }


            zwlr_layer_surface_v1::Event::Closed => {

                state.closed =
                    true;
            }


            _ => {}
        }
    }
}


delegate_noop!(
    WaylandState:
    ignore wl_compositor::WlCompositor
);


delegate_noop!(
    WaylandState:
    ignore wl_output::WlOutput
);


delegate_noop!(
    WaylandState:
    ignore wl_region::WlRegion
);


delegate_noop!(
    WaylandState:
    ignore wl_surface::WlSurface
);


delegate_noop!(
    WaylandState:
    ignore ZwlrLayerShellV1
);


pub fn probe_capabilities(
) -> Result<WallpaperWaylandCapabilities, String> {

    let (
        _connection,
        _event_queue,
        state,
    ) =
        connect_and_bind()?;


    Ok(
        WallpaperWaylandCapabilities {
            compositor_version:
                state.compositor_version,

            layer_shell_version:
                state.layer_shell_version,

            output_count:
                state.output_count,
        }
    )
}


pub fn test_egl_background_surface(
    fragment_source: &str,
) -> Result<WallpaperSurfaceConfiguration, String> {

    let (
        connection,
        mut event_queue,
        mut state,
    ) =
        connect_and_bind()?;


    let queue_handle =
        event_queue.handle();


    let compositor =
        state
            .compositor
            .clone()
            .ok_or_else(
                || {
                    "The Wayland compositor did not advertise wl_compositor"
                        .to_string()
                }
            )?;


    let layer_shell =
        state
            .layer_shell
            .clone()
            .ok_or_else(
                || {
                    "The Wayland compositor does not advertise zwlr_layer_shell_v1"
                        .to_string()
                }
            )?;


    let output =
        state
            .output
            .clone()
            .ok_or_else(
                || {
                    "The Wayland compositor did not advertise any wl_output objects"
                        .to_string()
                }
            )?;


    let surface =
        compositor.create_surface(
            &queue_handle,
            (),
        );


    let layer_surface =
        layer_shell.get_layer_surface(
            &surface,
            Some(
                &output
            ),
            Layer::Background,
            "screenshaver-wallpaper".to_string(),
            &queue_handle,
            (),
        );


    layer_surface.set_size(
        0,
        0,
    );


    layer_surface.set_anchor(
        Anchor::Top
            | Anchor::Bottom
            | Anchor::Left
            | Anchor::Right
    );


    layer_surface.set_exclusive_zone(
        -1
    );


    layer_surface.set_keyboard_interactivity(
        KeyboardInteractivity::None
    );


    let empty_input_region =
        compositor.create_region(
            &queue_handle,
            (),
        );


    surface.set_input_region(
        Some(
            &empty_input_region
        )
    );


    surface.commit();


    connection
        .flush()
        .map_err(
            |error| {
                format!(
                    "Unable to send the wallpaper surface request to the Wayland compositor: {}",
                    error,
                )
            }
        )?;


    while state
        .configured
        .is_none()
        && !state.closed
    {
        event_queue
            .blocking_dispatch(
                &mut state
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to receive the wallpaper surface configure event: {}",
                        error,
                    )
                }
            )?;
    }


    let configuration =
        state
            .configured
            .take()
            .ok_or_else(
                || {
                    "Mango closed the wallpaper layer surface before configuring it"
                        .to_string()
                }
            )?;


    connection
        .flush()
        .map_err(
            |error| {
                format!(
                    "Unable to send the wallpaper configure acknowledgement: {}",
                    error,
                )
            }
        )?;


    let buffer_width =
        i32::try_from(
            configuration.width
        )
        .map_err(
            |_| {
                "Wallpaper surface width exceeds the EGL window range"
                    .to_string()
            }
        )?;


    let buffer_height =
        i32::try_from(
            configuration.height
        )
        .map_err(
            |_| {
                "Wallpaper surface height exceeds the EGL window range"
                    .to_string()
            }
        )?;


    let egl_window =
        wayland_egl::WlEglSurface::new(
            surface.id(),
            buffer_width,
            buffer_height,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to create wl_egl_window: {}",
                    error,
                )
            }
        )?;


    render_egl_shader_test(
        &connection,
        &egl_window,
        buffer_width,
        buffer_height,
        fragment_source,
    )?;


    drop(
        egl_window
    );


    layer_surface.destroy();


    surface.destroy();


    empty_input_region.destroy();


    layer_shell.destroy();


    connection
        .flush()
        .map_err(
            |error| {
                format!(
                    "Unable to send wallpaper surface cleanup requests: {}",
                    error,
                )
            }
        )?;


    Ok(
        configuration
    )
}


fn connect_and_bind(
) -> Result<
    (
        Connection,
        wayland_client::EventQueue<WaylandState>,
        WaylandState,
    ),
    String,
> {

    let connection =
        Connection::connect_to_env()
            .map_err(
                |error| {
                    format!(
                        "Unable to connect to the Wayland compositor: {}",
                        error,
                    )
                }
            )?;


    let display =
        connection.display();


    let mut event_queue =
        connection.new_event_queue();


    let queue_handle =
        event_queue.handle();


    let _registry =
        display.get_registry(
            &queue_handle,
            (),
        );


    let mut state =
        WaylandState::default();


    event_queue
        .roundtrip(
            &mut state
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read Wayland compositor capabilities: {}",
                    error,
                )
            }
        )?;


    if state
        .compositor
        .is_none()
    {
        return Err(
            "The Wayland compositor did not advertise wl_compositor"
                .to_string()
        );
    }


    if state
        .layer_shell
        .is_none()
    {
        return Err(
            "The Wayland compositor does not advertise zwlr_layer_shell_v1"
                .to_string()
        );
    }


    if state
        .output
        .is_none()
    {
        return Err(
            "The Wayland compositor did not advertise any wl_output objects"
                .to_string()
        );
    }


    Ok(
        (
            connection,
            event_queue,
            state,
        )
    )
}

type EglBoolean = u32;
type EglEnum = u32;
type EglInt = i32;
type EglDisplay = *mut c_void;
type EglConfig = *mut c_void;
type EglContext = *mut c_void;
type EglSurface = *mut c_void;
type EglNativeDisplay = *mut c_void;
type EglNativeWindow = *mut c_void;

const EGL_FALSE: EglBoolean = 0;

const EGL_NONE: EglInt = 0x3038;
const EGL_RED_SIZE: EglInt = 0x3024;
const EGL_GREEN_SIZE: EglInt = 0x3023;
const EGL_BLUE_SIZE: EglInt = 0x3022;
const EGL_ALPHA_SIZE: EglInt = 0x3021;
const EGL_RENDERABLE_TYPE: EglInt = 0x3040;
const EGL_SURFACE_TYPE: EglInt = 0x3033;
const EGL_WINDOW_BIT: EglInt = 0x0004;
const EGL_OPENGL_BIT: EglInt = 0x0008;

const EGL_CONTEXT_MAJOR_VERSION: EglInt = 0x3098;
const EGL_CONTEXT_MINOR_VERSION: EglInt = 0x30FB;
const EGL_CONTEXT_OPENGL_PROFILE_MASK: EglInt = 0x30FD;
const EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT: EglInt = 0x0001;

const EGL_OPENGL_API: EglEnum = 0x30A2;

const EGL_NO_DISPLAY: EglDisplay = ptr::null_mut();
const EGL_NO_CONTEXT: EglContext = ptr::null_mut();
const EGL_NO_SURFACE: EglSurface = ptr::null_mut();


#[link(
    name = "EGL"
)]
unsafe extern "C" {

    fn eglGetDisplay(
        native_display: EglNativeDisplay,
    ) -> EglDisplay;


    fn eglInitialize(
        display: EglDisplay,
        major: *mut EglInt,
        minor: *mut EglInt,
    ) -> EglBoolean;


    fn eglBindAPI(
        api: EglEnum,
    ) -> EglBoolean;


    fn eglChooseConfig(
        display: EglDisplay,
        attributes: *const EglInt,
        configs: *mut EglConfig,
        config_size: EglInt,
        config_count: *mut EglInt,
    ) -> EglBoolean;


    fn eglCreateContext(
        display: EglDisplay,
        config: EglConfig,
        shared_context: EglContext,
        attributes: *const EglInt,
    ) -> EglContext;


    fn eglCreateWindowSurface(
        display: EglDisplay,
        config: EglConfig,
        native_window: EglNativeWindow,
        attributes: *const EglInt,
    ) -> EglSurface;


    fn eglMakeCurrent(
        display: EglDisplay,
        draw_surface: EglSurface,
        read_surface: EglSurface,
        context: EglContext,
    ) -> EglBoolean;


    fn eglSwapBuffers(
        display: EglDisplay,
        surface: EglSurface,
    ) -> EglBoolean;


    fn eglDestroySurface(
        display: EglDisplay,
        surface: EglSurface,
    ) -> EglBoolean;


    fn eglDestroyContext(
        display: EglDisplay,
        context: EglContext,
    ) -> EglBoolean;


    fn eglTerminate(
        display: EglDisplay,
    ) -> EglBoolean;


    fn eglGetError(
    ) -> EglInt;


    fn eglGetProcAddress(
        name: *const c_char,
    ) -> *const c_void;
}


fn render_egl_shader_test(
    connection: &Connection,
    egl_window: &wayland_egl::WlEglSurface,
    width: i32,
    height: i32,
    fragment_source: &str,
) -> Result<(), String> {

    let native_display =
        connection
            .backend()
            .display_id()
            .as_ptr()
            as EglNativeDisplay;


    let display =
        unsafe {
            eglGetDisplay(
                native_display
            )
        };


    if display
        == EGL_NO_DISPLAY
    {
        return Err(
            egl_failure(
                "eglGetDisplay"
            )
        );
    }


    let mut egl_major = 0;
    let mut egl_minor = 0;


    if unsafe {
        eglInitialize(
            display,
            &mut egl_major,
            &mut egl_minor,
        )
    } == EGL_FALSE
    {
        return Err(
            egl_failure(
                "eglInitialize"
            )
        );
    }


    if unsafe {
        eglBindAPI(
            EGL_OPENGL_API
        )
    } == EGL_FALSE
    {
        unsafe {
            eglTerminate(
                display
            );
        }

        return Err(
            egl_failure(
                "eglBindAPI(EGL_OPENGL_API)"
            )
        );
    }


    let config_attributes = [
        EGL_SURFACE_TYPE,
        EGL_WINDOW_BIT,
        EGL_RENDERABLE_TYPE,
        EGL_OPENGL_BIT,
        EGL_RED_SIZE,
        8,
        EGL_GREEN_SIZE,
        8,
        EGL_BLUE_SIZE,
        8,
        EGL_ALPHA_SIZE,
        8,
        EGL_NONE,
    ];


    let mut config =
        ptr::null_mut();


    let mut config_count = 0;


    if unsafe {
        eglChooseConfig(
            display,
            config_attributes.as_ptr(),
            &mut config,
            1,
            &mut config_count,
        )
    } == EGL_FALSE
        || config_count
            == 0
    {
        unsafe {
            eglTerminate(
                display
            );
        }

        return Err(
            egl_failure(
                "eglChooseConfig"
            )
        );
    }


    let context_attributes = [
        EGL_CONTEXT_MAJOR_VERSION,
        3,
        EGL_CONTEXT_MINOR_VERSION,
        3,
        EGL_CONTEXT_OPENGL_PROFILE_MASK,
        EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT,
        EGL_NONE,
    ];


    let context =
        unsafe {
            eglCreateContext(
                display,
                config,
                EGL_NO_CONTEXT,
                context_attributes.as_ptr(),
            )
        };


    if context
        == EGL_NO_CONTEXT
    {
        unsafe {
            eglTerminate(
                display
            );
        }

        return Err(
            egl_failure(
                "eglCreateContext"
            )
        );
    }


    let egl_surface =
        unsafe {
            eglCreateWindowSurface(
                display,
                config,
                egl_window.ptr()
                    as EglNativeWindow,
                ptr::null(),
            )
        };


    if egl_surface
        == EGL_NO_SURFACE
    {
        unsafe {
            eglDestroyContext(
                display,
                context,
            );

            eglTerminate(
                display
            );
        }

        return Err(
            egl_failure(
                "eglCreateWindowSurface"
            )
        );
    }


    if unsafe {
        eglMakeCurrent(
            display,
            egl_surface,
            egl_surface,
            context,
        )
    } == EGL_FALSE
    {
        unsafe {
            eglDestroySurface(
                display,
                egl_surface,
            );

            eglDestroyContext(
                display,
                context,
            );

            eglTerminate(
                display
            );
        }

        return Err(
            egl_failure(
                "eglMakeCurrent"
            )
        );
    }


    gl::load_with(
        |name| {
            let Ok(
                symbol
            ) =
                CString::new(
                    name
                )
            else {
                return ptr::null();
            };


            unsafe {
                eglGetProcAddress(
                    symbol.as_ptr()
                )
            }
        }
    );


    let render_result =
        render_native_shader_frames(
            display,
            egl_surface,
            width,
            height,
            fragment_source,
        );


    unsafe {
        eglMakeCurrent(
            display,
            EGL_NO_SURFACE,
            EGL_NO_SURFACE,
            EGL_NO_CONTEXT,
        );

        eglDestroySurface(
            display,
            egl_surface,
        );

        eglDestroyContext(
            display,
            context,
        );

        eglTerminate(
            display
        );
    }


    render_result?;


    println!(
        "Native EGL shader test completed:"
    );


    println!(
        "    EGL version: {}.{}",
        egl_major,
        egl_minor
    );


    println!(
        "    OpenGL context: 3.3 core"
    );


    println!(
        "    Buffer size: {}x{}",
        width,
        height
    );


    println!(
        "    Render duration: 5 seconds"
    );


    Ok(
        ()
    )
}


fn render_native_shader_frames(
    display: EglDisplay,
    egl_surface: EglSurface,
    width: i32,
    height: i32,
    fragment_source: &str,
) -> Result<(), String> {

    let program =
        crate::compile_shader::build_program(
            crate::define_constants::VERTEX_SHADER,
            fragment_source,
        )?;


    let mut vao =
        0_u32;


    unsafe {
        gl::GenVertexArrays(
            1,
            &mut vao,
        );

        gl::BindVertexArray(
            vao
        );

        gl::UseProgram(
            program
        );
    }


    let i_time =
        uniform_location(
            program,
            "iTime",
        )?;


    let i_resolution =
        uniform_location(
            program,
            "iResolution",
        )?;


    let i_mouse =
        uniform_location(
            program,
            "iMouse",
        )?;


    let i_frame =
        uniform_location(
            program,
            "iFrame",
        )?;


    let start_time =
        Instant::now();


    let test_duration =
        Duration::from_secs(
            5
        );


    let mut frame =
        0_i32;


    let result =
        loop {

            let elapsed =
                start_time.elapsed();


            if elapsed
                >= test_duration
            {
                break Ok(());
            }


            unsafe {
                gl::Viewport(
                    0,
                    0,
                    width,
                    height,
                );

                gl::ClearColor(
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                );

                gl::Clear(
                    gl::COLOR_BUFFER_BIT
                );

                gl::UseProgram(
                    program
                );

                set_uniform_1f(
                    i_time,
                    elapsed.as_secs_f32(),
                );

                set_uniform_3f(
                    i_resolution,
                    width as f32,
                    height as f32,
                    1.0,
                );

                set_uniform_4f(
                    i_mouse,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                );

                set_uniform_1i(
                    i_frame,
                    frame,
                );

                gl::BindVertexArray(
                    vao
                );

                gl::DrawArrays(
                    gl::TRIANGLES,
                    0,
                    3,
                );
            }


            if unsafe {
                eglSwapBuffers(
                    display,
                    egl_surface,
                )
            } == EGL_FALSE
            {
                break Err(
                    egl_failure(
                        "eglSwapBuffers"
                    )
                );
            }


            frame =
                frame.saturating_add(
                    1
                );


            thread::sleep(
                Duration::from_millis(
                    1
                )
            );
        };


    unsafe {
        gl::UseProgram(
            0
        );

        gl::BindVertexArray(
            0
        );

        gl::DeleteVertexArrays(
            1,
            &vao,
        );

        gl::DeleteProgram(
            program
        );
    }


    result
}


fn uniform_location(
    program: u32,
    name: &str,
) -> Result<i32, String> {

    let c_name =
        CString::new(
            name
        )
        .map_err(
            |_| {
                format!(
                    "Uniform name contains an interior null byte: {}",
                    name,
                )
            }
        )?;


    Ok(
        unsafe {
            gl::GetUniformLocation(
                program,
                c_name.as_ptr(),
            )
        }
    )
}


fn set_uniform_1f(
    location: i32,
    value: f32,
) {

    if location >= 0 {
        unsafe {
            gl::Uniform1f(
                location,
                value,
            );
        }
    }
}


fn set_uniform_1i(
    location: i32,
    value: i32,
) {

    if location >= 0 {
        unsafe {
            gl::Uniform1i(
                location,
                value,
            );
        }
    }
}


fn set_uniform_3f(
    location: i32,
    x: f32,
    y: f32,
    z: f32,
) {

    if location >= 0 {
        unsafe {
            gl::Uniform3f(
                location,
                x,
                y,
                z,
            );
        }
    }
}


fn set_uniform_4f(
    location: i32,
    x: f32,
    y: f32,
    z: f32,
    w: f32,
) {

    if location >= 0 {
        unsafe {
            gl::Uniform4f(
                location,
                x,
                y,
                z,
                w,
            );
        }
    }
}


fn egl_failure(
    operation: &str,
) -> String {

    let error =
        unsafe {
            eglGetError()
        };


    format!(
        "{} failed with EGL error 0x{:04X}",
        operation,
        error,
    )
}

