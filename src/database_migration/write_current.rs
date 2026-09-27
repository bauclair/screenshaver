//! Current-schema reconstruction writer.
//!
//! Historical readers produce storage-independent MigrationData.  This module
//! consumes that representation and constructs a brand-new database using the
//! schema expected by the current Screenshaver executable.
//!
//! Important safety boundary:
//! - the destination path must not already exist;
//! - the source database is never opened or modified here;
//! - local SQLite IDs are newly allocated and migration-local relationships
//!   are explicitly remapped;
//! - derived shader runtime data is not copied from historical databases.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use rusqlite::{
    params,
    Connection,
    OpenFlags,
};

use super::migration_data::{
    MigrationApplicationDefaults,
    MigrationData,
    MigrationDefaultPalette,
    MigrationDefaultTexture,
    MigrationDisplayMode,
    MigrationIdleTimeout,
    MigrationPalette,
    MigrationPlaylistId,
    MigrationPolicyId,
    MigrationPolicyTarget,
    MigrationShaderId,
    MigrationTargetDefault,
    MigrationTexture,
};


const CURRENT_SCHEMA_SQL: &str =
    include_str!(
        "../../assets/database/schema_v002.sql"
    );


pub fn write(
    destination_path: &Path,
    data: &MigrationData,
) -> Result<Connection, String> {

    if destination_path.exists() {
        return Err(
            format!(
                "Refusing to reconstruct database because destination already exists: {}",
                destination_path.display(),
            )
        );
    }

    let mut connection =
        Connection::open_with_flags(
            destination_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to create reconstruction database '{}': {}",
                    destination_path.display(),
                    error,
                )
            }
        )?;

    let result =
        (|| -> Result<(), String> {
            crate::open_database::configure_connection(
                &connection
            )?;

            connection
                .execute_batch(
                    CURRENT_SCHEMA_SQL
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to create current database schema in '{}': {}",
                            destination_path.display(),
                            error,
                        )
                    }
                )?;

            seed_factory_catalogs(
                &mut connection
            )?;

            let factory_default_shader =
                register_factory_default_shader_if_present(
                    &mut connection,
                    data,
                )?;

            let transaction =
                connection
                    .transaction()
                    .map_err(
                        |error| {
                            format!(
                                "Unable to begin reconstruction transaction: {}",
                                error,
                            )
                        }
                    )?;

            let shader_ids =
                write_shaders(
                    &transaction,
                    data,
                    factory_default_shader,
                )?;

            let policy_ids =
                write_policies(
                    &transaction,
                    data,
                    &shader_ids,
                )?;

            let playlist_ids =
                write_playlists(
                    &transaction,
                    data,
                    &policy_ids,
                )?;

            write_application_defaults(
                &transaction,
                &data.application_defaults,
            )?;

            write_target_default(
                &transaction,
                "screensaver",
                &data.target_defaults.screensaver,
            )?;

            write_target_default(
                &transaction,
                "wallpaper",
                &data.target_defaults.wallpaper,
            )?;

            write_runtime_target(
                &transaction,
                "screensaver",
                &data.runtime_configuration.screensaver,
                &policy_ids,
                &playlist_ids,
            )?;

            write_runtime_target(
                &transaction,
                "wallpaper",
                &data.runtime_configuration.wallpaper,
                &policy_ids,
                &playlist_ids,
            )?;

            write_schema_metadata(
                &transaction
            )?;

            transaction
                .commit()
                .map_err(
                    |error| {
                        format!(
                            "Unable to commit reconstructed current database: {}",
                            error,
                        )
                    }
                )?;

            crate::validate_database::validate_startup(
                &connection
            )?;

            crate::validate_database::validate_integrity(
                &connection
            )?;

            validate_reconstruction_counts(
                &connection,
                data,
            )?;

            Ok(())
        })();

    if let Err(error) = result {
        drop(
            connection
        );

        let cleanup_result =
            fs::remove_file(
                destination_path
            );

        return match cleanup_result {
            Ok(()) => {
                Err(error)
            }

            Err(cleanup_error) => {
                Err(
                    format!(
                        "{}; additionally unable to remove failed reconstruction database '{}': {}",
                        error,
                        destination_path.display(),
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


fn seed_factory_catalogs(
    connection: &mut Connection,
) -> Result<(), String> {

    let transaction =
        connection
            .transaction()
            .map_err(
                |error| {
                    format!(
                        "Unable to begin factory-catalog reconstruction transaction: {}",
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
                            "Unable to prepare reconstructed texture-catalog insert: {}",
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
                            "Unable to reconstruct texture family '{}': {}",
                            family.name(),
                            error,
                        )
                    }
                )?;
        }
    }

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
                            "Unable to prepare reconstructed curated-palette insert: {}",
                            error,
                        )
                    }
                )?;

        for entry in
            crate::palettes::CURATED_PALETTE_COLORS
                .iter()
        {
            statement
                .execute(
                    params![
                        entry.color.to_hex(),
                        entry.name,
                    ]
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to reconstruct curated palette entry '{}': {}",
                            entry.name,
                            error,
                        )
                    }
                )?;
        }
    }

    transaction
        .commit()
        .map_err(
            |error| {
                format!(
                    "Unable to commit reconstructed factory catalogs: {}",
                    error,
                )
            }
        )?;


    crate::database_factory::seed_localization_catalog(
        connection
    )
}


