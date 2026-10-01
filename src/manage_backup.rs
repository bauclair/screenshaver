//! Full Screenshaver backup scheduling and Control Center actions.
//!
//! Backup archives live outside ~/.config/screenshaver so they survive loss or
//! recreation of the operational configuration tree. Scheduling is database-
//! backed: app_defaults.last_backup is the authoritative reference time.

use std::path::PathBuf;


pub fn ensure_directory() -> Result<PathBuf, String> {
    let path = crate::locate_paths::backup_dir();
    std::fs::create_dir_all(&path)
        .map_err(|error| {
            crate::manage_localization::runtime_text_with_params(
                "backup.directory_create_failed",
                &[
                    ("path", &path.display().to_string()),
                    ("error", &error.to_string()),
                ],
            )
        })?;
    Ok(path)
}


pub fn backup_now() -> Result<PathBuf, String> {
    ensure_directory()?;

    let path = crate::export_data::create_full_backup()?;

    // Never advance the schedule unless the completed archive is already in
    // its final name. A failed archive therefore remains due for retry.
    crate::manage_configuration::mark_backup_completed()?;

    Ok(path)
}


pub fn run_scheduled_backup_if_due() -> Result<Option<PathBuf>, String> {
    ensure_directory()?;

    if !crate::manage_configuration::automatic_backup_due()? {
        return Ok(None);
    }

    backup_now().map(Some)
}



#[derive(Clone, Debug, serde::Deserialize)]
struct RestoreManifestFileIntegrity {
    sha256: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct RestoreManifest {
    format: String,
    format_version: u32,
    screenshaver_version: String,
    database_schema_version: u32,
    created: String,
    backup: bool,
    database_snapshot: Option<String>,
    files: std::collections::BTreeMap<String, RestoreManifestFileIntegrity>,
    package_sha256: String,
}

#[derive(Clone, Debug)]
struct RestoreInspection {
    archive_path: PathBuf,
    source_screenshaver_version: String,
    source_schema_version: i64,
    staged_schema_version: i64,
    created: String,
    managed_shader_count: usize,
    staging_directory: PathBuf,
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect()
}

fn package_sha256(payloads: &std::collections::BTreeMap<String, Vec<u8>>) -> String {
    use sha2::Digest;

    let mut canonical = Vec::<u8>::new();
    for (name, bytes) in payloads {
        canonical.extend_from_slice(name.as_bytes());
        canonical.push(0);
        canonical.extend_from_slice(bytes.len().to_string().as_bytes());
        canonical.push(0);
        canonical.extend_from_slice(sha256_hex(bytes).as_bytes());
        canonical.push(b'\n');
    }

    sha256_hex(&canonical)
}

fn safe_archive_member(name: &str) -> bool {
    let path = std::path::Path::new(name);
    !name.is_empty()
        && !name.starts_with('/')
        && !name.contains('\\')
        && path.components().all(|component| {
            matches!(component, std::path::Component::Normal(_))
        })
}

fn restore_staging_directory() -> Result<PathBuf, String> {
    let root = crate::locate_paths::backup_dir().join(".restore-staging");
    if root.exists() {
        std::fs::remove_dir_all(&root).map_err(|error| {
            format!(
                "Unable to remove stale restore staging directory '{}': {}",
                root.display(),
                error,
            )
        })?;
    }
    std::fs::create_dir_all(&root).map_err(|error| {
        format!(
            "Unable to create restore staging directory '{}': {}",
            root.display(),
            error,
        )
    })?;
    Ok(root)
}

fn inspect_and_stage_backup(archive_path: &std::path::Path) -> Result<RestoreInspection, String> {
    use std::io::Read;

    let file = std::fs::File::open(archive_path).map_err(|error| {
        format!(
            "Unable to open backup archive '{}': {}",
            archive_path.display(),
            error,
        )
    })?;
    let mut zip = zip::ZipArchive::new(file).map_err(|error| {
        format!("Unable to read backup ZIP archive: {}", error)
    })?;

    let mut payloads = std::collections::BTreeMap::<String, Vec<u8>>::new();
    let mut manifest_bytes = None::<Vec<u8>>;

    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|error| {
            format!("Unable to inspect backup ZIP member {}: {}", index, error)
        })?;
        if entry.is_dir() {
            continue;
        }

