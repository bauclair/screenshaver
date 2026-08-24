use rusqlite::Connection;


const CURRENT_SCHEMA_VERSION: i64 = 1;


#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub enum MigrationOutcome {

    Current {
        schema_version: i64,
    },

    Migrated {
        from_version: i64,
        to_version: i64,
    },
}


pub fn migrate(
    connection: &mut Connection,
) -> Result<MigrationOutcome, String> {

    let schema_version =
        read_schema_version(
            connection
        )?;


    if schema_version
        < 1
    {
        return Err(
            format!(
                "Database schema version {} is invalid; schema versions must be 1 or greater",
                schema_version,
            )
        );
    }


    if schema_version
        > CURRENT_SCHEMA_VERSION
    {
        return Err(
            format!(
                "Database schema version {} is newer than the maximum supported schema version {} for Screenshaver {}; refusing to modify the database",
                schema_version,
                CURRENT_SCHEMA_VERSION,
                env!(
                    "CARGO_PKG_VERSION"
                ),
            )
        );
    }


    if schema_version
        == CURRENT_SCHEMA_VERSION
    {
        return Ok(
            MigrationOutcome::Current {
                schema_version,
            }
        );
    }


    /*
     * Future migration dispatcher.
     *
     * When Schema Version 2 exists, migration will proceed one schema
     * version at a time:
     *
     *     1 -> 2
     *     2 -> 3
     *     ...
     *
     * Each migration step must:
     *
     *   - execute transactionally;
     *   - preserve shader source files;
     *   - update schema_metadata only after the migration succeeds;
     *   - update last_migrated_by_version to the running Screenshaver
     *     version;
     *   - validate the resulting schema before committing;
     *   - never skip an intermediate schema version.
     *
     * CURRENT_SCHEMA_VERSION is 1, so this branch is unreachable today.
     */

    Err(
        format!(
            "No migration path is implemented from database schema version {} to schema version {}",
            schema_version,
            CURRENT_SCHEMA_VERSION,
        )
    )
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
