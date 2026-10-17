//! Historical reader for Screenshaver Database Schema Version 2.
//!
//! This module is permanent compatibility code.  It understands the released
//! Schema-2 SQLite representation and translates its durable semantics into
//! storage-independent MigrationData.
//!
//! The reader never modifies the supplied database connection.

use std::collections::HashMap;

use rusqlite::Connection;

use super::migration_data::{
    MigrationApplicationDefaults,
    MigrationData,
    MigrationDefaultPalette,
    MigrationDefaultTexture,
    MigrationDisplayMode,
    MigrationIdleTimeout,
    MigrationPalette,
    MigrationPlaylist,
    MigrationPlaylistId,
    MigrationPolicy,
    MigrationPolicyId,
    MigrationPolicyTarget,
    MigrationRuntimeConfiguration,
    MigrationShader,
    MigrationShaderId,
    MigrationSource,
    MigrationTargetDefault,
    MigrationTargetDefaults,
    MigrationTexture,
};


const SCHEMA_VERSION: i64 = 2;


pub fn read(
    connection: &Connection,
) -> Result<MigrationData, String> {

    let source =
        read_source_metadata(
            connection,
        )?;

    let (
        shaders,
        shader_ids,
    ) =
        read_shaders(
            connection,
        )?;

    let (
        policies,
        policy_ids,
    ) =
        read_policies(
            connection,
            &shader_ids,
        )?;

    let (
        playlists,
        playlist_ids,
    ) =
        read_playlists(
            connection,
            &policy_ids,
        )?;

    let runtime_configuration =
        read_runtime_configuration(
            connection,
            &policy_ids,
            &playlist_ids,
        )?;

    let application_defaults =
        read_application_defaults(
            connection,
        )?;

    let target_defaults =
        read_target_defaults(
            connection,
        )?;

    Ok(
        MigrationData {
            source,
            shaders,
            policies,
            playlists,
            runtime_configuration,
            application_defaults,
            target_defaults,
        }
    )
}


fn read_source_metadata(
    connection: &Connection,
) -> Result<MigrationSource, String> {

    let row_count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM schema_metadata",
                [],
                |row| {
                    row.get(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to count Schema-2 metadata rows: {}",
                        error,
                    )
                }
            )?;

    if row_count
        != 1
    {
        return Err(
            format!(
                "Schema-2 reader expected exactly one schema_metadata row, found {}",
                row_count,
            )
        );
    }

    let (
        metadata_id,
        schema_version,
        created_by_version,
        last_migrated_by_version,
    ): (
        i64,
        i64,
        String,
        String,
    ) =
        connection
            .query_row(
                "SELECT
                     metadata_id,
                     schema_version,
                     created_by_version,
                     last_migrated_by_version
                 FROM schema_metadata",
                [],
                |row| {
                    Ok(
                        (
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                        )
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to read Schema-2 metadata: {}",
                        error,
                    )
                }
            )?;

    if metadata_id
        != 1
    {
        return Err(
            format!(
                "Schema-2 reader expected metadata_id 1, found {}",
                metadata_id,
            )
        );
    }

    if schema_version
        != SCHEMA_VERSION
    {
        return Err(
            format!(
                "Schema-2 reader cannot read database schema version {}; expected {}",
                schema_version,
                SCHEMA_VERSION,
            )
        );
    }

    Ok(
        MigrationSource {
            schema_version,
            created_by_version,
            last_migrated_by_version,
        }
    )
}


fn read_shaders(
    connection: &Connection,
) -> Result<
    (
        Vec<MigrationShader>,
        HashMap<i64, MigrationShaderId>,
    ),
    String,
