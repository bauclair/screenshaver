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


pub fn config_path() -> PathBuf {

    screenshaver_dir()
        .join("screenshaver.toml")
}