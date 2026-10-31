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


pub(crate) struct EglWallpaperTarget {
    pub(crate) registry_name: u32,
    pub(crate) surface: EglSurface,
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
    // Diagnostic-only swap counters; never affect rendering or presentation.
    wallpaper_swap_diagnostics: HashMap<u32, (u64, Instant)>,
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
                crate::logger::warning(
                    &crate::locate_paths::runtime_log_path(),
                    &format!("[WALLPAPER_SURFACE_DIAG] Configure output={} serial={} width={} height={}",
                        data.registry_name, serial, width, height),
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
                crate::logger::warning(
                    &crate::locate_paths::runtime_log_path(),
                    &format!("[WALLPAPER_SURFACE_DIAG] Closed output={}", data.registry_name),
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


// Renderer-owned status cleanup. Never clear the other presentation pipeline.
struct WallpaperTrayStatusCleanup(crate::tray_icon::TrayStatusControl);

impl Drop for WallpaperTrayStatusCleanup {
    fn drop(&mut self) {
        self.0.set_wallpaper_disabled();
    }
}

pub fn run_egl_background_surface(
    mut shader_manager: crate::manage_shader::ShaderManager,
    wallpaper_directory: &Path,
    shader_interval: Option<Duration>,
    runtime: &crate::define_wallpaper::WallpaperRuntime,
    running: Arc<AtomicBool>,
    control: crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
) -> Result<(), String> {
    // Keep the active-policy record accurate on normal exit and early errors.
    let _tray_status_cleanup = WallpaperTrayStatusCleanup(runtime.tray_status.clone());


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
        active_shader.policy_name.clone(),
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
pub(crate) type EglDisplay = *mut c_void;
type EglConfig = *mut c_void;
pub(crate) type EglContext = *mut c_void;
pub(crate) type EglSurface = *mut c_void;
type EglNativeDisplay = *mut c_void;
type EglNativeWindow = *mut c_void;

pub(crate) const EGL_FALSE: EglBoolean = 0;

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
pub(crate) const EGL_NO_CONTEXT: EglContext = ptr::null_mut();
pub(crate) const EGL_NO_SURFACE: EglSurface = ptr::null_mut();


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


    pub(crate) fn eglMakeCurrent(
        display: EglDisplay,
        draw_surface: EglSurface,
        read_surface: EglSurface,
        context: EglContext,
    ) -> EglBoolean;


    pub(crate) fn eglSwapBuffers(
        display: EglDisplay,
        surface: EglSurface,
    ) -> EglBoolean;


    pub(crate) fn eglDestroySurface(
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


    pub(crate) fn eglGetProcAddress(
        name: *const c_char,
    ) -> *const c_void;
}




// Read-only, pipeline-neutral diagnostic values. No mutable XDG state escapes
// through the shared renderer interface.
#[derive(Debug, Default)]
pub(crate) struct PresentationDiagnostics {
    pub(crate) frame_ready: bool,
    pub(crate) pending_description: String,
    pub(crate) pending_sequence: Option<u64>,
    pub(crate) pending_age_ms: Option<u128>,
    pub(crate) activated: bool,
    pub(crate) activation_established: bool,
    pub(crate) configure_count: u64,
}

// Stage 2B-4B2C: shared rendering, independently dispatched presentation.
pub(crate) trait RenderWaylandDispatch:
    wayland_client::Dispatch<wl_callback::WlCallback, crate::wayland_windowshader::FrameCallbackDispatchData>
{
    fn surfaces(&mut self) -> &mut Vec<WallpaperSurfaceState>;
    fn presentation_diagnostics(&self, native_targets: &[NativeWallpaperTarget]) -> PresentationDiagnostics;
    fn removed_outputs(&mut self) -> &mut Vec<u32>;
    // Stage 2D-2C Step 5: each presentation pipeline decides whether a
    // compositor close request also terminates its runtime supervisor.
    fn compositor_close_requests_shutdown() -> bool;
    // Stage 2D-2C Step 6: pipeline-owned pause-entry compositor bookkeeping.
    fn on_pause_entering(&mut self);
    fn on_pause_entered(&mut self, native_targets: &[NativeWallpaperTarget]);
    // Stage 2D-2C Step 7: pipeline-specific resume lifecycle diagnostics.
    fn on_pause_released(&mut self);
    fn on_resumed_frame_presented(&mut self);
    // Stage 2D-2C Step 9: compositor callback swap bookkeeping belongs to its pipeline.
    fn on_frame_swap_completed(&mut self, registry_name: u32);
    // Stage 2D-2C Step 10: frame readiness is decided by the presentation pipeline.
    fn frames_ready_for_presentation(&mut self, native_targets: &[NativeWallpaperTarget]) -> bool;
    // Stage 2D-2C Step 13: pipeline-owned outstanding callback diagnostics.
    fn outstanding_frame_callbacks(&mut self, native_targets: &[NativeWallpaperTarget]) -> Vec<u32>;
    fn log_outstanding_callback_details(&mut self, outstanding: &[u32]);
    // Stage 2D-2C Step 11: pipeline-owned compositor frame registration.
    fn request_next_presentation_frame(
        &mut self, surface: &wl_surface::WlSurface, registry_name: u32,
        queue: &QueueHandle<Self>,
    ) where Self: Sized;
    // Stage 2D-2C Step 8: pipeline-owned frame-deadline reset.
    fn reset_frame_deadline(deadline: &mut Instant);
    fn pace_frame(deadline: &mut Instant, frame_duration: Duration);
    fn pump_events(queue: &mut wayland_client::EventQueue<Self>, state: &mut Self) -> Result<(), String>
    where Self: Sized;
}

impl RenderWaylandDispatch for WaylandState {
    fn surfaces(&mut self) -> &mut Vec<WallpaperSurfaceState> { &mut self.surface_states }
    fn presentation_diagnostics(&self, _native_targets: &[NativeWallpaperTarget]) -> PresentationDiagnostics { PresentationDiagnostics { frame_ready: true, ..PresentationDiagnostics::default() } }
    fn removed_outputs(&mut self) -> &mut Vec<u32> { &mut self.removed_output_names }
    fn compositor_close_requests_shutdown() -> bool { false }
    fn on_pause_entering(&mut self) {}
    fn on_pause_entered(&mut self, _native_targets: &[NativeWallpaperTarget]) {}
    fn on_pause_released(&mut self) {}
    fn on_resumed_frame_presented(&mut self) {}
    fn on_frame_swap_completed(&mut self, registry_name: u32) {
        let (count, reported_at) = self.wallpaper_swap_diagnostics
            .entry(registry_name)
            .or_insert_with(|| (0, Instant::now()));
        *count = count.saturating_add(1);
        if reported_at.elapsed() >= Duration::from_secs(5) {
            crate::logger::information(
                &crate::locate_paths::runtime_log_path(),
                &format!("[WALLPAPER_SURFACE_DIAG] EGL swaps successful output={} count={} interval_ms={}",
                    registry_name, count, reported_at.elapsed().as_millis()),
            );
            *count = 0;
            *reported_at = Instant::now();
        }
    }
    fn frames_ready_for_presentation(&mut self, _native_targets: &[NativeWallpaperTarget]) -> bool { true }
    fn outstanding_frame_callbacks(&mut self, _native_targets: &[NativeWallpaperTarget]) -> Vec<u32> { Vec::new() }
    fn log_outstanding_callback_details(&mut self, _outstanding: &[u32]) {}
    fn request_next_presentation_frame(
        &mut self, _surface: &wl_surface::WlSurface, _registry_name: u32,
        _queue: &QueueHandle<Self>,
    ) {}
    fn reset_frame_deadline(deadline: &mut Instant) {
        *deadline = Instant::now();
    }
    fn pace_frame(deadline: &mut Instant, frame_duration: Duration) {
        *deadline += frame_duration;
        let now = Instant::now();
        if *deadline > now {
            thread::sleep(*deadline - now);
        } else {
            *deadline = now;
        }
    }
    fn pump_events(queue: &mut wayland_client::EventQueue<Self>, state: &mut Self) -> Result<(), String> {
        process_wayland_events(queue, state)
    }
}

// Stage 2D-2B Steps 3A/3B: EGL resources are scoped to a caller-owned session.
// The session owns the EGL display, context, and surfaces; GL shader resources
// continue to be released by the existing frame-processing implementation.
pub(crate) struct EglRenderSession {
    pub(crate) display: EglDisplay,
    pub(crate) context: EglContext,
    pub(crate) targets: Vec<EglWallpaperTarget>,
    pub(crate) major: i32,
    pub(crate) minor: i32,
}

impl EglRenderSession {
    pub(crate) fn initialize(
        connection: &Connection,
        native_targets: &[NativeWallpaperTarget],
    ) -> Result<Self, String> {
        let (display, context, targets, major, minor) =
            initialize_egl_targets(connection, native_targets)?;
        Ok(Self { display, context, targets, major, minor })
    }
}

impl Drop for EglRenderSession {
    fn drop(&mut self) {
        unsafe {
            eglMakeCurrent(
                self.display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT,
            );
        }
        destroy_egl_targets(self.display, &self.targets);
        unsafe {
            eglDestroyContext(self.display, self.context);
            eglTerminate(self.display);
        }
    }
}

// Stage 2D-2A step 1: EGL acquisition is separate from frame-loop ownership.
// The caller still owns GL dispatch loading, frame processing, and teardown.
fn initialize_egl_targets(
    connection: &Connection,
    native_targets: &[NativeWallpaperTarget],
) -> Result<(EglDisplay, EglContext, Vec<EglWallpaperTarget>, i32, i32), String> {
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


    let first_surface = match egl_targets.first() {
        Some(target) => target.surface,
        None => {
            unsafe {
                eglDestroyContext(display, context);
                eglTerminate(display);
            }
            return Err("No EGL wallpaper targets were created".to_string());
        }
    };


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


    Ok((display, context, egl_targets, egl_major, egl_minor))
}


// Stage 2D-2C Step 2: each pipeline owns its initial frame-pacing state.
// The shared GL implementation consumes this state; later pacing updates remain
// inside that implementation until their extraction is independently tested.
pub(crate) struct FrameLoopInitialTiming {
    pub(crate) rendered_fps: u32,
    pub(crate) frame_duration: Duration,
    pub(crate) next_frame_deadline: Instant,
}

impl FrameLoopInitialTiming {
    pub(crate) fn for_shader(
        runtime: &crate::define_wallpaper::WallpaperRuntime,
        shader: &ActiveWallpaperShader,
    ) -> Self {
        let rendered_fps = runtime.fps_policy.rendered_fps_for_policy(
            shader.policy_id, &shader.shader_name,
            Some(shader.source_path.as_path()), None,
        );
        Self {
            frame_duration: crate::wayland_render_core::frame_duration_for_fps(rendered_fps),
            rendered_fps,
            next_frame_deadline: Instant::now(),
        }
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

    let mut egl_session = EglRenderSession::initialize(connection, native_targets)?;
    render_with_egl_session(
        &mut egl_session, event_queue, state, native_targets, active_shader,
        shader_manager, wallpaper_directory, shader_interval, runtime, running, control,
    )
}

pub(crate) fn render_with_egl_session<S: RenderWaylandDispatch + 'static>(
    egl_session: &mut EglRenderSession,
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
    let initial_timing = FrameLoopInitialTiming::for_shader(runtime, active_shader);
    prepare_egl_frame_processing();
    execute_shared_egl_frame_processing(
        egl_session, event_queue, state, native_targets, active_shader,
        shader_manager, wallpaper_directory, shader_interval, runtime, running, control,
        initial_timing,
    )
}

// Compatibility wrappers for existing callers. The shared GL dispatch and EGL
// execution entry points are now housed in wayland_render_core.
pub(crate) fn prepare_egl_frame_processing() {
    crate::wayland_render_core::prepare_egl_frame_processing();
}

pub(crate) fn execute_shared_egl_frame_processing<S: RenderWaylandDispatch + 'static>(
    egl_session: &mut EglRenderSession,
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
    initial_timing: FrameLoopInitialTiming,
) -> Result<(), String> {
    crate::wayland_render_core::execute_shared_egl_frame_processing(
        egl_session, event_queue, state, native_targets, active_shader,
        shader_manager, wallpaper_directory, shader_interval, runtime,
        running, control, initial_timing,
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
    // Stage 2D-2C Step 12: Wallpaper has no Windowshader callback records.
    process_wayland_events_shared(event_queue, state, |_| {})
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


pub(crate) fn uniform_location(
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


pub(crate) fn set_uniform_1f(
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


pub(crate) fn set_uniform_1i(
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


pub(crate) fn set_uniform_3f(
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


pub(crate) fn set_uniform_4f(
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


pub(crate) fn egl_failure(
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

