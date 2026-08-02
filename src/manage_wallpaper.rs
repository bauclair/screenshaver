use crate::define_wallpaper::WallpaperRuntime;


pub fn run(
    configured_mode: &str,
    runtime: &WallpaperRuntime,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    control: crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
) -> Result<(), String> {

    let parsed_mode =
        crate::parse_mode::parse_mode(
            configured_mode
        );


    let (
        shader_mode,
        shader_interval,
    ) =
        match parsed_mode.mode {

            crate::parse_mode::ModeType::Single => {
                (
                    crate::manage_shader::ShaderMode::Single(
                        parsed_mode.argument.clone()
                    ),
                    None,
                )
            }


            crate::parse_mode::ModeType::Random => {
                (
                    crate::manage_shader::ShaderMode::Random,
                    Some(
                        std::time::Duration::from_secs(
                            crate::parse_interval::parse_interval(
                                &parsed_mode.argument
                            )
                            .seconds
                        )
                    ),
                )
            }


            crate::parse_mode::ModeType::Ordered => {
                (
                    crate::manage_shader::ShaderMode::Ordered,
                    Some(
                        std::time::Duration::from_secs(
                            crate::parse_interval::parse_interval(
                                &parsed_mode.argument
                            )
                            .seconds
                        )
                    ),
                )
            }


            crate::parse_mode::ModeType::Invalid => {
                return Err(
                    format!(
                        "Invalid wallpaper mode '{}'; expected single:<shader>, random:<seconds>, or ordered:<seconds>",
                        configured_mode,
                    )
                );
            }
        };


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
        "    Shader mode: {}",
        configured_mode
    );


    println!(
        "    Monitor mode: {}",
        runtime.monitor_mode.name()
    );


    println!(
        "    Global animation speed: {:.3}x",
        runtime.animation_speed_policy.global_speed
    );


    println!(
        "    Notifications: {}",
        if runtime.notifications {
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
            "Wallpaper rendering was not started."
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


    let shader_names =
        shader_files
            .iter()
            .filter_map(
                |shader_file| {
                    shader_file
                        .file_name()
                        .and_then(
                            |name| {
                                name.to_str()
                            }
                        )
                        .map(
                            |name| {
                                name.to_string()
                            }
                        )
                }
            )
            .collect::<Vec<_>>();


    let shader_manager =
        crate::manage_shader::ShaderManager::from_shader_list(
            shader_mode,
            shader_names,
        );


    let backend =
        crate::wallpaper_backend::create_backend()?;


    backend.report_capabilities();


    backend.run(
        shader_manager,
        &wallpaper_directory,
        shader_interval,
        runtime,
        running,
        control,
    )?;


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

