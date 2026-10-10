use std::path::PathBuf;
use std::sync::{
    atomic::{
        AtomicBool,
        AtomicU8,
        Ordering,
    },
    Arc,
    Mutex,
};
use std::thread::JoinHandle;
use std::time::Duration;


const MAX_RESTART_ATTEMPTS: usize = 3;
const RESTART_DELAY_SECONDS: u64 = 2;


#[derive(Clone)]
pub(crate) struct WallpaperPolicyReload {
    pub(crate) animation_speed_policy:
        crate::load_config::AnimationSpeedPolicy,
    pub(crate) fps_policy:
        crate::load_config::FpsPolicy,
    pub(crate) texture_policy:
        crate::load_config::TexturePolicy,
    pub(crate) postprocess_policy:
        crate::load_config::PostprocessPolicy,
}


impl WallpaperPolicyReload {
    pub(crate) fn from_config(
        config: &crate::load_config::Config,
    ) -> Self {
        Self {
            animation_speed_policy:
                config.wallpaper_speed_policy.clone(),
            fps_policy:
                config.wallpaper_fps_policy.clone(),
            texture_policy:
                config.wallpaper_texture_policy.clone(),
            postprocess_policy:
                config.wallpaper_postprocess_policy.clone(),
        }
    }
}


#[derive(Clone)]
pub struct WallpaperRuntimeControl {
    enabled: bool,
    active: Arc<AtomicBool>,
    pause_requested: Arc<AtomicBool>,
    pause_acknowledged: Arc<AtomicBool>,
    pause_detected: Arc<AtomicBool>,
    render_checkpoint: Arc<AtomicU8>,
    resume_frame_ready: Arc<AtomicBool>,
    pending_policy_reload:
        Arc<Mutex<Option<WallpaperPolicyReload>>>,
}


impl WallpaperRuntimeControl {

    fn new(
        enabled: bool,
    ) -> Self {

        Self {
            enabled,
            active: Arc::new(AtomicBool::new(false)),
            pause_requested: Arc::new(AtomicBool::new(false)),
            pause_acknowledged: Arc::new(AtomicBool::new(false)),
            pause_detected: Arc::new(AtomicBool::new(false)),
            render_checkpoint: Arc::new(AtomicU8::new(0)),
            resume_frame_ready: Arc::new(AtomicBool::new(true)),
            pending_policy_reload:
                Arc::new(Mutex::new(None)),
        }
    }


    pub fn request_pause_after_first_frame(
        &self,
        running: &AtomicBool,
    ) -> bool {

        if !self.enabled
            || !self.active.load(Ordering::SeqCst)
        {
            return true;
        }


        self.resume_frame_ready.store(
            false,
            Ordering::SeqCst,
        );

        // Reset both signals before publishing the new pause request.
        self.pause_acknowledged.store(false, Ordering::SeqCst);
        self.pause_detected.store(false, Ordering::SeqCst);
        self.pause_requested.store(true, Ordering::SeqCst);

        let started = std::time::Instant::now();
        let detection_deadline = started + Duration::from_millis(500);
        let overall_deadline = started + Duration::from_secs(2);
        crate::logger::warning(
            &crate::locate_paths::runtime_log_path(),
            "[AMBIENT_HANDOFF_DIAG] Screensaver requested wallpaper pause (500ms detection, 2000ms overall deadline)",
        );

        // Stage 1: confirm the wallpaper renderer has entered its pause path.
        while running.load(Ordering::SeqCst)
            && self.active.load(Ordering::SeqCst)
            && !self.pause_detected.load(Ordering::SeqCst)
            && !self.pause_acknowledged.load(Ordering::SeqCst)
            && std::time::Instant::now() < detection_deadline
        {
            std::thread::sleep(Duration::from_millis(1));
        }

        let detected = self.pause_detected.load(Ordering::SeqCst);
        let early_acknowledged = self.pause_acknowledged.load(Ordering::SeqCst);
        crate::logger::warning(
            &crate::locate_paths::runtime_log_path(),
            &format!(
                "[AMBIENT_HANDOFF_DIAG] Pause detection stage ended after {}ms: detected={}, acknowledged={}",
                started.elapsed().as_millis(), detected, early_acknowledged,
            ),
        );

        if !detected && !early_acknowledged {
            let checkpoint = self.render_checkpoint.load(Ordering::SeqCst);
            crate::logger::warning(
                &crate::locate_paths::runtime_log_path(),
                &format!(
                    "[AMBIENT_HANDOFF_DIAG] Pause detection timeout: renderer checkpoint={} ({})",
                    checkpoint,
                    Self::checkpoint_description(checkpoint),
                ),
            );
        }

        // Stage 2: only a renderer that detected the request may use the
        // remaining budget to stop its worker and verify hardware restoration.
        if detected || early_acknowledged {
            while running.load(Ordering::SeqCst)
                && self.active.load(Ordering::SeqCst)
                && !self.pause_acknowledged.load(Ordering::SeqCst)
                && std::time::Instant::now() < overall_deadline
            {
                std::thread::sleep(Duration::from_millis(1));
            }
        }

        let still_active = self.active.load(Ordering::SeqCst);
        let acknowledged = self.pause_acknowledged.load(Ordering::SeqCst);
        let still_running = running.load(Ordering::SeqCst);
        crate::logger::warning(
            &crate::locate_paths::runtime_log_path(),
            &format!(
                "[AMBIENT_HANDOFF_DIAG] Screensaver pause wait ended after {}ms: detected={}, active={}, acknowledged={}, running={}",
                started.elapsed().as_millis(), detected, still_active,
                acknowledged, still_running,
            ),
        );
        // Acknowledgment is authoritative; a terminated wallpaper backend
        // cannot continue to own the device, but its durable recovery record
        // still guards any subsequent OpenRGB acquisition.
        !still_active || acknowledged
    }

