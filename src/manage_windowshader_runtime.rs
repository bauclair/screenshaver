//! Windowshader-owned supervisor lifecycle (Stage 2D-1).
//! The pause/reload protocol is still shared via WallpaperRuntimeControl.
use std::path::PathBuf;
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use std::thread::JoinHandle;
use std::time::Duration;
use crate::define_wallpaper::WallpaperRuntime;
use crate::manage_wallpaper_runtime::WallpaperRuntimeControl;

const MAX_RESTART_ATTEMPTS: usize = 3;
const RESTART_DELAY_SECONDS: u64 = 2;

pub struct WindowshaderRuntimeManager {
    running: Arc<AtomicBool>,
    control: WallpaperRuntimeControl,
    thread: Option<JoinHandle<()>>,
}

impl WindowshaderRuntimeManager {
    pub fn start(enabled: bool, configured_mode: String, runtime: WallpaperRuntime,
        logfile: PathBuf, running: Arc<AtomicBool>) -> Self {
        let control = WallpaperRuntimeControl::new(enabled, true);
        if !enabled {
            crate::logger::information(&logfile,
                &crate::manage_localization::runtime_text("wallpaper.runtime.disabled_by_config"));
            return Self { running, control, thread: None };
        }
        crate::logger::information(&logfile,
            &crate::manage_localization::runtime_text("wallpaper.runtime.starting"));
        let thread_running = Arc::clone(&running);
        let thread_control = control.clone();
        let thread_logfile = logfile.clone();
        let thread = std::thread::Builder::new()
            .name("screenshaver-windowshader".to_string())
            .spawn(move || {
                let logfile = thread_logfile;
                thread_control.set_supervisor_active(true);
                crate::control_windowshader::set_runtime_active(true);
                for attempt in 1..=MAX_RESTART_ATTEMPTS {
                    if !thread_running.load(Ordering::SeqCst) { break; }
                    let attempt_running = Arc::clone(&thread_running);
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        crate::manage_windowshader::run(
                            &configured_mode, &runtime, attempt_running, thread_control.clone())
                    }));
                    match result {
                        Ok(Ok(())) => {
                            crate::logger::information(&logfile,
                                &crate::manage_localization::runtime_text("wallpaper.runtime.stopped_cleanly"));
                            thread_control.set_supervisor_active(false);
                            crate::control_windowshader::set_runtime_active(false);
                            return;
                        }
                        Ok(Err(error)) => crate::logger::error(&logfile,
                            &crate::manage_localization::runtime_text_with_params(
                                "wallpaper.runtime.attempt_failed", &[
                                    ("attempt", &attempt.to_string()),
                                    ("maximum", &MAX_RESTART_ATTEMPTS.to_string()),
                                    ("error", &error.to_string()),
                                ])),
                        Err(_) => crate::logger::error(&logfile,
                            &crate::manage_localization::runtime_text_with_params(
                                "wallpaper.runtime.attempt_panicked", &[
                                    ("attempt", &attempt.to_string()),
                                    ("maximum", &MAX_RESTART_ATTEMPTS.to_string()),
                                ])),
                    }
                    if attempt == MAX_RESTART_ATTEMPTS || !thread_running.load(Ordering::SeqCst) {
                        break;
                    }
                    std::thread::sleep(Duration::from_secs(RESTART_DELAY_SECONDS));
                }
                if thread_running.load(Ordering::SeqCst) {
                    crate::logger::error(&logfile,
                        &crate::manage_localization::runtime_text("wallpaper.runtime.disabled_after_failures"));
                }
                thread_control.set_supervisor_active(false);
                crate::control_windowshader::set_runtime_active(false);
            })
            .map_err(|error| {
                crate::logger::error(&logfile,
                    &crate::manage_localization::runtime_text_with_params(
                        "wallpaper.runtime.thread_create_failed", &[("error", &error.to_string())]));
                error
            }).ok();
        Self { running, control, thread }
    }

    pub fn control(&self) -> WallpaperRuntimeControl { self.control.clone() }

    pub fn stop_and_join(&mut self) {
        self.control.clear_supervisor_pause();
        self.running.store(false, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                eprintln!("{}", crate::manage_localization::runtime_text(
                    "wallpaper.runtime.supervisor_panicked_shutdown"));
            }
        }
        crate::control_windowshader::set_runtime_active(false);
    }
}

impl Drop for WindowshaderRuntimeManager {
    fn drop(&mut self) { self.stop_and_join(); }
}
