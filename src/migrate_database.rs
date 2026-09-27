use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{
    Instant,
    SystemTime,
    UNIX_EPOCH,
};

use rusqlite::{Connection, OpenFlags};


pub const CURRENT_SCHEMA_VERSION: i64 = 1;


#[derive(Debug, Clone)]
pub struct MigrationTiming {
    pub started_utc: String,
    pub source_schema_version: i64,
    pub destination_schema_version: i64,
    started: Instant,
}


impl MigrationTiming {
    pub fn start(
        source_schema_version: i64,
        destination_schema_version: i64,
    ) -> Result<Self, String> {
        Ok(
            Self {
                started_utc:
                    current_utc_timestamp()?,

                source_schema_version,
                destination_schema_version,

                started:
                    Instant::now(),
            }
        )
    }


    pub fn elapsed_milliseconds(
        &self,
    ) -> u128 {
        self.started
            .elapsed()
            .as_millis()
    }
}


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

    let timing =
        MigrationTiming::start(
            schema_version,
            CURRENT_SCHEMA_VERSION,
        )?;

    let migration_data =
        read_historical_database(
            schema_version,
            &source_connection,
        )?;

    let extraction_elapsed =
        timing.elapsed_milliseconds();

    drop(
        source_connection
    );


    let staging_path =
        migrating_path(
            database_path
        );

    if staging_path.exists() {
        fs::remove_file(
            &staging_path
        )
        .map_err(
            |error| {
                format!(
                    "Unable to remove stale migration staging database '{}': {}",
                    staging_path.display(),
                    error,
                )
            }
        )?;
    }


    let staging_connection =
        match crate::database_migration::write_current::write(
            &staging_path,
            &migration_data,
        ) {
            Ok(connection) => connection,

            Err(error) => {
                return Err(
                    format!(
                        "Database reconstruction from schema {} to schema {} failed before cutover: {}",
                        schema_version,
                        CURRENT_SCHEMA_VERSION,
                        error,
                    )
                );
            }
        };

    let reconstruction_elapsed =
        timing.elapsed_milliseconds();


    if let Err(error) =
        validate_reconstructed_database(
            &staging_connection
        )
    {
        drop(
            staging_connection
        );

        let cleanup_result =
            remove_if_exists(
                &staging_path
            );

        return match cleanup_result {
            Ok(()) => {
                Err(
                    format!(
                        "Reconstructed staging database failed validation before cutover; original database remains untouched: {}",
                        error,
                    )
                )
            }

            Err(cleanup_error) => {
                Err(
                    format!(
                        "Reconstructed staging database failed validation before cutover: {}. Original database remains untouched, but staging cleanup also failed: {}",
                        error,
                        cleanup_error,
                    )
                )
            }
        };
    }

    let staged_validation_elapsed =
        timing.elapsed_milliseconds();

    drop(
        staging_connection
    );


    let recovery_path =
        pre_migration_path(
            database_path,
            schema_version,
            &timing.started_utc,
        )?;

    if recovery_path.exists() {
        return Err(
            format!(
                "Refusing database migration because recovery destination '{}' already exists",
                recovery_path.display(),
            )
        );
    }


    let live_connection =
        promote_staging_with_rollback(
            database_path,
            &staging_path,
            &recovery_path,
            &timing.started_utc,
            validate_reconstructed_database,
        )?;

    let cutover_elapsed =
        timing.elapsed_milliseconds();


    let total_elapsed =
        timing.elapsed_milliseconds();

    println!(
        "[DATABASE MIGRATION] Schema {} -> {} completed successfully",
        schema_version,
        CURRENT_SCHEMA_VERSION,
    );

    println!(
        "[DATABASE MIGRATION] Started UTC: {}",
        timing.started_utc,
    );

    println!(
        "[DATABASE MIGRATION] Historical extraction: {} ms",
        extraction_elapsed,
    );

    println!(
        "[DATABASE MIGRATION] Reconstruction: {} ms cumulative",
        reconstruction_elapsed,
    );

    println!(
        "[DATABASE MIGRATION] Staged validation: {} ms cumulative",
        staged_validation_elapsed,
    );

    println!(
        "[DATABASE MIGRATION] Cutover: {} ms cumulative",
        cutover_elapsed,
    );

    println!(
        "[DATABASE MIGRATION] Total elapsed: {} ms",
        total_elapsed,
    );

    println!(
        "[DATABASE MIGRATION] Recovery database retained: {}",
        recovery_path.display(),
    );


    Ok(
        live_connection
    )
}


fn read_historical_database(
    schema_version: i64,
    connection: &Connection,
) -> Result<
    crate::database_migration::migration_data::MigrationData,
    String,