> {

    let mut statement =
        connection
            .prepare(
                "SELECT
                     shader_id,
                     shader_added_at,
                     filename,
                     source_path
                 FROM shaders
                 ORDER BY shader_id"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare Schema-2 shader read: {}",
                        error,
                    )
                }
            )?;

    let mut rows =
        statement
            .query([])
            .map_err(
                |error| {
                    format!(
                        "Unable to query Schema-2 shaders: {}",
                        error,
                    )
                }
            )?;

    let mut shaders =
        Vec::new();

    let mut shader_ids =
        HashMap::new();

    while let Some(row) =
        rows
            .next()
            .map_err(
                |error| {
                    format!(
                        "Unable to advance through Schema-2 shaders: {}",
                        error,
                    )
                }
            )?
    {
        let source_shader_id: i64 =
            row
                .get(0)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read Schema-2 shader_id: {}",
                            error,
                        )
                    }
                )?;

        let migration_id =
            MigrationShaderId(
                positive_u64(
                    source_shader_id,
                    "shader_id",
                )?
            );

        if shader_ids
            .insert(
                source_shader_id,
                migration_id,
            )
            .is_some()
        {
            return Err(
                format!(
                    "Schema-2 reader encountered duplicate shader_id {}",
                    source_shader_id,
                )
            );
        }

        let added_at: String =
            row
                .get(1)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read shader_added_at for Schema-2 shader {}: {}",
                            source_shader_id,
                            error,
                        )
                    }
                )?;

        let filename: String =
            row
                .get(2)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read filename for Schema-2 shader {}: {}",
                            source_shader_id,
                            error,
                        )
                    }
                )?;

        let source_path: String =
            row
                .get(3)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read source_path for Schema-2 shader {}: {}",
                            source_shader_id,
                            error,
                        )
                    }
                )?;

        shaders.push(
            MigrationShader {
                migration_id,
                filename,
                source_path,
                added_at:
                    Some(
                        added_at
                    ),
            }
        );
    }

    Ok(
        (
            shaders,
            shader_ids,
        )
    )
}


fn read_policies(
    connection: &Connection,
    shader_ids: &HashMap<i64, MigrationShaderId>,
) -> Result<
    (
        Vec<MigrationPolicy>,
        HashMap<i64, MigrationPolicyId>,
    ),
    String,