fn register_factory_default_shader_if_present(
    connection: &mut Connection,
    data: &MigrationData,
) -> Result<Option<(MigrationShaderId, i64)>, String> {

    let default_shaders =
        data.shaders
            .iter()
            .filter(
                |shader| {
                    shader.filename
                        == "default.glsl"
                }
            )
            .collect::<Vec<_>>();

    match default_shaders.as_slice() {
        [] => {
            Ok(None)
        }

        [shader] => {
            let destination_id =
                crate::database_factory::register_default_shader(
                    connection
                )?;

            Ok(
                Some(
                    (
                        shader.migration_id,
                        destination_id,
                    )
                )
            )
        }

        _ => {
            Err(
                format!(
                    "MigrationData contains {} default.glsl shader records; exactly one factory default shader is permitted",
                    default_shaders.len(),
                )
            )
        }
    }
}


fn write_shaders(
    transaction: &rusqlite::Transaction<'_>,
    data: &MigrationData,
    factory_default_shader: Option<(MigrationShaderId, i64)>,
) -> Result<HashMap<MigrationShaderId, i64>, String> {

    let mut ids =
        HashMap::new();

    if let Some(
        (
            migration_id,
            destination_id,
        )
    ) = factory_default_shader
    {
        ids.insert(
            migration_id,
            destination_id,
        );
    }

    let mut statement =
        transaction
            .prepare(
                "INSERT INTO shaders (
                     shader_added_at,
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
                     COALESCE(?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                     ?2,
                     ?3,
                     'unknown',
                     NULL,
                     'missing',
                     'unknown',
                     NULL,
                     NULL,
                     NULL,
                     NULL,
                     NULL,
                     NULL
                 )"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare reconstructed shader insert: {}",
                        error,
                    )
                }
            )?;

    for shader in &data.shaders {
        if shader.filename
            == "default.glsl"
        {
            continue;
        }

        statement
            .execute(
                params![
                    shader.added_at.as_deref(),
                    shader.filename,
                    shader.source_path,
                ]
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to reconstruct shader '{}' from '{}': {}",
                        shader.filename,
                        shader.source_path,
                        error,
                    )
                }
            )?;

        let destination_id =
            transaction
                .last_insert_rowid();

        if ids
            .insert(
                shader.migration_id,
                destination_id,
            )
            .is_some()
        {
            return Err(
                format!(
                    "MigrationData contains duplicate shader migration ID {}",
                    shader.migration_id.0,
                )
            );
        }
    }

    if ids.len()
        != data.shaders.len()
    {
        return Err(
            format!(
                "MigrationData shader-ID mapping is incomplete: mapped {}, expected {}",
                ids.len(),
                data.shaders.len(),
            )
        );
    }

    Ok(
        ids
    )
}