> {

    match schema_version {

        1 => {
            crate::database_migration::read_schema_v001::read(
                connection
            )
        }

        _ => {
            Err(
                format!(
                    "No historical database reader is available for schema version {}",
                    schema_version,
                )
            )
        }
    }
}


fn validate_reconstructed_database(
    connection: &Connection,
) -> Result<(), String> {

    crate::validate_database::validate_startup(
        connection
    )?;

    crate::validate_database::validate_integrity(
        connection
    )?;


    let schema_version =
        read_schema_version(
            connection
        )?;

    if schema_version
        != CURRENT_SCHEMA_VERSION
    {
        return Err(
            format!(
                "Reconstructed database reports schema version {}; expected current schema version {}",
                schema_version,
                CURRENT_SCHEMA_VERSION,
            )
        );
    }


    Ok(())
}


fn open_existing_read_write(
    database_path: &Path,
) -> Result<Connection, String> {

    let connection =
        Connection::open_with_flags(
            database_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to open existing database '{}': {}",
                    database_path.display(),
                    error,
                )
            }
        )?;


    crate::open_database::configure_connection(
        &connection
    )?;


    Ok(
        connection
    )
}


fn promote_staging_with_rollback<F>(
    database_path: &Path,
    staging_path: &Path,
    recovery_path: &Path,
    timestamp_utc: &str,
    final_validator: F,
) -> Result<Connection, String>
where
    F: FnOnce(&Connection) -> Result<(), String>,
{

    fs::rename(
        database_path,
        recovery_path,
    )
    .map_err(
        |error| {
            format!(
                "Unable to preserve source database '{}' as '{}': {}",
                database_path.display(),
                recovery_path.display(),
                error,
            )
        }
    )?;


    if let Err(error) =
        fs::rename(
            staging_path,
            database_path,
        )
    {
        let rollback_result =
            fs::rename(
                recovery_path,
                database_path,
            );

        return match rollback_result {
            Ok(()) => {
                Err(
                    format!(
                        "Unable to promote reconstructed database '{}' to '{}': {}. Original database was restored successfully",
                        staging_path.display(),
                        database_path.display(),
                        error,
                    )
                )
            }

            Err(rollback_error) => {
                Err(
                    format!(
                        "CRITICAL: unable to promote reconstructed database '{}' to '{}': {}. Automatic rollback from '{}' also failed: {}",
                        staging_path.display(),
                        database_path.display(),
                        error,
                        recovery_path.display(),
                        rollback_error,
                    )
                )
            }
        };
    }


    let live_connection =
        match open_existing_read_write(
            database_path
        ) {
            Ok(connection) => connection,

            Err(error) => {
                rollback_after_promotion_failure(
                    database_path,
                    recovery_path,
                    timestamp_utc,
                    &format!(
                        "Unable to reopen promoted database: {}",
                        error,
                    ),
                )?;

                return Err(
                    "Promoted database could not be reopened; original database was restored"
                        .to_string()
                );
            }
        };


    if let Err(error) =
        final_validator(
            &live_connection
        )
    {
        drop(
            live_connection
        );

        rollback_after_promotion_failure(
            database_path,
            recovery_path,
            timestamp_utc,
            &format!(
                "Final promoted-database validation failed: {}",
                error,
            ),
        )?;

        return Err(
            "Promoted database failed final validation; original database was restored"
                .to_string()
        );
    }


    Ok(
        live_connection
    )
}


pub(crate) fn test_promote_staging(
    database_path: &Path,
    staging_path: &Path,
    recovery_path: &Path,
    timestamp_utc: &str,
    force_final_validation_failure: bool,
) -> Result<Connection, String> {

    promote_staging_with_rollback(
        database_path,
        staging_path,
        recovery_path,
        timestamp_utc,
        |connection| {
            if force_final_validation_failure {
                return Err(
                    "forced coordinator diagnostic final-validation failure"
                        .to_string()
                );
            }

            validate_reconstructed_database(
                connection
            )
        },
    )
}


fn rollback_after_promotion_failure(
    database_path: &Path,
    recovery_path: &Path,
    timestamp_utc: &str,
    failure_reason: &str,
) -> Result<(), String> {

    let failed_path =
        failed_migration_path(
            database_path,
            timestamp_utc,
        )?;


    if failed_path.exists() {
        return Err(
            format!(
                "CRITICAL: {}. Cannot roll back automatically because failed-migration evidence path '{}' already exists; recovery database remains at '{}'",
                failure_reason,
                failed_path.display(),
                recovery_path.display(),
            )
        );
    }


    fs::rename(
        database_path,
        &failed_path,
    )
    .map_err(
        |error| {
            format!(
                "CRITICAL: {}. Unable to preserve failed promoted database '{}' as '{}': {}. Recovery database remains at '{}'",
                failure_reason,
                database_path.display(),
                failed_path.display(),
                error,
                recovery_path.display(),
            )
        }
    )?;


    fs::rename(
        recovery_path,
        database_path,
    )
    .map_err(
        |error| {
            format!(
                "CRITICAL: {}. Failed promoted database was preserved as '{}', but unable to restore recovery database '{}' to '{}': {}",
                failure_reason,
                failed_path.display(),
                recovery_path.display(),
                database_path.display(),
                error,
            )
        }
    )?;


    Err(
        format!(
            "{}; original database restored and failed promoted database retained at '{}'",
            failure_reason,
            failed_path.display(),
        )
    )
}


