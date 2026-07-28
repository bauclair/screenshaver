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


        println!();


        println!(
            "Wallpaper rendering is not implemented yet."
        );


        return Ok(());
    }


    for shader_file in
        &shader_files
    {
        println!(
            "    {}",
            display_name(
                shader_file
            )
        );
    }


    let selected_shader =
        &shader_files[0];


    println!();


    println!(
        "Selected wallpaper shader:"
    );


    println!(
        "    {}",
        display_name(
            selected_shader
        )
    );


    println!(
        "    {}",
        selected_shader.display()
    );


    println!();


    println!(
        "Loading and preprocessing selected shader..."
    );


    match crate::load_shader::load_shader_for_preview(
        selected_shader
    ) {

        crate::load_shader::ShaderLoadResult::Ready {
            source,
            shader_name,
            built_in_default,
            ..
        } => {

            println!(
                "Wallpaper shader is ready:"
            );


            println!(
                "    Shader: {}",
                shader_name
            );


            println!(
                "    Processed source: {} bytes",
                source.len()
            );


            println!(
                "    Built-in default: {}",
                built_in_default
            );
        }


        crate::load_shader::ShaderLoadResult::Rejected {
            shader_name,
            reasons,
        } => {

            println!(
                "Wallpaper shader was rejected:"
            );


            println!(
                "    Shader: {}",
                shader_name
            );


            for reason in
                reasons
            {
                println!(
                    "    Reason: {}",
                    reason
                );
            }
        }


        crate::load_shader::ShaderLoadResult::Unavailable {
            shader_name,
            error,
        } => {

            println!(
                "Wallpaper shader is unavailable:"
            );


            println!(
                "    Shader: {}",
                shader_name
            );


            println!(
                "    Error: {}",
                error
            );
        }
    }


    println!();


    println!(
        "Wallpaper shader compilation and rendering are not implemented yet."
    );


    Ok(())
}


fn display_name(
    path: &std::path::Path,
) -> &str {

    path.file_name()
        .and_then(
            |name| {
                name.to_str()
            }
        )
        .unwrap_or(
            "<invalid filename>"
        )
}

