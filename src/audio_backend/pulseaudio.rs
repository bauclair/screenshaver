use std::thread;
use std::time::{
    Duration,
    Instant,
};

use libpulse_binding as pulse;

use pulse::context::{
    Context,
    FlagSet,
    State,
};

use pulse::mainloop::standard::{
    IterateResult,
    Mainloop,
};


const CONNECTION_TIMEOUT:
    Duration =
    Duration::from_secs(2);


pub struct PulseAudioBackend {

    mainloop: Mainloop,
    context: Context,
}


impl PulseAudioBackend {

    pub fn new(
    ) -> Result<Self, crate::audio_backend::AudioError> {

        let mut mainloop =
            Mainloop::new()
                .ok_or_else(
                    || {
                        crate::audio_backend::AudioError::InitializationFailed(
                            "Unable to create PulseAudio main loop"
                                .to_string()
                        )
                    }
                )?;


        let mut context =
            Context::new(
                &mainloop,
                "Screenshaver",
            )
            .ok_or_else(
                || {
                    crate::audio_backend::AudioError::InitializationFailed(
                        "Unable to create PulseAudio context"
                            .to_string()
                    )
                }
            )?;


        context
            .connect(
                None,
                FlagSet::NOAUTOSPAWN,
                None,
            )
            .map_err(
                |error| {
                    crate::audio_backend::AudioError::InitializationFailed(
                        format!(
                            "Unable to connect to PulseAudio-compatible server: {}",
                            error,
                        )
                    )
                }
            )?;


        let started =
            Instant::now();


        loop {

            match mainloop.iterate(
                false
            ) {

                IterateResult::Success(_) => {}

                IterateResult::Quit(
                    retval
                ) => {

                    return Err(
                        crate::audio_backend::AudioError::InitializationFailed(
                            format!(
                                "PulseAudio main loop quit during initialization: {:?}",
                                retval,
                            )
                        )
                    );
                }

                IterateResult::Err(
                    error
                ) => {

                    return Err(
                        crate::audio_backend::AudioError::InitializationFailed(
                            format!(
                                "PulseAudio main loop failed during initialization: {}",
                                error,
                            )
                        )
                    );
                }
            }


            match context.get_state() {

                State::Ready => {

                    return Ok(
                        Self {
                            mainloop,
                            context,
                        }
                    );
                }

                State::Failed => {

                    return Err(
                        crate::audio_backend::AudioError::InitializationFailed(
                            format!(
                                "PulseAudio context failed: {}",
                                context.errno(),
                            )
                        )
                    );
                }

                State::Terminated => {

                    return Err(
                        crate::audio_backend::AudioError::InitializationFailed(
                            "PulseAudio context terminated during initialization"
                                .to_string()
                        )
                    );
                }

                _ => {}
            }


            if started.elapsed()
                >= CONNECTION_TIMEOUT
            {

                return Err(
                    crate::audio_backend::AudioError::InitializationFailed(
                        "Timed out while connecting to PulseAudio-compatible server"
                            .to_string()
                    )
                );
            }


            thread::sleep(
                Duration::from_millis(5)
            );
        }
    }
}


impl crate::audio_backend::AudioBackend
    for PulseAudioBackend
{

    fn backend_name(
        &self,
    ) -> &'static str {

        "PulseAudio"
    }
}


impl Drop for PulseAudioBackend {

    fn drop(
        &mut self,
    ) {

        self.context.disconnect();

        self.mainloop.quit(
            pulse::def::Retval(0)
        );
    }
}