> {

    let mut statement =
        connection
            .prepare(
                "SELECT
                     policy_id,
                     policy_created_at,
                     policy_modified_at,
                     policy_name,
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
                 FROM shader_policies
                 ORDER BY policy_id"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare Schema-2 policy read: {}",
                        error,
                    )
                }
            )?;

    let mut rows =
        statement
            .query([])
            .map_err(
                |error| {
                    format!(
                        "Unable to query Schema-2 policies: {}",
                        error,
                    )
                }
            )?;

    let mut policies =
        Vec::new();

    let mut policy_ids =
        HashMap::new();

    while let Some(row) =
        rows
            .next()
            .map_err(
                |error| {
                    format!(
                        "Unable to advance through Schema-2 policies: {}",
                        error,
                    )
                }
            )?
    {
        let source_policy_id: i64 =
            row
                .get(0)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read Schema-2 policy_id: {}",
                            error,
                        )
                    }
                )?;

        let migration_id =
            MigrationPolicyId(
                positive_u64(
                    source_policy_id,
                    "policy_id",
                )?
            );

        if policy_ids
            .insert(
                source_policy_id,
                migration_id,
            )
            .is_some()
        {
            return Err(
                format!(
                    "Schema-2 reader encountered duplicate policy_id {}",
                    source_policy_id,
                )
            );
        }

        let source_shader_id: i64 =
            row
                .get(4)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read shader_id for Schema-2 policy {}: {}",
                            source_policy_id,
                            error,
                        )
                    }
                )?;

        let shader =
            shader_ids
                .get(
                    &source_shader_id
                )
                .copied()
                .ok_or_else(
                    || {
                        format!(
                            "Schema-2 policy {} references missing shader_id {}",
                            source_policy_id,
                            source_shader_id,
                        )
                    }
                )?;

        let target_text: String =
            row
                .get(5)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read policy_target for Schema-2 policy {}: {}",
                            source_policy_id,
                            error,
                        )
                    }
                )?;

        let texture_mode: Option<String> =
            row
                .get(6)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read texture_mode for Schema-2 policy {}: {}",
                            source_policy_id,
                            error,
                        )
                    }
                )?;

        let texture_family: Option<String> =
            row
                .get(7)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read texture_family for Schema-2 policy {}: {}",
                            source_policy_id,
                            error,
                        )
                    }
                )?;

        let texture_primitives: Option<i64> =
            row
                .get(8)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read texture_primitives for Schema-2 policy {}: {}",
                            source_policy_id,
                            error,
                        )
                    }
                )?;

        let palette_mode: Option<String> =
            row
                .get(9)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read palette_mode for Schema-2 policy {}: {}",
                            source_policy_id,
                            error,
                        )
                    }
                )?;

        let palette_color: Option<String> =
            row
                .get(10)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read palette_color for Schema-2 policy {}: {}",
                            source_policy_id,
                            error,
                        )
                    }
                )?;

        let rendered_fps =
            optional_u32(
                row
                    .get::<_, Option<i64>>(11)
                    .map_err(
                        |error| {
                            format!(
                                "Unable to read rendered_fps for Schema-2 policy {}: {}",
                                source_policy_id,
                                error,
                            )
                        }
                    )?,
                "rendered_fps",
            )?;

        policies.push(
            MigrationPolicy {
                migration_id,
                shader,

                created_at:
                    Some(
                        row
                            .get(1)
                            .map_err(
                                |error| {
                                    format!(
                                        "Unable to read policy_created_at for Schema-2 policy {}: {}",
                                        source_policy_id,
                                        error,
                                    )
                                }
                            )?
                    ),

                modified_at:
                    Some(
                        row
                            .get(2)
                            .map_err(
                                |error| {
                                    format!(
                                        "Unable to read policy_modified_at for Schema-2 policy {}: {}",
                                        source_policy_id,
                                        error,
                                    )
                                }
                            )?
                    ),

                policy_name:
                    row
                        .get(3)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read policy_name for Schema-2 policy {}: {}",
                                    source_policy_id,
                                    error,
                                )
                            }
                        )?,

                target:
                    parse_policy_target(
                        &target_text,
                        source_policy_id,
                    )?,

                texture:
                    parse_policy_texture(
                        texture_mode,
                        texture_family,
                        texture_primitives,
                        source_policy_id,
                    )?,

                palette:
                    parse_policy_palette(
                        palette_mode,
                        palette_color,
                        source_policy_id,
                    )?,

                rendered_fps,

                animation_speed:
                    row
                        .get(12)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read animation_speed for Schema-2 policy {}: {}",
                                    source_policy_id,
                                    error,
                                )
                            }
                        )?,

                starting_offset_seconds:
                    row
                        .get(13)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read starting_offset for Schema-2 policy {}: {}",
                                    source_policy_id,
                                    error,
                                )
                            }
                        )?,

                anti_aliasing:
                    row
                        .get(14)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read anti_aliasing for Schema-2 policy {}: {}",
                                    source_policy_id,
                                    error,
                                )
                            }
                        )?,

                dithering:
                    row
                        .get(15)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read dithering for Schema-2 policy {}: {}",
                                    source_policy_id,
                                    error,
                                )
                            }
                        )?,

                color_precision:
                    row
                        .get(16)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read color_precision for Schema-2 policy {}: {}",
                                    source_policy_id,
                                    error,
                                )
                            }
                        )?,

                render_scale:
                    row
                        .get(17)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read render_scale for Schema-2 policy {}: {}",
                                    source_policy_id,
                                    error,
                                )
                            }
                        )?,

                audiovisual_effect:
                    row
                        .get(18)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read audiovisual_effect for Schema-2 policy {}: {}",
                                    source_policy_id,
                                    error,
                                )
                            }
                        )?,

                audio_motion_effect:
                    row
                        .get(19)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read audio_motion_effect for Schema-2 policy {}: {}",
                                    source_policy_id,
                                    error,
                                )
                            }
                        )?,

                bloom_intensity:
                    row
                        .get(20)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read bloom_intensity for Schema-2 policy {}: {}",
                                    source_policy_id,
                                    error,
                                )
                            }
                        )?,

                bloom_saturation:
                    row
                        .get(21)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read bloom_saturation for Schema-2 policy {}: {}",
                                    source_policy_id,
                                    error,
                                )
                            }
                        )?,

                bloom_threshold:
                    row
                        .get(22)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read bloom_threshold for Schema-2 policy {}: {}",
                                    source_policy_id,
                                    error,
                                )
                            }
                        )?,

                bloom_frequency_rotation:
                    row
                        .get(23)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read bloom_frequency_rotation for Schema-2 policy {}: {}",
                                    source_policy_id,
                                    error,
                                )
                            }
                        )?,

                bloom_frequency_invert:
                    read_bool(
                        row,
                        24,
                        "bloom_frequency_invert",
                        source_policy_id,
                    )?,

                invert_colors:
                    read_bool(
                        row,
                        25,
                        "invert_colors",
                        source_policy_id,
                    )?,

                flip_horizontal:
                    read_bool(
                        row,
                        26,
                        "flip_horizontal",
                        source_policy_id,
                    )?,

                flip_vertical:
                    read_bool(
                        row,
                        27,
                        "flip_vertical",
                        source_policy_id,
                    )?,

                hue_rotation:
                    row
                        .get(28)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read hue_rotation for Schema-2 policy {}: {}",
                                    source_policy_id,
                                    error,
                                )
                            }
                        )?,
            }
        );
    }

    Ok(
        (
            policies,
            policy_ids,
        )
    )
}


fn read_playlists(
    connection: &Connection,
    policy_ids: &HashMap<i64, MigrationPolicyId>,
) -> Result<
    (
        Vec<MigrationPlaylist>,
        HashMap<i64, MigrationPlaylistId>,
    ),
    String,
