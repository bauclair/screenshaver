use std::sync::{
    Arc,
    Mutex,
};
use std::sync::atomic::{
    AtomicBool,
    Ordering,
};
use std::thread;
use std::thread::JoinHandle;
use std::time::{
    Duration,
    Instant,
};

use libpulse_binding as pulse;

use pulse::callbacks::ListResult;
use pulse::context::{
    Context,
    FlagSet as ContextFlagSet,
    State as ContextState,
};
use pulse::mainloop::standard::{
    IterateResult,
    Mainloop,
};
use pulse::sample::{
    Format,
    Spec,
};
use pulse::stream::{
    FlagSet as StreamFlagSet,
    PeekResult,
    State as StreamState,
    Stream,
};


const CONNECTION_TIMEOUT:
    Duration =
    Duration::from_secs(2);

const QUERY_TIMEOUT:
    Duration =
    Duration::from_secs(2);

const STREAM_TIMEOUT:
    Duration =
    Duration::from_secs(2);

const CAPTURE_RATE:
    u32 =
    48_000;

const CAPTURE_CHANNELS:
    u8 =
    2;

const CAPTURE_REPORT_INTERVAL:
    Duration =
    Duration::from_secs(5);


pub struct PulseAudioBackend {

    stop_requested:
        Arc<AtomicBool>,

    worker:
        Option<JoinHandle<()>>,
}


impl PulseAudioBackend {

