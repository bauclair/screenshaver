use wayland_client::protocol::{
    wl_compositor,
    wl_output,
    wl_region,
    wl_registry,
    wl_surface,
};
use wayland_client::{
    delegate_noop,
    Connection,
    Dispatch,
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


pub fn test_background_surface(
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
            .ok_or_else(|| {
                "The Wayland compositor did not advertise wl_compositor"
                    .to_string()
            })?;

    let layer_shell =
        state
            .layer_shell
            .clone()
            .ok_or_else(|| {
                "The Wayland compositor does not advertise zwlr_layer_shell_v1"
                    .to_string()
            })?;

    let output =
        state
            .output
            .clone()
            .ok_or_else(|| {
                "The Wayland compositor did not advertise any wl_output objects"
                    .to_string()
            })?;

    let surface =
        compositor.create_surface(
            &queue_handle,
            (),
        );

    let layer_surface =
        layer_shell.get_layer_surface(
            &surface,
            Some(&output),
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

