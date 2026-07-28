use wayland_client::protocol::wl_registry;
use wayland_client::{
    Connection,
    Dispatch,
    QueueHandle,
};


#[derive(Debug, Default)]
pub struct WallpaperWaylandCapabilities {
    pub compositor_version: Option<u32>,
    pub layer_shell_version: Option<u32>,
    pub output_count: usize,
}


#[derive(Debug, Default)]
struct RegistryState {
    capabilities: WallpaperWaylandCapabilities,
}


impl Dispatch<wl_registry::WlRegistry, ()>
    for RegistryState
{
    fn event(
        state: &mut Self,
        _registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {

        match event {

            wl_registry::Event::Global {
                interface,
                version,
                ..
            } => {

                match interface.as_str() {

                    "wl_compositor" => {
                        state
                            .capabilities
                            .compositor_version =
                            Some(
                                version
                            );
                    }


                    "zwlr_layer_shell_v1" => {
                        state
                            .capabilities
                            .layer_shell_version =
                            Some(
                                version
                            );
                    }


                    "wl_output" => {
                        state
                            .capabilities
                            .output_count +=
                            1;
                    }


                    _ => {}
                }
            }


            wl_registry::Event::GlobalRemove {
                ..
            } => {}


            _ => {}
        }
    }
}


pub fn probe_capabilities(
) -> Result<WallpaperWaylandCapabilities, String> {

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
        RegistryState::default();


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
        .capabilities
        .compositor_version
        .is_none()
    {
        return Err(
            "The Wayland compositor did not advertise wl_compositor"
                .to_string()
        );
    }


    if state
        .capabilities
        .layer_shell_version
        .is_none()
    {
        return Err(
            "The Wayland compositor does not advertise zwlr_layer_shell_v1"
                .to_string()
        );
    }


    if state
        .capabilities
        .output_count
        == 0
    {
        return Err(
            "The Wayland compositor did not advertise any wl_output objects"
                .to_string()
        );
    }


    Ok(
        state.capabilities
    )
}

