use wayland_client::protocol::{
    wl_callback,
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
use std::path::{Path, PathBuf};
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

use wayland_protocols::xdg::decoration::zv1::client::{
    zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
    zxdg_toplevel_decoration_v1::{self, ZxdgToplevelDecorationV1},
};

use wayland_protocols::xdg::shell::client::{
    xdg_surface::{
        self,
        XdgSurface,
    },
    xdg_toplevel::{
        self,
        XdgToplevel,
    },
    xdg_wm_base::{
        self,
        XdgWmBase,
    },
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
pub(crate) struct WallpaperTargetInfo {
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
pub(crate) struct WallpaperSurfaceState {
    pub(crate) registry_name: u32,
    pub(crate) configured: Option<WallpaperSurfaceConfiguration>,
    pub(crate) closed: bool,
}


pub(crate) struct NativeWallpaperTarget {
    pub(crate) info: WallpaperTargetInfo,
    pub(crate) surface: wl_surface::WlSurface,
    pub(crate) layer_surface: Option<ZwlrLayerSurfaceV1>,
    pub(crate) input_region: Option<wl_region::WlRegion>,
    pub(crate) xdg_surface: Option<XdgSurface>,
    pub(crate) xdg_toplevel: Option<XdgToplevel>,
    pub(crate) toplevel_decoration: Option<ZxdgToplevelDecorationV1>,
    pub(crate) egl_window: wayland_egl::WlEglSurface,
    pub(crate) width: i32,
    pub(crate) height: i32,
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
pub(crate) struct WallpaperSurfaceConfiguration {
    pub width: u32,
    pub height: u32,
    pub serial: u32,
}


#[derive(Debug, Default)]
pub(crate) struct WaylandState {
    pub(crate) compositor: Option<wl_compositor::WlCompositor>,
    pub(crate) layer_shell: Option<ZwlrLayerShellV1>,
    pub(crate) xdg_wm_base: Option<XdgWmBase>,
    pub(crate) decoration_manager: Option<ZxdgDecorationManagerV1>,
    pub(crate) compositor_version: Option<u32>,
    pub(crate) layer_shell_version: Option<u32>,
    pub(crate) xdg_wm_base_version: Option<u32>,
    pub(crate) output_count: usize,
    pub(crate) targets: Vec<WallpaperTarget>,
    pub(crate) surface_states: Vec<WallpaperSurfaceState>,
    pub(crate) removed_output_names: Vec<u32>,
    pub(crate) windowshader: crate::wayland_windowshader::WindowshaderWaylandState,
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


                    "xdg_wm_base" | "zxdg_decoration_manager_v1" => {
                        crate::wayland_windowshader::bind_windowshader_registry_global(
                            state, registry, name, interface.as_str(), version, queue_handle,
                        );
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
    ignore ZxdgDecorationManagerV1
);

impl Dispatch<ZxdgToplevelDecorationV1, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _decoration: &ZxdgToplevelDecorationV1,
        _event: zxdg_toplevel_decoration_v1::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        // The compositor chooses the effective decoration mode.
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


/// Windowshader requires xdg_wm_base, not the layer-shell protocol used by wallpapers.
/// This probe uses the same compositor requirements as the windowed rendering path.
pub(crate) fn probe_windowshader_capabilities() -> Result<WallpaperWaylandCapabilities, String> {
    let (_connection, _event_queue, state) = crate::wayland_windowshader::connect_and_bind_windowed()?;
    Ok(WallpaperWaylandCapabilities {
        compositor_version: state.compositor_version,
        layer_shell_version: state.layer_shell_version,
        output_count: state.output_count,
        targets: state.targets.into_iter().map(|target| target.info).collect(),
    })
}

#[derive(Debug, Clone)]
pub(crate) struct ActiveWallpaperShader {
    pub(crate) manager_name: String,
    pub(crate) policy_id: i64,
    pub(crate) policy_name: String,
    pub(crate) source_path: PathBuf,
    pub(crate) source: String,
    pub(crate) shader_name: String,
    pub(crate) channel_usage:
        crate::preprocess_shader::ShaderChannelUsage,
    pub(crate) shader_inputs:
        Vec<crate::isf_types::ShaderInput>,
    pub(crate) built_in_default: bool,
}


pub(crate) fn select_safe_wallpaper_shader(
    shader_manager: &mut crate::manage_shader::ShaderManager,
    wallpaper_directory: &Path,
) -> Result<ActiveWallpaperShader, String> {

    let maximum_attempts =
        shader_manager.shader_count();


    for _ in
        0..maximum_attempts
    {
        let Some(
            requested_shader
        ) =
            shader_manager.next_entry()
        else {
            break;
        };


        let requested_shader_name =
            requested_shader.name.clone();


        let resolved_source_path =
            requested_shader.source_path.clone()
                .or_else(
                    || {
                        Some(
                            wallpaper_directory.join(
                                &requested_shader_name
                            )
                        )
                    }
                );


        let managed_source_path =
            wallpaper_directory.join(
                &requested_shader_name
            );


        let is_managed_shader =
            resolved_source_path
                .as_ref()
                .is_none_or(
                    |path| {
                        paths_refer_to_same_file(
                            path,
                            &managed_source_path,
                        )
                    }
                );


        println!(
            "Evaluating wallpaper shader:"
        );


        if is_managed_shader {
            println!(
                "    {} (database-backed managed shader)",
                requested_shader_name
            );
        } else if let Some(source_path) =
            resolved_source_path.as_ref()
        {
            println!(
                "    {} (external physical shader)",
                source_path.display()
            );
        }


        let loaded_shader =
            if is_managed_shader {
                crate::load_shader::load_shader(
                    &requested_shader_name
                )
            } else if let Some(source_path) =
                resolved_source_path.as_ref()
            {
                crate::load_shader::load_shader_for_preview(
                    source_path
                )
            } else {
                crate::load_shader::load_shader(
                    &requested_shader_name
                )
            };


        match loaded_shader {
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
                        policy_id:
                            requested_shader.policy_id,
                        policy_name:
                            requested_shader.policy_name.clone(),
                        source_path:
                            resolved_source_path
                                .unwrap_or(
                                    managed_source_path
                                ),
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


fn paths_refer_to_same_file(
    left: &Path,
    right: &Path,
) -> bool {

    if left
        == right
    {
        return true;
    }


    match (
        std::fs::canonicalize(
            left
        ),
        std::fs::canonicalize(
            right
        ),
    ) {
        (
            Ok(left),
            Ok(right),
        ) => {
            left
                == right
        }

        _ => {
            false
        }
    }
}


pub(crate) fn print_active_wallpaper_shader(
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


    runtime.tray_status.set_active(
        active_shader.policy_id,
        active_shader.shader_name.clone(),
        active_shader.source_path.clone(),
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
                layer_surface:
                    Some(
                        layer_surface
                    ),
                input_region:
                    Some(
                        input_region
                    ),
                xdg_surface:
                    None,
                xdg_toplevel:
                    None,
                toplevel_decoration:
                    None,
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


        if let Some(layer_surface) =
            target.layer_surface
        {
            layer_surface.destroy();
        }


        if let Some(decoration) = target.toplevel_decoration {
            decoration.destroy();
        }

        if let Some(xdg_toplevel) =
            target.xdg_toplevel
        {
            xdg_toplevel.destroy();
        }


        if let Some(xdg_surface) =
            target.xdg_surface
        {
            xdg_surface.destroy();
        }


        target.surface.destroy();


        if let Some(input_region) =
            target.input_region
        {
            input_region.destroy();
        }
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



pub fn run_egl_windowed_surface(
    shader_manager: crate::manage_shader::ShaderManager,
    wallpaper_directory: &Path,
    shader_interval: Option<Duration>,
    runtime: &crate::define_wallpaper::WallpaperRuntime,
    running: Arc<AtomicBool>,
    control: crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
) -> Result<(), String> {
    crate::wayland_windowshader::run_egl_windowed_surface(
        shader_manager, wallpaper_directory, shader_interval, runtime, running, control
    )
}

pub(crate) fn configured_dimension(
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




// Stage 2B-4B2C: shared rendering, independently dispatched presentation.
pub(crate) trait RenderWaylandDispatch:
    wayland_client::Dispatch<wl_callback::WlCallback, crate::wayland_windowshader::FrameCallbackDispatchData>
{
    fn surfaces(&mut self) -> &mut Vec<WallpaperSurfaceState>;
    fn windowshader_state(&mut self) -> &mut crate::wayland_windowshader::WindowshaderWaylandState;
    fn removed_outputs(&mut self) -> &mut Vec<u32>;
    fn pump_events(queue: &mut wayland_client::EventQueue<Self>, state: &mut Self) -> Result<(), String>
    where Self: Sized;
}

impl RenderWaylandDispatch for WaylandState {
    fn surfaces(&mut self) -> &mut Vec<WallpaperSurfaceState> { &mut self.surface_states }
    fn windowshader_state(&mut self) -> &mut crate::wayland_windowshader::WindowshaderWaylandState { &mut self.windowshader }
    fn removed_outputs(&mut self) -> &mut Vec<u32> { &mut self.removed_output_names }
    fn pump_events(queue: &mut wayland_client::EventQueue<Self>, state: &mut Self) -> Result<(), String> {
        process_wayland_events(queue, state)
    }
}

pub(crate) fn render_egl_wallpapers<S: RenderWaylandDispatch + 'static>(
    connection: &Connection,
    event_queue: &mut wayland_client::EventQueue<S>,
    state: &mut S,
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
            .surfaces()
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


fn render_mirror_frames<S: RenderWaylandDispatch + 'static>(
    display: EglDisplay,
    context: EglContext,
    egl_targets: &mut Vec<EglWallpaperTarget>,
    event_queue: &mut wayland_client::EventQueue<S>,
    state: &mut S,
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
        texture_manager.prepare_for_policy_with_path(
            active_shader.policy_id,
            &active_shader.shader_name,
            Some(
                active_shader.source_path.as_path()
            ),
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
            .animation_speed_for_policy(
                active_shader.policy_id,
                &active_shader.shader_name,
                Some(
                    active_shader.source_path.as_path()
                ),
                None,
            );


    let mut rendered_fps =
        runtime.fps_policy.rendered_fps_for_policy(
            active_shader.policy_id,
            &active_shader.shader_name,
            Some(
                active_shader.source_path.as_path()
            ),
            None,
        );


    let mut frame_duration =
        frame_duration_for_fps(
            rendered_fps
        );


    let mut next_frame_deadline =
        Instant::now();


    let mut notification_state =
        crate::notify_wallpaper::WallpaperNotificationState::new();


    notify_active_wallpaper(
        &mut notification_state,
        true,
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


    // FPS Warning/CRITICAL must measure GPU rendering work rather than CPU
    // wall-clock time.  This is especially important for Windowshader: a
    // compositor may throttle or defer a non-visible xdg_toplevel while its
    // workspace is inactive.  CPU elapsed time across that lifecycle event is
    // not evidence that the shader itself is slow.
    let mut performance_queries =
        [0_u32; 2];

    unsafe {
        gl::GenQueries(
            2,
            performance_queries.as_mut_ptr(),
        );
    }


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


        let postprocess_profile =
            runtime.postprocess_policy.profile_for_policy(
                active_shader.policy_id,
                &active_shader.shader_name,
                Some(
                    active_shader.source_path.as_path()
                ),
            );

        let pipeline =
            crate::postprocess_shader::PostprocessPipeline::new(
                width,
                height,
                postprocess_profile,
            )?;


        postprocess_pipelines.insert(
            target.info.registry_name,
            pipeline,
        );
    }


    // Per-output diagnostic overlays; resources are owned by the active EGL context.
    let mut renderer_status_panels:
        HashMap<u32, crate::display_overlay::RendererStatusPanel> = HashMap::new();

    let mut lyrics_overlays:
        HashMap<u32, crate::display_overlay::OpenGlOverlay> =
            HashMap::new();

    let mut lyrics_overlay_states:
        HashMap<u32, (crate::manage_lyrics::LyricsState, u32, u32)> =
            HashMap::new();


    let initial_postprocess_profile =
        runtime.postprocess_policy
            .profile_for_policy(
                active_shader.policy_id,
                &active_shader.shader_name,
                Some(
                    active_shader.source_path.as_path()
                ),
            );


    let audio_demand = crate::audio_backend::register_audio_demand();

    let mut audio_required =
        matches!(
            initial_postprocess_profile.bloom.name(),
            "audio" | "spectral" | "loudness"
        ) || initial_postprocess_profile.audio_motion.is_enabled();


    let mut audio_motion_effect =
        initial_postprocess_profile.audio_motion;


    let mut audio_motion_state =
        crate::render_audio_motion::AudioMotionState::default();


    let mut audio_motion_lyrics_manager =
        if matches!(
            audio_motion_effect,
            crate::render_audio_motion::AudioMotionEffect::WooferFromHell
        ) {
            crate::manage_lyrics::LyricsManager::start().ok()
        } else {
            None
        };


    let mut last_audio_motion_update =
        Instant::now();


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


    // The initial wallpaper has now loaded, compiled, prepared textures,
    // created post-processing pipelines, and resolved its uniforms. Persist
    // Ordered position only after reaching this accepted render state.
    shader_manager.record_rendered_policy(
        current_shader.policy_id
    );


    let mut frame_times =
        crate::fps_monitor::FrameTimeWindow::new();


    let mut fps_warning_state =
        crate::fps_monitor::FpsWarningState::Normal;


    let mut paused =
        false;


    let mut pause_started: Option<Instant> =
        None;


    let mut windowed_inactive_started: Option<Instant> =
        None;

    // Track the most recent pause interval so an outstanding compositor
    // callback cannot charge the same inactive time to both clocks twice.
    let mut last_windowed_pause_interval: Option<(Instant, Instant)> = None;

    // Diagnostics only: preserve compositor-driven pacing. In particular,
    // never synthesize a wl_surface.frame completion after a screensaver pause.
    let mut callback_wait_reported = false;
    let mut resume_presentation_pending = false;
    let mut resume_wait_reported = false;
    let mut last_successful_swap = Instant::now();
    let mut last_callback_stall_report: Option<Instant> = None;
    // Diagnostic-only windowshader presentation watchdog. It never changes
    // pacing, requests callbacks, or touches the EGL/Wayland surface.
    let mut presentation_watchdog_last_report: Option<Instant> = None;
    let mut presentation_watchdog_loop_ticks: u64 = 0;
    let mut presentation_watchdog_last_ticks: u64 = 0;
    let mut presentation_watchdog_swaps: u64 = 0;
    let mut presentation_watchdog_last_swaps: u64 = 0;
    // Windowed-only, bounded diagnostics; no changes to rendering decisions.
    let mut windowshader_summary_at = Instant::now();
    let mut windowshader_rotation_attempts: u64 = 0;
    let mut windowshader_rotation_successes: u64 = 0;
    let mut windowshader_rotation_failures: u64 = 0;
    let mut windowshader_rotation_block_reported = false;
    let mut windowshader_last_configure_count = state.windowshader_state().windowshader_configure_count;
    // Diagnostic accounting only: distinguish legitimate pause exclusions from
    // repeated callback-wait exclusions that may prevent rotation progress.
    let mut rotation_clock_pause_shifts: u64 = 0;
    let mut rotation_clock_callback_shifts: u64 = 0;
    let mut rotation_clock_callback_shift_ms: u128 = 0;
    let mut rotation_clock_callback_short_shifts: u64 = 0;
    let mut rotation_clock_last_shift_reason: &'static str = "none";
    let mut rotation_clock_last_shift_ms: u128 = 0;


    let status_pipeline = if runtime.display_format
        == crate::manage_configuration::WallpaperDisplayFormat::Windowed
    {
        crate::renderer_status::Pipeline::Windowshader
    } else {
        crate::renderer_status::Pipeline::Wallpaper
    };
    let status_enabled = crate::renderer_status::enabled();
    if status_enabled {
        crate::renderer_status::publish(status_pipeline, crate::renderer_status::State::Starting);
    }
    let mut status_heartbeat = Instant::now() - Duration::from_secs(2);

    let result =
        'render_loop: loop {

            control.diagnostic_checkpoint(1);
            S::pump_events(event_queue, state)?;


            control.diagnostic_checkpoint(2);
            if runtime.display_format
                == crate::manage_configuration::WallpaperDisplayFormat::Windowed
            {
                presentation_watchdog_loop_ticks = presentation_watchdog_loop_ticks.wrapping_add(1);
                let swap_age = last_successful_swap.elapsed();
                if swap_age >= Duration::from_millis(1500)
                    && presentation_watchdog_last_report.is_none_or(
                        |last| last.elapsed() >= Duration::from_secs(10)
                    )
                {
                    presentation_watchdog_last_report = Some(Instant::now());
                    let ready = native_targets.iter().all(|target| {
                        state.windowshader_state().frame_ready.get(&target.info.registry_name)
                            .copied().unwrap_or(true)
                    });
                    let pending = state.windowshader_state().windowshader_pending_callbacks.get(&0);
                    let pending_description = pending.map_or_else(
                        || "none".to_string(),
                        |record| format!(
                            "seq={} age_ms={} swap_completed={} flushed={}",
                            record.sequence,
                            record.requested_at.elapsed().as_millis(),
                            record.swap_completed,
                            record.flushed,
                        ),
                    );
                    let configure_pending = state.surfaces().iter().any(|surface| {
                        surface.registry_name == 0 && surface.configured.is_some()
                    });
                    let (activated, activation_established) = {
                        let ws = state.windowshader_state();
                        (ws.windowed_activated, ws.windowed_activation_established)
                    };
                    crate::logger::warning(
                        &crate::locate_paths::runtime_log_path(),
                        &format!(
                            "[WINDOWSHADER_PRESENTATION_DIAG] no_swap_ms={} frame_ready={} pending_callback=[{}] paused={} pause_requested={} post_pause={} activated={} activation_established={} configure_pending={} loop_ticks_delta={} swaps_delta={} next_deadline_in_ms={}",
                            swap_age.as_millis(),
                            ready,
                            pending_description,
                            paused,
                            control.pause_requested(),
                            resume_presentation_pending,
                            activated,
                            activation_established,
                            configure_pending,
                            presentation_watchdog_loop_ticks.wrapping_sub(presentation_watchdog_last_ticks),
                            presentation_watchdog_swaps.wrapping_sub(presentation_watchdog_last_swaps),
                            next_frame_deadline.saturating_duration_since(Instant::now()).as_millis(),
                        ),
                    );
                    presentation_watchdog_last_ticks = presentation_watchdog_loop_ticks;
                    presentation_watchdog_last_swaps = presentation_watchdog_swaps;
                }
            }
            if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed
                && windowshader_summary_at.elapsed() >= Duration::from_secs(10)
            {
                let (ready, pending_sequence, pending_age_ms, activated, configure_delta) = {
                    let ws = state.windowshader_state();
                    let pending = ws.windowshader_pending_callbacks.get(&0);
                    (
                        ws.frame_ready.get(&0).copied().unwrap_or(true),
                        pending.map(|p| p.sequence),
                        pending.map(|p| p.requested_at.elapsed().as_millis()),
                        ws.windowed_activated,
                        ws.windowshader_configure_count.wrapping_sub(windowshader_last_configure_count),
                    )
                };
                let rotation_due = shader_interval.is_some_and(|interval| last_shader_switch.elapsed() >= interval);
                crate::logger::warning(
                    &crate::locate_paths::runtime_log_path(),
                    &format!("[WINDOWSHADER_LIFECYCLE_DIAG] shader='{}' policy_id={} program={} frame={} swaps_total={} swap_age_ms={} frame_ready={} callback_seq={:?} callback_age_ms={:?} paused={} pause_requested={} activated={} rotation_due={} rotation_elapsed_ms={} rotation_interval_ms={:?} rotation_attempts={} rotation_successes={} rotation_failures={} configure_delta={} loop_ticks={}",
                        current_shader.shader_name, current_shader.policy_id, program, frame,
                        presentation_watchdog_swaps, last_successful_swap.elapsed().as_millis(),
                        ready, pending_sequence, pending_age_ms,
                        paused, control.pause_requested(), activated,
                        rotation_due, last_shader_switch.elapsed().as_millis(),
                        shader_interval.map(|interval| interval.as_millis()),
                        windowshader_rotation_attempts, windowshader_rotation_successes,
                        windowshader_rotation_failures,
                        configure_delta,
                        presentation_watchdog_loop_ticks),
                );
                crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                    &format!("[WINDOWSHADER_CLOCK_DIAG] summary elapsed_ms={} interval_ms={:?} due={} pause_shifts={} callback_shifts={} callback_shifts_under_2s={} callback_shift_total_ms={} last_reason={} last_shift_ms={} paused={} frame_ready={} activated={} swaps_total={}",
                        last_shader_switch.elapsed().as_millis(), shader_interval.map(|v| v.as_millis()),
                        rotation_due, rotation_clock_pause_shifts, rotation_clock_callback_shifts,
                        rotation_clock_callback_short_shifts, rotation_clock_callback_shift_ms,
                        rotation_clock_last_shift_reason, rotation_clock_last_shift_ms,
                        paused, ready, state.windowshader_state().windowed_activated, presentation_watchdog_swaps));
                windowshader_last_configure_count = state.windowshader_state().windowshader_configure_count;
                windowshader_summary_at = Instant::now();
            }
            remove_disconnected_targets_from_names(
                display,
                context,
                state.removed_outputs(),
                egl_targets,
                native_targets,
                &mut postprocess_pipelines,
            )?;
            // Prune records for disconnected outputs after EGL cleanup.
            state.surfaces().retain(|surface| native_targets.iter().any(|target| target.info.registry_name == surface.registry_name));

            if native_targets
                .is_empty()
            {
                println!(
                    "All wallpaper outputs have been disconnected."
                );


                break Ok(());
            }


            apply_pending_target_resizes_from_states(
                state.surfaces(),
                native_targets,
            )?;


            let compositor_closed = state
                .surfaces()
                .iter()
                .any(|surface_state| surface_state.closed);

            if compositor_closed {
                // Only Windowshader's xdg_toplevel Close means Quit.
                // A wallpaper layer closing must not terminate Screenshaver.
                if native_targets.iter().any(|target| target.xdg_toplevel.is_some()) {
                    running.store(false, Ordering::SeqCst);
                }
                break Ok(());
            }
            if !running.load(Ordering::SeqCst) {
                break Ok(());
            }


            if let Some(reload) =
                control.take_policy_reload()
            {
                let mut replacement_texture_manager =
                    crate::manage_textures::TextureManager::new(
                        reload.texture_policy.clone()
                    );

                match replacement_texture_manager.prepare_for_policy_with_path(
                    current_shader.policy_id,
                    &current_shader.shader_name,
                    Some(
                        current_shader.source_path.as_path()
                    ),
                    current_shader.channel_usage,
                ) {
                    Ok(()) => {
                        let replacement_profile =
                            reload.postprocess_policy
                                .profile_for_policy(
                                    current_shader.policy_id,
                                    &current_shader.shader_name,
                                    Some(
                                        current_shader.source_path.as_path()
                                    ),
                                );

                        let mut profile_error =
                            None;

                        for pipeline in
                            postprocess_pipelines.values_mut()
                        {
                            if let Err(error) =
                                pipeline.set_profile(
                                    replacement_profile
                                )
                            {
                                profile_error =
                                    Some(error);
                                break;
                            }
                        }

                        if let Some(error) =
                            profile_error
                        {
                            replacement_texture_manager.delete_all();
                            println!(
                                "Unable to reload Wayland active wallpaper policy; keeping the previous settings: {}",
                                error,
                            );
                        } else {
                            replacement_texture_manager.configure_program(
                                program
                            );

                            texture_manager.delete_all();
                            texture_manager =
                                replacement_texture_manager;

                            audio_required =
                                matches!(
                                    replacement_profile.bloom.name(),
                                    "audio" | "spectral" | "loudness"
                                ) || replacement_profile.audio_motion.is_enabled();

                            audio_motion_effect =
                                replacement_profile.audio_motion;

                            if matches!(
                                audio_motion_effect,
                                crate::render_audio_motion::AudioMotionEffect::WooferFromHell
                            ) && audio_motion_lyrics_manager.is_none()
                            {
                                audio_motion_lyrics_manager =
                                    crate::manage_lyrics::LyricsManager::start().ok();
                            }

                            if !matches!(
                                audio_motion_effect,
                                crate::render_audio_motion::AudioMotionEffect::WooferFromHell
                            ) {
                                audio_motion_lyrics_manager = None;
                            }

                            if !audio_motion_effect.is_enabled() {
                                audio_motion_lyrics_manager = None;
                                audio_motion_state.reset();

                                for pipeline in
                                    postprocess_pipelines.values_mut()
                                {
                                    pipeline.set_audio_motion_scale(
                                        1.0
                                    );
                                }
                            }

                            last_audio_motion_update =
                                Instant::now();

                            animation_speed =
                                reload.animation_speed_policy
                                    .animation_speed_for_policy(
                                        current_shader.policy_id,
                                        &current_shader.shader_name,
                                        Some(
                                            current_shader.source_path.as_path()
                                        ),
                                        None,
                                    );

                            rendered_fps =
                                reload.fps_policy
                                    .rendered_fps_for_policy(
                                        current_shader.policy_id,
                                        &current_shader.shader_name,
                                        Some(
                                            current_shader.source_path.as_path()
                                        ),
                                        None,
                                    );

                            frame_duration =
                                frame_duration_for_fps(
                                    rendered_fps
                                );

                            next_frame_deadline =
                                Instant::now();

                            frame_times.clear();
                            fps_warning_state =
                                crate::fps_monitor::FpsWarningState::Normal;

                            notify_active_wallpaper(
                                &mut notification_state,
                                false,
                                runtime.notifications,
                                &current_shader,
                                &texture_manager,
                                rendered_fps,
                                animation_speed,
                                fps_warning_state,
                            );

                            println!(
                                "Wayland active wallpaper policy reloaded."
                            );
                        }
                    }

                    Err(error) => {
                        println!(
                            "Unable to reload Wayland active wallpaper texture policy; keeping the previous settings: {}",
                            error,
                        );
                    }
                }
            }

            control.diagnostic_checkpoint(3);
            if control.pause_requested() {
                if !paused {
                    crate::logger::warning(
                        &crate::locate_paths::runtime_log_path(),
                        "[AMBIENT_HANDOFF_DIAG] Wayland renderer detected pause request",
                    );
                    control.acknowledge_pause_detected();
                    resume_presentation_pending = true;
                    resume_wait_reported = false;
                    if runtime.display_format
                        == crate::manage_configuration::WallpaperDisplayFormat::Windowed
                    {
                        state.windowshader_state().windowshader_trace_surface_configure = true;
                        state.windowshader_state().windowshader_trace_toplevel_configure = true;
                    }
                    control.diagnostic_checkpoint(8);
                    if runtime.display_format
                        == crate::manage_configuration::WallpaperDisplayFormat::Windowed
                    {
                        if let Some(pending) = state.windowshader_state().windowshader_pending_callbacks.get(&0) {
                            crate::logger::warning(
                                &crate::locate_paths::runtime_log_path(),
                                &format!("[WINDOWSHADER_CALLBACK_DIAG] Pause entered: seq={} age_ms={} swap_completed={} flushed={}",
                                    pending.sequence, pending.requested_at.elapsed().as_millis(), pending.swap_completed, pending.flushed),
                            );
                        }
                        let outstanding = native_targets.iter()
                            .filter(|target| !state.windowshader_state().frame_ready.get(&target.info.registry_name)
                                .copied().unwrap_or(true))
                            .count();
                        crate::logger::warning(
                            &crate::locate_paths::runtime_log_path(),
                            &format!("[WINDOWSHADER_RESUME_DIAG] Pause entered; outstanding_frame_callbacks={outstanding}"),
                        );
                    }
                    paused =
                        true;


                    pause_started =
                        Some(
                            Instant::now()
                        );
                    control.acknowledge_paused();
                    if status_enabled {
                        crate::renderer_status::publish(status_pipeline, crate::renderer_status::State::Paused);
                    }


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
                    let pause_ended_at = Instant::now();
                    let paused_duration = pause_ended_at.duration_since(paused_at);
                    if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed {
                        last_windowed_pause_interval = Some((paused_at, pause_ended_at));
                    }


                    start_time +=
                        paused_duration;


                    if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed {
                        let before_ms = last_shader_switch.elapsed().as_millis();
                        rotation_clock_pause_shifts = rotation_clock_pause_shifts.wrapping_add(1);
                        rotation_clock_last_shift_reason = "screensaver_pause";
                        rotation_clock_last_shift_ms = paused_duration.as_millis();
                        crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                            &format!("[WINDOWSHADER_CLOCK_DIAG] reason=screensaver_pause shift_ms={} elapsed_before_ms={} elapsed_after_expected_ms={} pause_shift_count={}",
                                paused_duration.as_millis(), before_ms,
                                before_ms.saturating_sub(paused_duration.as_millis()), rotation_clock_pause_shifts));
                    }
                    last_shader_switch +=
                        paused_duration;
                }


                next_frame_deadline =
                    Instant::now();


                frame_times.clear();
                // The pause has ended even if the compositor has not yet
                // allowed another presentation. Keep the separate pending
                // flag until a successful EGL swap acknowledges resumption.
                paused = false;
                if resume_presentation_pending {
                    if let Some(pending) = state.windowshader_state().windowshader_pending_callbacks.get(&0) {
                        crate::logger::warning(
                            &crate::locate_paths::runtime_log_path(),
                            &format!("[WINDOWSHADER_CALLBACK_DIAG] Pause released: seq={} age_ms={} swap_completed={} flushed={}",
                                pending.sequence, pending.requested_at.elapsed().as_millis(), pending.swap_completed, pending.flushed),
                        );
                    }
                    crate::logger::warning(
                        &crate::locate_paths::runtime_log_path(),
                        "[WINDOWSHADER_RESUME_DIAG] Pause released; awaiting compositor-paced presentation",
                    );
                }
            }


            control.diagnostic_checkpoint(4);
            // Do not use xdg_toplevel::State::Activated as a visibility
            // signal.  Activated represents compositor activation/focus, so
            // Mango legitimately removes it when the pointer/keyboard focus
            // leaves a still-visible Windowshader.  Instead, pace Windowed
            // presentation with wl_surface.frame callbacks.  A visible,
            // unfocused Windowshader continues receiving callbacks, while a
            // surface on a non-visible workspace can stop receiving them.
            // The render worker therefore remains in this event/control loop
            // instead of entering a compositor-throttled eglSwapBuffers().
            let windowed_frame_ready =
                if runtime.display_format
                    == crate::manage_configuration::WallpaperDisplayFormat::Windowed
                {
                    native_targets
                        .iter()
                        .all(
                            |target| {
                                state
                                    .windowshader_state().frame_ready
                                    .get(
                                        &target.info.registry_name
                                    )
                                    .copied()
                                    .unwrap_or(true)
                            }
                        )
                } else {
                    true
                };


            if !windowed_frame_ready {
                if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed
                    && !windowshader_rotation_block_reported
                    && shader_interval.is_some_and(|interval| last_shader_switch.elapsed() >= interval)
                {
                    windowshader_rotation_block_reported = true;
                    crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                        "[WINDOWSHADER_ROTATION_DIAG] Rotation deadline reached while awaiting compositor frame callback; no rotation attempted");
                }
                if windowed_inactive_started.is_none() {
                    windowed_inactive_started =
                        Some(
                            Instant::now()
                        );
                }


                if windowed_inactive_started.is_some_and(|since| since.elapsed() >= Duration::from_millis(1500))
                    && !callback_wait_reported
                {
                    callback_wait_reported = true;
                    let outstanding: Vec<u32> = native_targets.iter()
                        .filter(|target| !state.windowshader_state().frame_ready.get(&target.info.registry_name)
                            .copied().unwrap_or(true))
                        .map(|target| target.info.registry_name).collect();
                    crate::logger::warning(
                        &crate::locate_paths::runtime_log_path(),
                        &format!("[WINDOWSHADER_RESUME_DIAG] Frame callback outstanding >1500ms; targets={outstanding:?}, post_pause={resume_presentation_pending}"),
                    );
                }
                // A one-shot warning is useful at 1500ms; thereafter emit
                // a bounded heartbeat every 10s so a persistent stall can be
                // distinguished from a recovered callback without log spam.
                if windowed_inactive_started.is_some_and(|since| since.elapsed() >= Duration::from_secs(10))
                    && last_callback_stall_report.is_none_or(|last| last.elapsed() >= Duration::from_secs(10))
                {
                    last_callback_stall_report = Some(Instant::now());
                    let outstanding: Vec<u32> = native_targets.iter()
                        .filter(|target| !state.windowshader_state().frame_ready.get(&target.info.registry_name)
                            .copied().unwrap_or(true))
                        .map(|target| target.info.registry_name).collect();
                    let wait_ms = windowed_inactive_started.map_or(0, |since| since.elapsed().as_millis());
                    let callback_details: Vec<String> = outstanding.iter().map(|target| {
                        match state.windowshader_state().windowshader_pending_callbacks.get(target) {
                            Some(pending) => format!(
                                "target={} seq={} age_ms={} swap_completed={} flushed={}",
                                target, pending.sequence, pending.requested_at.elapsed().as_millis(),
                                pending.swap_completed, pending.flushed,
                            ),
                            None => format!("target={} missing_pending_record", target),
                        }
                    }).collect();
                    crate::logger::warning(
                        &crate::locate_paths::runtime_log_path(),
                        &format!("[WINDOWSHADER_CALLBACK_DIAG] Outstanding callback details: {}", callback_details.join("; ")),
                    );
                    crate::logger::warning(
                        &crate::locate_paths::runtime_log_path(),
                        &format!("[WINDOWSHADER_RESUME_DIAG] Callback stall persists: targets={outstanding:?}, wait_ms={wait_ms}, last_swap_age_ms={}, post_pause={resume_presentation_pending}; preserving compositor pacing", last_successful_swap.elapsed().as_millis()),
                    );
                }
                if resume_presentation_pending && !resume_wait_reported {
                    resume_wait_reported = true;
                    crate::logger::warning(
                        &crate::locate_paths::runtime_log_path(),
                        "[WINDOWSHADER_RESUME_DIAG] Post-pause presentation blocked by outstanding frame callback",
                    );
                }
                thread::sleep(Duration::from_millis(10));
                continue 'render_loop;
            }

            windowshader_rotation_block_reported = false;
            if callback_wait_reported {
                crate::logger::warning(
                    &crate::locate_paths::runtime_log_path(),
                    "[WINDOWSHADER_RESUME_DIAG] Outstanding frame callback delivered; presentation eligible",
                );
                callback_wait_reported = false;
            }
            last_callback_stall_report = None;

            if let Some(inactive_at) =
                windowed_inactive_started.take()
            {
                let callback_ended_at = Instant::now();
                let callback_wait = callback_ended_at.duration_since(inactive_at);
                // A callback may have been outstanding before the screensaver
                // paused and remain outstanding after it resumed. The pause
                // duration was already excluded above; exclude only the
                // non-overlapping part of this callback wait.
                let pause_overlap = last_windowed_pause_interval
                    .map(|(pause_start, pause_end)| {
                        let overlap_start = inactive_at.max(pause_start);
                        let overlap_end = callback_ended_at.min(pause_end);
                        overlap_end.saturating_duration_since(overlap_start)
                    })
                    .unwrap_or_default();
                let effective_callback_wait = callback_wait.saturating_sub(pause_overlap);
                // Keep the clock anchor at or before now, even if an earlier
                // adjustment has already consumed part of the interval.
                let rotation_shift = effective_callback_wait.min(last_shader_switch.elapsed());
                let animation_shift = effective_callback_wait.min(start_time.elapsed());

                // Ordinary frame-callback pacing is only a frame or two and
                // must remain part of shader animation time.  A long callback
                // silence indicates that presentation was suppressed (for
                // example because the workspace was not visible); exclude
                // only that long interval from shader/rotation timing.
                if callback_wait
                    >= Duration::from_millis(250)
                {
                    start_time +=
                        animation_shift;


                    if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed {
                        let before_ms = last_shader_switch.elapsed().as_millis();
                        rotation_clock_callback_shifts = rotation_clock_callback_shifts.wrapping_add(1);
                        rotation_clock_callback_shift_ms += rotation_shift.as_millis();
                        if callback_wait < Duration::from_secs(2) {
                            rotation_clock_callback_short_shifts = rotation_clock_callback_short_shifts.wrapping_add(1);
                        }
                        rotation_clock_last_shift_reason = "callback_wait";
                        rotation_clock_last_shift_ms = rotation_shift.as_millis();
                        // First few occurrences and periodic aggregates avoid per-frame log spam.
                        if rotation_clock_callback_shifts <= 5 || rotation_clock_callback_shifts % 100 == 0 || callback_wait >= Duration::from_secs(2) {
                            crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                                &format!("[WINDOWSHADER_CLOCK_DIAG] reason=callback_wait raw_ms={} pause_overlap_ms={} shift_ms={} elapsed_before_ms={} elapsed_after_expected_ms={} callback_shifts={} short_under_2s={} cumulative_shift_ms={} frame_ready={} paused={} activated={}",
                                    callback_wait.as_millis(), pause_overlap.as_millis(), rotation_shift.as_millis(), before_ms,
                                    before_ms.saturating_sub(rotation_shift.as_millis()),
                                    rotation_clock_callback_shifts, rotation_clock_callback_short_shifts,
                                    rotation_clock_callback_shift_ms, windowed_frame_ready, paused, state.windowshader_state().windowed_activated));
                        }
                    }
                    last_shader_switch +=
                        rotation_shift;
                    // The overlapping pause interval has been accounted for.
                    last_windowed_pause_interval = None;


                    next_frame_deadline =
                        Instant::now();


                    frame_times.clear();


                    fps_warning_state =
                        crate::fps_monitor::FpsWarningState::Normal;
                }
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
                if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed {
                    windowshader_rotation_attempts = windowshader_rotation_attempts.wrapping_add(1);
                    crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                        &format!("[WINDOWSHADER_ROTATION_DIAG] Rotation attempt={} previous_shader='{}' policy_id={} elapsed_ms={} interval_ms={:?} frame_ready={} program={}",
                            windowshader_rotation_attempts, current_shader.shader_name,
                            current_shader.policy_id, last_shader_switch.elapsed().as_millis(),
                            shader_interval.map(|interval| interval.as_millis()), windowed_frame_ready, program));
                }
                if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed {
                    rotation_clock_last_shift_reason = "rotation_attempt_reset";
                    rotation_clock_last_shift_ms = last_shader_switch.elapsed().as_millis();
                    crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                        &format!("[WINDOWSHADER_CLOCK_DIAG] reason=rotation_attempt_reset elapsed_before_ms={} interval_ms={:?} attempts={}",
                            rotation_clock_last_shift_ms, shader_interval.map(|v| v.as_millis()), windowshader_rotation_attempts));
                }
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
                                    next_texture_manager.prepare_for_policy_with_path(
                                        next_shader.policy_id,
                                        &next_shader.shader_name,
                                        Some(
                                            next_shader.source_path.as_path()
                                        ),
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
                                        .animation_speed_for_policy(
                                            next_shader.policy_id,
                                            &next_shader.shader_name,
                                            Some(
                                                next_shader.source_path.as_path()
                                            ),
                                            None,
                                        );


                                rendered_fps =
                                    runtime.fps_policy.rendered_fps_for_policy(
                                        next_shader.policy_id,
                                        &next_shader.shader_name,
                                        Some(
                                            next_shader.source_path.as_path()
                                        ),
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
                                if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed {
                                    windowshader_rotation_successes = windowshader_rotation_successes.wrapping_add(1);
                                    crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                                        &format!("[WINDOWSHADER_ROTATION_DIAG] Rotation succeeded shader='{}' policy_id={} program={} successes={}",
                                            current_shader.shader_name, current_shader.policy_id, program, windowshader_rotation_successes));
                                }


                                let postprocess_profile =
                                    runtime.postprocess_policy
                                        .profile_for_policy(
                                            current_shader.policy_id,
                                            &current_shader.shader_name,
                                            Some(
                                                current_shader.source_path.as_path()
                                            ),
                                        );


                                audio_required =
                                    matches!(
                                        postprocess_profile.bloom.name(),
                                        "audio" | "spectral" | "loudness"
                                    ) || postprocess_profile.audio_motion.is_enabled();


                                audio_motion_effect =
                                    postprocess_profile.audio_motion;


                                if matches!(
                                    audio_motion_effect,
                                    crate::render_audio_motion::AudioMotionEffect::WooferFromHell
                                ) && audio_motion_lyrics_manager.is_none()
                                {
                                    audio_motion_lyrics_manager =
                                        crate::manage_lyrics::LyricsManager::start().ok();
                                }

                                if !matches!(
                                    audio_motion_effect,
                                    crate::render_audio_motion::AudioMotionEffect::WooferFromHell
                                ) {
                                    audio_motion_lyrics_manager = None;
                                }


                                if !audio_motion_effect.is_enabled() {
                                    audio_motion_lyrics_manager =
                                        None;

                                    audio_motion_state.reset();
                                }


                                last_audio_motion_update =
                                    Instant::now();


                                for pipeline in
                                    postprocess_pipelines.values_mut()
                                {
                                    pipeline.set_profile(
                                        postprocess_profile
                                    )?;
                                }


                                runtime.tray_status.set_active(
                                    current_shader.policy_id,
                                    current_shader.shader_name.clone(),
                                    current_shader.source_path.clone(),
                                );


                                shader_manager.record_rendered_policy(
                                    current_shader.policy_id
                                );


                                frame_times.clear();


                                fps_warning_state =
                                    crate::fps_monitor::FpsWarningState::Normal;


                                notify_active_wallpaper(
                                    &mut notification_state,
                                    true,
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
                                if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed {
                                    windowshader_rotation_failures = windowshader_rotation_failures.wrapping_add(1);
                                    crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                                        &format!("[WINDOWSHADER_ROTATION_DIAG] Compilation failed shader='{}' error={}", next_shader.shader_name, error));
                                }
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
                        if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed {
                            windowshader_rotation_failures = windowshader_rotation_failures.wrapping_add(1);
                            crate::logger::warning(&crate::locate_paths::runtime_log_path(),
                                &format!("[WINDOWSHADER_ROTATION_DIAG] Selection failed: {}", error));
                        }
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


            unsafe {
                gl::QueryCounter(
                    performance_queries[0],
                    gl::TIMESTAMP,
                );
            }


            audio_demand.set_required(
                audio_required
            );


            let current_audio_bands =
                runtime.audio_bands
                    .as_ref()
                    .and_then(
                        |shared| {
                            shared
                                .read()
                                .ok()
                                .map(
                                    |bands| *bands
                                )
                        }
                    )
                    .unwrap_or_default();


            let now =
                Instant::now();

            let audio_motion_frame_seconds =
                now.duration_since(
                    last_audio_motion_update
                )
                .as_secs_f32();

            last_audio_motion_update =
                now;

            let mut audio_motion_scale =
                1.0_f32;

            let mut audio_motion_fft_trace =
                [0.0_f32; crate::analyze_audio::AUDIO_MOTION_TRACE_CHANNELS];

            match audio_motion_effect {
                crate::render_audio_motion::AudioMotionEffect::WooferFromHell => {
                    let spectrum =
                        crate::analyze_audio::shared_audio_motion_spectrum()
                            .read()
                            .ok()
                            .map(
                                |value| *value
                            )
                            .unwrap_or_default();

                    let vocal_timing =
                        crate::manage_lyrics::shared_audio_motion_vocal_timing_state()
                            .lock()
                            .ok()
                            .map(
                                |value| value.clone()
                            )
                            .unwrap_or_default();

                    audio_motion_scale =
                        audio_motion_state.update(
                            spectrum,
                            audio_motion_frame_seconds,
                            &vocal_timing,
                        );
                }

                crate::render_audio_motion::AudioMotionEffect::FftMirrorWarp => {
                    let spectrum =
                        crate::analyze_audio::shared_audio_motion_fft_trace()
                            .read()
                            .ok()
                            .map(
                                |value| *value
                            )
                            .unwrap_or_default();

                    audio_motion_fft_trace =
                        audio_motion_state.update_fft_mirror_warp(
                            spectrum,
                            audio_motion_frame_seconds,
                        );
                }

                crate::render_audio_motion::AudioMotionEffect::PolarPropeller => {
                    let spectrum =
                        crate::analyze_audio::shared_audio_motion_fft_trace()
                            .read()
                            .ok()
                            .map(
                                |value| *value
                            )
                            .unwrap_or_default();

                    audio_motion_fft_trace =
                        audio_motion_state.update_polar_propeller(
                            spectrum,
                            audio_motion_frame_seconds,
                        );
                }

                crate::render_audio_motion::AudioMotionEffect::Off => {
                    audio_motion_state.reset();
                }
            }


            let frame_queue_handle =
                event_queue.handle();


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


                postprocess.set_audio_bands(
                    current_audio_bands
                );


                postprocess.set_audio_motion_scale(
                    audio_motion_scale
                );

                postprocess.set_audio_motion_fft_trace(
                    audio_motion_fft_trace
                );
                postprocess.set_audio_motion_rotation_turns(
                    audio_motion_state.polar_rotation_turns()
                );


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


                let (
                    scene_width,
                    scene_height,
                ) =
                    postprocess.scene_dimensions();


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
                        scene_width as f32,
                        scene_height as f32,
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

                // Diagnostic only: compose after post-processing, before EGL presentation.
                let status_text = crate::renderer_status::enabled()
                    .then(crate::renderer_status::panel_text);
                renderer_status_panels
                    .entry(native_target.info.registry_name)
                    .or_insert_with(crate::display_overlay::RendererStatusPanel::new)
                    .draw(target_width, target_height, status_text.as_deref());

                if runtime.display_format
                    == crate::manage_configuration::WallpaperDisplayFormat::Windowed
                {
                    let registry_name =
                        native_target.info.registry_name;

                    let lyrics_state =
                        runtime.lyrics_state
                            .as_ref()
                            .and_then(
                                |shared| {
                                    shared
                                        .lock()
                                        .ok()
                                        .map(
                                            |state| state.clone()
                                        )
                                }
                            )
                            .unwrap_or_default();

                    if lyrics_state.current.is_none() {
                        lyrics_overlays.remove(
                            &registry_name
                        );
                        lyrics_overlay_states.remove(
                            &registry_name
                        );
                    } else {
                        let needs_rebuild =
                            lyrics_overlay_states
                                .get(
                                    &registry_name
                                )
                                .is_none_or(
                                    |(state, width, height)| {
                                        state != &lyrics_state
                                            || *width != target_width
                                            || *height != target_height
                                    }
                                );

                        if needs_rebuild {
                            match crate::construct_text_overlay::construct_lyrics_panel(
                                lyrics_state.previous.as_deref(),
                                lyrics_state.current.as_deref(),
                                lyrics_state.next.as_deref(),
                                target_width,
                                target_height,
                            )
                            .and_then(
                                |constructed| {
                                    crate::display_overlay::OpenGlOverlay::new_from_constructed(
                                        constructed,
                                        crate::parse_subtitle_placement::parse(None).placement,
                                    )
                                }
                            ) {
                                Ok(overlay) => {
                                    lyrics_overlays.insert(
                                        registry_name,
                                        overlay,
                                    );
                                    lyrics_overlay_states.insert(
                                        registry_name,
                                        (
                                            lyrics_state,
                                            target_width,
                                            target_height,
                                        ),
                                    );
                                }

                                Err(error) => {
                                    crate::logger::warning(
                                        &crate::locate_paths::runtime_log_path(),
                                        &format!(
                                            "[LYRICS] Unable to build Wayland Windowpaper lyrics overlay: {}",
                                            error,
                                        ),
                                    );
                                    lyrics_overlays.remove(
                                        &registry_name
                                    );
                                    lyrics_overlay_states.remove(
                                        &registry_name
                                    );
                                }
                            }
                        }

                        if let Some(overlay) =
                            lyrics_overlays.get(
                                &registry_name
                            )
                        {
                            overlay.display_bottom_edge(
                                target_width,
                                target_height,
                            );
                        }
                    }
                }


                if runtime.display_format
                    == crate::manage_configuration::WallpaperDisplayFormat::Windowed
                {
                    state.windowshader_state().windowshader_next_callback_sequence += 1;
                    let sequence = state.windowshader_state().windowshader_next_callback_sequence;
                    let registry_name = native_target.info.registry_name;
                    native_target.surface.frame(
                        &frame_queue_handle,
                        crate::wayland_windowshader::FrameCallbackDispatchData {
                            registry_name,
                            sequence,
                        },
                    );
                    state.windowshader_state().windowshader_pending_callbacks.insert(registry_name, crate::wayland_windowshader::WindowshaderCallbackDiagnostic {
                        sequence,
                        requested_at: Instant::now(),
                        swap_completed: false,
                        flushed: false,
                    });

                    state.windowshader_state().frame_ready.insert(
                        native_target.info.registry_name,
                        false,
                    );
                }


                // Finish measuring Screenshaver's GPU work before handing
                // the completed frame to the Wayland compositor.  An
                // xdg_toplevel on an inactive workspace may block for seconds
                // in eglSwapBuffers(); that presentation wait is not shader
                // rendering cost.
                unsafe {
                    gl::QueryCounter(
                        performance_queries[1],
                        gl::TIMESTAMP,
                    );
                }


                control.diagnostic_checkpoint(5);
                let presentation_started =
                    Instant::now();


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


                control.diagnostic_checkpoint(6);
                if runtime.display_format == crate::manage_configuration::WallpaperDisplayFormat::Windowed {
                    if let Some(pending) = state.windowshader_state().windowshader_pending_callbacks.get_mut(&native_target.info.registry_name) {
                        pending.swap_completed = true;
                    }
                }
                last_successful_swap = Instant::now();
                if status_enabled && (paused || resume_presentation_pending
                    || status_heartbeat.elapsed() >= Duration::from_secs(1))
                {
                    crate::renderer_status::publish(status_pipeline, crate::renderer_status::State::Rendering);
                    status_heartbeat = Instant::now();
                }

                if runtime.display_format
                    == crate::manage_configuration::WallpaperDisplayFormat::Windowed
                {
                    presentation_watchdog_swaps = presentation_watchdog_swaps.wrapping_add(1);
                    presentation_watchdog_last_report = None;
                }
                let presentation_duration =
                    presentation_started.elapsed();

                // Diagnostic only: report a compositor wait that overlapped a
                // pause request. Do not change frame pacing or handoff behavior.
                if control.pause_requested() {
                    crate::logger::warning(
                        &crate::locate_paths::runtime_log_path(),
                        &format!(
                            "[AMBIENT_HANDOFF_DIAG] eglSwapBuffers returned after {}ms while pause pending",
                            presentation_duration.as_millis(),
                        ),
                    );
                }


                // A long compositor wait breaks continuous presentation.
                // Begin a new rolling performance epoch instead of carrying
                // samples from before the hidden/inactive interval forward.
                if presentation_duration
                    >= Duration::from_millis(250)
                {
                    frame_times.clear();

                    fps_warning_state =
                        crate::fps_monitor::FpsWarningState::Normal;
                }
            }


            if paused || resume_presentation_pending {
                paused = false;
                resume_presentation_pending = false;

                // Acknowledge only after a successful EGL presentation. This
                // also covers pauses observed by the independent lighting
                // coordinator while EGL was blocked.
                control.acknowledge_resumed_frame();
                crate::logger::warning(
                    &crate::locate_paths::runtime_log_path(),
                    "[WINDOWSHADER_RESUME_DIAG] Successful post-pause EGL presentation acknowledged",
                );


                println!(
                    "Wayland wallpaper rendering resumed."
                );
            }


            unsafe {
                // Preserve the existing synchronization behavior.  The end
                // timestamp was issued before eglSwapBuffers(), so any
                // compositor presentation stall is excluded from the GPU
                // performance sample.
                gl::Finish();
            }


            let mut gpu_start_ns =
                0_u64;

            let mut gpu_end_ns =
                0_u64;

            unsafe {
                gl::GetQueryObjectui64v(
                    performance_queries[0],
                    gl::QUERY_RESULT,
                    &mut gpu_start_ns,
                );

                gl::GetQueryObjectui64v(
                    performance_queries[1],
                    gl::QUERY_RESULT,
                    &mut gpu_end_ns,
                );
            }


            let gpu_render_duration =
                Duration::from_nanos(
                    gpu_end_ns.saturating_sub(
                        gpu_start_ns
                    )
                );


            let performance_status =
                frame_times.record(
                    gpu_render_duration,
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
                        &mut notification_state,
                        false,
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


            control.diagnostic_checkpoint(7);
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

        gl::DeleteQueries(
            2,
            performance_queries.as_ptr(),
        );

        gl::DeleteProgram(
            program
        );
    }


    if status_enabled {
        crate::renderer_status::publish(status_pipeline, crate::renderer_status::State::Stopping);
        crate::renderer_status::clear(status_pipeline);
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
    notification_state:
        &mut crate::notify_wallpaper::WallpaperNotificationState,
    shader_changed: bool,
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
            policy_name:
                shader.policy_name.clone(),

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


    if shader_changed {
        notification_state
            .show_shader_changed(
                enabled,
                &metadata,
            );
    } else {
        notification_state
            .show_update(
                enabled,
                &metadata,
            );
    }
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

    remove_disconnected_targets_from_names(
        display,
        context,
        &mut state.removed_output_names,
        egl_targets,
        native_targets,
        postprocess_pipelines,
    )
}

// Keep EGL cleanup independent of the Wayland event-dispatch state type.
// A Windowshader-owned queue can supply its own removed-output list.
fn remove_disconnected_targets_from_names(
    display: EglDisplay,
    context: EglContext,
    removed_output_names: &mut Vec<u32>,
    egl_targets: &mut Vec<EglWallpaperTarget>,
    native_targets: &mut Vec<NativeWallpaperTarget>,
    postprocess_pipelines: &mut HashMap<u32, crate::postprocess_shader::PostprocessPipeline>,
) -> Result<(), String> {
    let removed_names = std::mem::take(removed_output_names);

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
            xdg_surface,
            xdg_toplevel,
            toplevel_decoration,
            egl_window,
            ..
        } = native_target;


        drop(
            egl_window
        );


        if let Some(layer_surface) =
            layer_surface
        {
            layer_surface.destroy();
        }


        if let Some(decoration) = toplevel_decoration {
            decoration.destroy();
        }

        if let Some(xdg_toplevel) =
            xdg_toplevel
        {
            xdg_toplevel.destroy();
        }


        if let Some(xdg_surface) =
            xdg_surface
        {
            xdg_surface.destroy();
        }


        surface.destroy();


        if let Some(input_region) =
            input_region
        {
            input_region.destroy();
        }


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
    apply_pending_target_resizes_from_states(&mut state.surface_states, native_targets)
}

// Resizing is independent of the dispatch-state type; only surface records
// and native EGL targets are required.
fn apply_pending_target_resizes_from_states(
    surface_states: &mut [WallpaperSurfaceState],
    native_targets: &mut [NativeWallpaperTarget],
) -> Result<(), String> {

    for surface_state in surface_states.iter_mut() {

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
    process_wayland_events_shared(event_queue, state, |state| {
        for (target, pending) in state.windowshader.windowshader_pending_callbacks.iter_mut() {
            if !pending.flushed {
                pending.flushed = true;
                if pending.requested_at.elapsed() >= Duration::from_millis(250) {
                    crate::logger::warning(
                        &crate::locate_paths::runtime_log_path(),
                        &format!("[WINDOWSHADER_CALLBACK_DIAG] Wayland flush succeeded: target={} seq={} age_ms={} swap_completed={}",
                            target, pending.sequence, pending.requested_at.elapsed().as_millis(), pending.swap_completed),
                    );
                }
            }
        }
    })
}

/// Dispatch-state-neutral event pump.  The callback runs only after a
/// successful flush, preserving the existing callback diagnostic ordering.
/// A dedicated Windowshader queue can use this without depending on
/// EventQueue<WaylandState>.
pub(crate) fn process_wayland_events_shared<S, F>(
    event_queue: &mut wayland_client::EventQueue<S>,
    state: &mut S,
    mut after_flush: F,
) -> Result<(), String>
where
    S: 'static,
    F: FnMut(&mut S),
{
    event_queue
        .flush()
        .map_err(|error| format!("Unable to flush Wayland wallpaper requests: {}", error))?;

    after_flush(state);

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

