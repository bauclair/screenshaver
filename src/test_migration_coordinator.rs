use std::fs;
use std::path::{
    Path,
    PathBuf,
};

use rusqlite::{
    Connection,
    OpenFlags,
};


const FIXTURE_PATH: &str =
    "tests/database_migration/fixtures/schema_v001.db";

const SUCCESS_TIMESTAMP: &str =
    "20260927T140000Z";

const ROLLBACK_TIMESTAMP: &str =
    "20260927T150000Z";


pub fn run() -> Result<(), String> {

    let fixture_path =
        PathBuf::from(
            FIXTURE_PATH
        );

    if !fixture_path.is_file() {
        return Err(
            format!(
                "Permanent Schema-1 fixture does not exist: {}",
                fixture_path.display(),
            )
        );
    }


    let fixture_bytes =
        fs::read(
            &fixture_path
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read permanent Schema-1 fixture: {}",
                    error,
                )
            }
        )?;


    let root =
        std::env::temp_dir()
            .join(
                "screenshaver-migration-coordinator-test"
            );


    if root.exists() {
        fs::remove_dir_all(
            &root
        )
        .map_err(
            |error| {
                format!(
                    "Unable to remove previous coordinator-test directory '{}': {}",
                    root.display(),
                    error,
                )
            }
        )?;
    }


    fs::create_dir_all(
        &root
    )
    .map_err(
        |error| {
            format!(
                "Unable to create coordinator-test directory '{}': {}",
                root.display(),
                error,
            )
        }
    )?;


    println!(
        "[MIGRATION COORDINATOR TEST] Fixture: {}",
        fixture_path.display()
    );

    println!(
        "[MIGRATION COORDINATOR TEST] Work directory: {}",
        root.display()
    );

    println!(
        "[MIGRATION COORDINATOR TEST] Live Screenshaver database is not used by this test"
    );


    let result =
        run_tests(
            &fixture_path,
            &fixture_bytes,
            &root,
        );


    let fixture_after =
        fs::read(
            &fixture_path
        )
        .map_err(
            |error| {
                format!(
                    "Unable to reread permanent fixture after coordinator test: {}",
                    error,
                )
            }
        )?;


    if fixture_bytes
        != fixture_after
    {
        return Err(
            "Permanent Schema-1 fixture changed during coordinator testing"
                .to_string()
        );
    }


    let cleanup_result =
        fs::remove_dir_all(
            &root
        )
        .map_err(
            |error| {
                format!(
                    "Unable to remove coordinator-test directory '{}': {}",
                    root.display(),
                    error,
                )
            }
        );


    match (
        result,
        cleanup_result,
    ) {
        (
            Ok(()),
            Ok(()),
        ) => {
            println!(
                "[MIGRATION COORDINATOR TEST] Verified: permanent Schema-1 fixture remained byte-for-byte unchanged"
            );

            println!(
                "[MIGRATION COORDINATOR TEST] PASS: production cutover and rollback behavior operated as required."
            );

            Ok(())
        }

        (
            Err(error),
            Ok(()),
        ) => {
            Err(
                error
            )
        }

        (
            Ok(()),
            Err(cleanup_error),
        ) => {
            Err(
                cleanup_error
            )
        }

        (
            Err(error),
            Err(cleanup_error),
        ) => {
            Err(
                format!(
                    "{}; additionally, cleanup failed: {}",
                    error,
                    cleanup_error,
                )
            )
        }
    }
}


fn run_tests(
    fixture_path: &Path,
    fixture_bytes: &[u8],
    root: &Path,
) -> Result<(), String> {

    test_successful_cutover(
        fixture_path,
        fixture_bytes,
        root,
    )?;

    test_final_validation_rollback(
        fixture_path,
        fixture_bytes,
        root,
    )?;


    Ok(())
}


fn test_successful_cutover(
    fixture_path: &Path,
    fixture_bytes: &[u8],
    root: &Path,
) -> Result<(), String> {

    let directory =
        root.join(
            "successful-cutover"
        );

    fs::create_dir_all(
        &directory
    )
    .map_err(
        |error| {
            format!(
                "Unable to create successful-cutover directory: {}",
                error,
            )
        }
    )?;


    let database_path =
        directory.join(
            "screenshaver.db"
        );

    fs::copy(
        fixture_path,
        &database_path,
    )
    .map_err(
        |error| {
            format!(
                "Unable to copy fixture for successful-cutover test: {}",
                error,
            )
        }
    )?;


    let staging_path =
        crate::migrate_database::migrating_path(
            &database_path
        );

    reconstruct_fixture(
        fixture_path,
        &staging_path,
    )?;


    let recovery_path =
        crate::migrate_database::pre_migration_path(
            &database_path,
            1,
            SUCCESS_TIMESTAMP,
        )?;


    let connection =
        crate::migrate_database::test_promote_staging(
            &database_path,
            &staging_path,
            &recovery_path,
            SUCCESS_TIMESTAMP,
            false,
        )?;

    drop(
        connection
    );


    require_bytes(
        "successful cutover recovery copy",
        &recovery_path,
        fixture_bytes,
    )?;

    require_absent(
        "successful cutover staging",
        &staging_path,
    )?;

    validate_current_database(
        &database_path
    )?;


    println!(
        "[MIGRATION COORDINATOR TEST] Verified: successful promotion retained original source database under timestamped recovery filename"
    );


    Ok(())
}


