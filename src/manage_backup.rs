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
            format!(
                "Unable to create Screenshaver backup directory '{}': {}",
                path.display(),
                error,
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


#[derive(Clone, Debug)]
struct BackupUiState {
    loaded: bool,
    automatic_backups: bool,
    backup_interval_days: i64,
    last_backup: String,
    result_message: Option<(bool, String)>,
}


impl Default for BackupUiState {
    fn default() -> Self {
        Self {
            loaded: false,
            automatic_backups: true,
            backup_interval_days: 7,
            last_backup: String::new(),
            result_message: None,
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
    })
}


pub fn draw_controls(
    ui: &mut egui::Ui,
    status_message: &mut String,
) {
    let id = state_id();
    let mut state = ui.ctx().data(|data| data.get_temp::<BackupUiState>(id).unwrap_or_default());

    if !state.loaded {
        match load_state() {
            Ok(loaded) => state = loaded,
            Err(error) => {
                ui.label(egui::RichText::new(format!("Backup configuration unavailable: {}", error)).strong());
                return;
            }
        }
    }

    ui.label(egui::RichText::new("Full Backups").strong());
    ui.add_space(6.0);

    let old_enabled = state.automatic_backups;
    let old_interval = state.backup_interval_days;

    ui.horizontal(|ui| {
        ui.checkbox(
            &mut state.automatic_backups,
            "Make full Screenshaver backups every",
        );

        ui.add_enabled(
            state.automatic_backups,
            egui::DragValue::new(&mut state.backup_interval_days)
                .clamp_range(1..=3650)
                .speed(1.0),
        );

        ui.label("days");
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
                *status_message = "Backup configuration saved.".to_string();
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
        egui::RichText::new(format!("Last backup reference: {}", state.last_backup))
            .weak(),
    );
    ui.label(
        egui::RichText::new(format!("Backup folder: {}", crate::locate_paths::backup_dir().display()))
            .weak(),
    );

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if ui.button("Backup Now").clicked() {
            *status_message = "Creating full Screenshaver backup...".to_string();

            match backup_now() {
                Ok(path) => {
                    if let Ok(settings) = crate::manage_configuration::load_backup_settings() {
                        state.last_backup = settings.last_backup;
                    }
                    let message = format!("Backup created: {}", path.display());
                    *status_message = message.clone();
                    state.result_message = Some((true, message));
                }
                Err(error) => {
                    let message = format!("Backup failed: {}", error);
                    *status_message = message.clone();
                    state.result_message = Some((false, message));
                }
            }
        }

        let restore = ui.add_enabled(false, egui::Button::new("Restore from Backup"));
        restore.on_disabled_hover_text(
            "Restore from Backup will be enabled in the restore implementation phase."
        );
    });

    if let Some((success, message)) = state.result_message.as_ref() {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(message)
                .color(if *success { egui::Color32::GREEN } else { egui::Color32::RED }),
        );
    }

    ui.ctx().data_mut(|data| data.insert_temp(id, state));
}
