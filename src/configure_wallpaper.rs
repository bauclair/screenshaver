use crate::define_wallpaper::{
    WallpaperMonitorMode,
    WallpaperSettings,
};


pub fn resolve(
    monitor_mode: &str,
    notifications: bool,
) -> Result<WallpaperSettings, String> {

    let monitor_mode =
        WallpaperMonitorMode::parse(
            monitor_mode
        )?;


    Ok(
        WallpaperSettings {
            monitor_mode,
            notifications,
        }
    )
}
