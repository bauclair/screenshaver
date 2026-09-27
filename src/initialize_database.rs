use std::fs;
use std::path::Path;

use rusqlite::{
    params,
    Connection,
    OpenFlags,
};


const SCHEMA_V001: &str =
    include_str!(
        "../assets/database/schema_v001.sql"
    );


pub fn initialize(
    database_path: &Path,
) -> Result<Connection, String> {

    if database_path.exists() {

        return Err(
            format!(
                "Refusing to initialize database because '{}' already exists",
                database_path.display(),
            )
        );
    }


    let mut connection =
        Connection::open_with_flags(
            database_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to create database '{}': {}",
                    database_path.display(),
                    error,
                )
            }
        )?;


    let initialization_result =
        (
            || -> Result<(), String> {

                crate::open_database::configure_connection(
                    &connection
                )?;


                initialize_contents(
                    &mut connection,
                    database_path,
                )
            }
        )();


    if let Err(error) =
        initialization_result
    {
        drop(
            connection
        );


        let cleanup_result =
            fs::remove_file(
                database_path
            );


        return match cleanup_result {

            Ok(()) => {
                Err(
                    error
                )
            }

            Err(cleanup_error) => {
                Err(
                    format!(
                        "{}; additionally unable to remove incomplete database '{}': {}",
                        error,
                        database_path.display(),
                        cleanup_error,
                    )
                )
            }
        };
    }


    Ok(
        connection
    )
}


fn initialize_contents(
    connection: &mut Connection,
    database_path: &Path,
) -> Result<(), String> {

    connection
        .execute_batch(
            SCHEMA_V001
        )
        .map_err(
            |error| {
                format!(
                    "Unable to initialize Schema Version 1 in '{}': {}",
                    database_path.display(),
                    error,
                )
            }
        )?;


    seed_textures(
        connection
    )?;


    seed_curated_palette(
        connection
    )?;


    create_app_defaults(
        connection
    )?;


    create_target_defaults(
        connection
    )?;


    let default_shader_id =
        crate::database_factory::register_default_shader(
            connection
        )?;


    let (
        default_screensaver_policy_id,
        default_wallpaper_policy_id,
    ) =
        create_default_policies(
            connection,
            default_shader_id,
        )?;


    create_runtime_targets(
        connection,
        default_screensaver_policy_id,
        default_wallpaper_policy_id,
    )?;


    insert_schema_metadata(
        connection
    )?;


    crate::validate_database::validate_initialization(
        connection
    )?;


    Ok(())
}


fn seed_textures(
    connection: &mut Connection,
) -> Result<(), String> {

    let transaction =
        connection
            .transaction()
            .map_err(
                |error| {
                    format!(
                        "Unable to begin texture-catalog initialization transaction: {}",
                        error,
                    )
                }
            )?;


    {
        let mut statement =
            transaction
                .prepare(
                    "INSERT INTO textures (
                         texture_name,
                         display_order
                     )
                     VALUES (?1, ?2)"
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to prepare texture-catalog insert statement: {}",
                            error,
                        )
                    }
                )?;


        for (
            display_order,
            family,
        ) in crate::generate_textures::TextureFamily::ALL
            .iter()
            .enumerate()
        {
            statement
                .execute(
                    params![
                        family.name(),
                        display_order as i64,
                    ]
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to insert texture family '{}': {}",
                            family.name(),
                            error,
                        )
                    }
                )?;
        }
    }


    let stored_count: i64 =
        transaction
            .query_row(
                "SELECT COUNT(*) FROM textures",
                [],
                |row| row.get(0),
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to verify texture-catalog row count: {}",
                        error,
                    )
                }
            )?;


    let expected_count =
        crate::generate_textures::TextureFamily::ALL.len() as i64;


    if stored_count != expected_count {
        return Err(
            format!(
                "Texture-catalog initialization verification failed: expected {} rows, found {}",
                expected_count,
                stored_count,
            )
        );
    }


    transaction
        .commit()
        .map_err(
            |error| {
                format!(
                    "Unable to commit texture-catalog initialization: {}",
                    error,
                )
            }
        )?;


    Ok(())
}


fn seed_curated_palette(
    connection: &mut Connection,
) -> Result<(), String> {

    let transaction =
        connection
            .transaction()
            .map_err(
                |error| {
                    format!(
                        "Unable to begin curated-palette initialization transaction: {}",
                        error,
                    )
                }
            )?;


    {
        let mut statement =
            transaction
                .prepare(
                    "INSERT INTO curated_palette (
                         color_hex,
                         description
                     )
                     VALUES (?1, ?2)"
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to prepare curated-palette insert statement: {}",
                            error,
                        )
                    }
                )?;


        for entry in
            crate::palettes::CURATED_PALETTE_COLORS
                .iter()
        {
            let color_hex =
                entry.color.to_hex();


            statement
                .execute(
                    params![
                        color_hex,
                        entry.name,
                    ]
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to insert curated palette color '{}' ({}): {}",
                            entry.name,
                            color_hex,
                            error,
                        )
                    }
                )?;
        }
    }


    let stored_count: i64 =
        transaction
            .query_row(
                "SELECT COUNT(*)
                 FROM curated_palette",
                [],
                |row| {
                    row.get(
                        0
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to verify curated-palette row count: {}",
                        error,
                    )
                }
            )?;


    let expected_count =
        crate::palettes::CURATED_PALETTE_COLORS
            .len() as i64;


    if stored_count
        != expected_count
    {
        return Err(
            format!(
                "Curated-palette initialization verification failed: expected {} rows, found {}",
                expected_count,
                stored_count,
            )
        );
    }


    transaction
        .commit()
        .map_err(
            |error| {
                format!(
                    "Unable to commit curated-palette initialization: {}",
                    error,
                )
            }
        )?;


    Ok(())
}


