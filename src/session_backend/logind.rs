use std::time::Duration;

use zbus::blocking::{
    Connection,
    Proxy,
};

use zbus::zvariant::OwnedObjectPath;

use crate::query_session::{
    SessionBackend,
    SessionError,
    SessionState,
};


pub struct LogindBackend {
    connection: Connection,
    session_path: OwnedObjectPath,
    _idle_timeout: Duration,
}


impl LogindBackend {

    pub fn new(
        idle_timeout: Duration
    ) -> Result<Self, SessionError> {

        let connection =
            Connection::system()
                .map_err(|e| {
                    SessionError::BackendUnavailable(
                        format!(
                            "Unable to connect to system bus: {}",
                            e
                        )
                    )
                })?;


        let manager = Proxy::new(
            &connection,
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
        )
        .map_err(|e| {
            SessionError::BackendUnavailable(
                format!(
                    "Unable to create logind manager proxy: {}",
                    e
                )
            )
        })?;


        let pid =
            std::process::id();


        let session_path: OwnedObjectPath =
            manager
                .call(
                    "GetSessionByPID",
                    &(pid),
                )
                .map_err(|e| {
                    SessionError::BackendUnavailable(
                        format!(
                            "Unable to locate session: {}",
                            e
                        )
                    )
                })?;


        Ok(Self {
            connection,
            session_path,
            _idle_timeout: idle_timeout,
        })
    }
}


impl SessionBackend for LogindBackend {

    fn poll_state(
        &self
    ) -> Result<SessionState, SessionError> {

        let session = Proxy::new(
            &self.connection,
            "org.freedesktop.login1",
            self.session_path.as_ref(),
            "org.freedesktop.login1.Session",
        )
        .map_err(|e| {
            SessionError::QueryFailed(
                format!(
                    "Unable to create logind session proxy: {}",
                    e
                )
            )
        })?;


        let idle_hint: bool =
            session
                .get_property("IdleHint")
                .map_err(|e| {
                    SessionError::QueryFailed(
                        format!(
                            "Unable to read logind IdleHint: {}",
                            e
                        )
                    )
                })?;


        if idle_hint {
            Ok(SessionState::Idle)
        } else {
            Ok(SessionState::Active)
        }
    }


    fn backend_name(
        &self
    ) -> &'static str {

        "logind"
    }
}