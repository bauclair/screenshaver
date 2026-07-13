use std::time::Duration;

use crate::query_session::{
    SessionBackend,
    SessionError,
    SessionState,
};

pub struct X11Backend;

impl X11Backend {

    pub fn new(
        _idle_timeout: Duration,
    ) -> Result<Self, SessionError> {

        Err(
            SessionError::BackendUnavailable(
                "X11 backend not yet implemented".to_string()
            )
        )
    }
}

impl SessionBackend for X11Backend {

    fn poll_state(
        &self,
    ) -> Result<SessionState, SessionError> {

        Err(
            SessionError::QueryFailed(
                "X11 backend not yet implemented".to_string()
            )
        )
    }

    fn backend_name(
        &self,
    ) -> &'static str {

        "x11"
    }
}