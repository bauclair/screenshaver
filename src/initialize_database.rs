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


const RUNTIME_SOURCE_PREPARATION_VERSION: i64 = 1;


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


    seed_curated_palette(
        connection
    )?;


    let default_shader_id =
        register_default_shader(
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


fn register_default_shader(
    connection: &mut Connection,
) -> Result<i64, String> {

    let shader_path =
        crate::locate_paths::shader_dir()
            .join(
                "default.glsl"
            );


    let source_bytes =
        fs::read(
            &shader_path
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read default shader '{}': {}",
                    shader_path.display(),
                    error,
                )
            }
        )?;


    let source =
        String::from_utf8(
            source_bytes.clone()
        )
        .map_err(
            |error| {
                format!(
                    "Default shader '{}' is not valid UTF-8: {}",
                    shader_path.display(),
                    error,
                )
            }
        )?;


    let shader_kind =
        crate::classify_shader::classify_shader(
            &source
        );


    let shader_type =
        match shader_kind {

            crate::classify_shader::ShaderKind::NativeGLSL => {
                "native"
            }

            crate::classify_shader::ShaderKind::ShaderToy => {
                return Err(
                    format!(
                        "Default shader '{}' was unexpectedly classified as ShaderToy",
                        shader_path.display(),
                    )
                );
            }

            crate::classify_shader::ShaderKind::Isf => {
                return Err(
                    format!(
                        "Default shader '{}' was unexpectedly classified as ISF",
                        shader_path.display(),
                    )
                );
            }
        };


    let (
        _warnings,
        rejection_reasons,
    ) =
        crate::preprocess_shader::analyze_native_shader(
            &source
        );


    let channel_usage =
        crate::preprocess_shader::analyze_native_channel_usage(
            &source
        );


    let channel_usage_mask: i64 =
        (if channel_usage.channels[0] { 1 } else { 0 })
        | (if channel_usage.channels[1] { 1 << 1 } else { 0 })
        | (if channel_usage.channels[2] { 1 << 2 } else { 0 })
        | (if channel_usage.channels[3] { 1 << 3 } else { 0 })
        | (if channel_usage.requires_mipmaps { 1 << 4 } else { 0 });


    let shader_inputs_json =
        "[]";


    let (
        validation_status,
        validation_reason,
        validation_message,
    ) =
        if rejection_reasons.is_empty() {

            (
                "valid",
                None::<String>,
                None::<String>,
            )

        } else {

            (
                "rejected",
                Some(
                    "static_analysis_failed"
                        .to_string()
                ),
                Some(
                    rejection_reasons
                        .join(
                            "; "
                        )
                ),
            )
        };


    let source_hash =
        crate::hash_shader::hash_source(
            &source_bytes
        );


    let filename =
        shader_path
            .file_name()
            .and_then(
                |value| {
                    value.to_str()
                }
            )
            .ok_or_else(
                || {
                    format!(
                        "Default shader path '{}' has no valid UTF-8 filename",
                        shader_path.display(),
                    )
                }
            )?
            .to_string();


    let source_path =
        shader_path
            .parent()
            .ok_or_else(
                || {
                    format!(
                        "Default shader path '{}' has no parent directory",
                        shader_path.display(),
                    )
                }
            )?
            .to_string_lossy()
            .to_string();


    connection
        .execute(
            "INSERT INTO shaders (
                 filename,
                 source_path,
                 shader_type,
                 source_hash,
                 file_status,
                 validation_status,
                 validation_reason,
                 validation_message,
                 preprocessed_source,
                 preprocessor_version,
                 channel_usage_mask,
                 shader_inputs_json
             )
             VALUES (
                 ?1,
                 ?2,
                 ?3,
                 ?4,
                 'present',
                 ?5,
                 ?6,
                 ?7,
                 ?8,
                 ?9,
                 ?10,
                 ?11
             )",
            params![
                filename,
                source_path,
                shader_type,
                source_hash,
                validation_status,
                validation_reason,
                validation_message,
                source_bytes,
                RUNTIME_SOURCE_PREPARATION_VERSION,
                channel_usage_mask,
                shader_inputs_json,
            ],
        )
        .map_err(
            |error| {
                format!(
                    "Unable to register default shader '{}': {}",
                    shader_path.display(),
                    error,
                )
            }
        )?;


    Ok(
        connection.last_insert_rowid()
    )
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
                 policy_name,
                 policy_name_key,
                 shader_id,
                 policy_target
             )
             VALUES (
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
                     single_policy_id
                 )
                 VALUES (
                     ?1,
                     'single',
                     NULL,
                     ?2
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
                 1,
                 ?1,
                 ?1
             )",
            params![
                application_version,
            ],
        )
        .map_err(
            |error| {
                format!(
                    "Unable to finalize Schema Version 1 metadata: {}",
                    error,
                )
            }
        )?;


    Ok(())
}
