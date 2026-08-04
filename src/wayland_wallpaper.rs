use wayland_client::protocol::{
    wl_compositor,
    wl_output,
    wl_region,
    wl_registry,
    wl_surface,
};
use std::collections::HashMap;
use std::ffi::{
    c_char,
    c_void,
    CString,
};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::ptr;
use std::sync::{
    atomic::{
        AtomicBool,
        Ordering,
    },
    Arc,
};
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


#[derive(Debug, Clone, Copy)]
enum WallpaperLayerStrategy {
    Background,
    BottomCompatibility,
}


impl WallpaperLayerStrategy {
    fn detect() -> Self {
        if let Some(value) = std::env::var_os("SWAYSOCK") {
            if !value.is_empty() {
                return Self::BottomCompatibility;
            }
        }

        for variable in [
            "XDG_CURRENT_DESKTOP",
            "DESKTOP_SESSION",
        ] {
            if let Ok(value) = std::env::var(variable) {
                if value
                    .to_ascii_lowercase()
                    .contains("sway")
                {
                    return Self::BottomCompatibility;
                }
            }
        }

        Self::Background
    }


    fn layer(self) -> Layer {
        match self {
            Self::Background => Layer::Background,
            Self::BottomCompatibility => Layer::Bottom,
        }
    }


    fn description(self) -> &'static str {
        match self {
            Self::Background => "standard background layer",
            Self::BottomCompatibility => "Sway bottom-layer compatibility",
        }
    }
}


#[derive(Debug, Clone, Default)]
pub struct WallpaperTargetInfo {
    pub registry_name: u32,
    pub connector_name: Option<String>,
    pub description: Option<String>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub logical_x: i32,
    pub logical_y: i32,
    pub physical_width_mm: i32,
    pub physical_height_mm: i32,
    pub mode_width: i32,
    pub mode_height: i32,
    pub refresh_millihertz: i32,
    pub scale: i32,
    pub subpixel: Option<String>,
    pub transform: Option<String>,
    pub complete: bool,
}


#[derive(Debug, Clone)]
struct OutputDispatchData {
    registry_name: u32,
}


#[derive(Debug, Clone)]
struct WallpaperTarget {
    output: wl_output::WlOutput,
    info: WallpaperTargetInfo,
}


#[derive(Debug, Clone)]
struct LayerSurfaceDispatchData {
    registry_name: u32,
}


#[derive(Debug, Clone, Default)]
struct WallpaperSurfaceState {
    registry_name: u32,
    configured: Option<WallpaperSurfaceConfiguration>,
    closed: bool,
}


struct NativeWallpaperTarget {
    info: WallpaperTargetInfo,
    surface: wl_surface::WlSurface,
    layer_surface: ZwlrLayerSurfaceV1,
    input_region: wl_region::WlRegion,
    egl_window: wayland_egl::WlEglSurface,
    width: i32,
    height: i32,
}


struct EglWallpaperTarget {
    registry_name: u32,
    surface: EglSurface,
}


#[derive(Debug, Default)]
pub struct WallpaperWaylandCapabilities {
    pub compositor_version: Option<u32>,
    pub layer_shell_version: Option<u32>,
    pub output_count: usize,
    pub targets: Vec<WallpaperTargetInfo>,
}


#[derive(Debug, Clone)]
pub struct WallpaperSurfaceConfiguration {
    pub width: u32,
    pub height: u32,
    pub serial: u32,
}


#[derive(Debug, Default)]
struct WaylandState {
    compositor: Option<wl_compositor::WlCompositor>,
    layer_shell: Option<ZwlrLayerShellV1>,
    compositor_version: Option<u32>,
    layer_shell_version: Option<u32>,
    output_count: usize,
    targets: Vec<WallpaperTarget>,
    surface_states: Vec<WallpaperSurfaceState>,
    removed_output_names: Vec<u32>,
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

        match event {

            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {

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


                        let output =
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
                                OutputDispatchData {
                                    registry_name:
                                        name,
                                },
                            );


                        state.targets.push(
                            WallpaperTarget {
                                output,

                                info:
                                    WallpaperTargetInfo {
                                        registry_name:
                                            name,

                                        scale:
                                            1,

                                        ..WallpaperTargetInfo::default()
                                    },
                            }
                        );
                    }


                    _ => {}
                }
            }


            wl_registry::Event::GlobalRemove {
                name,
            } => {

                let was_wallpaper_output =
                    state
                        .targets
                        .iter()
                        .any(
                            |target| {
                                target.info.registry_name
                                    == name
                            }
                        );


                if !was_wallpaper_output {
                    return;
                }


                state.targets.retain(
                    |target| {
                        target.info.registry_name
                            != name
                    }
                );


                state.output_count =
                    state
                        .output_count
                        .saturating_sub(
                            1
                        );


                if !state
                    .removed_output_names
                    .contains(
                        &name
                    )
                {
                    state
                        .removed_output_names
                        .push(
                            name
                        );
                }
            }


            _ => {}
        }
    }
}


impl Dispatch<ZwlrLayerSurfaceV1, LayerSurfaceDispatchData>
    for WaylandState
{
    fn event(
        state: &mut Self,
        layer_surface: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        data: &LayerSurfaceDispatchData,
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


                if let Some(
                    surface_state
                ) =
                    state
                        .surface_states
                        .iter_mut()
                        .find(
                            |surface_state| {
                                surface_state.registry_name
                                    == data.registry_name
                            }
                        )
                {
                    surface_state.configured =
                        Some(
                            WallpaperSurfaceConfiguration {
                                width,
                                height,
                                serial,
                            }
                        );
                }
            }


            zwlr_layer_surface_v1::Event::Closed => {

                if let Some(
                    surface_state
                ) =
                    state
                        .surface_states
                        .iter_mut()
                        .find(
                            |surface_state| {
                                surface_state.registry_name
                                    == data.registry_name
                            }
                        )
                {
                    surface_state.closed =
                        true;
                }
            }


            _ => {}
        }
    }
}


