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
        "    Advertised outputs: {}",
        capabilities.output_count
    );


    println!();


    println!(
        "Creating unbuffered Wayland background surface..."
    );


    let surface_configuration =
        crate::wayland_wallpaper::test_egl_background_surface()?;


    println!(
        "Wayland background surface configured successfully:"
    );


    println!(
        "    Width: {}",
        surface_configuration.width
    );


    println!(
        "    Height: {}",
        surface_configuration.height
    );


    println!(
        "    Configure serial: {}",
        surface_configuration.serial
    );


    println!(
        "    Layer: background"
    );


    println!(
        "    Anchors: top, bottom, left, right"
    );


    println!(
        "    Keyboard input: disabled"
    );


    println!(
        "    Pointer input: disabled"
    );


    println!();


    println!(
        "Native Wayland/EGL diagnostic surface test completed."
    );


    println!();


    println!(
        "Opening wallpaper rendering test window..."
    );


    println!(
        "Press Escape or close the window to exit."
    );


    crate::render_wallpaper::run_test_window(
        &source
    )?;


    println!();


    println!(
        "Wallpaper rendering test ended cleanly."
    );


    println!(
        "Compositor wallpaper-surface integration is not implemented yet."
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

