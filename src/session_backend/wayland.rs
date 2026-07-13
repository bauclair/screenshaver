use std::cell::RefCell;
use std::time::Duration;

use wayland_client::{
    protocol::{
        wl_registry,
        wl_registry::WlRegistry,
        wl_seat,
        wl_seat::WlSeat,
    },
    Connection,
    Dispatch,
    EventQueue,
    QueueHandle,
};

use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1,
    ext_idle_notification_v1::ExtIdleNotificationV1,
    ext_idle_notifier_v1,
    ext_idle_notifier_v1::ExtIdleNotifierV1,
};

use crate::query_session::{
    SessionBackend,
    SessionError,
    SessionState,
};


pub struct WaylandGlobal {
    pub name: u32,
    pub interface: String,
    pub version: u32,
}


pub struct WaylandProbeReport {
    pub globals: Vec<WaylandGlobal>,
}


pub struct WaylandBackend {
    _connection: Connection,
    event_queue: RefCell<EventQueue<WaylandState>>,
    state: RefCell<WaylandState>,
    _registry: WlRegistry,
    _seat: WlSeat,
    _idle_notifier: ExtIdleNotifierV1,
    _idle_notification: ExtIdleNotificationV1,
    report: WaylandProbeReport,
}


struct WaylandState {
    globals: Vec<WaylandGlobal>,
    session_state: SessionState,
}


impl WaylandState {

    fn new() -> Self {

        Self {
            globals: Vec::new(),
            session_state: SessionState::Active,
        }
    }
}


impl Dispatch<WlRegistry, ()> for WaylandState {

    fn event(
        state: &mut Self,
        _registry: &WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event {

            state.globals.push(
                WaylandGlobal {
                    name,
                    interface,
                    version,
                }
            );
        }
    }
}


impl Dispatch<WlSeat, ()> for WaylandState {

    fn event(
        _state: &mut Self,
        _proxy: &WlSeat,
        _event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}


impl Dispatch<ExtIdleNotifierV1, ()> for WaylandState {

    fn event(
        _state: &mut Self,
        _proxy: &ExtIdleNotifierV1,
        _event: ext_idle_notifier_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}


impl Dispatch<ExtIdleNotificationV1, ()> for WaylandState {

    fn event(
        state: &mut Self,
        _proxy: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {

            ext_idle_notification_v1::Event::Idled => {
                println!(
                    "[SESSION] Wayland idle notification: idled"
                );

                state.session_state =
                    SessionState::Idle;
            }

            ext_idle_notification_v1::Event::Resumed => {
                println!(
                    "[SESSION] Wayland idle notification: resumed"
                );

                state.session_state =
                    SessionState::Active;
            }

            _ => {}
        }
    }
}


impl WaylandBackend {

    pub fn new(
        idle_timeout: Duration
    ) -> Result<Self, SessionError> {

        let connection =
            Connection::connect_to_env()
                .map_err(|e| {
                    SessionError::BackendUnavailable(
                        format!(
                            "Unable to connect to Wayland compositor: {}",
                            e
                        )
                    )
                })?;


        let display =
            connection.display();


        let mut event_queue =
            connection.new_event_queue();


        let qh =
            event_queue.handle();


        let registry =
            display.get_registry(
                &qh,
                (),
            );


        let mut state =
            WaylandState::new();


        event_queue
            .roundtrip(&mut state)
            .map_err(|e| {
                SessionError::BackendUnavailable(
                    format!(
                        "Unable to read Wayland registry: {}",
                        e
                    )
                )
            })?;


        let idle_global =
            state
                .globals
                .iter()
                .find(|global| {
                    global.interface == "ext_idle_notifier_v1"
                })
                .map(|global| {
                    (global.name, global.version)
                });


        let seat_global =
            state
                .globals
                .iter()
                .find(|global| {
                    global.interface == "wl_seat"
                })
                .map(|global| {
                    (global.name, global.version)
                });


        let (idle_global_name, idle_global_version) =
            idle_global
                .ok_or_else(|| {
                    SessionError::BackendUnavailable(
                        "Wayland ext_idle_notifier_v1 not advertised".to_string()
                    )
                })?;


        let (seat_global_name, seat_global_version) =
            seat_global
                .ok_or_else(|| {
                    SessionError::BackendUnavailable(
                        "Wayland wl_seat not advertised".to_string()
                    )
                })?;


        let idle_notifier: ExtIdleNotifierV1 =
            registry.bind(
                idle_global_name,
                idle_global_version.min(2),
                &qh,
                (),
            );


        let seat: WlSeat =
            registry.bind(
                seat_global_name,
                seat_global_version.min(9),
                &qh,
                (),
            );


        event_queue
            .roundtrip(&mut state)
            .map_err(|e| {
                SessionError::BackendUnavailable(
                    format!(
                        "Unable to complete Wayland object binds: {}",
                        e
                    )
                )
            })?;


        let timeout_ms: u32 =
            idle_timeout
                .as_millis()
                .try_into()
                .map_err(|_| {
                    SessionError::BackendUnavailable(
                        "Wayland idle timeout is too large".to_string()
                    )
                })?;


        let idle_notification =
            idle_notifier.get_input_idle_notification(
                timeout_ms,
                &seat,
                &qh,
                (),
            );


        event_queue
            .roundtrip(&mut state)
            .map_err(|e| {
                SessionError::BackendUnavailable(
                    format!(
                        "Unable to create Wayland idle notification: {}",
                        e
                    )
                )
            })?;


        let report =
            WaylandProbeReport {
                globals: state.globals.drain(..).collect(),
            };


        Ok(
            Self {
                _connection: connection,
                event_queue: RefCell::new(event_queue),
                state: RefCell::new(state),
                _registry: registry,
                _seat: seat,
                _idle_notifier: idle_notifier,
                _idle_notification: idle_notification,
                report,
            }
        )
    }


    pub fn report(
        &self
    ) -> &WaylandProbeReport {

        &self.report
    }
}


impl SessionBackend for WaylandBackend {

    fn poll_state(
        &self
    ) -> Result<SessionState, SessionError> {

        {
            let mut event_queue =
                self.event_queue.borrow_mut();

            let mut state =
                self.state.borrow_mut();


            event_queue
                .roundtrip(&mut state)
                .map_err(|e| {
                    SessionError::QueryFailed(
                        format!(
                            "Wayland dispatch failed: {}",
                            e
                        )
                    )
                })?;
        }


        let state =
            self.state.borrow();


        Ok(
            state.session_state
        )
    }


    fn backend_name(
        &self
    ) -> &'static str {

        "wayland"
    }
}