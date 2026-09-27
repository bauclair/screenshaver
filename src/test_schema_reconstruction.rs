//! Developer diagnostic for reconstruction migration.
//!
//! This command opens a selected historical Schema-1 database read-only,
//! translates it to MigrationData, reconstructs a separate current-schema
//! database, reads that reconstructed database back through the Schema-1
//! historical reader, and compares durable semantics.
//!
//! It deliberately bypasses normal Screenshaver database evaluation,
//! initialization, shader reconciliation, and policy assignment.

use std::fs;
use std::path::{
    Path,
    PathBuf,
};

use rusqlite::{
    Connection,
    OpenFlags,
};

use crate::database_migration::migration_data::{
    MigrationData,
    MigrationDefaultPalette,
    MigrationDefaultTexture,
    MigrationDisplayMode,
    MigrationIdleTimeout,
    MigrationPalette,
    MigrationPolicyTarget,
    MigrationTexture,
};


pub fn run(
    source_path: &str,
    destination_path: &str,
) -> Result<(), String> {

    let source_path =
        PathBuf::from(
            source_path
        );

    let destination_path =
        PathBuf::from(
            destination_path
        );

    if !source_path.exists() {
        return Err(
            format!(
                "Source database does not exist: {}",
                source_path.display(),
            )
        );
    }

    if same_path(
        &source_path,
        &destination_path,
    )? {
        return Err(
            "Source and reconstruction destination must be different paths"
                .to_string()
        );
    }

    if destination_path.exists() {
        return Err(
            format!(
                "Reconstruction destination already exists: {}",
                destination_path.display(),
            )
        );
    }

    println!(
        "[SCHEMA RECONSTRUCTION TEST] Source: {}",
        source_path.display()
    );

    println!(
        "[SCHEMA RECONSTRUCTION TEST] Source open mode: READ ONLY"
    );

    println!(
        "[SCHEMA RECONSTRUCTION TEST] Destination: {}",
        destination_path.display()
    );

    let source_connection =
        open_read_only(
            &source_path
        )?;

    let source_data =
        crate::database_migration::read_schema_v001::read(
            &source_connection
        )?;

    println!(
        "[SCHEMA RECONSTRUCTION TEST] Source extracted: {} shaders, {} policies, {} playlists, {} memberships",
        source_data.shaders.len(),
        source_data.policies.len(),
        source_data.playlists.len(),
        membership_count(
            &source_data
        ),
    );

    let destination_connection =
        crate::database_migration::write_current::write(
            &destination_path,
            &source_data,
        )?;

    drop(
        destination_connection
    );

    let reconstructed_connection =
        open_read_only(
            &destination_path
        )?;

    let reconstructed_data =
        crate::database_migration::read_schema_v001::read(
            &reconstructed_connection
        )?;

    println!(
        "[SCHEMA RECONSTRUCTION TEST] Reconstructed database read back successfully"
    );

    compare_migration_data(
        &source_data,
        &reconstructed_data,
    )?;

    test_factory_default_reconstruction(
        &source_data,
        &destination_path,
    )?;

    println!(
        "[SCHEMA RECONSTRUCTION TEST] PASS: durable Schema-1 semantics survived read -> MigrationData -> current-schema reconstruction -> read."
    );

    println!(
        "[SCHEMA RECONSTRUCTION TEST] Source database was never opened for writing."
    );

    println!(
        "[SCHEMA RECONSTRUCTION TEST] Reconstructed database retained at: {}",
        destination_path.display()
    );

    Ok(())
}


fn open_read_only(
    path: &Path,
) -> Result<Connection, String> {

    let connection =
        Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to open database read-only at '{}': {}",
                    path.display(),
                    error,
                )
            }
        )?;

    connection
        .pragma_update(
            None,
            "foreign_keys",
            "ON",
        )
        .map_err(
            |error| {
                format!(
                    "Unable to enable foreign-key checking for '{}': {}",
                    path.display(),
                    error,
                )
            }
        )?;

    Ok(
        connection
    )
}


