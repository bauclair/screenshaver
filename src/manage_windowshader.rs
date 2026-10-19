//! Dedicated Windowshader rendering-dispatch boundary.
//!
//! Stage 1 intentionally reuses the existing, tested wallpaper rendering core.
//! Windowshader has independent supervisor and cross-process pause/resume
//! ownership; native Wayland/X11 presentation remains shared until the next
//! backend extraction. No shader-policy eligibility changes in this stage.

use std::sync::{atomic::AtomicBool, Arc};
use crate::define_wallpaper::WallpaperRuntime;
use crate::manage_wallpaper_runtime::WallpaperRuntimeControl;

pub fn run(
    configured_mode: &str,
    runtime: &WallpaperRuntime,
    running: Arc<AtomicBool>,
    control: WallpaperRuntimeControl,
) -> Result<(), String> {
    if runtime.display_format != crate::manage_configuration::WallpaperDisplayFormat::Windowed {
        return Err("Windowshader dispatcher requires Windowed display format".to_string());
    }
    crate::manage_wallpaper::run_with_backend_for_target(
        configured_mode, runtime, running, control,
        crate::windowshader_backend::create_backend,
        crate::manage_policies::PolicyTarget::Windowshader,
        crate::manage_shader::OrderedTarget::Windowshader,
    )
}
