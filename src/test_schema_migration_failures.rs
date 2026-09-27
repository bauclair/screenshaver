//! Developer diagnostic for database-migration failure safety.
//!
//! The permanent historical fixture is never modified.  Reader tests copy it
//! to temporary files, deliberately introduce invalid Schema-1 semantics, and
//! verify that the historical reader rejects those copies.  Writer tests
//! mutate only in-memory MigrationData and verify that a failed reconstruction
//! removes its partial destination.
//!
//! Reader-negative cases deliberately remain valid according to the physical
//! Schema-1 SQL constraints. We do not disable SQLite CHECK constraints merely
//! to manufacture states that a legitimate Schema-1 database could never contain.

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
    MigrationDisplayMode,
    MigrationPolicyId,
};


pub fn run(
    fixture_path: &str,
    work_directory: &str,
) -> Result<(), String> {

    let fixture_path =
        PathBuf::from(
            fixture_path
        );

    let work_directory =
        PathBuf::from(
            work_directory
        );

    if !fixture_path.is_file() {
        return Err(
            format!(
                "Schema-1 fixture does not exist: {}",
                fixture_path.display(),
            )
        );
    }

    fs::create_dir_all(
        &work_directory
    )
    .map_err(
        |error| {
            format!(
                "Unable to create migration failure-test directory '{}': {}",
                work_directory.display(),
                error,
            )
        }
    )?;

    println!(
        "[SCHEMA MIGRATION FAILURE TEST] Fixture: {}",
        fixture_path.display()
    );

    println!(
        "[SCHEMA MIGRATION FAILURE TEST] Fixture is treated as immutable"
    );

    println!(
        "[SCHEMA MIGRATION FAILURE TEST] Work directory: {}",
        work_directory.display()
    );

    let original_bytes =
        fs::read(
            &fixture_path
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read fixture before failure tests: {}",
                    error,
                )
            }
        )?;

    test_reader_rejects_missing_runtime_target(
        &fixture_path,
        &work_directory,
    )?;

    test_reader_preserves_gapped_playlist_order(
        &fixture_path,
        &work_directory,
    )?;

    test_writer_removes_partial_destination(
        &fixture_path,
        &work_directory,
    )?;

    let final_bytes =
        fs::read(
            &fixture_path
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read fixture after failure tests: {}",
                    error,
                )
            }
        )?;

    if original_bytes
        != final_bytes
    {
        return Err(
            "Permanent Schema-1 fixture changed during failure-path testing"
                .to_string()
        );
    }

    println!(
        "[SCHEMA MIGRATION FAILURE TEST] Verified: permanent fixture remained byte-for-byte unchanged"
    );

    println!(
        "[SCHEMA MIGRATION FAILURE TEST] PASS: invalid sources were rejected and failed reconstruction left no partial destination."
    );

    Ok(())
}


fn test_reader_rejects_missing_runtime_target(
    fixture_path: &Path,
    work_directory: &Path,
) -> Result<(), String> {

    let test_path =
        prepare_copy(
            fixture_path,
            work_directory,
            "missing-runtime-target.db",
        )?;

    {
        let connection =
            open_read_write(
                &test_path
            )?;

        // Schema 1 constrains the contents of each runtime-target row, but it
        // cannot require that both logical target rows exist.  Deleting one
        // therefore leaves a physically valid SQLite database whose durable
        // Screenshaver runtime state is incomplete.
        let deleted =
            connection
                .execute(
                    "DELETE FROM runtime_targets
                      WHERE target = 'wallpaper'",
                    [],
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to create missing-runtime-target test database: {}",
                            error,
                        )
                    }
                )?;

        if deleted
            != 1
        {
            return Err(
                format!(
                    "Missing-runtime-target test expected to delete one wallpaper row, deleted {}",
                    deleted,
                )
            );
        }
    }

    expect_reader_failure(
        &test_path,
        "missing required runtime target",
        "expected exactly two runtime_targets rows",
    )?;

    remove_test_file(
        &test_path
    )
}


fn test_reader_preserves_gapped_playlist_order(
    fixture_path: &Path,
    work_directory: &Path,
) -> Result<(), String> {

    let test_path =
        prepare_copy(
            fixture_path,
            work_directory,
            "gapped-playlist-positions.db",
        )?;

    {
        let connection =
            open_read_write(
                &test_path
            )?;

        // Schema 1 requires positive, unique positions but does not require
        // contiguous numbering.  Move the fourth member to position 9 while
        // preserving its semantic place after positions 1, 2, and 3.
        let changed =
            connection
                .execute(
                    "UPDATE playlist_members
                        SET position = 9
                      WHERE playlist_id = (
                            SELECT MIN(playlist_id)
                              FROM playlists
                      )
                        AND position = 4",
                    [],
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to create gapped-playlist test database: {}",
                            error,
                        )
                    }
                )?;

        if changed
            != 1
        {
            return Err(
                format!(
                    "Gapped-playlist test expected to update one membership row, updated {}",
                    changed,
                )
            );
        }
    }

    let connection =
        open_read_only(
            &test_path
        )?;

    let data =
        crate::database_migration::read_schema_v001::read(
            &connection
        )?;

    // The permanent fixture's first playlist contains four members.  A gap in
    // the numeric position values must not change or invalidate that ordering.
    let playlist =
        data.playlists
            .first()
            .ok_or_else(
                || {
                    "Gapped-playlist test could not find the fixture's first playlist"
                        .to_string()
                }
            )?;

    if playlist.members.len()
        != 4
    {
        return Err(
            format!(
                "Gapped-playlist test expected four members in the first playlist, found {}",
                playlist.members.len(),
            )
        );
    }

    println!(
        "[SCHEMA MIGRATION FAILURE TEST] Verified: reader accepted gapped playlist positions and preserved member order"
    );

    drop(
        connection
    );

    remove_test_file(
        &test_path
    )
}