    /// Diagnostic-only, lock-free render-loop checkpoint. Never changes ownership.
    pub fn diagnostic_checkpoint(&self, checkpoint: u8) {
        self.render_checkpoint.store(checkpoint, Ordering::SeqCst);
    }

    fn checkpoint_description(checkpoint: u8) -> &'static str {
        match checkpoint {
            0 => "not yet in instrumented render loop",
            1 => "processing Wayland events",
            2 => "handling output changes / policy reload",
            3 => "pause checkpoint reached; no pause observed",
            4 => "rendering or GPU sampling",
            5 => "inside eglSwapBuffers",
            6 => "post-presentation processing",
            7 => "frame pacing sleep",
            8 => "paused / releasing lighting",
            _ => "unknown",
        }
    }

    /// Called by the wallpaper render thread before releasing OpenRGB.
    pub fn acknowledge_pause_detected(&self) {
        self.pause_detected.store(true, Ordering::SeqCst);
    }

    pub fn resume_and_wait_for_frame(
        &self,
        running: &AtomicBool,
    ) {

        if !self.enabled
            || !self.active.load(Ordering::SeqCst)
        {
            return;
        }


        self.resume_frame_ready.store(
            false,
            Ordering::SeqCst,
        );


        self.pause_requested.store(
            false,
            Ordering::SeqCst,
        );


        let deadline =
            std::time::Instant::now()
                + Duration::from_millis(500);


        while running.load(Ordering::SeqCst)
            && self.active.load(Ordering::SeqCst)
            && !self.resume_frame_ready.load(Ordering::SeqCst)
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(
                Duration::from_millis(1)
            );
        }
    }


    pub(crate) fn request_policy_reload(
        &self,
        reload: WallpaperPolicyReload,
    ) {
        match self.pending_policy_reload.lock() {
            Ok(mut pending) => {
                *pending = Some(reload);
            }
            Err(poisoned) => {
                let mut pending = poisoned.into_inner();
                *pending = Some(reload);
            }
        }
    }


    pub(crate) fn take_policy_reload(
        &self,
    ) -> Option<WallpaperPolicyReload> {
        match self.pending_policy_reload.lock() {
            Ok(mut pending) => pending.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        }
    }


    pub fn pause_requested(
        &self,
    ) -> bool {

        self.pause_requested.load(
            Ordering::SeqCst
        )
            || crate::control_wallpaper::external_pause_requested()
    }


    pub fn acknowledge_paused(
        &self,
    ) {

        self.pause_acknowledged.store(
            true,
            Ordering::SeqCst,
        );


        crate::control_wallpaper::acknowledge_paused();
    }


    pub fn acknowledge_resumed_frame(
        &self,
    ) {

        self.pause_acknowledged.store(
            false,
            Ordering::SeqCst,
        );


        self.resume_frame_ready.store(
            true,
            Ordering::SeqCst,
        );


        crate::control_wallpaper::acknowledge_resumed_frame();
    }
}


pub struct WallpaperRuntimeManager {
    running: Arc<AtomicBool>,
    control: WallpaperRuntimeControl,
    thread: Option<JoinHandle<()>>,
}


impl WallpaperRuntimeManager {