fn same_path(
    source: &Path,
    destination: &Path,
) -> Result<bool, String> {

    let source =
        fs::canonicalize(
            source
        )
        .map_err(
            |error| {
                format!(
                    "Unable to canonicalize source database '{}': {}",
                    source.display(),
                    error,
                )
            }
        )?;

    let destination =
        if destination.exists() {
            fs::canonicalize(
                destination
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to canonicalize destination database '{}': {}",
                        destination.display(),
                        error,
                    )
                }
            )?
        } else {
            let parent =
                destination
                    .parent()
                    .unwrap_or_else(
                        || Path::new(".")
                    );

            let canonical_parent =
                fs::canonicalize(
                    parent
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to canonicalize reconstruction destination directory '{}': {}",
                            parent.display(),
                            error,
                        )
                    }
                )?;

            let filename =
                destination
                    .file_name()
                    .ok_or_else(
                        || {
                            format!(
                                "Reconstruction destination does not contain a filename: {}",
                                destination.display(),
                            )
                        }
                    )?;

            canonical_parent
                .join(
                    filename
                )
        };

    Ok(
        source == destination
    )
}


fn membership_count(
    data: &MigrationData,
) -> usize {

    data.playlists
        .iter()
        .map(
            |playlist| {
                playlist.members.len()
            }
        )
        .sum()
}


fn test_factory_default_reconstruction(
    source_data: &MigrationData,
    destination_path: &Path,
) -> Result<(), String> {

    let mut factory_data =
        source_data.clone();

    let factory_shader =
        factory_data.shaders
            .first_mut()
            .ok_or_else(
                || {
                    "Factory-default reconstruction test requires at least one fixture shader"
                        .to_string()
                }
            )?;

    factory_shader.filename =
        "default.glsl".to_string();

    factory_shader.source_path =
        "/historical/factory/shaders".to_string();

    let factory_destination =
        destination_path
            .with_extension(
                "factory-default-test.db"
            );

    if factory_destination.exists() {
        fs::remove_file(
            &factory_destination
        )
        .map_err(
            |error| {
                format!(
                    "Unable to remove stale factory-default reconstruction test database '{}': {}",
                    factory_destination.display(),
                    error,
                )
            }
        )?;
    }

    let result =
        (|| -> Result<(), String> {
            let connection =
                crate::database_migration::write_current::write(
                    &factory_destination,
                    &factory_data,
                )?;

            verify_factory_default_runtime_package(
                &connection
            )?;

            drop(
                connection
            );

            let reconstructed_connection =
                open_read_only(
                    &factory_destination
                )?;

            let reconstructed_data =
                crate::database_migration::read_schema_v001::read(
                    &reconstructed_connection
                )?;

            compare_migration_data(
                &factory_data,
                &reconstructed_data,
            )?;

            println!(
                "[SCHEMA RECONSTRUCTION TEST] Verified: historical default.glsl identity remapped to current factory shader"
            );

            println!(
                "[SCHEMA RECONSTRUCTION TEST] Verified: policies, playlists, and runtime references survived factory-shader remapping"
            );

            Ok(())
        })();

    if factory_destination.exists() {
        fs::remove_file(
            &factory_destination
        )
        .map_err(
            |error| {
                format!(
                    "Unable to remove factory-default reconstruction test database '{}': {}",
                    factory_destination.display(),
                    error,
                )
            }
        )?;
    }

    result
}


