//! Independent windowshader supervisor identity and control ownership.
//! Reuses the tested restart/supervision algorithm without duplicating it.
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc};
use crate::define_wallpaper::WallpaperRuntime;
use crate::manage_wallpaper_runtime::{WallpaperRuntimeControl, WallpaperRuntimeManager};

pub struct WindowshaderRuntimeManager { inner: WallpaperRuntimeManager }
impl WindowshaderRuntimeManager {
    pub fn start(enabled: bool, configured_mode: String, runtime: WallpaperRuntime,
        logfile: PathBuf, running: Arc<AtomicBool>) -> Self {
        Self { inner: WallpaperRuntimeManager::start_for_kind(
            enabled, true, configured_mode, runtime, logfile, running) }
    }
    pub fn control(&self) -> WallpaperRuntimeControl { self.inner.control() }
    pub fn stop_and_join(&mut self) { self.inner.stop_and_join(); }
}
