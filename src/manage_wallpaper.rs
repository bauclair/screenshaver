use crate::define_wallpaper::WallpaperSettings;


pub fn run(
    settings: WallpaperSettings,
) -> Result<(), String> {

    let wallpaper_directory =
        crate::locate_wallpaper::wallpaper_directory()
            .map_err(
                |error| {
                    format!(
                        "Unable to locate the wallpaper directory: {}",
                        error,
                    )
                }
            )?;


    let shader_files =
        crate::locate_wallpaper::locate_shader_files(
            &wallpaper_directory
        )
        .map_err(
            |error| {
                format!(
                    "Unable to enumerate wallpaper shaders in {}: {}",
                    wallpaper_directory.display(),
                    error,
                )
            }
        )?;


    println!(
        "Screenshaver {}\n",
        env!(
            "CARGO_PKG_VERSION"
        )
    );


    println!(
        "Wallpaper mode configuration:"
    );


    println!(
        "    Monitor mode: {}",
        settings.monitor_mode.name()
    );


    println!(
        "    Notifications: {}",
        if settings.notifications {
            "enabled"
        } else {
            "disabled"
        }
    );


    println!(
        "    Wallpaper directory: {}",
        wallpaper_directory.display()
    );


    println!();


    println!(
        "Compatible wallpaper shaders: {}",
        shader_files.len()
    );


    if shader_files.is_empty() {

        println!(
            "    No .glsl, .fs, or .shaver files were found."
        );

    } else {

        for shader_file in
            &shader_files
        {
            let display_name =
                shader_file
                    .file_name()
                    .and_then(
                        |name| {
                            name.to_str()
                        }
                    )
                    .unwrap_or_else(
                        || {
                            shader_file
                                .as_os_str()
                                .to_str()
                                .unwrap_or(
                                    "<invalid filename>"
                                )
                        }
                    );


            println!(
                "    {}",
                display_name
            );
        }
    }


    println!();


    println!(
        "Wallpaper rendering is not implemented yet."
    );


    Ok(())
}