delegate_noop!(
    WaylandState:
    ignore wl_compositor::WlCompositor
);


impl Dispatch<wl_output::WlOutput, OutputDispatchData>
    for WaylandState
{
    fn event(
        state: &mut Self,
        _output: &wl_output::WlOutput,
        event: wl_output::Event,
        data: &OutputDispatchData,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {

        let Some(
            target
        ) =
            state
                .targets
                .iter_mut()
                .find(
                    |target| {
                        target.info.registry_name
                            == data.registry_name
                    }
                )
        else {
            return;
        };


        let output =
            &mut target.info;


        match event {

            wl_output::Event::Geometry {
                x,
                y,
                physical_width,
                physical_height,
                subpixel,
                make,
                model,
                transform,
            } => {

                output.logical_x =
                    x;


                output.logical_y =
                    y;


                output.physical_width_mm =
                    physical_width;


                output.physical_height_mm =
                    physical_height;


                output.subpixel =
                    Some(
                        format!(
                            "{:?}",
                            subpixel,
                        )
                    );


                output.make =
                    Some(
                        make
                    );


                output.model =
                    Some(
                        model
                    );


                output.transform =
                    Some(
                        format!(
                            "{:?}",
                            transform,
                        )
                    );
            }


            wl_output::Event::Mode {
                flags,
                width,
                height,
                refresh,
            } => {

                if format!(
                    "{:?}",
                    flags,
                )
                .contains(
                    "Current"
                ) {
                    output.mode_width =
                        width;


                    output.mode_height =
                        height;


                    output.refresh_millihertz =
                        refresh;
                }
            }


            wl_output::Event::Done => {

                output.complete =
                    true;
            }


            wl_output::Event::Scale {
                factor,
            } => {

                output.scale =
                    factor;
            }


            wl_output::Event::Name {
                name,
            } => {

                output.connector_name =
                    Some(
                        name
                    );
            }


            wl_output::Event::Description {
                description,
            } => {

                output.description =
                    Some(
                        description
                    );
            }


            _ => {}
        }
    }
}


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

            targets:
                state
                    .targets
                    .into_iter()
                    .map(
                        |target| {
                            target.info
                        }
                    )
                    .collect(),
        }
    )
}


#[derive(Debug, Clone)]
struct ActiveWallpaperShader {
    manager_name: String,
    source: String,
    shader_name: String,
    channel_usage:
        crate::preprocess_shader::ShaderChannelUsage,
    shader_inputs:
        Vec<crate::isf_types::ShaderInput>,
    built_in_default: bool,
}


fn select_safe_wallpaper_shader(
    shader_manager: &mut crate::manage_shader::ShaderManager,
    wallpaper_directory: &Path,
) -> Result<ActiveWallpaperShader, String> {

    let maximum_attempts =
        shader_manager.shader_count();


    for _ in
        0..maximum_attempts
    {
        let Some(
            requested_shader_name
        ) =
            shader_manager.next()
        else {
            break;
        };


        let shader_path =
            wallpaper_directory.join(
                &requested_shader_name
            );


        println!(
            "Evaluating wallpaper shader:"
        );


        println!(
            "    {}",
            shader_path.display()
        );


        match crate::load_shader::load_shader_for_preview(
            &shader_path
        ) {
            crate::load_shader::ShaderLoadResult::Ready {
                source,
                shader_name,
                channel_usage,
                shader_inputs,
                built_in_default,
            } => {
                return Ok(
                    ActiveWallpaperShader {
                        manager_name: requested_shader_name,
                        source,
                        shader_name,
                        channel_usage,
                        shader_inputs,
                        built_in_default,
                    }
                );
            }


            crate::load_shader::ShaderLoadResult::Rejected {
                shader_name,
                reasons,
            } => {
                println!(
                    "Wallpaper shader was rejected:"
                );


                println!(
                    "    Shader: {}",
                    shader_name
                );


                for reason in
                    reasons
                {
                    println!(
                        "    Reason: {}",
                        reason
                    );
                }


                shader_manager.remove_shader(
                    &requested_shader_name
                );
            }


            crate::load_shader::ShaderLoadResult::Unavailable {
                shader_name,
                error,
            } => {
                println!(
                    "Wallpaper shader is unavailable:"
                );


                println!(
                    "    Shader: {}",
                    shader_name
                );


                println!(
                    "    Error: {}",
                    error
                );


                shader_manager.remove_shader(
                    &requested_shader_name
                );
            }
        }


        println!();
    }


    Err(
        "No usable wallpaper shaders remain"
            .to_string()
    )
}


fn print_active_wallpaper_shader(
    shader: &ActiveWallpaperShader,
) {

    println!();


    println!(
        "Wallpaper shader is ready:"
    );


    println!(
        "    Shader: {}",
        shader.shader_name
    );


    println!(
        "    Processed source: {} bytes",
        shader.source.len()
    );


    println!(
        "    Built-in default: {}",
        shader.built_in_default
    );


    println!();
}


