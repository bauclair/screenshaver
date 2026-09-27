use rusqlite::Connection;


pub fn evaluate() -> Result<Connection, String> {

    let database_path =
        crate::locate_paths::database_path();


    // Recovery must run before "missing database" is interpreted as a fresh
    // installation. An interrupted cutover may temporarily leave the live
    // filename absent while the known-good source is retained as
    // screenshaver.db.pre-migration.
    crate::migrate_database::recover_interrupted_cutover(
        &database_path
    )?;


    if !database_path.exists() {

        return crate::initialize_database::initialize(
            &database_path
        );
    }


    // Existing databases are inspected read-only inside prepare(). Only a
    // current database, or a fully reconstructed and cut-over database in a
    // future migration implementation, is returned as the writable runtime
    // connection.
    crate::migrate_database::prepare(
        &database_path
    )
}
