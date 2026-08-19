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


pub fn shader_dir() -> PathBuf {

    screenshaver_dir()
        .join("shaders")
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


pub fn database_path() -> PathBuf {

    screenshaver_dir()
        .join("screenshaver.db")
}