        let name = entry.name().to_string();
        if !safe_archive_member(&name) {
            return Err(format!(
                "Backup archive contains an unsafe member path '{}'; restore was refused",
                name,
            ));
        }
        if payloads.contains_key(&name) || (name == "manifest.json" && manifest_bytes.is_some()) {
            return Err(format!(
                "Backup archive contains duplicate member '{}'; restore was refused",
                name,
            ));
        }

        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(|error| {
            format!("Unable to read backup ZIP member '{}': {}", name, error)
        })?;

        if name == "manifest.json" {
            manifest_bytes = Some(bytes);
        } else {
            payloads.insert(name, bytes);
        }
    }

    let manifest_bytes = manifest_bytes.ok_or_else(|| {
        "Selected archive does not contain manifest.json".to_string()
    })?;
    let manifest: RestoreManifest = serde_json::from_slice(&manifest_bytes).map_err(|error| {
        format!("Unable to parse backup manifest: {}", error)
    })?;

    if manifest.format.trim().is_empty() || manifest.format_version != 1 {
        return Err(format!(
            "Unsupported Screenshaver backup format '{}' version {}",
            manifest.format,
            manifest.format_version,
        ));
    }
    if !manifest.backup {
        return Err("Selected Screenshaver archive is an export, not a full backup".to_string());
    }

    let database_member = manifest.database_snapshot.as_deref().ok_or_else(|| {
        "Backup manifest does not identify a database snapshot".to_string()
    })?;
    if database_member != "backup/screenshaver.db" || !safe_archive_member(database_member) {
        return Err(format!(
            "Backup manifest contains an unsupported database snapshot path '{}'",
            database_member,
        ));
    }

    for (name, integrity) in &manifest.files {
        let bytes = payloads.get(name).ok_or_else(|| {
            format!("Backup manifest references missing archive member '{}'", name)
        })?;
        let actual = sha256_hex(bytes);
        if actual != integrity.sha256.to_lowercase() {
            return Err(format!(
                "Backup archive member '{}' failed SHA-256 verification",
                name,
            ));
        }
    }

    let actual_package_hash = package_sha256(&payloads);
    if actual_package_hash != manifest.package_sha256.to_lowercase() {
        return Err("Backup archive failed package SHA-256 verification".to_string());
    }

    let database_bytes = payloads.get(database_member).ok_or_else(|| {
        "Backup archive does not contain its declared database snapshot".to_string()
    })?;

    let staging_directory = restore_staging_directory()?;
    let staged_database = staging_directory.join("screenshaver.db");
    std::fs::write(&staged_database, database_bytes).map_err(|error| {
        format!(
            "Unable to write staged restore database '{}': {}",
            staged_database.display(),
            error,
        )
    })?;

    let managed_shader_directory = staging_directory.join("managed-shaders");
    std::fs::create_dir_all(&managed_shader_directory).map_err(|error| {
        format!(
            "Unable to create staged managed-shader directory '{}': {}",
            managed_shader_directory.display(),
            error,
        )
    })?;

    let mut managed_shader_count = 0_usize;
    for (name, bytes) in &payloads {
        let Some(filename) = name.strip_prefix("backup/managed-shaders/") else {
            continue;
        };
        if filename.is_empty() || filename.contains('/') || filename.contains('\\') {
            return Err(format!(
                "Backup contains an invalid managed-shader member '{}'",
                name,
            ));
        }
        std::fs::write(managed_shader_directory.join(filename), bytes).map_err(|error| {
            format!("Unable to stage managed shader '{}': {}", filename, error)
        })?;
        managed_shader_count += 1;
    }

    let source_schema_version = manifest.database_schema_version as i64;
    let migrated_from_schema = crate::migrate_database::prepare_staged_restore(&staged_database)?;
    if migrated_from_schema != source_schema_version {
        return Err(format!(
            "Backup manifest reports database schema {}, but the staged database reports schema {}",
            source_schema_version,
            migrated_from_schema,
        ));
    }

    Ok(RestoreInspection {
        archive_path: archive_path.to_path_buf(),
        source_screenshaver_version: manifest.screenshaver_version,
        source_schema_version,
        staged_schema_version: crate::migrate_database::CURRENT_SCHEMA_VERSION,
        created: manifest.created,
        managed_shader_count,
        staging_directory,
    })
}

