//! Developer diagnostic for the permanent Schema-1 historical reader.
//!
//! This command deliberately bypasses normal database evaluation.  It opens
//! a selected Schema-1 database read-only, translates it into MigrationData,
//! prints a compact semantic summary, and exits. With no explicit path, the
//! normal Screenshaver database is used.

use rusqlite::{Connection, OpenFlags};


pub fn run(
    database_path: Option<&str>,
) -> Result<(), String> {
    let database_path =
        match database_path {
            Some(path) => {
                std::path::PathBuf::from(path)
            }

            None => {
                crate::locate_paths::database_path()
            }
        };

    if !database_path.exists() {
        return Err(
            format!(
                "Screenshaver database does not exist: {}",
                database_path.display(),
            )
        );
    }

    let connection =
        Connection::open_with_flags(
            &database_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to open database read-only at {}: {}",
                    database_path.display(),
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
                    "Unable to enable foreign-key checking on the read-only diagnostic connection: {}",
                    error,
                )
            }
        )?;

    let data =
        crate::database_migration::read_schema_v001::read(
            &connection,
        )?;

    print_summary(
        &database_path,
        &data,
    );

    Ok(())
}


fn print_summary(
    database_path: &std::path::Path,
    data: &crate::database_migration::migration_data::MigrationData,
) {
    let screensaver_policies =
        data.policies
            .iter()
            .filter(
                |policy| {
                    matches!(
                        policy.target,
                        crate::database_migration::migration_data::MigrationPolicyTarget::Screensaver
                    )
                }
            )
            .count();

    let wallpaper_policies =
        data.policies
            .iter()
            .filter(
                |policy| {
                    matches!(
                        policy.target,
                        crate::database_migration::migration_data::MigrationPolicyTarget::Wallpaper
                    )
                }
            )
            .count();

    let unassigned_policies =
        data.policies
            .iter()
            .filter(
                |policy| {
                    matches!(
                        policy.target,
                        crate::database_migration::migration_data::MigrationPolicyTarget::Unassigned
                    )
                }
            )
            .count();

    let playlist_members =
        data.playlists
            .iter()
            .map(
                |playlist| {
                    playlist.members.len()
                }
            )
            .sum::<usize>();

    println!(
        "[SCHEMA READER TEST] Database: {}",
        database_path.display()
    );

    println!(
        "[SCHEMA READER TEST] Open mode: READ ONLY"
    );

    println!(
        "[SCHEMA READER TEST] Source schema: {}",
        data.source.schema_version
    );

    println!(
        "[SCHEMA READER TEST] Created by Screenshaver: {}",
        data.source.created_by_version
    );

    println!(
        "[SCHEMA READER TEST] Last migrated by Screenshaver: {}",
        data.source.last_migrated_by_version
    );

    println!(
        "[SCHEMA READER TEST] Shaders: {}",
        data.shaders.len()
    );

    println!(
        "[SCHEMA READER TEST] Policies: {} total ({} screensaver, {} wallpaper, {} unassigned)",
        data.policies.len(),
        screensaver_policies,
        wallpaper_policies,
        unassigned_policies,
    );

    println!(
        "[SCHEMA READER TEST] Playlists: {} ({} total memberships)",
        data.playlists.len(),
        playlist_members,
    );

    println!(
        "[SCHEMA READER TEST] Screensaver runtime: {}",
        display_mode_summary(
            &data.runtime_configuration.screensaver
        )
    );

    println!(
        "[SCHEMA READER TEST] Wallpaper runtime: {}",
        display_mode_summary(
            &data.runtime_configuration.wallpaper
        )
    );

    println!(
        "[SCHEMA READER TEST] Application defaults extracted: yes"
    );

    println!(
        "[SCHEMA READER TEST] Target defaults extracted: screensaver=yes, wallpaper=yes"
    );

    println!(
        "[SCHEMA READER TEST] PASS: Schema-1 durable semantics were extracted into MigrationData without modifying the source database."
    );
}


fn display_mode_summary(
    mode: &crate::database_migration::migration_data::MigrationDisplayMode,
) -> String {
    use crate::database_migration::migration_data::MigrationDisplayMode;

    match mode {
        MigrationDisplayMode::Single { policy } => {
            match policy {
                Some(policy) => {
                    format!(
                        "single (policy migration id {})",
                        policy.0,
                    )
                }

                None => {
                    "single (no selected policy)"
                        .to_string()
                }
            }
        }

        MigrationDisplayMode::Ordered {
            interval_seconds,
        } => {
            format!(
                "ordered (interval {}s)",
                interval_seconds,
            )
        }

        MigrationDisplayMode::Random {
            interval_seconds,
        } => {
            format!(
                "random (interval {}s)",
                interval_seconds,
            )
        }

        MigrationDisplayMode::Playlist {
            playlist,
            interval_seconds,
        } => {
            match playlist {
                Some(playlist) => {
                    format!(
                        "playlist (playlist migration id {}, interval {}s)",
                        playlist.0,
                        interval_seconds,
                    )
                }

                None => {
                    format!(
                        "playlist (no selected playlist, interval {}s)",
                        interval_seconds,
                    )
                }
            }
        }
    }
}
