use rusqlite::Connection;


pub fn evaluate() -> Result<Connection, String> {

    let database_path =
        crate::locate_paths::database_path();


    if !database_path.exists() {

        return crate::initialize_database::initialize(
            &database_path
        );
    }


    let mut connection =
        crate::open_database::open()?;


    crate::migrate_database::migrate(
        &mut connection
    )?;


    crate::validate_database::validate_startup(
        &connection
    )?;


    Ok(
        connection
    )
}