fn open_restore_database(path: &std::path::Path) -> Result<rusqlite::Connection, String> {
    let connection = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
    )
    .map_err(|error| {
        format!(
            "Unable to open restore database '{}': {}",
            path.display(),
            error,
        )
    })?;

    crate::open_database::configure_connection(&connection)?;
    Ok(connection)
}

fn validate_restore_database(path: &std::path::Path) -> Result<(), String> {
    let connection = open_restore_database(path)?;
    crate::validate_database::validate_startup(&connection)?;
    crate::validate_database::validate_integrity(&connection)?;
    Ok(())
}

fn remove_path_if_exists(path: &std::path::Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
    .map_err(|error| {
        format!("Unable to remove '{}': {}", path.display(), error)
    })
}

fn copy_directory_tree(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), String> {
    std::fs::create_dir_all(destination).map_err(|error| {
        format!(
            "Unable to create restore directory '{}': {}",
            destination.display(),
            error,
        )
    })?;

    if !source.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(source).map_err(|error| {
        format!("Unable to enumerate '{}': {}", source.display(), error)
    })? {
        let entry = entry.map_err(|error| {
            format!("Unable to read directory entry in '{}': {}", source.display(), error)
        })?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|error| {
            format!("Unable to inspect '{}': {}", source_path.display(), error)
        })?;

        if file_type.is_dir() {
            copy_directory_tree(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&source_path, &destination_path).map_err(|error| {
                format!(
                    "Unable to copy '{}' to '{}': {}",
                    source_path.display(),
                    destination_path.display(),
                    error,
                )
            })?;
        } else {
            return Err(format!(
                "Restore refused to copy unsupported filesystem entry '{}'",
                source_path.display(),
            ));
        }
    }

    Ok(())
}

fn create_rollback_database_snapshot(
    live_database: &std::path::Path,
    rollback_database: &std::path::Path,
) -> Result<(), String> {
    if !live_database.exists() {
        return Err(format!(
            "Live database '{}' does not exist; restore was refused",
            live_database.display(),
        ));
    }

    remove_path_if_exists(rollback_database)?;
    let connection = crate::open_database::open()?;
    let quoted = rollback_database.to_string_lossy().replace('\'', "''");
    connection
        .execute_batch(&format!("VACUUM INTO '{}';", quoted))
        .map_err(|error| {
            format!(
                "Unable to create pre-restore database snapshot '{}': {}",
                rollback_database.display(),
                error,
            )
        })?;
    drop(connection);
    validate_restore_database(rollback_database)
}

fn restore_pre_restore_state(
    live_database: &std::path::Path,
    live_shaders: &std::path::Path,
    rollback_database: &std::path::Path,
    rollback_shaders: &std::path::Path,
) -> Result<(), String> {
    let failed_database = live_database.with_extension("db.restore-failed");
    let failed_shaders = live_shaders.with_file_name("shaders.restore-failed");
    let _ = remove_path_if_exists(&failed_database);
    let _ = remove_path_if_exists(&failed_shaders);

    if live_database.exists() {
        std::fs::rename(live_database, &failed_database).map_err(|error| {
            format!("Unable to preserve failed restored database: {}", error)
        })?;
    }
    std::fs::copy(rollback_database, live_database).map_err(|error| {
        format!("Unable to restore pre-restore database: {}", error)
    })?;

    if live_shaders.exists() {
        std::fs::rename(live_shaders, &failed_shaders).map_err(|error| {
            format!("Unable to preserve failed restored shader directory: {}", error)
        })?;
    }
    copy_directory_tree(rollback_shaders, live_shaders)?;
    validate_restore_database(live_database)?;

    let _ = remove_path_if_exists(&failed_database);
    let _ = remove_path_if_exists(&failed_shaders);
    Ok(())
}

