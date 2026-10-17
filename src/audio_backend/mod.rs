pub mod pulseaudio;

use std::fmt;
use std::sync::{
    Arc,
    OnceLock,
    RwLock,
    Mutex,
};
use std::sync::atomic::{
    AtomicBool,
    Ordering,
};
use std::sync::mpsc;


pub type SharedAudioBands =
    Arc<
        RwLock<
            crate::analyze_audio::AudioBands
        >
    >;


pub fn new_shared_audio_bands(
) -> SharedAudioBands {

    Arc::new(
        RwLock::new(
            crate::analyze_audio::AudioBands::default()
        )
    )
}


pub trait AudioBackend {

    fn backend_name(
        &self,
    ) -> &'static str;


    fn shared_bands(
        &self,
    ) -> SharedAudioBands;
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


/// Process-local, demand-driven Audio Bloom runtime.
///
/// Merely creating or accessing this runtime does not open an audio capture
/// stream.  Renderers call `set_audio_required(true)` only while the active
/// policy uses Audio Bloom.  Backend startup and shutdown occur on this
/// runtime's worker thread so PulseAudio discovery can never stall a render
/// transition.
///
/// The `SharedAudioBands` allocation is deliberately persistent for the
/// lifetime of the process.  Renderers therefore retain one stable Arc while
/// capture may start and stop behind it.  When capture is not required, the
/// bands are zero and Audio Bloom contributes nothing.
#[derive(Default)]
struct AudioDemands {
    legacy: bool,
    next_id: u64,
    owners: std::collections::HashMap<u64, bool>,
}

/// A renderer-owned demand registration, automatically released on drop.
pub struct AudioDemandHandle { id: u64 }
impl AudioDemandHandle {
    pub fn set_required(&self, required: bool) {
        audio_runtime().set_owner_required(self.id, required);
    }
}
impl Drop for AudioDemandHandle {
    fn drop(&mut self) { audio_runtime().unregister(self.id); }
}
pub fn register_audio_demand() -> AudioDemandHandle {
    audio_runtime().register()
}

struct AudioRuntime {
    shared_bands: SharedAudioBands,
    required: Arc<AtomicBool>,
    demands: Mutex<AudioDemands>,
    wake_sender: mpsc::Sender<()>,
}


static AUDIO_RUNTIME:
    OnceLock<AudioRuntime> =
    OnceLock::new();


impl AudioRuntime {

    fn new() -> Self {
        let shared_bands =
            new_shared_audio_bands();

        let required =
            Arc::new(
                AtomicBool::new(
                    false
                )
            );

        let (
            wake_sender,
            wake_receiver,
        ) =
            mpsc::channel::<()>();

        let worker_bands =
            Arc::clone(
                &shared_bands
            );

        let worker_required =
            Arc::clone(
                &required
            );

        let spawn_result =
            std::thread::Builder::new()
                .name(
                    "screenshaver-audio-runtime"
                        .to_string()
                )
                .spawn(
                    move || {
                        run_audio_runtime_worker(
                            worker_required,
                            worker_bands,
                            wake_receiver,
                        );
                    }
                );

        if let Err(error) =
            spawn_result
        {
            let message =
                format!(
                    "[AUDIO] Unable to start demand-driven audio runtime worker: {}",
                    error,
                );

            eprintln!(
                "{}",
                message
            );

            let logfile =
                crate::locate_paths::runtime_log_path();

            crate::logger::warning(
                &logfile,
                &message,
            );
        }

        Self {
            shared_bands,
            required,
            demands: Mutex::new(AudioDemands::default()),
            wake_sender,
        }
    }


    fn shared_bands(
        &self,
    ) -> SharedAudioBands {
        Arc::clone(
            &self.shared_bands
        )
    }