> {

    let mut statement =
        connection
            .prepare(
                "SELECT
                     playlist_id,
                     playlist_created_at,
                     playlist_modified_at,
                     playlist_name,
                     description
                 FROM playlists
                 ORDER BY playlist_id"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare Schema-2 playlist read: {}",
                        error,
                    )
                }
            )?;

    let mut rows =
        statement
            .query([])
            .map_err(
                |error| {
                    format!(
                        "Unable to query Schema-2 playlists: {}",
                        error,
                    )
                }
            )?;

    let mut playlists =
        Vec::new();

    let mut playlist_ids =
        HashMap::new();

    while let Some(row) =
        rows
            .next()
            .map_err(
                |error| {
                    format!(
                        "Unable to advance through Schema-2 playlists: {}",
                        error,
                    )
                }
            )?
    {
        let source_playlist_id: i64 =
            row
                .get(0)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read Schema-2 playlist_id: {}",
                            error,
                        )
                    }
                )?;

        let migration_id =
            MigrationPlaylistId(
                positive_u64(
                    source_playlist_id,
                    "playlist_id",
                )?
            );

        if playlist_ids
            .insert(
                source_playlist_id,
                migration_id,
            )
            .is_some()
        {
            return Err(
                format!(
                    "Schema-2 reader encountered duplicate playlist_id {}",
                    source_playlist_id,
                )
            );
        }

        let members =
            read_playlist_members(
                connection,
                source_playlist_id,
                policy_ids,
            )?;

        playlists.push(
            MigrationPlaylist {
                migration_id,

                created_at:
                    Some(
                        row
                            .get(1)
                            .map_err(
                                |error| {
                                    format!(
                                        "Unable to read playlist_created_at for Schema-2 playlist {}: {}",
                                        source_playlist_id,
                                        error,
                                    )
                                }
                            )?
                    ),

                modified_at:
                    Some(
                        row
                            .get(2)
                            .map_err(
                                |error| {
                                    format!(
                                        "Unable to read playlist_modified_at for Schema-2 playlist {}: {}",
                                        source_playlist_id,
                                        error,
                                    )
                                }
                            )?
                    ),

                playlist_name:
                    row
                        .get(3)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read playlist_name for Schema-2 playlist {}: {}",
                                    source_playlist_id,
                                    error,
                                )
                            }
                        )?,

                description:
                    row
                        .get(4)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read description for Schema-2 playlist {}: {}",
                                    source_playlist_id,
                                    error,
                                )
                            }
                        )?,

                members,
            }
        );
    }

    Ok(
        (
            playlists,
            playlist_ids,
        )
    )
}


fn read_playlist_members(
    connection: &Connection,
    source_playlist_id: i64,
    policy_ids: &HashMap<i64, MigrationPolicyId>,
) -> Result<Vec<MigrationPolicyId>, String> {

    let mut statement =
        connection
            .prepare(
                "SELECT
                     policy_id,
                     position
                 FROM playlist_members
                 WHERE playlist_id = ?1
                 ORDER BY position"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare Schema-2 playlist-member read for playlist {}: {}",
                        source_playlist_id,
                        error,
                    )
                }
            )?;

    let mut rows =
        statement
            .query(
                [source_playlist_id]
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query Schema-2 members for playlist {}: {}",
                        source_playlist_id,
                        error,
                    )
                }
            )?;

    let mut members =
        Vec::new();

    // Schema 2 requires playlist positions to be positive and unique, but it
    // does not require them to be contiguous.  Position is ordering metadata;
    // MigrationData preserves the semantic member order rather than the
    // historical numeric position values themselves.
    while let Some(row) =
        rows
            .next()
            .map_err(
                |error| {
                    format!(
                        "Unable to advance through Schema-2 members for playlist {}: {}",
                        source_playlist_id,
                        error,
                    )
                }
            )?
    {
        let source_policy_id: i64 =
            row
                .get(0)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read policy_id from Schema-2 playlist {}: {}",
                            source_playlist_id,
                            error,
                        )
                    }
                )?;

        let _position: i64 =
            row
                .get(1)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read position from Schema-2 playlist {}: {}",
                            source_playlist_id,
                            error,
                        )
                    }
                )?;

        let migration_policy_id =
            policy_ids
                .get(
                    &source_policy_id
                )
                .copied()
                .ok_or_else(
                    || {
                        format!(
                            "Schema-2 playlist {} references missing policy_id {}",
                            source_playlist_id,
                            source_policy_id,
                        )
                    }
                )?;

        members.push(
            migration_policy_id
        );

    }

    Ok(
        members
    )
}