fn write_policies(
    transaction: &rusqlite::Transaction<'_>,
    data: &MigrationData,
    shader_ids: &HashMap<MigrationShaderId, i64>,
) -> Result<HashMap<MigrationPolicyId, i64>, String> {

    let mut ids =
        HashMap::new();

    let mut statement =
        transaction
            .prepare(
                "INSERT INTO shader_policies (
                     policy_created_at,
                     policy_modified_at,
                     policy_name,
                     policy_name_key,
                     shader_id,
                     policy_target,
                     texture_mode,
                     texture_family,
                     texture_primitives,
                     palette_mode,
                     palette_color,
                     rendered_fps,
                     animation_speed,
                     starting_offset,
                     anti_aliasing,
                     dithering,
                     color_precision,
                     render_scale,
                     audiovisual_effect,
                     audio_motion_effect,
                     bloom_intensity,
                     bloom_saturation,
                     bloom_threshold,
                     bloom_frequency_rotation,
                     bloom_frequency_invert,
                     invert_colors,
                     flip_horizontal,
                     flip_vertical,
                     hue_rotation
                 )
                 VALUES (
                     COALESCE(?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                     COALESCE(?2, COALESCE(?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))),
                     ?3, ?4, ?5, ?6,
                     ?7, ?8, ?9,
                     ?10, ?11,
                     ?12, ?13, ?14,
                     ?15, ?16, ?17, ?18,
                     ?19, ?20,
                     ?21, ?22, ?23, ?24, ?25,
                     ?26, ?27, ?28, ?29
                 )"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare reconstructed policy insert: {}",
                        error,
                    )
                }
            )?;

    for policy in &data.policies {
        let shader_id =
            shader_ids
                .get(
                    &policy.shader
                )
                .copied()
                .ok_or_else(
                    || {
                        format!(
                            "Policy '{}' references unknown shader migration ID {}",
                            policy.policy_name,
                            policy.shader.0,
                        )
                    }
                )?;

        let (
            texture_mode,
            texture_family,
            texture_primitives,
        ) =
            match &policy.texture {
                MigrationTexture::Inherit => {
                    (None, None, None)
                }

                MigrationTexture::Random => {
                    (
                        Some("random".to_string()),
                        None,
                        None,
                    )
                }

                MigrationTexture::Specific {
                    family,
                    primitives,
                } => {
                    (
                        Some("specific".to_string()),
                        Some(family.clone()),
                        Some(i64::from(*primitives)),
                    )
                }
            };

        let (
            palette_mode,
            palette_color,
        ) =
            match &policy.palette {
                MigrationPalette::Inherit => {
                    (None, None)
                }

                MigrationPalette::Random => {
                    (
                        Some("random".to_string()),
                        None,
                    )
                }

                MigrationPalette::Specific {
                    color,
                } => {
                    (
                        Some("specific".to_string()),
                        Some(color.clone()),
                    )
                }
            };

        let target =
            match policy.target {
                MigrationPolicyTarget::Unassigned => {
                    "unassigned"
                }

                MigrationPolicyTarget::Screensaver => {
                    "screensaver"
                }

                MigrationPolicyTarget::Wallpaper => {
                    "wallpaper"
                }
            };

        let policy_name =
            policy.policy_name.trim();

        let policy_name_key =
            comparison_key(
                policy_name,
                "Policy Name",
            )?;

        statement
            .execute(
                params![
                    policy.created_at.as_deref(),
                    policy.modified_at.as_deref(),
                    policy_name,
                    policy_name_key,
                    shader_id,
                    target,
                    texture_mode,
                    texture_family,
                    texture_primitives,
                    palette_mode,
                    palette_color,
                    policy.rendered_fps.map(i64::from),
                    policy.animation_speed,
                    policy.starting_offset_seconds,
                    policy.anti_aliasing.as_deref(),
                    policy.dithering.as_deref(),
                    policy.color_precision.as_deref(),
                    policy.render_scale,
                    policy.audiovisual_effect,
                    policy.audio_motion_effect,
                    policy.bloom_intensity,
                    policy.bloom_saturation,
                    policy.bloom_threshold,
                    policy.bloom_frequency_rotation,
                    bool_integer(policy.bloom_frequency_invert),
                    bool_integer(policy.invert_colors),
                    bool_integer(policy.flip_horizontal),
                    bool_integer(policy.flip_vertical),
                    policy.hue_rotation,
                ]
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to reconstruct policy '{}': {}",
                        policy.policy_name,
                        error,
                    )
                }
            )?;

        let destination_id =
            transaction
                .last_insert_rowid();

        if ids
            .insert(
                policy.migration_id,
                destination_id,
            )
            .is_some()
        {
            return Err(
                format!(
                    "MigrationData contains duplicate policy migration ID {}",
                    policy.migration_id.0,
                )
            );
        }
    }

    Ok(
        ids
    )
}


