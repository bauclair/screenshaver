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


fn localized_restore_text(key: &str, params: &[(&str, String)]) -> String {
    let borrowed = params
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect::<Vec<_>>();
    crate::manage_localization::runtime_text_with_params(key, &borrowed)
}

fn restore_staging_directory() -> Result<PathBuf, String> {
    let root = crate::locate_paths::backup_dir().join(".restore-staging");
    if root.exists() {
        std::fs::remove_dir_all(&root).map_err(|error| {
            localized_restore_text(
                "backup.restore.stale_staging_remove_failed",
                &[("path", root.display().to_string()), ("error", error.to_string())],
            )
        })?;
    }
    std::fs::create_dir_all(&root).map_err(|error| {
        localized_restore_text(
            "backup.restore.staging_create_failed",
            &[("path", root.display().to_string()), ("error", error.to_string())],
        )
    })?;
    Ok(root)
}

fn inspect_and_stage_backup(archive_path: &std::path::Path) -> Result<RestoreInspection, String> {
    use std::io::Read;

    let file = std::fs::File::open(archive_path).map_err(|error| {
        localized_restore_text(
            "backup.restore.archive_open_failed",
            &[("path", archive_path.display().to_string()), ("error", error.to_string())],
        )
    })?;
    let mut zip = zip::ZipArchive::new(file).map_err(|error| {
        localized_restore_text("backup.restore.zip_read_failed", &[("error", error.to_string())])
    })?;

    let mut payloads = std::collections::BTreeMap::<String, Vec<u8>>::new();
    let mut manifest_bytes = None::<Vec<u8>>;

    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|error| {
            localized_restore_text("backup.restore.zip_member_inspect_failed", &[("index", index.to_string()), ("error", error.to_string())])
        })?;
        if entry.is_dir() {
            continue;
        }

        let name = entry.name().to_string();
        if !safe_archive_member(&name) {
            return Err(localized_restore_text(
                "backup.restore.unsafe_member",
                &[("member", name.to_string())],
            ));
        }
        if payloads.contains_key(&name) || (name == "manifest.json" && manifest_bytes.is_some()) {
            return Err(localized_restore_text(
                "backup.restore.duplicate_member",
                &[("member", name.to_string())],
            ));
        }

        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(|error| {
            localized_restore_text("backup.restore.zip_member_read_failed", &[("member", name.to_string()), ("error", error.to_string())])
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
        localized_restore_text("backup.restore.manifest_parse_failed", &[("error", error.to_string())])
    })?;

    if manifest.format.trim().is_empty() || manifest.format_version != 1 {
        return Err(localized_restore_text(
            "backup.restore.unsupported_format",
            &[("format", manifest.format.to_string()), ("version", manifest.format_version.to_string())],
        ));
    }
    if !manifest.backup {
        return Err(crate::manage_localization::runtime_text("backup.restore.export_not_backup"));
    }

    let database_member = manifest.database_snapshot.as_deref().ok_or_else(|| {
        crate::manage_localization::runtime_text("backup.restore.snapshot_missing_from_manifest")
    })?;
    if database_member != "backup/screenshaver.db" || !safe_archive_member(database_member) {
        return Err(localized_restore_text(
            "backup.restore.unsupported_snapshot_path",
            &[("path", database_member.to_string())],
        ));
    }

    for (name, integrity) in &manifest.files {
        let bytes = payloads.get(name).ok_or_else(|| {
            localized_restore_text("backup.restore.manifest_member_missing", &[("member", name.to_string())])
        })?;
        let actual = sha256_hex(bytes);
        if actual != integrity.sha256.to_lowercase() {
            return Err(localized_restore_text(
                "backup.restore.member_sha256_failed",
                &[("member", name.to_string())],
            ));
        }
    }

    let actual_package_hash = package_sha256(&payloads);
    if actual_package_hash != manifest.package_sha256.to_lowercase() {
        return Err(crate::manage_localization::runtime_text("backup.restore.package_sha256_failed"));
    }

    let database_bytes = payloads.get(database_member).ok_or_else(|| {
        crate::manage_localization::runtime_text("backup.restore.snapshot_payload_missing")
    })?;

    let staging_directory = restore_staging_directory()?;
    let staged_database = staging_directory.join("screenshaver.db");
    std::fs::write(&staged_database, database_bytes).map_err(|error| {
        localized_restore_text(
            "backup.restore.staged_database_write_failed",
            &[("path", staged_database.display().to_string()), ("error", error.to_string())],
        )
    })?;

    let managed_shader_directory = staging_directory.join("managed-shaders");
    std::fs::create_dir_all(&managed_shader_directory).map_err(|error| {
        localized_restore_text(
            "backup.restore.staged_shader_directory_create_failed",
            &[("path", managed_shader_directory.display().to_string()), ("error", error.to_string())],
        )
    })?;

    let mut managed_shader_count = 0_usize;
    for (name, bytes) in &payloads {
        let Some(filename) = name.strip_prefix("backup/managed-shaders/") else {
            continue;
        };
        if filename.is_empty() || filename.contains('/') || filename.contains('\\') {
            return Err(localized_restore_text(
                "backup.restore.invalid_managed_shader_member",
                &[("member", name.to_string())],
            ));
        }
        std::fs::write(managed_shader_directory.join(filename), bytes).map_err(|error| {
            localized_restore_text("backup.restore.managed_shader_stage_failed", &[("filename", filename.to_string()), ("error", error.to_string())])
        })?;
        managed_shader_count += 1;
    }

    let source_schema_version = manifest.database_schema_version as i64;
    let migrated_from_schema = crate::migrate_database::prepare_staged_restore(&staged_database)?;
    if migrated_from_schema != source_schema_version {
        return Err(localized_restore_text(
            "backup.restore.schema_mismatch",
            &[("manifest_schema", source_schema_version.to_string()), ("database_schema", migrated_from_schema.to_string())],
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
        localized_restore_text(
            "backup.restore.database_open_failed",
            &[("path", path.display().to_string()), ("error", error.to_string())],
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
        localized_restore_text("backup.restore.path_remove_failed", &[("path", path.display().to_string()), ("error", error.to_string())])
    })
}

fn copy_directory_tree(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), String> {
    std::fs::create_dir_all(destination).map_err(|error| {
        localized_restore_text(
            "backup.restore.directory_create_failed",
            &[("path", destination.display().to_string()), ("error", error.to_string())],
        )
    })?;

    if !source.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(source).map_err(|error| {
        localized_restore_text("backup.restore.directory_enumerate_failed", &[("path", source.display().to_string()), ("error", error.to_string())])
    })? {
        let entry = entry.map_err(|error| {
            localized_restore_text("backup.restore.directory_entry_read_failed", &[("path", source.display().to_string()), ("error", error.to_string())])
        })?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|error| {
            localized_restore_text("backup.restore.entry_inspect_failed", &[("path", source_path.display().to_string()), ("error", error.to_string())])
        })?;

        if file_type.is_dir() {
            copy_directory_tree(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&source_path, &destination_path).map_err(|error| {
                localized_restore_text(
                    "backup.restore.copy_failed",
                    &[("source", source_path.display().to_string()), ("destination", destination_path.display().to_string()), ("error", error.to_string())],
                )
            })?;
        } else {
            return Err(localized_restore_text(
                "backup.restore.unsupported_filesystem_entry",
                &[("path", source_path.display().to_string())],
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
        return Err(localized_restore_text(
            "backup.restore.live_database_missing",
            &[("path", live_database.display().to_string())],
        ));
    }

    remove_path_if_exists(rollback_database)?;
    let connection = crate::open_database::open()?;
    let quoted = rollback_database.to_string_lossy().replace('\'', "''");
    connection
        .execute_batch(&format!("VACUUM INTO '{}';", quoted))
        .map_err(|error| {
            localized_restore_text(
                "backup.restore.rollback_snapshot_create_failed",
                &[("path", rollback_database.display().to_string()), ("error", error.to_string())],
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
            localized_restore_text("backup.restore.failed_database_preserve_failed", &[("error", error.to_string())])
        })?;
    }
    std::fs::copy(rollback_database, live_database).map_err(|error| {
        localized_restore_text("backup.restore.database_rollback_failed", &[("error", error.to_string())])
    })?;

    if live_shaders.exists() {
        std::fs::rename(live_shaders, &failed_shaders).map_err(|error| {
            localized_restore_text("backup.restore.failed_shader_directory_preserve_failed", &[("error", error.to_string())])
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
        localized_restore_text(
            "backup.restore.configuration_directory_prepare_failed",
            &[("path", screenshaver_directory.display().to_string()), ("error", error.to_string())],
        )
    })?;

    let install_database = screenshaver_directory.join(".screenshaver.db.restore-new");
    let install_shaders = screenshaver_directory.join(".shaders.restore-new");
    remove_path_if_exists(&install_database)?;
    remove_path_if_exists(&install_shaders)?;

    std::fs::copy(&staged_database, &install_database).map_err(|error| {
        localized_restore_text("backup.restore.database_cutover_prepare_failed", &[("error", error.to_string())])
    })?;
    copy_directory_tree(&staged_shaders, &install_shaders)?;
    validate_restore_database(&install_database)?;

    let rollback_root = crate::locate_paths::backup_dir().join(".restore-rollback");
    remove_path_if_exists(&rollback_root)?;
    std::fs::create_dir_all(&rollback_root).map_err(|error| {
        localized_restore_text("backup.restore.rollback_directory_create_failed", &[("error", error.to_string())])
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
        localized_restore_text("backup.restore.database_cutover_begin_failed", &[("error", error.to_string())])
    })?;

    if let Err(error) = std::fs::rename(&install_database, &live_database) {
        let _ = std::fs::rename(&old_database, &live_database);
        return Err(localized_restore_text("backup.restore.database_install_failed_retained", &[("error", error.to_string())]));
    }

    if live_shaders.exists() {
        if let Err(error) = std::fs::rename(&live_shaders, &old_shaders) {
            let _ = remove_path_if_exists(&live_database);
            let _ = std::fs::rename(&old_database, &live_database);
            return Err(localized_restore_text("backup.restore.shader_cutover_begin_failed", &[("error", error.to_string())]));
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
            Ok(()) => Err(localized_restore_text("backup.restore.shader_install_failed_rolled_back", &[("error", error.to_string())])),
            Err(rollback_error) => Err(localized_restore_text("backup.restore.shader_install_and_rollback_failed", &[("error", error.to_string()), ("rollback_error", rollback_error.to_string())])),
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
            Ok(()) => Err(localized_restore_text("backup.restore.final_validation_failed_rolled_back", &[("error", error.to_string())])),
            Err(rollback_error) => Err(localized_restore_text("backup.restore.final_validation_and_rollback_failed", &[("error", error.to_string()), ("rollback_error", rollback_error.to_string())])),
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
            let message = localized_restore_text(
                "backup.restore.staged_successfully",
                &[
                    ("archive", inspection.archive_path.display().to_string()),
                    ("created", inspection.created.to_string()),
                    ("version", inspection.source_screenshaver_version.to_string()),
                    ("source_schema", inspection.source_schema_version.to_string()),
                    ("staged_schema", inspection.staged_schema_version.to_string()),
                    ("shader_count", inspection.managed_shader_count.to_string()),
                    ("staging", inspection.staging_directory.display().to_string()),
                ],
            );
            state.result_message = Some((true, message));
            state.pending_restore = Some(inspection);
        }
        Err(error) => {
            let message = localized_restore_text(
                "backup.restore.staging_failed",
                &[("error", error.to_string())],
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
            egui::RichText::new(crate::manage_localization::runtime_text("backup.restore.ready_to_install"))
                .strong()
        );
        ui.label(
            crate::manage_localization::runtime_text("backup.restore.confirmation_explanation")
        );
        ui.label(
            egui::RichText::new(
                localized_restore_text("backup.restore.selected_backup", &[("path", inspection.archive_path.display().to_string())])
            )
            .weak()
        );

        ui.horizontal(|ui| {
            if ui.button(crate::manage_localization::runtime_text("backup.restore.confirm")).clicked() {
                *status_message = crate::manage_localization::runtime_text("backup.restore.installing");
                match commit_staged_restore(&inspection) {
                    Ok(()) => {
                        state.pending_restore = None;
                        if let Ok(settings) = crate::manage_configuration::load_backup_settings() {
                            state.last_backup = settings.last_backup;
                        }
                        let message = crate::manage_localization::runtime_text("backup.restore.success");
                        *status_message = message.clone();
                        state.result_message = Some((true, message));
                    }
                    Err(error) => {
                        let message = localized_restore_text("backup.restore.failed", &[("error", error.to_string())]);
                        *status_message = message.clone();
                        state.result_message = Some((false, message));
                    }
                }
            }

            if ui.button(crate::manage_localization::runtime_text("backup.restore.cancel")).clicked() {
                let _ = remove_path_if_exists(&inspection.staging_directory);
                state.pending_restore = None;
                let message = crate::manage_localization::runtime_text("backup.restore.cancelled");
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