pub fn run_egl_background_surface(
    mut shader_manager: crate::manage_shader::ShaderManager,
    wallpaper_directory: &Path,
    shader_interval: Option<Duration>,
    runtime: &crate::define_wallpaper::WallpaperRuntime,
    running: Arc<AtomicBool>,
    control: crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
) -> Result<(), String> {

    runtime.tray_status
        .set_starting();


    let active_shader =
        select_safe_wallpaper_shader(
            &mut shader_manager,
            wallpaper_directory,
        )?;


    print_active_wallpaper_shader(
        &active_shader
    );


    runtime.tray_status
        .set_active(
            active_shader.shader_name.clone()
        );


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


    let targets =
        state.targets.clone();


    let wallpaper_layer_strategy =
        WallpaperLayerStrategy::detect();


    println!(
        "Wayland wallpaper compatibility:"
    );


    println!(
        "    Layer strategy: {}",
        wallpaper_layer_strategy.description()
    );


    println!();


    println!(
        "Creating mirror-mode wallpaper surfaces:"
    );


    println!(
        "    Target count: {}",
        targets.len()
    );


    let mut pending_targets =
        Vec::with_capacity(
            targets.len()
        );


    for target in targets {

        print_selected_target(
            &target.info
        );


        state.surface_states.push(
            WallpaperSurfaceState {
                registry_name:
                    target.info.registry_name,

                configured:
                    None,

                closed:
                    false,
            }
        );


        let surface =
            compositor.create_surface(
                &queue_handle,
                (),
            );


        let layer_surface =
            layer_shell.get_layer_surface(
                &surface,
                Some(
                    &target.output
                ),
                wallpaper_layer_strategy.layer(),
                format!(
                    "screenshaver-wallpaper-{}",
                    target.info.registry_name,
                ),
                &queue_handle,
                LayerSurfaceDispatchData {
                    registry_name:
                        target.info.registry_name,
                },
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


        let input_region =
            compositor.create_region(
                &queue_handle,
                (),
            );


        surface.set_input_region(
            Some(
                &input_region
            )
        );


        surface.commit();


        pending_targets.push(
            (
                target.info,
                surface,
                layer_surface,
                input_region,
            )
        );
    }


    connection
        .flush()
        .map_err(
            |error| {
                format!(
                    "Unable to send mirror wallpaper surface requests: {}",
                    error,
                )
            }
        )?;


    while state
        .surface_states
        .iter()
        .any(
            |surface_state| {
                surface_state.configured.is_none()
                    && !surface_state.closed
            }
        )
    {
        event_queue
            .blocking_dispatch(
                &mut state
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to receive mirror wallpaper configure events: {}",
                        error,
                    )
                }
            )?;
    }


    if let Some(
        closed_target
    ) =
        state
            .surface_states
            .iter()
            .find(
                |surface_state| {
                    surface_state.closed
                        && surface_state.configured.is_none()
                }
            )
    {
        return Err(
            format!(
                "The compositor closed wallpaper target {} before configuring it",
                closed_target.registry_name,
            )
        );
    }


    connection
        .flush()
        .map_err(
            |error| {
                format!(
                    "Unable to send mirror wallpaper configure acknowledgements: {}",
                    error,
                )
            }
        )?;


    let mut native_targets =
        Vec::with_capacity(
            pending_targets.len()
        );


    for (
        info,
        surface,
        layer_surface,
        input_region,
    ) in pending_targets {

        let configuration =
            state
                .surface_states
                .iter_mut()
                .find(
                    |surface_state| {
                        surface_state.registry_name
                            == info.registry_name
                    }
                )
                .and_then(
                    |surface_state| {
                        surface_state.configured.take()
                    }
                )
                .ok_or_else(
                    || {
                        format!(
                            "Wallpaper target {} was not configured",
                            info.registry_name,
                        )
                    }
                )?;


        let width =
            configured_dimension(
                configuration.width,
                info.mode_width,
                "width",
            )?;


        let height =
            configured_dimension(
                configuration.height,
                info.mode_height,
                "height",
            )?;


        print_target_surface_configuration(
            &info,
            &configuration,
            width,
            height,
        );


        let egl_window =
            wayland_egl::WlEglSurface::new(
                surface.id(),
                width,
                height,
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to create wl_egl_window for target {}: {}",
                        info.registry_name,
                        error,
                    )
                }
            )?;


        native_targets.push(
            NativeWallpaperTarget {
                info,
                surface,
                layer_surface,
                input_region,
                egl_window,
                width,
                height,
            }
        );
    }


    render_egl_wallpapers(
        &connection,
        &mut event_queue,
        &mut state,
        &mut native_targets,
        &active_shader,
        &mut shader_manager,
        wallpaper_directory,
        shader_interval,
        runtime,
        &running,
        &control,
    )?;


    for target in native_targets {

        drop(
            target.egl_window
        );


        target.layer_surface.destroy();


        target.surface.destroy();


        target.input_region.destroy();
    }


    layer_shell.destroy();


    connection
        .flush()
        .map_err(
            |error| {
                format!(
                    "Unable to send mirror wallpaper cleanup requests: {}",
                    error,
                )
            }
        )?;


    Ok(
        ()
    )
}