fn write_playlists(
    transaction: &rusqlite::Transaction<'_>,
    data: &MigrationData,
    policy_ids: &HashMap<MigrationPolicyId, i64>,
) -> Result<HashMap<MigrationPlaylistId, i64>, String> {

    let mut ids =
        HashMap::new();

    for playlist in &data.playlists {
        let playlist_name =
            playlist.playlist_name.trim();

        let playlist_name_key =
            comparison_key(
                playlist_name,
                "Playlist Name",
            )?;

        transaction
            .execute(
                "INSERT INTO playlists (
                     playlist_created_at,
                     playlist_modified_at,
                     playlist_name,
                     playlist_name_key,
                     description
                 )
                 VALUES (
                     COALESCE(?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                     COALESCE(?2, COALESCE(?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))),
                     ?3,
                     ?4,
                     ?5
                 )",
                params![
                    playlist.created_at.as_deref(),
                    playlist.modified_at.as_deref(),
                    playlist_name,
                    playlist_name_key,
                    playlist.description.as_deref(),
                ],
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to reconstruct playlist '{}': {}",
                        playlist.playlist_name,
                        error,
                    )
                }
            )?;

        let destination_playlist_id =
            transaction
                .last_insert_rowid();

        if ids
            .insert(
                playlist.migration_id,
                destination_playlist_id,
            )
            .is_some()
        {
            return Err(
                format!(
                    "MigrationData contains duplicate playlist migration ID {}",
                    playlist.migration_id.0,
                )
            );
        }

        for (
            index,
            migration_policy_id,
        ) in playlist.members
            .iter()
            .enumerate()
        {
            let destination_policy_id =
                policy_ids
                    .get(
                        migration_policy_id
                    )
                    .copied()
                    .ok_or_else(
                        || {
                            format!(
                                "Playlist '{}' references unknown policy migration ID {}",
                                playlist.playlist_name,
                                migration_policy_id.0,
                            )
                        }
                    )?;

            transaction
                .execute(
                    "INSERT INTO playlist_members (
                         playlist_id,
                         policy_id,
                         position
                     )
                     VALUES (?1, ?2, ?3)",
                    params![
                        destination_playlist_id,
                        destination_policy_id,
                        (index + 1) as i64,
                    ],
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to reconstruct member {} of playlist '{}': {}",
                            index + 1,
                            playlist.playlist_name,
                            error,
                        )
                    }
                )?;
        }
    }

    Ok(
        ids
    )
}


fn write_application_defaults(
    transaction: &rusqlite::Transaction<'_>,
    defaults: &MigrationApplicationDefaults,
) -> Result<(), String> {

    transaction
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
                 ?1, ?2, ?3, ?4, ?5, ?6,
                 ?7, ?8, ?9, ?10, ?11,
                 ?12, ?13,
                 COALESCE(?14, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
             )",
            params![
                bool_integer(defaults.show_splash),
                bool_integer(defaults.screensaver_subtitles),
                defaults.subtitle_placement,
                bool_integer(defaults.wallpaper_notifications),
                bool_integer(defaults.lyrics_enabled),
                defaults.wallpaper_display_format,
                i64::from(defaults.rendered_fps),
                defaults.anti_aliasing,
                defaults.dithering,
                defaults.color_precision,
                defaults.render_scale,
                bool_integer(defaults.automatic_backups),
                i64::from(defaults.backup_interval_days),
                defaults.last_backup.as_deref(),
            ],
        )
        .map_err(
            |error| {
                format!(
                    "Unable to reconstruct application defaults: {}",
                    error,
                )
            }
        )?;

    Ok(())
}