fn commit_staged_restore(inspection: &RestoreInspection) -> Result<(), String> {
    let staged_database = inspection.staging_directory.join("screenshaver.db");
    let staged_shaders = inspection.staging_directory.join("managed-shaders");
    validate_restore_database(&staged_database)?;

    let live_database = crate::locate_paths::database_path();
    let live_shaders = crate::locate_paths::shader_dir();
    let screenshaver_directory = crate::locate_paths::screenshaver_dir();
    std::fs::create_dir_all(&screenshaver_directory).map_err(|error| {
        format!(
            "Unable to prepare Screenshaver configuration directory '{}': {}",
            screenshaver_directory.display(),
            error,
        )
    })?;

    let install_database = screenshaver_directory.join(".screenshaver.db.restore-new");
    let install_shaders = screenshaver_directory.join(".shaders.restore-new");
    remove_path_if_exists(&install_database)?;
    remove_path_if_exists(&install_shaders)?;

    std::fs::copy(&staged_database, &install_database).map_err(|error| {
        format!("Unable to prepare restored database for cutover: {}", error)
    })?;
    copy_directory_tree(&staged_shaders, &install_shaders)?;
    validate_restore_database(&install_database)?;

    let rollback_root = crate::locate_paths::backup_dir().join(".restore-rollback");
    remove_path_if_exists(&rollback_root)?;
    std::fs::create_dir_all(&rollback_root).map_err(|error| {
        format!("Unable to create restore rollback directory: {}", error)
    })?;
    let rollback_database = rollback_root.join("screenshaver.db");
    let rollback_shaders = rollback_root.join("managed-shaders");

    create_rollback_database_snapshot(&live_database, &rollback_database)?;
    copy_directory_tree(&live_shaders, &rollback_shaders)?;

    let old_database = screenshaver_directory.join(".screenshaver.db.restore-old");
    let old_shaders = screenshaver_directory.join(".shaders.restore-old");
    remove_path_if_exists(&old_database)?;
    remove_path_if_exists(&old_shaders)?;

    std::fs::rename(&live_database, &old_database).map_err(|error| {
        format!("Unable to begin database restore cutover: {}", error)
    })?;

    if let Err(error) = std::fs::rename(&install_database, &live_database) {
        let _ = std::fs::rename(&old_database, &live_database);
        return Err(format!("Unable to install restored database; original database was retained: {}", error));
    }

    if live_shaders.exists() {
        if let Err(error) = std::fs::rename(&live_shaders, &old_shaders) {
            let _ = remove_path_if_exists(&live_database);
            let _ = std::fs::rename(&old_database, &live_database);
            return Err(format!("Unable to begin managed-shader restore; original database was restored: {}", error));
        }
    }

    if let Err(error) = std::fs::rename(&install_shaders, &live_shaders) {
        let rollback_result = restore_pre_restore_state(
            &live_database,
            &live_shaders,
            &rollback_database,
            &rollback_shaders,
        );
        return match rollback_result {
            Ok(()) => Err(format!("Unable to install restored managed shaders; pre-restore state was restored: {}", error)),
            Err(rollback_error) => Err(format!("Unable to install restored managed shaders: {}. Automatic rollback also failed: {}", error, rollback_error)),
        };
    }

    let final_validation = validate_restore_database(&live_database);
    if let Err(error) = final_validation {
        let rollback_result = restore_pre_restore_state(
            &live_database,
            &live_shaders,
            &rollback_database,
            &rollback_shaders,
        );
        return match rollback_result {
            Ok(()) => Err(format!("Restored database failed final validation; pre-restore state was restored: {}", error)),
            Err(rollback_error) => Err(format!("Restored database failed final validation: {}. Automatic rollback also failed: {}", error, rollback_error)),
        };
    }

    remove_path_if_exists(&old_database)?;
    remove_path_if_exists(&old_shaders)?;
    remove_path_if_exists(&rollback_root)?;
    remove_path_if_exists(&inspection.staging_directory)?;
    Ok(())
}

