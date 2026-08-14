pub mod pulseaudio;

use std::fmt;


pub trait AudioBackend {

    fn backend_name(
        &self,
    ) -> &'static str;
}


#[derive(Debug)]
pub enum AudioError {

    BackendUnavailable(
        String
    ),

    InitializationFailed(
        String
    ),
}


impl fmt::Display for AudioError {

    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {

        match self {

            Self::BackendUnavailable(
                message
            )
            | Self::InitializationFailed(
                message
            ) => {

                write!(
                    formatter,
                    "{}",
                    message,
                )
            }
        }
    }
}


impl std::error::Error for AudioError {}


pub fn create_backend(
) -> Result<Box<dyn AudioBackend>, AudioError> {

    println!(
        "[AUDIO] Attempting backend: PulseAudio"
    );


    match pulseaudio::PulseAudioBackend::new() {

        Ok(backend) => {

            println!(
                "[AUDIO] Selected [PULSEAUDIO] backend"
            );


            let logfile =
                crate::locate_paths::runtime_log_path();


            crate::logger::information(
                &logfile,
                "[AUDIO] Selected [PULSEAUDIO] backend",
            );


            Ok(
                Box::new(backend)
            )
        }


        Err(error) => {

            log_backend_unavailable(
                "PulseAudio",
                &error,
            );


            Err(
                AudioError::BackendUnavailable(
                    "No compatible audio backend available"
                        .to_string()
                )
            )
        }
    }
}


fn log_backend_unavailable(
    backend_name: &str,
    error: &AudioError,
) {

    let message =
        format!(
            "[AUDIO] {} backend unavailable: {}",
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