fn write_target_default(
    transaction: &rusqlite::Transaction<'_>,
    target_name: &str,
    defaults: &MigrationTargetDefault,
) -> Result<(), String> {

    let (
        idle_timeout_value,
        idle_timeout_unit,
    ) =
        match &defaults.idle_timeout {
            MigrationIdleTimeout::NotApplicable => {
                (None, None)
            }

            MigrationIdleTimeout::Value {
                value,
                unit,
            } => {
                (
                    Some(
                        i64::try_from(*value)
                            .map_err(
                                |_| {
                                    format!(
                                        "{} idle-timeout value {} exceeds SQLite integer range",
                                        target_name,
                                        value,
                                    )
                                }
                            )?
                    ),
                    Some(
                        unit.as_str()
                    ),
                )
            }
        };

    let (
        texture_mode,
        texture_family,
    ) =
        match &defaults.texture {
            MigrationDefaultTexture::Random => {
                (
                    "random",
                    None,
                )
            }

            MigrationDefaultTexture::Specific {
                family,
                primitives,
            } => {
                if *primitives
                    != defaults.texture_primitives
                {
                    return Err(
                        format!(
                            "{} target default contains conflicting texture primitive counts: {} and {}",
                            target_name,
                            primitives,
                            defaults.texture_primitives,
                        )
                    );
                }

                (
                    "specific",
                    Some(
                        family.as_str()
                    ),
                )
            }
        };

    let (
        palette_mode,
        palette_color,
    ) =
        match &defaults.palette {
            MigrationDefaultPalette::Random => {
                (
                    "random",
                    None,
                )
            }

            MigrationDefaultPalette::Specific {
                color,
            } => {
                (
                    "specific",
                    Some(
                        color.as_str()
                    ),
                )
            }
        };

    transaction
        .execute(
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
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9
             )",
            params![
                target_name,
                idle_timeout_value,
                idle_timeout_unit,
                defaults.animation_speed,
                texture_mode,
                texture_family,
                i64::from(defaults.texture_primitives),
                palette_mode,
                palette_color,
            ],
        )
        .map_err(
            |error| {
                format!(
                    "Unable to reconstruct {} target defaults: {}",
                    target_name,
                    error,
                )
            }
        )?;

    Ok(())
}


fn write_runtime_target(
    transaction: &rusqlite::Transaction<'_>,
    target_name: &str,
    mode: &MigrationDisplayMode,
    policy_ids: &HashMap<MigrationPolicyId, i64>,
    playlist_ids: &HashMap<MigrationPlaylistId, i64>,
) -> Result<(), String> {

    let (
        display_mode,
        interval_seconds,
        single_policy_id,
        playlist_id,
    ) =
        match mode {
            MigrationDisplayMode::Single {
                policy,
            } => {
                let single_policy_id =
                    match policy {
                        Some(migration_id) => {
                            Some(
                                policy_ids
                                    .get(
                                        migration_id
                                    )
                                    .copied()
                                    .ok_or_else(
                                        || {
                                            format!(
                                                "{} runtime target references unknown policy migration ID {}",
                                                target_name,
                                                migration_id.0,
                                            )
                                        }
                                    )?
                            )
                        }

                        None => {
                            None
                        }
                    };

                (
                    "single",
                    None,
                    single_policy_id,
                    None,
                )
            }

            MigrationDisplayMode::Ordered {
                interval_seconds,
            } => {
                (
                    "ordered",
                    Some(
                        sqlite_u64(
                            *interval_seconds,
                            "ordered interval",
                        )?
                    ),
                    None,
                    None,
                )
            }

            MigrationDisplayMode::Random {
                interval_seconds,
            } => {
                (
                    "random",
                    Some(
                        sqlite_u64(
                            *interval_seconds,
                            "random interval",
                        )?
                    ),
                    None,
                    None,
                )
            }

            MigrationDisplayMode::Playlist {
                playlist,
                interval_seconds,
            } => {
                let playlist_id =
                    match playlist {
                        Some(migration_id) => {
                            Some(
                                playlist_ids
                                    .get(
                                        migration_id
                                    )
                                    .copied()
                                    .ok_or_else(
                                        || {
                                            format!(
                                                "{} runtime target references unknown playlist migration ID {}",
                                                target_name,
                                                migration_id.0,
                                            )
                                        }
                                    )?
                            )
                        }

                        None => {
                            None
                        }
                    };

                (
                    "playlist",
                    Some(
                        sqlite_u64(
                            *interval_seconds,
                            "playlist interval",
                        )?
                    ),
                    None,
                    playlist_id,
                )
            }
        };

    transaction
        .execute(
            "INSERT INTO runtime_targets (
                 target,
                 display_mode,
                 interval_seconds,
                 single_policy_id,
                 playlist_id
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                target_name,
                display_mode,
                interval_seconds,
                single_policy_id,
                playlist_id,
            ],
        )
        .map_err(
            |error| {
                format!(
                    "Unable to reconstruct {} runtime target: {}",
                    target_name,
                    error,
                )
            }
        )?;

    Ok(())
}


