use std::time::Duration;

use rusqlite::{
    Connection,
    OpenFlags,
};


const DATABASE_BUSY_TIMEOUT_SECONDS: u64 = 5;


pub fn open() -> Result<Connection, String> {

    let database_path =
        crate::locate_paths::database_path();


    if database_path.exists() {

        let connection =
            Connection::open_with_flags(
                &database_path,
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


        configure_connection(
            &connection
        )?;


        // Schema-version inspection and migration dispatch will be added
        // after the Version 1 initialization path is complete.
        return Ok(
            connection
        );
    }


    crate::initialize_database::initialize(
        &database_path
    )
}


pub(crate) fn configure_connection(
    connection: &Connection,
) -> Result<(), String> {

    connection
        .pragma_update(
            None,
            "foreign_keys",
            "ON",
        )
        .map_err(
            |error| {
                format!(
                    "Unable to enable SQLite foreign-key enforcement: {}",
                    error,
                )
            }
        )?;


    connection
        .pragma_update(
            None,
            "synchronous",
            "FULL",
        )
        .map_err(
            |error| {
                format!(
                    "Unable to configure SQLite synchronous mode: {}",
                    error,
                )
            }
        )?;


    connection
        .busy_timeout(
            Duration::from_secs(
                DATABASE_BUSY_TIMEOUT_SECONDS
            )
        )
        .map_err(
            |error| {
                format!(
                    "Unable to configure SQLite busy timeout: {}",
                    error,
                )
            }
        )?;


    Ok(())
}
