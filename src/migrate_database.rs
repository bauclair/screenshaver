use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};


pub const CURRENT_SCHEMA_VERSION: i64 = 1;


pub fn prepare(database_path: &Path) -> Result<Connection, String> {
    if !database_path.exists() {
        return Err(format!(
            "Unable to prepare database because '{}' does not exist",
            database_path.display(),
        ));
    }

    let source_connection = open_read_only(database_path)?;
    let schema_version = read_schema_version(&source_connection)?;
    validate_schema_version(schema_version)?;

    if schema_version == CURRENT_SCHEMA_VERSION {
        drop(source_connection);

        let connection = crate::open_database::open()?;
        crate::validate_database::validate_startup(&connection)?;
        return Ok(connection);
    }

    drop(source_connection);

    /*
     * Future reconstruction dispatcher.
     *
     * Once Schema Version 2 exists, this branch will:
     *   1. reopen the historical source READ ONLY;
     *   2. dispatch directly to that released schema's historical reader;
     *   3. extract durable semantics into MigrationData;
     *   4. reconstruct CURRENT_SCHEMA_VERSION at screenshaver.db.migrating;
     *   5. fully validate the staged database;
     *   6. rename screenshaver.db to screenshaver.db.pre-migration;
     *   7. rename screenshaver.db.migrating to screenshaver.db;
     *   8. reopen and validate the new live database.
     *
     * Migration is reconstruction/ETL, not a serial ALTER chain.
     */
    Err(format!(
        "No reconstruction migration path is implemented from database schema version {} to schema version {}",
        schema_version,
        CURRENT_SCHEMA_VERSION,
    ))
}


pub fn recover_interrupted_cutover(database_path: &Path) -> Result<(), String> {
    let migrating_path = migrating_path(database_path);
    let pre_migration_path = pre_migration_path(database_path);

    if database_path.exists() {
        // A live database wins. Retained .pre-migration state is normal after
        // successful cutover. Any leftover staging database is disposable.
        if migrating_path.exists() {
            fs::remove_file(&migrating_path).map_err(|error| {
                format!(
                    "Unable to remove stale migration staging database '{}': {}",
                    migrating_path.display(),
                    error,
                )
            })?;
        }
        return Ok(());
    }

    if pre_migration_path.exists() {
        fs::rename(&pre_migration_path, database_path).map_err(|error| {
            format!(
                "Unable to recover interrupted database migration by restoring '{}' to '{}': {}",
                pre_migration_path.display(),
                database_path.display(),
                error,
            )
        })?;

        if migrating_path.exists() {
            fs::remove_file(&migrating_path).map_err(|error| {
                format!(
                    "Recovered the pre-migration database to '{}', but unable to remove stale staging database '{}': {}",
                    database_path.display(),
                    migrating_path.display(),
                    error,
                )
            })?;
        }
        return Ok(());
    }

    if migrating_path.exists() {
        return Err(format!(
            "Database migration state is ambiguous: '{}' exists, but neither '{}' nor '{}' exists. Refusing to promote the staging database or initialize a replacement database",
            migrating_path.display(),
            database_path.display(),
            pre_migration_path.display(),
        ));
    }

    Ok(())
}


pub fn migrating_path(database_path: &Path) -> PathBuf {
    companion_path(database_path, ".migrating")
}


pub fn pre_migration_path(database_path: &Path) -> PathBuf {
    companion_path(database_path, ".pre-migration")
}


fn companion_path(database_path: &Path, suffix: &str) -> PathBuf {
    let mut value = OsString::from(database_path.as_os_str());
    value.push(suffix);
    PathBuf::from(value)
}


fn open_read_only(database_path: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(
        database_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|error| {
        format!(
            "Unable to open database '{}' read-only for schema inspection: {}",
            database_path.display(),
            error,
        )
    })
}


fn validate_schema_version(schema_version: i64) -> Result<(), String> {
    if schema_version < 1 {
        return Err(format!(
            "Database schema version {} is invalid; schema versions must be 1 or greater",
            schema_version,
        ));
    }

    if schema_version > CURRENT_SCHEMA_VERSION {
        return Err(format!(
            "Database schema version {} is newer than the maximum supported schema version {} for Screenshaver {}; refusing to modify the database",
            schema_version,
            CURRENT_SCHEMA_VERSION,
            env!("CARGO_PKG_VERSION"),
        ));
    }

    Ok(())
}


fn read_schema_version(
    connection: &Connection,
) -> Result<i64, String> {

    let metadata_row_count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM schema_metadata",
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
                        "Unable to inspect schema_metadata: {}",
                        error,
                    )
                }
            )?;


    if metadata_row_count
        != 1
    {
        return Err(
            format!(
                "Database metadata is invalid: expected exactly one schema_metadata row, found {}",
                metadata_row_count,
            )
        );
    }


    let metadata_id: i64 =
        connection
            .query_row(
                "SELECT metadata_id
                 FROM schema_metadata",
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
                        "Unable to read schema metadata identifier: {}",
                        error,
                    )
                }
            )?;


    if metadata_id
        != 1
    {
        return Err(
            format!(
                "Database metadata is invalid: expected metadata_id 1, found {}",
                metadata_id,
            )
        );
    }


    connection
        .query_row(
            "SELECT schema_version
             FROM schema_metadata
             WHERE metadata_id = 1",
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
                    "Unable to read database schema version: {}",
                    error,
                )
            }
        )
}
