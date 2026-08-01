use std::path::PathBuf;
use std::sync::{
    atomic::{
        AtomicBool,
        Ordering,
    },
    Arc,
};
use std::thread::JoinHandle;
use std::time::Duration;


const MAX_RESTART_ATTEMPTS: usize = 3;
const RESTART_DELAY_SECONDS: u64 = 2;


#[derive(Clone)]
pub struct WallpaperRuntimeControl {
    enabled: bool,
    active: Arc<AtomicBool>,
    pause_requested: Arc<AtomicBool>,
    pause_acknowledged: Arc<AtomicBool>,
    resume_frame_ready: Arc<AtomicBool>,
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
            resume_frame_ready: Arc::new(AtomicBool::new(true)),
        }
    }


    pub fn request_pause_after_first_frame(
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
            true,
            Ordering::SeqCst,
        );


        let deadline =
            std::time::Instant::now()
                + Duration::from_millis(500);


        while running.load(Ordering::SeqCst)
            && self.active.load(Ordering::SeqCst)
            && !self.pause_acknowledged.load(Ordering::SeqCst)
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(
                Duration::from_millis(1)
            );
        }
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
                "[WALLPAPER] Wallpaper is disabled by screenshaver.toml",
            );


            return Self {
                running,
                control,
                thread: None,
            };
        }


        crate::logger::information(
            &logfile,
            "[WALLPAPER] Starting automatic wallpaper runtime",
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
                                        "[WALLPAPER] Wallpaper runtime stopped cleanly",
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
                                        &format!(
                                            "[WALLPAPER] Runtime attempt {}/{} failed: {}",
                                            attempt,
                                            MAX_RESTART_ATTEMPTS,
                                            error,
                                        ),
                                    );
                                }


                                Err(_) => {

                                    crate::logger::error(
                                        &logfile,
                                        &format!(
                                            "[WALLPAPER] Runtime attempt {}/{} panicked",
                                            attempt,
                                            MAX_RESTART_ATTEMPTS,
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
                                "[WALLPAPER] Wallpaper disabled for the current session after repeated failures",
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
                            &format!(
                                "[WALLPAPER] Unable to create wallpaper thread: {}",
                                error,
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
                    "[WALLPAPER] Wallpaper supervisor thread panicked during shutdown."
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