fn failed_migration_path(
    database_path: &Path,
    timestamp_utc: &str,
) -> Result<PathBuf, String> {

    validate_timestamp(
        timestamp_utc
    )?;


    Ok(
        companion_path(
            database_path,
            &format!(
                ".failed-migration-{}",
                timestamp_utc,
            ),
        )
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
                "Unable to remove migration staging database '{}': {}",
                path.display(),
                error,
            )
        }
    )
}


pub fn recover_interrupted_cutover(
    database_path: &Path,
) -> Result<(), String> {

    let migrating_path =
        migrating_path(
            database_path
        );

    let recovery_paths =
        discover_pre_migration_paths(
            database_path
        )?;


    if database_path.exists() {

        // A live database wins. Retained pre-migration generations are normal
        // after successful cutover. Any leftover staging database is disposable.
        if migrating_path.exists() {
            fs::remove_file(
                &migrating_path
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to remove stale migration staging database '{}': {}",
                        migrating_path.display(),
                        error,
                    )
                }
            )?;
        }

        return Ok(());
    }


    if let Some(
        recovery_path
    ) =
        newest_pre_migration_path(
            &recovery_paths
        )
    {

        fs::rename(
            &recovery_path,
            database_path,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to recover interrupted database migration by restoring '{}' to '{}': {}",
                    recovery_path.display(),
                    database_path.display(),
                    error,
                )
            }
        )?;


        if migrating_path.exists() {
            fs::remove_file(
                &migrating_path
            )
            .map_err(
                |error| {
                    format!(
                        "Recovered the pre-migration database to '{}', but unable to remove stale staging database '{}': {}",
                        database_path.display(),
                        migrating_path.display(),
                        error,
                    )
                }
            )?;
        }

        return Ok(());
    }


    if migrating_path.exists() {
        return Err(
            format!(
                "Database migration state is ambiguous: '{}' exists, but '{}' does not exist and no timestamped pre-migration recovery database was found. Refusing to promote the staging database or initialize a replacement database",
                migrating_path.display(),
                database_path.display(),
            )
        );
    }


    Ok(())
}


pub fn migrating_path(
    database_path: &Path,
) -> PathBuf {

    companion_path(
        database_path,
        ".migrating",
    )
}


pub fn pre_migration_path(
    database_path: &Path,
    source_schema_version: i64,
    timestamp_utc: &str,
) -> Result<PathBuf, String> {

    validate_timestamp(
        timestamp_utc
    )?;


    Ok(
        companion_path(
            database_path,
            &format!(
                ".pre-migration-v{:03}-{}",
                source_schema_version,
                timestamp_utc,
            ),
        )
    )
}


pub fn current_utc_timestamp() -> Result<String, String> {

    let duration =
        SystemTime::now()
            .duration_since(
                UNIX_EPOCH
            )
            .map_err(
                |error| {
                    format!(
                        "System clock is before the Unix epoch: {}",
                        error,
                    )
                }
            )?;


    utc_timestamp_from_unix_seconds(
        duration.as_secs()
    )
}


fn discover_pre_migration_paths(
    database_path: &Path,
) -> Result<Vec<PathBuf>, String> {

    let parent =
        database_path
            .parent()
            .unwrap_or_else(
                || Path::new(".")
            );

    let database_name =
        database_path
            .file_name()
            .ok_or_else(
                || {
                    format!(
                        "Database path '{}' has no filename",
                        database_path.display(),
                    )
                }
            )?
            .to_string_lossy();

    let prefix =
        format!(
            "{}.pre-migration-v",
            database_name
        );


    let entries =
        match fs::read_dir(
            parent
        ) {
            Ok(entries) => entries,

            Err(error)
                if error.kind()
                    == std::io::ErrorKind::NotFound =>
            {
                return Ok(
                    Vec::new()
                );
            }

            Err(error) => {
                return Err(
                    format!(
                        "Unable to inspect database directory '{}' for migration recovery files: {}",
                        parent.display(),
                        error,
                    )
                );
            }
        };


    let mut paths =
        Vec::new();


    for entry in entries {

        let entry =
            entry.map_err(
                |error| {
                    format!(
                        "Unable to inspect an entry in database directory '{}': {}",
                        parent.display(),
                        error,
                    )
                }
            )?;

        let file_type =
            entry.file_type()
                .map_err(
                    |error| {
                        format!(
                            "Unable to inspect migration recovery candidate '{}': {}",
                            entry.path().display(),
                            error,
                        )
                    }
                )?;


        if !file_type.is_file() {
            continue;
        }


        let filename =
            entry.file_name();

        let filename =
            filename.to_string_lossy();


        if !filename.starts_with(
            &prefix
        ) {
            continue;
        }


        if parse_recovery_filename(
            &database_name,
            &filename,
        )
        .is_some()
        {
            paths.push(
                entry.path()
            );
        }
    }


    Ok(
        paths
    )
}