fn verify_factory_default_runtime_package(
    connection: &Connection,
) -> Result<(), String> {

    let (
        shader_type,
        source_hash,
        source_path,
        file_status,
        validation_status,
        runtime_source_length,
        preprocessor_version,
        channel_usage_mask,
        shader_inputs_json,
    ): (
        String,
        String,
        String,
        String,
        String,
        i64,
        i64,
        i64,
        String,
    ) =
        connection
            .query_row(
                "SELECT
                     shader_type,
                     source_hash,
                     source_path,
                     file_status,
                     validation_status,
                     length(preprocessed_source),
                     preprocessor_version,
                     channel_usage_mask,
                     shader_inputs_json
                 FROM shaders
                 WHERE filename = 'default.glsl'",
                [],
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
                            row.get(8)?,
                        )
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to inspect reconstructed factory default.glsl runtime package: {}",
                        error,
                    )
                }
            )?;

    let expected_source_path =
        crate::locate_paths::shader_dir()
            .to_string_lossy()
            .to_string();

    if shader_type != "native"
        || source_hash.trim().is_empty()
        || source_path != expected_source_path
        || file_status != "present"
        || validation_status != "valid"
        || runtime_source_length <= 0
        || preprocessor_version != 1
        || channel_usage_mask < 0
        || shader_inputs_json != "[]"
    {
        return Err(
            format!(
                "Reconstructed factory default.glsl does not contain the current runtime package: type='{}', hash_present={}, source_path='{}', file_status='{}', validation_status='{}', runtime_bytes={}, preprocessor_version={}, channel_usage_mask={}, shader_inputs_json='{}'",
                shader_type,
                !source_hash.trim().is_empty(),
                source_path,
                file_status,
                validation_status,
                runtime_source_length,
                preprocessor_version,
                channel_usage_mask,
                shader_inputs_json,
            )
        );
    }

    println!(
        "[SCHEMA RECONSTRUCTION TEST] Verified: default.glsl runtime package was rebuilt from the current installed factory shader"
    );

    Ok(())
}


fn shader_source_identity(
    shader: &crate::database_migration::migration_data::MigrationShader,
) -> String {

    if shader.filename
        == "default.glsl"
    {
        "<current-factory-default>"
            .to_string()
    } else {
        shader.source_path.clone()
    }
}


fn compare_migration_data(
    source: &MigrationData,
    reconstructed: &MigrationData,
) -> Result<(), String> {

    compare_shaders(
        source,
        reconstructed,
    )?;

    compare_policies(
        source,
        reconstructed,
    )?;

    compare_playlists(
        source,
        reconstructed,
    )?;

    compare_runtime_configuration(
        source,
        reconstructed,
    )?;

    compare_application_defaults(
        source,
        reconstructed,
    )?;

    compare_target_defaults(
        source,
        reconstructed,
    )?;

    Ok(())
}


fn compare_shaders(
    source: &MigrationData,
    reconstructed: &MigrationData,
) -> Result<(), String> {

    let mut source_values =
        source.shaders
            .iter()
            .map(
                |shader| {
                    (
                        shader.filename.clone(),
                        shader_source_identity(
                            shader
                        ),
                    )
                }
            )
            .collect::<Vec<_>>();

    let mut reconstructed_values =
        reconstructed.shaders
            .iter()
            .map(
                |shader| {
                    (
                        shader.filename.clone(),
                        shader_source_identity(
                            shader
                        ),
                    )
                }
            )
            .collect::<Vec<_>>();

    source_values.sort();
    reconstructed_values.sort();

    require_equal(
        "shader identities",
        &source_values,
        &reconstructed_values,
    )
}


fn compare_policies(
    source: &MigrationData,
    reconstructed: &MigrationData,
) -> Result<(), String> {

    let mut source_values =
        source.policies
            .iter()
            .map(
                |policy| {
                    policy_signature(
                        source,
                        policy
                    )
                }
            )
            .collect::<Result<Vec<_>, _>>()?;

    let mut reconstructed_values =
        reconstructed.policies
            .iter()
            .map(
                |policy| {
                    policy_signature(
                        reconstructed,
                        policy
                    )
                }
            )
            .collect::<Result<Vec<_>, _>>()?;

    source_values.sort();
    reconstructed_values.sort();

    require_equal(
        "policy semantics",
        &source_values,
        &reconstructed_values,
    )
}


