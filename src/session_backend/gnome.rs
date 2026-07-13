use std::time::Duration;

use zbus::blocking::{
    Connection,
    Proxy,
};

use crate::query_session::{
    SessionBackend,
    SessionError,
    SessionState,
};

pub struct GnomeBackend {
    _connection: Connection,
    proxy: Proxy<'static>,
    idle_timeout: Duration,
}

impl GnomeBackend {
    pub fn new(
        idle_timeout: Duration,
    ) -> Result<Self, SessionError> {
        println!("[GNOME] Connecting to session bus");

        let connection =
            Connection::session()
                .map_err(|e| {
                    SessionError::BackendUnavailable(
                        format!(
                            "Failed to connect to GNOME session bus: {}",
                            e
                        )
                    )
                })?;

        println!("[GNOME] Session bus connected");

        let proxy =
            Proxy::new(
                &connection,
                "org.gnome.Mutter.IdleMonitor",
                "/org/gnome/Mutter/IdleMonitor/Core",
                "org.gnome.Mutter.IdleMonitor",
            )
            .map_err(|e| {
                SessionError::BackendUnavailable(
                    format!(
                        "Failed to create GNOME IdleMonitor proxy: {}",
                        e
                    )
                )
            })?;

        println!("[GNOME] Mutter IdleMonitor proxy created");

        println!("[GNOME] Testing GNOME IdleMonitor availability");

        let _: u64 =
            proxy
                .call(
                    "GetIdletime",
                    &(),
                )
                .map_err(|e| {
                    SessionError::BackendUnavailable(
                        format!(
                            "GNOME IdleMonitor unavailable: {}",
                            e
                        )
                    )
                })?;

        println!("[GNOME] GNOME IdleMonitor available");
        println!("[GNOME] GNOME backend initialized");

        Ok(
            Self {
                _connection: connection,
                proxy,
                idle_timeout,
            }
        )
    }
}

impl SessionBackend for GnomeBackend {
    fn poll_state(
        &self,
    ) -> Result<SessionState, SessionError> {
        let idle_ms: u64 =
            self.proxy
                .call(
                    "GetIdletime",
                    &(),
                )
                .map_err(|e| {
                    SessionError::QueryFailed(
                        format!(
                            "Failed to query GNOME idle time: {}",
                            e
                        )
                    )
                })?;

        if idle_ms >= self.idle_timeout.as_millis() as u64 {
            Ok(SessionState::Idle)
        } else {
            Ok(SessionState::Active)
        }
    }

    fn backend_name(
        &self,
    ) -> &'static str {
        "gnome"
    }
}