fn newest_pre_migration_path(
    paths: &[PathBuf],
) -> Option<PathBuf> {

    paths
        .iter()
        .filter_map(
            |path| {
                let filename =
                    path.file_name()?
                        .to_string_lossy();

                let timestamp =
                    filename
                        .rsplit_once('-')?
                        .1
                        .to_string();

                Some(
                    (
                        timestamp,
                        path.clone(),
                    )
                )
            }
        )
        .max_by(
            |left, right| {
                left.0.cmp(
                    &right.0
                )
            }
        )
        .map(
            |(_, path)| path
        )
}


fn parse_recovery_filename(
    database_name: &str,
    filename: &str,
) -> Option<(i64, String)> {

    let prefix =
        format!(
            "{}.pre-migration-v",
            database_name
        );

    let remainder =
        filename.strip_prefix(
            &prefix
        )?;

    let (
        schema_text,
        timestamp,
    ) =
        remainder.split_once(
            '-'
        )?;


    if schema_text.len()
        != 3
        || !schema_text
            .chars()
            .all(
                |character| {
                    character.is_ascii_digit()
                }
            )
    {
        return None;
    }


    let schema_version =
        schema_text.parse::<i64>()
            .ok()?;


    if validate_timestamp(
        timestamp
    )
    .is_err()
    {
        return None;
    }


    Some(
        (
            schema_version,
            timestamp.to_string(),
        )
    )
}


fn validate_timestamp(
    timestamp: &str,
) -> Result<(), String> {

    let bytes =
        timestamp.as_bytes();


    let valid =
        bytes.len()
            == 16
        && bytes[8]
            == b'T'
        && bytes[15]
            == b'Z'
        && bytes
            .iter()
            .enumerate()
            .all(
                |(
                    index,
                    byte,
                )| {
                    index
                        == 8
                    || index
                        == 15
                    || byte.is_ascii_digit()
                }
            );


    if !valid {
        return Err(
            format!(
                "Invalid migration timestamp '{}'; expected YYYYMMDDTHHMMSSZ",
                timestamp,
            )
        );
    }


    Ok(())
}


fn utc_timestamp_from_unix_seconds(
    seconds: u64,
) -> Result<String, String> {

    let days =
        seconds
            / 86_400;

    let seconds_of_day =
        seconds
            % 86_400;


    let hour =
        seconds_of_day
            / 3_600;

    let minute =
        (
            seconds_of_day
                % 3_600
        )
            / 60;

    let second =
        seconds_of_day
            % 60;


    let (
        year,
        month,
        day,
    ) =
        civil_date_from_unix_days(
            i64::try_from(
                days
            )
            .map_err(
                |_| {
                    "System timestamp is too large to format"
                        .to_string()
                }
            )?
        );


    Ok(
        format!(
            "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
            year,
            month,
            day,
            hour,
            minute,
            second,
        )
    )
}


// Gregorian civil-date conversion adapted from the standard era/day-of-era
// decomposition.  Input day 0 is 1970-01-01.
fn civil_date_from_unix_days(
    unix_days: i64,
) -> (
    i64,
    u64,
    u64,
) {

    let z =
        unix_days
            + 719_468;

    let era =
        if z >= 0 {
            z
        } else {
            z - 146_096
        }
            / 146_097;

    let day_of_era =
        z
            - era
                * 146_097;

    let year_of_era =
        (
            day_of_era
                - day_of_era / 1_460
                + day_of_era / 36_524
                - day_of_era / 146_096
        )
            / 365;

    let mut year =
        year_of_era
            + era
                * 400;

    let day_of_year =
        day_of_era
            - (
                365
                    * year_of_era
                + year_of_era / 4
                - year_of_era / 100
            );

    let month_prime =
        (
            5
                * day_of_year
            + 2
        )
            / 153;

    let day =
        day_of_year
            - (
                153
                    * month_prime
                + 2
            )
                / 5
            + 1;

    let month =
        month_prime
            + if month_prime < 10 {
                3
            } else {
                -9
            };


    if month <= 2 {
        year += 1;
    }


    (
        year,
        month as u64,
        day as u64,
    )
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