fn policy_signature(
    data: &MigrationData,
    policy: &crate::database_migration::migration_data::MigrationPolicy,
) -> Result<String, String> {

    let shader =
        data.shaders
            .iter()
            .find(
                |shader| {
                    shader.migration_id
                        == policy.shader
                }
            )
            .ok_or_else(
                || {
                    format!(
                        "Policy '{}' references missing shader migration ID {} while building diagnostic signature",
                        policy.policy_name,
                        policy.shader.0,
                    )
                }
            )?;

    Ok(
        format!(
            "{:?}|{:?}|{:?}|{}|{}|{}|{:?}|{:?}|{}|{:?}|{:?}|{:?}|{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            policy.policy_name,
            shader_source_identity(
                shader
            ),
            shader.filename,
            policy_target(
                policy.target
            ),
            texture_signature(
                &policy.texture
            ),
            palette_signature(
                &policy.palette
            ),
            policy.rendered_fps,
            policy.animation_speed,
            policy.starting_offset_seconds.to_bits(),
            policy.anti_aliasing,
            policy.dithering,
            policy.color_precision,
            policy.render_scale.map(f64::to_bits),
            policy.audiovisual_effect,
            policy.audio_motion_effect,
            policy.bloom_intensity.to_bits(),
            policy.bloom_saturation.to_bits(),
            policy.bloom_threshold.to_bits(),
            policy.bloom_frequency_rotation.to_bits(),
            policy.bloom_frequency_invert,
            policy.invert_colors,
            policy.flip_horizontal,
            policy.flip_vertical,
            policy.hue_rotation.to_bits(),
            policy.created_at.as_deref().unwrap_or(""),
            policy.modified_at.as_deref().unwrap_or(""),
        )
    )
}


fn compare_playlists(
    source: &MigrationData,
    reconstructed: &MigrationData,
) -> Result<(), String> {

    let mut source_values =
        source.playlists
            .iter()
            .map(
                |playlist| {
                    playlist_signature(
                        source,
                        playlist
                    )
                }
            )
            .collect::<Result<Vec<_>, _>>()?;

    let mut reconstructed_values =
        reconstructed.playlists
            .iter()
            .map(
                |playlist| {
                    playlist_signature(
                        reconstructed,
                        playlist
                    )
                }
            )
            .collect::<Result<Vec<_>, _>>()?;

    source_values.sort();
    reconstructed_values.sort();

    require_equal(
        "playlist semantics and ordering",
        &source_values,
        &reconstructed_values,
    )
}


fn playlist_signature(
    data: &MigrationData,
    playlist: &crate::database_migration::migration_data::MigrationPlaylist,
) -> Result<String, String> {

    let mut members =
        Vec::new();

    for member_id in
        &playlist.members
    {
        let policy =
            data.policies
                .iter()
                .find(
                    |policy| {
                        policy.migration_id
                            == *member_id
                    }
                )
                .ok_or_else(
                    || {
                        format!(
                            "Playlist '{}' references missing policy migration ID {} while building diagnostic signature",
                            playlist.playlist_name,
                            member_id.0,
                        )
                    }
                )?;

        members.push(
            policy.policy_name.clone()
        );
    }

    Ok(
        format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}",
            playlist.playlist_name,
            playlist.description,
            playlist.created_at,
            playlist.modified_at,
            members,
        )
    )
}


