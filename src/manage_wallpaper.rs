use crate::define_wallpaper::WallpaperRuntime;


pub fn load_shader_entries(
) -> Result<
    Vec<crate::manage_shader::ShaderEntry>,
    String,
> {

    let managed_source_path =
        crate::locate_paths::shader_dir()
            .to_string_lossy()
            .to_string();


    let mut shader_entries =
        Vec::new();


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database for wallpaper shader discovery: {}",
                        error,
                    )
                }
            )?;


    let mut statement =
        connection
            .prepare(
                "SELECT
                     p.policy_id,
                     s.filename,
                     s.source_path,
                     p.policy_name
                 FROM shader_policies AS p
                 JOIN shaders AS s
                   ON s.shader_id = p.shader_id
                 WHERE s.source_path = ?1
                   AND s.file_status = 'present'
                   AND p.policy_target = 'wallpaper'
                 ORDER BY s.filename COLLATE NOCASE,
                          s.filename,
                          p.policy_name COLLATE NOCASE,
                          p.policy_name"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare wallpaper shader discovery query: {}",
                        error,
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                rusqlite::params![
                    managed_source_path
                ],
                |row| {
                    Ok(
                        (
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                        )
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query wallpaper shader discovery rows: {}",
                        error,
                    )
                }
            )?;


    for row in rows {

        let (
            policy_id,
            filename,
            source_path,
            policy_name,
        ) =
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode wallpaper shader discovery row: {}",
                        error,
                    )
                }
            )?;


        shader_entries.push(
            crate::manage_shader::ShaderEntry::with_policy_id_source_path(
                policy_id,
                filename.clone(),
                policy_name,
                std::path::PathBuf::from(
                    source_path
                )
                .join(
                    filename
                ),
            )
        );
    }


    let config_path =
        crate::locate_paths::config_path();


    match crate::manage_policies::external_policy_entries(
        &config_path,
        crate::manage_policies::PolicyTarget::Wallpaper,
    ) {
        Ok(external_paths) => {

            for (
                policy_id,
                policy_name,
                name,
                source_path,
            ) in external_paths
            {
                if !source_path.is_file() {
                    eprintln!(
                        "[WALLPAPER] External wallpaper shader '{}' is unavailable: {}",
                        name,
                        source_path.display(),
                    );

                    continue;
                }


                shader_entries.push(
                    crate::manage_shader::ShaderEntry::with_policy_id_source_path(
                        policy_id,
                        name,
                        policy_name,
                        source_path,
                    )
                );
            }
        }


        Err(error) => {
            return Err(
                format!(
                    "Unable to load external wallpaper shader paths: {}",
                    error,
                )
            );
        }
    }


    shader_entries.sort_by(
        |left, right| {
            left.name.cmp(
                &right.name
            )
            .then_with(
                || {
                    left.policy_name.cmp(
                        &right.policy_name
                    )
                }
            )
            .then_with(
                || {
                    left.source_path.cmp(
                        &right.source_path
                    )
                }
            )
            .then_with(
                || {
                    left.policy_id.cmp(
                        &right.policy_id
                    )
                }
            )
        }
    );


    Ok(
        shader_entries
    )
}

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


    let shader_entries =
        load_shader_entries()?;


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
        "Eligible wallpaper shaders: {}",
        shader_entries.len()
    );


    if shader_entries.is_empty() {

        println!(
            "    No present shaders with a Wallpaper policy were found."
        );


        println!();


        println!(
            "Wallpaper rendering was not started."
        );


        return Ok(());
    }


    for shader_entry in
        &shader_entries
    {
        println!(
            "    {}",
            shader_entry.name
        );
    }


    let shader_interval =
        if shader_entries.len() <= 1
            && shader_interval.is_some()
        {
            println!();

            println!(
                "Wallpaper rotation disabled: only one eligible shader is available."
            );

            None
        } else {
            shader_interval
        };


    let shader_manager =
        crate::manage_shader::ShaderManager::from_shader_entries(
            shader_mode,
            shader_entries,
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