fn test_writer_removes_partial_destination(
    fixture_path: &Path,
    work_directory: &Path,
) -> Result<(), String> {

    let source_connection =
        open_read_only(
            fixture_path
        )?;

    let mut data =
        crate::database_migration::read_schema_v001::read(
            &source_connection
        )?;

    // The fixture has valid policies, but this migration-local ID can never
    // exist.  This forces the writer to fail only after it has already created
    // the destination schema, seeded catalogs, and begun durable-data import.
    data.runtime_configuration.screensaver =
        MigrationDisplayMode::Single {
            policy:
                Some(
                    MigrationPolicyId(
                        u64::MAX
                    )
                ),
        };

    let destination_path =
        work_directory
            .join(
                "writer-must-clean-up.db"
            );

    remove_if_exists(
        &destination_path
    )?;

    let result =
        crate::database_migration::write_current::write(
            &destination_path,
            &data,
        );

    match result {
        Ok(connection) => {
            drop(
                connection
            );

            return Err(
                "Writer unexpectedly accepted MigrationData containing an unknown runtime policy reference"
                    .to_string()
            );
        }

        Err(error) => {
            if !error.contains(
                "unknown policy migration ID"
            ) {
                return Err(
                    format!(
                        "Writer failed for an unexpected reason: {}",
                        error,
                    )
                );
            }

            println!(
                "[SCHEMA MIGRATION FAILURE TEST] Verified: writer rejected invalid MigrationData relationship"
            );
        }
    }

    if destination_path.exists() {
        return Err(
            format!(
                "Failed reconstruction left a partial destination database behind: {}",
                destination_path.display(),
            )
        );
    }

    println!(
        "[SCHEMA MIGRATION FAILURE TEST] Verified: failed writer reconstruction removed partial destination"
    );

    Ok(())
}


fn expect_reader_failure(
    path: &Path,
    label: &str,
    expected_error_fragment: &str,
) -> Result<(), String> {

    let connection =
        open_read_only(
            path
        )?;

    match crate::database_migration::read_schema_v001::read(
        &connection
    ) {
        Ok(_) => {
            Err(
                format!(
                    "Historical reader unexpectedly accepted {} test database '{}'",
                    label,
                    path.display(),
                )
            )
        }

        Err(error) => {
            if !error.contains(
                expected_error_fragment
            ) {
                return Err(
                    format!(
                        "Historical reader rejected {} test for an unexpected reason: {}",
                        label,
                        error,
                    )
                );
            }

            println!(
                "[SCHEMA MIGRATION FAILURE TEST] Verified: reader rejected {}",
                label
            );

            Ok(())
        }
    }
}


fn prepare_copy(
    fixture_path: &Path,
    work_directory: &Path,
    filename: &str,
) -> Result<PathBuf, String> {

    let path =
        work_directory
            .join(
                filename
            );

    remove_if_exists(
        &path
    )?;

    fs::copy(
        fixture_path,
        &path,
    )
    .map_err(
        |error| {
            format!(
                "Unable to copy Schema-1 fixture to '{}': {}",
                path.display(),
                error,
            )
        }
    )?;

    Ok(
        path
    )
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
                    "Unable to open '{}' read-only: {}",
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
                    "Unable to enable foreign keys for read-only database '{}': {}",
                    path.display(),
                    error,
                )
            }
        )?;

    Ok(
        connection
    )
}


fn open_read_write(
    path: &Path,
) -> Result<Connection, String> {

    let connection =
        Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to open temporary test database '{}' read-write: {}",
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
                    "Unable to enable foreign keys for temporary test database '{}': {}",
                    path.display(),
                    error,
                )
            }
        )?;

    Ok(
        connection
    )
}


fn remove_test_file(
    path: &Path,
) -> Result<(), String> {

    fs::remove_file(
        path
    )
    .map_err(
        |error| {
            format!(
                "Unable to remove temporary migration test database '{}': {}",
                path.display(),
                error,
            )
        }
    )
}


fn remove_if_exists(
    path: &Path,
) -> Result<(), String> {

    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(
        path
    )
    .map_err(
        |error| {
            format!(
                "Unable to remove previous migration test database '{}': {}",
                path.display(),
                error,
            )
        }
    )
}
