use std::path::PathBuf;


pub fn home_dir() -> PathBuf {

    PathBuf::from(
        std::env::var("HOME")
            .unwrap_or_else(
                |_| "/home".to_string()
            )
    )
}


pub fn screenshaver_dir() -> PathBuf {

    home_dir()
        .join(".config")
        .join("screenshaver")
}


pub fn screensaver_shader_dir() -> PathBuf {

    screenshaver_dir()
        .join("screensavers")
}


pub fn wallpaper_shader_dir() -> PathBuf {

    screenshaver_dir()
        .join("wallpapers")
}


/// Compatibility alias for existing screensaver-path callers.
///
/// New code should use `screensaver_shader_dir()` so that screensaver and
/// wallpaper shader locations remain unambiguous.
pub fn shader_dir() -> PathBuf {

    screensaver_shader_dir()
}


pub fn shader_cache_dir() -> PathBuf {

    screenshaver_dir()
        .join("cache")
}


pub fn rejected_shader_dir() -> PathBuf {

    screenshaver_dir()
        .join("rejected")
}


pub fn runtime_log_path() -> PathBuf {

    screenshaver_dir()
        .join("screenshaver.log")
}


pub fn state_path() -> PathBuf {

    screenshaver_dir()
        .join("state.json")
}


pub fn legacy_recent_shader_history_path() -> PathBuf {

    screenshaver_dir()
        .join("recent-shaders.json")
}


pub fn config_path() -> PathBuf {

    screenshaver_dir()
        .join("screenshaver.toml")
}

