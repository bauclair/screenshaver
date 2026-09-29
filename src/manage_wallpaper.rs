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


            crate::parse_mode::ModeType::Playlist => {

                let playlist_id =
                    parsed_mode.argument
                        .parse::<i64>()
                        .map_err(
                            |error| {
                                format!(
                                    "Invalid wallpaper Playlist ID '{}' in mode '{}': {}",
                                    parsed_mode.argument,
                                    configured_mode,
                                    error,
                                )
                            }
                        )?;

                if playlist_id <= 0 {
                    return Err(
                        format!(
                            "Invalid wallpaper Playlist ID '{}' in mode '{}'; expected a positive integer",
                            playlist_id,
                            configured_mode,
                        )
                    );
                }

                let interval_text =
                    configured_mode
                        .split(':')
                        .nth(2)
                        .ok_or_else(
                            || {
                                format!(
                                    "Invalid wallpaper Playlist mode '{}'; expected playlist:<playlist_id>:<seconds>",
                                    configured_mode,
                                )
                            }
                        )?;

                let interval_seconds =
                    interval_text
                        .parse::<u64>()
                        .map_err(
                            |error| {
                                format!(
                                    "Invalid wallpaper Playlist interval '{}' in mode '{}': {}",
                                    interval_text,
                                    configured_mode,
                                    error,
                                )
                            }
                        )?;

                if interval_seconds == 0 {
                    return Err(
                        format!(
                            "Invalid wallpaper Playlist interval in mode '{}'; expected a positive number of seconds",
                            configured_mode,
                        )
                    );
                }

                (
                    crate::manage_shader::ShaderMode::Playlist(
                        playlist_id
                    ),
                    Some(
                        std::time::Duration::from_secs(
                            interval_seconds
                        )
                    ),
                )
            }


            crate::parse_mode::ModeType::Invalid => {
                return Err(
                    format!(
                        "Invalid wallpaper mode '{}'; expected single:<shader>, random:<seconds>, ordered:<seconds>, or playlist:<playlist_id>:<seconds>",
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


    let version =
        env!(
            "CARGO_PKG_VERSION"
        );

    println!(
        "{}",
        crate::manage_localization::runtime_text_with_params(
            "wallpaper.cli.version",
            &[
                (
                    "version",
                    version,
                )
            ],
        )
    );


    println!(
        "{}",
        crate::manage_localization::runtime_text(
            "wallpaper.cli.configuration_heading"
        )
    );


    println!(
        "{}",
        crate::manage_localization::runtime_text_with_params(
            "wallpaper.cli.shader_mode",
            &[
                (
                    "mode",
                    configured_mode,
                )
            ],
        )
    );


    println!(
        "{}",
        crate::manage_localization::runtime_text_with_params(
            "wallpaper.cli.monitor_mode",
            &[
                (
                    "mode",
                    runtime.monitor_mode.name(),
                )
            ],
        )
    );


    let animation_speed =
        format!(
            "{:.3}",
            runtime.animation_speed_policy.global_speed
        );

    println!(
        "{}",
        crate::manage_localization::runtime_text_with_params(
            "wallpaper.cli.animation_speed",
            &[
                (
                    "speed",
                    animation_speed.as_str(),
                )
            ],
        )
    );


    let notification_state =
        if runtime.notifications {
            crate::manage_localization::runtime_text(
                "common.enabled"
            )
        } else {
            crate::manage_localization::runtime_text(
                "common.disabled"
            )
        };

    println!(
        "{}",
        crate::manage_localization::runtime_text_with_params(
            "wallpaper.cli.notifications",
            &[
                (
                    "state",
                    notification_state.as_str(),
                )
            ],
        )
    );


    let wallpaper_directory_text =
        wallpaper_directory
            .display()
            .to_string();

    println!(
        "{}",
        crate::manage_localization::runtime_text_with_params(
            "wallpaper.cli.directory",
            &[
                (
                    "path",
                    wallpaper_directory_text.as_str(),
                )
            ],
        )
    );


    println!();


    let eligible_count =
        shader_entries.len()
            .to_string();

    println!(
        "{}",
        crate::manage_localization::runtime_text_with_params(
            "wallpaper.cli.eligible_count",
            &[
                (
                    "count",
                    eligible_count.as_str(),
                )
            ],
        )
    );


    if shader_entries.is_empty() {

        println!(
            "{}",
            crate::manage_localization::runtime_text(
                "wallpaper.cli.no_eligible_shaders"
            )
        );


        println!();


        println!(
            "{}",
            crate::manage_localization::runtime_text(
                "wallpaper.cli.not_started"
            )
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
                "{}",
                crate::manage_localization::runtime_text(
                    "wallpaper.cli.rotation_disabled_single_shader"
                )
            );

            None
        } else {
            shader_interval
        };


    let shader_manager =
        crate::manage_shader::ShaderManager::from_shader_entries_for_target(
            shader_mode,
            shader_entries,
            crate::manage_shader::OrderedTarget::Wallpaper,
        );


    let backend =
        crate::wallpaper_backend::create_backend()?;


    backend.report_capabilities();


    // Lyrics are currently a Windowpaper-only feature. Keep the manager at
    // this common lifecycle level so MPRIS/provider logic remains independent
    // of the native Wayland and X11 presentation backends.
    let _lyrics_manager =
        if runtime.lyrics_enabled
            && runtime.display_format
                == crate::manage_configuration::WallpaperDisplayFormat::Windowed
        {
            match crate::manage_lyrics::LyricsManager::start() {
                Ok(manager) => {
                    println!(
                        "{}",
                        crate::manage_localization::runtime_text(
                            "wallpaper.cli.lyrics_enabled_windowpaper"
                        )
                    );
                    Some(manager)
                }

                Err(error) => {
                    eprintln!(
                        "[LYRICS] Unable to start synchronized lyrics manager; Windowpaper will continue without lyrics: {}",
                        error,
                    );

                    crate::logger::warning(
                        &crate::locate_paths::runtime_log_path(),
                        &format!(
                            "[LYRICS] Unable to start synchronized lyrics manager; Windowpaper will continue without lyrics: {}",
                            error,
                        ),
                    );

                    None
                }
            }
        } else {
            None
        };


    let mut presentation_runtime =
        runtime.clone();

    presentation_runtime.lyrics_state =
        _lyrics_manager
            .as_ref()
            .map(
                |manager| {
                    manager.shared_state()
                }
            );

    backend.run(
        shader_manager,
        &wallpaper_directory,
        shader_interval,
        &presentation_runtime,
        running,
        control,
    )?;


    Ok(())
}