    pub fn start(
        enabled: bool,
        configured_mode: String,
        runtime: crate::define_wallpaper::WallpaperRuntime,
        logfile: PathBuf,
        running: Arc<AtomicBool>,
    ) -> Self {

        let control =
            WallpaperRuntimeControl::new(
                enabled
            );


        if !enabled {

            crate::logger::information(
                &logfile,
                &crate::manage_localization::runtime_text("wallpaper.runtime.disabled_by_config"),
            );


            return Self {
                running,
                control,
                thread: None,
            };
        }


        crate::logger::information(
            &logfile,
            &crate::manage_localization::runtime_text("wallpaper.runtime.starting"),
        );


        let thread_running =
            Arc::clone(
                &running
            );


        let thread_logfile =
            logfile.clone();


        let thread_control =
            control.clone();


        let thread =
            std::thread::Builder::new()
                .name(
                    "screenshaver-wallpaper".to_string()
                )
                .spawn(
                    move || {

                        let logfile =
                            thread_logfile;


                        thread_control.active.store(
                            true,
                            Ordering::SeqCst,
                        );


                        crate::control_wallpaper::set_runtime_active(
                            true
                        );


                        for attempt in
                            1..=MAX_RESTART_ATTEMPTS
                        {
                            if !thread_running.load(
                                Ordering::SeqCst
                            ) {
                                break;
                            }


                            let attempt_running =
                                Arc::clone(
                                    &thread_running
                                );


                            let result =
                                std::panic::catch_unwind(
                                    std::panic::AssertUnwindSafe(
                                        || {
                                            crate::manage_wallpaper::run(
                                                &configured_mode,
                                                &runtime,
                                                attempt_running,
                                                thread_control.clone(),
                                            )
                                        }
                                    )
                                );


                            match result {

                                Ok(Ok(())) => {

                                    crate::logger::information(
                                        &logfile,
                                        &crate::manage_localization::runtime_text("wallpaper.runtime.stopped_cleanly"),
                                    );


                                    thread_control.active.store(
                                        false,
                                        Ordering::SeqCst,
                                    );


                                    crate::control_wallpaper::set_runtime_active(
                                        false
                                    );


                                    return;
                                }


                                Ok(Err(error)) => {

                                    crate::logger::error(
                                        &logfile,
                                        &crate::manage_localization::runtime_text_with_params(
                                            "wallpaper.runtime.attempt_failed",
                                            &[
                                                ("attempt", &attempt.to_string()),
                                                ("maximum", &MAX_RESTART_ATTEMPTS.to_string()),
                                                ("error", &error.to_string()),
                                            ],
                                        ),
                                    );
                                }


                                Err(_) => {

                                    crate::logger::error(
                                        &logfile,
                                        &crate::manage_localization::runtime_text_with_params(
                                            "wallpaper.runtime.attempt_panicked",
                                            &[
                                                ("attempt", &attempt.to_string()),
                                                ("maximum", &MAX_RESTART_ATTEMPTS.to_string()),
                                            ],
                                        ),
                                    );
                                }
                            }


                            if attempt
                                == MAX_RESTART_ATTEMPTS
                                || !thread_running.load(
                                    Ordering::SeqCst
                                )
                            {
                                break;
                            }


                            std::thread::sleep(
                                Duration::from_secs(
                                    RESTART_DELAY_SECONDS
                                )
                            );
                        }


                        if thread_running.load(
                            Ordering::SeqCst
                        ) {

                            crate::logger::error(
                                &logfile,
                                &crate::manage_localization::runtime_text("wallpaper.runtime.disabled_after_failures"),
                            );
                        }


                        thread_control.active.store(
                            false,
                            Ordering::SeqCst,
                        );


                        crate::control_wallpaper::set_runtime_active(
                            false
                        );
                    }
                )
                .map_err(
                    |error| {
                        crate::logger::error(
                            &logfile,
                            &crate::manage_localization::runtime_text_with_params(
                                "wallpaper.runtime.thread_create_failed",
                                &[
                                    ("error", &error.to_string()),
                                ],
                            ),
                        );


                        error
                    }
                )
                .ok();


        Self {
            running,
            control,
            thread,
        }
    }


    pub fn control(
        &self,
    ) -> WallpaperRuntimeControl {

        self.control.clone()
    }


    pub fn stop_and_join(
        &mut self,
    ) {

        self.control.pause_requested.store(
            false,
            Ordering::SeqCst,
        );


        self.running.store(
            false,
            Ordering::SeqCst,
        );


        if let Some(thread) =
            self.thread.take()
        {
            if thread.join().is_err() {

                eprintln!(
                    "{}",
                    crate::manage_localization::runtime_text(
                        "wallpaper.runtime.supervisor_panicked_shutdown"
                    )
                );
            }
        }


        crate::control_wallpaper::set_runtime_active(
            false
        );
    }
}


impl Drop for WallpaperRuntimeManager {

    fn drop(
        &mut self,
    ) {
        self.stop_and_join();
    }
}