    pub fn new(
    ) -> Result<Self, crate::audio_backend::AudioError> {

        let stop_requested =
            Arc::new(
                AtomicBool::new(
                    false
                )
            );


        let worker_stop =
            Arc::clone(
                &stop_requested
            );


        let (
            startup_sender,
            startup_receiver,
        ) =
            std::sync::mpsc::sync_channel::<
                Result<(), String>
            >(
                1
            );


        let worker =
            thread::Builder::new()
                .name(
                    "screenshaver-audio-pulseaudio"
                        .to_string()
                )
                .spawn(
                    move || {

                        run_capture_worker(
                            worker_stop,
                            startup_sender,
                        );
                    }
                )
                .map_err(
                    |error| {
                        crate::audio_backend::AudioError::InitializationFailed(
                            format!(
                                "Unable to start PulseAudio worker thread: {}",
                                error,
                            )
                        )
                    }
                )?;


        match startup_receiver.recv_timeout(
            CONNECTION_TIMEOUT
                + QUERY_TIMEOUT
                + QUERY_TIMEOUT
                + STREAM_TIMEOUT
                + Duration::from_secs(1)
        ) {

            Ok(
                Ok(())
            ) => {

                Ok(
                    Self {
                        stop_requested,
                        worker:
                            Some(
                                worker
                            ),
                    }
                )
            }

            Ok(
                Err(error)
            ) => {

                stop_requested.store(
                    true,
                    Ordering::SeqCst,
                );


                let _ =
                    worker.join();


                Err(
                    crate::audio_backend::AudioError::InitializationFailed(
                        error
                    )
                )
            }

            Err(error) => {

                stop_requested.store(
                    true,
                    Ordering::SeqCst,
                );


                let _ =
                    worker.join();


                Err(
                    crate::audio_backend::AudioError::InitializationFailed(
                        format!(
                            "Timed out while starting PulseAudio playback capture: {}",
                            error,
                        )
                    )
                )
            }
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

        self.stop_requested.store(
            true,
            Ordering::SeqCst,
        );


        if let Some(worker) =
            self.worker.take()
        {

            let _ =
                worker.join();
        }
    }
}


fn run_capture_worker(
    stop_requested: Arc<AtomicBool>,
    startup_sender:
        std::sync::mpsc::SyncSender<
            Result<(), String>
        >,
) {

    let result =
        run_capture_worker_inner(
            &stop_requested,
            &startup_sender,
        );


    if let Err(error) =
        result
    {

        let _ =
            startup_sender.send(
                Err(
                    error.clone()
                )
            );


        log_capture_information(
            &format!(
                "[AUDIO] PulseAudio capture worker stopped: {}",
                error,
            )
        );
    }
}


fn run_capture_worker_inner(
    stop_requested: &Arc<AtomicBool>,
    startup_sender:
        &std::sync::mpsc::SyncSender<
            Result<(), String>
        >,
) -> Result<(), String> {

    let mut mainloop =
        Mainloop::new()
            .ok_or_else(
                || {
                    "Unable to create PulseAudio main loop"
                        .to_string()
                }
            )?;


    let mut context =
        Context::new(
            &mainloop,
            "Screenshaver",
        )
        .ok_or_else(
            || {
                "Unable to create PulseAudio context"
                    .to_string()
            }
        )?;


    context
        .connect(
            None,
            ContextFlagSet::NOAUTOSPAWN,
            None,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to connect to PulseAudio-compatible server: {}",
                    error,
                )
            }
        )?;


    wait_for_context_ready(
        &mut mainloop,
        &context,
    )?;


    let default_sink =
        query_default_sink(
            &mut mainloop,
            &context,
        )?;


    let monitor_source =
        query_monitor_source(
            &mut mainloop,
            &context,
            &default_sink,
        )?;


    log_capture_information(
        &format!(
            "[AUDIO] Default playback sink: {}",
            default_sink,
        )
    );


    log_capture_information(
        &format!(
            "[AUDIO] Playback monitor source: {}",
            monitor_source,
        )
    );


    let sample_spec =
        Spec {
            format:
                Format::S16NE,
            channels:
                CAPTURE_CHANNELS,
            rate:
                CAPTURE_RATE,
        };


    if !sample_spec.is_valid() {

        return Err(
            "PulseAudio capture sample specification is invalid"
                .to_string()
        );
    }


    let mut stream =
        Stream::new(
            &mut context,
            "Screenshaver Audio Bloom Capture",
            &sample_spec,
            None,
        )
        .ok_or_else(
            || {
                "Unable to create PulseAudio recording stream"
                    .to_string()
            }
        )?;


    stream
        .connect_record(
            Some(
                &monitor_source
            ),
            None,
            StreamFlagSet::ADJUST_LATENCY,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to connect recording stream to '{}': {}",
                    monitor_source,
                    error,
                )
            }
        )?;


    wait_for_stream_ready(
        &mut mainloop,
        &stream,
    )?;


    log_capture_information(
        &format!(
            "[AUDIO] Playback capture active: {} Hz, {} channels, S16 native-endian PCM",
            CAPTURE_RATE,
            CAPTURE_CHANNELS,
        )
    );


    startup_sender
        .send(
            Ok(())
        )
        .map_err(
            |error| {
                format!(
                    "Unable to report PulseAudio capture readiness: {}",
                    error,
                )
            }
        )?;


    let mut captured_bytes:
        u64 =
        0;

    let mut captured_fragments:
        u64 =
        0;

    let mut last_report =
        Instant::now();

    let mut last_report_bytes:
        u64 =
        0;


    while !stop_requested.load(
        Ordering::SeqCst
    ) {

        iterate_mainloop(
            &mut mainloop
        )?;


        match context.get_state() {

            ContextState::Failed => {

                return Err(
                    format!(
                        "PulseAudio context failed during capture: {}",
                        context.errno(),
                    )
                );
            }

            ContextState::Terminated => {

                return Err(
                    "PulseAudio context terminated during capture"
                        .to_string()
                );
            }

            _ => {}
        }


        match stream.get_state() {

            StreamState::Failed => {

                return Err(
                    "PulseAudio recording stream failed during capture"
                        .to_string()
                );
            }

            StreamState::Terminated => {

                return Err(
                    "PulseAudio recording stream terminated during capture"
                        .to_string()
                );
            }

            StreamState::Ready => {

                while stream
                    .readable_size()
                    .unwrap_or(0)
                    > 0
                {

                    match stream.peek() {

                        Ok(
                            PeekResult::Data(
                                data
                            )
                        ) => {

                            captured_bytes +=
                                data.len() as u64;

                            captured_fragments +=
                                1;


                            stream
                                .discard()
                                .map_err(
                                    |error| {
                                        format!(
                                            "Unable to discard captured PulseAudio data: {}",
                                            error,
                                        )
                                    }
                                )?;
                        }

                        Ok(
                            PeekResult::Hole(
                                _
                            )
                        ) => {

                            stream
                                .discard()
                                .map_err(
                                    |error| {
                                        format!(
                                            "Unable to discard PulseAudio capture hole: {}",
                                            error,
                                        )
                                    }
                                )?;
                        }

                        Ok(
                            PeekResult::Empty
                        ) => {
                            break;
                        }

                        Err(error) => {

                            return Err(
                                format!(
                                    "Unable to read PulseAudio capture data: {}",
                                    error,
                                )
                            );
                        }
                    }
                }
            }

            _ => {}
        }


        if last_report.elapsed()
            >= CAPTURE_REPORT_INTERVAL
        {

            let interval_bytes =
                captured_bytes
                    .saturating_sub(
                        last_report_bytes
                    );


            log_capture_information(
                &format!(
                    "[AUDIO] Capture diagnostic: {} bytes received in last {} s; {} bytes / {} fragments total",
                    interval_bytes,
                    CAPTURE_REPORT_INTERVAL.as_secs(),
                    captured_bytes,
                    captured_fragments,
                )
            );


            last_report =
                Instant::now();

            last_report_bytes =
                captured_bytes;
        }


        thread::sleep(
            Duration::from_millis(2)
        );
    }


    log_capture_information(
        "[AUDIO] PulseAudio playback capture stopped"
    );


    Ok(())
}