#[derive(Clone, Debug)]
struct BackupUiState {
    loaded: bool,
    automatic_backups: bool,
    backup_interval_days: i64,
    last_backup: String,
    result_message: Option<(bool, String)>,
    pending_restore: Option<RestoreInspection>,
}


impl Default for BackupUiState {
    fn default() -> Self {
        Self {
            loaded: false,
            automatic_backups: true,
            backup_interval_days: 7,
            last_backup: String::new(),
            result_message: None,
            pending_restore: None,
        }
    }
}


fn state_id() -> egui::Id {
    egui::Id::new("screenshaver_backup_configuration_state")
}


fn load_state() -> Result<BackupUiState, String> {
    let settings = crate::manage_configuration::load_backup_settings()?;
    Ok(BackupUiState {
        loaded: true,
        automatic_backups: settings.automatic_backups,
        backup_interval_days: settings.backup_interval_days,
        last_backup: settings.last_backup,
        result_message: None,
        pending_restore: None,
    })
}


pub fn set_restore_archive(
    context: &egui::Context,
    archive_path: &std::path::Path,
) {
    let id = state_id();
    let mut state = context.data(|data| {
        data.get_temp::<BackupUiState>(id).unwrap_or_default()
    });

    if !state.loaded {
        match load_state() {
            Ok(loaded) => state = loaded,
            Err(error) => {
                state.loaded = true;
                state.result_message = Some((false, error));
                context.data_mut(|data| data.insert_temp(id, state));
                return;
            }
        }
    }

    match inspect_and_stage_backup(archive_path) {
        Ok(inspection) => {
            let message = format!(
                "Backup verified and staged successfully: {} | created {} | Screenshaver {} | database schema {} -> {} | managed shaders {} | staging {}. No live Screenshaver files were changed.",
                inspection.archive_path.display(),
                inspection.created,
                inspection.source_screenshaver_version,
                inspection.source_schema_version,
                inspection.staged_schema_version,
                inspection.managed_shader_count,
                inspection.staging_directory.display(),
            );
            state.result_message = Some((true, message));
            state.pending_restore = Some(inspection);
        }
        Err(error) => {
            let message = format!(
                "Backup restore staging failed: {}",
                error,
            );
            state.result_message = Some((false, message));
            state.pending_restore = None;
        }
    }

    context.data_mut(|data| data.insert_temp(id, state));
}


