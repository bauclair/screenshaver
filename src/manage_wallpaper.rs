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


    let (
        source,
        shader_name,
        built_in_default,
    ) =
        match crate::load_shader::load_shader_for_preview(
            selected_shader
        ) {

            crate::load_shader::ShaderLoadResult::Ready {
                source,
                shader_name,
                built_in_default,
                ..
            } => {
                (
                    source,
                    shader_name,
                    built_in_default,
                )
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


                println!();


                println!(
                    "Wallpaper rendering was not started."
                );


                return Ok(());
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


                println!();


                println!(
                    "Wallpaper rendering was not started."
                );


                return Ok(());
            }
        };


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


    println!();


    println!(
        "Probing native Wayland wallpaper capabilities..."
    );


    let capabilities =
        crate::wayland_wallpaper::probe_capabilities()?;


    println!(
        "Wayland wallpaper capabilities are available:"
    );


    println!(
        "    wl_compositor: version {}",
        capabilities
            .compositor_version
            .unwrap_or(
                0
            )
    );


    println!(
        "    zwlr_layer_shell_v1: version {}",
        capabilities
            .layer_shell_version
            .unwrap_or(
                0
            )
    );


    println!(
        "    Wallpaper targets: {}",
        capabilities.targets.len()
    );


    for (
        index,
        output,
    ) in capabilities
        .targets
        .iter()
        .enumerate()
    {
        println!();


        println!(
            "    Target {}:",
            index + 1
        );


        println!(
            "        Registry name: {}",
            output.registry_name
        );


        println!(
            "        Connector: {}",
            output
                .connector_name
                .as_deref()
                .unwrap_or(
                    "<not advertised>"
                )
        );


        println!(
            "        Description: {}",
            output
                .description
                .as_deref()
                .unwrap_or(
                    "<not advertised>"
                )
        );


        println!(
            "        Make: {}",
            output
                .make
                .as_deref()
                .unwrap_or(
                    "<not advertised>"
                )
        );


        println!(
            "        Model: {}",
            output
                .model
                .as_deref()
                .unwrap_or(
                    "<not advertised>"
                )
        );


        println!(
            "        Position: {},{}",
            output.logical_x,
            output.logical_y
        );


        println!(
            "        Current mode: {}x{} @ {:.3} Hz",
            output.mode_width,
            output.mode_height,
            output.refresh_millihertz as f64
                / 1000.0
        );


        println!(
            "        Physical size: {}x{} mm",
            output.physical_width_mm,
            output.physical_height_mm
        );


        println!(
            "        Scale: {}",
            output.scale
        );


        println!(
            "        Transform: {}",
            output
                .transform
                .as_deref()
                .unwrap_or(
                    "<not advertised>"
                )
        );


        println!(
            "        Metadata complete: {}",
            output.complete
        );
    }


    println!();


    println!(
        "Starting native Wayland/EGL wallpaper renderer..."
    );


    crate::wayland_wallpaper::run_egl_background_surface(
        &source
    )?;


    println!();


    println!(
        "Native Wayland/EGL wallpaper renderer ended cleanly."
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