fn configured_dimension(
    configured: u32,
    fallback: i32,
    dimension_name: &str,
) -> Result<i32, String> {

    if configured
        != 0
    {
        return i32::try_from(
            configured
        )
        .map_err(
            |_| {
                format!(
                    "Wallpaper surface {} exceeds the EGL window range",
                    dimension_name,
                )
            }
        );
    }


    if fallback
        > 0
    {
        return Ok(
            fallback
        );
    }


    Err(
        format!(
            "The compositor returned zero {} and no valid output-mode fallback is available",
            dimension_name,
        )
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


    event_queue
        .roundtrip(
            &mut state
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read Wayland output metadata: {}",
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
        .targets
        .is_empty()
    {
        return Err(
            "The Wayland compositor did not provide any wallpaper targets"
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


fn render_egl_wallpapers(
    connection: &Connection,
    event_queue: &mut wayland_client::EventQueue<WaylandState>,
    state: &mut WaylandState,
    native_targets: &mut Vec<NativeWallpaperTarget>,
    active_shader: &ActiveWallpaperShader,
    shader_manager: &mut crate::manage_shader::ShaderManager,
    wallpaper_directory: &Path,
    shader_interval: Option<Duration>,
    runtime: &crate::define_wallpaper::WallpaperRuntime,
    running: &Arc<AtomicBool>,
    control: &crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
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


    let mut config_count =
        0;


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


    let mut egl_targets =
        Vec::with_capacity(
            native_targets.len()
        );


    for target in native_targets.iter() {

        let surface =
            unsafe {
                eglCreateWindowSurface(
                    display,
                    config,
                    target.egl_window.ptr()
                        as EglNativeWindow,
                    ptr::null(),
                )
            };


        if surface
            == EGL_NO_SURFACE
        {
            destroy_egl_targets(
                display,
                &egl_targets,
            );


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


        egl_targets.push(
            EglWallpaperTarget {
                registry_name:
                    target.info.registry_name,

                surface,
            }
        );
    }


    let first_surface =
        egl_targets
            .first()
            .ok_or_else(
                || {
                    "No EGL wallpaper targets were created"
                        .to_string()
                }
            )?
            .surface;


    if unsafe {
        eglMakeCurrent(
            display,
            first_surface,
            first_surface,
            context,
        )
    } == EGL_FALSE
    {
        destroy_egl_targets(
            display,
            &egl_targets,
        );


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
        render_mirror_frames(
            display,
            context,
            &mut egl_targets,
            event_queue,
            state,
            native_targets,
            active_shader,
            shader_manager,
            wallpaper_directory,
            shader_interval,
            runtime,
            running,
            control,
        );


    unsafe {
        eglMakeCurrent(
            display,
            EGL_NO_SURFACE,
            EGL_NO_SURFACE,
            EGL_NO_CONTEXT,
        );
    }


    destroy_egl_targets(
        display,
        &egl_targets,
    );


    unsafe {
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
        "Native EGL mirror wallpaper renderer stopped:"
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
        "    Rendered targets: {}",
        native_targets.len()
    );


    println!(
        "    Shutdown reason: {}",
        if state
            .surface_states
            .iter()
            .any(
                |surface_state| {
                    surface_state.closed
                }
            )
        {
            "compositor closed a layer surface"
        } else if native_targets
            .is_empty()
        {
            "all wallpaper outputs disconnected"
        } else {
            "Screenshaver shutdown requested"
        }
    );


    Ok(
        ()
    )
}


fn destroy_egl_targets(
    display: EglDisplay,
    targets: &[EglWallpaperTarget],
) {

    for target in targets {

        unsafe {
            eglDestroySurface(
                display,
                target.surface,
            );
        }
    }
}


fn render_mirror_frames(
    display: EglDisplay,
    context: EglContext,
    egl_targets: &mut Vec<EglWallpaperTarget>,
    event_queue: &mut wayland_client::EventQueue<WaylandState>,
    state: &mut WaylandState,
    native_targets: &mut Vec<NativeWallpaperTarget>,
    active_shader: &ActiveWallpaperShader,
    shader_manager: &mut crate::manage_shader::ShaderManager,
    wallpaper_directory: &Path,
    shader_interval: Option<Duration>,
    runtime: &crate::define_wallpaper::WallpaperRuntime,
    running: &Arc<AtomicBool>,
    control: &crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
) -> Result<(), String> {

    let mut program =
        crate::compile_shader::build_program(
            crate::define_constants::VERTEX_SHADER,
            &active_shader.source,
        )?;


    let mut texture_manager =
        crate::manage_textures::TextureManager::new(
            runtime.texture_policy.clone()
        );


    if let Err(error) =
        texture_manager.prepare_for_shader(
            &active_shader.shader_name,
            active_shader.channel_usage,
        )
    {
        unsafe {
            gl::DeleteProgram(
                program
            );
        }


        return Err(
            format!(
                "Unable to prepare textures for wallpaper shader '{}': {}",
                active_shader.shader_name,
                error,
            )
        );
    }


    texture_manager.configure_program(
        program
    );


    let mut animation_speed =
        runtime.animation_speed_policy
            .animation_speed_for_shader(
                &active_shader.shader_name,
                None,
            );


    let mut rendered_fps =
        runtime.fps_policy.rendered_fps_for_shader(
            &active_shader.shader_name,
            None,
        );


    let mut frame_duration =
        frame_duration_for_fps(
            rendered_fps
        );


    let mut next_frame_deadline =
        Instant::now();


    notify_active_wallpaper(
        runtime.notifications,
        active_shader,
        &texture_manager,
        rendered_fps,
        animation_speed,
        crate::fps_monitor::FpsWarningState::Normal,
    );


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


    crate::apply_shader_inputs::apply(
        program,
        &active_shader.shader_inputs,
    );


    let mut postprocess_pipelines =
        HashMap::with_capacity(
            native_targets.len()
        );


    for target in native_targets.iter() {
        let width =
            u32::try_from(
                target.width
            )
            .map_err(
                |_| {
                    format!(
                        "Wallpaper target {} has an invalid post-processing width: {}",
                        target.info.registry_name,
                        target.width,
                    )
                }
            )?;


        let height =
            u32::try_from(
                target.height
            )
            .map_err(
                |_| {
                    format!(
                        "Wallpaper target {} has an invalid post-processing height: {}",
                        target.info.registry_name,
                        target.height,
                    )
                }
            )?;


        let mut pipeline =
            crate::postprocess_shader::PostprocessPipeline::new(
                width,
                height,
            )?;

        pipeline.set_profile(
            runtime.postprocess_policy.profile_for_shader(
                &active_shader.shader_name
            )
        );


        postprocess_pipelines.insert(
            target.info.registry_name,
            pipeline,
        );
    }


    let mut i_time =
        uniform_location(
            program,
            "iTime",
        )?;


    let mut i_resolution =
        uniform_location(
            program,
            "iResolution",
        )?;


    let mut i_mouse =
        uniform_location(
            program,
            "iMouse",
        )?;


    let mut i_frame =
        uniform_location(
            program,
            "iFrame",
        )?;


    let mut start_time =
        Instant::now();


    let mut last_shader_switch =
        Instant::now();


    let mut frame =
        0_i32;


    let mut current_shader =
        active_shader.clone();


    let mut frame_times =
        crate::fps_monitor::FrameTimeWindow::new();


    let mut fps_warning_state =
        crate::fps_monitor::FpsWarningState::Normal;


    let mut paused =
        false;


    let mut pause_started: Option<Instant> =
        None;


    let result =
        'render_loop: loop {

            process_wayland_events(
                event_queue,
                state,
            )?;


            remove_disconnected_targets(
                display,
                context,
                state,
                egl_targets,
                native_targets,
                &mut postprocess_pipelines,
            )?;


            if native_targets
                .is_empty()
            {
                println!(
                    "All wallpaper outputs have been disconnected."
                );


                break Ok(());
            }


            apply_pending_target_resizes(
                state,
                native_targets,
            )?;


            if state
                .surface_states
                .iter()
                .any(
                    |surface_state| {
                        surface_state.closed
                    }
                )
                || !running.load(
                    Ordering::SeqCst
                )
            {
                break Ok(());
            }


            if control.pause_requested() {
                if !paused {
                    paused =
                        true;


                    pause_started =
                        Some(
                            Instant::now()
                        );


                    control.acknowledge_paused();


                    println!(
                        "Wayland wallpaper rendering paused."
                    );
                }


                thread::sleep(
                    Duration::from_millis(10)
                );


                continue 'render_loop;
            }


            if paused {
                if let Some(paused_at) =
                    pause_started.take()
                {
                    let paused_duration =
                        paused_at.elapsed();


                    start_time +=
                        paused_duration;


                    last_shader_switch +=
                        paused_duration;
                }


                next_frame_deadline =
                    Instant::now();


                frame_times.clear();
            }


            if !paused
                && shader_interval
                    .map(
                        |interval| {
                            last_shader_switch.elapsed()
                                >= interval
                        }
                    )
                    .unwrap_or(
                        false
                    )
            {
                last_shader_switch =
                    Instant::now();


                if let Some(
                    first_target
                ) = egl_targets.first()
                {
                    if unsafe {
                        eglMakeCurrent(
                            display,
                            first_target.surface,
                            first_target.surface,
                            context,
                        )
                    } == EGL_FALSE
                    {
                        break 'render_loop Err(
                            egl_failure(
                                "eglMakeCurrent before switching wallpaper shaders"
                            )
                        );
                    }
                }


                match select_safe_wallpaper_shader(
                    shader_manager,
                    wallpaper_directory,
                ) {
                    Ok(
                        next_shader
                    ) => {
                        match crate::compile_shader::build_program(
                            crate::define_constants::VERTEX_SHADER,
                            &next_shader.source,
                        ) {
                            Ok(
                                next_program
                            ) => {
                                let mut next_texture_manager =
                                    crate::manage_textures::TextureManager::new(
                                        runtime.texture_policy.clone()
                                    );


                                if let Err(
                                    error
                                ) =
                                    next_texture_manager.prepare_for_shader(
                                        &next_shader.shader_name,
                                        next_shader.channel_usage,
                                    )
                                {
                                    println!(
                                        "Wallpaper shader texture preparation failed; keeping the current shader:"
                                    );


                                    println!(
                                        "    Shader: {}",
                                        next_shader.shader_name
                                    );


                                    println!(
                                        "    Error: {}",
                                        error
                                    );


                                    println!();


                                    unsafe {
                                        gl::DeleteProgram(
                                            next_program
                                        );
                                    }


                                    shader_manager.remove_shader(
                                        &next_shader.manager_name
                                    );


                                    continue 'render_loop;
                                }


                                next_texture_manager.configure_program(
                                    next_program
                                );


                                unsafe {
                                    gl::UseProgram(
                                        next_program
                                    );
                                }


                                crate::apply_shader_inputs::apply(
                                    next_program,
                                    &next_shader.shader_inputs,
                                );


                                let next_i_time =
                                    uniform_location(
                                        next_program,
                                        "iTime",
                                    )?;


                                let next_i_resolution =
                                    uniform_location(
                                        next_program,
                                        "iResolution",
                                    )?;


                                let next_i_mouse =
                                    uniform_location(
                                        next_program,
                                        "iMouse",
                                    )?;


                                let next_i_frame =
                                    uniform_location(
                                        next_program,
                                        "iFrame",
                                    )?;


                                texture_manager.delete_all();


                                texture_manager =
                                    next_texture_manager;


                                unsafe {
                                    gl::UseProgram(
                                        0
                                    );

                                    gl::DeleteProgram(
                                        program
                                    );
                                }


                                program =
                                    next_program;


                                i_time =
                                    next_i_time;


                                i_resolution =
                                    next_i_resolution;


                                i_mouse =
                                    next_i_mouse;


                                i_frame =
                                    next_i_frame;


                                start_time =
                                    Instant::now();


                                frame =
                                    0;


                                animation_speed =
                                    runtime.animation_speed_policy
                                        .animation_speed_for_shader(
                                            &next_shader.shader_name,
                                            None,
                                        );


                                rendered_fps =
                                    runtime.fps_policy.rendered_fps_for_shader(
                                        &next_shader.shader_name,
                                        None,
                                    );


                                frame_duration =
                                    frame_duration_for_fps(
                                        rendered_fps
                                    );


                                next_frame_deadline =
                                    Instant::now();


                                current_shader =
                                    next_shader.clone();


                                let postprocess_profile =
                                    runtime.postprocess_policy
                                        .profile_for_shader(
                                            &current_shader.shader_name
                                        );


                                for pipeline in
                                    postprocess_pipelines.values_mut()
                                {
                                    pipeline.set_profile(
                                        postprocess_profile
                                    );
                                }


                                runtime.tray_status
                                    .set_active(
                                        current_shader.shader_name.clone()
                                    );


                                frame_times.clear();


                                fps_warning_state =
                                    crate::fps_monitor::FpsWarningState::Normal;


                                notify_active_wallpaper(
                                    runtime.notifications,
                                    &current_shader,
                                    &texture_manager,
                                    rendered_fps,
                                    animation_speed,
                                    fps_warning_state,
                                );


                                println!(
                                    "Wallpaper shader changed:"
                                );


                                println!(
                                    "    Shader: {}",
                                    next_shader.shader_name
                                );


                                println!(
                                    "    FPS: {}",
                                    rendered_fps
                                );


                                println!(
                                    "    Animation speed: {:.3}x",
                                    animation_speed
                                );


                                if let Some(
                                    interval
                                ) = shader_interval
                                {
                                    println!(
                                        "    Interval: {} seconds",
                                        interval.as_secs()
                                    );
                                }


                                println!();
                            }


                            Err(
                                error
                            ) => {
                                println!(
                                    "Wallpaper shader compilation failed; keeping the current shader:"
                                );


                                println!(
                                    "    Shader: {}",
                                    next_shader.shader_name
                                );


                                println!(
                                    "    Error: {}",
                                    error
                                );


                                println!();


                                shader_manager.remove_shader(
                                    &next_shader.manager_name
                                );
                            }
                        }
                    }


                    Err(
                        error
                    ) => {
                        println!(
                            "Wallpaper shader selection failed; keeping the current shader:"
                        );


                        println!(
                            "    Error: {}",
                            error
                        );


                        println!();
                    }
                }
            }


            let elapsed =
                start_time.elapsed();


            let shader_render_start =
                Instant::now();


            for (
                egl_target,
                native_target,
            ) in egl_targets
                .iter()
                .zip(
                    native_targets.iter()
                )
            {
                if unsafe {
                    eglMakeCurrent(
                        display,
                        egl_target.surface,
                        egl_target.surface,
                        context,
                    )
                } == EGL_FALSE
                {
                    break 'render_loop Err(
                        egl_failure(
                            "eglMakeCurrent"
                        )
                    );
                }


                let postprocess =
                    postprocess_pipelines
                        .get_mut(
                            &native_target.info.registry_name
                        )
                        .ok_or_else(
                            || {
                                format!(
                                    "No post-processing pipeline exists for wallpaper target {}",
                                    native_target.info.registry_name,
                                )
                            }
                        )?;


                let target_width =
                    u32::try_from(
                        native_target.width
                    )
                    .map_err(
                        |_| {
                            format!(
                                "Wallpaper target {} has an invalid width: {}",
                                native_target.info.registry_name,
                                native_target.width,
                            )
                        }
                    )?;


                let target_height =
                    u32::try_from(
                        native_target.height
                    )
                    .map_err(
                        |_| {
                            format!(
                                "Wallpaper target {} has an invalid height: {}",
                                native_target.info.registry_name,
                                native_target.height,
                            )
                        }
                    )?;


                postprocess.resize(
                    target_width,
                    target_height,
                )?;


                postprocess.bind_scene_target();


                unsafe {
                    gl::Disable(
                        gl::BLEND
                    );

                    gl::ColorMask(
                        gl::TRUE,
                        gl::TRUE,
                        gl::TRUE,
                        gl::TRUE,
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

                    gl::ColorMask(
                        gl::TRUE,
                        gl::TRUE,
                        gl::TRUE,
                        gl::FALSE,
                    );

                    gl::UseProgram(
                        program
                    );
                }


                texture_manager.bind_channels();


                unsafe {
                    set_uniform_1f(
                        i_time,
                        elapsed.as_secs_f32()
                            * animation_speed,
                    );

                    set_uniform_3f(
                        i_resolution,
                        native_target.width as f32,
                        native_target.height as f32,
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

                    gl::ColorMask(
                        gl::TRUE,
                        gl::TRUE,
                        gl::TRUE,
                        gl::TRUE,
                    );
                }


                postprocess.present_scene();


                if unsafe {
                    eglSwapBuffers(
                        display,
                        egl_target.surface,
                    )
                } == EGL_FALSE
                {
                    break 'render_loop Err(
                        egl_failure(
                            "eglSwapBuffers"
                        )
                    );
                }
            }


            if paused {
                paused =
                    false;


                control.acknowledge_resumed_frame();


                println!(
                    "Wayland wallpaper rendering resumed."
                );
            }


            unsafe {
                gl::Finish();
            }


            let performance_status =
                frame_times.record(
                    shader_render_start.elapsed(),
                    rendered_fps,
                );


            let new_warning_state =
                performance_status.warning_state;


            if new_warning_state
                != fps_warning_state
            {
                fps_warning_state =
                    new_warning_state;


                println!(
                    "Wallpaper performance changed:"
                );


                println!(
                    "    Target FPS: {}",
                    rendered_fps
                );


                println!(
                    "    Average FPS: {}",
                    performance_status.average_fps
                );


                println!(
                    "    State: {:?}",
                    fps_warning_state
                );


                println!();


                if fps_warning_state
                    != crate::fps_monitor::FpsWarningState::Normal
                {
                    notify_active_wallpaper(
                        runtime.notifications,
                        &current_shader,
                        &texture_manager,
                        rendered_fps,
                        animation_speed,
                        fps_warning_state,
                    );
                }
            }


            frame =
                frame.saturating_add(
                    1
                );


            next_frame_deadline +=
                frame_duration;


            let now =
                Instant::now();


            if next_frame_deadline
                > now
            {
                thread::sleep(
                    next_frame_deadline
                        - now
                );
            } else {
                next_frame_deadline =
                    now;
            }
        };


    drop(
        postprocess_pipelines
    );


    texture_manager.delete_all();


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


fn frame_duration_for_fps(
    fps: u32,
) -> Duration {

    Duration::from_secs_f64(
        1.0 / fps.max(1) as f64
    )
}


fn notify_active_wallpaper(
    enabled: bool,
    shader: &ActiveWallpaperShader,
    texture_manager: &crate::manage_textures::TextureManager,
    fps: u32,
    animation_speed: f32,
    warning_state: crate::fps_monitor::FpsWarningState,
) {

    let selection =
        texture_manager.active_specification_selection();


    let metadata =
        crate::notify_wallpaper::WallpaperMetadata {
            wallpaper:
                shader.shader_name.clone(),

            animation_speed,

            texture:
                selection.map(
                    |(specification, _)| {
                        specification.display_name()
                    }
                ),

            palette:
                selection.map(
                    |(_, palette)| {
                        palette.name()
                            .to_string()
                    }
                ),

            fps:
                fps.max(1),

            warning_state,
        };


    crate::notify_wallpaper::show(
        enabled,
        &metadata,
    );
}


fn remove_disconnected_targets(
    display: EglDisplay,
    context: EglContext,
    state: &mut WaylandState,
    egl_targets: &mut Vec<EglWallpaperTarget>,
    native_targets: &mut Vec<NativeWallpaperTarget>,
    postprocess_pipelines:
        &mut HashMap<
            u32,
            crate::postprocess_shader::PostprocessPipeline,
        >,
) -> Result<(), String> {

    let removed_names =
        std::mem::take(
            &mut state.removed_output_names
        );


    for registry_name in removed_names {

        let Some(
            egl_index
        ) =
            egl_targets
                .iter()
                .position(
                    |target| {
                        target.registry_name
                            == registry_name
                    }
                )
        else {
            continue;
        };


        let Some(
            native_index
        ) =
            native_targets
                .iter()
                .position(
                    |target| {
                        target.info.registry_name
                            == registry_name
                    }
                )
        else {
            return Err(
                format!(
                    "Wallpaper output {} disappeared, but its native target was not found",
                    registry_name,
                )
            );
        };


        let removed_surface =
            egl_targets[
                egl_index
            ]
            .surface;


        if unsafe {
            eglMakeCurrent(
                display,
                removed_surface,
                removed_surface,
                context,
            )
        } == EGL_FALSE
        {
            return Err(
                egl_failure(
                    "eglMakeCurrent before removing wallpaper post-processing resources"
                )
            );
        }


        postprocess_pipelines.remove(
            &registry_name
        );


        if unsafe {
            eglMakeCurrent(
                display,
                EGL_NO_SURFACE,
                EGL_NO_SURFACE,
                EGL_NO_CONTEXT,
            )
        } == EGL_FALSE
        {
            return Err(
                egl_failure(
                    "eglMakeCurrent while removing a wallpaper target"
                )
            );
        }


        let egl_target =
            egl_targets.remove(
                egl_index
            );


        if unsafe {
            eglDestroySurface(
                display,
                egl_target.surface,
            )
        } == EGL_FALSE
        {
            return Err(
                egl_failure(
                    "eglDestroySurface while removing a wallpaper target"
                )
            );
        }


        let native_target =
            native_targets.remove(
                native_index
            );


        let connector =
            native_target
                .info
                .connector_name
                .clone()
                .unwrap_or_else(
                    || {
                        "<unknown>"
                            .to_string()
                    }
                );


        let NativeWallpaperTarget {
            surface,
            layer_surface,
            input_region,
            egl_window,
            ..
        } = native_target;


        drop(
            egl_window
        );


        layer_surface.destroy();


        surface.destroy();


        input_region.destroy();


        state
            .surface_states
            .retain(
                |surface_state| {
                    surface_state.registry_name
                        != registry_name
                }
            );


        println!(
            "Wallpaper target removed:"
        );


        println!(
            "    Registry name: {}",
            registry_name
        );


        println!(
            "    Connector: {}",
            connector
        );


        println!(
            "    Remaining targets: {}",
            native_targets.len()
        );


        println!();


        if let Some(
            next_target
        ) =
            egl_targets.first()
        {
            if unsafe {
                eglMakeCurrent(
                    display,
                    next_target.surface,
                    next_target.surface,
                    context,
                )
            } == EGL_FALSE
            {
                return Err(
                    egl_failure(
                        "eglMakeCurrent after removing a wallpaper target"
                    )
                );
            }
        }
    }


    Ok(())
}


fn apply_pending_target_resizes(
    state: &mut WaylandState,
    native_targets: &mut [NativeWallpaperTarget],
) -> Result<(), String> {

    for surface_state in state.surface_states.iter_mut() {

        let Some(
            configuration
        ) =
            surface_state.configured.take()
        else {
            continue;
        };


        let Some(
            target
        ) =
            native_targets
                .iter_mut()
                .find(
                    |target| {
                        target.info.registry_name
                            == surface_state.registry_name
                    }
                )
        else {
            continue;
        };


        let new_width =
            if configuration.width
                == 0
            {
                target.width
            } else {
                i32::try_from(
                    configuration.width
                )
                .map_err(
                    |_| {
                        "Wallpaper resize width exceeds the EGL window range"
                            .to_string()
                    }
                )?
            };


        let new_height =
            if configuration.height
                == 0
            {
                target.height
            } else {
                i32::try_from(
                    configuration.height
                )
                .map_err(
                    |_| {
                        "Wallpaper resize height exceeds the EGL window range"
                            .to_string()
                    }
                )?
            };


        if new_width
            == target.width
            && new_height
                == target.height
        {
            continue;
        }


        target
            .egl_window
            .resize(
                new_width,
                new_height,
                0,
                0,
            );


        target.width =
            new_width;


        target.height =
            new_height;


        println!(
            "Wayland wallpaper target resized:"
        );


        println!(
            "    Connector: {}",
            target
                .info
                .connector_name
                .as_deref()
                .unwrap_or(
                    "<not advertised>"
                )
        );


        println!(
            "    Width: {}",
            new_width
        );


        println!(
            "    Height: {}",
            new_height
        );


        println!(
            "    Configure serial: {}",
            configuration.serial
        );
    }


    Ok(
        ()
    )
}


#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}


const POLLIN: i16 = 0x0001;
const POLLERR: i16 = 0x0008;
const POLLHUP: i16 = 0x0010;


unsafe extern "C" {
    fn poll(
        file_descriptors: *mut PollFd,
        descriptor_count: usize,
        timeout_milliseconds: i32,
    ) -> i32;
}


fn process_wayland_events(
    event_queue: &mut wayland_client::EventQueue<WaylandState>,
    state: &mut WaylandState,
) -> Result<(), String> {

    event_queue
        .flush()
        .map_err(
            |error| {
                format!(
                    "Unable to flush Wayland wallpaper requests: {}",
                    error,
                )
            }
        )?;


    event_queue
        .dispatch_pending(
            state
        )
        .map_err(
            |error| {
                format!(
                    "Unable to dispatch pending Wayland wallpaper events: {}",
                    error,
                )
            }
        )?;


    let Some(
        read_guard
    ) =
        event_queue.prepare_read()
    else {
        return Ok(());
    };


    let mut descriptor =
        PollFd {
            fd:
                read_guard
                    .connection_fd()
                    .as_raw_fd(),

            events:
                POLLIN,

            revents:
                0,
        };


    let poll_result =
        unsafe {
            poll(
                &mut descriptor,
                1,
                0,
            )
        };


    if poll_result
        < 0
    {
        return Err(
            format!(
                "Unable to poll the Wayland connection: {}",
                std::io::Error::last_os_error(),
            )
        );
    }


    if poll_result
        == 0
    {
        drop(
            read_guard
        );


        return Ok(());
    }


    if descriptor.revents
        & (
            POLLIN
                | POLLERR
                | POLLHUP
        )
        != 0
    {
        read_guard
            .read()
            .map_err(
                |error| {
                    format!(
                        "Unable to read Wayland wallpaper events: {}",
                        error,
                    )
                }
            )?;


        event_queue
            .dispatch_pending(
                state
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to dispatch Wayland wallpaper events: {}",
                        error,
                    )
                }
            )?;
    } else {
        drop(
            read_guard
        );
    }


    Ok(
        ()
    )
}


fn apply_surface_resize(
    egl_window: &wayland_egl::WlEglSurface,
    configuration: &WallpaperSurfaceConfiguration,
    width: &mut i32,
    height: &mut i32,
) -> Result<(), String> {

    let requested_width =
        if configuration.width
            == 0
        {
            *width
        } else {
            i32::try_from(
                configuration.width
            )
            .map_err(
                |_| {
                    "Configured wallpaper width exceeds the EGL window range"
                        .to_string()
                }
            )?
        };


    let requested_height =
        if configuration.height
            == 0
        {
            *height
        } else {
            i32::try_from(
                configuration.height
            )
            .map_err(
                |_| {
                    "Configured wallpaper height exceeds the EGL window range"
                        .to_string()
                }
            )?
        };


    if requested_width
        <= 0
        || requested_height
            <= 0
    {
        return Err(
            format!(
                "The compositor configured an invalid wallpaper size: {}x{}",
                requested_width,
                requested_height,
            )
        );
    }


    if requested_width
        == *width
        && requested_height
            == *height
    {
        return Ok(());
    }


    egl_window.resize(
        requested_width,
        requested_height,
        0,
        0,
    );


    *width =
        requested_width;


    *height =
        requested_height;


    println!(
        "Wayland wallpaper surface resized:"
    );


    println!(
        "    Width: {}",
        *width
    );


    println!(
        "    Height: {}",
        *height
    );


    println!(
        "    Configure serial: {}",
        configuration.serial
    );


    Ok(())
}


fn print_selected_target(
    target: &WallpaperTargetInfo,
) {

    println!(
        "Creating wallpaper target:"
    );


    println!(
        "    Registry name: {}",
        target.registry_name
    );


    println!(
        "    Connector: {}",
        target
            .connector_name
            .as_deref()
            .unwrap_or(
                "<not advertised>"
            )
    );


    println!(
        "    Description: {}",
        target
            .description
            .as_deref()
            .unwrap_or(
                "<not advertised>"
            )
    );


    println!(
        "    Current mode: {}x{} @ {:.3} Hz",
        target.mode_width,
        target.mode_height,
        target.refresh_millihertz as f64
            / 1000.0
    );


    println!(
        "    Scale: {}",
        target.scale
    );


    println!();
}


fn print_target_surface_configuration(
    target: &WallpaperTargetInfo,
    configuration: &WallpaperSurfaceConfiguration,
    width: i32,
    height: i32,
) {

    println!(
        "Wayland background surface configured successfully:"
    );


    println!(
        "    Connector: {}",
        target
            .connector_name
            .as_deref()
            .unwrap_or(
                "<not advertised>"
            )
    );


    println!(
        "    Registry name: {}",
        target.registry_name
    );


    println!(
        "    Width: {}",
        width
    );


    println!(
        "    Height: {}",
        height
    );


    println!(
        "    Configure serial: {}",
        configuration.serial
    );


    println!(
        "    Layer: background"
    );


    println!(
        "    Anchors: top, bottom, left, right"
    );


    println!(
        "    Keyboard input: disabled"
    );


    println!(
        "    Pointer input: disabled"
    );


    println!();
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