pub fn draw_controls(
    ui: &mut egui::Ui,
    status_message: &mut String,
    restore_archive_browse_requested: &mut Option<std::path::PathBuf>,
) {
    let id = state_id();
    let mut state = ui.ctx().data(|data| data.get_temp::<BackupUiState>(id).unwrap_or_default());

    if !state.loaded {
        match load_state() {
            Ok(loaded) => state = loaded,
            Err(error) => {
                ui.label(
                    egui::RichText::new(
                        crate::manage_localization::runtime_text_with_params(
                            "backup.configuration_unavailable",
                            &[("error", &error)],
                        )
                    )
                    .strong()
                );
                return;
            }
        }
    }

    ui.label(
        egui::RichText::new(
            crate::manage_localization::runtime_text("backup.heading")
        )
        .strong()
    );
    ui.add_space(6.0);

    let old_enabled = state.automatic_backups;
    let old_interval = state.backup_interval_days;

    ui.horizontal(|ui| {
        ui.checkbox(
            &mut state.automatic_backups,
            crate::manage_localization::runtime_text("backup.schedule_prefix"),
        );

        ui.add_enabled(
            state.automatic_backups,
            egui::DragValue::new(&mut state.backup_interval_days)
                .clamp_range(1..=3650)
                .speed(1.0),
        );

        ui.label(
            crate::manage_localization::runtime_text("backup.days")
        );
    });

    state.backup_interval_days = state.backup_interval_days.max(1);

    if old_enabled != state.automatic_backups
        || old_interval != state.backup_interval_days
    {
        match crate::manage_configuration::save_backup_settings(
            state.automatic_backups,
            state.backup_interval_days,
        ) {
            Ok(()) => {
                *status_message =
                    crate::manage_localization::runtime_text(
                        "backup.configuration_saved"
                    );
            }
            Err(error) => {
                state.automatic_backups = old_enabled;
                state.backup_interval_days = old_interval;
                state.result_message = Some((false, error));
            }
        }
    }

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            crate::manage_localization::runtime_text_with_params(
                "backup.last_reference",
                &[("reference", &state.last_backup)],
            )
        )
        .weak(),
    );
    ui.label(
        egui::RichText::new(
            crate::manage_localization::runtime_text_with_params(
                "backup.folder",
                &[(
                    "path",
                    &crate::locate_paths::backup_dir()
                        .display()
                        .to_string(),
                )],
            )
        )
        .weak(),
    );

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if ui
            .button(
                crate::manage_localization::runtime_text(
                    "backup.now"
                )
            )
            .clicked()
        {
            *status_message =
                crate::manage_localization::runtime_text(
                    "backup.creating"
                );

            match backup_now() {
                Ok(path) => {
                    if let Ok(settings) = crate::manage_configuration::load_backup_settings() {
                        state.last_backup = settings.last_backup;
                    }
                    let message =
                        crate::manage_localization::runtime_text_with_params(
                            "backup.created",
                            &[(
                                "path",
                                &path.display().to_string(),
                            )],
                        );
                    *status_message = message.clone();
                    state.result_message = Some((true, message));
                }
                Err(error) => {
                    let message =
                        crate::manage_localization::runtime_text_with_params(
                            "backup.failed",
                            &[("error", &error)],
                        );
                    *status_message = message.clone();
                    state.result_message = Some((false, message));
                }
            }
        }

        if ui
            .button(
                crate::manage_localization::runtime_text(
                    "backup.restore"
                )
            )
            .clicked()
        {
            *restore_archive_browse_requested =
                Some(crate::locate_paths::backup_dir());
        }
    });


    if let Some(inspection) = state.pending_restore.clone() {
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Restore is ready to install")
                .strong()
        );
        ui.label(
            "The selected backup has passed verification. Confirming will replace the current Screenshaver database and managed shaders. A verified rollback copy of the current installation will be created before cutover."
        );
        ui.label(
            egui::RichText::new(
                format!("Backup: {}", inspection.archive_path.display())
            )
            .weak()
        );

        ui.horizontal(|ui| {
            if ui.button("Confirm Restore").clicked() {
                *status_message = "Installing verified Screenshaver backup...".to_string();
                match commit_staged_restore(&inspection) {
                    Ok(()) => {
                        state.pending_restore = None;
                        if let Ok(settings) = crate::manage_configuration::load_backup_settings() {
                            state.last_backup = settings.last_backup;
                        }
                        let message = "Backup restored successfully. The restored database and managed shaders passed final validation. Close the Control Center so Screenshaver can reload the restored configuration.".to_string();
                        *status_message = message.clone();
                        state.result_message = Some((true, message));
                    }
                    Err(error) => {
                        let message = format!("Backup restore failed: {}", error);
                        *status_message = message.clone();
                        state.result_message = Some((false, message));
                    }
                }
            }

            if ui.button("Cancel Restore").clicked() {
                let _ = remove_path_if_exists(&inspection.staging_directory);
                state.pending_restore = None;
                let message = "Restore cancelled. No live Screenshaver files were changed.".to_string();
                *status_message = message.clone();
                state.result_message = Some((true, message));
            }
        });
    }

    if let Some((success, message)) = state.result_message.as_ref() {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(message)
                .color(if *success { egui::Color32::GREEN } else { egui::Color32::RED }),
        );
    }

    ui.ctx().data_mut(|data| data.insert_temp(id, state));
}