fn test_final_validation_rollback(
    fixture_path: &Path,
    fixture_bytes: &[u8],
    root: &Path,
) -> Result<(), String> {

    let directory =
        root.join(
            "forced-final-validation-failure"
        );

    fs::create_dir_all(
        &directory
    )
    .map_err(
        |error| {
            format!(
                "Unable to create rollback-test directory: {}",
                error,
            )
        }
    )?;


    let database_path =
        directory.join(
            "screenshaver.db"
        );

    fs::copy(
        fixture_path,
        &database_path,
    )
    .map_err(
        |error| {
            format!(
                "Unable to copy fixture for rollback test: {}",
                error,
            )
        }
    )?;


    let staging_path =
        crate::migrate_database::migrating_path(
            &database_path
        );

    reconstruct_fixture(
        fixture_path,
        &staging_path,
    )?;


    let recovery_path =
        crate::migrate_database::pre_migration_path(
            &database_path,
            1,
            ROLLBACK_TIMESTAMP,
        )?;

    let failed_path =
        failed_path(
            &database_path,
            ROLLBACK_TIMESTAMP,
        );


    let result =
        crate::migrate_database::test_promote_staging(
            &database_path,
            &staging_path,
            &recovery_path,
            ROLLBACK_TIMESTAMP,
            true,
        );


    if result.is_ok() {
        return Err(
            "Forced final-validation failure unexpectedly succeeded"
                .to_string()
        );
    }


    require_bytes(
        "restored original live database",
        &database_path,
        fixture_bytes,
    )?;

    require_absent(
        "consumed rollback recovery filename",
        &recovery_path,
    )?;

    require_absent(
        "rollback staging filename",
        &staging_path,
    )?;

    validate_current_database(
        &failed_path
    )?;


    println!(
        "[MIGRATION COORDINATOR TEST] Verified: forced post-promotion validation failure restored original database byte-for-byte"
    );

    println!(
        "[MIGRATION COORDINATOR TEST] Verified: failed promoted database retained as diagnostic evidence"
    );


    Ok(())
}


fn reconstruct_fixture(
    fixture_path: &Path,
    staging_path: &Path,
) -> Result<(), String> {

    let source =
        Connection::open_with_flags(
            fixture_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to open fixture read-only: {}",
                    error,
                )
            }
        )?;


    let data =
        crate::database_migration::read_schema_v001::read(
            &source
        )?;

    drop(
        source
    );


    let staging =
        crate::database_migration::write_current::write(
            staging_path,
            &data,
        )?;

    crate::validate_database::validate_startup(
        &staging
    )?;

    crate::validate_database::validate_integrity(
        &staging
    )?;

    drop(
        staging
    );


    Ok(())
}


fn validate_current_database(
    path: &Path,
) -> Result<(), String> {

    let connection =
        Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to open reconstructed test database '{}': {}",
                    path.display(),
                    error,
                )
            }
        )?;

    crate::open_database::configure_connection(
        &connection
    )?;

    crate::validate_database::validate_startup(
        &connection
    )?;

    crate::validate_database::validate_integrity(
        &connection
    )?;


    Ok(())
}


fn failed_path(
    database_path: &Path,
    timestamp: &str,
) -> PathBuf {

    let mut filename =
        database_path
            .as_os_str()
            .to_os_string();

    filename.push(
        format!(
            ".failed-migration-{}",
            timestamp,
        )
    );


    PathBuf::from(
        filename
    )
}


fn require_bytes(
    label: &str,
    path: &Path,
    expected: &[u8],
) -> Result<(), String> {

    let actual =
        fs::read(
            path
        )
        .map_err(
            |error| {
                format!(
                    "{}: unable to read '{}': {}",
                    label,
                    path.display(),
                    error,
                )
            }
        )?;


    if actual
        != expected
    {
        return Err(
            format!(
                "{}: file contents differ from expected bytes: {}",
                label,
                path.display(),
            )
        );
    }


    Ok(())
}


fn require_absent(
    label: &str,
    path: &Path,
) -> Result<(), String> {

    if path.exists() {
        return Err(
            format!(
                "{} unexpectedly exists: {}",
                label,
                path.display(),
            )
        );
    }


    Ok(())
}
