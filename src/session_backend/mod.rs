pub mod gnome;
pub mod logind;
pub mod wayland;
pub mod x11;

use std::time::Duration;

use crate::query_session::{
    SessionBackend,
    SessionError,
};


pub fn create_backend(
    idle_timeout: Duration,
) -> Result<Box<dyn SessionBackend>, SessionError> {

    match wayland::WaylandBackend::new(
        idle_timeout
    ) {

        Ok(backend) => {

            let report =
                backend.report();


            println!(
                "[SESSION] Wayland backend initialized"
            );


            println!(
                "[SESSION] Wayland globals:"
            );


            for global in
                &report.globals
            {
                println!(
                    "[SESSION]   [{}] {} v{}",
                    global.name,
                    global.interface,
                    global.version,
                );
            }


            println!(
                "[SESSION] Selected [WAYLAND] backend"
            );


            return Ok(
                Box::new(backend)
            );
        }


        Err(error) => {

            println!(
                "[SESSION] Wayland backend unavailable: {}",
                error,
            );
        }
    }


    match gnome::GnomeBackend::new(
        idle_timeout
    ) {

        Ok(backend) => {

            println!(
                "[SESSION] Selected [GNOME] backend"
            );


            return Ok(
                Box::new(backend)
            );
        }


        Err(error) => {

            println!(
                "[SESSION] GNOME backend unavailable: {}",
                error,
            );
        }
    }


    match x11::X11Backend::new(
        idle_timeout
    ) {

        Ok(backend) => {

            println!(
                "[SESSION] Selected [X11] backend"
            );


            return Ok(
                Box::new(backend)
            );
        }


        Err(error) => {

            println!(
                "[SESSION] X11 backend unavailable: {}",
                error,
            );
        }
    }


    match logind::LogindBackend::new(
        idle_timeout
    ) {

        Ok(backend) => {

            println!(
                "[SESSION] Selected [LOGIND] backend"
            );


            Ok(
                Box::new(backend)
            )
        }


        Err(error) => {

            println!(
                "[SESSION] logind backend unavailable: {}",
                error,
            );


            Err(
                SessionError::BackendUnavailable(
                    "No compatible session backend available"
                        .to_string()
                )
            )
        }
    }
}