fn wait_for_context_ready(
    mainloop: &mut Mainloop,
    context: &Context,
) -> Result<(), String> {

    let started =
        Instant::now();


    loop {

        iterate_mainloop(
            mainloop
        )?;


        match context.get_state() {

            ContextState::Ready => {
                return Ok(());
            }

            ContextState::Failed => {

                return Err(
                    format!(
                        "PulseAudio context failed: {}",
                        context.errno(),
                    )
                );
            }

            ContextState::Terminated => {

                return Err(
                    "PulseAudio context terminated during initialization"
                        .to_string()
                );
            }

            _ => {}
        }


        if started.elapsed()
            >= CONNECTION_TIMEOUT
        {

            return Err(
                "Timed out while connecting to PulseAudio-compatible server"
                    .to_string()
            );
        }


        thread::sleep(
            Duration::from_millis(5)
        );
    }
}


fn query_default_sink(
    mainloop: &mut Mainloop,
    context: &Context,
) -> Result<String, String> {

    let result =
        Arc::new(
            Mutex::new(
                None::<Result<String, String>>
            )
        );


    let callback_result =
        Arc::clone(
            &result
        );


    let _operation =
        context
            .introspect()
            .get_server_info(
                move |server_info| {

                    let value =
                        server_info
                            .default_sink_name
                            .as_ref()
                            .map(
                                |name| {
                                    name.to_string()
                                }
                            )
                            .ok_or_else(
                                || {
                                    "PulseAudio server did not report a default playback sink"
                                        .to_string()
                                }
                            );


                    if let Ok(mut slot) =
                        callback_result.lock()
                    {
                        *slot =
                            Some(
                                value
                            );
                    }
                }
            );


    wait_for_query_result(
        mainloop,
        result,
        "default playback sink",
    )
}


fn query_monitor_source(
    mainloop: &mut Mainloop,
    context: &Context,
    sink_name: &str,
) -> Result<String, String> {

    let result =
        Arc::new(
            Mutex::new(
                None::<Result<String, String>>
            )
        );


    let callback_result =
        Arc::clone(
            &result
        );


    let _operation =
        context
            .introspect()
            .get_sink_info_by_name(
                sink_name,
                move |list_result| {

                    let value =
                        match list_result {

                            ListResult::Item(
                                sink_info
                            ) => {

                                sink_info
                                    .monitor_source_name
                                    .as_ref()
                                    .map(
                                        |name| {
                                            name.to_string()
                                        }
                                    )
                                    .ok_or_else(
                                        || {
                                            "Default PulseAudio sink has no monitor source"
                                                .to_string()
                                        }
                                    )
                            }

                            ListResult::End => {
                                return;
                            }

                            ListResult::Error => {
                                Err(
                                    "PulseAudio failed while querying the default sink"
                                        .to_string()
                                )
                            }
                        };


                    if let Ok(mut slot) =
                        callback_result.lock()
                    {
                        *slot =
                            Some(
                                value
                            );
                    }
                }
            );


    wait_for_query_result(
        mainloop,
        result,
        "playback monitor source",
    )
}


fn wait_for_query_result(
    mainloop: &mut Mainloop,
    result: Arc<
        Mutex<
            Option<
                Result<String, String>
            >
        >
    >,
    description: &str,
) -> Result<String, String> {

    let started =
        Instant::now();


    loop {

        iterate_mainloop(
            mainloop
        )?;


        if let Ok(mut slot) =
            result.lock()
        {

            if let Some(value) =
                slot.take()
            {
                return value;
            }
        }


        if started.elapsed()
            >= QUERY_TIMEOUT
        {

            return Err(
                format!(
                    "Timed out while querying PulseAudio {}",
                    description,
                )
            );
        }


        thread::sleep(
            Duration::from_millis(5)
        );
    }
}


fn wait_for_stream_ready(
    mainloop: &mut Mainloop,
    stream: &Stream,
) -> Result<(), String> {

    let started =
        Instant::now();


    loop {

        iterate_mainloop(
            mainloop
        )?;


        match stream.get_state() {

            StreamState::Ready => {
                return Ok(());
            }

            StreamState::Failed => {

                return Err(
                    "PulseAudio recording stream failed during initialization"
                        .to_string()
                );
            }

            StreamState::Terminated => {

                return Err(
                    "PulseAudio recording stream terminated during initialization"
                        .to_string()
                );
            }

            _ => {}
        }


        if started.elapsed()
            >= STREAM_TIMEOUT
        {

            return Err(
                "Timed out while starting PulseAudio recording stream"
                    .to_string()
            );
        }


        thread::sleep(
            Duration::from_millis(5)
        );
    }
}


fn iterate_mainloop(
    mainloop: &mut Mainloop,
) -> Result<(), String> {

    match mainloop.iterate(
        false
    ) {

        IterateResult::Success(_) => {
            Ok(())
        }

        IterateResult::Quit(
            retval
        ) => {

            Err(
                format!(
                    "PulseAudio main loop quit unexpectedly: {:?}",
                    retval,
                )
            )
        }

        IterateResult::Err(
            error
        ) => {

            Err(
                format!(
                    "PulseAudio main loop failed: {}",
                    error,
                )
            )
        }
    }
}


fn log_capture_information(
    message: &str,
) {

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
}
