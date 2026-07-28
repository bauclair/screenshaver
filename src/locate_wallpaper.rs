use std::io;
use std::path::PathBuf;


pub fn wallpaper_directory() -> io::Result<PathBuf> {

    Ok(
        crate::initialize_user_files::config_directory()?
            .join(
                "wallpapers"
            )
    )
}