fn read_runtime_configuration(
    connection: &Connection,
    policy_ids: &HashMap<i64, MigrationPolicyId>,
    playlist_ids: &HashMap<i64, MigrationPlaylistId>,
) -> Result<MigrationRuntimeConfiguration, String> {

    let row_count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM runtime_targets",
                [],
                |row| {
                    row.get(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to count Schema-2 runtime targets: {}",
                        error,
                    )
                }
            )?;

    if row_count
        != 2
    {
        return Err(
            format!(
                "Schema-2 reader expected exactly two runtime_targets rows, found {}",
                row_count,
            )
        );
    }

    Ok(
        MigrationRuntimeConfiguration {
            screensaver:
                read_runtime_target(
                    connection,
                    "screensaver",
                    policy_ids,
                    playlist_ids,
                )?,

            wallpaper:
                read_runtime_target(
                    connection,
                    "wallpaper",
                    policy_ids,
                    playlist_ids,
                )?,
        }
    )
}


fn read_runtime_target(
    connection: &Connection,
    target: &str,
    policy_ids: &HashMap<i64, MigrationPolicyId>,
    playlist_ids: &HashMap<i64, MigrationPlaylistId>,
) -> Result<MigrationDisplayMode, String> {

    let (
        display_mode,
        interval_seconds,
        single_policy_id,
        playlist_id,
    ): (
        String,
        Option<i64>,
        Option<i64>,
        Option<i64>,
    ) =
        connection
            .query_row(
                "SELECT
                     display_mode,
                     interval_seconds,
                     single_policy_id,
                     playlist_id
                 FROM runtime_targets
                 WHERE target = ?1",
                [target],
                |row| {
                    Ok(
                        (
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                        )
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to read Schema-2 runtime target '{}': {}",
                        target,
                        error,
                    )
                }
            )?;

    match display_mode
        .as_str()
    {
        "single" => {
            if interval_seconds
                    .is_some()
                || playlist_id
                    .is_some()
            {
                return Err(
                    format!(
                        "Schema-2 runtime target '{}' has invalid Single-mode field combination",
                        target,
                    )
                );
            }

            let policy =
                map_optional_policy_id(
                    single_policy_id,
                    policy_ids,
                    target,
                )?;

            Ok(
                MigrationDisplayMode::Single {
                    policy,
                }
            )
        }

        "ordered" => {
            if single_policy_id
                    .is_some()
                || playlist_id
                    .is_some()
            {
                return Err(
                    format!(
                        "Schema-2 runtime target '{}' has invalid Ordered-mode field combination",
                        target,
                    )
                );
            }

            Ok(
                MigrationDisplayMode::Ordered {
                    interval_seconds:
                        required_positive_u64(
                            interval_seconds,
                            "interval_seconds",
                            target,
                        )?,
                }
            )
        }

        "random" => {
            if single_policy_id
                    .is_some()
                || playlist_id
                    .is_some()
            {
                return Err(
                    format!(
                        "Schema-2 runtime target '{}' has invalid Random-mode field combination",
                        target,
                    )
                );
            }

            Ok(
                MigrationDisplayMode::Random {
                    interval_seconds:
                        required_positive_u64(
                            interval_seconds,
                            "interval_seconds",
                            target,
                        )?,
                }
            )
        }

        "playlist" => {
            if single_policy_id
                .is_some()
            {
                return Err(
                    format!(
                        "Schema-2 runtime target '{}' has invalid Playlist-mode field combination",
                        target,
                    )
                );
            }

            let playlist =
                map_optional_playlist_id(
                    playlist_id,
                    playlist_ids,
                    target,
                )?;

            Ok(
                MigrationDisplayMode::Playlist {
                    playlist,

                    interval_seconds:
                        required_positive_u64(
                            interval_seconds,
                            "interval_seconds",
                            target,
                        )?,
                }
            )
        }

        _ => {
            Err(
                format!(
                    "Schema-2 runtime target '{}' contains unsupported display_mode '{}'",
                    target,
                    display_mode,
                )
            )
        }
    }
}


