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


            if report.session_lock_available {

                let version =
                    report
                        .session_lock_version
                        .unwrap_or(1);

                let message =
                    format!(
                        "[LOCK] Wayland ext-session-lock-v1: available (v{})",
                        version,
                    );

                println!(
                    "{}",
                    message
                );

                let logfile =
                    crate::locate_paths::runtime_log_path();

                crate::logger::information(
                    &logfile,
                    &message,
                );

                crate::logger::information(
                    &logfile,
                    "[LOCK] Wayland secure session locking supported",
                );

            } else {

                let message =
                    "[LOCK] Wayland ext-session-lock-v1: unavailable";

                println!(
                    "{}",
                    message
                );

                let logfile =
                    crate::locate_paths::runtime_log_path();

                crate::logger::information(
                    &logfile,
                    message,
                );

                crate::logger::information(
                    &logfile,
                    "[LOCK] Wayland secure session locking unavailable",
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

            log_backend_unavailable(
                "Wayland",
                &error,
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

            log_backend_unavailable(
                "GNOME",
                &error,
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

            log_backend_unavailable(
                "X11",
                &error,
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

            log_backend_unavailable(
                "logind",
                &error,
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

fn log_backend_unavailable(
    backend_name: &str,
    error: &SessionError,
) {

    let message =
        format!(
            "[SESSION] {} backend unavailable: {}",
            backend_name,
            error,
        );


    println!(
        "{}",
        message,
    );


    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::information(
        &logfile,
        &message,
    );
}