    fn update_demand(&self, update: impl FnOnce(&mut AudioDemands)) {
        let mut demands = self.demands.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        update(&mut demands);
        let effective = demands.legacy || demands.owners.values().any(|v| *v);
        let previous = self.required.swap(effective, Ordering::SeqCst);
        drop(demands);
        if previous != effective {
            if !effective { zero_shared_bands(&self.shared_bands); }
            let _ = self.wake_sender.send(());
        }
    }
    fn set_required(&self, required: bool) {
        self.update_demand(|d| d.legacy = required);
    }
    fn register(&self) -> AudioDemandHandle {
        let mut id = 0;
        self.update_demand(|d| {
            id = d.next_id;
            d.next_id = d.next_id.checked_add(1).expect("audio demand IDs exhausted");
            d.owners.insert(id, false);
        });
        AudioDemandHandle { id }
    }
    fn set_owner_required(&self, id: u64, required: bool) {
        self.update_demand(|d| {
            if let Some(value) = d.owners.get_mut(&id) { *value = required; }
        });
    }
    fn unregister(&self, id: u64) {
        self.update_demand(|d| { d.owners.remove(&id); });
    }

}


/// Returns the stable, process-local audio-band handle without activating
/// PulseAudio.  This is safe to pass to renderers at process startup.
pub fn shared_audio_bands(
) -> SharedAudioBands {
    audio_runtime()
        .shared_bands()
}


/// Declare whether the renderer that currently owns presentation requires
/// live playback-monitor capture for Audio Bloom.
///
/// Repeating the same value is intentionally a no-op.  This makes it safe for
/// an active renderer to reassert its requirement once per frame, which is
/// important when a paused wallpaper resumes after a screensaver or editor
/// temporarily owned presentation.
pub fn set_audio_required(
    required: bool,
) {
    audio_runtime()
        .set_required(
            required
        );
}


fn audio_runtime(
) -> &'static AudioRuntime {
    AUDIO_RUNTIME.get_or_init(
        AudioRuntime::new
    )
}


fn run_audio_runtime_worker(
    required: Arc<AtomicBool>,
    shared_bands: SharedAudioBands,
    wake_receiver: mpsc::Receiver<()>,
) {
    let mut backend:
        Option<Box<dyn AudioBackend>> =
        None;

    while wake_receiver.recv().is_ok() {
        let should_run =
            required.load(
                Ordering::SeqCst
            );

        if should_run
            && backend.is_none()
        {
            match create_backend_with_shared_bands(
                Arc::clone(
                    &shared_bands
                )
            ) {
                Ok(created_backend) => {
                    let backend_name =
                        created_backend.backend_name();

                    // A render transition may have disabled Audio Bloom while
                    // backend discovery was still in progress.  In that case,
                    // never expose a stale capture stream after startup.
                    if !required.load(
                        Ordering::SeqCst
                    ) {
                        drop(
                            created_backend
                        );

                        zero_shared_bands(
                            &shared_bands
                        );

                        continue;
                    }

                    let logfile =
                        crate::locate_paths::runtime_log_path();

                    crate::logger::information(
                        &logfile,
                        &format!(
                            "[AUDIO] Demand-driven Audio Bloom capture active: {}",
                            backend_name,
                        ),
                    );

                    backend =
                        Some(
                            created_backend
                        );
                }

                Err(error) => {
                    zero_shared_bands(
                        &shared_bands
                    );

                    let logfile =
                        crate::locate_paths::runtime_log_path();

                    crate::logger::warning(
                        &logfile,
                        &format!(
                            "[AUDIO] Audio Bloom capture could not be started; continuing with zero audio bands: {}",
                            error,
                        ),
                    );
                }
            }
        }
        else if !should_run
            && backend.is_some()
        {
            backend =
                None;

            zero_shared_bands(
                &shared_bands
            );

            let logfile =
                crate::locate_paths::runtime_log_path();

            crate::logger::information(
                &logfile,
                "[AUDIO] Demand-driven Audio Bloom capture inactive",
            );
        }
    }
}


fn zero_shared_bands(
    shared_bands: &SharedAudioBands,
) {
    if let Ok(mut bands) =
        shared_bands.write()
    {
        *bands =
            crate::analyze_audio::AudioBands::default();
    }
}


pub fn create_backend(
) -> Result<Box<dyn AudioBackend>, AudioError> {
    create_backend_with_shared_bands(
        new_shared_audio_bands()
    )
}


fn create_backend_with_shared_bands(
    shared_bands: SharedAudioBands,
) -> Result<Box<dyn AudioBackend>, AudioError> {

    println!(
        "[AUDIO] Attempting backend: PulseAudio"
    );


    match pulseaudio::PulseAudioBackend::new_with_shared_bands(
        shared_bands
    ) {

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