fn read_application_defaults(
    connection: &Connection,
) -> Result<MigrationApplicationDefaults, String> {

    let row_count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM app_defaults",
                [],
                |row| {
                    row.get(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to count Schema-2 app_defaults rows: {}",
                        error,
                    )
                }
            )?;

    if row_count
        != 1
    {
        return Err(
            format!(
                "Schema-2 reader expected exactly one app_defaults row, found {}",
                row_count,
            )
        );
    }

    connection
        .query_row(
            "SELECT
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
             FROM app_defaults",
            [],
            |row| {
                Ok(
                    (
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, String>(10)?,
                        row.get::<_, f64>(11)?,
                        row.get::<_, i64>(12)?,
                        row.get::<_, i64>(13)?,
                        row.get::<_, String>(14)?,
                    )
                )
            },
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read Schema-2 application defaults: {}",
                    error,
                )
            }
        )
        .and_then(
            |(
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
                last_backup,
            )| {
                if defaults_id
                    != 1
                {
                    return Err(
                        format!(
                            "Schema-2 reader expected app_defaults defaults_id 1, found {}",
                            defaults_id,
                        )
                    );
                }

                Ok(
                    MigrationApplicationDefaults {
                        show_splash:
                            integer_bool(
                                show_splash,
                                "app_defaults.show_splash",
                            )?,

                        screensaver_subtitles:
                            integer_bool(
                                screensaver_subtitles,
                                "app_defaults.screensaver_subtitles",
                            )?,

                        subtitle_placement,

                        wallpaper_notifications:
                            integer_bool(
                                wallpaper_notifications,
                                "app_defaults.wallpaper_notifications",
                            )?,

                        lyrics_enabled:
                            integer_bool(
                                lyrics_enabled,
                                "app_defaults.lyrics_enabled",
                            )?,

                        wallpaper_display_format,

                        rendered_fps:
                            positive_u32(
                                rendered_fps,
                                "app_defaults.rendered_fps",
                            )?,

                        anti_aliasing,
                        dithering,
                        color_precision,
                        render_scale,

                        automatic_backups:
                            integer_bool(
                                automatic_backups,
                                "app_defaults.automatic_backups",
                            )?,

                        backup_interval_days:
                            positive_u32(
                                backup_interval_days,
                                "app_defaults.backup_interval_days",
                            )?,

                        last_backup:
                            Some(
                                last_backup
                            ),
                    }
                )
            }
        )
}


fn read_target_defaults(
    connection: &Connection,
) -> Result<MigrationTargetDefaults, String> {

    let row_count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM target_defaults",
                [],
                |row| {
                    row.get(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to count Schema-2 target_defaults rows: {}",
                        error,
                    )
                }
            )?;

    if row_count
        != 2
    {
        return Err(
            format!(
                "Schema-2 reader expected exactly two target_defaults rows, found {}",
                row_count,
            )
        );
    }

    Ok(
        MigrationTargetDefaults {
            screensaver:
                read_target_default(
                    connection,
                    "screensaver",
                )?,

            wallpaper:
                read_target_default(
                    connection,
                    "wallpaper",
                )?,
        }
    )
}


fn read_target_default(
    connection: &Connection,
    target: &str,
) -> Result<MigrationTargetDefault, String> {

    let (
        idle_timeout_value,
        idle_timeout_unit,
        animation_speed,
        texture_mode,
        texture_family,
        texture_primitives,
        palette_mode,
        palette_color,
    ): (
        Option<i64>,
        Option<String>,
        f64,
        String,
        Option<String>,
        i64,
        String,
        Option<String>,
    ) =
        connection
            .query_row(
                "SELECT
                     idle_timeout_value,
                     idle_timeout_unit,
                     animation_speed,
                     texture_mode,
                     texture_family,
                     texture_primitives,
                     palette_mode,
                     palette_color
                 FROM target_defaults
                 WHERE target = ?1",
                [target],
                |row| {
                    Ok(
                        (
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                            row.get(6)?,
                            row.get(7)?,
                        )
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to read Schema-2 target defaults for '{}': {}",
                        target,
                        error,
                    )
                }
            )?;

    let idle_timeout =
        match target
        {
            "screensaver" => {
                match (
                    idle_timeout_value,
                    idle_timeout_unit,
                )
                {
                    (
                        Some(value),
                        Some(unit),
                    ) => {
                        MigrationIdleTimeout::Value {
                            value:
                                positive_u64(
                                    value,
                                    "target_defaults.idle_timeout_value",
                                )?,

                            unit,
                        }
                    }

                    _ => {
                        return Err(
                            "Schema-2 screensaver target defaults require both idle-timeout value and unit"
                                .to_string()
                        );
                    }
                }
            }

            "wallpaper" => {
                if idle_timeout_value
                        .is_some()
                    || idle_timeout_unit
                        .is_some()
                {
                    return Err(
                        "Schema-2 wallpaper target defaults must not contain an idle timeout"
                            .to_string()
                    );
                }

                MigrationIdleTimeout::NotApplicable
            }

            _ => {
                return Err(
                    format!(
                        "Schema-2 reader does not recognize target-default target '{}'",
                        target,
                    )
                );
            }
        };

    let texture_primitives =
        positive_u32(
            texture_primitives,
            "target_defaults.texture_primitives",
        )?;

    let texture =
        match texture_mode
            .as_str()
        {
            "random" => {
                if texture_family
                    .is_some()
                {
                    return Err(
                        format!(
                            "Schema-2 target defaults for '{}' use Random texture mode but contain a texture family",
                            target,
                        )
                    );
                }

                MigrationDefaultTexture::Random
            }

            "specific" => {
                let family =
                    texture_family
                        .ok_or_else(
                            || {
                                format!(
                                    "Schema-2 target defaults for '{}' use Specific texture mode without a texture family",
                                    target,
                                )
                            }
                        )?;

                MigrationDefaultTexture::Specific {
                    family,
                    primitives:
                        texture_primitives,
                }
            }

            _ => {
                return Err(
                    format!(
                        "Schema-2 target defaults for '{}' contain unsupported texture_mode '{}'",
                        target,
                        texture_mode,
                    )
                );
            }
        };

    let palette =
        match palette_mode
            .as_str()
        {
            "random" => {
                if palette_color
                    .is_some()
                {
                    return Err(
                        format!(
                            "Schema-2 target defaults for '{}' use Random palette mode but contain a palette color",
                            target,
                        )
                    );
                }

                MigrationDefaultPalette::Random
            }

            "specific" => {
                let color =
                    palette_color
                        .ok_or_else(
                            || {
                                format!(
                                    "Schema-2 target defaults for '{}' use Specific palette mode without a palette color",
                                    target,
                                )
                            }
                        )?;

                MigrationDefaultPalette::Specific {
                    color,
                }
            }

            _ => {
                return Err(
                    format!(
                        "Schema-2 target defaults for '{}' contain unsupported palette_mode '{}'",
                        target,
                        palette_mode,
                    )
                );
            }
        };

    Ok(
        MigrationTargetDefault {
            idle_timeout,
            animation_speed,
            texture,
            texture_primitives,
            palette,
        }
    )
}


