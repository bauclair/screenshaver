//
// Screenshaver database migration subsystem.
//
// Historical database-schema readers translate durable data from their
// respective released schemas into the storage-independent MigrationData
// representation.
//
// Current-schema reconstruction code consumes MigrationData rather than
// depending directly on historical database layouts.
//

pub mod migration_data;
pub mod read_schema_v001;
pub mod read_schema_v002;
pub mod write_current;