fn create_app_defaults(
    connection: &Connection,
) -> Result<(), String> {

    connection
        .execute(
            "INSERT INTO app_defaults (
                 defaults_id,
                 show_splash,
                 screensaver_subtitles,
                 subtitle_placement,
                 wallpaper_notifications,
                 lyrics_enabled,
                 wallpaper_display_format,
                 rendered_fps,
                 anti_aliasing,
                 dithering,
                 color_precision,
                 render_scale,
                 automatic_backups,
                 backup_interval_days,
                 last_backup
             )
             VALUES (
                 1,
                 1,
                 1,
                 'bottom:center',
                 1,
                 0,
                 'full_screen',
                 30,
                 'fxaa',
                 'subtle',
                 'auto',
                 1.0,
                 1,
                 7,
                 strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             )",
            [],
        )
        .map_err(
            |error| {
                format!(
                    "Unable to create initial application defaults: {}",
                    error,
                )
            }
        )?;


    Ok(())
}


fn create_target_defaults(
    connection: &Connection,
) -> Result<(), String> {

    let mut statement =
        connection
            .prepare(
                "INSERT INTO target_defaults (
                     target,
                     idle_timeout_value,
                     idle_timeout_unit,
                     animation_speed,
                     texture_mode,
                     texture_family,
                     texture_primitives,
                     palette_mode,
                     palette_color
                 )
                 VALUES (
                     ?1,
                     ?2,
                     ?3,
                     ?4,
                     'random',
                     NULL,
                     64,
                     'random',
                     NULL
                 )"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare initial target-default configuration: {}",
                        error,
                    )
                }
            )?;


    for (
        target,
        idle_timeout_value,
        idle_timeout_unit,
        animation_speed,
    ) in [
        (
            "screensaver",
            Some(10_i64),
            Some("minutes"),
            1.0_f64,
        ),
        (
            "wallpaper",
            None,
            None,
            0.03_f64,
        ),
    ] {
        statement
            .execute(
                params![
                    target,
                    idle_timeout_value,
                    idle_timeout_unit,
                    animation_speed,
                ]
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to create initial {} defaults: {}",
                        target,
                        error,
                    )
                }
            )?;
    }


    Ok(())
}


fn create_default_policies(
    connection: &Connection,
    shader_id: i64,
) -> Result<(i64, i64), String> {

    let screensaver_policy_id =
        insert_default_policy(
            connection,
            shader_id,
            "screensaver default",
            "screensaver default",
            "screensaver",
        )?;


    let wallpaper_policy_id =
        insert_default_policy(
            connection,
            shader_id,
            "wallpaper default",
            "wallpaper default",
            "wallpaper",
        )?;


    Ok(
        (
            screensaver_policy_id,
            wallpaper_policy_id,
        )
    )
}


fn insert_default_policy(
    connection: &Connection,
    shader_id: i64,
    policy_name: &str,
    policy_name_key: &str,
    policy_target: &str,
) -> Result<i64, String> {

    connection
        .execute(
            "INSERT INTO shader_policies (
                 policy_created_at,
                 policy_modified_at,
                 policy_name,
                 policy_name_key,
                 shader_id,
                 policy_target
             )
             VALUES (
                 strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                 ?1,
                 ?2,
                 ?3,
                 ?4
             )",
            params![
                policy_name,
                policy_name_key,
                shader_id,
                policy_target,
            ],
        )
        .map_err(
            |error| {
                format!(
                    "Unable to create initial policy '{}': {}",
                    policy_name,
                    error,
                )
            }
        )?;


    Ok(
        connection.last_insert_rowid()
    )
}


fn create_runtime_targets(
    connection: &Connection,
    screensaver_policy_id: i64,
    wallpaper_policy_id: i64,
) -> Result<(), String> {

    let mut statement =
        connection
            .prepare(
                "INSERT INTO runtime_targets (
                     target,
                     display_mode,
                     interval_seconds,
                     single_policy_id,
                     playlist_id
                 )
                 VALUES (
                     ?1,
                     'single',
                     NULL,
                     ?2,
                     NULL
                 )"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare initial runtime-target configuration: {}",
                        error,
                    )
                }
            )?;


    for (
        target,
        policy_id,
    ) in [
        (
            "screensaver",
            screensaver_policy_id,
        ),
        (
            "wallpaper",
            wallpaper_policy_id,
        ),
    ] {
        statement
            .execute(
                params![
                    target,
                    policy_id,
                ]
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to create initial {} runtime target using policy ID {}: {}",
                        target,
                        policy_id,
                        error,
                    )
                }
            )?;
    }


    Ok(())
}


fn insert_schema_metadata(
    connection: &Connection,
) -> Result<(), String> {

    let application_version =
        env!(
            "CARGO_PKG_VERSION"
        );


    connection
        .execute(
            "INSERT INTO schema_metadata (
                 metadata_id,
                 schema_version,
                 created_by_version,
                 last_migrated_by_version
             )
             VALUES (
                 1,
                 ?1,
                 ?2,
                 ?2
             )",
            params![
                crate::migrate_database::CURRENT_SCHEMA_VERSION,
                application_version,
            ],
        )
        .map_err(
            |error| {
                format!(
                    "Unable to finalize Schema Version {} metadata: {}",
                    crate::migrate_database::CURRENT_SCHEMA_VERSION,
                    error,
                )
            }
        )?;


    Ok(())
}