fn write_schema_metadata(
    transaction: &rusqlite::Transaction<'_>,
) -> Result<(), String> {

    transaction
        .execute(
            "INSERT INTO schema_metadata (
                 metadata_id,
                 schema_version,
                 created_by_version,
                 last_migrated_by_version
             )
             VALUES (1, ?1, ?2, ?2)",
            params![
                crate::migrate_database::CURRENT_SCHEMA_VERSION,
                env!("CARGO_PKG_VERSION"),
            ],
        )
        .map_err(
            |error| {
                format!(
                    "Unable to write reconstructed schema metadata: {}",
                    error,
                )
            }
        )?;

    Ok(())
}


fn validate_reconstruction_counts(
    connection: &Connection,
    data: &MigrationData,
) -> Result<(), String> {

    validate_count(
        connection,
        "shaders",
        data.shaders.len(),
    )?;

    validate_count(
        connection,
        "shader_policies",
        data.policies.len(),
    )?;

    validate_count(
        connection,
        "playlists",
        data.playlists.len(),
    )?;

    let expected_memberships =
        data.playlists
            .iter()
            .map(
                |playlist| {
                    playlist.members.len()
                }
            )
            .sum::<usize>();

    validate_count(
        connection,
        "playlist_members",
        expected_memberships,
    )?;

    validate_count(
        connection,
        "runtime_targets",
        2,
    )?;

    validate_count(
        connection,
        "app_defaults",
        1,
    )?;

    validate_count(
        connection,
        "target_defaults",
        2,
    )?;

    Ok(())
}


fn validate_count(
    connection: &Connection,
    table_name: &str,
    expected: usize,
) -> Result<(), String> {

    let sql =
        format!(
            "SELECT COUNT(*) FROM {}",
            table_name,
        );

    let actual: i64 =
        connection
            .query_row(
                &sql,
                [],
                |row| {
                    row.get(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to validate reconstructed {} row count: {}",
                        table_name,
                        error,
                    )
                }
            )?;

    if actual
        != expected as i64
    {
        return Err(
            format!(
                "Reconstructed {} row count mismatch: expected {}, found {}",
                table_name,
                expected,
                actual,
            )
        );
    }

    Ok(())
}


fn comparison_key(
    value: &str,
    label: &str,
) -> Result<String, String> {

    let value =
        value.trim();

    let length =
        value
            .chars()
            .count();

    if !(1..=128)
        .contains(
            &length
        )
    {
        return Err(
            format!(
                "{} must contain between 1 and 128 characters; found {}",
                label,
                length,
            )
        );
    }

    let key =
        value
            .chars()
            .flat_map(
                |character| {
                    character.to_lowercase()
                }
            )
            .collect::<String>();

    if key.is_empty() {
        return Err(
            format!(
                "{} produced an empty comparison key",
                label,
            )
        );
    }

    Ok(
        key
    )
}


fn bool_integer(
    value: bool,
) -> i64 {

    if value {
        1
    } else {
        0
    }
}


fn sqlite_u64(
    value: u64,
    label: &str,
) -> Result<i64, String> {

    i64::try_from(
        value
    )
    .map_err(
        |_| {
            format!(
                "{} value {} exceeds SQLite integer range",
                label,
                value,
            )
        }
    )
}
