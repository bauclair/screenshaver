use std::time::Duration;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Active,
    Idle,
}


#[derive(Debug)]
pub enum SessionError {
    BackendUnavailable(String),
    QueryFailed(String),
}


impl std::fmt::Display for SessionError {

    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>
    ) -> std::fmt::Result {

        match self {

            SessionError::BackendUnavailable(msg) => {
                write!(f, "Backend unavailable: {}", msg)
            }

            SessionError::QueryFailed(msg) => {
                write!(f, "Session query failed: {}", msg)
            }
        }
    }
}


impl std::error::Error for SessionError {}


/// Trait implemented by session backends.
///
/// Backends own the details of how idle state is determined.
pub trait SessionBackend {

    fn poll_state(
        &self
    ) -> Result<SessionState, SessionError>;

    fn backend_name(
        &self
    ) -> &'static str;
}


pub struct SessionQuery {
    backend: Box<dyn SessionBackend>,
}


impl SessionQuery {

    pub fn new(
        idle_timeout: Duration
    ) -> Result<Self, SessionError> {

        let backend =
            crate::session_backend::create_backend(
                idle_timeout
            )?;

        Ok(Self {
            backend,
        })
    }


    pub fn poll_state(
        &self
    ) -> Result<SessionState, SessionError> {

        self.backend.poll_state()
    }


    pub fn backend_name(
        &self
    ) -> &'static str {

        self.backend.backend_name()
    }
}