fn parse_policy_target(
    value: &str,
    source_policy_id: i64,
) -> Result<MigrationPolicyTarget, String> {

    match value
    {
        "unassigned" => {
            Ok(
                MigrationPolicyTarget::Unassigned
            )
        }

        "screensaver" => {
            Ok(
                MigrationPolicyTarget::Screensaver
            )
        }

        "wallpaper" => {
            Ok(
                MigrationPolicyTarget::Wallpaper
            )
        }

        _ => {
            Err(
                format!(
                    "Schema-2 policy {} contains unsupported policy_target '{}'",
                    source_policy_id,
                    value,
                )
            )
        }
    }
}


fn parse_policy_texture(
    mode: Option<String>,
    family: Option<String>,
    primitives: Option<i64>,
    source_policy_id: i64,
) -> Result<MigrationTexture, String> {

    match mode
        .as_deref()
    {
        None => {
            if family
                    .is_some()
                || primitives
                    .is_some()
            {
                return Err(
                    format!(
                        "Schema-2 policy {} inherits texture settings but contains texture-specific values",
                        source_policy_id,
                    )
                );
            }

            Ok(
                MigrationTexture::Inherit
            )
        }

        Some("random") => {
            if family
                    .is_some()
                || primitives
                    .is_some()
            {
                return Err(
                    format!(
                        "Schema-2 policy {} uses Random texture mode but contains texture-specific values",
                        source_policy_id,
                    )
                );
            }

            Ok(
                MigrationTexture::Random
            )
        }

        Some("specific") => {
            let family =
                family
                    .ok_or_else(
                        || {
                            format!(
                                "Schema-2 policy {} uses Specific texture mode without a texture family",
                                source_policy_id,
                            )
                        }
                    )?;

            let primitives =
                primitives
                    .ok_or_else(
                        || {
                            format!(
                                "Schema-2 policy {} uses Specific texture mode without a primitive count",
                                source_policy_id,
                            )
                        }
                    )?;

            Ok(
                MigrationTexture::Specific {
                    family,

                    primitives:
                        positive_u32(
                            primitives,
                            "shader_policies.texture_primitives",
                        )?,
                }
            )
        }

        Some(other) => {
            Err(
                format!(
                    "Schema-2 policy {} contains unsupported texture_mode '{}'",
                    source_policy_id,
                    other,
                )
            )
        }
    }
}