fn compare_runtime_configuration(
    source: &MigrationData,
    reconstructed: &MigrationData,
) -> Result<(), String> {

    let source_screensaver =
        runtime_signature(
            source,
            &source.runtime_configuration.screensaver,
        )?;

    let reconstructed_screensaver =
        runtime_signature(
            reconstructed,
            &reconstructed.runtime_configuration.screensaver,
        )?;

    require_equal(
        "screensaver runtime configuration",
        &source_screensaver,
        &reconstructed_screensaver,
    )?;

    let source_wallpaper =
        runtime_signature(
            source,
            &source.runtime_configuration.wallpaper,
        )?;

    let reconstructed_wallpaper =
        runtime_signature(
            reconstructed,
            &reconstructed.runtime_configuration.wallpaper,
        )?;

    require_equal(
        "wallpaper runtime configuration",
        &source_wallpaper,
        &reconstructed_wallpaper,
    )
}


fn runtime_signature(
    data: &MigrationData,
    mode: &MigrationDisplayMode,
) -> Result<String, String> {

    match mode {
        MigrationDisplayMode::Single {
            policy,
        } => {
            let name =
                match policy {
                    Some(id) => {
                        Some(
                            policy_name(
                                data,
                                *id
                            )?
                        )
                    }

                    None => None,
                };

            Ok(
                format!(
                    "single|{:?}",
                    name
                )
            )
        }

        MigrationDisplayMode::Ordered {
            interval_seconds,
        } => {
            Ok(
                format!(
                    "ordered|{}",
                    interval_seconds
                )
            )
        }

        MigrationDisplayMode::Random {
            interval_seconds,
        } => {
            Ok(
                format!(
                    "random|{}",
                    interval_seconds
                )
            )
        }

        MigrationDisplayMode::Playlist {
            playlist,
            interval_seconds,
        } => {
            let name =
                match playlist {
                    Some(id) => {
                        Some(
                            playlist_name(
                                data,
                                *id
                            )?
                        )
                    }

                    None => None,
                };

            Ok(
                format!(
                    "playlist|{:?}|{}",
                    name,
                    interval_seconds,
                )
            )
        }
    }
}


fn compare_application_defaults(
    source: &MigrationData,
    reconstructed: &MigrationData,
) -> Result<(), String> {

    let left =
        &source.application_defaults;

    let right =
        &reconstructed.application_defaults;

    require_equal(
        "application defaults",
        &format!(
            "{}|{}|{:?}|{}|{}|{:?}|{}|{:?}|{:?}|{:?}|{}|{}|{}|{:?}",
            left.show_splash,
            left.screensaver_subtitles,
            left.subtitle_placement,
            left.wallpaper_notifications,
            left.lyrics_enabled,
            left.wallpaper_display_format,
            left.rendered_fps,
            left.anti_aliasing,
            left.dithering,
            left.color_precision,
            left.render_scale.to_bits(),
            left.automatic_backups,
            left.backup_interval_days,
            left.last_backup,
        ),
        &format!(
            "{}|{}|{:?}|{}|{}|{:?}|{}|{:?}|{:?}|{:?}|{}|{}|{}|{:?}",
            right.show_splash,
            right.screensaver_subtitles,
            right.subtitle_placement,
            right.wallpaper_notifications,
            right.lyrics_enabled,
            right.wallpaper_display_format,
            right.rendered_fps,
            right.anti_aliasing,
            right.dithering,
            right.color_precision,
            right.render_scale.to_bits(),
            right.automatic_backups,
            right.backup_interval_days,
            right.last_backup,
        ),
    )
}


fn compare_target_defaults(
    source: &MigrationData,
    reconstructed: &MigrationData,
) -> Result<(), String> {

    require_equal(
        "screensaver target defaults",
        &target_default_signature(
            &source.target_defaults.screensaver
        ),
        &target_default_signature(
            &reconstructed.target_defaults.screensaver
        ),
    )?;

    require_equal(
        "wallpaper target defaults",
        &target_default_signature(
            &source.target_defaults.wallpaper
        ),
        &target_default_signature(
            &reconstructed.target_defaults.wallpaper
        ),
    )
}


