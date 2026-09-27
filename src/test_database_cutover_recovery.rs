use std::fs;
use std::path::{
    Path,
    PathBuf,
};


const LIVE_BYTES: &[u8] =
    b"SCREENSHAVER CUTOVER TEST: LIVE DATABASE\n";

const MIGRATING_BYTES: &[u8] =
    b"SCREENSHAVER CUTOVER TEST: MIGRATING DATABASE\n";

const PRE_MIGRATION_BYTES: &[u8] =
    b"SCREENSHAVER CUTOVER TEST: PRE-MIGRATION DATABASE\n";


pub fn run() -> Result<(), String> {

    let root =
        std::env::temp_dir()
            .join(
                "screenshaver-cutover-recovery-test"
            );


    if root.exists() {
        fs::remove_dir_all(
            &root
        )
        .map_err(
            |error| {
                format!(
                    "Unable to remove previous cutover-recovery test directory '{}': {}",
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
                "Unable to create cutover-recovery test directory '{}': {}",
                root.display(),
                error,
            )
        }
    )?;


    println!(
        "[CUTOVER RECOVERY TEST] Work directory: {}",
        root.display()
    );

    println!(
        "[CUTOVER RECOVERY TEST] Live Screenshaver database is not used by this test"
    );


    let result =
        run_cases(
            &root
        );


    let cleanup_result =
        fs::remove_dir_all(
            &root
        )
        .map_err(
            |error| {
                format!(
                    "Unable to remove cutover-recovery test directory '{}': {}",
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
                "[CUTOVER RECOVERY TEST] PASS: all filesystem recovery states behaved as required."
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


fn run_cases(
    root: &Path,
) -> Result<(), String> {

    run_case(
        root,
        "01-none",
        false,
        false,
        false,
        Expected::NoFiles,
    )?;

    run_case(
        root,
        "02-live",
        true,
        false,
        false,
        Expected::LiveOnly,
    )?;

    run_case(
        root,
        "03-live-migrating",
        true,
        true,
        false,
        Expected::LiveOnly,
    )?;

    run_case(
        root,
        "04-live-pre",
        true,
        false,
        true,
        Expected::LiveAndPre,
    )?;

    run_case(
        root,
        "05-live-migrating-pre",
        true,
        true,
        true,
        Expected::LiveAndPre,
    )?;

    run_case(
        root,
        "06-pre",
        false,
        false,
        true,
        Expected::RestoredPre,
    )?;

    run_case(
        root,
        "07-migrating-pre",
        false,
        true,
        true,
        Expected::RestoredPre,
    )?;

    run_case(
        root,
        "08-migrating",
        false,
        true,
        false,
        Expected::Ambiguous,
    )?;

    test_newest_timestamp_wins(
        root
    )?;


    Ok(())
}


#[derive(
    Debug,
    Clone,
    Copy,
)]
enum Expected {
    NoFiles,
    LiveOnly,
    LiveAndPre,
    RestoredPre,
    Ambiguous,
}


fn run_case(
    root: &Path,
    name: &str,
    live: bool,
    migrating: bool,
    pre_migration: bool,
    expected: Expected,
) -> Result<(), String> {

    let directory =
        root.join(
            name
        );


    fs::create_dir_all(
        &directory
    )
    .map_err(
        |error| {
            format!(
                "{}: unable to create test directory '{}': {}",
                name,
                directory.display(),
                error,
            )
        }
    )?;


    let database_path =
        directory.join(
            "screenshaver.db"
        );

    let migrating_path =
        crate::migrate_database::migrating_path(
            &database_path
        );

    let pre_migration_path =
        crate::migrate_database::pre_migration_path(
            &database_path,
            1,
            "20260927T120000Z",
        )?;


    if live {
        write_marker(
            &database_path,
            LIVE_BYTES,
        )?;
    }


    if migrating {
        write_marker(
            &migrating_path,
            MIGRATING_BYTES,
        )?;
    }


    if pre_migration {
        write_marker(
            &pre_migration_path,
            PRE_MIGRATION_BYTES,
        )?;
    }


    let recovery =
        crate::migrate_database::recover_interrupted_cutover(
            &database_path
        );


    match expected {

        Expected::NoFiles => {

            recovery.map_err(
                |error| {
                    format!(
                        "{}: recovery unexpectedly failed: {}",
                        name,
                        error,
                    )
                }
            )?;


            require_absent(
                name,
                "live",
                &database_path,
            )?;

            require_absent(
                name,
                "migrating",
                &migrating_path,
            )?;

            require_absent(
                name,
                "pre-migration",
                &pre_migration_path,
            )?;


            println!(
                "[CUTOVER RECOVERY TEST] Verified: no migration artifacts leaves fresh-install state untouched"
            );
        }


        Expected::LiveOnly => {

            recovery.map_err(
                |error| {
                    format!(
                        "{}: recovery unexpectedly failed: {}",
                        name,
                        error,
                    )
                }
            )?;


            require_bytes(
                name,
                "live",
                &database_path,
                LIVE_BYTES,
            )?;

            require_absent(
                name,
                "migrating",
                &migrating_path,
            )?;

            require_absent(
                name,
                "pre-migration",
                &pre_migration_path,
            )?;


            if migrating {
                println!(
                    "[CUTOVER RECOVERY TEST] Verified: live database retained and stale staging database removed"
                );
            } else {
                println!(
                    "[CUTOVER RECOVERY TEST] Verified: live database retained"
                );
            }
        }


        Expected::LiveAndPre => {

            recovery.map_err(
                |error| {
                    format!(
                        "{}: recovery unexpectedly failed: {}",
                        name,
                        error,
                    )
                }
            )?;


            require_bytes(
                name,
                "live",
                &database_path,
                LIVE_BYTES,
            )?;

            require_absent(
                name,
                "migrating",
                &migrating_path,
            )?;

            require_bytes(
                name,
                "pre-migration",
                &pre_migration_path,
                PRE_MIGRATION_BYTES,
            )?;


            if migrating {
                println!(
                    "[CUTOVER RECOVERY TEST] Verified: live database and recovery copy retained; stale staging database removed"
                );
            } else {
                println!(
                    "[CUTOVER RECOVERY TEST] Verified: live database and recovery copy retained"
                );
            }
        }


        Expected::RestoredPre => {

            recovery.map_err(
                |error| {
                    format!(
                        "{}: recovery unexpectedly failed: {}",
                        name,
                        error,
                    )
                }
            )?;


            require_bytes(
                name,
                "restored live",
                &database_path,
                PRE_MIGRATION_BYTES,
            )?;

            require_absent(
                name,
                "migrating",
                &migrating_path,
            )?;

            require_absent(
                name,
                "pre-migration",
                &pre_migration_path,
            )?;


            if migrating {
                println!(
                    "[CUTOVER RECOVERY TEST] Verified: pre-migration database restored and stale staging database removed"
                );
            } else {
                println!(
                    "[CUTOVER RECOVERY TEST] Verified: pre-migration database restored to live filename"
                );
            }
        }


        Expected::Ambiguous => {

            if recovery.is_ok() {
                return Err(
                    format!(
                        "{}: ambiguous orphaned staging state was unexpectedly accepted",
                        name,
                    )
                );
            }


            require_absent(
                name,
                "live",
                &database_path,
            )?;

            require_bytes(
                name,
                "migrating",
                &migrating_path,
                MIGRATING_BYTES,
            )?;

            require_absent(
                name,
                "pre-migration",
                &pre_migration_path,
            )?;


            println!(
                "[CUTOVER RECOVERY TEST] Verified: orphaned staging database rejected without promotion or deletion"
            );
        }
    }


    Ok(())
}


fn test_newest_timestamp_wins(
    root: &Path,
) -> Result<(), String> {

    let directory =
        root.join(
            "09-newest-recovery"
        );

    fs::create_dir_all(
        &directory
    )
    .map_err(
        |error| {
            format!(
                "Unable to create newest-recovery test directory '{}': {}",
                directory.display(),
                error,
            )
        }
    )?;


    let database_path =
        directory.join(
            "screenshaver.db"
        );

    let older_path =
        crate::migrate_database::pre_migration_path(
            &database_path,
            9,
            "20260927T120000Z",
        )?;

    let newer_path =
        crate::migrate_database::pre_migration_path(
            &database_path,
            1,
            "20260927T130000Z",
        )?;


    write_marker(
        &older_path,
        b"SCREENSHAVER CUTOVER TEST: OLDER RECOVERY\n",
    )?;

    write_marker(
        &newer_path,
        b"SCREENSHAVER CUTOVER TEST: NEWER RECOVERY\n",
    )?;


    crate::migrate_database::recover_interrupted_cutover(
        &database_path
    )?;


    require_bytes(
        "09-newest-recovery",
        "restored live",
        &database_path,
        b"SCREENSHAVER CUTOVER TEST: NEWER RECOVERY\n",
    )?;

    require_bytes(
        "09-newest-recovery",
        "older retained recovery",
        &older_path,
        b"SCREENSHAVER CUTOVER TEST: OLDER RECOVERY\n",
    )?;

    require_absent(
        "09-newest-recovery",
        "selected newer recovery",
        &newer_path,
    )?;


    println!(
        "[CUTOVER RECOVERY TEST] Verified: newest recovery timestamp wins regardless of schema number"
    );


    Ok(())
}


fn write_marker(
    path: &Path,
    bytes: &[u8],
) -> Result<(), String> {

    fs::write(
        path,
        bytes,
    )
    .map_err(
        |error| {
            format!(
                "Unable to write test marker '{}': {}",
                path.display(),
                error,
            )
        }
    )
}


fn require_absent(
    case_name: &str,
    label: &str,
    path: &Path,
) -> Result<(), String> {

    if path.exists() {
        return Err(
            format!(
                "{}: expected {} file '{}' to be absent",
                case_name,
                label,
                path.display(),
            )
        );
    }


    Ok(())
}


fn require_bytes(
    case_name: &str,
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
                    "{}: unable to read expected {} file '{}': {}",
                    case_name,
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
                "{}: {} file '{}' did not contain the expected marker bytes",
                case_name,
                label,
                path.display(),
            )
        );
    }


    Ok(())
}