fn parse_policy_palette(
    mode: Option<String>,
    color: Option<String>,
    source_policy_id: i64,
) -> Result<MigrationPalette, String> {

    match mode
        .as_deref()
    {
        None => {
            if color
                .is_some()
            {
                return Err(
                    format!(
                        "Schema-2 policy {} inherits palette settings but contains a palette color",
                        source_policy_id,
                    )
                );
            }

            Ok(
                MigrationPalette::Inherit
            )
        }

        Some("random") => {
            if color
                .is_some()
            {
                return Err(
                    format!(
                        "Schema-2 policy {} uses Random palette mode but contains a palette color",
                        source_policy_id,
                    )
                );
            }

            Ok(
                MigrationPalette::Random
            )
        }

        Some("specific") => {
            let color =
                color
                    .ok_or_else(
                        || {
                            format!(
                                "Schema-2 policy {} uses Specific palette mode without a palette color",
                                source_policy_id,
                            )
                        }
                    )?;

            Ok(
                MigrationPalette::Specific {
                    color,
                }
            )
        }

        Some(other) => {
            Err(
                format!(
                    "Schema-2 policy {} contains unsupported palette_mode '{}'",
                    source_policy_id,
                    other,
                )
            )
        }
    }
}


fn map_optional_policy_id(
    source_policy_id: Option<i64>,
    policy_ids: &HashMap<i64, MigrationPolicyId>,
    target: &str,
) -> Result<Option<MigrationPolicyId>, String> {

    source_policy_id
        .map(
            |source_policy_id| {
                policy_ids
                    .get(
                        &source_policy_id
                    )
                    .copied()
                    .ok_or_else(
                        || {
                            format!(
                                "Schema-2 runtime target '{}' references missing policy_id {}",
                                target,
                                source_policy_id,
                            )
                        }
                    )
            }
        )
        .transpose()
}


fn map_optional_playlist_id(
    source_playlist_id: Option<i64>,
    playlist_ids: &HashMap<i64, MigrationPlaylistId>,
    target: &str,
) -> Result<Option<MigrationPlaylistId>, String> {

    source_playlist_id
        .map(
            |source_playlist_id| {
                playlist_ids
                    .get(
                        &source_playlist_id
                    )
                    .copied()
                    .ok_or_else(
                        || {
                            format!(
                                "Schema-2 runtime target '{}' references missing playlist_id {}",
                                target,
                                source_playlist_id,
                            )
                        }
                    )
            }
        )
        .transpose()
}


fn required_positive_u64(
    value: Option<i64>,
    field_name: &str,
    target: &str,
) -> Result<u64, String> {

    let value =
        value
            .ok_or_else(
                || {
                    format!(
                        "Schema-2 runtime target '{}' requires {}",
                        target,
                        field_name,
                    )
                }
            )?;

    positive_u64(
        value,
        field_name,
    )
}


fn positive_u64(
    value: i64,
    field_name: &str,
) -> Result<u64, String> {

    if value
        <= 0
    {
        return Err(
            format!(
                "Schema-2 {} must be positive, found {}",
                field_name,
                value,
            )
        );
    }

    u64::try_from(
        value
    )
    .map_err(
        |_| {
            format!(
                "Schema-2 {} value {} cannot be represented by the migration model",
                field_name,
                value,
            )
        }
    )
}


fn positive_u32(
    value: i64,
    field_name: &str,
) -> Result<u32, String> {

    if value
        <= 0
    {
        return Err(
            format!(
                "Schema-2 {} must be positive, found {}",
                field_name,
                value,
            )
        );
    }

    u32::try_from(
        value
    )
    .map_err(
        |_| {
            format!(
                "Schema-2 {} value {} cannot be represented by the migration model",
                field_name,
                value,
            )
        }
    )
}


fn optional_u32(
    value: Option<i64>,
    field_name: &str,
) -> Result<Option<u32>, String> {

    value
        .map(
            |value| {
                positive_u32(
                    value,
                    field_name,
                )
            }
        )
        .transpose()
}


fn integer_bool(
    value: i64,
    field_name: &str,
) -> Result<bool, String> {

    match value
    {
        0 => {
            Ok(
                false
            )
        }

        1 => {
            Ok(
                true
            )
        }

        _ => {
            Err(
                format!(
                    "Schema-2 {} must be 0 or 1, found {}",
                    field_name,
                    value,
                )
            )
        }
    }
}


fn read_bool(
    row: &rusqlite::Row<'_>,
    column_index: usize,
    field_name: &str,
    source_policy_id: i64,
) -> Result<bool, String> {

    let value: i64 =
        row
            .get(column_index)
            .map_err(
                |error| {
                    format!(
                        "Unable to read {} for Schema-2 policy {}: {}",
                        field_name,
                        source_policy_id,
                        error,
                    )
                }
            )?;

    integer_bool(
        value,
        &format!(
            "shader_policies.{}",
            field_name,
        ),
    )
}