fn target_default_signature(
    value: &crate::database_migration::migration_data::MigrationTargetDefault,
) -> String {

    format!(
        "{}|{}|{}|{}|{}",
        idle_timeout_signature(
            &value.idle_timeout
        ),
        value.animation_speed.to_bits(),
        default_texture_signature(
            &value.texture
        ),
        value.texture_primitives,
        default_palette_signature(
            &value.palette
        ),
    )
}


fn policy_name(
    data: &MigrationData,
    id: crate::database_migration::migration_data::MigrationPolicyId,
) -> Result<String, String> {

    data.policies
        .iter()
        .find(
            |policy| {
                policy.migration_id
                    == id
            }
        )
        .map(
            |policy| {
                policy.policy_name.clone()
            }
        )
        .ok_or_else(
            || {
                format!(
                    "Runtime configuration references missing policy migration ID {}",
                    id.0,
                )
            }
        )
}


fn playlist_name(
    data: &MigrationData,
    id: crate::database_migration::migration_data::MigrationPlaylistId,
) -> Result<String, String> {

    data.playlists
        .iter()
        .find(
            |playlist| {
                playlist.migration_id
                    == id
            }
        )
        .map(
            |playlist| {
                playlist.playlist_name.clone()
            }
        )
        .ok_or_else(
            || {
                format!(
                    "Runtime configuration references missing playlist migration ID {}",
                    id.0,
                )
            }
        )
}


fn policy_target(
    value: MigrationPolicyTarget,
) -> &'static str {

    match value {
        MigrationPolicyTarget::Unassigned => "unassigned",
        MigrationPolicyTarget::Screensaver => "screensaver",
        MigrationPolicyTarget::Wallpaper => "wallpaper",
    }
}


fn texture_signature(
    value: &MigrationTexture,
) -> String {

    match value {
        MigrationTexture::Inherit => {
            "inherit".to_string()
        }

        MigrationTexture::Random => {
            "random".to_string()
        }

        MigrationTexture::Specific {
            family,
            primitives,
        } => {
            format!(
                "specific:{}:{}",
                family,
                primitives,
            )
        }
    }
}


fn palette_signature(
    value: &MigrationPalette,
) -> String {

    match value {
        MigrationPalette::Inherit => {
            "inherit".to_string()
        }

        MigrationPalette::Random => {
            "random".to_string()
        }

        MigrationPalette::Specific {
            color,
        } => {
            format!(
                "specific:{}",
                color
            )
        }
    }
}


fn idle_timeout_signature(
    value: &MigrationIdleTimeout,
) -> String {

    match value {
        MigrationIdleTimeout::NotApplicable => {
            "n/a".to_string()
        }

        MigrationIdleTimeout::Value {
            value,
            unit,
        } => {
            format!(
                "{}:{}",
                value,
                unit,
            )
        }
    }
}


fn default_texture_signature(
    value: &MigrationDefaultTexture,
) -> String {

    match value {
        MigrationDefaultTexture::Random => {
            "random".to_string()
        }

        MigrationDefaultTexture::Specific {
            family,
            primitives,
        } => {
            format!(
                "specific:{}:{}",
                family,
                primitives,
            )
        }
    }
}


fn default_palette_signature(
    value: &MigrationDefaultPalette,
) -> String {

    match value {
        MigrationDefaultPalette::Random => {
            "random".to_string()
        }

        MigrationDefaultPalette::Specific {
            color,
        } => {
            format!(
                "specific:{}",
                color
            )
        }
    }
}


fn require_equal<T>(
    label: &str,
    source: &T,
    reconstructed: &T,
) -> Result<(), String>
where
    T: PartialEq + std::fmt::Debug,
{
    if source
        != reconstructed
    {
        return Err(
            format!(
                "{} changed during reconstruction.\n  source: {:?}\n  reconstructed: {:?}",
                label,
                source,
                reconstructed,
            )
        );
    }

    println!(
        "[SCHEMA RECONSTRUCTION TEST] Verified: {}",
        label
    );

    Ok(())
}
